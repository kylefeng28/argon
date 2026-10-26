use crate::atlas::{Atlas, FontMode, Fonts};
use argon_core::draw::{BlendMode, DrawCmd, ImageId, Rect, SpriteOpts};
use argon_core::raster::{self, Span};
use argon_core::{Color, TextGrid, CELL_H, CELL_W};
use std::collections::HashMap;
use std::sync::Arc;

const FB_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [u8; 4],
}

impl Vertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4],
    };
}

/// Device handles shared by everything on the main thread.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// Create a device. `compatible_surface` steers adapter selection when
    /// presenting to a window; `None` for headless.
    pub fn new(compatible_surface: Option<&wgpu::Surface<'_>>) -> Result<Gpu, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::all()),
            ..Default::default()
        });
        Gpu::new_with_instance(instance, compatible_surface)
    }

    pub fn new_with_instance(instance: wgpu::Instance, compatible_surface: Option<&wgpu::Surface<'_>>) -> Result<Gpu, String> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface,
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| "no compatible GPU adapter found".to_string())?;
        log::info!("adapter: {:?}", adapter.get_info());
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("argon"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|e| e.to_string())?;
        Ok(Gpu { instance, adapter, device, queue })
    }
}

struct TexBundle {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    w: u32,
    h: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum TexKey {
    Atlas,
    Image(ImageId),
    /// Not a draw: copy framebuffer -> saved buffer (alt screen).
    SaveScreen,
    /// Not a draw: copy saved buffer -> framebuffer.
    RestoreScreen,
}

impl TexKey {
    fn is_marker(self) -> bool {
        matches!(self, TexKey::SaveScreen | TexKey::RestoreScreen)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Batch {
    target: Option<ImageId>,
    blend: BlendMode,
    clip: Option<Rect>,
    tex: TexKey,
    start: u32,
    end: u32,
}

/// Mutable draw state carried across commands and frames.
#[derive(Clone, Copy, Debug)]
struct DrawState {
    clip: Option<Rect>,
    camera: (f32, f32),
    blend: BlendMode,
    target: Option<ImageId>,
    line_width: u32,
}

impl Default for DrawState {
    fn default() -> Self {
        DrawState { clip: None, camera: (0.0, 0.0), blend: BlendMode::Alpha, target: None, line_width: 1 }
    }
}

/// Display filter applied by the presenter (FR-C4). Bits: 1 glow,
/// 2 scanlines, 4 vignette.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct DisplayFilter(pub u32);

impl DisplayFilter {
    pub const NONE: DisplayFilter = DisplayFilter(0);
    pub const GLOW: DisplayFilter = DisplayFilter(1);
    pub const SCANLINES: DisplayFilter = DisplayFilter(2);
    pub const GLASS: DisplayFilter = DisplayFilter(8);
    /// glow + glass + vignette: the light-ribbon look.
    pub const NEON: DisplayFilter = DisplayFilter(1 | 4 | 8);
    pub const CRT: DisplayFilter = DisplayFilter(1 | 2 | 4);

    pub fn parse(name: &str) -> Option<DisplayFilter> {
        match name.to_ascii_lowercase().as_str() {
            "none" | "off" => Some(Self::NONE),
            "glow" | "bloom" => Some(Self::GLOW),
            "scanlines" => Some(Self::SCANLINES),
            "glass" | "water" => Some(Self::GLASS),
            "neon" => Some(Self::NEON),
            "crt" => Some(Self::CRT),
            _ => None,
        }
    }

    pub const NAMES: &'static [&'static str] = &["none", "glow", "glass", "scanlines", "neon", "crt"];
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PresentParams {
    texel: [f32; 2],
    mode: u32,
    rows: f32,
    rect: [f32; 4],
}

/// Where `present` draws: a surface texture view with its size and format.
pub struct PresentTarget<'a> {
    pub view: &'a wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

pub struct Renderer {
    pub gpu: Arc<Gpu>,
    shader: wgpu::ShaderModule,
    bgl: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    pipelines: HashMap<(BlendMode, wgpu::TextureFormat), wgpu::RenderPipeline>,
    present_shader: wgpu::ShaderModule,
    present_bgl: wgpu::BindGroupLayout,
    present_layout: wgpu::PipelineLayout,
    present_pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    present_params: wgpu::Buffer,
    present_bind_group: Option<wgpu::BindGroup>,
    pub filter: DisplayFilter,
    sampler: wgpu::Sampler,
    fonts: Arc<Fonts>,
    font_mode: FontMode,
    /// Glyphs at framebuffer scale (for `draw_text`).
    atlas: Atlas,
    atlas_tex: TexBundle,
    /// Glyphs at display scale (for the text layer).
    ui_atlas: Atlas,
    ui_atlas_tex: TexBundle,
    /// Integer factor between the framebuffer and the composite/window.
    display_scale: u32,
    framebuffer: TexBundle,
    composite: TexBundle,
    /// Copy of the framebuffer taken by `SaveScreen`.
    saved_screen: Option<TexBundle>,
    images: HashMap<ImageId, TexBundle>,
    vbuf: wgpu::Buffer,
    verts: Vec<Vertex>,
    batches: Vec<Batch>,
    spans: Vec<Span>,
    state: DrawState,
    width: u32,
    height: u32,
    /// Background color for the composite (shows where the text layer has
    /// no background and the framebuffer is transparent).
    pub clear_color: Color,
}

impl Renderer {
    pub fn new(gpu: Arc<Gpu>, width: u32, height: u32) -> Renderer {
        let device = &gpu.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("argon-batch"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("argon-tex"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("argon-batch"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let present_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("argon-present"),
            source: wgpu::ShaderSource::Wgsl(include_str!("present.wgsl").into()),
        });
        let present_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("argon-present"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let present_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("argon-present"),
            bind_group_layouts: &[&present_bgl],
            push_constant_ranges: &[],
        });
        let present_params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("present-params"),
            size: std::mem::size_of::<PresentParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let fonts = Fonts::load();
        let font_mode = FontMode::default();
        let atlas = Atlas::new(fonts.clone(), font_mode, 1);
        let atlas_tex = create_texture(device, &bgl, &sampler, atlas.width, atlas.height, Some("atlas"));
        let ui_atlas = Atlas::new(fonts.clone(), font_mode, 1);
        let ui_atlas_tex = create_texture(device, &bgl, &sampler, ui_atlas.width, ui_atlas.height, Some("ui-atlas"));

        let framebuffer = create_texture(device, &bgl, &sampler, width, height, Some("framebuffer"));
        let composite = create_texture(device, &bgl, &sampler, width, height, Some("composite"));
        let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("verts"),
            size: 1 << 20,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut r = Renderer {
            gpu,
            shader,
            bgl,
            pipeline_layout,
            pipelines: HashMap::new(),
            present_shader,
            present_bgl,
            present_layout,
            present_pipelines: HashMap::new(),
            present_params,
            present_bind_group: None,
            filter: DisplayFilter::NONE,
            sampler,
            fonts,
            font_mode,
            atlas,
            atlas_tex,
            ui_atlas,
            ui_atlas_tex,
            display_scale: 1,
            framebuffer,
            composite,
            saved_screen: None,
            images: HashMap::new(),
            vbuf,
            verts: Vec::new(),
            batches: Vec::new(),
            spans: Vec::new(),
            state: DrawState::default(),
            width,
            height,
            clear_color: Color::BLACK,
        };
        r.clear_framebuffer(Color::BLACK);
        r
    }

    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn display_scale(&self) -> u32 {
        self.display_scale
    }

    /// Set the integer factor between the framebuffer and the window. The
    /// text layer is rasterized at this scale so glyphs stay sharp.
    pub fn set_display_scale(&mut self, scale: u32) {
        let scale = scale.clamp(1, 8);
        if scale == self.display_scale {
            return;
        }
        self.display_scale = scale;
        self.rebuild_ui_atlas();
        self.rebuild_composite();
    }

    pub fn font_mode(&self) -> FontMode {
        self.font_mode
    }

    pub fn set_font_mode(&mut self, mode: FontMode) {
        if mode == self.font_mode {
            return;
        }
        self.font_mode = mode;
        self.atlas = Atlas::new(self.fonts.clone(), mode, 1);
        self.atlas_tex = create_texture(&self.gpu.device, &self.bgl, &self.sampler, self.atlas.width, self.atlas.height, Some("atlas"));
        self.rebuild_ui_atlas();
    }

    fn rebuild_ui_atlas(&mut self) {
        self.ui_atlas = Atlas::new(self.fonts.clone(), self.font_mode, self.display_scale);
        self.ui_atlas_tex =
            create_texture(&self.gpu.device, &self.bgl, &self.sampler, self.ui_atlas.width, self.ui_atlas.height, Some("ui-atlas"));
    }

    fn rebuild_composite(&mut self) {
        let (cw, ch) = (self.width * self.display_scale, self.height * self.display_scale);
        self.composite = create_texture(&self.gpu.device, &self.bgl, &self.sampler, cw, ch, Some("composite"));
        self.present_bind_group = None;
    }

    /// Change the virtual resolution. Clears the framebuffer.
    pub fn resize(&mut self, width: u32, height: u32) {
        if (width, height) == (self.width, self.height) {
            return;
        }
        self.width = width;
        self.height = height;
        self.framebuffer = create_texture(&self.gpu.device, &self.bgl, &self.sampler, width, height, Some("framebuffer"));
        self.rebuild_composite();
        self.state = DrawState::default();
        self.clear_framebuffer(Color::BLACK);
    }

    /// Forget per-program state: clip, camera, blend, target, images.
    pub fn reset_program_state(&mut self) {
        self.state = DrawState::default();
        self.images.clear();
    }

    pub fn clear_framebuffer(&mut self, color: Color) {
        let mut enc = self.gpu.device.create_command_encoder(&Default::default());
        clear_pass(&mut enc, &self.framebuffer.view, color);
        self.gpu.queue.submit([enc.finish()]);
    }

    fn pipeline(&mut self, blend: BlendMode, format: wgpu::TextureFormat) -> &wgpu::RenderPipeline {
        let key = (blend, format);
        if !self.pipelines.contains_key(&key) {
            let blend_state = match blend {
                BlendMode::Alpha => wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                },
                BlendMode::Add => wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                },
                BlendMode::Multiply => wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Dst,
                        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::Zero,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                },
                BlendMode::Replace => wgpu::BlendState::REPLACE,
            };
            let p = self.gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("argon-batch"),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::LAYOUT],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &self.shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend_state),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            });
            self.pipelines.insert(key, p);
        }
        &self.pipelines[&key]
    }

    // ----- geometry helpers -------------------------------------------------

    fn target_size(&self) -> (u32, u32) {
        match self.state.target {
            Some(id) => self.images.get(&id).map(|t| (t.w, t.h)).unwrap_or((self.width, self.height)),
            None => (self.width, self.height),
        }
    }

    fn begin_batch(&mut self, tex: TexKey) {
        let start = self.verts.len() as u32;
        let st = self.state;
        if let Some(last) = self.batches.last_mut() {
            if last.tex == tex && last.blend == st.blend && last.target == st.target && last.clip == st.clip {
                return;
            }
            if last.start == last.end && !last.tex.is_marker() {
                // Empty batch: just retarget it.
                *last = Batch { target: st.target, blend: st.blend, clip: st.clip, tex, start, end: start };
                return;
            }
        }
        self.batches.push(Batch { target: st.target, blend: st.blend, clip: st.clip, tex, start, end: start });
    }

    fn end_batch(&mut self) {
        let end = self.verts.len() as u32;
        if let Some(last) = self.batches.last_mut() {
            last.end = end;
        }
    }

    /// Push a quad given pixel-space corners (any order forming a convex
    /// quad in sequence) with per-corner UVs.
    fn quad_px(&mut self, corners: [[f32; 2]; 4], uvs: [[f32; 2]; 4], color: Color, tw: u32, th: u32) {
        let c = color.to_bytes();
        let to_ndc = |p: [f32; 2]| [p[0] / tw as f32 * 2.0 - 1.0, 1.0 - p[1] / th as f32 * 2.0];
        let v = |i: usize| Vertex { pos: to_ndc(corners[i]), uv: uvs[i], color: c };
        self.verts.extend_from_slice(&[v(0), v(1), v(2), v(0), v(2), v(3)]);
    }

    #[allow(clippy::too_many_arguments)]
    fn solid_rect_px(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, color: Color, tw: u32, th: u32) {
        let w = self.atlas.white_uv();
        self.quad_px([[x0, y0], [x1, y0], [x1, y1], [x0, y1]], [w, w, w, w], color, tw, th);
    }

    fn flush_spans(&mut self, color: Color) {
        let (tw, th) = self.target_size();
        let (cx, cy) = (self.state.camera.0.round(), self.state.camera.1.round());
        let spans = std::mem::take(&mut self.spans);
        for s in &spans {
            // Skip spans fully outside the target to keep vertex counts sane.
            let y = s.y as f32 + cy;
            if y < 0.0 || y >= th as f32 {
                continue;
            }
            let x0 = (s.x0 as f32 + cx).max(0.0);
            let x1 = (s.x1 as f32 + 1.0 + cx).min(tw as f32);
            if x1 <= x0 {
                continue;
            }
            self.solid_rect_px(x0, y, x1, y + 1.0, color, tw, th);
        }
        self.spans = spans;
        self.spans.clear();
    }

    #[allow(clippy::too_many_arguments)]
    fn glyph_quad(&mut self, ch: char, x: f32, y: f32, scale: f32, color: Color, tw: u32, th: u32) {
        let [u0, v0, u1, v1] = self.atlas.glyph_uv(ch);
        let (w, h) = (CELL_W as f32 * scale, CELL_H as f32 * scale);
        self.quad_px(
            [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
            [[u0, v0], [u1, v0], [u1, v1], [u0, v1]],
            color,
            tw,
            th,
        );
    }

    // ----- command recording --------------------------------------------------

    fn record(&mut self, cmd: &DrawCmd) {
        match cmd {
            DrawCmd::SetClip(c) => self.state.clip = *c,
            DrawCmd::SetCamera { dx, dy } => self.state.camera = (*dx, *dy),
            DrawCmd::SetBlend(b) => self.state.blend = *b,
            DrawCmd::SetTarget(t) => {
                self.state.target = t.filter(|id| self.images.contains_key(id));
            }
            DrawCmd::SetLineWidth(w) => self.state.line_width = (*w).max(1),
            DrawCmd::SaveScreen | DrawCmd::RestoreScreen => {
                let tex = if matches!(cmd, DrawCmd::SaveScreen) { TexKey::SaveScreen } else { TexKey::RestoreScreen };
                let start = self.verts.len() as u32;
                self.batches.push(Batch { target: None, blend: self.state.blend, clip: None, tex, start, end: start });
            }
            DrawCmd::CreateImage { id, width, height, pixels } => {
                let tex = create_texture(&self.gpu.device, &self.bgl, &self.sampler, *width, *height, Some("image"));
                if pixels.len() == (*width as usize) * (*height as usize) * 4 {
                    self.gpu.queue.write_texture(
                        tex.texture.as_image_copy(),
                        pixels,
                        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 4), rows_per_image: None },
                        wgpu::Extent3d { width: *width, height: *height, depth_or_array_layers: 1 },
                    );
                }
                self.images.insert(*id, tex);
            }
            DrawCmd::DestroyImage(id) => {
                self.images.remove(id);
                if self.state.target == Some(*id) {
                    self.state.target = None;
                }
            }
            DrawCmd::Clear(color) => {
                let (tw, th) = self.target_size();
                let saved = self.state.blend;
                self.state.blend = BlendMode::Replace;
                self.begin_batch(TexKey::Atlas);
                self.solid_rect_px(0.0, 0.0, tw as f32, th as f32, *color, tw, th);
                self.end_batch();
                self.state.blend = saved;
            }
            DrawCmd::Pixel { x, y, color } => {
                self.spans.push(Span::new(*y, *x, *x));
                self.emit_spans(*color);
            }
            DrawCmd::Line { x1, y1, x2, y2, color } => {
                if self.state.line_width <= 1 {
                    raster::line(*x1, *y1, *x2, *y2, &mut self.spans);
                } else {
                    let hw = self.state.line_width as f32 / 2.0;
                    let (dx, dy) = ((*x2 - *x1) as f32, (*y2 - *y1) as f32);
                    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
                    let (nx, ny) = (-dy / len * hw, dx / len * hw);
                    let (ax, ay, bx, by) = (*x1 as f32 + 0.5, *y1 as f32 + 0.5, *x2 as f32 + 0.5, *y2 as f32 + 0.5);
                    raster::fill_polygon(
                        &[[ax + nx, ay + ny], [bx + nx, by + ny], [bx - nx, by - ny], [ax - nx, ay - ny]],
                        &mut self.spans,
                    );
                }
                self.emit_spans(*color);
            }
            DrawCmd::Rect { x, y, w, h, color, fill } => {
                if *fill {
                    raster::fill_rect(*x, *y, *w, *h, &mut self.spans);
                } else {
                    raster::rect(*x, *y, *w, *h, &mut self.spans);
                }
                self.emit_spans(*color);
            }
            DrawCmd::Circle { x, y, r, color, fill } => {
                raster::ellipse(*x, *y, *r, *r, *fill, &mut self.spans);
                self.emit_spans(*color);
            }
            DrawCmd::Ellipse { x, y, rx, ry, color, fill } => {
                raster::ellipse(*x, *y, *rx, *ry, *fill, &mut self.spans);
                self.emit_spans(*color);
            }
            DrawCmd::Triangle { pts, color, fill } => {
                if *fill {
                    raster::fill_polygon(pts, &mut self.spans);
                } else {
                    raster::polygon_outline(pts, &mut self.spans);
                }
                self.emit_spans(*color);
            }
            DrawCmd::Polygon { pts, color, fill } => {
                if *fill {
                    raster::fill_polygon(pts, &mut self.spans);
                } else {
                    raster::polygon_outline(pts, &mut self.spans);
                }
                self.emit_spans(*color);
            }
            DrawCmd::Text { x, y, text, color, scale } => {
                let (tw, th) = self.target_size();
                let scale = (*scale).max(1) as f32;
                let (cx, cy) = self.state.camera;
                self.begin_batch(TexKey::Atlas);
                let mut px = *x as f32 + cx;
                let mut py = *y as f32 + cy;
                for ch in text.chars() {
                    if ch == '\n' {
                        px = *x as f32 + cx;
                        py += CELL_H as f32 * scale;
                        continue;
                    }
                    self.glyph_quad(ch, px, py, scale, *color, tw, th);
                    px += CELL_W as f32 * scale;
                }
                self.end_batch();
            }
            DrawCmd::Sprite { image, src, x, y, opts } => {
                let Some(img) = self.images.get(image) else { return };
                let (iw, ih) = (img.w as f32, img.h as f32);
                let (tw, th) = self.target_size();
                let (cx, cy) = self.state.camera;
                let corners = sprite_corners(*x + cx, *y + cy, src.w, src.h, opts);
                let (mut u0, mut v0, mut u1, mut v1) = (src.x / iw, src.y / ih, (src.x + src.w) / iw, (src.y + src.h) / ih);
                if opts.flip_x {
                    std::mem::swap(&mut u0, &mut u1);
                }
                if opts.flip_y {
                    std::mem::swap(&mut v0, &mut v1);
                }
                self.begin_batch(TexKey::Image(*image));
                self.quad_px(corners, [[u0, v0], [u1, v0], [u1, v1], [u0, v1]], opts.tint, tw, th);
                self.end_batch();
            }
            DrawCmd::ImageQuad { image, pts, uvs, tint } => {
                if !self.images.contains_key(image) {
                    return;
                }
                let (tw, th) = self.target_size();
                let (cx, cy) = self.state.camera;
                let pts = pts.map(|p| [p[0] + cx, p[1] + cy]);
                self.begin_batch(TexKey::Image(*image));
                self.quad_px(pts, *uvs, *tint, tw, th);
                self.end_batch();
            }
        }
    }

    fn emit_spans(&mut self, color: Color) {
        self.begin_batch(TexKey::Atlas);
        self.flush_spans(color);
        self.end_batch();
    }

    // ----- frame -----------------------------------------------------------------

    /// Render one frame: apply `cmds` to the framebuffer (and image targets),
    /// then composite the framebuffer with the text layer.
    pub fn render(&mut self, cmds: &[DrawCmd], grid: Option<&TextGrid>, cursor_on: bool) {
        self.verts.clear();
        self.batches.clear();
        for c in cmds {
            self.record(c);
        }
        let program_vert_end = self.verts.len() as u32;
        let program_batches = std::mem::take(&mut self.batches);

        // Composite geometry: framebuffer quad then text cells, in composite
        // pixel space (framebuffer size times the display scale).
        let (tw, th) = (self.width * self.display_scale, self.height * self.display_scale);
        let fb_start = self.verts.len() as u32;
        self.quad_px(
            [[0.0, 0.0], [tw as f32, 0.0], [tw as f32, th as f32], [0.0, th as f32]],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            Color::WHITE,
            tw,
            th,
        );
        let fb_end = self.verts.len() as u32;
        let text_start = fb_end;
        if let Some(grid) = grid {
            self.text_layer_verts(grid, cursor_on, tw, th);
        }
        let text_end = self.verts.len() as u32;

        self.upload_verts();
        self.atlas.flush(&self.gpu.queue, &self.atlas_tex.texture);
        self.ui_atlas.flush(&self.gpu.queue, &self.ui_atlas_tex.texture);

        let mut enc = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });

        // Program draws, grouped by target into passes.
        let mut i = 0;
        while i < program_batches.len() {
            if program_batches[i].tex.is_marker() {
                self.screen_copy(&mut enc, program_batches[i].tex == TexKey::SaveScreen);
                i += 1;
                continue;
            }
            let target = program_batches[i].target;
            let mut j = i;
            while j < program_batches.len() && program_batches[j].target == target && !program_batches[j].tex.is_marker() {
                j += 1;
            }
            // Pipelines must exist before we borrow views for the pass.
            for b in &program_batches[i..j] {
                self.pipeline(b.blend, FB_FORMAT);
            }
            let (view, tsz) = match target {
                Some(id) => match self.images.get(&id) {
                    Some(t) => (&t.view, (t.w, t.h)),
                    None => {
                        i = j;
                        continue;
                    }
                },
                None => (&self.framebuffer.view, (self.width, self.height)),
            };
            {
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("program"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    ..Default::default()
                });
                pass.set_vertex_buffer(0, self.vbuf.slice(..));
                for b in &program_batches[i..j] {
                    if b.start == b.end {
                        continue;
                    }
                    let Some(bg) = self.bind_group(b.tex) else { continue };
                    let (sx, sy, sw, sh) = clip_to_scissor(b.clip, tsz);
                    if sw == 0 || sh == 0 {
                        continue;
                    }
                    pass.set_scissor_rect(sx, sy, sw, sh);
                    pass.set_pipeline(&self.pipelines[&(b.blend, FB_FORMAT)]);
                    pass.set_bind_group(0, bg, &[]);
                    pass.draw(b.start..b.end, 0..1);
                }
            }
            i = j;
        }
        debug_assert!(program_vert_end <= fb_start);

        // Composite pass.
        self.pipeline(BlendMode::Replace, FB_FORMAT);
        self.pipeline(BlendMode::Alpha, FB_FORMAT);
        {
            let cc = self.clear_color.to_f32();
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: cc[0] as f64, g: cc[1] as f64, b: cc[2] as f64, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_vertex_buffer(0, self.vbuf.slice(..));
            pass.set_pipeline(&self.pipelines[&(BlendMode::Alpha, FB_FORMAT)]);
            pass.set_bind_group(0, &self.framebuffer.bind_group, &[]);
            pass.draw(fb_start..fb_end, 0..1);
            if text_end > text_start {
                pass.set_bind_group(0, &self.ui_atlas_tex.bind_group, &[]);
                pass.draw(text_start..text_end, 0..1);
            }
        }
        self.gpu.queue.submit([enc.finish()]);
    }

    /// Alt screen support: copy the framebuffer to the saved buffer or back.
    fn screen_copy(&mut self, enc: &mut wgpu::CommandEncoder, save: bool) {
        let (w, h) = (self.framebuffer.w, self.framebuffer.h);
        if save && self.saved_screen.as_ref().map(|t| (t.w, t.h) != (w, h)).unwrap_or(true) {
            self.saved_screen = Some(create_texture(&self.gpu.device, &self.bgl, &self.sampler, w, h, Some("saved-screen")));
        }
        let Some(saved) = &self.saved_screen else { return };
        if (saved.w, saved.h) != (w, h) {
            return;
        }
        let (src, dst) = if save { (&self.framebuffer, saved) } else { (saved, &self.framebuffer) };
        enc.copy_texture_to_texture(
            src.texture.as_image_copy(),
            dst.texture.as_image_copy(),
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }

    fn text_layer_verts(&mut self, grid: &TextGrid, cursor_on: bool, tw: u32, th: u32) {
        let cols = grid.cols();
        let s = self.display_scale;
        let (cw, chh) = ((CELL_W * s) as f32, (CELL_H * s) as f32);
        for (i, cell) in grid.cells().iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = (col as u32 * CELL_W * s) as f32;
            let y = (row as u32 * CELL_H * s) as f32;
            let is_cursor = cursor_on && grid.cursor_visible && col == grid.cursor_col && row == grid.cursor_row;
            let (fg, bg) = if is_cursor {
                (cell.bg.unwrap_or(self.clear_color), Some(cell.fg))
            } else {
                (cell.fg, cell.bg)
            };
            if let Some(bg) = bg {
                self.solid_rect_px(x, y, x + cw, y + chh, bg, tw, th);
            }
            if cell.ch != ' ' {
                let [u0, v0, u1, v1] = self.ui_atlas.glyph_uv(cell.ch);
                self.quad_px(
                    [[x, y], [x + cw, y], [x + cw, y + chh], [x, y + chh]],
                    [[u0, v0], [u1, v0], [u1, v1], [u0, v1]],
                    fg,
                    tw,
                    th,
                );
            }
        }
    }

    fn upload_verts(&mut self) {
        let bytes: &[u8] = bytemuck::cast_slice(&self.verts);
        if (bytes.len() as u64) > self.vbuf.size() {
            let size = (bytes.len() as u64).next_power_of_two();
            self.vbuf = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("verts"),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !bytes.is_empty() {
            self.gpu.queue.write_buffer(&self.vbuf, 0, bytes);
        }
    }

    fn bind_group(&self, key: TexKey) -> Option<&wgpu::BindGroup> {
        match key {
            TexKey::Atlas => Some(&self.atlas_tex.bind_group),
            TexKey::Image(id) => self.images.get(&id).map(|t| &t.bind_group),
            TexKey::SaveScreen | TexKey::RestoreScreen => None,
        }
    }

    fn present_pipeline(&mut self, format: wgpu::TextureFormat) {
        if self.present_pipelines.contains_key(&format) {
            return;
        }
        let p = self.gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("argon-present"),
            layout: Some(&self.present_layout),
            vertex: wgpu::VertexState {
                module: &self.present_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.present_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, cull_mode: None, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        self.present_pipelines.insert(format, p);
    }

    fn ensure_present_bind_group(&mut self) {
        if self.present_bind_group.is_none() {
            self.present_bind_group = Some(self.gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("argon-present"),
                layout: &self.present_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.composite.view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: self.present_params.as_entire_binding() },
                ],
            }));
        }
    }

    /// Blit the composite onto a window surface with integer scaling,
    /// letterboxing (FR-C3) and the current display filter (FR-C4).
    pub fn present(&mut self, target: PresentTarget<'_>) {
        let scale = (target.width / self.width).min(target.height / self.height).max(1);
        let (dw, dh) = (self.width * scale, self.height * scale);
        let (dw, dh) = (dw.min(target.width).max(1), dh.min(target.height).max(1));
        let ox = ((target.width as i64 - dw as i64) / 2).max(0) as f32;
        let oy = ((target.height as i64 - dh as i64) / 2).max(0) as f32;
        self.verts.clear();
        self.quad_px(
            [[ox, oy], [ox + dw as f32, oy], [ox + dw as f32, oy + dh as f32], [ox, oy + dh as f32]],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            Color::WHITE,
            target.width,
            target.height,
        );
        self.upload_verts();
        let params = PresentParams {
            // Filter radii are in framebuffer pixels regardless of scale.
            texel: [1.0 / self.width as f32, 1.0 / self.height as f32],
            mode: self.filter.0,
            rows: self.height as f32,
            rect: [ox, oy, dw as f32, dh as f32],
        };
        self.gpu.queue.write_buffer(&self.present_params, 0, bytemuck::bytes_of(&params));
        self.present_pipeline(target.format);
        self.ensure_present_bind_group();
        let mut enc = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("present") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_vertex_buffer(0, self.vbuf.slice(..));
            pass.set_pipeline(&self.present_pipelines[&target.format]);
            pass.set_bind_group(0, self.present_bind_group.as_ref().unwrap(), &[]);
            pass.draw(0..6, 0..1);
        }
        self.gpu.queue.submit([enc.finish()]);
    }

    /// Run the presenter into an offscreen texture at an integer scale and
    /// read it back (what the user would see, filter included).
    pub fn read_display(&mut self, scale: u32) -> (u32, u32, Vec<u8>) {
        let scale = scale.max(1);
        debug_assert_eq!(scale, self.display_scale, "set_display_scale before rendering");
        let (w, h) = (self.width * scale, self.height * scale);
        let tex = create_texture(&self.gpu.device, &self.bgl, &self.sampler, w, h, Some("display"));
        self.present(PresentTarget { view: &tex.view, width: w, height: h, format: FB_FORMAT });
        (w, h, self.read_texture(&tex))
    }

    /// Read back the composite as tightly packed RGBA8 (for screenshots and
    /// tests). Blocks until the GPU is done.
    pub fn read_composite(&self) -> Vec<u8> {
        self.read_texture(&self.composite)
    }

    /// Read back the framebuffer (what `get_pixel` sees).
    pub fn read_framebuffer(&self) -> Vec<u8> {
        self.read_texture(&self.framebuffer)
    }

    pub fn read_image(&self, id: ImageId) -> Option<(u32, u32, Vec<u8>)> {
        self.images.get(&id).map(|t| (t.w, t.h, self.read_texture(t)))
    }

    fn read_texture(&self, tex: &TexBundle) -> Vec<u8> {
        let padded = (tex.w * 4).div_ceil(256) * 256;
        let buf = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded * tex.h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self.gpu.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            tex.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: None },
            },
            wgpu::Extent3d { width: tex.w, height: tex.h, depth_or_array_layers: 1 },
        );
        self.gpu.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.gpu.device.poll(wgpu::Maintain::Wait);
        rx.recv().ok().and_then(|r| r.ok()).expect("map readback");
        let data = slice.get_mapped_range();
        let mut out = Vec::with_capacity((tex.w * tex.h * 4) as usize);
        for row in 0..tex.h {
            let s = (row * padded) as usize;
            out.extend_from_slice(&data[s..s + (tex.w * 4) as usize]);
        }
        drop(data);
        buf.unmap();
        out
    }
}

