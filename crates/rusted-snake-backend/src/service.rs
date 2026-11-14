use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Result, anyhow};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use rusted_snake_core::effect::{SoftwareEffect, SoftwareEffectSettings};
use rusted_snake_core::macros::MacroDef;
use rusted_snake_core::{
    DpiStages, OnboardAction, OnboardData, OnboardLighting, Rgb, Settings, ZoneEffect, ZoneId,
    paths,
};

use crate::audio::{self, AudioSource, LevelMeter};
use crate::macros::{EngineStatus, MacroEngine};
use crate::onboard::{Session, known_caps};
use crate::openrazer::OpenRazer;
use crate::openrgb::{self, OpenRgbApply, OpenRgbDevice};

#[derive(Debug)]
pub enum Command {
    RefreshDevices,
    SetBrightness {
        device: String,
        value: f64,
    },
    SetZoneBrightness {
        device: String,
        zone: ZoneId,
        value: f64,
    },
    SetZoneEffect {
        device: String,
        zone: ZoneId,
        effect: ZoneEffect,
    },
    SetDpi {
        device: String,
        x: i32,
        y: i32,
    },
    SetPollRate {
        device: String,
        rate: u32,
    },
    ApplySoftwareEffect {
        device: String,
        settings: SoftwareEffectSettings,
    },
    RefreshOnboard {
        device: String,
    },
    SetExperimental {
        experimental: bool,
    },
    SetOnboardButton {
        device: String,
        profile: u8,
        button: u8,
        action: OnboardAction,
    },
    SetOnboardDpiStages {
        device: String,
        profile: u8,
        stages: DpiStages,
    },
    SetOnboardLighting {
        device: String,
        profile: u8,
        zone: u8,
        lighting: OnboardLighting,
    },
    BackupOnboard {
        device: String,
        path: Option<PathBuf>,
    },
    RestoreOnboard {
        device: String,
        path: PathBuf,
    },
    ListBackups,
    RefreshAudioSources,
    ReloadMacros(Vec<MacroDef>),
    ListOpenRgbDevices,
    ApplyOpenRgb(OpenRgbApply),
    Shutdown,
}

#[derive(Clone, Debug)]
pub struct BackendInfo {
    pub connected: bool,
    pub version: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BackupEntry {
    pub path: PathBuf,
    pub label: String,
    pub modified: SystemTime,
}

#[derive(Clone, Debug)]
pub enum Event {
    Devices(Vec<rusted_snake_core::DeviceSnapshot>),
    DevicesError(String),
    Backend(BackendInfo),
    Onboard {
        device: String,
        data: Option<OnboardData>,
        error: Option<String>,
    },
    Status {
        message: String,
        error: bool,
    },
    AudioSources(Vec<AudioSource>),
    MacroStatus(EngineStatus),
    Backups(Vec<BackupEntry>),
    SoftwareEffect {
        device: String,
        active: Option<SoftwareEffect>,
    },
    OpenRgbDevices {
        devices: Vec<OpenRgbDevice>,
        error: Option<String>,
    },
}

pub struct BackendHandle {
    commands: Sender<Command>,
    events: Receiver<Event>,
    thread: Option<JoinHandle<()>>,
}

impl BackendHandle {
    pub fn spawn(settings: Settings) -> Self {
        let (commands, command_rx) = unbounded();
        let (event_tx, events) = unbounded();
        let thread = thread::spawn(move || {
            let mut backend = Backend::new(settings, event_tx);
            backend.run(command_rx);
        });
        Self {
            commands,
            events,
            thread: Some(thread),
        }
    }

    pub fn send(&self, command: Command) {
        let _ = self.commands.send(command);
    }

