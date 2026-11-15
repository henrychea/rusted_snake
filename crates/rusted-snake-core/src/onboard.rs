// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::keymap;

/// What a button does, in the mouse's own onboard profile format.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardAction {
    pub action_type: u8,
    pub params: Vec<u8>,
}

impl OnboardAction {
    pub const DISABLED: u8 = 0x00;
    pub const MOUSE: u8 = 0x01;
    pub const KEY: u8 = 0x02;
    pub const MACRO: u8 = 0x04;
    pub const MACRO_REPEAT: u8 = 0x05;
    pub const DPI: u8 = 0x06;
    pub const PROFILE: u8 = 0x07;
    pub const SCROLL: u8 = 0x0e;

    pub fn key(modifiers: u8, hid_usage: u8) -> Self {
        Self {
            action_type: Self::KEY,
            params: vec![modifiers, hid_usage],
        }
    }

    pub fn mouse(button: u8) -> Self {
        Self {
            action_type: Self::MOUSE,
            params: vec![button],
        }
    }

    pub fn dpi_up() -> Self {
        Self {
            action_type: Self::DPI,
            params: vec![1],
        }
    }

    pub fn dpi_down() -> Self {
        Self {
            action_type: Self::DPI,
            params: vec![2],
        }
    }

    pub fn next_profile() -> Self {
        Self {
            action_type: Self::PROFILE,
            params: vec![1],
        }
    }

    pub fn disabled() -> Self {
        Self {
            action_type: Self::DISABLED,
            params: Vec::new(),
        }
    }

    pub fn key_modifiers(&self) -> u8 {
        if self.action_type == Self::KEY && self.params.len() > 1 {
            self.params[0]
        } else {
            0
        }
    }

    pub fn key_usage(&self) -> u8 {
        if self.action_type == Self::KEY {
            self.params.last().copied().unwrap_or(0)
        } else {
            0
        }
    }

