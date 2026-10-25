//! The full-screen program editor (FR-E1 to FR-E6).

use crate::highlight::{highlight_line, LineState, Tok};
use crate::lineedit::{LineAction, LineEdit};
use crate::menu::{Menu, MenuAction, MenuItem};
use argon_core::api::{expand_snippet, ApiTable, Category};
use argon_core::input::{KeyCode, KeyEvent};
use argon_core::{Cell, TextGrid, Theme};

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

#[derive(Clone)]
struct Snapshot {
    lines: Vec<String>,
    cur: Pos,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EditKind {
    None,
    Typing,
    Deleting,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PromptKind {
    Find,
    ReplaceFind,
    ReplaceWith,
    GotoLine,
    ConfirmExit,
}

struct Prompt {
    kind: PromptKind,
    label: String,
    le: LineEdit,
}

/// What the console should do after a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorAction {
    None,
    /// Save, then run the program.
    Run,
    /// Save and go back to the shell.
    SaveAndExit,
    /// Discard changes and go back to the shell.
    Exit,
    Save,
}

pub struct Editor {
    pub path: String,
    pub name: String,
    lines: Vec<String>,
    cur: Pos,
    anchor: Option<Pos>,
    scroll_row: usize,
    scroll_col: usize,
    pub modified: bool,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_edit: EditKind,
    clipboard: String,
    status: String,
    status_error: bool,
    menu: Option<Menu>,
    prompt: Option<Prompt>,
    placeholders: Vec<(usize, usize, usize)>,
    find_query: String,
    replace_query: String,
    tab_size: usize,
    /// Desired column when moving vertically through short lines.
    sticky_col: usize,
}

impl Editor {
    pub fn new(path: &str, name: &str, text: &str, tab_size: usize) -> Editor {
        let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Editor {
            path: path.to_string(),
            name: name.to_string(),
            lines,
            cur: Pos { line: 0, col: 0 },
            anchor: None,
            scroll_row: 0,
            scroll_col: 0,
            modified: false,
            undo: Vec::new(),
            redo: Vec::new(),
            last_edit: EditKind::None,
            clipboard: String::new(),
            status: String::new(),
            status_error: false,
            menu: None,
            prompt: None,
            placeholders: Vec::new(),
            find_query: String::new(),
            replace_query: String::new(),
            tab_size: tab_size.clamp(1, 8),
            sticky_col: 0,
        }
    }

    pub fn text(&self) -> String {
        let mut s = self.lines.join("\n");
        s.push('\n');
        s
    }

    pub fn set_status(&mut self, msg: &str, error: bool) {
        self.status = msg.to_string();
        self.status_error = error;
    }

    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    /// Move the cursor to a 1-based line (error jump).
    pub fn goto_line(&mut self, line: u32) {
        let l = (line.max(1) as usize - 1).min(self.lines.len() - 1);
        self.cur = Pos { line: l, col: 0 };
        self.anchor = None;
        self.sticky_col = 0;
    }

    pub fn cursor(&self) -> Pos {
        self.cur
    }

    // ----- editing primitives ------------------------------------------------

    fn line_len(&self, l: usize) -> usize {
        self.lines[l].chars().count()
    }

    fn byte_idx(s: &str, col: usize) -> usize {
        s.char_indices().nth(col).map(|(i, _)| i).unwrap_or(s.len())
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot { lines: self.lines.clone(), cur: self.cur }
    }

