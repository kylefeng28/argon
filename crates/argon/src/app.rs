//! The windowed application (winit + wgpu).

use crate::keymap;
use argon_core::input::KeyCode;
use argon_gfx::{wgpu, Gpu, PresentTarget, Renderer};
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
}

pub fn run(console: Console, startup_program: Option<String>) -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App { console, gfx: None, mods: ModifiersState::empty(), last_frame: Instant::now(), startup_program };
    event_loop.run_app(&mut app)?;
    Ok(())
}

impl App {
    fn init_graphics(&mut self, event_loop: &ActiveEventLoop) {
        let (vw, vh) = (argon_core::DEFAULT_WIDTH, argon_core::DEFAULT_HEIGHT);
        let scale = if self.console.config.window_scale > 0 {
            self.console.config.window_scale
        } else {
            // Largest integer scale that leaves some margin on the monitor.
            event_loop
                .primary_monitor()
                .map(|m| {
                    let s = m.size();
                    ((s.width * 85 / 100) / vw).min((s.height * 85 / 100) / vh).max(1)
                })
                .unwrap_or(2)
        };
        let attrs = Window::default_attributes()
            .with_title("Argon")
            .with_inner_size(PhysicalSize::new(vw * scale, vh * scale))
            .with_min_inner_size(PhysicalSize::new(vw, vh));
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
        if self.console.program_started {
            g.renderer.reset_program_state();
            self.console.program_started = false;
        }
        g.renderer.clear_color = out.clear_color;
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
            }
            WindowEvent::ModifiersChanged(m) => self.mods = m.state(),
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
