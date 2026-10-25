//! The console state machine: Shell <-> Editor <-> Running.

use crate::lineedit::{LineAction, LineEdit};
use crate::shell::{self, ShellCmd};
use argon_core::config::{Config, Manifest, Paths};
use argon_core::draw::DrawCmd;
use argon_core::input::{InputState, KeyCode, KeyEvent};
use argon_core::vfs::{self, Jail, ProgramLocation};
use argon_core::theme::DEFAULT_THEME;
use argon_core::{Color, TextGrid, Theme, DEFAULT_HEIGHT, DEFAULT_WIDTH};
use argon_lua::machine::{MachineOptions, NoFlip};
use argon_lua::program::{spawn_program, ProgramExit, ProgramHandle, ProgramSpec};
use argon_lua::runtime::Runtime;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

pub struct ConsoleOptions {
    pub paths: Paths,
    pub config: Config,
    /// Demo programs copied into DEMOS/ at startup: (relative path, source).
    pub demos: Vec<(&'static str, &'static str)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Shell,
    // Editor,
    Running,
}

struct Running {
    handle: ProgramHandle,
    // from_editor: bool,
    location: ProgramLocation,
    /// Text layer of the last presented frame.
    grid: TextGrid,
    resolution_changed: bool,
    fps: u32,
}

/// What the renderer needs this frame.
pub struct FrameOutput {
    pub cmds: Vec<DrawCmd>,
    pub grid: TextGrid,
    pub cursor_on: bool,
    pub resolution: (u32, u32),
    pub clear_color: Color,
    /// True when a program is running (affects frame pacing in the host).
    pub running: bool,
}

pub struct Console {
    pub paths: Paths,
    pub config: Config,

    // Display and I/O
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

    cwd: String,
    /// Lua runtime
    imm: Runtime,
    /// Set when the renderer must reset per-program GPU state.
    pub program_started: bool,
    program: Option<Running>,

    disk: Jail,
}

impl Console {
    pub fn new(opts: ConsoleOptions) -> Result<Console, String> {
        let ConsoleOptions { paths, config, demos } = opts;

        // Copy demos to disk disk
        paths.ensure_dirs().map_err(|e| format!("cannot create {}: {e}", paths.disk.display()))?;
        install_demos(&paths, &demos);

        // Initialize vfs jail and lua runtime
        let disk = Jail::new(paths.disk.clone()).with_read_only(&["DEMOS"]);
        let imm = new_immediate(&disk)?;

        let theme = &DEFAULT_THEME;
        let mut grid = TextGrid::for_pixels(DEFAULT_WIDTH, DEFAULT_HEIGHT);
        grid.fg = theme.fg;
        grid.bg = None;
        grid.cursor_visible = true;
        let mut c = Console {
            paths,
            config,
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

            cwd: String::new(),
            imm,
            program_started: false,
            program: None,
            disk,
        };

        c.banner();
        c.prompt_row = c.grid.cursor_row;
        Ok(c)
    }

    pub fn wants_quit(&self) -> bool {
        self.quit
    }

    pub fn is_running(&self) -> bool {
        self.mode == Mode::Running
    }

    /// Current shell screen text (for tests and the headless runner).
    pub fn screen_text(&self) -> String {
        match (&self.mode, &self.program) {
            (Mode::Running, Some(r)) => r.grid.to_text(),
            _ => self.grid.to_text(),
        }
    }

