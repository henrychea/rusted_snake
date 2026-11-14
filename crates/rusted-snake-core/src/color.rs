use serde::{Deserialize, Serialize};

/// An sRGB colour, as shown in the interface.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const RAZER_GREEN: Self = Self {
        r: 0x44,
        g: 0xd6,
        b: 0x2c,
    };
    pub const WHITE: Self = Self {
        r: 0xff,
        g: 0xff,
        b: 0xff,
    };
    pub const RED: Self = Self {
        r: 0xff,
        g: 0x2a,
        b: 0x2a,
    };
    pub const BLUE: Self = Self {
        r: 0x2a,
        g: 0x7b,
        b: 0xff,
    };
    pub const AMBER: Self = Self {
        r: 0xff,
        g: 0xb0,
        b: 0x00,
    };
    pub const VIOLET: Self = Self {
        r: 0xb0,
        g: 0x4c,
        b: 0xff,
    };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn from_hex(text: &str) -> Option<Self> {
        let text = text.trim().trim_start_matches('#');
        let value = u32::from_str_radix(text, 16).ok()?;
        match text.len() {
            3 => Some(Self::new(
                ((value >> 8) & 0xf) as u8 * 17,
                ((value >> 4) & 0xf) as u8 * 17,
                (value & 0xf) as u8 * 17,
            )),
            6 => Some(Self::new(
                ((value >> 16) & 0xff) as u8,
                ((value >> 8) & 0xff) as u8,
                (value & 0xff) as u8,
            )),
            _ => None,
        }
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Mice store linear LED values while the screen is gamma encoded. Converting keeps
    /// orange, pink and purple from looking washed out on the device.
    pub fn to_led(self) -> [u8; 3] {
        [
            gamma_encode(self.r),
            gamma_encode(self.g),
            gamma_encode(self.b),
        ]
    }

    pub fn from_led(values: [u8; 3]) -> Self {
        Self::new(
            gamma_decode(values[0]),
            gamma_decode(values[1]),
            gamma_decode(values[2]),
        )
    }

    /// Hue in degrees, saturation and value in 0..=1.
    pub fn to_hsv(self) -> (f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let hue = if delta <= f32::EPSILON {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        let hue = if hue < 0.0 { hue + 360.0 } else { hue };
        let sat = if max <= f32::EPSILON {
            0.0
        } else {
            delta / max
        };
        (hue, sat, max)
    }

    pub fn from_hsv(hue: f32, sat: f32, value: f32) -> Self {
        let hue = hue.rem_euclid(360.0);
        let sat = sat.clamp(0.0, 1.0);
        let value = value.clamp(0.0, 1.0);
        let c = value * sat;
        let h = hue / 60.0;
        let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
        let (r, g, b) = match h as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = value - c;
        Self::new(
            ((r + m) * 255.0).round() as u8,
            ((g + m) * 255.0).round() as u8,
            ((b + m) * 255.0).round() as u8,
        )
    }

    pub fn mix(self, other: Self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount).round() as u8;
        Self::new(
            mix(self.r, other.r),
            mix(self.g, other.g),
            mix(self.b, other.b),
        )
    }

    pub fn scale(self, value: f32) -> Self {
        let scale = |v: u8| (v as f32 * value).clamp(0.0, 255.0).round() as u8;
        Self::new(scale(self.r), scale(self.g), scale(self.b))
    }

    pub fn distance(self, other: Self) -> f32 {
        let dr = self.r as f32 - other.r as f32;
        let dg = self.g as f32 - other.g as f32;
        let db = self.b as f32 - other.b as f32;
        (dr * dr + dg * dg + db * db).sqrt()
    }
}

fn gamma_encode(value: u8) -> u8 {
    ((value as f32 / 255.0).powf(2.2) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn gamma_decode(value: u8) -> u8 {
    ((value as f32 / 255.0).powf(1.0 / 2.2) * 255.0)
        .round()
        .clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        for hex in ["#44d62c", "#000000", "#ffffff", "#123abc"] {
            assert_eq!(Rgb::from_hex(hex).unwrap().to_hex(), hex);
        }
        assert_eq!(Rgb::from_hex("0f8"), Some(Rgb::new(0, 0xff, 0x88)));
    }

    #[test]
    fn led_gamma_round_trip() {
        let colour = Rgb::new(0x44, 0xd6, 0x2c);
        let back = Rgb::from_led(colour.to_led());
        assert!(colour.distance(back) <= 2.0, "{colour:?} became {back:?}");
    }

    #[test]
    fn hsv_round_trip() {
        for colour in [
            Rgb::RAZER_GREEN,
            Rgb::new(255, 128, 0),
            Rgb::new(20, 40, 200),
        ] {
            let (h, s, v) = colour.to_hsv();
            let back = Rgb::from_hsv(h, s, v);
            assert!(colour.distance(back) <= 2.0, "{colour:?} became {back:?}");
        }
    }
}
