//! Headless runner: boots the console, feeds a key script, renders N frames
//! offscreen and writes a screenshot. Used by CI and by developers without a
//! display.

use crate::script::{self, Step};
use crate::HeadlessArgs;
use argon_gfx::{DisplayFilter, Gpu, Renderer};
use argon_shell::Console;
use std::collections::VecDeque;
use std::sync::Arc;

pub fn run(mut console: Console, args: &HeadlessArgs, program: Option<&str>) -> anyhow::Result<()> {
    let gpu = Arc::new(Gpu::new(None).map_err(|e| anyhow::anyhow!(e))?);
    let (w, h) = (argon_core::DEFAULT_WIDTH, argon_core::DEFAULT_HEIGHT);
    let mut renderer = Renderer::new(gpu, w, h);

    // Set fixed frame time for reproducibility (this also sets PRNG to a fixed seed)
    console.fixed_dt = Some(1.0 / 60.0);
    if let Some(p) = program {
        console.execute_line(&format!("RUN {p}"));
    }
    let mut steps: VecDeque<Step> = match &args.keys {
        Some(s) => script::parse(s).map_err(|e| anyhow::anyhow!(e))?.into(),
        None => VecDeque::new(),
    };
    let mut wait = 0u32;
    let dt = 1.0 / 60.0;

    for _ in 0..args.frames {
        // Feed script: all keys until the next wait.
        if wait > 0 {
            wait -= 1;
        } else {
            while let Some(step) = steps.pop_front() {
                match step {
                    Step::Key(ev) => console.key(ev),
                    Step::Wait(n) => {
                        wait = n;
                        break;
                    }
                }
            }
        }
        let out = console.update(dt);
        if out.resolution != renderer.size() {
            renderer.resize(out.resolution.0, out.resolution.1);
        }
        if console.program_started {
            renderer.reset_program_state();
            console.program_started = false;
        }
        renderer.clear_color = out.clear_color;
        renderer.filter = DisplayFilter::parse(console.filter_name()).unwrap_or_default();
        renderer.render(&out.cmds, Some(&out.grid), out.cursor_on);
        if console.wants_quit() {
            break;
        }
        // Give the program thread a chance to produce the next frame at a
        // steady pace, like vsync would.
        std::thread::sleep(std::time::Duration::from_micros(2000));
    }

    if args.text {
        println!("{}", console.screen_text());
    }
    if let Some(path) = &args.screenshot {
        let (w, h, pixels) = if args.scale <= 1 {
            let (w, h) = renderer.size();
            (w, h, renderer.read_composite())
        } else {
            renderer.read_display(args.scale)
        };
        let img = image::RgbaImage::from_raw(w, h, pixels).ok_or_else(|| anyhow::anyhow!("bad framebuffer size"))?;
        img.save(path)?;
        eprintln!("wrote {} ({w}x{h})", path.display());
    }
    Ok(())
}