fn clip_to_scissor(clip: Option<Rect>, (tw, th): (u32, u32)) -> (u32, u32, u32, u32) {
    match clip {
        None => (0, 0, tw, th),
        Some(r) => {
            let x0 = r.x.max(0.0).min(tw as f32) as u32;
            let y0 = r.y.max(0.0).min(th as f32) as u32;
            let x1 = (r.x + r.w).max(0.0).min(tw as f32) as u32;
            let y1 = (r.y + r.h).max(0.0).min(th as f32) as u32;
            (x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0))
        }
    }
}

fn sprite_corners(x: f32, y: f32, w: f32, h: f32, o: &SpriteOpts) -> [[f32; 2]; 4] {
    let (sw, sh) = (w * o.scale_x, h * o.scale_y);
    let (ox, oy) = (o.origin_x * o.scale_x, o.origin_y * o.scale_y);
    // Corners relative to the pivot.
    let rel = [[-ox, -oy], [sw - ox, -oy], [sw - ox, sh - oy], [-ox, sh - oy]];
    let (s, c) = o.rotation.sin_cos();
    rel.map(|[px, py]| {
        if o.rotation == 0.0 {
            [x + px, y + py]
        } else {
            [x + px * c - py * s, y + px * s + py * c]
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn create_texture(
    device: &wgpu::Device,
    bgl: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    w: u32,
    h: u32,
    label: Option<&str>,
) -> TexBundle {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label,
        size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FB_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label,
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    });
    TexBundle { texture, view, bind_group, w: w.max(1), h: h.max(1) }
}

fn clear_pass(enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, color: Color) {
    let c = color.to_f32();
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color { r: c[0] as f64, g: c[1] as f64, b: c[2] as f64, a: c[3] as f64 }),
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
}
