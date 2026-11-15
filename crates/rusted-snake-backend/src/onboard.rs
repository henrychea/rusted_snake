// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow, bail};
use rusted_snake_core::{
    DpiStages, OnboardAction, OnboardButton, OnboardCaps, OnboardData, OnboardLighting,
    OnboardProfile, OnboardZone,
};

use crate::hidraw::{Hidraw, Reply};

pub const RAZER_VID: u16 = 0x1532;

#[derive(Clone, Debug)]
pub struct Model {
    pub name: Option<String>,
    pub transaction_id: u8,
    pub profiles: u8,
    pub buttons: BTreeMap<u8, (String, String)>,
    pub leds: BTreeMap<u8, String>,
    pub verified: bool,
}

const LED_NAMES: &[(u8, &str)] = &[
    (0x01, "Scroll wheel"),
    (0x04, "Logo"),
    (0x05, "Backlight"),
    (0x0a, "Left side"),
    (0x0b, "Right side"),
];

fn known_leds(ids: &[u8]) -> BTreeMap<u8, String> {
    let mut leds = BTreeMap::new();
    for id in ids {
        let name = LED_NAMES
            .iter()
            .find(|(led, _)| led == id)
            .map(|(_, name)| *name)
            .unwrap_or("LED");
        leds.insert(*id, name.to_string());
    }
    leds
}

fn naga_trinity() -> Model {
    let mut buttons = BTreeMap::new();
    buttons.insert(0x01, ("Top".into(), "Left click".into()));
    buttons.insert(0x02, ("Top".into(), "Right click".into()));
    buttons.insert(0x03, ("Top".into(), "Wheel click".into()));
    buttons.insert(0x34, ("Top".into(), "Wheel up".into()));
    buttons.insert(0x35, ("Top".into(), "Wheel down".into()));
    buttons.insert(0x09, ("Top".into(), "Wheel left".into()));
    buttons.insert(0x0a, ("Top".into(), "Wheel right".into()));
    buttons.insert(0x0b, ("Top".into(), "DPI up".into()));
    buttons.insert(0x0c, ("Top".into(), "DPI down".into()));
    buttons.insert(0x0e, ("Bottom".into(), "Profile button".into()));
    buttons.insert(0x04, ("2-button plate".into(), "Top".into()));
    buttons.insert(0x05, ("2-button plate".into(), "Bottom".into()));
    for index in 0..7u8 {
        buttons.insert(
            0x50 + index,
            ("7-button plate".into(), (index + 1).to_string()),
        );
    }
    for index in 0..12u8 {
        buttons.insert(
            0x40 + index,
            ("12-button plate".into(), (index + 1).to_string()),
        );
    }
    Model {
        name: Some("Razer Naga Trinity".into()),
        transaction_id: 0x1f,
        profiles: 5,
        buttons,
        leds: known_leds(&[0x01, 0x04]),
        verified: true,
    }
}

/// Onboard information we can show before opening the raw HID node.
pub fn known_caps(pid: u16) -> Option<OnboardCaps> {
    match pid {
        0x0067 => {
            let model = naga_trinity();
            Some(OnboardCaps {
                pid,
                model_name: model.name.clone(),
                verified: model.verified,
                experimental: false,
                profile_count: model.profiles,
            })
        }
        _ => None,
    }
}

pub struct Session {
    hid: Hidraw,
    model: Model,
    experimental: bool,
    pid: u16,
}

impl Session {
    pub fn open(pid: u16, experimental: bool) -> Result<Option<Self>> {
        let Some(hid) = Hidraw::open(RAZER_VID, pid)? else {
            return Ok(None);
        };
        let mut session = Self {
            hid,
            model: Model {
                name: None,
                transaction_id: 0x1f,
                profiles: 5,
                buttons: BTreeMap::new(),
                leds: BTreeMap::new(),
                verified: false,
            },
            experimental,
            pid,
        };
        let model = match pid {
            0x0067 => Some(naga_trinity()),
            _ => session.detect()?,
        };
        let Some(model) = model else {
            return Ok(None);
        };
        session.hid.set_transaction_id(model.transaction_id);
        session.model = model;
        Ok(Some(session))
    }

    pub fn caps(&self) -> OnboardCaps {
        OnboardCaps {
            pid: self.pid,
            model_name: self.model.name.clone(),
            verified: self.model.verified,
            experimental: self.experimental,
            profile_count: self.model.profiles,
        }
    }

    pub fn set_experimental(&mut self, experimental: bool) {
        self.experimental = experimental;
    }

    pub fn is_writable(&self) -> bool {
        self.model.verified || self.experimental
    }

