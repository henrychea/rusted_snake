// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rusted_snake_core::keymap::{Modifiers, char_to_evdev};
use rusted_snake_core::macros::{MacroAction, MacroDef, RepeatMode};

use crossbeam_channel::{Receiver, Sender, unbounded};

const EV_SYN: u16 = 0;
const EV_KEY: u16 = 1;
const EV_REL: u16 = 2;
const EV_MSC: u16 = 4;
const EV_LED: u16 = 17;
const SYN_REPORT: u16 = 0;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EngineState {
    Stopped,
    Running,
    NeedsAccess,
    Error,
}

#[derive(Clone, Debug)]
pub struct EngineStatus {
    pub state: EngineState,
    pub message: String,
    pub devices: Vec<String>,
}

impl Default for EngineStatus {
    fn default() -> Self {
        Self {
            state: EngineState::Stopped,
            message: "Macros are not running.".into(),
            devices: Vec::new(),
        }
    }
}

enum EngineCommand {
    Reload(Vec<MacroDef>),
    Stop,
}

pub struct MacroEngine {
    commands: Sender<EngineCommand>,
    status: Arc<Mutex<EngineStatus>>,
    thread: Option<JoinHandle<()>>,
}

impl MacroEngine {
    pub fn start() -> Self {
        let (commands, receiver) = unbounded();
        let status = Arc::new(Mutex::new(EngineStatus::default()));
        let thread_status = status.clone();
        let thread = thread::spawn(move || engine_loop(receiver, thread_status));
        Self {
            commands,
            status,
            thread: Some(thread),
        }
    }

    pub fn reload(&self, macros: Vec<MacroDef>) {
        let _ = self.commands.send(EngineCommand::Reload(macros));
    }

