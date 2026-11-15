// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Henry Chea

use gpui::{Background, Hsla, linear_color_stop, linear_gradient, rgb, rgba};

pub const ACCENT: u32 = 0x44d62c;

pub fn accent() -> Hsla {
    rgb(ACCENT).into()
}

pub fn on_accent() -> Hsla {
    rgb(0x081006).into()
}

pub fn accent_soft(alpha: f32) -> Hsla {
    with_alpha(ACCENT, alpha)
}

pub fn bg() -> Hsla {
    rgb(0x0a0b0d).into()
}

pub fn backdrop() -> Background {
    linear_gradient(
        155.0,
        linear_color_stop(rgb(0x0e140c), 0.0),
        linear_color_stop(rgb(0x08090a), 0.55),
    )
}

pub fn chrome() -> Hsla {
    rgb(0x0e0f12).into()
}

pub fn surface() -> Hsla {
    rgb(0x141619).into()
}

pub fn surface_raised() -> Hsla {
    rgb(0x191c20).into()
}

pub fn surface_hover() -> Hsla {
    rgb(0x1f2328).into()
}

pub fn border() -> Hsla {
    rgb(0x262a30).into()
}

pub fn border_strong() -> Hsla {
    rgb(0x363b43).into()
}

pub fn text() -> Hsla {
    rgb(0xf1f4f2).into()
}

pub fn muted() -> Hsla {
    rgb(0x9ba3ab).into()
}

pub fn dim() -> Hsla {
    rgb(0x69717a).into()
}

pub fn danger() -> Hsla {
    rgb(0xff5a5f).into()
}

pub fn warning() -> Hsla {
    rgb(0xffb020).into()
}

pub fn info() -> Hsla {
    rgb(0x57b6ff).into()
}

pub fn white_soft(alpha: f32) -> Hsla {
    with_alpha(0xffffff, alpha)
}

pub fn black_soft(alpha: f32) -> Hsla {
    with_alpha(0x000000, alpha)
}

pub fn with_alpha(hex: u32, alpha: f32) -> Hsla {
    let mut color: Hsla = rgba((hex << 8) | ((alpha.clamp(0.0, 1.0) * 255.0) as u32)).into();
    color.a = alpha;
    color
}

pub fn from_rgb(color: rusted_snake_core::Rgb) -> Hsla {
    rgb(((color.r as u32) << 16) | ((color.g as u32) << 8) | color.b as u32).into()
}