    fn detect(&mut self) -> Result<Option<Model>> {
        for transaction in [0x1f, 0x3f, 0xff] {
            self.hid.set_transaction_id(transaction);
            if !self.hid.send(0x04, 0x86, 0x26, &[1], 1)?.ok() {
                continue;
            }
            let mut profiles = 5;
            if let Ok(reply) = self.hid.send(0x05, 0x8a, 0x08, &[], 0)
                && reply.ok()
                && (1..=10).contains(&reply.args[0])
            {
                profiles = reply.args[0];
            }
            let mut buttons = BTreeMap::new();
            for id in 0x01..=0x6fu8 {
                let Ok(reply) = self.hid.send(0x02, 0x8c, 0x14, &[1, id, 0], 2) else {
                    continue;
                };
                if reply.ok() && reply.args.len() > 4 && (reply.args[3] != 0 || reply.args[4] != 0)
                {
                    buttons.insert(id, ("Buttons".to_string(), format!("Button {id:02x}")));
                }
            }
            if buttons.is_empty() {
                continue;
            }
            let mut led_ids = Vec::new();
            for led in [0x01u8, 0x04, 0x05, 0x0a, 0x0b] {
                let effect = self.hid.send_matching(0x0f, 0x82, 0x0c, &[1, led], &[1]);
                let brightness = self.hid.send(0x0f, 0x84, 0x03, &[1, led], 2);
                if let (Ok(effect), Ok(brightness)) = (effect, brightness)
                    && (effect.ok() || brightness.ok())
                {
                    led_ids.push(led);
                }
            }
            return Ok(Some(Model {
                name: None,
                transaction_id: transaction,
                profiles,
                buttons,
                leds: known_leds(&led_ids),
                verified: false,
            }));
        }
        Ok(None)
    }

    pub fn read_all(&mut self) -> Result<OnboardData> {
        let active_profile = self.active_profile().unwrap_or(1);
        let profiles_in_use = self
            .profiles_in_use()
            .unwrap_or_else(|_| (1..=self.model.profiles).collect());
        let mut profiles = Vec::new();
        for slot in 1..=self.model.profiles {
            let mut profile = OnboardProfile {
                slot,
                ..Default::default()
            };
            if let Ok(stages) = self.dpi_stages(slot) {
                profile.dpi = stages;
            }
            let button_ids: Vec<u8> = self.model.buttons.keys().copied().collect();
            for id in button_ids {
                if let Ok(action) = self.read_button(slot, id) {
                    profile.buttons.insert(id, action);
                }
            }
            let zone_ids: Vec<u8> = self.model.leds.keys().copied().collect();
            for zone in zone_ids {
                if let Ok(lighting) = self.read_lighting(slot, zone) {
                    profile.lighting.insert(zone, lighting);
                }
            }
            profiles.push(profile);
        }
        Ok(OnboardData {
            caps: self.caps(),
            active_profile,
            profiles_in_use,
            buttons: self
                .model
                .buttons
                .iter()
                .map(|(id, (group, name))| OnboardButton {
                    id: *id,
                    name: name.clone(),
                    group: group.clone(),
                })
                .collect(),
            zones: self
                .model
                .leds
                .iter()
                .map(|(id, name)| OnboardZone {
                    id: *id,
                    name: name.clone(),
                })
                .collect(),
            profiles,
        })
    }

    fn profiles_in_use(&mut self) -> Result<Vec<u8>> {
        let count = self.hid.send(0x05, 0x80, 0x08, &[], 0)?;
        let list = self.hid.send(0x05, 0x81, 0x08, &[], 0)?;
        if count.ok() && list.ok() {
            let count = (count.args[0] as usize).min(7);
            let mut profiles = Vec::new();
            for index in 0..count {
                let id = list.args.get(1 + index).copied().unwrap_or(0);
                if id >= 1 && id <= self.model.profiles && !profiles.contains(&id) {
                    profiles.push(id);
                }
            }
            if !profiles.is_empty() {
                return Ok(profiles);
            }
        }
        Ok((1..=self.model.profiles).collect())
    }

    fn active_profile(&mut self) -> Result<u8> {
        let reply = self.hid.send(0x05, 0x84, 0x08, &[], 0)?;
        Ok(check(reply, "read the active profile")?.args[0])
    }

    fn read_button(&mut self, profile: u8, id: u8) -> Result<OnboardAction> {
        let reply = self
            .hid
            .send(0x02, 0x8c, 0x14, &[profile, id, 0], 2)
            .with_context(|| format!("read button {id:02x} of profile {profile}"))?;
        let args = check(reply, "read a button")?.args;
        let length = (args.get(4).copied().unwrap_or(0) as usize).min(15);
        Ok(OnboardAction {
            action_type: args[3],
            params: args.get(5..5 + length).unwrap_or_default().to_vec(),
        })
    }

    pub fn write_button(&mut self, profile: u8, id: u8, action: &OnboardAction) -> Result<()> {
        self.check_write(profile)?;
        let mut args = vec![0u8; 0x14];
        args[0] = profile;
        args[1] = id;
        args[2] = 0; // 1 is acknowledged but ignored by the mouse
        args[3] = action.action_type;
        args[4] = action.params.len() as u8;
        let length = action.params.len().min(15);
        args[5..5 + length].copy_from_slice(&action.params[..length]);
        let reply = self.hid.send(0x02, 0x0c, 0x14, &args, 2)?;
        check(reply, "save a button")?;
        let now = self.read_button(profile, id)?;
        if now != *action {
            bail!("the mouse did not keep the new button setting");
        }
        Ok(())
    }

