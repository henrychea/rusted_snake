// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

pub struct Assets;

macro_rules! assets {
    ($($path:literal => $file:literal),* $(,)?) => {
        fn embedded(path: &str) -> Option<&'static [u8]> {
            match path {
                $($path => Some(include_bytes!($file)),)*
                _ => None,
            }
        }
        fn embedded_paths() -> Vec<&'static str> {
            vec![$($path),*]
        }
    };
}

assets! {
    "art/mouse.png" => "../../../assets/art/mouse.png",
    "icons/logo.svg" => "../../../assets/icons/logo.svg",
    "icons/devices.svg" => "../../../assets/icons/devices.svg",
    "icons/buttons.svg" => "../../../assets/icons/buttons.svg",
    "icons/performance.svg" => "../../../assets/icons/performance.svg",
    "icons/lighting.svg" => "../../../assets/icons/lighting.svg",
    "icons/macros.svg" => "../../../assets/icons/macros.svg",
    "icons/settings.svg" => "../../../assets/icons/settings.svg",
    "icons/mouse.svg" => "../../../assets/icons/mouse.svg",
    "icons/keyboard.svg" => "../../../assets/icons/keyboard.svg",
    "icons/headset.svg" => "../../../assets/icons/headset.svg",
    "icons/mat.svg" => "../../../assets/icons/mat.svg",
    "icons/monitor.svg" => "../../../assets/icons/monitor.svg",
    "icons/accessory.svg" => "../../../assets/icons/accessory.svg",
    "icons/core.svg" => "../../../assets/icons/core.svg",
    "icons/check.svg" => "../../../assets/icons/check.svg",
    "icons/plus.svg" => "../../../assets/icons/plus.svg",
    "icons/trash.svg" => "../../../assets/icons/trash.svg",
    "icons/refresh.svg" => "../../../assets/icons/refresh.svg",
    "icons/alert.svg" => "../../../assets/icons/alert.svg",
    "icons/info.svg" => "../../../assets/icons/info.svg",
    "icons/palette.svg" => "../../../assets/icons/palette.svg",
    "icons/save.svg" => "../../../assets/icons/save.svg",
    "icons/download.svg" => "../../../assets/icons/download.svg",
    "icons/upload.svg" => "../../../assets/icons/upload.svg",
    "icons/chevron-right.svg" => "../../../assets/icons/chevron-right.svg",
    "icons/close.svg" => "../../../assets/icons/close.svg",
    "icons/usb.svg" => "../../../assets/icons/usb.svg",
    "icons/power.svg" => "../../../assets/icons/power.svg",
    "icons/volume.svg" => "../../../assets/icons/volume.svg",
    "icons/sliders.svg" => "../../../assets/icons/sliders.svg",
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(embedded(path).map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        if path.trim_matches('/') == "icons" {
            Ok(embedded_paths()
                .into_iter()
                .filter_map(|path| path.strip_prefix("icons/"))
                .map(SharedString::from)
                .collect())
        } else {
            Ok(Vec::new())
        }
    }
}

pub fn fonts() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("../../../assets/fonts/Barlow-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../../../assets/fonts/Barlow-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../../../assets/fonts/Barlow-SemiBold.ttf")),
        Cow::Borrowed(include_bytes!(
            "../../../assets/fonts/BarlowSemiCondensed-SemiBold.ttf"
        )),
    ]
}
