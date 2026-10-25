//! Glyph atlas built from the bitmap font. Slot 0 is a solid white block used
//! as the texture for untextured geometry.

use argon_core::font::{Font, GLYPH_H, GLYPH_W};
use std::collections::HashMap;

pub const SLOT_W: u32 = GLYPH_W as u32;
pub const SLOT_H: u32 = GLYPH_H as u32;
const SLOTS_PER_ROW: u32 = 64;

pub struct Atlas {
    pub width: u32,
    pub height: u32,
    /// RGBA8, white with alpha coverage.
    pub pixels: Vec<u8>,
    slots: HashMap<u32, u32>,
    fallback_slot: u32,
}

impl Atlas {
    pub fn build(font: &Font) -> Atlas {
        let mut cps: Vec<(u32, &[u8; GLYPH_H])> = font.iter().collect();
        cps.sort_by_key(|(cp, _)| *cp);
        let n = cps.len() as u32 + 1; // + solid slot
        let rows = n.div_ceil(SLOTS_PER_ROW);
        let width = SLOTS_PER_ROW * SLOT_W;
        let height = rows * SLOT_H;
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let mut slots = HashMap::with_capacity(cps.len());

        // Slot 0: solid white.
        blit(&mut pixels, width, 0, &[0xFF; GLYPH_H]);

        for (i, (cp, rows)) in cps.iter().enumerate() {
            let slot = i as u32 + 1;
            blit(&mut pixels, width, slot, rows);
            slots.insert(*cp, slot);
        }
        let fallback_slot = slots.get(&0xFFFD).copied().unwrap_or_else(|| slots.get(&('?' as u32)).copied().unwrap_or(0));
        Atlas { width, height, pixels, slots, fallback_slot }
    }

    fn slot_origin(slot: u32) -> (u32, u32) {
        ((slot % SLOTS_PER_ROW) * SLOT_W, (slot / SLOTS_PER_ROW) * SLOT_H)
    }

    /// UV rectangle (u0, v0, u1, v1) of a glyph.
    pub fn glyph_uv(&self, ch: char) -> [f32; 4] {
        let slot = self.slots.get(&(ch as u32)).copied().unwrap_or(self.fallback_slot);
        self.slot_uv(slot)
    }

    pub fn has_glyph(&self, ch: char) -> bool {
        self.slots.contains_key(&(ch as u32))
    }

    /// UV in the middle of the solid white slot.
    pub fn white_uv(&self) -> [f32; 2] {
        let (x, y) = Self::slot_origin(0);
        [(x as f32 + SLOT_W as f32 / 2.0) / self.width as f32, (y as f32 + SLOT_H as f32 / 2.0) / self.height as f32]
    }

    fn slot_uv(&self, slot: u32) -> [f32; 4] {
        let (x, y) = Self::slot_origin(slot);
        [
            x as f32 / self.width as f32,
            y as f32 / self.height as f32,
            (x + SLOT_W) as f32 / self.width as f32,
            (y + SLOT_H) as f32 / self.height as f32,
        ]
    }
}

fn blit(pixels: &mut [u8], atlas_w: u32, slot: u32, rows: &[u8; GLYPH_H]) {
    let (ox, oy) = Atlas::slot_origin(slot);
    for (y, row) in rows.iter().enumerate() {
        for x in 0..GLYPH_W {
            let on = row & (0x80 >> x) != 0;
            let idx = (((oy as usize + y) * atlas_w as usize) + ox as usize + x) * 4;
            pixels[idx..idx + 4].copy_from_slice(&[255, 255, 255, if on { 255 } else { 0 }]);
        }
    }
}
