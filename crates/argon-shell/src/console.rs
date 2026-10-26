//! The console state machine: Shell <-> Editor <-> Running.

use crate::editor::{Editor, EditorAction};
use crate::help;
use crate::lineedit::{LineAction, LineEdit};
use crate::shell::{self, ShellCmd};
use argon_core::api::ApiTable;
use argon_core::config::{Config, Manifest, Paths};
use argon_core::draw::DrawCmd;
use argon_core::input::{InputState, KeyCode, KeyEvent};
use argon_core::vfs::{self, Jail, ProgramLocation};
use argon_core::{Color, TextGrid, Theme, DEFAULT_HEIGHT, DEFAULT_WIDTH};
use argon_audio::{AudioCmd, AudioEngine};
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
    /// Open the audio device (false for headless/CI).
    pub audio: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Shell,
    Editor,
    Running,
}

struct Running {
    handle: ProgramHandle,
    from_editor: bool,
    location: ProgramLocation,
    /// Text layer of the last presented frame.
    grid: TextGrid,
    resolution_changed: bool,
    fps: u32,
    /// The shell's text layer before RUN, restored if the program used
    /// `alt_screen()`.
    shell_grid: TextGrid,
    alt_screen: bool,
    /// Cart manifest filter override.
    filter: String,
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
    editor: Option<Editor>,
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
    /// Fixed frame time handed to programs (headless/CI); None = measured.
    pub fixed_dt: Option<f64>,

    disk: Jail,
    api: ApiTable,
    audio: AudioEngine,
}

impl Console {
    pub fn new(opts: ConsoleOptions) -> Result<Console, String> {
        let ConsoleOptions { paths, config, demos, audio } = opts;

        // Copy demos to disk disk
        paths.ensure_dirs().map_err(|e| format!("cannot create {}: {e}", paths.disk.display()))?;
        install_demos(&paths, &demos);

        // Initialize vfs jail and lua runtime
        let disk = Jail::new(paths.disk.clone()).with_read_only(&["DEMOS"]);
        let imm = new_immediate(&disk)?;

        let theme = Theme::by_name(&config.theme).unwrap_or_else(Theme::default_theme);
        let history = std::fs::read_to_string(&paths.history)
            .map(|s| s.lines().map(|l| l.to_string()).collect::<Vec<_>>())
            .unwrap_or_default();
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
            le: LineEdit::with_history(history),
            editor: None,
            resolution: (DEFAULT_WIDTH, DEFAULT_HEIGHT),
            time: 0.0,
            quit: false,
            input: InputState::default(),
            prompt_row: 0,

            cwd: String::new(),
            imm,
            program_started: false,
            program: None,
            fixed_dt: None,
            disk,
            api: argon_lua::docs::api_table(),
            audio: if audio { AudioEngine::new() } else { AudioEngine::disabled() },
        };

