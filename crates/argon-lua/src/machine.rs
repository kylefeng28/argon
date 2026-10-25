//! Per-program state shared by all API functions, and the frame protocol.

use argon_audio::AudioCmd;
use argon_core::draw::{DrawCmd, ImageId};
use argon_core::input::KeyEvent;
use argon_core::vfs::Jail;
use argon_core::{Color, InputSnapshot, TextGrid};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// What a program hands over at `flip()`.
#[derive(Clone, Debug)]
pub struct FrameOut {
    pub cmds: Vec<DrawCmd>,
    pub grid: TextGrid,
    /// Requested virtual resolution change, applied before drawing.
    pub resolution: Option<(u32, u32)>,
    pub fps: u32,
    /// The program is drawing on the alternate screen (see `alt_screen()`).
    pub alt_screen: bool,
    /// Sound commands issued since the last flip.
    pub audio: Vec<AudioCmd>,
}

/// What comes back after the frame was presented.
#[derive(Clone, Debug, Default)]
pub struct FrameAck {
    pub input: InputSnapshot,
    pub dt: f64,
    pub frame: u64,
}

/// How `flip()` reaches the outside world. The program thread sends over
/// channels; the shell's immediate mode returns at once.
pub trait Flip {
    fn flip(&mut self, out: FrameOut) -> Result<FrameAck, FlipError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlipError {
    /// The user pressed Esc or the host went away.
    Break,
    /// Immediate mode can't present frames.
    NotSupported,
}

/// A tiny PCG32, so `random()` is reproducible (FR-R9).
#[derive(Clone, Debug)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64) -> Pcg32 {
        let mut p = Pcg32 { state: 0, inc: (seed << 1) | 1 };
        p.next_u32();
        p.state = p.state.wrapping_add(seed);
        p.next_u32();
        p
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6364136223846793005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in [0, 1).
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u32() >> 8) as f64 / (1u64 << 24) as f64
    }

    /// Uniform integer in lo..=hi.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u32() as u64 % span) as i64
    }
}

#[derive(Clone, Debug)]
pub struct MachineOptions {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    /// Chunk name shown in errors.
    pub main_name: String,
    pub seed: u64,
    /// Immediate mode: no frames, draws go straight to the shell screen.
    pub immediate: bool,
    /// Host window size, for `set_resolution("native")`.
    pub window: (u32, u32),
}

impl Default for MachineOptions {
    fn default() -> Self {
        MachineOptions {
            width: argon_core::DEFAULT_WIDTH,
            height: argon_core::DEFAULT_HEIGHT,
            fps: 60,
            main_name: "main.lua".to_string(),
            seed: 0x5eed,
            immediate: false,
            window: (argon_core::DEFAULT_WIDTH, argon_core::DEFAULT_HEIGHT),
        }
    }
}

pub struct Machine {
    pub cmds: Vec<DrawCmd>,
    pub grid: TextGrid,
    pub width: u32,
    pub height: u32,
    pub pending_resolution: Option<(u32, u32)>,
    pub fps: u32,
    pub color: Color,
    pub input: InputSnapshot,
    /// Typed keys not yet consumed by `read_key`/`input`.
    pub key_queue: VecDeque<KeyEvent>,
    pub rng: Pcg32,
    pub start: Instant,
    pub frame: u64,
    pub dt: f64,
    pub jail: Jail,
    pub save_jail: Option<Jail>,
    pub break_flag: Arc<AtomicBool>,
    pub flip: Box<dyn Flip>,
    pub next_image: u32,
    pub images: HashMap<ImageId, (u32, u32)>,
    pub immediate: bool,
    /// Lines printed by `print` in immediate mode are also kept here so the
    /// shell can show them.
    pub main_name: String,
    /// Set by `exit()`.
    pub exit_requested: bool,
    /// Elapsed-time budget for immediate mode evaluation.
    pub deadline: Option<Instant>,
    /// Alternate screen state: the saved text layer while on the alt screen.
    pub alt_screen: bool,
    pub saved_grid: Option<TextGrid>,
    pub audio: Vec<AudioCmd>,
    pub next_sound: u32,
    pub next_voice: u32,
    pub next_shader: u32,
    /// Persistent uniform values set with `set_uniform`, per shader.
    pub shader_uniforms: HashMap<u32, [[f32; 4]; argon_core::shader::MAX_USER_UNIFORMS]>,
    /// Last mouse click position, for iMouse.zw.
    pub click_pos: (f32, f32),
    pub window: (u32, u32),
}

