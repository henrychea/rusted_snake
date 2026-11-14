use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use dbus::blocking::Connection;
use dbus::blocking::stdintf::org_freedesktop_dbus::Introspectable;
use rusted_snake_core::{
    BatteryInfo, DeviceKind, DeviceSnapshot, DpiInfo, EffectKind, Rgb, ZoneEffect, ZoneId,
    ZoneSnapshot,
};

const DEST: &str = "org.razer";
const ROOT: &str = "/org/razer";
const TIMEOUT: Duration = Duration::from_secs(3);

pub struct OpenRazer {
    connection: Connection,
}

impl OpenRazer {
    pub fn connect() -> Result<Self> {
        let connection = Connection::new_session().context("cannot connect to the session bus")?;
        let client = Self { connection };
        client.version().context(
            "OpenRazer did not answer on org.razer. Is openrazer-daemon installed and running?",
        )?;
        Ok(client)
    }

    pub fn version(&self) -> Result<String> {
        self.call::<_, (String,)>(ROOT, "razer.daemon", "version", ())
            .map(|(version,)| version)
    }

    fn call<A, R>(&self, path: &str, interface: &str, method: &str, args: A) -> Result<R>
    where
        A: dbus::arg::AppendAll,
        R: dbus::arg::ReadAll,
    {
        let proxy = self.connection.with_proxy(DEST, path, TIMEOUT);
        proxy
            .method_call(interface, method, args)
            .map_err(|error| anyhow!("{interface}.{method} failed: {error}"))
    }

    fn try_call<A, R>(&self, path: &str, interface: &str, method: &str, args: A) -> Option<R>
    where
        A: dbus::arg::AppendAll,
        R: dbus::arg::ReadAll,
    {
        self.call(path, interface, method, args).ok()
    }

    fn call_void<A: dbus::arg::AppendAll>(
        &self,
        path: &str,
        interface: &str,
        method: &str,
        args: A,
    ) -> Result<()> {
        self.call::<A, ()>(path, interface, method, args)
    }

    fn try_call_void<A: dbus::arg::AppendAll>(
        &self,
        path: &str,
        interface: &str,
        method: &str,
        args: A,
    ) -> Option<()> {
        self.try_call::<A, ()>(path, interface, method, args)
    }

    pub fn device_names(&self) -> Result<Vec<String>> {
        self.call::<_, (Vec<String>,)>(ROOT, "razer.devices", "getDevices", ())
            .map(|(names,)| names)
    }

    fn introspect(&self, path: &str) -> Result<String> {
        let proxy = self.connection.with_proxy(DEST, path, TIMEOUT);
        proxy
            .introspect()
            .map_err(|error| anyhow!("introspection failed: {error}"))
    }

    fn device_path(name: &str) -> String {
        format!("{ROOT}/device/{name}")
    }