    pub fn events(&self) -> &Receiver<Event> {
        &self.events
    }
}

impl Drop for BackendHandle {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct SoftwareRuntime {
    settings: SoftwareEffectSettings,
    started: Instant,
    last_applied: Instant,
    meter: Option<LevelMeter>,
    zone_colors: HashMap<String, Rgb>,
}

struct Backend {
    client: Option<OpenRazer>,
    version: String,
    devices: Vec<rusted_snake_core::DeviceSnapshot>,
    settings: Settings,
    events: Sender<Event>,
    last_connect: Instant,
    last_refresh: Instant,
    last_scan_error: Option<String>,
    onboard: HashMap<String, Session>,
    backed_up: HashSet<String>,
    software: HashMap<String, SoftwareRuntime>,
    hardware_effects: HashMap<String, HashMap<String, ZoneEffect>>,
    macro_engine: MacroEngine,
    last_macro_status: Option<EngineStatus>,
    audio_error: Option<String>,
}

impl Backend {
    fn new(settings: Settings, events: Sender<Event>) -> Self {
        let macro_engine = MacroEngine::start();
        Self {
            client: None,
            version: String::new(),
            devices: Vec::new(),
            settings,
            events,
            last_connect: Instant::now() - Duration::from_secs(60),
            last_refresh: Instant::now() - Duration::from_secs(60),
            last_scan_error: None,
            onboard: HashMap::new(),
            backed_up: HashSet::new(),
            software: HashMap::new(),
            hardware_effects: HashMap::new(),
            macro_engine,
            last_macro_status: None,
            audio_error: None,
        }
    }

    fn run(&mut self, commands: Receiver<Command>) {
        self.try_connect();
        self.refresh_devices();
        self.emit_macro_status();
        loop {
            match commands.recv_timeout(Duration::from_millis(25)) {
                Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(command) => self.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
            }
            self.keepalive();
            self.tick_software();
            self.emit_macro_status();
        }
        self.software.clear();
        self.onboard.clear();
        self.macro_engine.stop();
    }

    fn keepalive(&mut self) {
        if self.client.is_none() && self.last_connect.elapsed() >= Duration::from_secs(3) {
            self.try_connect();
            if self.client.is_some() {
                self.refresh_devices();
            }
        }
        if self.client.is_some() && self.last_refresh.elapsed() >= Duration::from_secs(4) {
            self.refresh_devices();
        }
    }

    fn try_connect(&mut self) {
        self.last_connect = Instant::now();
        match OpenRazer::connect() {
            Ok(client) => match client.version() {
                Ok(version) => {
                    self.version = version.clone();
                    self.last_scan_error = None;
                    self.client = Some(client);
                    self.send_event(Event::Backend(BackendInfo {
                        connected: true,
                        version,
                        error: None,
                    }));
                }
                Err(error) => {
                    self.client = None;
                    self.send_event(Event::Backend(BackendInfo {
                        connected: false,
                        version: String::new(),
                        error: Some(error.to_string()),
                    }));
                }
            },
            Err(error) => {
                self.client = None;
                let message = error.to_string();
                if self.last_scan_error.as_deref() != Some(message.as_str()) {
                    self.last_scan_error = Some(message.clone());
                    self.send_event(Event::Backend(BackendInfo {
                        connected: false,
                        version: String::new(),
                        error: Some(message),
                    }));
                }
            }
        }
    }