    pub fn describe(&self) -> String {
        match self.action_type {
            Self::DISABLED => "Disabled".into(),
            Self::MOUSE => match self.params.first().copied().unwrap_or(0) {
                1 => "Left click".into(),
                2 => "Right click".into(),
                3 => "Middle click".into(),
                4 => "Back".into(),
                5 => "Forward".into(),
                9 => "Scroll left".into(),
                10 => "Scroll right".into(),
                other => format!("Mouse button {other}"),
            },
            Self::KEY => {
                let mut text = String::new();
                let modifiers = self.key_modifiers();
                if modifiers & 0x11 != 0 {
                    text.push_str("Ctrl+");
                }
                if modifiers & 0x22 != 0 {
                    text.push_str("Shift+");
                }
                if modifiers & 0x44 != 0 {
                    text.push_str("Alt+");
                }
                if modifiers & 0x88 != 0 {
                    text.push_str("Super+");
                }
                text.push_str(&keymap::hid_usage_name(self.key_usage()));
                text
            }
            Self::MACRO | Self::MACRO_REPEAT => "Macro (from Synapse)".into(),
            Self::DPI => {
                if self.params.first().copied() == Some(2) {
                    "DPI down".into()
                } else {
                    "DPI up".into()
                }
            }
            Self::PROFILE => "Next profile".into(),
            Self::SCROLL => "Scroll".into(),
            other => format!("Other ({other:02x})"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardLighting {
    pub effect: u8,
    pub mode: u8,
    pub speed: u8,
    /// Raw values as stored on the mouse, not gamma corrected.
    pub colors: Vec<[u8; 3]>,
}

impl OnboardLighting {
    pub const OFF: u8 = 0x00;
    pub const STATIC: u8 = 0x01;
    pub const BREATH: u8 = 0x02;
    pub const SPECTRUM: u8 = 0x03;
    pub const WAVE: u8 = 0x04;
    pub const REACTIVE: u8 = 0x05;

    pub fn static_color(colour: [u8; 3]) -> Self {
        Self {
            effect: Self::STATIC,
            mode: 0,
            speed: 0,
            colors: vec![colour],
        }
    }

    pub fn spectrum() -> Self {
        Self {
            effect: Self::SPECTRUM,
            mode: 0,
            speed: 0x28,
            colors: vec![],
        }
    }

    pub fn off() -> Self {
        Self {
            effect: Self::OFF,
            mode: 0,
            speed: 0,
            colors: vec![],
        }
    }

    pub fn describe(&self) -> &'static str {
        match self.effect {
            Self::OFF => "Off",
            Self::STATIC => "Static",
            Self::BREATH => "Breath",
            Self::SPECTRUM => "Spectrum",
            Self::WAVE => "Wave",
            Self::REACTIVE => "Reactive",
            _ => "Saved effect",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DpiStages {
    /// Index of the stage the mouse starts on when this profile is selected.
    pub active: usize,
    pub dpi: Vec<u16>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardCaps {
    pub pid: u16,
    pub model_name: Option<String>,
    pub verified: bool,
    pub experimental: bool,
    pub profile_count: u8,
}

impl OnboardCaps {
    pub fn writable(&self) -> bool {
        self.verified || self.experimental
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardButton {
    pub id: u8,
    pub name: String,
    pub group: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardZone {
    pub id: u8,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardProfile {
    pub slot: u8,
    pub dpi: DpiStages,
    pub buttons: BTreeMap<u8, OnboardAction>,
    pub lighting: BTreeMap<u8, OnboardLighting>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardData {
    pub caps: OnboardCaps,
    pub active_profile: u8,
    pub profiles_in_use: Vec<u8>,
    pub buttons: Vec<OnboardButton>,
    pub zones: Vec<OnboardZone>,
    pub profiles: Vec<OnboardProfile>,
}

impl OnboardData {
    pub fn profile(&self, slot: u8) -> Option<&OnboardProfile> {
        self.profiles.iter().find(|profile| profile.slot == slot)
    }

    pub fn profile_mut(&mut self, slot: u8) -> Option<&mut OnboardProfile> {
        self.profiles
            .iter_mut()
            .find(|profile| profile.slot == slot)
    }

    pub fn backup(&self) -> BackupFile {
        BackupFile {
            model: self.caps.model_name.clone(),
            profiles: self.profiles.iter().map(BackupProfile::from).collect(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BackupFile {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub profiles: Vec<BackupProfile>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BackupProfile {
    pub profile: u8,
    #[serde(rename = "dpiActive", default)]
    pub dpi_active: usize,
    #[serde(rename = "dpiStages", default)]
    pub dpi_stages: Vec<u16>,
    #[serde(default)]
    pub buttons: BTreeMap<String, Vec<i32>>,
    #[serde(default)]
    pub lighting: BTreeMap<String, Vec<i32>>,
}

impl From<&OnboardProfile> for BackupProfile {
    fn from(profile: &OnboardProfile) -> Self {
        let mut buttons = BTreeMap::new();
        for (id, action) in &profile.buttons {
            let mut values = vec![action.action_type as i32];
            values.extend(action.params.iter().map(|value| *value as i32));
            buttons.insert(id.to_string(), values);
        }
        let mut lighting = BTreeMap::new();
        for (zone, light) in &profile.lighting {
            let mut values = vec![light.effect as i32, light.mode as i32, light.speed as i32];
            for colour in &light.colors {
                values.extend(colour.iter().map(|value| *value as i32));
            }
            lighting.insert(zone.to_string(), values);
        }
        Self {
            profile: profile.slot,
            dpi_active: profile.dpi.active,
            dpi_stages: profile.dpi.dpi.clone(),
            buttons,
            lighting,
        }
    }
}

#[derive(Debug, Error)]
pub enum BackupError {
    #[error("not a Rusted Snake or Snake backup: {0}")]
    Json(#[from] serde_json::Error),
    #[error("profile entry has an invalid {0}")]
    Invalid(&'static str),
    #[error("backup has no profile entries")]
    Empty,
}

impl BackupFile {
    pub fn parse(text: &str) -> Result<Self, BackupError> {
        let file: Self = serde_json::from_str(text)?;
        if file.profiles.is_empty() {
            return Err(BackupError::Empty);
        }
        for profile in &file.profiles {
            if profile.dpi_stages.is_empty() || profile.dpi_stages.len() > 5 {
                return Err(BackupError::Invalid("DPI stage list"));
            }
            if profile.dpi_active >= profile.dpi_stages.len() {
                return Err(BackupError::Invalid("active DPI stage"));
            }
        }
        Ok(file)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn into_profiles(self) -> Result<Vec<OnboardProfile>, BackupError> {
        let mut profiles = Vec::with_capacity(self.profiles.len());
        for entry in self.profiles {
            let mut buttons = BTreeMap::new();
            for (key, values) in entry.buttons {
                let id = key
                    .parse::<u8>()
                    .map_err(|_| BackupError::Invalid("button id"))?;
                let Some(action_type) = values.first().copied() else {
                    return Err(BackupError::Invalid("button action"));
                };
                if !(0..=255).contains(&action_type) {
                    return Err(BackupError::Invalid("button action type"));
                }
                let params = values[1..]
                    .iter()
                    .map(|value| {
                        u8::try_from(*value).map_err(|_| BackupError::Invalid("button parameter"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                buttons.insert(
                    id,
                    OnboardAction {
                        action_type: action_type as u8,
                        params,
                    },
                );
            }
            let mut lighting = BTreeMap::new();
            for (key, values) in entry.lighting {
                let zone = key
                    .parse::<u8>()
                    .map_err(|_| BackupError::Invalid("LED zone id"))?;
                if values.len() < 3 {
                    return Err(BackupError::Invalid("lighting entry"));
                }
                let mut colours = Vec::new();
                let mut chunks = values[3..].chunks_exact(3);
                for chunk in &mut chunks {
                    colours.push([
                        u8::try_from(chunk[0]).map_err(|_| BackupError::Invalid("colour"))?,
                        u8::try_from(chunk[1]).map_err(|_| BackupError::Invalid("colour"))?,
                        u8::try_from(chunk[2]).map_err(|_| BackupError::Invalid("colour"))?,
                    ]);
                }
                if !chunks.remainder().is_empty() {
                    return Err(BackupError::Invalid("colour list"));
                }
                let effect = u8::try_from(values[0]).map_err(|_| BackupError::Invalid("effect"))?;
                let mode =
                    u8::try_from(values[1]).map_err(|_| BackupError::Invalid("effect mode"))?;
                let speed =
                    u8::try_from(values[2]).map_err(|_| BackupError::Invalid("effect speed"))?;
                lighting.insert(
                    zone,
                    OnboardLighting {
                        effect,
                        mode,
                        speed,
                        colors: colours,
                    },
                );
            }
            profiles.push(OnboardProfile {
                slot: entry.profile,
                dpi: DpiStages {
                    active: entry.dpi_active,
                    dpi: entry.dpi_stages,
                },
                buttons,
                lighting,
            });
        }
        Ok(profiles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> OnboardData {
        let mut buttons = BTreeMap::new();
        buttons.insert(0x01, OnboardAction::key(0x22, 0x04));
        let mut lighting = BTreeMap::new();
        lighting.insert(0x01, OnboardLighting::static_color([0x11, 0x4b, 0x0c]));
        OnboardData {
            caps: OnboardCaps {
                pid: 0x67,
                model_name: Some("Razer Naga Trinity".into()),
                verified: true,
                experimental: false,
                profile_count: 5,
            },
            active_profile: 1,
            profiles_in_use: vec![1, 3],
            profiles: vec![OnboardProfile {
                slot: 1,
                dpi: DpiStages {
                    active: 1,
                    dpi: vec![800, 1800, 3200],
                },
                buttons,
                lighting,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn backup_round_trip() {
        let file = data().backup();
        let text = file.to_json();
        let parsed = BackupFile::parse(&text).unwrap();
        let profiles = parsed.into_profiles().unwrap();
        assert_eq!(profiles[0].slot, 1);
        assert_eq!(profiles[0].dpi.dpi, vec![800, 1800, 3200]);
        assert_eq!(profiles[0].buttons[&0x01], OnboardAction::key(0x22, 0x04));
        assert_eq!(profiles[0].lighting[&0x01].colors, vec![[0x11, 0x4b, 0x0c]]);
    }

    #[test]
    fn rejects_empty_backup() {
        assert!(BackupFile::parse(r#"{"profiles":[]}"#).is_err());
    }
}
