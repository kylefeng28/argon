//! Glyph atlas. Glyphs are rasterized lazily on first use, at the atlas'
//! scale, from the TrueType console font (Share Tech Mono) with the Unscii
//! bitmap font as fallback for code points the TTF lacks (box drawing,
//! blocks, arrows). Slot 0 is a solid white block used for untextured
//! geometry.

use argon_core::font::{Font, GLYPH_H, GLYPH_W};
use std::collections::HashMap;
use std::sync::Arc;

const SLOTS_PER_ROW: u32 = 64;
const SLOT_ROWS: u32 = 48;

pub const SHARE_TECH_MONO: &[u8] = include_bytes!("../../../assets/fonts/ShareTechMono-Regular.ttf");
pub const PROGGY_CLEAN: &[u8] = include_bytes!("../../../assets/fonts/ProggyClean.ttf");

/// Which face renders text.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FontMode {
    /// Share Tech Mono (OFL): squared, futuristic, antialiased.
    #[default]
    Tech,
    /// ProggyClean (MIT): the crisp programmer's bitmap-style TTF.
    Proggy,
    /// Unscii bitmap only.
    Pixel,
}

impl FontMode {
    pub fn parse(s: &str) -> Option<FontMode> {
        match s.to_ascii_lowercase().as_str() {
            "tech" | "sharetech" | "ttf" => Some(FontMode::Tech),
            "proggy" | "proggyclean" => Some(FontMode::Proggy),
            "pixel" | "unscii" | "bitmap" => Some(FontMode::Pixel),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FontMode::Tech => "tech",
            FontMode::Proggy => "proggy",
            FontMode::Pixel => "pixel",
        }
    }

    pub const NAMES: &'static [&'static str] = &["tech", "proggy", "pixel"];
}

pub struct Fonts {
    pub bitmap: Font,
    pub tech: fontdue::Font,
    pub proggy: fontdue::Font,
}

impl Fonts {
    pub fn load() -> Arc<Fonts> {
        let tech = fontdue::Font::from_bytes(SHARE_TECH_MONO, fontdue::FontSettings::default()).expect("embedded TTF");
        let proggy = fontdue::Font::from_bytes(PROGGY_CLEAN, fontdue::FontSettings::default()).expect("embedded TTF");
        Arc::new(Fonts { bitmap: Font::builtin(), tech, proggy })
    }

    /// The TTF for a mode and its pixel size per unit of scale.
    fn face(&self, mode: FontMode) -> Option<(&fontdue::Font, f32)> {
        match mode {
            // Scale Share Tech Mono so its advance fills the 8 px cell.
            FontMode::Tech => {
                let adv = self.tech.metrics('M', 100.0).advance_width / 100.0;
                Some((&self.tech, GLYPH_W as f32 / adv.max(0.1)))
            }
            // ProggyClean is drawn for a 7x13 cell at 16 px.
            FontMode::Proggy => Some((&self.proggy, 16.0)),
            FontMode::Pixel => None,
        }
    }
}

pub struct Atlas {
    fonts: Arc<Fonts>,
    mode: FontMode,
    pub scale: u32,
    pub slot_w: u32,
    pub slot_h: u32,
    pub width: u32,
    pub height: u32,
    slots: HashMap<u32, u32>,
    next_slot: u32,
    fallback_slot: Option<u32>,
    /// Rasterized glyphs waiting to be uploaded: (slot, rgba pixels).
    pending: Vec<(u32, Vec<u8>)>,
}

impl Atlas {
    pub fn new(fonts: Arc<Fonts>, mode: FontMode, scale: u32) -> Atlas {
        let scale = scale.max(1);
        let slot_w = GLYPH_W as u32 * scale;
        let slot_h = GLYPH_H as u32 * scale;
        let mut a = Atlas {
            fonts,
            mode,
            scale,
            slot_w,
            slot_h,
            width: SLOTS_PER_ROW * slot_w,
            height: SLOT_ROWS * slot_h,
            slots: HashMap::new(),
            next_slot: 1,
            fallback_slot: None,
            pending: Vec::new(),
        };
        // Slot 0: solid white.
        a.pending.push((0, vec![255u8; (slot_w * slot_h * 4) as usize]));
        a
    }