    fn begin_edit(&mut self, kind: EditKind) {
        let coalesce = kind == self.last_edit && matches!(kind, EditKind::Typing | EditKind::Deleting);
        if !coalesce {
            self.undo.push(self.snapshot());
            if self.undo.len() > 200 {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        self.last_edit = kind;
        self.modified = true;
    }

    fn selection(&self) -> Option<(Pos, Pos)> {
        let a = self.anchor?;
        if a == self.cur {
            return None;
        }
        Some(if a < self.cur { (a, self.cur) } else { (self.cur, a) })
    }

    fn selected_text(&self) -> String {
        let Some((a, b)) = self.selection() else { return String::new() };
        if a.line == b.line {
            let l = &self.lines[a.line];
            return l[Self::byte_idx(l, a.col)..Self::byte_idx(l, b.col)].to_string();
        }
        let mut out = String::new();
        let first = &self.lines[a.line];
        out.push_str(&first[Self::byte_idx(first, a.col)..]);
        for l in &self.lines[a.line + 1..b.line] {
            out.push('\n');
            out.push_str(l);
        }
        out.push('\n');
        let last = &self.lines[b.line];
        out.push_str(&last[..Self::byte_idx(last, b.col)]);
        out
    }

    fn delete_range(&mut self, a: Pos, b: Pos) {
        if a.line == b.line {
            let l = &mut self.lines[a.line];
            let (i0, i1) = (Self::byte_idx(l, a.col), Self::byte_idx(l, b.col));
            l.replace_range(i0..i1, "");
            let delta = b.col - a.col;
            self.shift_placeholders(a.line, b.col, -(delta as isize));
        } else {
            let tail = {
                let last = &self.lines[b.line];
                last[Self::byte_idx(last, b.col)..].to_string()
            };
            let first = &mut self.lines[a.line];
            first.truncate(Self::byte_idx(first, a.col));
            first.push_str(&tail);
            self.lines.drain(a.line + 1..=b.line);
            self.placeholders.retain(|p| p.0 < a.line || p.0 > b.line);
            self.shift_placeholder_lines(a.line + 1, -((b.line - a.line) as isize));
        }
        self.cur = a;
        self.anchor = None;
    }

    fn delete_selection(&mut self) -> bool {
        if let Some((a, b)) = self.selection() {
            self.delete_range(a, b);
            true
        } else {
            false
        }
    }

    /// Insert text (may contain newlines) at the cursor.
    fn insert_text(&mut self, text: &str) {
        let mut parts = text.split('\n');
        let first = parts.next().unwrap_or("");
        let l = &mut self.lines[self.cur.line];
        let idx = Self::byte_idx(l, self.cur.col);
        let tail = l[idx..].to_string();
        l.truncate(idx);
        l.push_str(first);
        let added = first.chars().count();
        self.shift_placeholders(self.cur.line, self.cur.col, added as isize);
        self.cur.col += added;
        let mut extra_lines = 0;
        for part in parts {
            extra_lines += 1;
            self.cur.line += 1;
            self.lines.insert(self.cur.line, part.to_string());
            self.cur.col = part.chars().count();
        }
        if extra_lines > 0 {
            self.shift_placeholder_lines(self.cur.line, extra_lines as isize);
        }
        self.lines[self.cur.line].push_str(&tail);
        self.sticky_col = self.cur.col;
    }

    fn shift_placeholders(&mut self, line: usize, from_col: usize, delta: isize) {
        for p in &mut self.placeholders {
            if p.0 == line && p.1 >= from_col {
                p.1 = (p.1 as isize + delta).max(0) as usize;
                p.2 = (p.2 as isize + delta).max(0) as usize;
            }
        }
    }

    fn shift_placeholder_lines(&mut self, from_line: usize, delta: isize) {
        for p in &mut self.placeholders {
            if p.0 >= from_line {
                p.0 = (p.0 as isize + delta).max(0) as usize;
            }
        }
    }

    fn indent_of(line: &str) -> String {
        line.chars().take_while(|c| *c == ' ' || *c == '\t').collect()
    }

    fn indent_unit(&self) -> String {
        " ".repeat(self.tab_size)
    }

    /// Insert a snippet: `\t` becomes an indent, new lines inherit the current
    /// indentation, `${x}` placeholders become tab stops.
    pub fn insert_snippet(&mut self, snippet: &str) {
        self.begin_edit(EditKind::Other);
        self.delete_selection();
        let indent = Self::indent_of(&self.lines[self.cur.line]);
        let unit = self.indent_unit();
        let prepared = snippet.replace('\t', &unit).replace('\n', &format!("\n{indent}"));
        let (text, ranges) = expand_snippet(&prepared);
        // Convert byte ranges into (line, col_start, col_end).
        let start = self.cur;
        let mut stops = Vec::new();
        for (b0, b1) in ranges {
            let before = &text[..b0];
            let nl = before.matches('\n').count();
            let line = start.line + nl;
            let col_base = if nl == 0 { start.col } else { 0 };
            let last_nl = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
            let c0 = col_base + text[last_nl..b0].chars().count();
            let c1 = c0 + text[b0..b1].chars().count();
            stops.push((line, c0, c1));
        }
        self.insert_text(&text);
        self.placeholders = stops;
        if !self.next_placeholder() {
            self.anchor = None;
        }
    }

    /// Select the next placeholder; false if none left.
    fn next_placeholder(&mut self) -> bool {
        if self.placeholders.is_empty() {
            return false;
        }
        let (line, c0, c1) = self.placeholders.remove(0);
        if line < self.lines.len() && c1 <= self.line_len(line) {
            self.anchor = Some(Pos { line, col: c0 });
            self.cur = Pos { line, col: c1 };
            self.sticky_col = c1;
            true
        } else {
            self.next_placeholder()
        }
    }

    fn undo(&mut self) {
        if let Some(s) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.lines = s.lines;
            self.cur = s.cur;
            self.anchor = None;
            self.modified = true;
            self.last_edit = EditKind::None;
            self.placeholders.clear();
        }
    }

    fn redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.lines = s.lines;
            self.cur = s.cur;
            self.anchor = None;
            self.modified = true;
            self.last_edit = EditKind::None;
        }
    }

    // ----- movement -----------------------------------------------------------------

