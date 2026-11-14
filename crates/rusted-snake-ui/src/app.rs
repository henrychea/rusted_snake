use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, CursorStyle, Div, FocusHandle, FontWeight, KeyDownEvent, MouseButton,
    SharedString, Window, div, px,
};
use rusted_snake_backend::{
    AudioSource, BackendHandle, BackendInfo, BackupEntry, Command, EngineState, EngineStatus, Event,
};
use rusted_snake_core::effect::{SoftwareEffect, SoftwareEffectSettings};
use rusted_snake_core::macros::{MacroAction, MacroDef, MacroLibrary, RepeatMode};
use rusted_snake_core::{
    DeviceSnapshot, EffectKind, OnboardAction, OnboardData, OnboardLighting, Rgb, Settings,
    ZoneEffect, keymap,
};

use crate::theme;
use crate::widgets::{self, OnPress};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Page {
    Devices,
    Buttons,
    Performance,
    Lighting,
    Macros,
    Settings,
}

impl Page {
    pub const ALL: [Page; 6] = [
        Page::Devices,
        Page::Buttons,
        Page::Performance,
        Page::Lighting,
        Page::Macros,
        Page::Settings,
    ];

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "devices" => Some(Self::Devices),
            "buttons" => Some(Self::Buttons),
            "performance" | "sensor" => Some(Self::Performance),
            "lighting" | "chroma" => Some(Self::Lighting),
            "macros" => Some(Self::Macros),
            "settings" => Some(Self::Settings),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Page::Devices => "Devices",
            Page::Buttons => "Buttons",
            Page::Performance => "Performance",
            Page::Lighting => "Lighting",
            Page::Macros => "Macros",
            Page::Settings => "Settings",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Page::Devices => "icons/devices.svg",
            Page::Buttons => "icons/buttons.svg",
            Page::Performance => "icons/performance.svg",
            Page::Lighting => "icons/lighting.svg",
            Page::Macros => "icons/macros.svg",
            Page::Settings => "icons/settings.svg",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum FieldId {
    MacroName,
    MacroText,
    MacroCommand,
    SetName,
}

impl FieldId {
    pub const ALL: [FieldId; 4] = [
        FieldId::MacroName,
        FieldId::MacroText,
        FieldId::MacroCommand,
        FieldId::SetName,
    ];
}

#[derive(Clone, Debug, Default)]
pub struct FieldState {
    pub value: String,
    pub caret: usize,
}

impl FieldState {
    pub fn set(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.caret = self.value.len();
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CaptureTarget {
    OnboardKey,
    MacroKey,
    MacroTrigger,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Keep,
    Key,
    Mouse,
    DpiUp,
    DpiDown,
    NextProfile,
    Disabled,
}

impl ActionKind {
    pub fn label(self) -> &'static str {
        match self {
            ActionKind::Keep => "Keep current",
            ActionKind::Key => "Keyboard key",
            ActionKind::Mouse => "Mouse button",
            ActionKind::DpiUp => "DPI up",
            ActionKind::DpiDown => "DPI down",
            ActionKind::NextProfile => "Next profile",
            ActionKind::Disabled => "Turn off",
        }
    }

    pub fn from_action(action: &OnboardAction) -> Self {
        match action.action_type {
            OnboardAction::KEY => Self::Key,
            OnboardAction::MOUSE => Self::Mouse,
            OnboardAction::DPI => {
                if action.params.first().copied() == Some(2) {
                    Self::DpiDown
                } else {
                    Self::DpiUp
                }
            }
            OnboardAction::PROFILE => Self::NextProfile,
            OnboardAction::DISABLED => Self::Disabled,
            _ => Self::Keep,
        }
    }
}

pub struct OnboardUi {
    pub loading: bool,
    pub data: Option<OnboardData>,
    pub error: Option<String>,
    pub profile: u8,
    pub plate: Option<String>,
    pub editing: Option<u8>,
    pub pending_action: Option<OnboardAction>,
    pub action_kind: ActionKind,
    pub mouse_button: u8,
    pub lights: BTreeMap<u8, OnboardLighting>,
}

impl Default for OnboardUi {
    fn default() -> Self {
        Self {
            loading: false,
            data: None,
            error: None,
            profile: 1,
            plate: None,
            editing: None,
            pending_action: None,
            action_kind: ActionKind::Key,
            mouse_button: 4,
            lights: BTreeMap::new(),
        }
    }
}

#[derive(Default)]
pub struct PerformanceUi {
    pub dpi: Option<i32>,
    pub poll_rate: Option<u32>,
    pub stages: Vec<u16>,
    pub active_stage: usize,
}

#[derive(Clone, Debug)]
pub struct ZoneDraft {
    pub effect: EffectKind,
    pub colors: [Rgb; 2],
    pub speed: u8,
    pub direction: i32,
}

impl Default for ZoneDraft {
    fn default() -> Self {
        Self {
            effect: EffectKind::Static,
            colors: [Rgb::RAZER_GREEN, Rgb::new(0x1f, 0x8a, 0xff)],
            speed: 128,
            direction: 1,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ColorTarget {
    Zone(String, usize),
    Onboard(u8, usize),
    SoftwareColor(usize),
    AudioColor(usize),
}

#[derive(Clone)]
pub struct ColorPickerUi {
    pub target: ColorTarget,
    pub hue: f32,
    pub sat: f32,
    pub val: f32,
}

impl ColorPickerUi {
    pub fn color(&self) -> Rgb {
        Rgb::from_hsv(self.hue, self.sat, self.val)
    }
}

#[derive(Default)]
pub struct LightingUi {
    pub brightness: Option<f64>,
    pub zone_brightness: HashMap<String, f64>,
    pub zones: HashMap<String, ZoneDraft>,
    pub picker: Option<ColorPickerUi>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MacroKind {
    Key,
    Text,
    Command,
}

impl MacroKind {
    pub fn label(self) -> &'static str {
        match self {
            MacroKind::Key => "Press a key",
            MacroKind::Text => "Type text",
            MacroKind::Command => "Run a command",
        }
    }
}

pub struct MacroEditor {
    pub id: Option<u64>,
    pub kind: MacroKind,
    pub key: Option<u16>,
    pub key_modifiers: keymap::Modifiers,
    pub trigger: Option<u16>,
    pub repeat: RepeatMode,
    pub capturing_key: bool,
    pub capturing_trigger: bool,
}

#[derive(Default)]
pub struct MacroUi {
    pub editor: Option<MacroEditor>,
    pub renaming_set: bool,
    pub confirm_delete_set: bool,
    pub confirm_delete_macro: Option<u64>,
    pub status: EngineStatus,
    pub trigger_picker: Option<u64>,
}

#[derive(Default)]
pub struct SettingsUi {
    pub show_backups: bool,
    pub experimental: bool,
    pub autostart: bool,
    pub confirm_restore: Option<std::path::PathBuf>,
}

pub struct Toast {
    pub message: String,
    pub error: bool,
    pub created: Instant,
}

pub struct Root {
    pub backend: BackendHandle,
    pub events: crossbeam_channel::Receiver<Event>,
    pub settings: Settings,
    pub macros: MacroLibrary,
    pub devices: Vec<DeviceSnapshot>,
    pub selected: Option<String>,
    pub page: Page,
    pub backend_info: BackendInfo,
    pub onboard: OnboardUi,
    pub performance: PerformanceUi,
    pub lighting: LightingUi,
    pub macro_ui: MacroUi,
    pub settings_ui: SettingsUi,
    pub fields: HashMap<FieldId, FieldState>,
    pub field_focus: HashMap<FieldId, FocusHandle>,
    pub capture_focus: FocusHandle,
    pub capture: Option<CaptureTarget>,
    pub audio_sources: Vec<AudioSource>,
    pub backups: Vec<BackupEntry>,
    pub software_active: HashMap<String, SoftwareEffect>,
    pub toast: Option<Toast>,
    toast_seq: u64,
    _poll_task: gpui::Task<()>,
}

impl Root {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let settings = Settings::load();
        let macros = MacroLibrary::load();
        let backend = BackendHandle::spawn(settings.clone());
        let events = backend.events().clone();
        let mut field_focus = HashMap::new();
        for field in FieldId::ALL {
            field_focus.insert(field, cx.focus_handle());
        }
        let mut fields = HashMap::new();
        for field in FieldId::ALL {
            fields.insert(field, FieldState::default());
        }
        let capture_focus = cx.focus_handle();
        let mut root = Self {
            backend,
            events: events.clone(),
            settings,
            macros,
            devices: Vec::new(),
            selected: None,
            page: Page::Devices,
            backend_info: BackendInfo {
                connected: false,
                version: String::new(),
                error: None,
            },
            onboard: OnboardUi::default(),
            performance: PerformanceUi::default(),
            lighting: LightingUi::default(),
            macro_ui: MacroUi::default(),
            settings_ui: SettingsUi::default(),
            fields,
            field_focus,
            capture_focus,
            capture: None,
            audio_sources: Vec::new(),
            backups: Vec::new(),
            software_active: HashMap::new(),
            toast: None,
            toast_seq: 0,
            _poll_task: cx.spawn(async |_this, _cx| {}),
        };
        root.settings_ui.autostart = rusted_snake_core::settings::is_autostart_enabled();
        root.settings_ui.experimental = root.settings.experimental_onboard;
        root.lighting_brightness_from_settings();
        root.reload_macro_engine();
        root.backend.send(Command::RefreshDevices);
        root.backend.send(Command::RefreshAudioSources);
        root.backend.send(Command::ListBackups);
        let task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(120))
                    .await;
                let mut batch = Vec::new();
                while let Ok(event) = events.try_recv() {
                    batch.push(event);
                }
                if batch.is_empty() {
                    continue;
                }
                if this
                    .update(cx, |root, cx| {
                        for event in batch {
                            root.handle_event(event, cx);
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        root._poll_task = task;
        root
    }

    pub fn current_device(&self) -> Option<&DeviceSnapshot> {
        self.selected
            .as_deref()
            .and_then(|id| self.devices.iter().find(|device| device.id == id))
    }

    pub fn set_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        if page == Page::Buttons {
            self.request_onboard();
        }
        cx.notify();
    }

    pub fn select_device(&mut self, id: String, cx: &mut Context<Self>) {
        if self.selected.as_deref() == Some(id.as_str()) {
            self.page = Page::Buttons;
            cx.notify();
            return;
        }
        self.selected = Some(id.clone());
        self.settings.selected_device = Some(id.clone());
        let _ = self.settings.save();
        self.onboard = OnboardUi::default();
        self.performance = PerformanceUi::default();
        self.lighting = LightingUi::default();
        self.page = Page::Buttons;
        self.request_onboard();
        if let Some(device) = self.devices.iter().find(|device| device.id == id).cloned() {
            self.init_device_ui(&device);
        }
        cx.notify();
    }

    fn init_device_ui(&mut self, device: &DeviceSnapshot) {
        self.performance.dpi = device.dpi.as_ref().map(|dpi| dpi.x);
        self.performance.poll_rate = device.poll_rate;
        self.lighting.brightness = device.brightness;
        self.lighting.zones.clear();
        self.lighting.zone_brightness.clear();
        for zone in &device.zones {
            if let Some(brightness) = zone.brightness {
                self.lighting
                    .zone_brightness
                    .insert(zone.id.interface.clone(), brightness);
            }
            let effect = zone
                .current_effect
                .as_deref()
                .and_then(parse_effect)
                .filter(|effect| zone.supported.contains(effect))
                .or_else(|| {
                    zone.supported
                        .iter()
                        .find(|effect| **effect == EffectKind::Static)
                        .copied()
                })
                .unwrap_or(EffectKind::Off);
            let first = zone.colors.first().copied().unwrap_or(Rgb::RAZER_GREEN);
            let second = zone
                .colors
                .get(1)
                .copied()
                .unwrap_or(Rgb::new(0x1f, 0x8a, 0xff));
            self.lighting.zones.insert(
                zone.id.interface.clone(),
                ZoneDraft {
                    effect,
                    colors: [first, second],
                    speed: zone.speed.clamp(1, 255) as u8,
                    direction: 1,
                },
            );
        }
        if self.settings.software.effect != SoftwareEffect::None
            && device
                .zones
                .iter()
                .any(|zone| zone.supported.contains(&EffectKind::Static))
        {
            let settings = self.settings.software.clone();
            self.software_active
                .insert(device.id.clone(), settings.effect);
            self.backend.send(Command::ApplySoftwareEffect {
                device: device.id.clone(),
                settings,
            });
        }
    }

    pub fn request_onboard(&mut self) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        let device = self.devices.iter().find(|device| device.id == id);
        let supported = device
            .map(|device| {
                device.onboard.is_some() || device.kind == rusted_snake_core::DeviceKind::Mouse
            })
            .unwrap_or(false);
        if !supported {
            return;
        }
        self.onboard.loading = true;
        self.backend.send(Command::RefreshOnboard { device: id });
    }

    pub fn handle_event(&mut self, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::Devices(devices) => {
                self.devices = devices;
                let selected_exists = self
                    .selected
                    .as_ref()
                    .is_some_and(|id| self.devices.iter().any(|device| device.id == *id));
                if !selected_exists {
                    let preferred = self
                        .settings
                        .selected_device
                        .as_ref()
                        .filter(|id| self.devices.iter().any(|device| device.id == **id))
                        .cloned();
                    self.selected =
                        preferred.or_else(|| self.devices.first().map(|device| device.id.clone()));
                    self.onboard = OnboardUi::default();
                    self.performance = PerformanceUi::default();
                    self.lighting = LightingUi::default();
                    if let Some(device) = self.current_device().cloned() {
                        self.init_device_ui(&device);
                        self.request_onboard();
                    }
                } else if let Some(device) = self.current_device().cloned() {
                    if self.lighting.brightness.is_none() {
                        self.init_device_ui(&device);
                    } else {
                        self.performance.dpi = device.dpi.as_ref().map(|dpi| dpi.x);
                        self.performance.poll_rate = device.poll_rate;
                    }
                }
            }
            Event::DevicesError(message) => self.show_toast(message, true, cx),
            Event::Backend(info) => {
                self.backend_info = info;
            }
            Event::Onboard {
                device,
                data,
                error,
            } => {
                if self.selected.as_deref() != Some(device.as_str()) {
                    return;
                }
                self.onboard.loading = false;
                self.onboard.error = error;
                if let Some(data) = data {
                    let previous_profile = self.onboard.profile;
                    self.onboard.data = Some(data.clone());
                    let valid_previous = data
                        .profiles
                        .iter()
                        .any(|profile| profile.slot == previous_profile);
                    self.onboard.profile = if valid_previous {
                        previous_profile
                    } else if data.active_profile >= 1 {
                        data.active_profile
                    } else {
                        data.profiles_in_use.first().copied().unwrap_or(1)
                    };
                    self.refresh_onboard_drafts();
                    self.init_plate();
                    if let Some(device) = self.current_device().cloned() {
                        if self.performance.dpi.is_none() {
                            self.performance.dpi = device.dpi.as_ref().map(|dpi| dpi.x);
                        }
                        self.lighting.brightness = device.brightness;
                    }
                }
            }
            Event::Status { message, error } => self.show_toast(message, error, cx),
            Event::AudioSources(sources) => {
                self.audio_sources = sources;
            }
            Event::MacroStatus(status) => {
                self.macro_ui.status = status;
            }
            Event::Backups(entries) => {
                self.backups = entries;
            }
            Event::SoftwareEffect { device, active } => {
                if let Some(effect) = active {
                    self.software_active.insert(device, effect);
                } else {
                    self.software_active.remove(&device);
                }
            }
        }
    }

    fn refresh_onboard_drafts(&mut self) {
        let Some(data) = &self.onboard.data else {
            return;
        };
        let Some(profile) = data.profile(self.onboard.profile) else {
            return;
        };
        self.onboard.lights = profile.lighting.clone();
        self.performance.stages = profile.dpi.dpi.clone();
        self.performance.active_stage = profile
            .dpi
            .active
            .min(profile.dpi.dpi.len().saturating_sub(1));
    }

    fn init_plate(&mut self) {
        let Some(data) = &self.onboard.data else {
            return;
        };
        let mut plates: Vec<String> = data
            .buttons
            .iter()
            .map(|button| button.group.clone())
            .filter(|group| group.contains("plate"))
            .collect();
        plates.sort();
        plates.dedup();
        if plates.is_empty() {
            self.onboard.plate = None;
            return;
        }
        let saved = self
            .selected
            .as_ref()
            .and_then(|id| self.settings.onboard_plate.get(id))
            .cloned();
        self.onboard.plate = saved
            .filter(|plate| plates.contains(plate))
            .or_else(|| plates.first().cloned());
    }

    pub fn onboard_profile_changed(&mut self, profile: u8, cx: &mut Context<Self>) {
        self.onboard.profile = profile;
        self.onboard.editing = None;
        self.refresh_onboard_drafts();
        cx.notify();
    }

    pub fn set_onboard_lighting_effect(&mut self, zone: u8, effect: u8, cx: &mut Context<Self>) {
        let entry = self
            .onboard
            .lights
            .entry(zone)
            .or_insert_with(OnboardLighting::off);
        entry.effect = effect;
        if effect == OnboardLighting::SPECTRUM {
            entry.speed = 0x28;
            entry.colors.clear();
        } else if effect == OnboardLighting::OFF {
            entry.colors.clear();
        } else if entry.colors.is_empty() {
            entry.colors = vec![Rgb::RAZER_GREEN.to_led()];
        }
        cx.notify();
    }

    pub fn save_onboard_lighting(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        let Some(data) = &self.onboard.data else {
            return;
        };
        if !data.caps.writable() {
            self.show_toast(
                "Saving to this mouse is off. Turn on experimental saving in Settings.".into(),
                true,
                cx,
            );
            return;
        }
        let profile = self.onboard.profile;
        let lights = self.onboard.lights.clone();
        for (zone, lighting) in lights {
            self.backend.send(Command::SetOnboardLighting {
                device: id.clone(),
                profile,
                zone,
                lighting,
            });
        }
    }

    pub fn save_onboard_button(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        let Some(button) = self.onboard.editing else {
            return;
        };
        let Some(action) = self.onboard.pending_action.clone() else {
            self.show_toast("Choose what the button should do first.".into(), true, cx);
            return;
        };
        if button == 0x01 {
            self.show_toast(
                "Left click cannot be changed, so the mouse stays usable.".into(),
                true,
                cx,
            );
            return;
        }
        let Some(data) = &self.onboard.data else {
            return;
        };
        if !data.caps.writable() {
            self.show_toast(
                "Saving to this mouse is off. Turn on experimental saving in Settings.".into(),
                true,
                cx,
            );
            return;
        }
        let profile = self.onboard.profile;
        self.backend.send(Command::SetOnboardButton {
            device: id,
            profile,
            button,
            action,
        });
        self.onboard.editing = None;
        cx.notify();
    }

    pub fn save_onboard_stages(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        let Some(data) = &self.onboard.data else {
            return;
        };
        if !data.caps.writable() {
            self.show_toast(
                "Saving to this mouse is off. Turn on experimental saving in Settings.".into(),
                true,
                cx,
            );
            return;
        }
        if self.performance.stages.is_empty() {
            return;
        }
        let stages = rusted_snake_core::DpiStages {
            active: self
                .performance
                .active_stage
                .min(self.performance.stages.len() - 1),
            dpi: self.performance.stages.clone(),
        };
        self.backend.send(Command::SetOnboardDpiStages {
            device: id,
            profile: self.onboard.profile,
            stages,
        });
    }

    pub fn set_zone_brightness(
        &mut self,
        zone: rusted_snake_core::ZoneId,
        value: f64,
        cx: &mut Context<Self>,
    ) {
        let Some(device) = self.selected.clone() else {
            return;
        };
        self.lighting
            .zone_brightness
            .insert(zone.interface.clone(), value);
        self.backend.send(Command::SetZoneBrightness {
            device,
            zone,
            value,
        });
        cx.notify();
    }

    pub fn apply_zone_effect(
        &mut self,
        zone: rusted_snake_core::ZoneId,
        draft: ZoneDraft,
        cx: &mut Context<Self>,
    ) {
        let Some(device) = self.selected.clone() else {
            return;
        };
        self.backend.send(Command::SetZoneEffect {
            device,
            zone,
            effect: ZoneEffect {
                kind: draft.effect,
                colors: draft.colors.to_vec(),
                speed: draft.speed,
                direction: draft.direction,
            },
        });
        cx.notify();
    }

    pub fn reapply_onboard_lighting(&mut self, _cx: &mut Context<Self>) {
        let Some(data) = self.onboard.data.as_ref() else {
            return;
        };
        if !data.caps.writable() || self.onboard.profile != data.active_profile {
            return;
        }
        let Some(device) = self.selected.clone() else {
            return;
        };
        let profile = self.onboard.profile;
        let lights = self.onboard.lights.clone();
        for (zone, lighting) in lights {
            self.backend.send(Command::SetOnboardLighting {
                device: device.clone(),
                profile,
                zone,
                lighting,
            });
        }
    }

    pub fn apply_software_effect(
        &mut self,
        settings: SoftwareEffectSettings,
        cx: &mut Context<Self>,
    ) {
        let Some(device) = self.selected.clone() else {
            return;
        };
        self.settings.software = settings.clone();
        let _ = self.settings.save();
        if settings.effect == SoftwareEffect::None {
            self.software_active.remove(&device);
        } else {
            self.software_active.insert(device.clone(), settings.effect);
        }
        self.backend
            .send(Command::ApplySoftwareEffect { device, settings });
        cx.notify();
    }

    pub fn set_brightness(&mut self, value: f64, cx: &mut Context<Self>) {
        let Some(device) = self.selected.clone() else {
            return;
        };
        self.lighting.brightness = Some(value);
        self.backend.send(Command::SetBrightness { device, value });
        cx.notify();
    }

    pub fn set_dpi(&mut self, value: i32, cx: &mut Context<Self>) {
        let Some(device) = self.selected.clone() else {
            return;
        };
        self.performance.dpi = Some(value);
        self.backend.send(Command::SetDpi {
            device,
            x: value,
            y: value,
        });
        cx.notify();
    }

    pub fn set_poll_rate(&mut self, rate: u32, cx: &mut Context<Self>) {
        let Some(device) = self.selected.clone() else {
            return;
        };
        self.performance.poll_rate = Some(rate);
        self.backend.send(Command::SetPollRate { device, rate });
        cx.notify();
    }

    pub fn set_experimental(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.settings.experimental_onboard = enabled;
        self.settings_ui.experimental = enabled;
        let _ = self.settings.save();
        self.backend.send(Command::SetExperimental {
            experimental: enabled,
        });
        cx.notify();
    }

    pub fn set_autostart(&mut self, enabled: bool, cx: &mut Context<Self>) {
        match rusted_snake_core::settings::set_autostart(enabled) {
            Ok(()) => {
                self.settings_ui.autostart = enabled;
                self.settings.start_with_computer = enabled;
                let _ = self.settings.save();
                self.show_toast(
                    if enabled {
                        "Rusted Snake will start when you sign in.".into()
                    } else {
                        "Rusted Snake will not start when you sign in.".into()
                    },
                    false,
                    cx,
                );
            }
            Err(error) => self.show_toast(format!("Could not change startup: {error}"), true, cx),
        }
    }

    pub fn backup_now(&mut self, cx: &mut Context<Self>) {
        let Some(device) = self.selected.clone() else {
            return;
        };
        self.backend
            .send(Command::BackupOnboard { device, path: None });
        cx.notify();
    }

    pub fn restore_backup(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        let Some(device) = self.selected.clone() else {
            return;
        };
        self.backend.send(Command::RestoreOnboard { device, path });
        cx.notify();
    }

    pub fn reload_macro_engine(&mut self) {
        let macros = self.macros.active().macros.clone();
        self.backend.send(Command::ReloadMacros(macros));
    }

    pub fn save_macros(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.macros.save() {
            self.show_toast(format!("Could not save macros: {error}"), true, cx);
            return;
        }
        self.reload_macro_engine();
    }

    pub fn open_macro_editor(
        &mut self,
        existing: Option<u64>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected.is_none() {
            self.show_toast("Select a device before adding a macro.".into(), true, cx);
            return;
        }
        let entry = existing.and_then(|id| {
            self.macros
                .active()
                .macros
                .iter()
                .find(|entry| entry.id == id)
                .cloned()
        });
        let mut editor = MacroEditor {
            id: entry.as_ref().map(|entry| entry.id),
            kind: MacroKind::Key,
            key: None,
            key_modifiers: keymap::Modifiers::default(),
            trigger: entry.as_ref().and_then(|entry| entry.trigger),
            repeat: entry
                .as_ref()
                .map(|entry| entry.repeat)
                .unwrap_or(RepeatMode::Once),
            capturing_key: false,
            capturing_trigger: false,
        };
        let (name, action) = match &entry {
            Some(entry) => (entry.name.clone(), entry.action.clone()),
            None => (
                String::new(),
                MacroAction::Key {
                    code: 0,
                    modifiers: keymap::Modifiers::default(),
                },
            ),
        };
        self.fields.entry(FieldId::MacroName).or_default().set(name);
        match action {
            MacroAction::Key { code, modifiers } => {
                editor.kind = MacroKind::Key;
                editor.key = Some(code);
                editor.key_modifiers = modifiers;
            }
            MacroAction::Text { text } => {
                editor.kind = MacroKind::Text;
                self.fields.entry(FieldId::MacroText).or_default().set(text);
            }
            MacroAction::Command { command } => {
                editor.kind = MacroKind::Command;
                self.fields
                    .entry(FieldId::MacroCommand)
                    .or_default()
                    .set(command);
            }
        }
        self.macro_ui.editor = Some(editor);
        self.macro_ui.confirm_delete_macro = None;
        if let Some(handle) = self.field_focus.get(&FieldId::MacroName).cloned() {
            window.focus(&handle);
        }
        cx.notify();
    }

    pub fn save_macro_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &self.macro_ui.editor else {
            return;
        };
        let Some(device) = self.selected.clone() else {
            return;
        };
        let name = self
            .fields
            .get(&FieldId::MacroName)
            .map(|field| field.value.trim().to_string())
            .unwrap_or_default();
        let action = match editor.kind {
            MacroKind::Key => match editor.key {
                Some(code) if code > 0 => MacroAction::Key {
                    code,
                    modifiers: editor.key_modifiers,
                },
                _ => {
                    self.show_toast("Press the key the macro should press.".into(), true, cx);
                    return;
                }
            },
            MacroKind::Text => {
                let text = self
                    .fields
                    .get(&FieldId::MacroText)
                    .map(|field| field.value.clone())
                    .unwrap_or_default();
                if text.is_empty() {
                    self.show_toast("Type the text the macro should type.".into(), true, cx);
                    return;
                }
                MacroAction::Text { text }
            }
            MacroKind::Command => {
                let command = self
                    .fields
                    .get(&FieldId::MacroCommand)
                    .map(|field| field.value.trim().to_string())
                    .unwrap_or_default();
                if command.is_empty() {
                    self.show_toast("Type the command the macro should run.".into(), true, cx);
                    return;
                }
                MacroAction::Command { command }
            }
        };
        if editor.trigger.is_none() {
            self.show_toast("Choose a button that runs the macro.".into(), true, cx);
            return;
        }
        let entry = MacroDef {
            id: editor.id.unwrap_or(0),
            device: Some(device),
            name: if name.is_empty() {
                format!("Macro {}", self.macros.active().macros.len() + 1)
            } else {
                name
            },
            trigger: editor.trigger,
            action,
            repeat: editor.repeat,
        };
        if entry.id == 0 {
            self.macros.add_macro(entry);
        } else {
            self.macros.replace_macro(entry);
        }
        self.macro_ui.editor = None;
        self.save_macros(cx);
    }

