// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

//! OpenRGB fallback for devices that OpenRazer does not manage.
//!
//! This uses the `openrgb` command line (which itself can talk to a local
//! OpenRGB SDK server) rather than linking OpenRGB's GPL-2.0 client library.

use std::process::Command;

use anyhow::{Context, Result, bail};
use rusted_snake_core::Rgb;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenRgbDevice {
    pub index: usize,
    pub name: String,
    pub kind: String,
    pub description: String,
    pub location: String,
    pub serial: String,
    pub modes: Vec<String>,
    pub zones: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct OpenRgbApply {
    pub device: usize,
    pub zone: Option<usize>,
    pub mode: Option<String>,
    pub color: Rgb,
    pub brightness: Option<u8>,
    pub speed: Option<u8>,
}

pub fn list_devices() -> Result<Vec<OpenRgbDevice>> {
    let output = run(&["-ld"]).context(
        "OpenRGB is not installed or could not be started. Install OpenRGB to use the OpenRGB tab.",
    )?;
    Ok(parse_detailed(&output))
}

pub fn apply(change: &OpenRgbApply) -> Result<()> {
    let mut arguments = vec!["--device".to_string(), change.device.to_string()];
    if let Some(zone) = change.zone {
        arguments.push("--zone".into());
        arguments.push(zone.to_string());
    }
    if let Some(mode) = change.mode.as_deref().filter(|mode| !mode.is_empty()) {
        arguments.push("--mode".into());
        arguments.push(mode.to_string());
    }
    arguments.push("--color".into());
    arguments.push(change.color.to_hex().trim_start_matches('#').to_string());
    if let Some(brightness) = change.brightness {
        arguments.push("--brightness".into());
        arguments.push(brightness.min(100).to_string());
    }
    if let Some(speed) = change.speed {
        arguments.push("--speed".into());
        arguments.push(speed.min(100).to_string());
    }
    let args: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let output = run(&args)?;
    let text = output.trim();
    if text.to_ascii_lowercase().contains("error") || text.to_ascii_lowercase().contains("invalid")
    {
        bail!("OpenRGB rejected the change: {text}");
    }
    Ok(())
}

fn run(arguments: &[&str]) -> Result<String> {
    let output = Command::new("openrgb")
        .args(arguments)
        .output()
        .with_context(|| format!("cannot run openrgb {}", arguments.join(" ")))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        bail!("OpenRGB failed: {detail}");
    }
    Ok(if stdout.trim().is_empty() {
        stderr.into_owned()
    } else {
        stdout.into_owned()
    })
}

pub fn parse_detailed(text: &str) -> Vec<OpenRgbDevice> {
    let mut devices = Vec::new();
    let mut current: Option<OpenRgbDevice> = None;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            if let Some(device) = current.take() {
                devices.push(device);
            }
            continue;
        }
        if !trimmed.starts_with(' ') {
            if let Some(device) = current.take() {
                devices.push(device);
            }
            if let Some((index, name)) = parse_start(trimmed) {
                current = Some(OpenRgbDevice {
                    index,
                    name,
                    ..Default::default()
                });
            }
            continue;
        }
        let Some(device) = current.as_mut() else {
            continue;
        };
        let trimmed = trimmed.trim_start();
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key {
            "Type" => device.kind = value.to_string(),
            "Description" => device.description = value.to_string(),
            "Location" => device.location = value.to_string(),
            "Serial" => device.serial = value.to_string(),
            "Modes" => device.modes = tokens(value),
            "Zones" => device.zones = tokens(value),
            _ => {}
        }
    }
    if let Some(device) = current {
        devices.push(device);
    }
    devices
}

fn parse_start(line: &str) -> Option<(usize, String)> {
    let (index, name) = line.split_once(':')?;
    let index = index.trim().parse().ok()?;
    let name = name.trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some((index, name))
    }
}

fn tokens(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for character in value.chars() {
        if let Some(open) = quote {
            if character == open {
                quote = None;
            } else {
                current.push(character);
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '[' => {}
            ']' => {}
            character if character.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(character),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
0: ENE DRAM
  Type:           DRAM
  Description:    ENE SMBus Device
  Location:       I2C: SMBus I801 adapter at 0000:00:1f.4 (/dev/i2c-3), address 0x71
  Modes: [Direct] Off Static Breathing 'Spectrum Cycle'
  Zones: DRAM
  LEDs: 'DRAM LED 1'

4: Razer Basilisk V3 Pro 35K Phantom Green Edition (Wireless) (Razer Mouse Dock Pro)
  Type:           Mouse
  Description:    Razer Device, Quartz
  Version:        v2.51
  Location:       HID: /dev/hidraw5
  Serial:         PM2547H36000081
  Modes: [Direct] Off Static Breathing 'Spectrum Cycle' Wave Reactive
  Zones: 'Scroll Wheel' 'LED Strip'
  LEDs: 'Scroll Wheel' 'LED Strip LED 1'
"#;

    #[test]
    fn parses_devices_modes_and_zones() {
        let devices = parse_detailed(SAMPLE);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].index, 0);
        assert_eq!(devices[0].name, "ENE DRAM");
        assert_eq!(devices[0].kind, "DRAM");
        assert_eq!(devices[0].zones, vec!["DRAM"]);
        assert!(devices[0].modes.contains(&"Spectrum Cycle".into()));
        assert_eq!(devices[1].index, 4);
        assert_eq!(
            devices[1].name,
            "Razer Basilisk V3 Pro 35K Phantom Green Edition (Wireless) (Razer Mouse Dock Pro)"
        );
        assert_eq!(devices[1].zones, vec!["Scroll Wheel", "LED Strip"]);
    }

    #[test]
    fn tokenizes_quoted_names() {
        assert_eq!(
            tokens("[Direct] 'Spectrum Cycle' Wave"),
            vec!["Direct", "Spectrum Cycle", "Wave"]
        );
    }
}
