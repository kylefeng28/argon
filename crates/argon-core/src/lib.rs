//! Shared, dependency-light types for Argon: colors, the bitmap font, the
//! text grid, draw commands, input snapshots, the virtual disk and config.
//!
//! This crate has no GPU, audio or Lua dependencies so that every other crate
//! (and a future browser build) can share it.

pub mod color;
pub mod draw;
pub mod font;
pub mod input;
pub mod raster;
pub mod text;
pub mod theme;

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