    pub fn delete_macro(&mut self, id: u64, cx: &mut Context<Self>) {
        self.macros.remove_macro(id);
        self.macro_ui.editor = None;
        self.save_macros(cx);
    }

    pub fn open_color_picker(&mut self, target: ColorTarget, color: Rgb, cx: &mut Context<Self>) {
        let (hue, sat, val) = color.to_hsv();
        self.lighting.picker = Some(ColorPickerUi {
            target,
            hue,
            sat,
            val,
        });
        cx.notify();
    }

    pub fn apply_color_picker(&mut self, cx: &mut Context<Self>) {
        let Some(picker) = self.lighting.picker.clone() else {
            return;
        };
        let color = picker.color();
        match picker.target {
            ColorTarget::Zone(interface, index) => {
                if let Some(draft) = self.lighting.zones.get_mut(&interface) {
                    draft.colors[index.min(1)] = color;
                }
                if let Some(zone) = self
                    .current_device()
                    .and_then(|device| {
                        device
                            .zones
                            .iter()
                            .find(|zone| zone.id.interface == interface)
                    })
                    .cloned()
                {
                    let draft = self
                        .lighting
                        .zones
                        .get(&interface)
                        .cloned()
                        .unwrap_or_default();
                    self.apply_zone_effect(zone.id, draft, cx);
                }
            }
            ColorTarget::Onboard(zone, _) => {
                let led = color.to_led();
                if let Some(lighting) = self.onboard.lights.get_mut(&zone) {
                    lighting.colors = vec![led];
                    if lighting.effect == OnboardLighting::OFF {
                        lighting.effect = OnboardLighting::STATIC;
                    }
                }
            }
            ColorTarget::SoftwareColor(index) => {
                if let Some(slot) = self.settings.software.colors.get_mut(index) {
                    *slot = color;
                }
                let settings = self.settings.software.clone();
                self.apply_software_effect(settings, cx);
            }
            ColorTarget::AudioColor(index) => {
                if let Some(slot) = self.settings.software.audio_colors.get_mut(index) {
                    *slot = color;
                }
                let settings = self.settings.software.clone();
                self.apply_software_effect(settings, cx);
            }
        }
        self.lighting.picker = None;
        cx.notify();
    }

