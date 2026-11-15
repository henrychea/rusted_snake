// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::effect::SoftwareEffectSettings;
use crate::paths;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Allow writes to onboard profiles of models that have not been verified.
    pub experimental_onboard: bool,
    pub start_with_computer: bool,
    /// Closing the window keeps the app running in the tray instead of quitting.
    pub close_to_tray: bool,
    pub selected_device: Option<String>,
    /// Last side plate chosen for each mouse id.
    pub onboard_plate: BTreeMap<String, String>,
    pub software: SoftwareEffectSettings,
}

impl Settings {
    pub fn load() -> Self {
        match std::fs::read_to_string(paths::settings_file()) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        paths::write_atomic(&paths::settings_file(), &text)
    }
}

pub fn is_autostart_enabled() -> bool {
    paths::autostart_file().is_file()
}

pub fn set_autostart(enabled: bool) -> std::io::Result<()> {
    let file = paths::autostart_file();
    if !enabled {
        return match std::fs::remove_file(file) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        };
    }
    let exe = std::env::current_exe()?;
    let contents = format!(
        "[Desktop Entry]\nType=Application\nName=Rusted Snake\nComment=Razer device settings, lighting and macros\nExec=\"{}\"\nIcon=rusted-snake\nTerminal=false\nX-GNOME-Autostart-enabled=true\nCategories=Settings;HardwareSettings;\n",
        exe.display()
    );
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, contents)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_defaults_and_round_trip() {
        let settings = Settings::default();
        let text = serde_json::to_string(&settings).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert!(!back.experimental_onboard);
        assert_eq!(back.software.effect, crate::effect::SoftwareEffect::None);
    }
}