    fn refresh_devices(&mut self) {
        self.last_refresh = Instant::now();
        let Some(client) = &self.client else {
            return;
        };
        match client.devices() {
            Ok(mut devices) => {
                for device in &mut devices {
                    if let Some(caps) = &mut device.onboard {
                        caps.experimental = self.settings.experimental_onboard;
                    }
                }
                let connected: HashSet<_> =
                    devices.iter().map(|device| device.id.clone()).collect();
                self.onboard.retain(|id, _| connected.contains(id));
                self.software.retain(|id, _| connected.contains(id));
                self.hardware_effects.retain(|id, _| connected.contains(id));
                self.devices = devices.clone();
                self.send_event(Event::Devices(devices));
            }
            Err(error) => {
                let message = error.to_string();
                if self.last_scan_error.as_deref() != Some(message.as_str()) {
                    self.last_scan_error = Some(message.clone());
                    self.send_event(Event::DevicesError(message));
                }
            }
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::RefreshDevices => self.refresh_devices(),
            Command::SetBrightness { device, value } => {
                let result = self.with_client(|client| client.set_brightness(&device, value));
                self.report(result, format!("Brightness set to {:.0}%.", value));
                self.refresh_devices();
            }
            Command::SetZoneBrightness {
                device,
                zone,
                value,
            } => {
                let result =
                    self.with_client(|client| client.set_zone_brightness(&device, &zone, value));
                self.report(
                    result,
                    format!("{} brightness set to {value:.0}%.", zone.name),
                );
                self.refresh_devices();
            }
            Command::SetZoneEffect {
                device,
                zone,
                effect,
            } => {
                if let Some(runtime) = self.software.remove(&device) {
                    drop(runtime);
                    self.send_event(Event::SoftwareEffect {
                        device: device.clone(),
                        active: None,
                    });
                }
                let result =
                    self.with_client(|client| client.set_zone_effect(&device, &zone, &effect));
                if result.is_ok() {
                    self.hardware_effects
                        .entry(device.clone())
                        .or_default()
                        .insert(zone.interface.clone(), effect.clone());
                }
                self.report(
                    result,
                    format!("{} set to {}.", zone.name, effect.kind.label()),
                );
                self.refresh_devices();
            }
            Command::SetDpi { device, x, y } => {
                let result = self.with_client(|client| client.set_dpi(&device, x, y));
                self.report(result, format!("DPI set to {x} x {y}."));
                self.refresh_devices();
            }
            Command::SetPollRate { device, rate } => {
                let result = self.with_client(|client| client.set_poll_rate(&device, rate));
                self.report(result, format!("Polling rate set to {rate} Hz."));
                self.refresh_devices();
            }
            Command::ApplySoftwareEffect { device, settings } => {
                self.apply_software_effect(device, settings);
            }
            Command::RefreshOnboard { device } => self.refresh_onboard(&device),
            Command::SetExperimental { experimental } => {
                self.settings.experimental_onboard = experimental;
                if let Err(error) = self.settings.save() {
                    self.status(format!("Could not save settings: {error}"), true);
                }
                for session in self.onboard.values_mut() {
                    session.set_experimental(experimental);
                }
                for device in &mut self.devices {
                    if let Some(caps) = &mut device.onboard {
                        caps.experimental = experimental;
                    }
                }
                self.send_event(Event::Devices(self.devices.clone()));
            }
            Command::SetOnboardButton {
                device,
                profile,
                button,
                action,
            } => {
                let result = self.with_onboard(&device, |session| {
                    session.write_button(profile, button, &action)
                });
                self.report(result, "Button saved on the mouse.".into());
                self.refresh_onboard(&device);
            }
            Command::SetOnboardDpiStages {
                device,
                profile,
                stages,
            } => {
                let result = self.with_onboard(&device, |session| {
                    session.write_dpi_stages(profile, &stages)
                });
                self.report(result, "DPI stages saved on the mouse.".into());
                self.refresh_onboard(&device);
            }
            Command::SetOnboardLighting {
                device,
                profile,
                zone,
                lighting,
            } => {
                let result = self.with_onboard(&device, |session| {
                    session.write_lighting(profile, zone, &lighting)
                });
                self.report(result, "Lighting saved on the mouse.".into());
                self.refresh_onboard(&device);
            }
            Command::BackupOnboard { device, path } => self.backup_onboard(&device, path),
            Command::RestoreOnboard { device, path } => self.restore_onboard(&device, path),
            Command::ListBackups => self.send_event(Event::Backups(backups())),
            Command::RefreshAudioSources => {
                let sources = audio::sources();
                self.send_event(Event::AudioSources(sources));
            }
            Command::ReloadMacros(macros) => {
                self.macro_engine.reload(macros);
                self.emit_macro_status();
            }
            Command::ListOpenRgbDevices => match openrgb::list_devices() {
                Ok(devices) => self.send_event(Event::OpenRgbDevices {
                    devices,
                    error: None,
                }),
                Err(error) => self.send_event(Event::OpenRgbDevices {
                    devices: Vec::new(),
                    error: Some(error.to_string()),
                }),
            },
            Command::ApplyOpenRgb(change) => {
                let result = openrgb::apply(&change);
                self.report(result, "OpenRGB lighting updated.".into());
            }
            Command::Shutdown => {}
        }
    }