    pub fn status(&self) -> EngineStatus {
        self.status
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn stop(&self) {
        let _ = self.commands.send(EngineCommand::Stop);
    }
}

impl Drop for MacroEngine {
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Runtime {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl Runtime {
    fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

fn engine_loop(receiver: Receiver<EngineCommand>, status: Arc<Mutex<EngineStatus>>) {
    let mut runtime: Option<Runtime> = None;
    for command in receiver {
        match command {
            EngineCommand::Reload(macros) => {
                if let Some(previous) = runtime.take() {
                    previous.stop();
                }
                runtime = start_runtime(macros, &status);
            }
            EngineCommand::Stop => break,
        }
    }
    if let Some(runtime) = runtime.take() {
        runtime.stop();
    }
    set_status(
        &status,
        EngineStatus {
            state: EngineState::Stopped,
            message: "Macros are stopped.".into(),
            devices: Vec::new(),
        },
    );
}

fn start_runtime(macros: Vec<MacroDef>, status: &Arc<Mutex<EngineStatus>>) -> Option<Runtime> {
    let macros: Vec<MacroDef> = macros
        .into_iter()
        .filter(|entry| entry.trigger.is_some())
        .collect();
    if macros.is_empty() {
        set_status(
            status,
            EngineStatus {
                state: EngineState::Stopped,
                message: "Add a macro to start the engine.".into(),
                devices: Vec::new(),
            },
        );
        return None;
    }

    let virtual_device = match VirtualDevice::open() {
        Ok(device) => Arc::new(device),
        Err(error) => {
            let message = if error.kind() == std::io::ErrorKind::PermissionDenied {
                "Macro engine needs access to /dev/uinput and /dev/input. Add your user to the input group and install the Rusted Snake udev rule, then sign in again.".to_string()
            } else {
                format!("Cannot open /dev/uinput: {error}")
            };
            let state = if error.kind() == std::io::ErrorKind::PermissionDenied {
                EngineState::NeedsAccess
            } else {
                EngineState::Error
            };
            set_status(
                status,
                EngineStatus {
                    state,
                    message,
                    devices: Vec::new(),
                },
            );
            return None;
        }
    };

    let stop = Arc::new(AtomicBool::new(false));
    let mut threads = Vec::new();
    let mut watched = Vec::new();
    let mut errors = Vec::new();
    let mut needs_access = false;

    for (path, name) in input_devices() {
        if !name.to_ascii_lowercase().contains("razer") {
            continue;
        }
        let relevant: Vec<MacroDef> = macros
            .iter()
            .filter(|entry| device_matches(entry.device.as_deref(), &name))
            .cloned()
            .collect();
        if relevant.is_empty() {
            continue;
        }
        match spawn_device_worker(
            path.clone(),
            name.clone(),
            relevant,
            virtual_device.clone(),
            stop.clone(),
        ) {
            Ok(thread) => {
                watched.push(name);
                threads.push(thread);
            }
            Err(error) => {
                needs_access |= error.kind() == std::io::ErrorKind::PermissionDenied;
                errors.push(format!("{}: {error}", path.display()));
            }
        }
    }

    if threads.is_empty() {
        let message = if errors.is_empty() {
            "No Razer input devices found. Connect a mouse, or check that your user can read /dev/input.".into()
        } else {
            format!("Cannot open a Razer input device. {}", errors.join("; "))
        };
        set_status(
            status,
            EngineStatus {
                state: if needs_access {
                    EngineState::NeedsAccess
                } else {
                    EngineState::Error
                },
                message,
                devices: Vec::new(),
            },
        );
        return None;
    }

    set_status(
        status,
        EngineStatus {
            state: EngineState::Running,
            message: format!(
                "Watching {} Razer input device{}.",
                watched.len(),
                if watched.len() == 1 { "" } else { "s" }
            ),
            devices: watched,
        },
    );
    Some(Runtime { stop, threads })
}

fn set_status(status: &Arc<Mutex<EngineStatus>>, value: EngineStatus) {
    if let Ok(mut lock) = status.lock() {
        *lock = value;
    }
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn device_matches(device: Option<&str>, input_name: &str) -> bool {
    match device {
        None => true,
        Some(device) => {
            let device = normalize(device);
            let input = normalize(input_name);
            !device.is_empty() && (input.contains(&device) || device.contains(&input))
        }
    }
}

fn input_devices() -> Vec<(PathBuf, String)> {
    let mut devices = Vec::new();
    let Ok(entries) = std::fs::read_dir("/dev/input") else {
        return devices;
    };
    let mut entries: Vec<_> = entries.filter_map(|entry| entry.ok()).collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("event") {
            continue;
        }
        let event_name = name.strip_prefix("event").unwrap_or("");
        let sysfs_name = Path::new("/sys/class/input")
            .join(&*name)
            .join("device/name");
        let Ok(display_name) = std::fs::read_to_string(&sysfs_name) else {
            continue;
        };
        if event_name.is_empty() {
            continue;
        }
        devices.push((entry.path(), display_name.trim().to_string()));
    }
    devices
}

fn spawn_device_worker(
    path: PathBuf,
    name: String,
    macros: Vec<MacroDef>,
    virtual_device: Arc<VirtualDevice>,
    stop: Arc<AtomicBool>,
) -> std::io::Result<JoinHandle<()>> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&path)?;
    let fd = file.as_raw_fd();
    let result = unsafe { libc::ioctl(fd, EVIOCGRAB, 1) };
    if result < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let mut triggers: HashMap<u16, Vec<MacroDef>> = HashMap::new();
    for entry in macros {
        if let Some(code) = entry.trigger {
            triggers.entry(code).or_default().push(entry);
        }
    }
    let triggers = Arc::new(triggers);
    let repeats: Arc<Mutex<HashMap<u64, Arc<AtomicBool>>>> = Arc::new(Mutex::new(HashMap::new()));
    let thread = thread::spawn(move || {
        let mut buffer = [0u8; 24 * 64];
        let mut held = Vec::new();
        while !stop.load(Ordering::Relaxed) {
            match (&file).read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    for chunk in buffer[..read].chunks_exact(24) {
                        let event = unsafe {
                            std::ptr::read_unaligned(chunk.as_ptr() as *const InputEvent)
                        };
                        if event.type_ == EV_KEY {
                            if event.value == 1 {
                                if let Some(entries) = triggers.get(&event.code) {
                                    held.push(event.code);
                                    for entry in entries {
                                        start_macro(entry, &virtual_device, &repeats);
                                    }
                                    continue;
                                }
                            } else if event.value == 0 && held.contains(&event.code) {
                                held.retain(|code| *code != event.code);
                                if let Some(entries) = triggers.get(&event.code) {
                                    for entry in entries {
                                        if entry.repeat == RepeatMode::WhileHeld {
                                            stop_repeat(&repeats, entry.id);
                                        }
                                    }
                                }
                                continue;
                            } else if event.value == 2 && held.contains(&event.code) {
                                continue;
                            }
                        }
                        virtual_device.emit(event.type_, event.code, event.value);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(4));
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => {
                    log::warn!("macro input read failed for {name}: {error}");
                    break;
                }
            }
        }
        let _ = unsafe { libc::ioctl(fd, EVIOCGRAB, 0) };
        for flag in repeats.lock().unwrap().values() {
            flag.store(false, Ordering::Relaxed);
        }
    });
    Ok(thread)
}

fn start_macro(
    entry: &MacroDef,
    virtual_device: &Arc<VirtualDevice>,
    repeats: &Arc<Mutex<HashMap<u64, Arc<AtomicBool>>>>,
) {
    match entry.repeat {
        RepeatMode::Once => {
            let action = entry.action.clone();
            let device = virtual_device.clone();
            thread::spawn(move || execute(&action, &device));
        }
        RepeatMode::WhileHeld => {
            stop_repeat(repeats, entry.id);
            let flag = Arc::new(AtomicBool::new(true));
            repeats.lock().unwrap().insert(entry.id, flag.clone());
            let action = entry.action.clone();
            let device = virtual_device.clone();
            let delay = Duration::from_millis(120);
            thread::spawn(move || {
                while flag.load(Ordering::Relaxed) {
                    execute(&action, &device);
                    thread::sleep(delay);
                }
            });
        }
        RepeatMode::Toggle => {
            let already_running = repeats
                .lock()
                .unwrap()
                .get(&entry.id)
                .is_some_and(|flag| flag.load(Ordering::Relaxed));
            if already_running {
                stop_repeat(repeats, entry.id);
            } else {
                let flag = Arc::new(AtomicBool::new(true));
                repeats.lock().unwrap().insert(entry.id, flag.clone());
                let action = entry.action.clone();
                let device = virtual_device.clone();
                let delay = Duration::from_millis(120);
                thread::spawn(move || {
                    while flag.load(Ordering::Relaxed) {
                        execute(&action, &device);
                        thread::sleep(delay);
                    }
                });
            }
        }
    }
}

fn stop_repeat(repeats: &Arc<Mutex<HashMap<u64, Arc<AtomicBool>>>>, id: u64) {
    if let Some(flag) = repeats.lock().unwrap().remove(&id) {
        flag.store(false, Ordering::Relaxed);
    }
}

fn execute(action: &MacroAction, device: &Arc<VirtualDevice>) {
    match action {
        MacroAction::Key { code, modifiers } => {
            let mods = modifier_codes(*modifiers);
            for modifier in &mods {
                device.emit(EV_KEY, *modifier, 1);
            }
            device.emit(EV_KEY, *code, 1);
            device.sync();
            thread::sleep(Duration::from_millis(18));
            device.emit(EV_KEY, *code, 0);
            for modifier in mods.iter().rev() {
                device.emit(EV_KEY, *modifier, 0);
            }
            device.sync();
        }
        MacroAction::Text { text } => {
            for character in text.chars() {
                let Some((code, shift)) = char_to_evdev(character) else {
                    continue;
                };
                if shift {
                    device.emit(EV_KEY, 42, 1);
                }
                device.emit(EV_KEY, code, 1);
                device.sync();
                thread::sleep(Duration::from_millis(10));
                device.emit(EV_KEY, code, 0);
                if shift {
                    device.emit(EV_KEY, 42, 0);
                }
                device.sync();
                thread::sleep(Duration::from_millis(6));
            }
        }
        MacroAction::Command { command } => {
            let _ = std::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
    }
}

fn modifier_codes(modifiers: Modifiers) -> Vec<u16> {
    let mut codes = Vec::new();
    if modifiers.ctrl {
        codes.push(29);
    }
    if modifiers.shift {
        codes.push(42);
    }
    if modifiers.alt {
        codes.push(56);
    }
    if modifiers.super_key {
        codes.push(125);
    }
    codes
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

#[repr(C)]
struct UinputUserDevice {
    name: [u8; 80],
    id: UinputId,
    ff_effects_max: u32,
    absmax: [i32; 64],
    absmin: [i32; 64],
    absfuzz: [i32; 64],
    absflat: [i32; 64],
}

#[repr(C)]
struct UinputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

const UI_DEV_CREATE: libc::c_ulong = 0x5501;
const UI_DEV_DESTROY: libc::c_ulong = 0x5502;
const UI_SET_EVBIT: libc::c_ulong = 0x40045564;
const UI_SET_KEYBIT: libc::c_ulong = 0x40045565;
const UI_SET_RELBIT: libc::c_ulong = 0x40045566;
const UI_SET_MSCBIT: libc::c_ulong = 0x40045568;
const UI_SET_LEDBIT: libc::c_ulong = 0x40045569;
const EVIOCGRAB: libc::c_ulong = 0x40044590;

struct VirtualDevice {
    file: Mutex<File>,
}

impl VirtualDevice {
    fn open() -> std::io::Result<Self> {
        let file = OpenOptions::new().write(true).open("/dev/uinput")?;
        let fd = file.as_raw_fd();
        unsafe {
            ioctl_arg(fd, UI_SET_EVBIT, EV_SYN as libc::c_ulong)?;
            ioctl_arg(fd, UI_SET_EVBIT, EV_KEY as libc::c_ulong)?;
            ioctl_arg(fd, UI_SET_EVBIT, EV_REL as libc::c_ulong)?;
            ioctl_arg(fd, UI_SET_EVBIT, EV_MSC as libc::c_ulong)?;
            ioctl_arg(fd, UI_SET_EVBIT, EV_LED as libc::c_ulong)?;
            for code in 0..=0x2ffu16 {
                ioctl_arg(fd, UI_SET_KEYBIT, code as libc::c_ulong)?;
            }
            for code in 0..=0x0fu16 {
                ioctl_arg(fd, UI_SET_RELBIT, code as libc::c_ulong)?;
            }
            for code in 0..=0x07u16 {
                ioctl_arg(fd, UI_SET_MSCBIT, code as libc::c_ulong)?;
            }
            for code in 0..=0x0fu16 {
                ioctl_arg(fd, UI_SET_LEDBIT, code as libc::c_ulong)?;
            }
        }
        let mut device = UinputUserDevice {
            name: [0u8; 80],
            id: UinputId {
                bustype: 0x06,
                vendor: 0x1532,
                product: 0x0001,
                version: 1,
            },
            ff_effects_max: 0,
            absmax: [0; 64],
            absmin: [0; 64],
            absfuzz: [0; 64],
            absflat: [0; 64],
        };
        let name = b"Rusted Snake Virtual Input";
        device.name[..name.len()].copy_from_slice(name);
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &device as *const UinputUserDevice as *const u8,
                std::mem::size_of::<UinputUserDevice>(),
            )
        };
        (&file).write_all(bytes)?;
        let result = unsafe { libc::ioctl(fd, UI_DEV_CREATE) };
        if result < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    fn emit(&self, event_type: u16, code: u16, value: i32) {
        let event = InputEvent {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            type_: event_type,
            code,
            value,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &event as *const InputEvent as *const u8,
                std::mem::size_of::<InputEvent>(),
            )
        };
        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(bytes);
        }
    }

    fn sync(&self) {
        self.emit(EV_SYN, SYN_REPORT, 0);
    }
}

impl Drop for VirtualDevice {
    fn drop(&mut self) {
        if let Ok(file) = self.file.lock() {
            let _ = unsafe { libc::ioctl(file.as_raw_fd(), UI_DEV_DESTROY) };
        }
    }
}

unsafe fn ioctl_arg(
    fd: i32,
    request: libc::c_ulong,
    argument: libc::c_ulong,
) -> std::io::Result<()> {
    let result = unsafe { libc::ioctl(fd, request, argument) };
    if result < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_names_match_and_normalize() {
        assert!(device_matches(
            Some("RazerNagaTrinity"),
            "Razer Naga Trinity"
        ));
        assert!(device_matches(
            Some("RazerNagaTrinity"),
            "Razer Naga Trinity Consumer Control"
        ));
        assert!(!device_matches(
            Some("RazerNagaTrinity"),
            "Razer BlackWidow"
        ));
        assert!(device_matches(None, "Razer BlackWidow"));
    }

    #[test]
    fn event_struct_is_kernel_sized() {
        assert_eq!(std::mem::size_of::<InputEvent>(), 24);
    }
}