    pub fn on_field_key(&mut self, field: FieldId, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let state = self.fields.entry(field).or_default();
        let key = event.keystroke.key.as_str();
        match key {
            "backspace" => {
                if state.caret > 0 {
                    let previous = prev_boundary(&state.value, state.caret);
                    state.value.replace_range(previous..state.caret, "");
                    state.caret = previous;
                }
            }
            "delete" => {
                if state.caret < state.value.len() {
                    let next = next_boundary(&state.value, state.caret);
                    state.value.replace_range(state.caret..next, "");
                }
            }
            "left" => state.caret = prev_boundary(&state.value, state.caret),
            "right" => state.caret = next_boundary(&state.value, state.caret),
            "home" => state.caret = 0,
            "end" => state.caret = state.value.len(),
            _ => {
                let modifiers = event.keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform {
                    return;
                }
                if let Some(character) = event
                    .keystroke
                    .key_char
                    .as_ref()
                    .and_then(|text| text.chars().next())
                {
                    state.value.insert(state.caret, character);
                    state.caret += character.len_utf8();
                }
            }
        }
        cx.notify();
    }

    pub fn on_capture_key(
        &mut self,
        target: CaptureTarget,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        if matches!(
            key,
            "shift" | "control" | "alt" | "super" | "meta" | "capslock"
        ) {
            return;
        }
        let modifiers = keymap::Modifiers {
            ctrl: event.keystroke.modifiers.control,
            shift: event.keystroke.modifiers.shift,
            alt: event.keystroke.modifiers.alt,
            super_key: event.keystroke.modifiers.platform,
        };
        match target {
            CaptureTarget::OnboardKey => {
                if let Some((mods, usage)) = keymap::key_to_hid(key, modifiers) {
                    let action = OnboardAction::key(mods, usage);
                    self.onboard.pending_action = Some(action.clone());
                    self.onboard.action_kind = ActionKind::Key;
                    self.capture = None;
                }
            }
            CaptureTarget::MacroKey => {
                if let Some(code) = keymap::key_to_evdev(key) {
                    if let Some(editor) = &mut self.macro_ui.editor {
                        editor.key = Some(code);
                        editor.key_modifiers = modifiers;
                        editor.kind = MacroKind::Key;
                    }
                    self.capture = None;
                }
            }
            CaptureTarget::MacroTrigger => {
                if let Some(code) = keymap::key_to_evdev(key) {
                    if let Some(editor) = &mut self.macro_ui.editor {
                        editor.trigger = Some(code);
                    }
                    self.capture = None;
                }
            }
        }
        cx.notify();
    }