        c.banner();
        c.prompt_row = c.grid.cursor_row;
        Ok(c)
    }

    pub fn wants_quit(&self) -> bool {
        self.quit
    }

    pub fn theme(&self) -> &'static Theme {
        self.theme
    }

    /// PRNG seed for a new program: fixed under a fixed frame step (headless
    /// runs stay reproducible), otherwise from the clock. `seed(n)` overrides.
    fn program_seed(&self) -> u64 {
        if self.fixed_dt.is_some() {
            return 0x5eed;
        }
        clock_seed()
    }

    pub fn is_running(&self) -> bool {
        self.mode == Mode::Running
    }

    /// Execute a shell line as if typed (used by `argon run`).
    pub fn execute_line(&mut self, line: &str) {
        if self.mode != Mode::Shell {
            return;
        }
        let prompt = self.prompt_text();
        self.grid.println(&format!("{prompt}{line}"));
        self.execute(line);
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
        let fg = self.theme.fg;
        let dim = self.theme.dim;

        let cols = self.grid.cols();
        let used = dir_size(&self.paths.disk);

        let rule: String = "\u{2500}".repeat(cols.saturating_sub(1));

        // A framed header line in the style of a system readout.
        self.print_colored(&rule, dim);
        let left = format!("  ARGON {}", argon_core::VERSION);
        let right = format!("{}x{}  {}x{}  LUAU  DISK {}  ",
            self.grid.cols(), self.grid.rows(), DEFAULT_WIDTH, DEFAULT_HEIGHT, shell::human_size(used).to_ascii_uppercase());
        let pad = cols.saturating_sub(left.chars().count() + right.chars().count());
        let row = self.grid.cursor_row;
        self.grid.put_str_colored(0, row, &left, accent, None);
        self.grid.put_str_colored(left.chars().count() + pad, row, &right, fg, None);
        self.grid.newline();
        self.print_colored(&rule, dim);
        self.print_colored("  A fantasy console.", fg);
        self.print_colored("  HELP for commands.  Run Lua using EVAL or LUA: LUA print(2^10)", dim);
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
        match self.cwd.is_empty() {
            true => "ARGON \u{25B8} ".to_string(),
            false => format!("ARGON:{} \u{25B8} ", self.cwd.replace('/', "\\").to_ascii_uppercase())
        }
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
            Mode::Editor => {
                if !ev.pressed {
                    return;
                }
                let rows = self.grid.rows();
                let action = match self.editor.as_mut() {
                    Some(ed) => ed.handle(&ev, &self.api, rows),
                    None => EditorAction::None,
                };
                self.editor_action(action);
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
                self.le.push_history(&line);
                self.save_history();
                self.execute(&line);
            }
            LineAction::Tab => self.complete(),
            LineAction::Cancel => {
                self.le.clear();
            }
            _ => {}
        }
    }

    // shell history
    fn save_history(&self) {
        let hist = self.le.history();
        let start = hist.len().saturating_sub(200);
        let _ = std::fs::write(&self.paths.history, hist[start..].join("\n"));
    }

    // tab completion
    fn complete(&mut self) {
        let (start, word) = self.le.word_before_cursor();
        if word.is_empty() {
            return;
        }
        let first_word = self.le.text.split_whitespace().next().map(|s| s.to_string());
        let is_first = start == self.le.text.chars().take_while(|c| c.is_whitespace()).count();
        let mut cands: Vec<String> = Vec::new();
        if is_first {
            for c in help::COMMANDS {
                if c.name.starts_with(&word.to_ascii_uppercase()) {
                    cands.push(c.name.to_string());
                }
            }
            for g in self.imm.global_names() {
                if g.starts_with(&word) {
                    cands.push(g);
                }
            }
        }
        let is_file_cmd = matches!(
            first_word.as_deref().map(|s| s.to_ascii_uppercase()).as_deref(),
            Some("RUN" | "EDIT" | "TYPE" | "COPY" | "REN" | "DEL" | "CD" | "DIR" | "EXPORT")
        );
        if !is_first && is_file_cmd {
            let (dir, base) = match word.rfind('/') {
                Some(i) => (vfs::join(&self.cwd, &word[..i]), word[i + 1..].to_string()),
                None => (self.cwd.clone(), word.clone()),
            };
            if let Ok(entries) = self.disk.list(&dir) {
                for e in entries {
                    if e.name.to_lowercase().starts_with(&base.to_lowercase()) {
                        let prefix = match word.rfind('/') {
                            Some(i) => &word[..=i],
                            None => "",
                        };
                        let name = if e.is_dir { format!("{prefix}{}/", e.name) } else { format!("{prefix}{}", vfs::strip_lua_ext(&e.name)) };
                        cands.push(name);
                    }
                }
            }
        }
        cands.sort();
        cands.dedup();
        match cands.len() {
            0 => {}
            1 => {
                let mut c = cands.remove(0);
                if !c.ends_with('/') && is_first {
                    c.push(' ');
                }
                self.le.replace_word(start, &c);
            }
            _ => {
                // Complete the common prefix and list candidates.
                let common = common_prefix(&cands);
                if common.chars().count() > word.chars().count() {
                    self.le.replace_word(start, &common);
                } else {
                    let refs: Vec<&str> = cands.iter().map(|s| s.as_str()).collect();
                    let cols = self.grid.cols();
                    self.grid.newline();
                    for line in help::columns(&refs, cols) {
                        self.print(&line);
                    }
                }
            }
        }
    }

    // ----- shell commands -----------------------------------------------------------------

    fn execute(&mut self, line: &str) {
        match shell::parse(line) {
            ShellCmd::NoOp => {},
            ShellCmd::Unknown(s) => self.print_error(&format!("Unknown command: {s}")),
            ShellCmd::Cls => self.cls(),
            ShellCmd::Echo(s) => self.print(s.as_deref().unwrap_or("")),
            ShellCmd::Help(topic) => {
                let cols = self.grid.cols();
                for l in help::help_lines(topic.as_deref(), &self.api, cols) {
                    self.print(&l);
                }
            }
            ShellCmd::Ver => {
                let v = format!("Argon {} (Luau, api 1)", argon_core::VERSION);
                self.print(&v);
            }
            ShellCmd::Exit => self.quit = true,

            ShellCmd::Theme(name) => self.cmd_theme(name),
            ShellCmd::Filter(name) => self.cmd_filter(name),

            ShellCmd::Cd(dir) => self.cmd_cd(dir),
            ShellCmd::Dir(pat) => self.cmd_dir(pat.as_deref()),
            ShellCmd::Type(name) => self.cmd_type(name),
            ShellCmd::Run(name) => self.cmd_run(name),
            ShellCmd::Lua(src) => if let Some(src) = src {
                if !src.trim().is_empty() {
                    self.eval_lua(&src);
                }
            }

            ShellCmd::New(name) => self.cmd_new(name),
            ShellCmd::Edit(name) => self.cmd_edit(name),

            ShellCmd::Mkdir(dir) => self.cmd_mkdir(dir),
            ShellCmd::Copy(a, b) => self.cmd_copy(a, b),
            ShellCmd::Ren(a, b) => self.cmd_ren(a, b),
            ShellCmd::Del(name) => self.cmd_del(name),

            ShellCmd::Export(name) => self.cmd_export(name),
            ShellCmd::Import(path) => self.cmd_import(path),
        }
    }

    fn resolve(&self, name: &str) -> Option<ProgramLocation> {
        vfs::resolve_program(&self.disk, &self.cwd, name)
    }

    fn file_arg(&self, name: &str) -> String {
        match self.resolve(name) {
            Some(loc) if !loc.is_cart => loc.main,
            Some(loc) => loc.dir,
            None => self.disk.canonical_vpath(&vfs::join(&self.cwd, name)).unwrap_or_else(|| vfs::join(&self.cwd, name)),
        }
    }

    fn cls(&mut self) {
        self.grid.clear();
        self.cmds.push(DrawCmd::Clear(self.theme.bg));
    }

    fn cmd_theme(&mut self, name: Option<String>) {
        match name {
            None => {
                let names: Vec<String> = Theme::names().map(|n| if n == self.theme.name { format!("{n} (current)") } else { n.to_string() }).collect();
                self.print(&format!("Themes: {}", names.join(", ")));
            }
            Some(n) => match Theme::by_name(&n) {
                Some(t) => {
                    self.theme = t;
                    self.config.theme = t.name.to_string();
                    let _ = self.config.save(&self.paths.config);
                    self.grid.fg = t.fg;
                    self.cls();
                    self.print(&format!("Theme: {}", t.name));
                }
                None => self.print_error(&format!("Unknown theme '{n}'. Try: {}", Theme::names().collect::<Vec<_>>().join(", "))),
            },
        }
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
            self.audio.send_all(m.audio.drain(..));
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

    fn cmd_cd(&mut self, dir: Option<String>) {
        let target = match dir.as_deref() {
            None | Some("/") | Some("\\") => String::new(),
            Some("..") => vfs::parent(&self.cwd),
            Some(d) => {
                let d = d.replace('\\', "/");
                match self.disk.canonical_vpath(&vfs::join(&self.cwd, d.trim_end_matches('/'))) {
                    Some(p) => p,
                    None => vfs::join(&self.cwd, &d),
                }
            }
        };
        if target.is_empty() || self.disk.is_dir(&target) {
            self.cwd = target;
        } else {
            self.print_error(&format!("Folder not found: {}", dir.unwrap_or_default()));
        }
    }

    fn cmd_dir(&mut self, pattern: Option<&str>) {
        let (dir, pat) = match pattern {
            None => (self.cwd.clone(), None),
            Some(p) => {
                if p.contains('*') || p.contains('?') {
                    match p.rfind('/') {
                        Some(i) => (vfs::join(&self.cwd, &p[..i]), Some(p[i + 1..].to_string())),
                        None => (self.cwd.clone(), Some(p.to_string())),
                    }
                } else {
                    (vfs::join(&self.cwd, p), None)
                }
            }
        };
        match self.disk.list(&dir) {
            Ok(mut entries) => {
                if let Some(p) = pat {
                    entries.retain(|e| shell::wildcard_match(&p, &e.name) || shell::wildcard_match(&p, vfs::strip_lua_ext(&e.name)));
                }
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

    fn cmd_type(&mut self, name: Option<String>) {
        let Some(name) = name else {
            self.print_error("TYPE what? Usage: TYPE name");
            return;
        };
        let path = match self.resolve(&name) {
            Some(loc) => loc.main,
            None => vfs::join(&self.cwd, &name),
        };
        match self.disk.read_string(&path) {
            Ok(s) => {
                for l in s.lines() {
                    self.print(l);
                }
            }
            Err(e) => self.print_error(&e.to_string()),
        }
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
        self.start_program(loc, false);
    }

    fn start_program(&mut self, loc: ProgramLocation, from_editor: bool) {
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
            options: MachineOptions { width: w, height: h, fps: manifest.fps.clamp(1, 240), main_name, seed: self.program_seed(), immediate: false },
        };
        let handle = spawn_program(spec);
        let resolution_changed = (w, h) != self.resolution;
        self.resolution = (w, h);
        let mut grid = TextGrid::for_pixels(w, h);
        grid.cursor_visible = false;
        self.input.clear_all();
        let shell_grid = self.grid.clone();
        self.program = Some(Running { handle, from_editor, location: loc, grid, resolution_changed, fps: manifest.fps.max(1), shell_grid, alt_screen: false, filter: manifest.filter.to_ascii_lowercase() });
        self.mode = Mode::Running;
        self.program_started = true;
        self.cmds.push(DrawCmd::Clear(Color::BLACK));
    }

    fn cmd_mkdir(&mut self, dir: Option<String>) {
        let Some(dir) = dir else {
            self.print_error("Usage: MKDIR name");
            return;
        };
        match self.disk.mkdir(&vfs::join(&self.cwd, &dir)) {
            Ok(()) => self.print(&format!("Created {dir}.")),
            Err(e) => self.print_error(&e.to_string()),
        }
    }

    fn cmd_copy(&mut self, a: Option<String>, b: Option<String>) {
        let (Some(a), Some(b)) = (a, b) else {
            self.print_error("Usage: COPY from to");
            return;
        };
        let from = self.file_arg(&a);
        let mut to = vfs::join(&self.cwd, &b);
        if from.ends_with(".lua") && !to.ends_with(".lua") && !self.disk.is_dir(&to) {
            to.push_str(".lua");
        }
        if self.disk.is_dir(&to) {
            to = vfs::join(&to, vfs::file_name(&from));
        }
        match self.disk.copy(&from, &to) {
            Ok(()) => self.print(&format!("Copied {from} to {to}.")),
            Err(e) => self.print_error(&e.to_string()),
        }
    }

    fn cmd_ren(&mut self, a: Option<String>, b: Option<String>) {
        let (Some(a), Some(b)) = (a, b) else {
            self.print_error("Usage: REN from to");
            return;
        };
        let from = self.file_arg(&a);
        let mut to = vfs::join(&self.cwd, &b);
        if from.ends_with(".lua") && !to.ends_with(".lua") {
            to.push_str(".lua");
        }
        match self.disk.rename(&from, &to) {
            Ok(()) => self.print(&format!("Renamed {from} to {to}.")),
            Err(e) => self.print_error(&e.to_string()),
        }
    }

    fn cmd_del(&mut self, name: Option<String>) {
        let Some(name) = name else {
            self.print_error("Usage: DEL name");
            return;
        };
        let target = self.file_arg(&name);
        if !self.disk.exists(&target) {
            self.print_error(&format!("Not found: {name}"));
            return;
        }

        match self.disk.delete(&target) {
            Ok(()) => self.print(&format!("Deleted {target}.")),
            Err(e) => self.print_error(&e.to_string()),
        }
    }

    // ----- editor commands -------------------------------------------------------------------
    fn cmd_new(&mut self, name: Option<String>) {
        let Some(name) = name else {
            self.print_error("NEW what? Usage: NEW name");
            return;
        };
        if self.resolve(&name).is_some() {
            self.print_error(&format!("{name} already exists. Use EDIT {name}."));
            return;
        }
        self.cmd_edit(Some(name));
    }

    fn cmd_edit(&mut self, name: Option<String>) {
        let name = match name {
            Some(n) => n,
            None => {
                self.print_error("EDIT what? Usage: EDIT name");
                return;
            }
        };
        match self.resolve(&name) {
            Some(loc) => self.open_editor(loc),
            None => {
                // Create a new single-file program.
                let clean = name.trim().trim_end_matches(".lua").to_string();
                if clean.is_empty() || clean.contains(['/', '\\']) && !self.disk.is_dir(&vfs::join(&self.cwd, &vfs::parent(&clean))) {
                    self.print_error(&format!("Cannot create {name}"));
                    return;
                }
                let path = vfs::join(&self.cwd, &format!("{clean}.lua"));
                let template = shell::new_program_template(vfs::file_name(&clean));
                if let Err(e) = self.disk.write(&path, template.as_bytes()) {
                    self.print_error(&e.to_string());
                    return;
                }
                if let Some(loc) = self.resolve(&clean) {
                    self.open_editor(loc);
                }
            }
        }
    }

    fn open_editor(&mut self, loc: ProgramLocation) {
        match self.disk.read_string(&loc.main) {
            Ok(text) => {
                let name = if loc.is_cart { format!("{}/main.lua", loc.name) } else { format!("{}.lua", loc.name) };
                let mut ed = Editor::new(&loc.main, &name, &text, self.config.editor.tab_size);
                if self.disk.resolve(&loc.main).map(|_| loc.main.to_ascii_uppercase().starts_with("DEMOS/")).unwrap_or(false) {
                    ed.set_status("DEMOS are read-only: COPY it to your own folder to edit", false);
                }
                self.editor = Some(ed);
                self.mode = Mode::Editor;
            }
            Err(e) => self.print_error(&e.to_string()),
        }
    }

    fn editor_action(&mut self, action: EditorAction) {
        match action {
            EditorAction::None => {}
            EditorAction::Save => {
                if let Err(e) = self.save_editor() {
                    if let Some(ed) = &mut self.editor {
                        ed.set_status(&e, true);
                    }
                } else if let Some(ed) = &mut self.editor {
                    ed.set_status("Saved", false);
                }
            }
            EditorAction::Run => {
                if let Err(e) = self.save_editor() {
                    if let Some(ed) = &mut self.editor {
                        ed.set_status(&e, true);
                    }
                    return;
                }
                let Some(ed) = &self.editor else { return };
                let path = ed.path.clone();
                let name = vfs::strip_lua_ext(vfs::file_name(&path)).to_string();
                let loc = if path.ends_with("/main.lua") {
                    let dir = vfs::parent(&path);
                    ProgramLocation { main: path.clone(), dir: dir.clone(), manifest: Some(vfs::join(&dir, "cart.toml")).filter(|m| self.disk.exists(m)), is_cart: true, name: vfs::file_name(&dir).to_string() }
                } else {
                    ProgramLocation { main: path.clone(), dir: vfs::parent(&path), manifest: None, is_cart: false, name }
                };
                self.start_program(loc, true);
            }
            EditorAction::SaveAndExit => {
                match self.save_editor() {
                    Ok(()) => self.leave_editor(),
                    Err(e) => {
                        if let Some(ed) = &mut self.editor {
                            ed.set_status(&e, true);
                        }
                    }
                }
            }
            EditorAction::Exit => self.leave_editor(),
        }
    }

    fn save_editor(&mut self) -> Result<(), String> {
        let Some(ed) = &mut self.editor else { return Ok(()) };
        if !ed.modified {
            return Ok(());
        }
        let text = ed.text();
        match self.disk.write(&ed.path, text.as_bytes()) {
            Ok(()) => {
                ed.mark_saved();
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    fn leave_editor(&mut self) {
        self.editor = None;
        self.mode = Mode::Shell;
        self.restore_shell_grid();
        self.input.clear_all();
    }

    /// Back in the shell after the editor: redraw a clean shell screen.
    fn restore_shell_grid(&mut self) {
        if self.resolution != (DEFAULT_WIDTH, DEFAULT_HEIGHT) {
            self.resolution = (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        }
        self.grid = TextGrid::for_pixels(self.resolution.0, self.resolution.1);
        self.grid.fg = self.theme.fg;
        self.grid.bg = None;
        self.grid.cursor_visible = true;
        self.prompt_row = 0;
        self.cmds.push(DrawCmd::Clear(self.theme.bg));
    }

    fn cmd_filter(&mut self, name: Option<String>) {
        const NAMES: &[&str] = &["none", "glow", "scanlines", "neon", "crt"];
        match name {
            None => {
                let list: Vec<String> = NAMES.iter().map(|n| if *n == self.config.filter { format!("{n} (current)") } else { n.to_string() }).collect();
                self.print(&format!("Filters: {}", list.join(", ")));
            }
            Some(n) => {
                let n = n.to_ascii_lowercase();
                if NAMES.contains(&n.as_str()) {
                    self.config.filter = n.clone();
                    let _ = self.config.save(&self.paths.config);
                    self.print(&format!("Filter: {n}"));
                } else {
                    self.print_error(&format!("Unknown filter '{n}'. Try: {}", NAMES.join(", ")));
                }
            }
        }
    }

    /// Display filter to apply right now: the running cart's override, or
    /// the user's setting.
    pub fn filter_name(&self) -> &str {
        if let Some(r) = &self.program {
            if !r.filter.is_empty() {
                return &r.filter;
            }
        }
        &self.config.filter
    }

    fn cmd_export(&mut self, name: Option<String>) {
        let Some(name) = name else {
            self.print_error("Usage: EXPORT name");
            return;
        };
        let Some(loc) = self.resolve(&name) else {
            self.print_error(&format!("Program not found: {name}"));
            return;
        };
        let src = if loc.is_cart { loc.dir.clone() } else { loc.main.clone() };
        let host_src = match self.disk.resolve(&src) {
            Ok(p) => p,
            Err(e) => {
                self.print_error(&e.to_string());
                return;
            }
        };
        let dest = self.paths.export.join(host_src.file_name().unwrap_or_default());
        let result = if host_src.is_dir() { copy_dir_host(&host_src, &dest) } else { std::fs::copy(&host_src, &dest).map(|_| ()) };
        match result {
            Ok(()) => self.print(&format!("Exported to {}", dest.display())),
            Err(e) => self.print_error(&format!("Export failed: {e}")),
        }
    }

    fn cmd_import(&mut self, path: Option<String>) {
        let Some(path) = path else {
            self.print_error("Usage: IMPORT host-path   (a .lua file or a program folder)");
            return;
        };
        let host = std::path::PathBuf::from(shellexpand_home(&path));
        if !host.exists() {
            self.print_error(&format!("Not found on this computer: {}", host.display()));
            return;
        }
        let name = host.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let dest_v = vfs::join(&self.cwd, &name);
        let dest = match self.disk.resolve(&dest_v) {
            Ok(p) => p,
            Err(e) => {
                self.print_error(&e.to_string());
                return;
            }
        };
        if dest.exists() {
            self.print_error(&format!("{name} already exists here."));
            return;
        }
        let result = if host.is_dir() { copy_dir_host(&host, &dest) } else { std::fs::copy(&host, &dest).map(|_| ()) };
        match result {
            Ok(()) => self.print(&format!("Imported {name}.")),
            Err(e) => self.print_error(&format!("Import failed: {e}")),
        }
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

        // TODO combine with if statement?
        match self.mode {
            Mode::Running => {
                let r = self.program.as_ref().expect("running without program");
                FrameOutput { cmds, grid: r.grid.clone(), cursor_on, resolution: self.resolution, clear_color: Color::BLACK, running: true }
            }

            Mode::Shell => {
                self.draw_prompt();
                FrameOutput { cmds, grid: self.grid.clone(), cursor_on, resolution: self.resolution, clear_color: self.theme.bg, running: false }
            }

            Mode::Editor => {
                let theme = self.theme;
                let api = &self.api;
                if let Some(ed) = &mut self.editor {
                    ed.draw(&mut self.grid, theme, api);
                }
                FrameOutput { cmds, grid: self.grid.clone(), cursor_on, resolution: self.resolution, clear_color: theme.bg, running: false }
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
                if self.fixed_dt.is_some() || elapsed + Duration::from_millis(2) >= interval {
                    let frame = r.handle.poll_frame().unwrap().clone();
                    if let Some((w, h)) = frame.resolution {
                        if (w, h) != self.resolution {
                            self.resolution = (w, h);
                            r.resolution_changed = true;
                        }
                    }
                    r.fps = frame.fps;
                    r.alt_screen = frame.alt_screen;
                    self.audio.send_all(frame.audio);
                    cmds.extend(frame.cmds);
                    r.grid = frame.grid;
                    let snap = self.input.snapshot(&keymap);
                    r.handle.ack(snap, self.fixed_dt);
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
        let from_editor = r.from_editor;
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
            r.alt_screen = f.alt_screen;
            self.audio.send_all(f.audio);
            cmds.extend(f.cmds);
            r.grid = f.grid;
        }
        // Programs may not leave sound running (FR-A3).
        self.audio.send(AudioCmd::StopAll);

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

        self.mode = Mode::Shell;
        if r.resolution_changed || self.resolution != (DEFAULT_WIDTH, DEFAULT_HEIGHT) {
            self.resolution = (DEFAULT_WIDTH, DEFAULT_HEIGHT);
            self.grid = TextGrid::for_pixels(DEFAULT_WIDTH, DEFAULT_HEIGHT);
            cmds.push(DrawCmd::Clear(self.theme.bg));
        } else if r.alt_screen {
            // The program drew on the alternate screen: bring the shell back.
            cmds.push(DrawCmd::RestoreScreen);
            self.grid = r.shell_grid;
            self.grid.locate(0, self.prompt_row);
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

fn clock_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0x5eed, |d| d.as_nanos() as u64)
}

fn new_immediate(disk: &Jail) -> Result<Runtime, String> {
    let save = disk.sub("SAVE").ok();
    let opts = MachineOptions { immediate: true, main_name: "shell".to_string(), seed: clock_seed(), ..Default::default() };
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

fn shellexpand_home(p: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return format!("{}/{rest}", home.to_string_lossy());
        }
    }
    p.to_string()
}

fn common_prefix(items: &[String]) -> String {
    let Some(first) = items.first() else { return String::new() };
    let mut prefix: Vec<char> = first.chars().collect();
    for it in &items[1..] {
        let chars: Vec<char> = it.chars().collect();
        let mut n = 0;
        while n < prefix.len() && n < chars.len() && prefix[n].eq_ignore_ascii_case(&chars[n]) {
            n += 1;
        }
        prefix.truncate(n);
    }
    prefix.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon_core::input::KeyEvent;

    fn console() -> (Console, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::at(dir.path().to_path_buf(), None);
        let c = Console::new(ConsoleOptions { paths, config: Config::default(), demos: vec![("hello.lua", "print('hi')\n")], audio: false }).unwrap();
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
        assert!(c.screen_text().contains("Shell commands"));
        type_line(&mut c, "LUA fill_rect(10, 10, 5, 5)");
        let out = c.update(0.016);
        assert!(out.cmds.iter().any(|cmd| matches!(cmd, DrawCmd::Rect { .. })));
    }

    #[test]
    fn dir_shows_demos_and_type_prints_file() {
        let (mut c, _d) = console();
        type_line(&mut c, "dir");
        assert!(c.screen_text().contains("DEMOS"));
        type_line(&mut c, "cd demos");
        type_line(&mut c, "type hello");
        assert!(c.screen_text().contains("print('hi')"));
        assert!(c.prompt_text().contains("DEMOS"));
    }

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
        assert!(t.contains("Did you mean fill_rect?"), "{t}");
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


    #[test]
    fn new_opens_editor_and_f5_runs_then_error_jumps() {
        let (mut c, _d) = console();
        type_line(&mut c, "new game");
        assert_eq!(c.mode, Mode::Editor);
        // Replace content with a broken program.
        c.key(KeyEvent::press(KeyCode::A).with_ctrl());
        for ch in "print(1)\nnope()".chars() {
            if ch == '\n' {
                c.key(KeyEvent::press(KeyCode::Enter));
            } else {
                c.key(KeyEvent::typed(ch));
            }
        }
        c.key(KeyEvent::press(KeyCode::F5));
        assert!(c.is_running());
        pump(&mut c, 50);
        assert_eq!(c.mode, Mode::Editor);
        let ed = c.editor.as_ref().unwrap();
        assert_eq!(ed.cursor().line, 1, "cursor should jump to the error line");
        assert!(c.disk.read_string("game.lua").unwrap().contains("nope()"));
        // Esc leaves the editor (no unsaved changes).
        c.key(KeyEvent::press(KeyCode::Escape));
        assert_eq!(c.mode, Mode::Shell);
    }

    /*
    #[test]
    fn alt_screen_restores_shell_contents() {
        let (mut c, _d) = console();
        type_line(&mut c, "print('keep me')");
        c.disk.write("full.lua", b"alt_screen() print('program text') flip()").unwrap();
        type_line(&mut c, "run full");
        pump(&mut c, 40);
        assert!(!c.is_running());
        let t = c.screen_text();
        assert!(t.contains("keep me"), "{t}");
        assert!(!t.contains("program text"), "{t}");
        let out = c.update(0.016);
        let _ = out;
    }

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
    */

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
}
