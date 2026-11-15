// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use serde::{Deserialize, Serialize};

use crate::Rgb;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    #[default]
    Off,
    Static,
    Spectrum,
    BreathSingle,
    BreathDual,
    BreathRandom,
    Reactive,
    Wave,
    StarlightSingle,
    StarlightDual,
    StarlightRandom,
    Pulsate,
    Ripple,
    RippleRandom,
}

impl EffectKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Static => "Static",
            Self::Spectrum => "Spectrum",
            Self::BreathSingle => "Breath",
            Self::BreathDual => "Dual breath",
            Self::BreathRandom => "Random breath",
            Self::Reactive => "Reactive",
            Self::Wave => "Wave",
            Self::StarlightSingle => "Starlight",
            Self::StarlightDual => "Dual starlight",
            Self::StarlightRandom => "Random starlight",
            Self::Pulsate => "Pulsate",
            Self::Ripple => "Ripple",
            Self::RippleRandom => "Random ripple",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Off => "Turn this zone off.",
            Self::Static => "One steady colour.",
            Self::Spectrum => "Cycle through every colour.",
            Self::BreathSingle => "Fade one colour in and out.",
            Self::BreathDual => "Alternate between two colours.",
            Self::BreathRandom => "Fade between random colours.",
            Self::Reactive => "Flash when you press a key.",
            Self::Wave => "A colour wave across the lighting.",
            Self::StarlightSingle => "Twinkling stars in one colour.",
            Self::StarlightDual => "Twinkling stars in two colours.",
            Self::StarlightRandom => "Twinkling stars in random colours.",
            Self::Pulsate => "Pulse between two colours.",
            Self::Ripple => "Ripples in one colour.",
            Self::RippleRandom => "Ripples in random colours.",
        }
    }

    pub fn needs_color(self) -> bool {
        matches!(
            self,
            Self::Static
                | Self::BreathSingle
                | Self::BreathDual
                | Self::Reactive
                | Self::StarlightSingle
                | Self::StarlightDual
                | Self::Ripple
        )
    }

    pub fn needs_two_colors(self) -> bool {
        matches!(self, Self::BreathDual | Self::StarlightDual | Self::Pulsate)
    }

    pub fn needs_speed(self) -> bool {
        matches!(
            self,
            Self::Reactive
                | Self::StarlightSingle
                | Self::StarlightDual
                | Self::StarlightRandom
                | Self::Ripple
                | Self::RippleRandom
        )
    }

    pub fn is_random(self) -> bool {
        matches!(
            self,
            Self::BreathRandom | Self::StarlightRandom | Self::RippleRandom
        )
    }
}

/// A software animation we drive ourselves, for devices that only accept static colours.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareEffect {
    #[default]
    None,
    Spectrum,
    Breath,
    Audio,
}

impl SoftwareEffect {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "Saved lighting",
            Self::Spectrum => "Spectrum",
            Self::Breath => "Breath",
            Self::Audio => "Audio",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoftwareEffectSettings {
    pub effect: SoftwareEffect,
    pub colors: Vec<Rgb>,
    pub speed: f32,
    pub audio_source: Option<String>,
    pub audio_gain: f32,
    pub audio_colors: Vec<Rgb>,
}

impl Default for SoftwareEffectSettings {
    fn default() -> Self {
        Self {
            effect: SoftwareEffect::None,
            colors: vec![Rgb::RAZER_GREEN, Rgb::new(0x1f, 0x8a, 0xff)],
            speed: 0.5,
            audio_source: None,
            audio_gain: 1.0,
            audio_colors: vec![
                Rgb::new(0x10, 0x2a, 0x08),
                Rgb::new(0x2f, 0xa0, 0x16),
                Rgb::new(0x8c, 0xff, 0x5c),
            ],
        }
    }
}