    pub fn field_element(
        &self,
        field: FieldId,
        placeholder: &'static str,
        mono: bool,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let handle = self
            .field_focus
            .get(&field)
            .cloned()
            .unwrap_or_else(|| cx.focus_handle());
        let state = self.fields.get(&field).cloned().unwrap_or_default();
        let focused = handle.is_focused(window);
        widgets::text_field(widgets::TextField {
            value: &state.value,
            placeholder,
            caret: state.caret,
            focused,
            mono,
            height: 38.0,
        })
        .track_focus(&handle)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, window, cx| {
                window.focus(&this.field_focus[&field]);
                cx.notify();
            }),
        )
        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
            this.on_field_key(field, event, cx);
        }))
        .flex_1()
    }

    pub fn capture_element(
        &self,
        target: CaptureTarget,
        label: impl Into<SharedString>,
        _window: &Window,
        cx: &mut Context<Self>,
    ) -> Div {
        let focused = self.capture == Some(target);
        let text = if focused {
            "Press any key...".to_string()
        } else {
            label.into().to_string()
        };
        div()
            .flex()
            .items_center()
            .justify_center()
            .h(px(40.))
            .w_full()
            .rounded(px(8.))
            .bg(theme::chrome())
            .border_1()
            .border_color(if focused {
                theme::accent_soft(0.65)
            } else {
                theme::border()
            })
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if focused {
                theme::accent()
            } else {
                theme::text()
            })
            .cursor(CursorStyle::IBeam)
            .track_focus(&self.capture_focus)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    this.capture = Some(target);
                    window.focus(&this.capture_focus);
                    cx.notify();
                }),
            )
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                this.on_capture_key(target, event, window, cx);
            }))
            .child(text)
    }

    pub fn show_toast(&mut self, message: String, error: bool, cx: &mut Context<Self>) {
        self.toast_seq += 1;
        let seq = self.toast_seq;
        self.toast = Some(Toast {
            message,
            error,
            created: Instant::now(),
        });
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(6)).await;
            let _ = this.update(cx, |root, cx| {
                if root.toast_seq == seq {
                    root.toast = None;
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub fn lighting_brightness_from_settings(&mut self) {
        self.lighting.brightness = None;
    }

    pub fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.page {
            Page::Devices => self.devices_page(window, cx),
            Page::Buttons => self.buttons_page(window, cx),
            Page::Performance => self.performance_page(window, cx),
            Page::Lighting => self.lighting_page(window, cx),
            Page::Macros => self.macros_page(window, cx),
            Page::Settings => self.settings_page(window, cx),
        };
        let toast = self.toast.as_ref().map(toast_element);
        let picker = self.color_picker_element(window, cx);
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme::backdrop())
            .font_family("Barlow")
            .text_color(theme::text())
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let key = event.keystroke.key.as_str();
                if key == "f11" {
                    window.toggle_fullscreen();
                    return;
                }
                if key == "escape" {
                    if this.lighting.picker.is_some() {
                        this.lighting.picker = None;
                    } else if this.onboard.editing.is_some() {
                        this.onboard.editing = None;
                        this.capture = None;
                    } else if this.macro_ui.editor.is_some() {
                        this.macro_ui.editor = None;
                        this.capture = None;
                    }
                    cx.notify();
                    return;
                }
                if event.keystroke.modifiers.control {
                    if let Some(index) = key
                        .chars()
                        .next()
                        .and_then(|character| character.to_digit(10))
                        .filter(|index| (1..=6).contains(index))
                    {
                        this.page = Page::ALL[index as usize - 1];
                        cx.notify();
                    } else if key == "q" {
                        cx.quit();
                    }
                }
            }))
            .child(div().h(px(2.)).w_full().bg(gpui::linear_gradient(
                90.0,
                gpui::linear_color_stop(theme::accent_soft(0.9), 0.0),
                gpui::linear_color_stop(theme::accent_soft(0.0), 0.55),
            )))
            .child(self.top_bar(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(self.sidebar(cx))
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .id("page-scroll")
                            .overflow_y_scroll()
                            .child(content),
                    ),
            )
            .children(picker)
            .children(if self.page == Page::Buttons {
                self.button_drawer(window, cx)
            } else {
                None
            })
            .children(toast)
    }

    fn top_bar(&self, cx: &mut Context<Self>) -> Div {
        let status_color = if self.backend_info.connected {
            theme::accent()
        } else {
            theme::danger()
        };
        let status_text = if self.backend_info.connected {
            format!("OPENRAZER {}", self.backend_info.version)
        } else {
            "OPENRAZER OFFLINE".to_string()
        };
        let mut device_chip = div().flex().items_center().gap_2();
        if let Some(device) = self.current_device() {
            device_chip = device_chip
                .child(widgets::icon(device.kind.icon(), 14.0, theme::muted()))
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme::muted())
                        .child(device.display_name.clone()),
                );
            if let Some(battery) = &device.battery {
                device_chip = device_chip.child(widgets::pill(
                    format!(
                        "{:.0}%{}",
                        battery.percent,
                        if battery.charging { " ↑" } else { "" }
                    ),
                    if battery.percent < 20.0 {
                        theme::danger()
                    } else {
                        theme::accent()
                    },
                ));
            }
        }
        div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(54.))
            .px(px(20.))
            .bg(theme::chrome())
            .border_b_1()
            .border_color(theme::border())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(widgets::logo(23.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::text())
                                    .child("RUSTED SNAKE"),
                            )
                            .child(
                                div()
                                    .text_size(px(8.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(theme::dim())
                                    .child("DEVICE CONFIGURATOR"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(device_chip)
                    .child(div().w(px(1.)).h(px(20.)).bg(theme::border()))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(widgets::dot(status_color))
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::muted())
                                    .child(status_text),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(30.))
                            .rounded(px(8.))
                            .bg(theme::surface_raised())
                            .border_1()
                            .border_color(theme::border())
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::surface_hover()))
                            .on_press(cx.listener(|this, _, _, cx| {
                                this.backend.send(Command::RefreshDevices);
                                this.show_toast("Refreshing devices...".into(), false, cx);
                            }))
                            .child(widgets::icon("icons/refresh.svg", 14.0, theme::muted())),
                    ),
            )
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> Div {
        let mut nav = div().flex().flex_col().gap_1();
        for page in Page::ALL {
            let active = self.page == page;
            let mut item = div()
                .flex()
                .items_center()
                .gap_3()
                .h(px(38.))
                .px(px(11.))
                .rounded(px(8.))
                .cursor_pointer()
                .text_size(px(12.5))
                .font_weight(if active {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(if active {
                    theme::text()
                } else {
                    theme::muted()
                })
                .bg(if active {
                    theme::surface_raised()
                } else {
                    gpui::transparent_black()
                })
                .hover(|style| style.bg(theme::surface()).text_color(theme::text()))
                .on_press(cx.listener(move |this, _, _, cx| this.set_page(page, cx)));
            if active {
                item = item
                    .border_1()
                    .border_color(theme::border())
                    .child(widgets::accent_bar())
                    .child(widgets::icon(page.icon(), 16.0, theme::accent()))
                    .child(page.label());
            } else {
                item = item
                    .child(widgets::icon(page.icon(), 16.0, theme::muted()))
                    .child(page.label());
            }
            nav = nav.child(item);
        }

        let mut device_list = div().flex().flex_col().gap_1();
        if self.devices.is_empty() {
            device_list = device_list.child(
                div()
                    .p(px(10.))
                    .rounded(px(8.))
                    .bg(theme::surface())
                    .text_size(px(11.))
                    .text_color(theme::dim())
                    .child("No devices found"),
            );
        } else {
            for device in &self.devices {
                let active = self.selected.as_deref() == Some(device.id.as_str());
                let id = device.id.clone();
                device_list = device_list.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px(px(9.))
                        .py(px(8.))
                        .rounded(px(8.))
                        .bg(if active {
                            theme::surface_raised()
                        } else {
                            gpui::transparent_black()
                        })
                        .border_1()
                        .border_color(if active {
                            theme::border()
                        } else {
                            gpui::transparent_black()
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::surface()))
                        .on_press(
                            cx.listener(move |this, _, _, cx| this.select_device(id.clone(), cx)),
                        )
                        .child(widgets::icon(
                            device.kind.icon(),
                            15.0,
                            if active {
                                theme::accent()
                            } else {
                                theme::muted()
                            },
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .min_w_0()
                                .child(
                                    div()
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(if active {
                                            theme::text()
                                        } else {
                                            theme::muted()
                                        })
                                        .overflow_hidden()
                                        .child(device.display_name.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.5))
                                        .text_color(theme::dim())
                                        .child(device.type_label()),
                                ),
                        ),
                );
            }
        }

        let macro_color = match self.macro_ui.status.state {
            EngineState::Running => theme::accent(),
            EngineState::NeedsAccess => theme::warning(),
            EngineState::Error => theme::danger(),
            EngineState::Stopped => theme::dim(),
        };
        let macro_text = match self.macro_ui.status.state {
            EngineState::Running => "Macros active",
            EngineState::NeedsAccess => "Macros need access",
            EngineState::Error => "Macros stopped",
            EngineState::Stopped => "Macros idle",
        };

        div()
            .flex()
            .flex_col()
            .w(px(226.))
            .h_full()
            .p(px(14.))
            .gap_4()
            .bg(theme::chrome())
            .border_r_1()
            .border_color(theme::border())
            .child(nav)
            .child(widgets::divider())
            .child(widgets::section_label("MY DEVICES"))
            .child(device_list)
            .child(widgets::spacer())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p(px(10.))
                    .rounded(px(9.))
                    .bg(theme::surface())
                    .border_1()
                    .border_color(theme::border())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(widgets::dot(if self.backend_info.connected {
                                theme::accent()
                            } else {
                                theme::danger()
                            }))
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::muted())
                                    .child(if self.backend_info.connected {
                                        "OPENRAZER READY"
                                    } else {
                                        "OPENRAZER OFFLINE"
                                    }),
                            ),
                    )
                    .child(div().text_size(px(9.5)).text_color(theme::dim()).child(
                        if self.backend_info.connected {
                            format!("Daemon {}", self.backend_info.version)
                        } else {
                            "Start openrazer-daemon".into()
                        },
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px(px(4.))
                    .child(widgets::dot(macro_color))
                    .child(
                        div()
                            .text_size(px(9.5))
                            .text_color(theme::dim())
                            .child(macro_text),
                    ),
            )
    }

    pub(crate) fn page_header(
        &self,
        title: &str,
        subtitle: impl Into<SharedString>,
        icon: &'static str,
    ) -> Div {
        div()
            .flex()
            .items_start()
            .justify_between()
            .w_full()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(42.))
                            .rounded(px(12.))
                            .bg(theme::accent_soft(0.10))
                            .border_1()
                            .border_color(theme::accent_soft(0.24))
                            .child(widgets::icon(icon, 20.0, theme::accent())),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(px(24.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::text())
                                    .child(title.to_string()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(theme::muted())
                                    .child(subtitle.into()),
                            ),
                    ),
            )
    }

    pub fn color_picker_element(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let picker = self.lighting.picker.clone()?;
        let color = picker.color();
        let weak = cx.entity().downgrade();
        let hue = picker.hue;
        let sat = picker.sat;
        let val = picker.val;
        let hue_cb = weak.clone();
        let sat_cb = weak.clone();
        let val_cb = weak.clone();
        let target = picker.target.clone();
        let presets = [
            Rgb::RAZER_GREEN,
            Rgb::RED,
            Rgb::BLUE,
            Rgb::WHITE,
            Rgb::AMBER,
            Rgb::VIOLET,
            Rgb::new(0xff, 0x4f, 0xc4),
            Rgb::new(0x00, 0xd4, 0xcf),
        ];
        let mut preset_row = div().flex().items_center().gap_2();
        for preset in presets {
            let weak = weak.clone();
            let preset_color = preset;
            preset_row = preset_row.child(
                widgets::swatch(preset, color.distance(preset) < 8.0).on_press(cx.listener(
                    move |this, _, _, cx| {
                        if let Some(picker) = &mut this.lighting.picker {
                            let (h, s, v) = preset_color.to_hsv();
                            picker.hue = h;
                            picker.sat = s;
                            picker.val = v;
                        }
                        cx.notify();
                    },
                )),
            );
            let _ = weak;
        }
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme::with_alpha(0x000000, 0.62))
                .child(
                    div()
                        .w(px(380.))
                        .p(px(22.))
                        .rounded(px(16.))
                        .bg(theme::surface())
                        .border_1()
                        .border_color(theme::border_strong())
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .child(widgets::heading("Custom colour"))
                                .child(widgets::tiny_icon_button("icons/close.svg").on_press(
                                    cx.listener(|this, _, _, cx| {
                                        this.lighting.picker = None;
                                        cx.notify();
                                    }),
                                )),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_3()
                                .child(widgets::color_preview(color).size(px(44.)))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .child(widgets::mono_value(color.to_hex()))
                                        .child(widgets::hint(format!(
                                            "Hue {:.0}° · Saturation {:.0}% · Brightness {:.0}%",
                                            hue,
                                            sat * 100.0,
                                            val * 100.0
                                        ))),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_3()
                                .child(widgets::label("Hue"))
                                .child(hue_strip())
                                .child(widgets::slider(
                                    hue,
                                    0.0,
                                    360.0,
                                    1.0,
                                    theme::accent(),
                                    move |value, _, cx| {
                                        let _ = hue_cb.update(cx, |root, cx| {
                                            if let Some(picker) = &mut root.lighting.picker {
                                                picker.hue = value;
                                            }
                                            cx.notify();
                                        });
                                    },
                                ))
                                .child(widgets::label("Saturation"))
                                .child(gradient_strip(
                                    Rgb::WHITE,
                                    Rgb::from_hsv(hue, 1.0, val.max(0.15)),
                                ))
                                .child(widgets::slider(
                                    sat * 100.0,
                                    0.0,
                                    100.0,
                                    1.0,
                                    theme::accent(),
                                    move |value, _, cx| {
                                        let _ = sat_cb.update(cx, |root, cx| {
                                            if let Some(picker) = &mut root.lighting.picker {
                                                picker.sat = value / 100.0;
                                            }
                                            cx.notify();
                                        });
                                    },
                                ))
                                .child(widgets::label("Brightness"))
                                .child(gradient_strip(
                                    Rgb::new(0, 0, 0),
                                    Rgb::from_hsv(hue, sat, 1.0),
                                ))
                                .child(widgets::slider(
                                    val * 100.0,
                                    0.0,
                                    100.0,
                                    1.0,
                                    theme::accent(),
                                    move |value, _, cx| {
                                        let _ = val_cb.update(cx, |root, cx| {
                                            if let Some(picker) = &mut root.lighting.picker {
                                                picker.val = value / 100.0;
                                            }
                                            cx.notify();
                                        });
                                    },
                                )),
                        )
                        .child(widgets::section_label("PRESETS"))
                        .child(preset_row)
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap_2()
                                .child(widgets::ghost_button("Cancel").on_press(cx.listener(
                                    |this, _, _, cx| {
                                        this.lighting.picker = None;
                                        cx.notify();
                                    },
                                )))
                                .child(widgets::primary_button("Apply").on_press(cx.listener(
                                    |this, _, _, cx| {
                                        let _ = target;
                                        this.apply_color_picker(cx);
                                    },
                                ))),
                        ),
                )
                .into_any_element(),
        )
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Root::render(self, window, cx)
    }
}