    pub fn mode(&self) -> FontMode {
        self.mode
    }

    fn slot_origin(&self, slot: u32) -> (u32, u32) {
        ((slot % SLOTS_PER_ROW) * self.slot_w, (slot / SLOTS_PER_ROW) * self.slot_h)
    }

    fn slot_uv(&self, slot: u32) -> [f32; 4] {
        let (x, y) = self.slot_origin(slot);
        [
            x as f32 / self.width as f32,
            y as f32 / self.height as f32,
            (x + self.slot_w) as f32 / self.width as f32,
            (y + self.slot_h) as f32 / self.height as f32,
        ]
    }

    /// UV in the middle of the solid white slot.
    pub fn white_uv(&self) -> [f32; 2] {
        [(self.slot_w as f32 / 2.0) / self.width as f32, (self.slot_h as f32 / 2.0) / self.height as f32]
    }

    /// UV rectangle of a glyph, rasterizing it on first use.
    pub fn glyph_uv(&mut self, ch: char) -> [f32; 4] {
        let cp = ch as u32;
        if let Some(&s) = self.slots.get(&cp) {
            return self.slot_uv(s);
        }
        if self.next_slot >= SLOTS_PER_ROW * SLOT_ROWS {
            let fb = self.fallback();
            return self.slot_uv(fb);
        }
        let slot = self.next_slot;
        self.next_slot += 1;
        let pixels = self.rasterize(ch);
        self.pending.push((slot, pixels));
        self.slots.insert(cp, slot);
        self.slot_uv(slot)
    }

    fn fallback(&mut self) -> u32 {
        if let Some(s) = self.fallback_slot {
            return s;
        }
        let slot = self.next_slot.min(SLOTS_PER_ROW * SLOT_ROWS - 1);
        let pixels = self.rasterize('\u{FFFD}');
        self.pending.push((slot, pixels));
        self.fallback_slot = Some(slot);
        slot
    }

    fn rasterize(&self, ch: char) -> Vec<u8> {
        let (w, h) = (self.slot_w as usize, self.slot_h as usize);
        let mut out = vec![0u8; w * h * 4];
        for px in out.chunks_mut(4) {
            px[0] = 255;
            px[1] = 255;
            px[2] = 255;
        }
        let face = self.fonts.face(self.mode).filter(|(f, _)| f.lookup_glyph_index(ch) != 0 && !ch.is_control());
        if let Some((ttf, base_px)) = face {
            let px_size = base_px * self.scale as f32;
            let (m, cov) = ttf.rasterize(ch, px_size);
            let lm = ttf.horizontal_line_metrics(px_size).unwrap();
            let total = lm.ascent - lm.descent;
            let baseline = (self.slot_h as f32 * (lm.ascent / total)).round() as i32;
            let x0 = ((self.slot_w as f32 - m.advance_width) / 2.0).round() as i32 + m.xmin;
            let y0 = baseline - (m.ymin + m.height as i32);
            for gy in 0..m.height {
                for gx in 0..m.width {
                    let x = x0 + gx as i32;
                    let y = y0 + gy as i32;
                    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                        continue;
                    }
                    let a = cov[gy * m.width + gx];
                    let i = (y as usize * w + x as usize) * 4 + 3;
                    out[i] = out[i].max(a);
                }
            }
        } else {
            let rows = self.fonts.bitmap.glyph(ch);
            let s = self.scale as usize;
            for y in 0..h {
                for x in 0..w {
                    if Font::pixel(rows, x / s, y / s) {
                        out[(y * w + x) * 4 + 3] = 255;
                    }
                }
            }
        }
        out
    }

    /// Upload newly rasterized glyphs.
    pub fn flush(&mut self, queue: &wgpu::Queue, texture: &wgpu::Texture) {
        for (slot, pixels) in self.pending.drain(..) {
            let (x, y) = ((slot % SLOTS_PER_ROW) * self.slot_w, (slot / SLOTS_PER_ROW) * self.slot_h);
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                &pixels,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(self.slot_w * 4), rows_per_image: None },
                wgpu::Extent3d { width: self.slot_w, height: self.slot_h, depth_or_array_layers: 1 },
            );
        }
    }
}