    fn clamp_cursor(&mut self) {
        self.cur.line = self.cur.line.min(self.lines.len() - 1);
        self.cur.col = self.cur.col.min(self.line_len(self.cur.line));
    }

    fn move_cursor(&mut self, to: Pos, select: bool) {
        if select {
            if self.anchor.is_none() {
                self.anchor = Some(self.cur);
            }
        } else {
            self.anchor = None;
        }
        self.cur = to;
        self.clamp_cursor();
    }

    fn word_left(&self) -> Pos {
        let chars: Vec<char> = self.lines[self.cur.line].chars().collect();
        let mut c = self.cur.col;
        if c == 0 {
            return if self.cur.line > 0 { Pos { line: self.cur.line - 1, col: self.line_len(self.cur.line - 1) } } else { self.cur };
        }
        while c > 0 && !chars[c - 1].is_alphanumeric() {
            c -= 1;
        }
        while c > 0 && chars[c - 1].is_alphanumeric() {
            c -= 1;
        }
        Pos { line: self.cur.line, col: c }
    }

    fn word_right(&self) -> Pos {
        let chars: Vec<char> = self.lines[self.cur.line].chars().collect();
        let mut c = self.cur.col;
        if c >= chars.len() {
            return if self.cur.line + 1 < self.lines.len() { Pos { line: self.cur.line + 1, col: 0 } } else { self.cur };
        }
        while c < chars.len() && chars[c].is_alphanumeric() {
            c += 1;
        }
        while c < chars.len() && !chars[c].is_alphanumeric() {
            c += 1;
        }
        Pos { line: self.cur.line, col: c }
    }

    // ----- find -----------------------------------------------------------------------

    fn find_next(&mut self, from: Pos) -> bool {
        if self.find_query.is_empty() {
            return false;
        }
        let q = self.find_query.to_lowercase();
        let n = self.lines.len();
        for step in 0..=n {
            let li = (from.line + step) % n;
            let line = self.lines[li].to_lowercase();
            let start_col = if step == 0 { from.col } else { 0 };
            let start_b = Self::byte_idx(&line, start_col);
            if let Some(off) = line[start_b..].find(&q) {
                let col = line[..start_b + off].chars().count();
                let end = col + q.chars().count();
                self.anchor = Some(Pos { line: li, col });
                self.cur = Pos { line: li, col: end };
                self.sticky_col = end;
                return true;
            }
        }
        false
    }

    fn replace_all(&mut self) -> usize {
        if self.find_query.is_empty() {
            return 0;
        }
        self.begin_edit(EditKind::Other);
        let mut count = 0;
        for l in &mut self.lines {
            let c = l.matches(&self.find_query).count();
            if c > 0 {
                *l = l.replace(&self.find_query, &self.replace_query);
                count += c;
            }
        }
        self.clamp_cursor();
        self.anchor = None;
        count
    }

    // ----- key handling ---------------------------------------------------------------

