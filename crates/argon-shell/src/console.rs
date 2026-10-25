//! The console state machine: Shell <-> Editor <-> Running.

use crate::lineedit::{LineAction, LineEdit};
use crate::shell::{self, ShellCmd};
use argon_core::draw::DrawCmd;
use argon_core::input::{InputState, KeyEvent};
use argon_core::theme::DEFAULT_THEME;
use argon_core::{Color, TextGrid, Theme, DEFAULT_HEIGHT, DEFAULT_WIDTH};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Shell,
    // Editor,
    // Running,
}

/// What the renderer needs this frame.
pub struct FrameOutput {
    pub cmds: Vec<DrawCmd>,
    pub grid: TextGrid,
    pub cursor_on: bool,
    pub resolution: (u32, u32),
    pub clear_color: Color,
}

pub struct Console {
    theme: &'static Theme,
    grid: TextGrid,
    cmds: Vec<DrawCmd>,
    mode: Mode,
    le: LineEdit,
    resolution: (u32, u32),
    time: f64,
    quit: bool,
    input: InputState,
    /// Row where the prompt is drawn (the logical print cursor lives at col 0
    /// of this row while the blinking cursor sits inside the input).
    prompt_row: usize,
}

impl Console {
    pub fn new() -> Result<Console, String> {
        let theme = &DEFAULT_THEME;
        let mut grid = TextGrid::for_pixels(DEFAULT_WIDTH, DEFAULT_HEIGHT);
        grid.fg = theme.fg;
        grid.bg = None;
        grid.cursor_visible = true;
        let mut c = Console {
            theme,
            grid,
            cmds: vec![DrawCmd::Clear(theme.bg)],
            mode: Mode::Shell,
            le: LineEdit::new(),
            resolution: (DEFAULT_WIDTH, DEFAULT_HEIGHT),
            time: 0.0,
            quit: false,
            input: InputState::default(),
            prompt_row: 0,
        };
        c.banner();
        c.prompt_row = c.grid.cursor_row;
        Ok(c)
    }

    pub fn wants_quit(&self) -> bool {
        self.quit
    }

    fn banner(&mut self) {
        let accent = self.theme.accent;
        let dim = self.theme.dim;
        self.print_colored(&format!("ARGON {}  -  a fantasy console", argon_core::VERSION), accent);
        self.print_colored(
            &format!("{}x{} text, {}x{} pixels, Luau.",
                self.grid.cols(), self.grid.rows(), DEFAULT_WIDTH, DEFAULT_HEIGHT),
            dim,
        );
        self.print_colored("Type HELP and press enter for information", dim);
        self.grid.newline();
    }

    fn print(&mut self, s: &str) {
        self.grid.println(s);
    }

    fn print_colored(&mut self, s: &str, fg: Color) {
        let saved = self.grid.fg;
        self.grid.fg = fg;
        self.grid.println(s);
        self.grid.fg = saved;
    }

    fn print_error(&mut self, s: &str) {
        let c = self.theme.error;
        for line in s.lines() {
            self.print_colored(line, c);
        }
    }

    fn prompt_text(&self) -> String {
        "ARGON> ".to_string()
    }

    // ----- input ----------------------------------------------------------------------

    pub fn key(&mut self, ev: KeyEvent) {
        self.input.key_event(&ev);
        match self.mode {
            Mode::Shell => {
                if !ev.pressed {
                    return;
                }
                self.shell_key(&ev);
            }
        }
    }

    fn shell_key(&mut self, ev: &KeyEvent) {
        // The blinking cursor sits inside the input; put the logical print
        // cursor back at the start of the prompt row.
        self.grid.locate(0, self.prompt_row);
        self.shell_key_inner(ev);
        if self.grid.cursor_col != 0 {
            self.grid.newline();
        }
        self.prompt_row = self.grid.cursor_row;
    }

    fn shell_key_inner(&mut self, ev: &KeyEvent) {
        match self.le.handle(ev) {
            LineAction::Submit(line) => {
                // Echo the full line where the prompt was and move on.
                let prompt = self.prompt_text();
                self.grid.write(&format!("{prompt}{line}"));
                self.grid.newline();
                // self.le.push_history(&line);
                self.execute(&line);
            }
            LineAction::Cancel => {
                self.le.clear();
            }
            _ => {}
        }
    }

    // ----- shell commands -----------------------------------------------------------------

    fn execute(&mut self, line: &str) {
        match shell::parse(line) {
            ShellCmd::NoOp => {},
            ShellCmd::Unknown(s) => self.print_error(&format!("Unknown command: {s}")),
            ShellCmd::Cls => self.cls(),
            ShellCmd::Echo(s) => self.print(s.as_deref().unwrap_or("")),
            ShellCmd::Help => {
                self.print("Available commands:");
                self.print("CLS - Clear the screen");
                self.print("ECHO [message] - Print message to the screen");
                self.print("HELP - Show this help text");
                self.print("EXIT - Exit Argon");
            }
            ShellCmd::Exit => self.quit = true,
        }
    }

    fn cls(&mut self) {
        self.grid.clear();
        self.cmds.push(DrawCmd::Clear(self.theme.bg));
    }

    // ----- frame -----------------------------------------------------------------------------

    /// Advance one display frame. `dt` is the display frame time.
    pub fn update(&mut self, dt: f64) -> FrameOutput {
        self.time += dt;
        let cursor_on = ((self.time * 2.5) as u64).is_multiple_of(2);
        let cmds = std::mem::take(&mut self.cmds);

        /*
        if self.mode == Mode::Running {
            self.poll_program(&mut cmds);
        }
        */

        match self.mode {
            Mode::Shell => {
                self.draw_prompt();
                FrameOutput {
                    cmds, grid: self.grid.clone(),
                    cursor_on,
                    resolution: self.resolution,
                    clear_color: self.theme.bg,
                }
            }
        }
    }

    /// Draw the prompt and line buffer starting at the logical cursor row,
    /// and place the blinking cursor inside the input.
    fn draw_prompt(&mut self) {
        // The logical print cursor lives at the start of the prompt row;
        // the previous draw moved the blinking cursor into the input.
        self.grid.locate(0, self.prompt_row);
        let prompt = self.prompt_text();
        let cols = self.grid.cols();
        let total = prompt.chars().count() + self.le.text.chars().count();
        let rows_needed = total / cols + 1;
        while self.grid.cursor_row + rows_needed > self.grid.rows() && self.grid.cursor_row > 0 {
            self.grid.scroll_up(1);
            self.grid.cursor_row -= 1;
        }
        let row = self.grid.cursor_row;
        self.prompt_row = row;
        for r in row..(row + rows_needed).min(self.grid.rows()) {
            self.grid.fill_rect(0, r, cols, 1, argon_core::Cell { ch: ' ', fg: self.grid.fg, bg: None });
        }
        let accent = self.theme.accent;
        self.grid.put_str_colored(0, row, &prompt, accent, None);
        let text = self.le.text.clone();
        let mut c = prompt.chars().count();
        let mut r = row;
        for ch in text.chars() {
            if c >= cols {
                c = 0;
                r += 1;
            }
            if r < self.grid.rows() {
                self.grid.put_char(c, r, ch);
            }
            c += 1;
        }
        let cur = prompt.chars().count() + self.le.cursor;
        self.grid.cursor_visible = true;
        self.grid.cursor_row = (row + cur / cols).min(self.grid.rows() - 1);
        self.grid.cursor_col = cur % cols;
    }

}