    fn with_client<R>(&self, f: impl FnOnce(&OpenRazer) -> Result<R>) -> Result<R> {
        match &self.client {
            Some(client) => f(client),
            None => Err(anyhow!("OpenRazer is not connected")),
        }
    }

    fn with_onboard<R>(
        &mut self,
        device: &str,
        f: impl FnOnce(&mut Session) -> Result<R>,
    ) -> Result<R> {
        let snapshot = self.devices.iter().find(|item| item.id == device).cloned();
        let snapshot = snapshot.ok_or_else(|| anyhow!("device {device} is not connected"))?;
        if !self.onboard.contains_key(device) {
            let session = Session::open(snapshot.pid, self.settings.experimental_onboard)
                .map_err(|error| anyhow!("{error}"))?
                .ok_or_else(|| anyhow!("no onboard profiles found on this mouse"))?;
            self.onboard.insert(device.to_string(), session);
        }
        if !self.backed_up.contains(device) {
            let session = self.onboard.get_mut(device).expect("session inserted");
            let data = session.read_all()?;
            let path = paths::backups_dir().join(format!("before-first-save-{}.json", stamp()));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, data.backup().to_json())?;
            self.backed_up.insert(device.to_string());
            self.status(
                format!("Backed up every profile to {}.", path.display()),
                false,
            );
        }
        let session = self.onboard.get_mut(device).expect("session inserted");
        f(session)
    }

    fn refresh_onboard(&mut self, device: &str) {
        let Some(snapshot) = self.devices.iter().find(|item| item.id == device).cloned() else {
            self.send_event(Event::Onboard {
                device: device.to_string(),
                data: None,
                error: Some("device is not connected".into()),
            });
            return;
        };
        if !self.onboard.contains_key(device) {
            match Session::open(snapshot.pid, self.settings.experimental_onboard) {
                Ok(Some(session)) => {
                    self.onboard.insert(device.to_string(), session);
                }
                Ok(None) => {
                    self.send_event(Event::Onboard {
                        device: device.to_string(),
                        data: None,
                        error: Some(if known_caps(snapshot.pid).is_some() {
                            "The mouse was not found on a readable hidraw node. Install the udev rule from Settings.".into()
                        } else {
                            "This device does not expose onboard profiles.".into()
                        }),
                    });
                    return;
                }
                Err(error) => {
                    self.send_event(Event::Onboard {
                        device: device.to_string(),
                        data: None,
                        error: Some(error.to_string()),
                    });
                    return;
                }
            }
        }
        let Some(session) = self.onboard.get_mut(device) else {
            return;
        };
        match session.read_all() {
            Ok(mut data) => {
                data.caps.pid = snapshot.pid;
                self.send_event(Event::Onboard {
                    device: device.to_string(),
                    data: Some(data),
                    error: None,
                });
            }
            Err(error) => {
                self.send_event(Event::Onboard {
                    device: device.to_string(),
                    data: None,
                    error: Some(error.to_string()),
                });
            }
        }
    }