    pub fn handle(&mut self, ev: &KeyEvent, api: &ApiTable, grid_rows: usize) -> EditorAction {
        if !ev.pressed {
            return EditorAction::None;
        }
        if self.menu.is_some() {
            let rows = Menu::list_rows(grid_rows);
            let action = self.menu.as_mut().unwrap().handle(ev, rows);
            match action {
                MenuAction::None => {}
                MenuAction::Close => self.menu = None,
                MenuAction::Insert(snippet) => {
                    self.menu = None;
                    self.insert_snippet(&snippet);
                }
            }
            return EditorAction::None;
        }
        if self.prompt.is_some() {
            return self.handle_prompt(ev);
        }
        self.status.clear();
        self.status_error = false;
        let shift = ev.mods.shift;
        let ctrl = ev.mods.ctrl;

        match ev.code {
            KeyCode::F1 => self.open_catalog(api),
            KeyCode::F2 => self.open_menu("Control", control_items()),
            KeyCode::F3 if !ctrl => {
                if self.find_query.is_empty() {
                    self.open_prompt(PromptKind::Find, "Find:");
                } else {
                    let from = Pos { line: self.cur.line, col: self.cur.col };
                    if !self.find_next(from) {
                        self.set_status("Not found", true);
                    }
                }
            }
            KeyCode::F4 => self.open_menu("Draw", category_items(api, &[Category::Draw, Category::Image])),
            KeyCode::F6 => self.open_menu("Math", category_items(api, &[Category::Math, Category::System])),
            KeyCode::F7 => self.open_menu("Text & Input", category_items(api, &[Category::Text, Category::Input])),
            KeyCode::F8 => self.open_menu("Files", category_items(api, &[Category::File])),
            KeyCode::F9 => self.open_menu("GPU", category_items(api, &[Category::Gpu, Category::Sound])),
            KeyCode::F5 => return EditorAction::Run,
            KeyCode::F10 => return EditorAction::SaveAndExit,
            KeyCode::Escape => {
                if self.anchor.is_some() || !self.placeholders.is_empty() {
                    self.anchor = None;
                    self.placeholders.clear();
                } else if self.modified {
                    self.open_prompt(PromptKind::ConfirmExit, "Save changes? (Y)es / (N)o / Esc = cancel");
                } else {
                    return EditorAction::Exit;
                }
            }
            KeyCode::S if ctrl => return EditorAction::Save,
            KeyCode::R if ctrl => return EditorAction::Run,
            KeyCode::Z if ctrl => self.undo(),
            KeyCode::Y if ctrl => self.redo(),
            KeyCode::F if ctrl => self.open_prompt(PromptKind::Find, "Find:"),
            KeyCode::H if ctrl => self.open_prompt(PromptKind::ReplaceFind, "Replace:"),
            KeyCode::G if ctrl => self.open_prompt(PromptKind::GotoLine, "Go to line:"),
            KeyCode::A if ctrl => {
                self.anchor = Some(Pos { line: 0, col: 0 });
                self.cur = Pos { line: self.lines.len() - 1, col: self.line_len(self.lines.len() - 1) };
            }
            KeyCode::C if ctrl => {
                if self.selection().is_some() {
                    self.clipboard = self.selected_text();
                    self.set_status("Copied", false);
                } else {
                    self.clipboard = format!("{}\n", self.lines[self.cur.line]);
                    self.set_status("Copied line", false);
                }
            }
            KeyCode::X if ctrl => {
                self.begin_edit(EditKind::Other);
                if self.selection().is_some() {
                    self.clipboard = self.selected_text();
                    self.delete_selection();
                } else {
                    self.clipboard = format!("{}\n", self.lines[self.cur.line]);
                    if self.lines.len() > 1 {
                        self.lines.remove(self.cur.line);
                        self.placeholders.clear();
                    } else {
                        self.lines[0].clear();
                    }
                    self.clamp_cursor();
                }
            }
            KeyCode::V if ctrl => {
                if !self.clipboard.is_empty() {
                    self.begin_edit(EditKind::Other);
                    self.delete_selection();
                    let clip = self.clipboard.clone();
                    self.insert_text(clip.trim_end_matches('\n'));
                    if clip.ends_with('\n') && !clip.contains("\n\n") && clip.matches('\n').count() == 1 {
                        // Whole-line paste: put it on its own line.
                        self.insert_text("\n");
                    }
                }
            }
            KeyCode::D if ctrl => {
                // Duplicate line
                self.begin_edit(EditKind::Other);
                let l = self.lines[self.cur.line].clone();
                self.lines.insert(self.cur.line + 1, l);
                self.cur.line += 1;
            }
            KeyCode::Slash if ctrl => self.toggle_comment(),
            KeyCode::Up if ev.mods.alt => self.swap_line(-1),
            KeyCode::Down if ev.mods.alt => self.swap_line(1),
            KeyCode::Left => {
                let to = if ctrl {
                    self.word_left()
                } else if self.cur.col > 0 {
                    Pos { line: self.cur.line, col: self.cur.col - 1 }
                } else if self.cur.line > 0 {
                    Pos { line: self.cur.line - 1, col: self.line_len(self.cur.line - 1) }
                } else {
                    self.cur
                };
                self.move_cursor(to, shift);
                self.sticky_col = self.cur.col;
            }
            KeyCode::Right => {
                let to = if ctrl {
                    self.word_right()
                } else if self.cur.col < self.line_len(self.cur.line) {
                    Pos { line: self.cur.line, col: self.cur.col + 1 }
                } else if self.cur.line + 1 < self.lines.len() {
                    Pos { line: self.cur.line + 1, col: 0 }
                } else {
                    self.cur
                };
                self.move_cursor(to, shift);
                self.sticky_col = self.cur.col;
            }
            KeyCode::Up => {
                if self.cur.line > 0 {
                    let to = Pos { line: self.cur.line - 1, col: self.sticky_col };
                    self.move_cursor(to, shift);
                } else {
                    self.move_cursor(Pos { line: 0, col: 0 }, shift);
                }
            }
            KeyCode::Down => {
                if self.cur.line + 1 < self.lines.len() {
                    let to = Pos { line: self.cur.line + 1, col: self.sticky_col };
                    self.move_cursor(to, shift);
                } else {
                    let l = self.lines.len() - 1;
                    self.move_cursor(Pos { line: l, col: self.line_len(l) }, shift);
                }
            }
            KeyCode::Home => {
                let to = if ctrl {
                    Pos { line: 0, col: 0 }
                } else {
                    let indent = Self::indent_of(&self.lines[self.cur.line]).chars().count();
                    Pos { line: self.cur.line, col: if self.cur.col == indent { 0 } else { indent } }
                };
                self.move_cursor(to, shift);
                self.sticky_col = self.cur.col;
            }
            KeyCode::End => {
                let to = if ctrl {
                    let l = self.lines.len() - 1;
                    Pos { line: l, col: self.line_len(l) }
                } else {
                    Pos { line: self.cur.line, col: self.line_len(self.cur.line) }
                };
                self.move_cursor(to, shift);
                self.sticky_col = self.cur.col;
            }
            KeyCode::PageUp => {
                let page = grid_rows.saturating_sub(2).max(1);
                let to = Pos { line: self.cur.line.saturating_sub(page), col: self.sticky_col };
                self.scroll_row = self.scroll_row.saturating_sub(page);
                self.move_cursor(to, shift);
            }
            KeyCode::PageDown => {
                let page = grid_rows.saturating_sub(2).max(1);
                let to = Pos { line: (self.cur.line + page).min(self.lines.len() - 1), col: self.sticky_col };
                self.scroll_row = (self.scroll_row + page).min(self.lines.len().saturating_sub(1));
                self.move_cursor(to, shift);
            }
            KeyCode::Backspace => {
                self.begin_edit(EditKind::Deleting);
                if !self.delete_selection() {
                    if self.cur.col > 0 {
                        // Dedent if only spaces before the cursor.
                        let line = &self.lines[self.cur.line];
                        let before: String = line.chars().take(self.cur.col).collect();
                        let n = if !before.is_empty() && before.chars().all(|c| c == ' ') {
                            let r = self.cur.col % self.tab_size;
                            if r == 0 { self.tab_size } else { r }
                        } else {
                            1
                        };
                        let a = Pos { line: self.cur.line, col: self.cur.col - n };
                        self.delete_range(a, self.cur);
                    } else if self.cur.line > 0 {
                        let a = Pos { line: self.cur.line - 1, col: self.line_len(self.cur.line - 1) };
                        self.delete_range(a, self.cur);
                    }
                }
                self.sticky_col = self.cur.col;
            }
            KeyCode::Delete => {
                self.begin_edit(EditKind::Deleting);
                if !self.delete_selection() {
                    if self.cur.col < self.line_len(self.cur.line) {
                        let b = Pos { line: self.cur.line, col: self.cur.col + 1 };
                        self.delete_range(self.cur, b);
                    } else if self.cur.line + 1 < self.lines.len() {
                        let b = Pos { line: self.cur.line + 1, col: 0 };
                        self.delete_range(self.cur, b);
                    }
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                self.begin_edit(EditKind::Other);
                self.delete_selection();
                let line = &self.lines[self.cur.line];
                let before: String = line.chars().take(self.cur.col).collect();
                let mut indent = Self::indent_of(line);
                if opens_block(before.trim_end()) {
                    indent.push_str(&self.indent_unit());
                }
                self.insert_text(&format!("\n{indent}"));
                self.placeholders.clear();
            }
            KeyCode::Tab => {
                if !self.placeholders.is_empty() && !shift {
                    self.next_placeholder();
                } else if shift {
                    self.begin_edit(EditKind::Other);
                    let line = &self.lines[self.cur.line];
                    let indent = Self::indent_of(line).chars().count();
                    let n = indent.min(self.tab_size);
                    if n > 0 {
                        self.delete_range(Pos { line: self.cur.line, col: 0 }, Pos { line: self.cur.line, col: n });
                    }
                } else {
                    self.begin_edit(EditKind::Typing);
                    self.delete_selection();
                    let n = self.tab_size - (self.cur.col % self.tab_size);
                    self.insert_text(&" ".repeat(n));
                }
            }
            _ => {
                if let Some(ch) = ev.text {
                    if !ch.is_control() && !ctrl && !ev.mods.alt {
                        self.begin_edit(EditKind::Typing);
                        self.delete_selection();
                        self.insert_text(&ch.to_string());
                        self.maybe_dedent();
                    }
                }
            }
        }
        self.clamp_cursor();
        EditorAction::None
    }

    /// After typing, if the line is exactly an indented block closer, dedent.
    fn maybe_dedent(&mut self) {
        let line = &self.lines[self.cur.line];
        let trimmed = line.trim();
        if matches!(trimmed, "end" | "else" | "elseif" | "until" | "}") && line.starts_with(' ') {
            let indent = Self::indent_of(line).chars().count();
            // Only dedent relative to the previous line's indentation.
            let prev_indent = if self.cur.line > 0 { Self::indent_of(&self.lines[self.cur.line - 1]).chars().count() } else { 0 };
            if indent >= prev_indent && indent >= self.tab_size && !opens_block(self.lines.get(self.cur.line.wrapping_sub(1)).map(|s| s.trim_end()).unwrap_or("")) {
                let n = self.tab_size;
                self.delete_range(Pos { line: self.cur.line, col: 0 }, Pos { line: self.cur.line, col: n });
            } else if indent > prev_indent && opens_block(self.lines.get(self.cur.line.wrapping_sub(1)).map(|s| s.trim_end()).unwrap_or("")) {
                // `if x then` / `end` typed right below: align with the opener.
                let n = indent - prev_indent;
                self.delete_range(Pos { line: self.cur.line, col: 0 }, Pos { line: self.cur.line, col: n });
            }
        }
    }

    fn toggle_comment(&mut self) {
        self.begin_edit(EditKind::Other);
        let (a, b) = match self.selection() {
            Some((a, b)) => (a.line, b.line),
            None => (self.cur.line, self.cur.line),
        };
        let all_commented = (a..=b).all(|l| self.lines[l].trim_start().starts_with("--") || self.lines[l].trim().is_empty());
        for l in a..=b {
            let line = &self.lines[l];
            if line.trim().is_empty() {
                continue;
            }
            let indent = Self::indent_of(line).len();
            if all_commented {
                let rest = &line[indent..];
                let rest = rest.strip_prefix("-- ").or_else(|| rest.strip_prefix("--")).unwrap_or(rest);
                self.lines[l] = format!("{}{}", &line[..indent], rest);
            } else {
                self.lines[l] = format!("{}-- {}", &line[..indent], &line[indent..]);
            }
        }
        self.clamp_cursor();
    }

    fn swap_line(&mut self, dir: isize) {
        let target = self.cur.line as isize + dir;
        if target < 0 || target >= self.lines.len() as isize {
            return;
        }
        self.begin_edit(EditKind::Other);
        self.lines.swap(self.cur.line, target as usize);
        self.cur.line = target as usize;
        self.anchor = None;
    }

    fn open_menu(&mut self, title: &str, items: Vec<MenuItem>) {
        self.menu = Some(Menu::new(title, items, false));
    }

    fn open_catalog(&mut self, api: &ApiTable) {
        let items = api
            .sorted()
            .into_iter()
            .map(|e| MenuItem { label: e.name.to_string(), detail: e.signature.to_string(), doc: e.doc.to_string(), snippet: e.snippet.to_string() })
            .collect();
        self.menu = Some(Menu::new("Catalog (type to filter)", items, true));
    }

    fn open_prompt(&mut self, kind: PromptKind, label: &str) {
        let mut le = LineEdit::new();
        match kind {
            PromptKind::Find | PromptKind::ReplaceFind => {
                let sel = self.selected_text();
                if !sel.is_empty() && !sel.contains('\n') {
                    le.set(&sel);
                } else {
                    le.set(&self.find_query.clone());
                }
            }
            _ => {}
        }
        self.prompt = Some(Prompt { kind, label: label.to_string(), le });
    }

    fn handle_prompt(&mut self, ev: &KeyEvent) -> EditorAction {
        let Some(p) = self.prompt.as_mut() else { return EditorAction::None };
        if p.kind == PromptKind::ConfirmExit {
            match (ev.code, ev.text.map(|c| c.to_ascii_lowercase())) {
                (KeyCode::Escape, _) => self.prompt = None,
                (_, Some('y')) => {
                    self.prompt = None;
                    return EditorAction::SaveAndExit;
                }
                (_, Some('n')) => {
                    self.prompt = None;
                    return EditorAction::Exit;
                }
                _ => {}
            }
            return EditorAction::None;
        }
        match p.le.handle(ev) {
            LineAction::Cancel => self.prompt = None,
            LineAction::Submit(text) => {
                let kind = p.kind;
                self.prompt = None;
                match kind {
                    PromptKind::Find => {
                        self.find_query = text;
                        let from = self.cur;
                        if !self.find_next(from) {
                            self.set_status(&format!("'{}' not found", self.find_query), true);
                        }
                    }
                    PromptKind::ReplaceFind => {
                        self.find_query = text;
                        if !self.find_query.is_empty() {
                            self.open_prompt(PromptKind::ReplaceWith, &format!("Replace '{}' with:", self.find_query));
                        }
                    }
                    PromptKind::ReplaceWith => {
                        self.replace_query = text;
                        let n = self.replace_all();
                        self.set_status(&format!("Replaced {n} occurrence(s)"), false);
                    }
                    PromptKind::GotoLine => {
                        if let Ok(n) = text.trim().parse::<u32>() {
                            self.goto_line(n);
                        }
                    }
                    PromptKind::ConfirmExit => {}
                }
            }
            _ => {}
        }
        EditorAction::None
    }

    // ----- drawing -----------------------------------------------------------------------

    fn gutter_width(&self) -> usize {
        let digits = format!("{}", self.lines.len()).len().max(3);
        digits + 2
    }

    fn ensure_visible(&mut self, text_rows: usize, text_cols: usize) {
        if self.cur.line < self.scroll_row {
            self.scroll_row = self.cur.line;
        }
        if self.cur.line >= self.scroll_row + text_rows {
            self.scroll_row = self.cur.line + 1 - text_rows;
        }
        if self.cur.col < self.scroll_col {
            self.scroll_col = self.cur.col;
        }
        if self.cur.col >= self.scroll_col + text_cols {
            self.scroll_col = self.cur.col + 1 - text_cols;
        }
    }

    pub fn draw(&mut self, grid: &mut TextGrid, theme: &Theme, api: &ApiTable) {
        let cols = grid.cols();
        let rows = grid.rows();
        let text_rows = rows.saturating_sub(1).max(1);
        let gutter = self.gutter_width();
        let text_cols = cols.saturating_sub(gutter).max(1);
        self.ensure_visible(text_rows, text_cols);

        grid.clear_with(theme.fg, Some(theme.bg));
        grid.cursor_visible = self.menu.is_none();

        // Highlight state up to the first visible line.
        let mut state = LineState::Normal;
        let is_builtin = |w: &str| api.get(w).is_some();
        for l in &self.lines[..self.scroll_row.min(self.lines.len())] {
            state = highlight_line(l, state, &is_builtin).1;
        }
        let sel = self.selection();

        for row in 0..text_rows {
            let li = self.scroll_row + row;
            if li >= self.lines.len() {
                break;
            }
            let line = &self.lines[li];
            let (toks, next) = highlight_line(line, state, &is_builtin);
            state = next;
            let num = format!("{:>w$} ", li + 1, w = gutter - 2);
            let num_fg = if li == self.cur.line { theme.accent } else { theme.dim };
            grid.put_str_colored(0, row, &num, num_fg, Some(theme.bg));
            for (ci, ch) in line.chars().enumerate().skip(self.scroll_col).take(text_cols) {
                let fg = match toks[ci] {
                    Tok::Plain => theme.fg,
                    Tok::Keyword => theme.syn_keyword,
                    Tok::String => theme.syn_string,
                    Tok::Number => theme.syn_number,
                    Tok::Comment => theme.syn_comment,
                    Tok::Builtin => theme.syn_builtin,
                };
                let p = Pos { line: li, col: ci };
                let selected = sel.map(|(a, b)| p >= a && p < b).unwrap_or(false);
                let bg = if selected { Some(theme.selection_bg) } else { Some(theme.bg) };
                grid.put_char_colored(gutter + ci - self.scroll_col, row, ch, fg, bg);
            }
            // Selection extending past end of line.
            if let Some((a, b)) = sel {
                if li >= a.line && li < b.line {
                    let len = line.chars().count();
                    if len >= self.scroll_col && gutter + len - self.scroll_col < cols {
                        grid.put_char_colored(gutter + len - self.scroll_col, row, ' ', theme.fg, Some(theme.selection_bg));
                    }
                }
            }
        }

        // Status line
        let status_row = rows - 1;
        grid.fill_rect(0, status_row, cols, 1, Cell { ch: ' ', fg: theme.status_fg, bg: Some(theme.status_bg) });
        if let Some(p) = &self.prompt {
            let s = format!(" {} {}", p.label, p.le.text);
            grid.put_str_colored(0, status_row, &s, theme.status_fg, Some(theme.status_bg));
            grid.cursor_col = (p.label.chars().count() + 2 + p.le.cursor).min(cols - 1);
            grid.cursor_row = status_row;
            grid.cursor_visible = true;
        } else {
            let left = format!(" {}{}  Ln {}, Col {}", self.name, if self.modified { "*" } else { "" }, self.cur.line + 1, self.cur.col + 1);
            grid.put_str_colored(0, status_row, &left, theme.status_fg, Some(theme.status_bg));
            let right = if self.status.is_empty() {
                "F1 Catalog  F2 Control  F4 Draw  F5 Run  F10 Save+Exit".to_string()
            } else {
                self.status.clone()
            };
            let fg = if self.status_error { theme.error } else { theme.status_fg };
            let rx = cols.saturating_sub(right.chars().count() + 1);
            if rx > left.chars().count() + 1 {
                grid.put_str_colored(rx, status_row, &right, fg, Some(theme.status_bg));
            } else {
                let avail = cols.saturating_sub(left.chars().count() + 2);
                let short: String = right.chars().take(avail).collect();
                grid.put_str_colored(left.chars().count() + 1, status_row, &short, fg, Some(theme.status_bg));
            }
            grid.cursor_col = (gutter + self.cur.col - self.scroll_col).min(cols - 1);
            grid.cursor_row = self.cur.line - self.scroll_row;
        }

        if let Some(menu) = &self.menu {
            menu.draw(grid, theme);
        }
    }
}

/// Does this line start a block that the next line should be indented into?
fn opens_block(line: &str) -> bool {
    let l = line.trim();
    if l.is_empty() || l.starts_with("--") {
        return false;
    }
    let last_word = l.rsplit(|c: char| !c.is_alphanumeric() && c != '_').find(|w| !w.is_empty()).unwrap_or("");
    matches!(last_word, "then" | "do" | "else" | "repeat")
        || l.ends_with('{')
        || l.ends_with('(')
        || (l.contains("function") && l.ends_with(')') && !l.contains("end"))
}

fn control_items() -> Vec<MenuItem> {
    argon_lua::docs::CONTROL_SNIPPETS
        .iter()
        .map(|(label, snippet)| MenuItem {
            label: label.to_string(),
            detail: snippet.lines().next().unwrap_or("").replace("${", "").replace('}', ""),
            doc: format!("Insert a {label} block."),
            snippet: snippet.to_string(),
        })
        .collect()
}

fn category_items(api: &ApiTable, cats: &[Category]) -> Vec<MenuItem> {
    let mut items = Vec::new();
    for c in cats {
        for e in api.in_category(*c) {
            items.push(MenuItem { label: e.name.to_string(), detail: e.signature.to_string(), doc: e.doc.to_string(), snippet: e.snippet.to_string() });
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed(text: &str) -> Editor {
        Editor::new("t.lua", "t", text, 2)
    }

    fn api() -> ApiTable {
        argon_lua::docs::api_table()
    }

    fn type_str(e: &mut Editor, s: &str) {
        let a = api();
        for c in s.chars() {
            if c == '\n' {
                e.handle(&KeyEvent::press(KeyCode::Enter), &a, 25);
            } else {
                e.handle(&KeyEvent::typed(c), &a, 25);
            }
        }
    }

    #[test]
    fn typing_and_auto_indent() {
        let mut e = ed("");
        type_str(&mut e, "if x then\nfoo()\nend");
        assert_eq!(e.text(), "if x then\n  foo()\nend\n");
        assert!(e.modified);
    }

    #[test]
    fn undo_redo() {
        let mut e = ed("abc");
        let a = api();
        e.handle(&KeyEvent::press(KeyCode::End), &a, 25);
        type_str(&mut e, "d");
        assert_eq!(e.text(), "abcd\n");
        e.handle(&KeyEvent::press(KeyCode::Z).with_ctrl(), &a, 25);
        assert_eq!(e.text(), "abc\n");
        e.handle(&KeyEvent::press(KeyCode::Y).with_ctrl(), &a, 25);
        assert_eq!(e.text(), "abcd\n");
    }

    #[test]
    fn snippet_placeholders_and_tab() {
        let mut e = ed("");
        e.insert_snippet("fill_rect(${x}, ${y}, ${w}, ${h})");
        assert_eq!(e.text(), "fill_rect(x, y, w, h)\n");
        // First placeholder selected: typing replaces it.
        type_str(&mut e, "10");
        assert_eq!(e.text(), "fill_rect(10, y, w, h)\n");
        let a = api();
        e.handle(&KeyEvent::press(KeyCode::Tab), &a, 25);
        type_str(&mut e, "20");
        assert_eq!(e.text(), "fill_rect(10, 20, w, h)\n");
        e.handle(&KeyEvent::press(KeyCode::Tab), &a, 25);
        e.handle(&KeyEvent::press(KeyCode::Tab), &a, 25);
        type_str(&mut e, "5");
        assert_eq!(e.text(), "fill_rect(10, 20, w, 5)\n");
    }

    #[test]
    fn selection_copy_paste_and_find() {
        let mut e = ed("hello world\nsecond");
        let a = api();
        for _ in 0..5 {
            e.handle(&KeyEvent::press(KeyCode::Right).with_shift(), &a, 25);
        }
        e.handle(&KeyEvent::press(KeyCode::C).with_ctrl(), &a, 25);
        e.handle(&KeyEvent::press(KeyCode::End), &a, 25);
        e.handle(&KeyEvent::press(KeyCode::V).with_ctrl(), &a, 25);
        assert_eq!(e.text(), "hello worldhello\nsecond\n");
        e.handle(&KeyEvent::press(KeyCode::F).with_ctrl(), &a, 25);
        type_str(&mut e, "sec");
        e.handle(&KeyEvent::press(KeyCode::Enter), &a, 25);
        assert_eq!(e.cursor(), Pos { line: 1, col: 3 });
    }

    #[test]
    fn menu_insert_from_catalog() {
        let mut e = ed("");
        let a = api();
        e.handle(&KeyEvent::press(KeyCode::F1), &a, 25);
        type_str(&mut e, "fill_re");
        e.handle(&KeyEvent::press(KeyCode::Enter), &a, 25);
        assert_eq!(e.text(), "fill_rect(x, y, w, h)\n");
    }

    #[test]
    fn draw_does_not_panic_on_small_grid() {
        let mut e = ed(&"x = 1\n".repeat(100));
        e.goto_line(80);
        let mut g = TextGrid::new(20, 5);
        e.draw(&mut g, Theme::default_theme(), &api());
        assert!(g.row_text(0).contains("80") || g.row_text(1).contains("80") || g.row_text(2).contains("80") || g.row_text(3).contains("80"));
    }
}