impl Machine {
    pub fn new(opts: MachineOptions, jail: Jail, save_jail: Option<Jail>, break_flag: Arc<AtomicBool>, flip: Box<dyn Flip>) -> Machine {
        let mut grid = TextGrid::for_pixels(opts.width, opts.height);
        grid.cursor_visible = false;
        Machine {
            cmds: Vec::new(),
            grid,
            width: opts.width,
            height: opts.height,
            pending_resolution: None,
            fps: opts.fps,
            color: Color::WHITE,
            input: InputSnapshot::default(),
            key_queue: VecDeque::new(),
            rng: Pcg32::new(opts.seed),
            start: Instant::now(),
            frame: 0,
            dt: 1.0 / opts.fps.max(1) as f64,
            jail,
            save_jail,
            break_flag,
            flip,
            next_image: 1,
            images: HashMap::new(),
            immediate: opts.immediate,
            main_name: opts.main_name,
            exit_requested: false,
            deadline: None,
            alt_screen: false,
            saved_grid: None,
            audio: Vec::new(),
            next_sound: 1,
            next_voice: 1,
            next_shader: 1,
            shader_uniforms: HashMap::new(),
            click_pos: (0.0, 0.0),
            window: opts.window,
        }
    }

    /// Default size multiplier for `draw_text`, matching the text grid.
    pub fn text_scale(&self) -> u32 {
        argon_core::text_scale_for_height(self.height)
    }

    /// Shadertoy iMouse from the current input: xy = position (y up), zw =
    /// click position, z/w negative while the button is up.
    pub fn imouse(&self) -> [f32; 4] {
        let m = &self.input.mouse;
        let y_up = self.height as f32 - m.y;
        let down = m.down.contains(&argon_core::input::MouseButton::Left);
        let (cx, cy) = self.click_pos;
        let cy_up = self.height as f32 - cy;
        if down {
            [m.x, y_up, cx.abs(), cy_up.abs()]
        } else {
            [m.x, y_up, -cx.abs(), -cy_up.abs()]
        }
    }

    /// Switch to a fresh screen, remembering the current one.
    pub fn enter_alt_screen(&mut self) {
        if self.alt_screen {
            return;
        }
        self.alt_screen = true;
        self.cmds.push(DrawCmd::SaveScreen);
        self.cmds.push(DrawCmd::Clear(Color::BLACK));
        // Same size and cell scale as the current screen.
        let mut fresh = TextGrid::for_pixels(self.width, self.height);
        fresh.fg = self.grid.fg;
        fresh.bg = self.grid.bg;
        self.saved_grid = Some(std::mem::replace(&mut self.grid, fresh));
    }

    /// Return to the screen saved by `enter_alt_screen`.
    pub fn leave_alt_screen(&mut self) {
        if !self.alt_screen {
            return;
        }
        self.alt_screen = false;
        self.cmds.push(DrawCmd::RestoreScreen);
        if let Some(g) = self.saved_grid.take() {
            self.grid = g;
        }
    }

    pub fn check_break(&self) -> bool {
        self.break_flag.load(Ordering::Relaxed)
    }

    /// Perform a flip: hand the frame out, wait for the ack, install input.
    pub fn do_flip(&mut self) -> Result<(), FlipError> {
        if self.check_break() {
            return Err(FlipError::Break);
        }
        let out = FrameOut {
            cmds: std::mem::take(&mut self.cmds),
            grid: self.grid.clone(),
            resolution: self.pending_resolution.take(),
            fps: self.fps,
            alt_screen: self.alt_screen,
            audio: std::mem::take(&mut self.audio),
        };
        let ack = self.flip.flip(out)?;
        self.key_queue.extend(ack.input.typed.iter().cloned());
        // Keep the queue bounded if the program never reads keys.
        while self.key_queue.len() > 256 {
            self.key_queue.pop_front();
        }
        if ack.input.mouse.pressed.contains(&argon_core::input::MouseButton::Left) {
            self.click_pos = (ack.input.mouse.x, ack.input.mouse.y);
        }
        self.input = ack.input;
        self.dt = ack.dt;
        self.frame = ack.frame;
        Ok(())
    }

    pub fn alloc_image(&mut self, w: u32, h: u32) -> ImageId {
        let id = ImageId(self.next_image);
        self.next_image += 1;
        self.images.insert(id, (w, h));
        id
    }

    pub fn set_resolution(&mut self, w: u32, h: u32) {
        self.width = w;
        self.height = h;
        self.pending_resolution = Some((w, h));
        self.grid = TextGrid::for_pixels(w, h);
        self.cmds.push(DrawCmd::Clear(Color::BLACK));
    }

    pub fn time(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }
}

/// Flip implementation that cannot present (shell immediate mode).
pub struct NoFlip;

impl Flip for NoFlip {
    fn flip(&mut self, _out: FrameOut) -> Result<FrameAck, FlipError> {
        Err(FlipError::NotSupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcg_is_deterministic_and_in_range() {
        let mut a = Pcg32::new(42);
        let mut b = Pcg32::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        for _ in 0..1000 {
            let v = a.range(3, 7);
            assert!((3..=7).contains(&v));
            let f = a.next_f64();
            assert!((0.0..1.0).contains(&f));
        }
    }
}
