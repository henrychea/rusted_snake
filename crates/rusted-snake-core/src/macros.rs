// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use serde::{Deserialize, Serialize};

use crate::keymap::{CapturedKey, Modifiers};
use crate::paths;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    #[default]
    Once,
    WhileHeld,
    Toggle,
}

impl RepeatMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Once => "Once",
            Self::WhileHeld => "Repeat while held",
            Self::Toggle => "Repeat on/off",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MacroAction {
    Key {
        code: u16,
        #[serde(default)]
        modifiers: Modifiers,
    },
    Text {
        text: String,
    },
    Command {
        command: String,
    },
}

impl MacroAction {
    pub fn describe(&self) -> String {
        match self {
            Self::Key { code, modifiers } => {
                format!(
                    "Presses {}",
                    CapturedKey {
                        code: *code,
                        modifiers: *modifiers
                    }
                    .label()
                )
            }
            Self::Text { text } => format!("Types \"{text}\""),
            Self::Command { command } => {
                let name = command.split_whitespace().next().unwrap_or(command);
                let name = name.rsplit('/').next().unwrap_or(name);
                format!("Runs {name}")
            }
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Key { .. } => "Key",
            Self::Text { .. } => "Text",
            Self::Command { .. } => "Command",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MacroDef {
    pub id: u64,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub trigger: Option<u16>,
    pub action: MacroAction,
    #[serde(default)]
    pub repeat: RepeatMode,
}

impl MacroDef {
    pub fn trigger_label(&self) -> String {
        self.trigger
            .map(crate::keymap::evdev_name)
            .unwrap_or_else(|| "No trigger".into())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MacroSet {
    pub name: String,
    #[serde(default)]
    pub macros: Vec<MacroDef>,
}

impl MacroSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            macros: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MacroLibrary {
    #[serde(default)]
    pub sets: Vec<MacroSet>,
    #[serde(default)]
    pub active_set: usize,
    #[serde(default)]
    next_id: u64,
}

impl Default for MacroLibrary {
    fn default() -> Self {
        Self {
            sets: vec![MacroSet::new("Default")],
            active_set: 0,
            next_id: 1,
        }
    }
}

impl MacroLibrary {
    pub fn load() -> Self {
        match std::fs::read_to_string(paths::macros_file()) {
            Ok(text) => {
                let mut library: Self = serde_json::from_str(&text).unwrap_or_default();
                library.normalise();
                library
            }
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        paths::write_atomic(&paths::macros_file(), &text)
    }

    fn normalise(&mut self) {
        if self.sets.is_empty() {
            self.sets.push(MacroSet::new("Default"));
        }
        self.active_set = self.active_set.min(self.sets.len() - 1);
        let mut max_id = self.next_id;
        for set in &self.sets {
            for entry in &set.macros {
                max_id = max_id.max(entry.id + 1);
            }
        }
        self.next_id = max_id;
    }

    pub fn active(&self) -> &MacroSet {
        &self.sets[self.active_set.min(self.sets.len().saturating_sub(1))]
    }

    pub fn active_mut(&mut self) -> &mut MacroSet {
        let index = self.active_set.min(self.sets.len().saturating_sub(1));
        &mut self.sets[index]
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.sets.len() {
            self.active_set = index;
        }
    }

    pub fn add_set(&mut self, name: impl Into<String>) {
        self.sets.push(MacroSet::new(name));
        self.active_set = self.sets.len() - 1;
    }

    pub fn remove_active_set(&mut self) -> bool {
        if self.sets.len() <= 1 {
            return false;
        }
        self.sets.remove(self.active_set);
        self.active_set = self.active_set.saturating_sub(1);
        true
    }

    pub fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_macro(&mut self, mut entry: MacroDef) -> u64 {
        if entry.id == 0 {
            entry.id = self.allocate_id();
        }
        self.active_mut().macros.push(entry);
        self.active()
            .macros
            .last()
            .map(|entry| entry.id)
            .unwrap_or(0)
    }

    pub fn replace_macro(&mut self, entry: MacroDef) {
        if let Some(existing) = self
            .active_mut()
            .macros
            .iter_mut()
            .find(|item| item.id == entry.id)
        {
            *existing = entry;
        }
    }

    pub fn remove_macro(&mut self, id: u64) {
        self.active_mut().macros.retain(|entry| entry.id != id);
    }

    /// Macros that can fire for a device (including macros with no device restriction).
    pub fn macros_for_device(&self, device: &str) -> Vec<MacroDef> {
        self.active()
            .macros
            .iter()
            .filter(|entry| entry.device.as_deref().is_none_or(|value| value == device))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_sets_switch() {
        let mut library = MacroLibrary::default();
        let first = library.add_macro(MacroDef {
            id: 0,
            device: None,
            name: "one".into(),
            trigger: Some(2),
            action: MacroAction::Text {
                text: "hello".into(),
            },
            repeat: RepeatMode::Once,
        });
        let second = library.add_macro(MacroDef {
            id: 0,
            device: Some("RazerNagaTrinity".into()),
            name: "two".into(),
            trigger: Some(3),
            action: MacroAction::Command {
                command: "true".into(),
            },
            repeat: RepeatMode::Once,
        });
        assert_ne!(first, second);
        assert_eq!(library.macros_for_device("RazerNagaTrinity").len(), 2);
        assert_eq!(library.macros_for_device("RazerOther").len(), 1);
        library.add_set("Work");
        assert_eq!(library.active().name, "Work");
        assert!(library.remove_active_set());
        assert_eq!(library.active().name, "Default");
    }
}