    fn dpi_stages(&mut self, profile: u8) -> Result<DpiStages> {
        let reply = self.hid.send(0x04, 0x86, 0x26, &[profile], 1)?;
        let args = check(reply, "read the DPI stages")?.args;
        let count = (args[2] as usize).min(5);
        let mut dpi = Vec::with_capacity(count);
        let mut active = 0;
        for index in 0..count {
            let offset = 3 + 7 * index;
            dpi.push(((args[offset + 1] as u16) << 8) | args[offset + 2] as u16);
            if args[offset] == args[1] {
                active = index;
            }
        }
        Ok(DpiStages { active, dpi })
    }

    pub fn write_dpi_stages(&mut self, profile: u8, stages: &DpiStages) -> Result<()> {
        self.check_write(profile)?;
        let count = stages.dpi.len().min(5);
        if count < 1 {
            bail!("need at least one DPI stage");
        }
        let mut args = vec![0u8; 0x26];
        args[0] = profile;
        args[1] = (stages.active.min(count - 1) + 1) as u8;
        args[2] = count as u8;
        for index in 0..count {
            let dpi = stages.dpi[index];
            check_dpi(dpi)?;
            let offset = 3 + 7 * index;
            args[offset] = (index + 1) as u8;
            args[offset + 1] = (dpi >> 8) as u8;
            args[offset + 2] = dpi as u8;
            args[offset + 3] = (dpi >> 8) as u8;
            args[offset + 4] = dpi as u8;
        }
        let reply = self.hid.send(0x04, 0x06, 0x26, &args, 1)?;
        check(reply, "save the DPI stages")?;
        let now = self.dpi_stages(profile)?;
        if now.dpi != stages.dpi[..count] {
            bail!("the mouse did not keep the new DPI stages");
        }
        Ok(())
    }

    fn read_lighting(&mut self, profile: u8, zone: u8) -> Result<OnboardLighting> {
        // The reply always has 0 in byte 0, so only the LED id is echoed.
        let reply = self
            .hid
            .send_matching(0x0f, 0x82, 0x0c, &[profile, zone], &[1])
            .with_context(|| format!("read LED zone {zone:02x} of profile {profile}"))?;
        let args = check(reply, "read the lighting")?.args;
        let count = (args[5] as usize).min(2);
        let mut colors = Vec::new();
        for index in 0..count {
            colors.push([
                args[6 + 3 * index],
                args[7 + 3 * index],
                args[8 + 3 * index],
            ]);
        }
        Ok(OnboardLighting {
            effect: args[2],
            mode: args[3],
            speed: args[4],
            colors,
        })
    }

    pub fn write_lighting(
        &mut self,
        profile: u8,
        zone: u8,
        lighting: &OnboardLighting,
    ) -> Result<()> {
        self.check_write(profile)?;
        let mut args = vec![0u8; 0x0c];
        args[0] = profile;
        args[1] = zone;
        args[2] = lighting.effect;
        args[3] = lighting.mode;
        args[4] = lighting.speed;
        let count = lighting.colors.len().min(2);
        args[5] = count as u8;
        for index in 0..count {
            for channel in 0..3 {
                args[6 + 3 * index + channel] = lighting.colors[index][channel];
            }
        }
        let reply = self.hid.send_matching(0x0f, 0x02, 0x0c, &args, &[1])?;
        check(reply, "save the lighting")?;
        let now = self.read_lighting(profile, zone)?;
        if now != *lighting {
            bail!("the mouse did not keep the new lighting");
        }
        Ok(())
    }

    fn check_write(&self, profile: u8) -> Result<()> {
        if !self.is_writable() {
            bail!(
                "saving to this mouse has not been tested. Turn on experimental saving in Settings"
            );
        }
        if profile < 1 || profile > self.model.profiles {
            bail!("profile must be 1-{}", self.model.profiles);
        }
        Ok(())
    }
}

fn check(reply: Reply, what: &str) -> Result<Reply> {
    if reply.ok() {
        Ok(reply)
    } else {
        Err(anyhow!(
            "the mouse refused to {what} (status {})",
            reply.status
        ))
    }
}

fn check_dpi(dpi: u16) -> Result<()> {
    if !(100..=45000).contains(&dpi) {
        bail!("DPI must be 100-45000");
    }
    Ok(())
}

/// Read the live device list into a simple view for the interface.
pub fn button_groups(buttons: &[OnboardButton]) -> BTreeMap<String, Vec<u8>> {
    let mut groups: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for button in buttons {
        groups
            .entry(button.group.clone())
            .or_default()
            .push(button.id);
    }
    groups
}
