//! The built-in 8x16 bitmap font (Unscii-16, public domain), parsed from the
//! `.hex` format: `CODEPOINT:ROW0ROW1...ROW15` with one byte per row, MSB on
//! the left.

use std::collections::HashMap;

pub const GLYPH_W: usize = 8;
pub const GLYPH_H: usize = 16;

/// Raw font data compiled into the binary.
pub const UNSCII_16_HEX: &str = include_str!("../../../assets/fonts/unscii-16.hex");

#[derive(Clone)]
pub struct Font {
    glyphs: HashMap<u32, [u8; GLYPH_H]>,
    /// Glyph used when a code point is missing.
    fallback: [u8; GLYPH_H],
}

impl Font {
    pub fn builtin() -> Font {
        Font::parse_hex(UNSCII_16_HEX)
    }

    pub fn parse_hex(src: &str) -> Font {
        let mut glyphs = HashMap::with_capacity(4096);
        for line in src.lines() {
            let Some((cp, bits)) = line.split_once(':') else { continue };
            let Ok(cp) = u32::from_str_radix(cp.trim(), 16) else { continue };
            // Only 8-wide glyphs (32 hex chars). Skip 16-wide ones.
            if bits.len() != GLYPH_H * 2 {
                continue;
            }
            let mut rows = [0u8; GLYPH_H];
            let mut ok = true;
            for (i, row) in rows.iter_mut().enumerate() {
                match u8::from_str_radix(&bits[i * 2..i * 2 + 2], 16) {
                    Ok(v) => *row = v,
                    Err(_) => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                glyphs.insert(cp, rows);
            }
        }
        // A hollow box as the missing-glyph marker.
        let fallback = glyphs
            .get(&0xFFFD)
            .copied()
            .unwrap_or([0x00, 0x00, 0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x00, 0x00, 0x00]);
        Font { glyphs, fallback }
    }

    pub fn glyph(&self, ch: char) -> &[u8; GLYPH_H] {
        self.glyphs.get(&(ch as u32)).unwrap_or(&self.fallback)
    }

    pub fn has(&self, ch: char) -> bool {
        self.glyphs.contains_key(&(ch as u32))
    }

    pub fn len(&self) -> usize {
        self.glyphs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// Iterate all (code point, rows) pairs, for atlas building.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &[u8; GLYPH_H])> {
        self.glyphs.iter().map(|(k, v)| (*k, v))
    }

    /// Is pixel (x, y) of this glyph set?
    pub fn pixel(rows: &[u8; GLYPH_H], x: usize, y: usize) -> bool {
        rows[y] & (0x80 >> x) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_ascii_and_box_drawing() {
        let f = Font::builtin();
        assert!(f.len() > 1000);
        for c in ' '..='~' {
            assert!(f.has(c), "missing {c:?}");
        }
        assert!(f.has('\u{2500}'));
        assert!(f.has('\u{2588}'));
        // 'A' has its crossbar somewhere in the middle rows.
        let a = f.glyph('A');
        assert!(a.contains(&0x7E));
    }
}