fn gradient_strip(from: Rgb, to: Rgb) -> Div {
    div()
        .h(px(8.))
        .w_full()
        .rounded_full()
        .overflow_hidden()
        .bg(gpui::linear_gradient(
            90.0,
            gpui::linear_color_stop(theme::from_rgb(from), 0.0),
            gpui::linear_color_stop(theme::from_rgb(to), 1.0),
        ))
}

fn hue_strip() -> Div {
    let stops = [
        Rgb::from_hsv(0.0, 1.0, 1.0),
        Rgb::from_hsv(60.0, 1.0, 1.0),
        Rgb::from_hsv(120.0, 1.0, 1.0),
        Rgb::from_hsv(180.0, 1.0, 1.0),
        Rgb::from_hsv(240.0, 1.0, 1.0),
        Rgb::from_hsv(300.0, 1.0, 1.0),
        Rgb::from_hsv(360.0, 1.0, 1.0),
    ];
    let mut strip = div()
        .flex()
        .h(px(8.))
        .w_full()
        .rounded_full()
        .overflow_hidden();
    for index in 0..6 {
        strip = strip.child(div().flex_1().h_full().bg(gpui::linear_gradient(
            90.0,
            gpui::linear_color_stop(theme::from_rgb(stops[index]), 0.0),
            gpui::linear_color_stop(theme::from_rgb(stops[index + 1]), 1.0),
        )));
    }
    strip
}

