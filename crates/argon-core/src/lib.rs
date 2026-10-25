//! Shared, dependency-light types for Argon: colors, the bitmap font, the
//! text grid, draw commands, input snapshots, the virtual disk and config.
//!
//! This crate has no GPU, audio or Lua dependencies so that every other crate
//! (and a future browser build) can share it.

pub mod api;
pub mod color;
pub mod config;
pub mod draw;
pub mod font;
pub mod input;
pub mod raster;
pub mod shader;
pub mod text;
pub mod theme;
pub mod vfs;

pub use color::Color;
pub use draw::{DrawCmd, ImageId, Rect, ShaderId};
pub use font::Font;
pub use input::{InputSnapshot, KeyCode, KeyEvent, Modifiers, MouseButton, MouseState, PadButton};
pub use text::{Cell, TextGrid};
pub use theme::Theme;

/// Width of one text cell in pixels.
pub const CELL_W: u32 = 8;
/// Height of one text cell in pixels.
pub const CELL_H: u32 = 16;

/// Default program resolution (`hd`).
pub const DEFAULT_WIDTH: u32 = 1280;
pub const DEFAULT_HEIGHT: u32 = 800;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Named resolution presets (FR-C2).
pub fn resolution_preset(name: &str) -> Option<Resolution> {
    match name.to_ascii_lowercase().as_str() {
        "tiny" => Some(Resolution::Fixed(128, 128)),
        "retro" => Some(Resolution::Fixed(320, 240)),
        "classic" => Some(Resolution::Fixed(640, 400)),
        "hd" => Some(Resolution::Fixed(1280, 800)),
        "uhd" => Some(Resolution::Fixed(2560, 1600)),
        "native" => Some(Resolution::Native),
        _ => parse_wxh(name).map(|(w, h)| Resolution::Fixed(w, h)),
    }
}

/// A requested virtual resolution: fixed pixels or the host window's size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Resolution {
    Fixed(u32, u32),
    /// The window's own pixels (whatever the display gives us).
    Native,
}

impl Resolution {
    /// Concrete size given the current window size.
    pub fn resolve(self, window: (u32, u32)) -> (u32, u32) {
        match self {
            Resolution::Fixed(w, h) => (w, h),
            Resolution::Native => (window.0.clamp(16, 8192), window.1.clamp(16, 8192)),
        }
    }
}

/// How many times the 8x16 base cell is scaled at a given screen height, so
/// the text grid stays around 80x25 instead of growing with the pixel count:
/// 1 at 400 rows (classic), 2 at 800 (hd), 3 at 1200, 5 at 2160.
pub fn text_scale_for_height(height: u32) -> u32 {
    ((height as f32 / 400.0).round() as u32).max(1)
}

/// Largest 16:10 multiple of 640x400 that fits a window: the shell's
/// resolution (FR-C2, D3).
pub fn shell_resolution_for_window(win_w: u32, win_h: u32) -> (u32, u32) {
    let k = (win_w / 640).min(win_h / 400).max(1);
    (640 * k, 400 * k)
}

fn parse_wxh(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once('x')?;
    let w: u32 = w.trim().parse().ok()?;
    let h: u32 = h.trim().parse().ok()?;
    if (16..=8192).contains(&w) && (16..=8192).contains(&h) {
        Some((w, h))
    } else {
        None
    }
}