    pub fn snapshot(&self, name: &str) -> Result<DeviceSnapshot> {
        let path = Self::device_path(name);
        let xml = self.introspect(&path)?;
        let mut snapshot = DeviceSnapshot {
            id: name.to_string(),
            ..Default::default()
        };

        if has_interface(&xml, "razer.device.misc") {
            snapshot.display_name = self
                .try_call::<_, (String,)>(&path, "razer.device.misc", "getDeviceName", ())
                .map(|(value,)| value)
                .unwrap_or_else(|| prettify(name));
            let kind = self
                .try_call::<_, (String,)>(&path, "razer.device.misc", "getDeviceType", ())
                .map(|(value,)| value)
                .unwrap_or_default();
            snapshot.kind = DeviceKind::from_openrazer(&kind);
            snapshot.serial = self
                .try_call::<_, (String,)>(&path, "razer.device.misc", "getSerial", ())
                .map(|(value,)| value)
                .unwrap_or_default();
            snapshot.firmware = self
                .try_call::<_, (String,)>(&path, "razer.device.misc", "getFirmware", ())
                .map(|(value,)| value)
                .unwrap_or_default();
            if let Some((ids,)) =
                self.try_call::<_, (Vec<i32>,)>(&path, "razer.device.misc", "getVidPid", ())
                && ids.len() >= 2
            {
                snapshot.vid = ids[0] as u16;
                snapshot.pid = ids[1] as u16;
            }
            if let Some((rate,)) =
                self.try_call::<_, (i16,)>(&path, "razer.device.misc", "getPollRate", ())
                && rate > 0
            {
                snapshot.poll_rate = Some(rate as u32);
            }
            if let Some((matrix,)) = self.try_call::<_, (Vec<i32>,)>(
                &path,
                "razer.device.misc",
                "getMatrixDimensions",
                (),
            ) && matrix.len() >= 2
            {
                snapshot.matrix_size = (matrix[0].max(0) as u32, matrix[1].max(0) as u32);
            }
            if let Some((has_matrix,)) =
                self.try_call::<_, (bool,)>(&path, "razer.device.misc", "hasMatrix", ())
            {
                snapshot.has_matrix = has_matrix;
            }
        } else {
            snapshot.display_name = prettify(name);
        }

        if has_interface(&xml, "razer.device.lighting.brightness") {
            snapshot.brightness = self
                .try_call::<_, (f64,)>(
                    &path,
                    "razer.device.lighting.brightness",
                    "getBrightness",
                    (),
                )
                .map(|(value,)| value.clamp(0.0, 100.0));
        }

        if has_interface(&xml, "razer.device.dpi") {
            let dpi = self
                .try_call::<_, (Vec<i32>,)>(&path, "razer.device.dpi", "getDPI", ())
                .map(|(values,)| values)
                .unwrap_or_default();
            let max = self
                .try_call::<_, (i32,)>(&path, "razer.device.dpi", "maxDPI", ())
                .map(|(value,)| value)
                .unwrap_or(16000);
            if dpi.len() >= 2 {
                snapshot.dpi = Some(DpiInfo {
                    x: dpi[0],
                    y: dpi[1],
                    max: max.max(dpi[0]).max(1),
                });
            }
        }

        if has_interface(&xml, "razer.device.power") {
            let percent = self
                .try_call::<_, (f64,)>(&path, "razer.device.power", "getBattery", ())
                .map(|(value,)| value)
                .unwrap_or(-1.0);
            if percent >= 0.0 {
                let charging = self
                    .try_call::<_, (bool,)>(&path, "razer.device.power", "isCharging", ())
                    .map(|(value,)| value)
                    .unwrap_or(false);
                snapshot.battery = Some(BatteryInfo { percent, charging });
            }
        }

        for (interface, block) in lighting_interfaces(&xml) {
            let Some(zone_name) = zone_name(&interface) else {
                continue;
            };
            let supported = effects(&block);
            if supported.is_empty() {
                continue;
            }
            let brightness = self
                .try_call::<_, (f64,)>(&path, &interface, "getBrightness", ())
                .map(|(value,)| value.clamp(0.0, 100.0));
            let current_effect = self
                .try_call::<_, (String,)>(&path, &interface, "getEffect", ())
                .map(|(value,)| value);
            let colors = self
                .try_call::<_, (Vec<u8>,)>(&path, &interface, "getEffectColors", ())
                .map(|(values,)| {
                    values
                        .chunks_exact(3)
                        .take(2)
                        .map(|chunk| Rgb::from_led([chunk[0], chunk[1], chunk[2]]))
                        .collect()
                })
                .unwrap_or_default();
            let speed = self
                .try_call::<_, (i32,)>(&path, &interface, "getEffectSpeed", ())
                .map(|(value,)| value)
                .unwrap_or(0);
            snapshot.zones.push(ZoneSnapshot {
                id: ZoneId::new(interface, zone_name),
                brightness,
                supported,
                current_effect,
                colors,
                speed,
            });
        }
        snapshot.zones.sort_by(|a, b| a.id.name.cmp(&b.id.name));

        if snapshot.kind == DeviceKind::Mouse
            && snapshot.vid == 0x1532
            && let Some(caps) = crate::onboard::known_caps(snapshot.pid)
        {
            snapshot.onboard = Some(caps);
        }

        Ok(snapshot)
    }

    pub fn devices(&self) -> Result<Vec<DeviceSnapshot>> {
        let names = self.device_names()?;
        let mut devices = Vec::new();
        for name in names {
            match self.snapshot(&name) {
                Ok(device) => devices.push(device),
                Err(error) => log::warn!("skipping OpenRazer device {name}: {error}"),
            }
        }
        devices.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(devices)
    }

    pub fn set_brightness(&self, device: &str, value: f64) -> Result<()> {
        let path = Self::device_path(device);
        self.call(
            path.as_str(),
            "razer.device.lighting.brightness",
            "setBrightness",
            (value.clamp(0.0, 100.0),),
        )
    }

    pub fn set_zone_brightness(&self, device: &str, zone: &ZoneId, value: f64) -> Result<()> {
        let path = Self::device_path(device);
        self.call_void(
            path.as_str(),
            zone.interface.as_str(),
            "setBrightness",
            (value.clamp(0.0, 100.0),),
        )
    }