fn parse_effect(name: &str) -> Option<EffectKind> {
    match name.to_ascii_lowercase().as_str() {
        "none" | "off" => Some(EffectKind::Off),
        "static" => Some(EffectKind::Static),
        "spectrum" => Some(EffectKind::Spectrum),
        "breath_single" | "breath" => Some(EffectKind::BreathSingle),
        "breath_dual" => Some(EffectKind::BreathDual),
        "breath_random" => Some(EffectKind::BreathRandom),
        "reactive" => Some(EffectKind::Reactive),
        "wave" => Some(EffectKind::Wave),
        "starlight_single" => Some(EffectKind::StarlightSingle),
        "starlight_dual" => Some(EffectKind::StarlightDual),
        "starlight_random" => Some(EffectKind::StarlightRandom),
        "pulsate" => Some(EffectKind::Pulsate),
        "ripple" => Some(EffectKind::Ripple),
        "ripple_random" => Some(EffectKind::RippleRandom),
        _ => None,
    }
}

fn prev_boundary(value: &str, caret: usize) -> usize {
    value[..caret.min(value.len())]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_boundary(value: &str, caret: usize) -> usize {
    let caret = caret.min(value.len());
    value[caret..]
        .char_indices()
        .nth(1)
        .map(|(index, _)| caret + index)
        .unwrap_or(value.len())
}

fn toast_element(toast: &Toast) -> AnyElement {
    let color = if toast.error {
        theme::danger()
    } else {
        theme::accent()
    };
    div()
        .absolute()
        .bottom(px(22.))
        .right(px(22.))
        .flex()
        .items_center()
        .gap_3()
        .max_w(px(440.))
        .px(px(16.))
        .py(px(12.))
        .rounded(px(11.))
        .bg(theme::surface_raised())
        .border_1()
        .border_color(theme::border_strong())
        .shadow_lg()
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(26.))
                .rounded(px(7.))
                .bg(theme::with_alpha(0xffffff, 0.04))
                .child(widgets::icon(
                    if toast.error {
                        "icons/alert.svg"
                    } else {
                        "icons/check.svg"
                    },
                    14.0,
                    color,
                )),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color)
                        .child(if toast.error {
                            "SOMETHING WENT WRONG"
                        } else {
                            "DONE"
                        }),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(theme::text())
                        .child(toast.message.clone()),
                ),
        )
        .into_any_element()
}

pub fn open(cx: &mut App) {
    open_page(cx, Page::Devices);
}

pub fn open_page(cx: &mut App, page: Page) {
    let bounds = gpui::Bounds::centered(None, gpui::size(px(1280.), px(820.)), cx);
    cx.open_window(
        gpui::WindowOptions {
            window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
            window_min_size: Some(gpui::size(px(1040.), px(700.))),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("Rusted Snake".into()),
                ..Default::default()
            }),
            app_id: Some("com.henrychea.RustedSnake".into()),
            ..Default::default()
        },
        |_, cx| {
            cx.new(|cx| {
                let mut root = Root::new(cx);
                root.page = page;
                root
            })
        },
    )
    .expect("could not open the Rusted Snake window");
    cx.activate(true);
}
