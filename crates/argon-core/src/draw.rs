//! Renderer-agnostic draw commands. Programs record these on their own
//! thread; the renderer consumes them on the main thread.

use crate::color::Color;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ImageId(pub u32);

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum BlendMode {
    #[default]
    Alpha,
    Add,
    Multiply,
    Replace,
}

#[derive(Clone, PartialEq, Debug)]
pub enum DrawCmd {
    /// Clear the current target.
    Clear(Color),
    Pixel { x: i32, y: i32, color: Color },
    Line { x1: i32, y1: i32, x2: i32, y2: i32, color: Color },
    Rect { x: i32, y: i32, w: i32, h: i32, color: Color, fill: bool },
    /// Pixel text in the console font, top-left at (x, y).
    Text { x: i32, y: i32, text: String, color: Color, scale: u32 },
    SetLineWidth(u32),
}

/// Everything the renderer needs for one frame.
#[derive(Clone, Debug, Default)]
pub struct FrameCommands {
    pub cmds: Vec<DrawCmd>,
}
