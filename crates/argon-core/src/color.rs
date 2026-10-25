use serde::{Deserialize, Serialize};

/// 8-bit RGBA color. Stored as four bytes so it can go straight into a vertex.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    /// Pack as 0xRRGGBBAA, the representation handed to Lua.
    pub const fn to_u32(self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }

    pub const fn from_u32(v: u32) -> Color {
        Color {
            r: (v >> 24) as u8,
            g: (v >> 16) as u8,
            b: (v >> 8) as u8,
            a: v as u8,
        }
    }

    pub fn to_bytes(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn to_f32(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    /// HSV with h in degrees, s and v in 0..=1.
    pub fn hsv(h: f32, s: f32, v: f32, a: u8) -> Color {
        let h = h.rem_euclid(360.0) / 60.0;
        let i = h.floor();
        let f = h - i;
        let p = v * (1.0 - s);
        let q = v * (1.0 - s * f);
        let t = v * (1.0 - s * (1.0 - f));
        let (r, g, b) = match i as i32 {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        let c = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color::rgba(c(r), c(g), c(b), a)
    }

    /// Parse "#rrggbb" or "#rrggbbaa".
    pub fn parse_hex(s: &str) -> Option<Color> {
        let s = s.trim().trim_start_matches('#');
        let v = u32::from_str_radix(s, 16).ok()?;
        match s.len() {
            6 => Some(Color::from_u32((v << 8) | 0xff)),
            8 => Some(Color::from_u32(v)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_u32() {
        let c = Color::rgba(1, 2, 3, 4);
        assert_eq!(Color::from_u32(c.to_u32()), c);
        assert_eq!(c.to_u32(), 0x01020304);
    }

    #[test]
    fn hsv_primaries() {
        assert_eq!(Color::hsv(0.0, 1.0, 1.0, 255), Color::rgb(255, 0, 0));
        assert_eq!(Color::hsv(120.0, 1.0, 1.0, 255), Color::rgb(0, 255, 0));
        assert_eq!(Color::hsv(240.0, 1.0, 1.0, 255), Color::rgb(0, 0, 255));
    }

    #[test]
    fn parse_hex_forms() {
        assert_eq!(Color::parse_hex("#ff8000"), Some(Color::rgb(255, 128, 0)));
        assert_eq!(Color::parse_hex("ff800080"), Some(Color::rgba(255, 128, 0, 128)));
        assert_eq!(Color::parse_hex("zzz"), None);
    }
}
