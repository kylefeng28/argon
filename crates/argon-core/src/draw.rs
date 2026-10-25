//! Renderer-agnostic draw commands. Programs record these on their own
//! thread; the renderer consumes them on the main thread.

use crate::color::Color;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ImageId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ShaderId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BufferId(pub u32);

/// Options for `draw_points`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PointsOpts {
    /// Number of points (elements of the position buffer).
    pub count: u32,
    /// Floats per element in the position buffer (2 for vec2, 4 for vec4);
    /// the first two are x, y in pixels.
    pub stride: u32,
    /// Optional per-point RGBA (vec4, 0..1), multiplied with `color`.
    pub colors: Option<BufferId>,
    pub color: crate::Color,
    /// Square size in pixels.
    pub size: f32,
}

/// Per-draw inputs for a shader: which images feed iChannel0..3, user
/// uniform values by slot, and the Shadertoy globals the program knows
/// (time, mouse). The renderer fills resolution and offset.
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderInputs {
    pub channels: [Option<ImageId>; 4],
    /// Storage buffers by binding number (group 1).
    pub buffers: [Option<BufferId>; crate::shader::MAX_BUFFERS],
    pub user: [[f32; 4]; crate::shader::MAX_USER_UNIFORMS],
    pub time: f32,
    pub time_delta: f32,
    pub frame: i32,
    pub frame_rate: f32,
    pub mouse: [f32; 4],
}

impl Default for ShaderInputs {
    fn default() -> Self {
        ShaderInputs {
            channels: [None; 4],
            buffers: [None; crate::shader::MAX_BUFFERS],
            user: [[0.0; 4]; crate::shader::MAX_USER_UNIFORMS],
            time: 0.0,
            time_delta: 1.0 / 60.0,
            frame: 0,
            frame_rate: 60.0,
            mouse: [0.0; 4],
        }
    }
}

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
    /// Copy the screen framebuffer into the saved buffer (alt screen).
    SaveScreen,
    /// Copy the saved buffer back onto the screen framebuffer.
    RestoreScreen,
    /// Register a compiled fragment shader.
    CreateShader { id: ShaderId, shader: std::sync::Arc<crate::shader::CompiledShader> },
    DestroyShader(ShaderId),
    /// Run a shader over the whole target or a rectangle of it.
    DrawShader { id: ShaderId, rect: Option<Rect>, inputs: ShaderInputs },
    /// Apply a shader to subsequent primitive/sprite draws (`None` = off).
    SetShader { id: Option<ShaderId>, inputs: ShaderInputs },
    /// Allocate a zeroed GPU storage buffer of `bytes` bytes.
    CreateBuffer { id: BufferId, bytes: u64 },
    DestroyBuffer(BufferId),
    /// Upload bytes at a byte offset (applied before this frame's GPU work).
    WriteBuffer { id: BufferId, offset: u64, data: Vec<u8> },
    /// Run a compute shader over `count` invocations (x, y, z).
    Dispatch { id: ShaderId, count: [u32; 3], inputs: ShaderInputs },
    /// Draw one square per element of a buffer, positions in pixels.
    DrawPoints { buffer: BufferId, opts: PointsOpts },
}

/// Everything the renderer needs for one frame.
#[derive(Clone, Debug, Default)]
pub struct FrameCommands {
    pub cmds: Vec<DrawCmd>,
}