    fn banner(&mut self) {
        let accent = self.theme.accent;
        let dim = self.theme.dim;

        self.print_colored(&format!("ARGON {}  -  a fantasy console", argon_core::VERSION), accent);

        let used = dir_size(&self.paths.disk);
        self.print_colored(
            &format!("{}x{} text, {}x{} pixels, Luau.  Disk: {} used.",
                self.grid.cols(), self.grid.rows(), DEFAULT_WIDTH, DEFAULT_HEIGHT, shell::human_size(used)),
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
            Mode::Running => {
                if ev.pressed && ev.code == KeyCode::Escape && !ev.repeat {
                    if let Some(r) = &self.program {
                        r.handle.request_break();
                    }
                }
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

            ShellCmd::Dir(pat) => self.cmd_dir(pat.as_deref()),
            ShellCmd::Run(name) => self.cmd_run(name),
            ShellCmd::Lua(src) => if let Some(src) = src {
                if !src.trim().is_empty() {
                    self.eval_lua(&src);
                }
            }
        }
    }

    fn cls(&mut self) {
        self.grid.clear();
        self.cmds.push(DrawCmd::Clear(self.theme.bg));
    }

    fn eval_lua(&mut self, src: &str) {
        // Hand the shell grid to the immediate-mode machine so print/locate
        // act on the screen directly.
        let placeholder = TextGrid::new(1, 1);
        let shell_grid = std::mem::replace(&mut self.grid, placeholder);
        {
            let mut m = self.imm.machine.borrow_mut();
            m.grid = shell_grid;
            m.width = self.resolution.0;
            m.height = self.resolution.1;
        }
        let result = self.imm.eval_immediate(src);
        {
            let mut m = self.imm.machine.borrow_mut();
            self.grid = std::mem::replace(&mut m.grid, TextGrid::new(1, 1));
            self.cmds.append(&mut m.cmds);
            m.pending_resolution = None;
        }
        self.grid.cursor_visible = true;
        match result {
            Ok(Some(s)) => self.print(&s),
            Ok(None) => {}
            Err(e) => {
                let text = e.render();
                self.print_error(&text);
            }
        }
    }

    fn cmd_dir(&mut self, pattern: Option<&str>) {
        let dir = match pattern {
            None => self.cwd.clone(),
            Some(p) => {
                vfs::join(&self.cwd, p)
            }
        };
        match self.disk.list(&dir) {
            Ok(entries) => {
                let header = format!(" Folder of \\{}", dir.replace('/', "\\").to_ascii_uppercase());
                self.print(&header);
                let cols = self.grid.cols();
                for l in shell::format_dir(&entries, cols) {
                    self.print(&l);
                }
            }
            Err(e) => self.print_error(&e.to_string()),
        }
    }

    fn resolve(&self, name: &str) -> Option<ProgramLocation> {
        vfs::resolve_program(&self.disk, &self.cwd, name)
    }

    fn cmd_run(&mut self, name: Option<String>) {
        let name = match name {
            Some(n) => n,
            None => {
                self.print_error("RUN what? Usage: RUN name");
                return;
            }
        };
        let Some(loc) = self.resolve(&name) else {
            self.print_error(&format!("Program not found: {name}"));
            return;
        };
        self.start_program(loc);
    }

    fn start_program(&mut self, loc: ProgramLocation) {
        let source = match self.disk.read_string(&loc.main) {
            Ok(s) => s,
            Err(e) => {
                self.print_error(&e.to_string());
                return;
            }
        };
        let jail = match self.disk.sub(&loc.dir) {
            Ok(j) => j,
            Err(e) => {
                self.print_error(&e.to_string());
                return;
            }
        };
        let save_jail = self.disk.sub("SAVE").ok();

        let manifest = match &loc.manifest {
            Some(m) => self.disk.read_string(m).ok().and_then(|s| Manifest::parse(&s).ok()).unwrap_or_default(),
            None => Manifest::from_header(&source),
        };

        let (w, h) = manifest.resolution();
        let main_name = if loc.is_cart { format!("{}/main.lua", loc.name) } else { format!("{}.lua", loc.name) };
        let spec = ProgramSpec {
            main_name: main_name.clone(),
            source,
            jail,
            save_jail,
            options: MachineOptions { width: w, height: h, fps: manifest.fps.clamp(1, 240), main_name, seed: 0x5eed, immediate: false },
        };
        let handle = spawn_program(spec);
        let resolution_changed = (w, h) != self.resolution;
        self.resolution = (w, h);
        let mut grid = TextGrid::for_pixels(w, h);
        grid.cursor_visible = false;
        self.input.clear_all();
        self.program = Some(Running { handle, location: loc, grid, resolution_changed, fps: manifest.fps.max(1) });
        self.mode = Mode::Running;
        self.program_started = true;
        self.cmds.push(DrawCmd::Clear(Color::BLACK));
    }

    // ----- frame -----------------------------------------------------------------------------

    /// Advance one display frame. `dt` is the display frame time.
    pub fn update(&mut self, dt: f64) -> FrameOutput {
        self.time += dt;
        let cursor_on = ((self.time * 2.5) as u64).is_multiple_of(2);
        let mut cmds = std::mem::take(&mut self.cmds);

        if self.mode == Mode::Running {
            self.poll_program(&mut cmds);
        }

        match self.mode {
            Mode::Shell => {
                self.draw_prompt();
                FrameOutput {
                    cmds, grid: self.grid.clone(),
                    cursor_on,
                    resolution: self.resolution,
                    clear_color: self.theme.bg,
                    running: false,
                }
            }

            Mode::Running => {
                let r = self.program.as_ref().expect("running without program");
                FrameOutput {
                    cmds,
                    grid: r.grid.clone(),
                    cursor_on,
                    resolution: self.resolution,
                    clear_color: Color::BLACK,
                    running: true,
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

    fn poll_program(&mut self, cmds: &mut Vec<DrawCmd>) {
        let keymap = self.config.keymap.clone();
        let mut exit: Option<ProgramExit> = None;
        if let Some(r) = &mut self.program {
            if r.handle.poll_frame().is_some() {
                let fps = r.handle.pending_fps().unwrap_or(r.fps).max(1);
                let interval = Duration::from_secs_f64(1.0 / fps as f64);
                let elapsed = r.handle.since_last_ack();
                if elapsed + Duration::from_millis(2) >= interval {
                    let frame = r.handle.poll_frame().unwrap().clone();
                    if let Some((w, h)) = frame.resolution {
                        if (w, h) != self.resolution {
                            self.resolution = (w, h);
                            r.resolution_changed = true;
                        }
                    }
                    r.fps = frame.fps;
                    cmds.extend(frame.cmds);
                    r.grid = frame.grid;
                    let snap = self.input.snapshot(&keymap);
                    r.handle.ack(snap);
                }
            }
            if let Some(e) = r.handle.poll_exit() {
                exit = Some(e.clone());
            }
        }
        if let Some(e) = exit {
            self.finish_program(e, cmds);
        }
    }

    fn finish_program(&mut self, exit: ProgramExit, cmds: &mut Vec<DrawCmd>) {
        let Some(mut r) = self.program.take() else { return };
        // let from_editor = r.from_editor;
        let (final_exit, final_frame) = r.handle.finish(Duration::from_millis(500));
        let exit = if matches!(exit, ProgramExit::Finished) { final_exit } else { exit };
        self.input.clear_all();
        if let Some(f) = final_frame {
            if let Some((w, h)) = f.resolution {
                if (w, h) != self.resolution {
                    self.resolution = (w, h);
                    r.resolution_changed = true;
                }
            }
            cmds.extend(f.cmds);
            r.grid = f.grid;
        }

        /*
        if from_editor {
            self.mode = Mode::Editor;
            self.resolution = (DEFAULT_WIDTH, DEFAULT_HEIGHT);
            self.grid = TextGrid::for_pixels(DEFAULT_WIDTH, DEFAULT_HEIGHT);
            cmds.push(DrawCmd::Clear(self.theme.bg));
            if let Some(ed) = &mut self.editor {
                match &exit {
                    ProgramExit::Finished => ed.set_status("Program finished", false),
                    ProgramExit::Break => ed.set_status("Stopped", false),
                    ProgramExit::Error(e) => {
                        if let Some(line) = e.line {
                            if e.file.as_deref().map(|f| f.ends_with(vfs::file_name(&ed.path)) || f == ed.name).unwrap_or(true) {
                                ed.goto_line(line);
                            }
                        }
                        ed.set_status(&e.one_line(), true);
                    }
                }
            }
            return;
        }
        */

        self.mode = Mode::Shell;
        if r.resolution_changed || self.resolution != (DEFAULT_WIDTH, DEFAULT_HEIGHT) {
            self.resolution = (DEFAULT_WIDTH, DEFAULT_HEIGHT);
            self.grid = TextGrid::for_pixels(DEFAULT_WIDTH, DEFAULT_HEIGHT);
            cmds.push(DrawCmd::Clear(self.theme.bg));
        } else {
            // Keep the program's screen; continue below its text cursor.
            self.grid = r.grid;
            if self.grid.cursor_col != 0 {
                self.grid.newline();
            }
        }
        self.grid.fg = self.theme.fg;
        self.grid.bg = None;
        self.grid.cursor_visible = true;
        match exit {
            ProgramExit::Finished => {}
            ProgramExit::Break => self.print_colored("Break.", self.theme.dim),
            ProgramExit::Error(e) => {
                let text = e.render();
                self.print_error(&text);
                let name = r.location.name.clone();
                self.print_colored(&format!("Type EDIT {name} to fix it."), self.theme.dim);
            }
        }
        if self.grid.cursor_col != 0 {
            self.grid.newline();
        }
        self.prompt_row = self.grid.cursor_row;
    }
}

fn new_immediate(disk: &Jail) -> Result<Runtime, String> {
    let save = disk.sub("SAVE").ok();
    let opts = MachineOptions { immediate: true, main_name: "shell".to_string(), ..Default::default() };
    Runtime::new(opts, disk.clone(), save, Arc::new(AtomicBool::new(false)), Box::new(NoFlip)).map_err(|e| e.to_string())
}

pub fn install_demos(paths: &Paths, demos: &[(&str, &str)]) {
    let dir = paths.disk.join("DEMOS");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    for (name, src) in demos {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(p, src);
    }
}

fn dir_size(p: &std::path::Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(p) {
        for e in rd.flatten() {
            if let Ok(m) = e.metadata() {
                if m.is_dir() {
                    total += dir_size(&e.path());
                } else {
                    total += m.len();
                }
            }
        }
    }
    total
}

fn copy_dir_host(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let dest = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir_host(&e.path(), &dest)?;
        } else {
            std::fs::copy(e.path(), dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon_core::input::KeyEvent;

    fn console() -> (Console, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::at(dir.path().to_path_buf(), None);
        let c = Console::new(ConsoleOptions { paths, config: Config::default(), demos: vec![("hello.lua", "print('hi')\n")] }).unwrap();
        (c, dir)
    }

    fn type_line(c: &mut Console, s: &str) {
        for ch in s.chars() {
            c.key(KeyEvent::typed(ch));
        }
        c.key(KeyEvent::press(KeyCode::Enter));
    }

    fn pump(c: &mut Console, frames: usize) {
        for _ in 0..frames {
            c.update(1.0 / 60.0);
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn banner_help_and_lua_at_prompt() {
        let (mut c, _d) = console();
        let text = c.screen_text();
        assert!(text.contains("ARGON"));
        type_line(&mut c, "EVAL 2^10");
        assert!(c.screen_text().contains("1024"));
        type_line(&mut c, "help");
        assert!(c.screen_text().contains("Available commands"));
        type_line(&mut c, "LUA fill_rect(10, 10, 5, 5)");
        let out = c.update(0.016);
        assert!(out.cmds.iter().any(|cmd| matches!(cmd, DrawCmd::Rect { .. })));
    }

    /*
    #[test]
    fn dir_shows_demos_and_type_prints_file() {
        let (mut c, _d) = console();
        type_line(&mut c, "dir");
        assert!(c.screen_text().contains("DEMOS"));
        type_line(&mut c, "cd demos");
        type_line(&mut c, "type hello");
        assert!(c.screen_text().contains("print('hi')"));
        assert!(c.screen_text().contains("ARGON:DEMOS>") || c.prompt_text().contains("DEMOS"));
    }
    */

    #[test]
    fn run_program_to_completion_and_show_error() {
        let (mut c, _d) = console();
        type_line(&mut c, "cls");
        type_line(&mut c, "run demos/hello");
        assert!(c.is_running());
        pump(&mut c, 50);
        assert!(!c.is_running(), "program should have finished");
        assert!(c.screen_text().contains("hi"), "{}", c.screen_text());

        c.disk.write("bad.lua", b"print('x')\nfil_rect(1)\n").unwrap();
        type_line(&mut c, "run bad");
        pump(&mut c, 50);
        let t = c.screen_text();
        assert!(t.contains("ERROR in bad.lua:2"), "{t}");
        // assert!(t.contains("Did you mean fill_rect?"), "{t}");
    }

    #[test]
    fn esc_breaks_running_program_and_frames_flow() {
        let (mut c, _d) = console();
        c.disk.write("loop.lua", b"while true do fill_rect(0,0,1,1) flip() end").unwrap();
        type_line(&mut c, "run loop");
        let mut saw_cmds = false;
        for _ in 0..30 {
            let out = c.update(1.0 / 60.0);
            if out.cmds.iter().any(|x| matches!(x, DrawCmd::Rect { .. })) {
                saw_cmds = true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(saw_cmds);
        c.key(KeyEvent::press(KeyCode::Escape));
        pump(&mut c, 30);
        assert!(!c.is_running());
        assert!(c.screen_text().contains("Break"));
    }

    /*
    #[test]
    fn del_requires_confirmation() {
        let (mut c, _d) = console();
        c.disk.write("junk.lua", b"").unwrap();
        type_line(&mut c, "del junk");
        assert!(c.disk.exists("junk.lua"));
        c.key(KeyEvent::typed('n'));
        assert!(c.disk.exists("junk.lua"));
        type_line(&mut c, "del junk");
        c.key(KeyEvent::typed('y'));
        assert!(!c.disk.exists("junk.lua"));
    }

    #[test]
    fn clear_resets_variables() {
        let (mut c, _d) = console();
        type_line(&mut c, "x = 41");
        type_line(&mut c, "x + 1");
        assert!(c.screen_text().contains("42"));
        type_line(&mut c, "varclear");
        type_line(&mut c, "x");
        assert!(c.screen_text().lines().last().unwrap_or("").contains("nil") || c.screen_text().contains("nil"));
    }

    #[test]
    fn tab_completes_commands_and_files() {
        let (mut c, _d) = console();
        for ch in "he".chars() {
            c.key(KeyEvent::typed(ch));
        }
        c.key(KeyEvent::press(KeyCode::Tab));
        assert_eq!(c.le.text, "HELP ");
        c.le.clear();
        for ch in "run dem".chars() {
            c.key(KeyEvent::typed(ch));
        }
        c.key(KeyEvent::press(KeyCode::Tab));
        assert_eq!(c.le.text, "run DEMOS/");
    }
    */
}
