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

impl BlendMode {
    pub fn parse(s: &str) -> Option<BlendMode> {
        match s {
            "alpha" => Some(BlendMode::Alpha),
            "add" => Some(BlendMode::Add),
            "multiply" => Some(BlendMode::Multiply),
            "replace" => Some(BlendMode::Replace),
            _ => None,
        }
    }
}

/// Options shared by `draw_image` and `draw_sprite`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SpriteOpts {
    pub scale_x: f32,
    pub scale_y: f32,
    /// Radians, clockwise on a y-down screen.
    pub rotation: f32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub tint: Color,
    /// Pivot inside the destination rectangle, in pixels of the source size.
    pub origin_x: f32,
    pub origin_y: f32,
}

impl Default for SpriteOpts {
    fn default() -> Self {
        SpriteOpts {
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
            flip_x: false,
            flip_y: false,
            tint: Color::WHITE,
            origin_x: 0.0,
            origin_y: 0.0,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum DrawCmd {
    /// Clear the current target.
    Clear(Color),
    Pixel { x: i32, y: i32, color: Color },
    Line { x1: i32, y1: i32, x2: i32, y2: i32, color: Color },
    Rect { x: i32, y: i32, w: i32, h: i32, color: Color, fill: bool },
    Circle { x: i32, y: i32, r: i32, color: Color, fill: bool },
    Ellipse { x: i32, y: i32, rx: i32, ry: i32, color: Color, fill: bool },
    Triangle { pts: [[f32; 2]; 3], color: Color, fill: bool },
    Polygon { pts: Vec<[f32; 2]>, color: Color, fill: bool },
    /// Pixel text in the console font, top-left at (x, y).
    Text { x: i32, y: i32, text: String, color: Color, scale: u32 },
    /// Upload a new RGBA8 image. Ids are allocated by the producer.
    CreateImage { id: ImageId, width: u32, height: u32, pixels: Vec<u8> },
    DestroyImage(ImageId),
    /// Draw `src` (pixels in the image) so that its top-left lands at (x, y)
    /// before scaling/rotation around the origin.
    Sprite { image: ImageId, src: Rect, x: f32, y: f32, opts: SpriteOpts },
    /// Arbitrary textured quad: four corners, UVs in 0..1.
    ImageQuad { image: ImageId, pts: [[f32; 2]; 4], uvs: [[f32; 2]; 4], tint: Color },
    SetClip(Option<Rect>),
    SetCamera { dx: f32, dy: f32 },
    SetBlend(BlendMode),
    /// Redirect drawing into an image (`None` = screen).
    SetTarget(Option<ImageId>),
    SetLineWidth(u32),
}

/// Everything the renderer needs for one frame.
#[derive(Clone, Debug, Default)]
pub struct FrameCommands {
    pub cmds: Vec<DrawCmd>,
}
