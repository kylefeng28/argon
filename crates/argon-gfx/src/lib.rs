//! wgpu renderer for Argon: a persistent virtual framebuffer, a text layer
//! composited over it, user images as render targets, and an integer-scaled
//! presenter. Also runs headless for CI and screenshots.

mod atlas;
mod renderer;

pub use atlas::{Atlas, FontMode, Fonts};
pub use renderer::{DisplayFilter, Gpu, PresentTarget, Renderer};
pub use wgpu;
