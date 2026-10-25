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
pub mod text;
pub mod theme;
pub mod vfs;

pub use color::Color;
pub use draw::{DrawCmd, ImageId, Rect};
pub use font::Font;
pub use input::{InputSnapshot, KeyCode, KeyEvent, Modifiers, PadButton};
pub use text::{Cell, TextGrid};
pub use theme::Theme;

/// Width of one text cell in pixels.
pub const CELL_W: u32 = 8;
/// Height of one text cell in pixels.
pub const CELL_H: u32 = 16;

pub const DEFAULT_WIDTH: u32 = 640;
pub const DEFAULT_HEIGHT: u32 = 400;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Named resolution presets (FR-C2).
pub fn resolution_preset(name: &str) -> Option<(u32, u32)> {
    match name.to_ascii_lowercase().as_str() {
        "tiny" => Some((128, 128)),
        "retro" => Some((320, 240)),
        "classic" => Some((640, 400)),
        "hd" => Some((1280, 720)),
        _ => parse_wxh(name),
    }
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