    pub fn set_dpi(&self, device: &str, x: i32, y: i32) -> Result<()> {
        let path = Self::device_path(device);
        self.call(
            path.as_str(),
            "razer.device.dpi",
            "setDPI",
            (x.clamp(100, 45000) as i16, y.clamp(100, 45000) as i16),
        )
    }

    pub fn set_poll_rate(&self, device: &str, rate: u32) -> Result<()> {
        let path = Self::device_path(device);
        self.call(
            path.as_str(),
            "razer.device.misc",
            "setPollRate",
            (rate as i16,),
        )
    }

    pub fn set_zone_effect(&self, device: &str, zone: &ZoneId, effect: &ZoneEffect) -> Result<()> {
        let path = Self::device_path(device);
        let interface = zone.interface.as_str();
        let color = |index: usize| -> (u8, u8, u8) {
            let rgb = effect
                .colors
                .get(index)
                .copied()
                .unwrap_or(Rgb::RAZER_GREEN)
                .to_led();
            (rgb[0], rgb[1], rgb[2])
        };
        let (r1, g1, b1) = color(0);
        let (r2, g2, b2) = color(1);
        let speed = effect.speed;
        match effect.kind {
            EffectKind::Off => {
                self.call_void(path.as_str(), interface, "setNone", ())?;
            }
            EffectKind::Static => {
                self.call_void(path.as_str(), interface, "setStatic", (r1, g1, b1))?;
            }
            EffectKind::Spectrum => {
                self.call_void(path.as_str(), interface, "setSpectrum", ())?;
            }
            EffectKind::BreathSingle => {
                self.call_void(path.as_str(), interface, "setBreathSingle", (r1, g1, b1))?;
            }
            EffectKind::BreathDual => {
                self.call_void(
                    path.as_str(),
                    interface,
                    "setBreathDual",
                    (r1, g1, b1, r2, g2, b2),
                )?;
            }
            EffectKind::BreathRandom => {
                self.call_void(path.as_str(), interface, "setBreathRandom", ())?;
            }
            EffectKind::Reactive => {
                self.call_void(path.as_str(), interface, "setReactive", (r1, g1, b1, speed))?;
            }
            EffectKind::Wave => {
                self.call_void(path.as_str(), interface, "setWave", (effect.direction,))?;
            }
            EffectKind::StarlightSingle => {
                self.call_void(
                    path.as_str(),
                    interface,
                    "setStarlightSingle",
                    (r1, g1, b1, speed),
                )?;
            }
            EffectKind::StarlightDual => {
                self.call_void(
                    path.as_str(),
                    interface,
                    "setStarlightDual",
                    (r1, g1, b1, r2, g2, b2, speed),
                )?;
            }
            EffectKind::StarlightRandom => {
                self.call_void(path.as_str(), interface, "setStarlightRandom", (speed,))?;
            }
            EffectKind::Pulsate => {
                self.call_void(path.as_str(), interface, "setPulsate", ())?;
            }
            EffectKind::Ripple => {
                let refreshed = speed.max(1) as f64;
                if self
                    .try_call_void(
                        path.as_str(),
                        interface,
                        "setRipple",
                        (r1, g1, b1, refreshed),
                    )
                    .is_none()
                {
                    self.call_void(
                        path.as_str(),
                        "razer.device.lighting.custom",
                        "setRipple",
                        (r1, g1, b1, refreshed),
                    )?;
                }
            }
            EffectKind::RippleRandom => {
                let refreshed = speed.max(1) as f64;
                if self
                    .try_call_void(
                        path.as_str(),
                        interface,
                        "setRippleRandomColour",
                        (refreshed,),
                    )
                    .is_none()
                {
                    self.call_void(
                        path.as_str(),
                        "razer.device.lighting.custom",
                        "setRippleRandomColour",
                        (refreshed,),
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn set_static(&self, device: &str, zone: &ZoneId, color: Rgb) -> Result<()> {
        let path = Self::device_path(device);
        let (r, g, b) = {
            let values = color.to_led();
            (values[0], values[1], values[2])
        };
        self.call_void(
            path.as_str(),
            zone.interface.as_str(),
            "setStatic",
            (r, g, b),
        )
    }

    pub fn restore_last_effect(&self, device: &str, zone: &ZoneId) -> Result<()> {
        let path = Self::device_path(device);
        if self
            .try_call_void(
                path.as_str(),
                zone.interface.as_str(),
                "restoreLastEffect",
                (),
            )
            .is_some()
        {
            Ok(())
        } else {
            Err(anyhow!("this zone cannot restore its previous effect"))
        }
    }
}

fn has_interface(xml: &str, interface: &str) -> bool {
    xml.contains(&format!("<interface name=\"{interface}\">"))
}

fn interface_blocks(xml: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<interface name=\"") {
        let after = &rest[start + "<interface name=\"".len()..];
        let Some(end_name) = after.find('"') else {
            break;
        };
        let name = &after[..end_name];
        let Some(open_end) = after.find('>') else {
            break;
        };
        let body_start = &after[open_end + 1..];
        let Some(close) = body_start.find("</interface>") else {
            break;
        };
        blocks.push((name.to_string(), body_start[..close].to_string()));
        rest = &body_start[close + "</interface>".len()..];
    }
    blocks
}

fn lighting_interfaces(xml: &str) -> Vec<(String, String)> {
    interface_blocks(xml)
        .into_iter()
        .filter(|(name, _)| {
            name.starts_with("razer.device.lighting.") && name != "razer.device.lighting.brightness"
        })
        .collect()
}

fn block_has(block: &str, method: &str) -> bool {
    block.contains(&format!("<method name=\"{method}\""))
}

fn effects(block: &str) -> Vec<EffectKind> {
    let mut effects = Vec::new();
    let mut add = |kind: EffectKind, present: bool| {
        if present {
            effects.push(kind);
        }
    };
    add(EffectKind::Off, block_has(block, "setNone"));
    add(EffectKind::Static, block_has(block, "setStatic"));
    add(EffectKind::Spectrum, block_has(block, "setSpectrum"));
    add(
        EffectKind::BreathSingle,
        block_has(block, "setBreathSingle"),
    );
    add(EffectKind::BreathDual, block_has(block, "setBreathDual"));
    add(
        EffectKind::BreathRandom,
        block_has(block, "setBreathRandom"),
    );
    add(EffectKind::Reactive, block_has(block, "setReactive"));
    add(EffectKind::Wave, block_has(block, "setWave"));
    add(
        EffectKind::StarlightSingle,
        block_has(block, "setStarlightSingle"),
    );
    add(
        EffectKind::StarlightDual,
        block_has(block, "setStarlightDual"),
    );
    add(
        EffectKind::StarlightRandom,
        block_has(block, "setStarlightRandom"),
    );
    add(EffectKind::Pulsate, block_has(block, "setPulsate"));
    add(
        EffectKind::Ripple,
        block_has(block, "setRipple") || block_has(block, "setRippleRandomColour"),
    );
    add(
        EffectKind::RippleRandom,
        block_has(block, "setRippleRandomColour"),
    );
    effects
}

fn zone_name(interface: &str) -> Option<String> {
    let suffix = interface.strip_prefix("razer.device.lighting.")?;
    let name = match suffix {
        "chroma" => "Main lighting",
        "logo" => "Logo",
        "scroll" => "Scroll wheel",
        "left" => "Left side",
        "right" => "Right side",
        "backlight" => "Backlight",
        "profile_led" => "Profile LEDs",
        "custom" => return None,
        "bw2013" => "Lighting",
        "spectrum" => "Spectrum",
        _ => return None,
    };
    Some(name.to_string())
}

fn prettify(id: &str) -> String {
    let mut output = String::new();
    let mut previous_lower = false;
    for character in id.chars() {
        if character.is_ascii_uppercase() && previous_lower {
            output.push(' ');
        }
        output.push(character);
        previous_lower = character.is_ascii_lowercase() || character.is_ascii_digit();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"
<node>
  <interface name="razer.device.lighting.chroma">
    <method name="setStatic"><arg direction="in" type="y"/></method>
    <method name="setSpectrum"/>
    <method name="getBrightness"><arg direction="out" type="d"/></method>
    <method name="setNone"/>
  </interface>
  <interface name="razer.device.lighting.brightness">
    <method name="getBrightness"><arg direction="out" type="d"/></method>
  </interface>
  <interface name="razer.device.misc">
    <method name="getDeviceName"><arg direction="out" type="s"/></method>
  </interface>
</node>"#;

    #[test]
    fn parses_lighting_interfaces_and_effects() {
        let zones = lighting_interfaces(XML);
        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].0, "razer.device.lighting.chroma");
        assert_eq!(zone_name(&zones[0].0).as_deref(), Some("Main lighting"));
        let effects = effects(&zones[0].1);
        assert_eq!(
            effects,
            vec![EffectKind::Off, EffectKind::Static, EffectKind::Spectrum]
        );
        assert!(has_interface(XML, "razer.device.misc"));
        assert!(!has_interface(XML, "razer.device.dpi"));
    }

    #[test]
    fn prettifies_ids() {
        assert_eq!(prettify("RazerNagaTrinity"), "Razer Naga Trinity");
        assert_eq!(prettify("PM2522U28103850"), "PM2522 U28103850");
    }
}
