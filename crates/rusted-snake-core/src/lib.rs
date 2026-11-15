// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

//! Shared domain model for Rusted Snake.

pub mod color;
pub mod device;
pub mod effect;
pub mod keymap;
pub mod macros;
pub mod onboard;
pub mod paths;
pub mod settings;

pub use settings::Settings;

pub use color::Rgb;
pub use device::ZoneEffect;
pub use device::{BatteryInfo, DeviceKind, DeviceSnapshot, DpiInfo, ZoneId, ZoneSnapshot};
pub use effect::EffectKind;
pub use onboard::{
    DpiStages, OnboardAction, OnboardButton, OnboardCaps, OnboardData, OnboardLighting,
    OnboardProfile, OnboardZone,
};
