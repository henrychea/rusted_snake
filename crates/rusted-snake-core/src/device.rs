use serde::{Deserialize, Serialize};

use crate::effect::EffectKind;
use crate::onboard::OnboardCaps;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Mouse,
    Keyboard,
    Keypad,
    Headset,
    MouseMat,
    Accessory,
    Monitor,
    Core,
    #[default]
    Other,
}

impl DeviceKind {
    pub fn from_openrazer(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "mouse" => Self::Mouse,
            "keyboard" => Self::Keyboard,
            "keypad" => Self::Keypad,
            "headset" | "audio" => Self::Headset,
            "mousemat" | "mousepad" => Self::MouseMat,
            "accessory" | "speaker" => Self::Accessory,
            "monitor" => Self::Monitor,
            "core" => Self::Core,
            _ => Self::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mouse => "Mouse",
            Self::Keyboard => "Keyboard",
            Self::Keypad => "Keypad",
            Self::Headset => "Headset",
            Self::MouseMat => "Mouse mat",
            Self::Accessory => "Accessory",
            Self::Monitor => "Monitor",
            Self::Core => "Core",
            Self::Other => "Device",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Mouse => "icons/mouse.svg",
            Self::Keyboard | Self::Keypad => "icons/keyboard.svg",
            Self::Headset => "icons/headset.svg",
            Self::MouseMat => "icons/mat.svg",
            Self::Monitor => "icons/monitor.svg",
            Self::Accessory => "icons/accessory.svg",
            Self::Core => "icons/core.svg",
            Self::Other => "icons/devices.svg",
        }
    }
}

/// One independently controllable lighting zone exposed by OpenRazer.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ZoneId {
    pub interface: String,
    pub name: String,
}

impl ZoneId {
    pub fn new(interface: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            interface: interface.into(),
            name: name.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ZoneSnapshot {
    pub id: ZoneId,
    pub brightness: Option<f64>,
    pub supported: Vec<EffectKind>,
    /// Effect name reported by OpenRazer, e.g. "static" or "spectrum".
    pub current_effect: Option<String>,
    pub colors: Vec<crate::Rgb>,
    pub speed: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DpiInfo {
    pub x: i32,
    pub y: i32,
    pub max: i32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub percent: f64,
    pub charging: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    /// Device path component, e.g. `RazerNagaTrinity`.
    pub id: String,
    pub display_name: String,
    pub kind: DeviceKind,
    pub serial: String,
    pub firmware: String,
    pub vid: u16,
    pub pid: u16,
    pub brightness: Option<f64>,
    pub dpi: Option<DpiInfo>,
    pub poll_rate: Option<u32>,
    pub battery: Option<BatteryInfo>,
    pub zones: Vec<ZoneSnapshot>,
    pub onboard: Option<OnboardCaps>,
    pub has_matrix: bool,
    pub matrix_size: (u32, u32),
}

impl DeviceSnapshot {
    pub fn zone(&self, name: &str) -> Option<&ZoneSnapshot> {
        self.zones.iter().find(|zone| zone.id.name == name)
    }

    pub fn supports_any_effect(&self) -> bool {
        self.zones.iter().any(|zone| !zone.supported.is_empty())
    }

    pub fn supports_static_software_effects(&self) -> bool {
        self.zones
            .iter()
            .any(|zone| zone.supported.contains(&EffectKind::Static))
    }

    pub fn type_label(&self) -> &'static str {
        self.kind.label()
    }
}

/// A lighting effect choice for one zone. Backend translates this to OpenRazer calls.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ZoneEffect {
    pub kind: EffectKind,
    pub colors: Vec<crate::Rgb>,
    pub speed: u8,
    pub direction: i32,
}
