// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

//! OpenRazer DBus, raw HID and OpenRGB fallback access for Rusted Snake.

pub mod audio;
pub mod hidraw;
pub mod macros;
pub mod onboard;
pub mod openrazer;
pub mod openrgb;
pub mod service;

pub use audio::AudioSource;
pub use macros::{EngineState, EngineStatus};
pub use openrgb::{OpenRgbApply, OpenRgbDevice};
pub use service::{BackendHandle, BackendInfo, BackupEntry, Command, Event};
