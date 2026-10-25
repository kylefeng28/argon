//! The windowed application (winit + wgpu).

use crate::keymap;
use argon_core::input::{KeyCode, MouseButton};
use argon_gfx::{wgpu, DisplayFilter, FontMode, Gpu, PresentTarget, Renderer};
use argon_shell::Console;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;
use winit::window::{Fullscreen, Window, WindowId};

struct Graphics {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
}

struct App {
    console: Console,
    gfx: Option<Graphics>,
    mods: ModifiersState,
    last_frame: Instant,
    startup_program: Option<String>,
    /// Where the virtual screen sits in the window (x, y, w, h), for mouse mapping.
    screen_rect: (f32, f32, f32, f32),
}

pub fn run(console: Console, startup_program: Option<String>) -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App { console, gfx: None, mods: ModifiersState::empty(), last_frame: Instant::now(), startup_program, screen_rect: (0.0, 0.0, 1.0, 1.0) };
    event_loop.run_app(&mut app)?;
    Ok(())
}

impl App {
    fn init_graphics(&mut self, event_loop: &ActiveEventLoop) {
        // The window is a multiple of the 640x400 shell cell layout: at least
        // 2x (1280x800), or as large as fits 85% of the monitor.
        let (bw, bh) = (640u32, 400u32);
        let scale = if self.console.config.window_scale > 0 {
            self.console.config.window_scale
        } else {
            event_loop
                .primary_monitor()
                .map(|m| {
                    let s = m.size();
                    ((s.width * 85 / 100) / bw).min((s.height * 85 / 100) / bh).max(2)
                })
                .unwrap_or(2)
        };
        let attrs = Window::default_attributes()
            .with_title("Argon")
            .with_inner_size(PhysicalSize::new(bw * scale, bh * scale))
            .with_min_inner_size(PhysicalSize::new(bw, bh));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        window.set_cursor_visible(false);

        if self.console.config.fullscreen {
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or(wgpu::Backends::all()),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).expect("create surface");
        let gpu = Gpu::new_with_instance(instance, Some(&surface)).expect("no GPU");
        let caps = surface.get_capabilities(&gpu.adapter);
        // Prefer a non-sRGB format: Argon's colors are exact byte values.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&gpu.device, &config);
        let gpu = Arc::new(gpu);
        self.console.set_window_size(config.width, config.height);
        let (vw, vh) = argon_core::shell_resolution_for_window(config.width, config.height);
        let renderer = Renderer::new(gpu, vw, vh);
        self.gfx = Some(Graphics { window, surface, config, renderer });

        if let Some(p) = self.startup_program.take() {
            self.console.execute_line(&format!("RUN {p}"));
        }
    }

    fn toggle_fullscreen(&mut self) {
        if let Some(g) = &self.gfx {
            let fs = g.window.fullscreen().is_none();
            g.window.set_fullscreen(if fs { Some(Fullscreen::Borderless(None)) } else { None });
            self.console.config.fullscreen = fs;
            let _ = self.console.config.save(&self.console.paths.config);
        }
    }

    fn redraw(&mut self) {
        let Some(g) = &mut self.gfx else { return };
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f64().min(0.1);
        self.last_frame = now;

        let out = self.console.update(dt);
        if out.resolution != g.renderer.size() {
            g.renderer.resize(out.resolution.0, out.resolution.1);
        }
        let (vw, vh) = g.renderer.size();
        let scale = (g.config.width / vw).min(g.config.height / vh).max(1);
        g.renderer.set_display_scale(scale);
        let (dw, dh) = ((vw * scale) as f32, (vh * scale) as f32);
        self.screen_rect = ((g.config.width as f32 - dw) / 2.0, (g.config.height as f32 - dh) / 2.0, dw, dh);
        g.renderer.set_font_mode(FontMode::parse(&self.console.config.font).unwrap_or_default());
        if self.console.program_started {
            g.renderer.reset_program_state();
            self.console.program_started = false;
        }
        g.renderer.clear_color = out.clear_color;
        g.renderer.filter = DisplayFilter::parse(self.console.filter_name()).unwrap_or_default();
        g.renderer.render(&out.cmds, Some(&out.grid), out.cursor_on);

        let frame = match g.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                g.surface.configure(&g.renderer.gpu.device, &g.config);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("out of GPU memory");
                std::process::exit(1);
            }
            Err(_) => return,
        };
        let view = frame.texture.create_view(&Default::default());
        g.renderer.present(PresentTarget { view: &view, width: g.config.width, height: g.config.height, format: g.config.format });
        g.window.pre_present_notify();
        frame.present();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_none() {
            self.init_graphics(event_loop);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(g) = &self.gfx {
            g.window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(g) = &mut self.gfx {
                    g.config.width = size.width.max(1);
                    g.config.height = size.height.max(1);
                    g.surface.configure(&g.renderer.gpu.device, &g.config);
                }
                self.console.set_window_size(size.width, size.height);
            }
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
            WindowEvent::CursorMoved { position, .. } => {
                self.console.mouse_move(position.x as f32, position.y as f32, self.screen_rect);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let b = match button {
                    winit::event::MouseButton::Left => Some(MouseButton::Left),
                    winit::event::MouseButton::Right => Some(MouseButton::Right),
                    winit::event::MouseButton::Middle => Some(MouseButton::Middle),
                    _ => None,
                };
                if let Some(b) = b {
                    self.console.mouse_button(b, state == ElementState::Pressed);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (x, y),
                    winit::event::MouseScrollDelta::PixelDelta(p) => ((p.x / 40.0) as f32, (p.y / 40.0) as f32),
                };
                self.console.mouse_wheel(dx, dy);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let ev = keymap::map_event(&event, self.mods);
                // Host shortcuts: F11 / Alt+Enter fullscreen.
                let fullscreen_key = ev.code == KeyCode::F11 || (ev.code == KeyCode::Enter && ev.mods.alt);
                if event.state == ElementState::Pressed && !event.repeat && fullscreen_key {
                    self.toggle_fullscreen();
                    return;
                }
                self.console.key(ev);
                if self.console.wants_quit() {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
                if self.console.wants_quit() {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}
