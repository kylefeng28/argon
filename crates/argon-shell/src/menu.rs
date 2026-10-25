//! Pop-up menus drawn on the text grid: the F-key insert menus and the F1
//! catalog. Keyboard only.

use argon_core::input::{KeyCode, KeyEvent};
use argon_core::{Cell, TextGrid, Theme};

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub label: String,
    /// Secondary text shown dimmed (signature).
    pub detail: String,
    /// Shown at the bottom when highlighted.
    pub doc: String,
    pub snippet: String,
}

pub struct Menu {
    pub title: String,
    items: Vec<MenuItem>,
    filtered: Vec<usize>,
    pub selected: usize,
    scroll: usize,
    pub filter: String,
    pub filterable: bool,
}

pub enum MenuAction {
    None,
    Close,
    Insert(String),
}

impl Menu {
    pub fn new(title: &str, items: Vec<MenuItem>, filterable: bool) -> Menu {
        let filtered = (0..items.len()).collect();
        Menu { title: title.to_string(), items, filtered, selected: 0, scroll: 0, filter: String::new(), filterable }
    }

    fn refilter(&mut self) {
        let f = self.filter.to_lowercase();
        self.filtered = (0..self.items.len())
            .filter(|&i| f.is_empty() || self.items[i].label.to_lowercase().contains(&f))
            .collect();
        // Prefer prefix matches first.
        self.filtered.sort_by_key(|&i| !self.items[i].label.to_lowercase().starts_with(&f));
        self.selected = 0;
        self.scroll = 0;
    }

    pub fn current(&self) -> Option<&MenuItem> {
        self.filtered.get(self.selected).map(|&i| &self.items[i])
    }

    pub fn handle(&mut self, ev: &KeyEvent, visible_rows: usize) -> MenuAction {
        if !ev.pressed {
            return MenuAction::None;
        }
        match ev.code {
            KeyCode::Escape => return MenuAction::Close,
            KeyCode::Enter | KeyCode::NumpadEnter => {
                return match self.current() {
                    Some(item) => MenuAction::Insert(item.snippet.clone()),
                    None => MenuAction::Close,
                };
            }
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected + 1 < self.filtered.len() {
                    self.selected += 1;
                }
            }
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(visible_rows.max(1)),
            KeyCode::PageDown => {
                self.selected = (self.selected + visible_rows.max(1)).min(self.filtered.len().saturating_sub(1))
            }
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = self.filtered.len().saturating_sub(1),
            KeyCode::Backspace if self.filterable => {
                self.filter.pop();
                self.refilter();
            }
            _ => {
                if self.filterable {
                    if let Some(c) = ev.text {
                        if !c.is_control() && !ev.mods.ctrl {
                            self.filter.push(c);
                            self.refilter();
                        }
                    }
                } else if let Some(c) = ev.text {
                    // Jump to the first item starting with the typed letter.
                    let c = c.to_ascii_lowercase();
                    if let Some(pos) = self.filtered.iter().position(|&i| self.items[i].label.to_lowercase().starts_with(c)) {
                        self.selected = pos;
                    }
                }
            }
        }
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
        if self.selected >= self.scroll + visible_rows.max(1) {
            self.scroll = self.selected + 1 - visible_rows.max(1);
        }
        MenuAction::None
    }

    /// Draw the menu as a box in the middle of the grid.
    pub fn draw(&self, grid: &mut TextGrid, theme: &Theme) {
        let cols = grid.cols();
        let rows = grid.rows();
        let width = cols.clamp(20, 76);
        let height = rows.clamp(6, 22);
        let x0 = (cols - width) / 2;
        let y0 = (rows - height) / 2;
        let bg = Some(theme.menu_bg);
        grid.fill_rect(x0, y0, width, height, Cell { ch: ' ', fg: theme.menu_fg, bg });

        // Border
        for x in x0..x0 + width {
            grid.put_char_colored(x, y0, '\u{2500}', theme.menu_fg, bg);
            grid.put_char_colored(x, y0 + height - 1, '\u{2500}', theme.menu_fg, bg);
        }
        for y in y0..y0 + height {
            grid.put_char_colored(x0, y, '\u{2502}', theme.menu_fg, bg);
            grid.put_char_colored(x0 + width - 1, y, '\u{2502}', theme.menu_fg, bg);
        }
        grid.put_char_colored(x0, y0, '\u{250C}', theme.menu_fg, bg);
        grid.put_char_colored(x0 + width - 1, y0, '\u{2510}', theme.menu_fg, bg);
        grid.put_char_colored(x0, y0 + height - 1, '\u{2514}', theme.menu_fg, bg);
        grid.put_char_colored(x0 + width - 1, y0 + height - 1, '\u{2518}', theme.menu_fg, bg);

        let title = if self.filterable {
            format!(" {} [{}_] ", self.title, self.filter)
        } else {
            format!(" {} ", self.title)
        };
        grid.put_str_colored(x0 + 2, y0, &title, theme.menu_hl_bg, bg);

        let list_rows = height.saturating_sub(4);
        let inner_w = width - 4;
        for (row, &idx) in self.filtered.iter().skip(self.scroll).take(list_rows).enumerate() {
            let item = &self.items[idx];
            let is_sel = self.scroll + row == self.selected;
            let (fg, bgc) = if is_sel { (theme.menu_hl_fg, Some(theme.menu_hl_bg)) } else { (theme.menu_fg, bg) };
            let y = y0 + 1 + row;
            grid.fill_rect(x0 + 1, y, width - 2, 1, Cell { ch: ' ', fg, bg: bgc });
            let label_w = 18.min(inner_w);
            let label: String = item.label.chars().take(label_w).collect();
            grid.put_str_colored(x0 + 2, y, &label, fg, bgc);
            if inner_w > label_w + 2 {
                let detail: String = item.detail.chars().take(inner_w - label_w - 1).collect();
                let dfg = if is_sel { theme.menu_hl_fg } else { theme.dim };
                grid.put_str_colored(x0 + 2 + label_w + 1, y, &detail, dfg, bgc);
            }
        }
        if self.filtered.is_empty() {
            grid.put_str_colored(x0 + 2, y0 + 1, "(no matches)", theme.dim, bg);
        }
        // Doc line(s)
        let doc_y = y0 + height - 3;
        for x in x0 + 1..x0 + width - 1 {
            grid.put_char_colored(x, doc_y - 1, '\u{2500}', theme.dim, bg);
        }
        if let Some(item) = self.current() {
            let mut words = item.doc.split_whitespace();
            for line in 0..2 {
                let mut s = String::new();
                for w in words.by_ref() {
                    if s.chars().count() + w.chars().count() + 1 > inner_w {
                        break;
                    }
                    if !s.is_empty() {
                        s.push(' ');
                    }
                    s.push_str(w);
                }
                grid.put_str_colored(x0 + 2, doc_y + line, &s, theme.menu_fg, bg);
            }
        }
        if self.filtered.len() > list_rows {
            let pos = format!("{}/{}", self.selected + 1, self.filtered.len());
            grid.put_str_colored(x0 + width - 2 - pos.len(), y0 + height - 1, &pos, theme.dim, bg);
        }
    }

    pub fn list_rows(grid_rows: usize) -> usize {
        grid_rows.clamp(6, 22).saturating_sub(4)
    }
}