    fn backup_onboard(&mut self, device: &str, path: Option<PathBuf>) {
        let Some(snapshot) = self.devices.iter().find(|item| item.id == device) else {
            self.status("Device is not connected.".into(), true);
            return;
        };
        if !self.onboard.contains_key(device) {
            match Session::open(snapshot.pid, self.settings.experimental_onboard) {
                Ok(Some(session)) => {
                    self.onboard.insert(device.to_string(), session);
                }
                Ok(None) => {
                    self.status("No onboard profiles found on this mouse.".into(), true);
                    return;
                }
                Err(error) => {
                    self.status(error.to_string(), true);
                    return;
                }
            }
        }
        let path =
            path.unwrap_or_else(|| paths::backups_dir().join(format!("profiles-{}.json", stamp())));
        let result = self
            .onboard
            .get_mut(device)
            .expect("session")
            .read_all()
            .and_then(|data| {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, data.backup().to_json())?;
                Ok(())
            });
        self.report(result, format!("Backed up to {}.", path.display()));
        self.send_event(Event::Backups(backups()));
    }

    fn restore_onboard(&mut self, device: &str, path: PathBuf) {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                self.status(format!("Could not read {}: {error}", path.display()), true);
                return;
            }
        };
        let file = match rusted_snake_core::onboard::BackupFile::parse(&text) {
            Ok(file) => file,
            Err(error) => {
                self.status(format!("Backup is not valid: {error}"), true);
                return;
            }
        };
        let profiles = match file.into_profiles() {
            Ok(profiles) => profiles,
            Err(error) => {
                self.status(format!("Backup is not valid: {error}"), true);
                return;
            }
        };
        let result = self.with_onboard(device, |session| {
            for profile in profiles {
                session.write_dpi_stages(profile.slot, &profile.dpi)?;
                for (button, action) in profile.buttons {
                    session.write_button(profile.slot, button, &action)?;
                }
                for (zone, lighting) in profile.lighting {
                    session.write_lighting(profile.slot, zone, &lighting)?;
                }
            }
            Ok(())
        });
        self.report(result, "Profiles restored to the mouse.".into());
        self.refresh_onboard(device);
    }

    fn apply_software_effect(&mut self, device: String, settings: SoftwareEffectSettings) {
        if settings.effect == SoftwareEffect::None {
            self.software.remove(&device);
            let zones = self
                .devices
                .iter()
                .find(|item| item.id == device)
                .map(|item| item.zones.clone())
                .unwrap_or_default();
            if let Some(client) = &self.client {
                for zone in &zones {
                    if let Some(effect) = self
                        .hardware_effects
                        .get(&device)
                        .and_then(|effects| effects.get(&zone.id.interface))
                    {
                        let _ = client.set_zone_effect(&device, &zone.id, effect);
                    } else {
                        let _ = client.restore_last_effect(&device, &zone.id);
                    }
                }
            }
            self.send_event(Event::SoftwareEffect {
                device,
                active: None,
            });
            return;
        }

        self.hardware_effects.entry(device.clone()).or_default();
        if self.software.contains_key(&device) {
            self.software.remove(&device);
        }
        let meter = if settings.effect == SoftwareEffect::Audio {
            Some(LevelMeter::start(
                settings
                    .audio_source
                    .clone()
                    .or_else(audio::default_monitor),
                settings.audio_gain,
            ))
        } else {
            None
        };
        self.audio_error = None;
        self.software.insert(
            device.clone(),
            SoftwareRuntime {
                settings: settings.clone(),
                started: Instant::now(),
                last_applied: Instant::now() - Duration::from_secs(1),
                meter,
                zone_colors: HashMap::new(),
            },
        );
        self.send_event(Event::SoftwareEffect {
            device,
            active: Some(settings.effect),
        });
    }

    fn tick_software(&mut self) {
        if self.software.is_empty() {
            return;
        }
        let Some(client) = &self.client else { return };
        let now = Instant::now();
        let mut removals = Vec::new();
        let mut audio_message = None;
        for (device_id, runtime) in self.software.iter_mut() {
            if let Some(message) = runtime.meter.as_ref().and_then(|meter| meter.error())
                && self.audio_error.as_deref() != Some(message.as_str())
            {
                audio_message = Some(message);
            }
            if now.duration_since(runtime.last_applied) < Duration::from_millis(50) {
                continue;
            }
            runtime.last_applied = now;
            let Some(device) = self.devices.iter().find(|item| item.id == *device_id) else {
                removals.push(device_id.clone());
                continue;
            };
            let elapsed = now.duration_since(runtime.started).as_secs_f32();
            let settings = &runtime.settings;
            for (index, zone) in device.zones.iter().enumerate() {
                if !zone
                    .supported
                    .contains(&rusted_snake_core::EffectKind::Static)
                {
                    continue;
                }
                let color = match settings.effect {
                    SoftwareEffect::Spectrum => {
                        let speed = 0.3 + settings.speed.clamp(0.0, 1.0) * 1.7;
                        let hue = (elapsed * 140.0 * speed + index as f32 * 37.0) % 360.0;
                        Rgb::from_hsv(hue, 0.95, 1.0)
                    }
                    SoftwareEffect::Breath => {
                        let speed = 0.35 + settings.speed.clamp(0.0, 1.0) * 1.65;
                        let phase =
                            (elapsed * speed * std::f32::consts::TAU * 0.35).sin() * 0.5 + 0.5;
                        let first = settings.colors.first().copied().unwrap_or(Rgb::RAZER_GREEN);
                        let second = settings.colors.get(1).copied().unwrap_or(first);
                        let mixed = first.mix(second, phase);
                        mixed.scale(0.35 + phase * 0.65)
                    }
                    SoftwareEffect::Audio => {
                        let level = runtime.meter.as_ref().map(LevelMeter::level).unwrap_or(0.0);
                        let colors = &settings.audio_colors;
                        let low = colors
                            .first()
                            .copied()
                            .unwrap_or(Rgb::new(0x10, 0x2a, 0x08));
                        let mid = colors.get(1).copied().unwrap_or(Rgb::RAZER_GREEN);
                        let high = colors.get(2).copied().unwrap_or(Rgb::WHITE);
                        if level < 0.5 {
                            low.mix(mid, level * 2.0)
                        } else {
                            mid.mix(high, (level - 0.5) * 2.0)
                        }
                    }
                    SoftwareEffect::None => continue,
                };
                let previous = runtime.zone_colors.get(&zone.id.interface).copied();
                if previous == Some(color) {
                    continue;
                }
                if let Err(error) = client.set_static(device_id, &zone.id, color) {
                    log::warn!("software effect on {device_id}: {error}");
                    continue;
                }
                runtime.zone_colors.insert(zone.id.interface.clone(), color);
            }
        }
        if let Some(message) = audio_message {
            self.audio_error = Some(message.clone());
            self.status(format!("Audio effect: {message}"), true);
        }
        for device in removals {
            self.software.remove(&device);
            self.send_event(Event::SoftwareEffect {
                device,
                active: None,
            });
        }
    }

    fn emit_macro_status(&mut self) {
        let status = self.macro_engine.status();
        let changed = match &self.last_macro_status {
            None => true,
            Some(previous) => {
                previous.state != status.state
                    || previous.message != status.message
                    || previous.devices != status.devices
            }
        };
        if changed {
            self.last_macro_status = Some(status.clone());
            self.send_event(Event::MacroStatus(status));
        }
    }

    fn report(&mut self, result: Result<()>, success: String) {
        match result {
            Ok(()) => self.status(success, false),
            Err(error) => self.status(error.to_string(), true),
        }
    }

    fn status(&self, message: String, error: bool) {
        self.send_event(Event::Status { message, error });
    }

    fn send_event(&self, event: Event) {
        let _ = self.events.send(event);
    }
}

pub fn backups() -> Vec<BackupEntry> {
    let mut entries = Vec::new();
    let Ok(read_dir) = std::fs::read_dir(paths::backups_dir()) else {
        return entries;
    };
    for entry in read_dir.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let label = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("backup.json")
            .to_string();
        entries.push(BackupEntry {
            path,
            label,
            modified,
        });
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.modified));
    entries
}

fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}
