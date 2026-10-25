//! A single-line editor used by the shell prompt and the editor's prompts
//! (find, go to line, confirmations).

use argon_core::input::{KeyCode, KeyEvent};

#[derive(Clone, Debug, Default)]
pub struct LineEdit {
    pub text: String,
    /// Cursor position in chars.
    pub cursor: usize,
    history: Vec<String>,
    history_pos: Option<usize>,
    stash: String,
}

pub enum LineAction {
    None,
    Changed,
    Submit(String),
    Cancel,
    Tab,
}

impl LineEdit {
    pub fn new() -> LineEdit {
        LineEdit::default()
    }

    pub fn with_history(history: Vec<String>) -> LineEdit {
        LineEdit { history, ..Default::default() }
    }

    pub fn history(&self) -> &[String] {
        &self.history
    }

    pub fn set(&mut self, s: &str) {
        self.text = s.to_string();
        self.cursor = self.text.chars().count();
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.history_pos = None;
    }

    fn byte_index(&self, char_idx: usize) -> usize {
        self.text.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(self.text.len())
    }

    pub fn insert_str(&mut self, s: &str) {
        let i = self.byte_index(self.cursor);
        self.text.insert_str(i, s);
        self.cursor += s.chars().count();
    }

    pub fn push_history(&mut self, line: &str) {
        if line.trim().is_empty() {
            return;
        }
        if self.history.last().map(|l| l == line).unwrap_or(false) {
            return;
        }
        self.history.push(line.to_string());
        if self.history.len() > 500 {
            self.history.remove(0);
        }
    }

    /// The word before the cursor (for completion).
    pub fn word_before_cursor(&self) -> (usize, String) {
        let chars: Vec<char> = self.text.chars().collect();
        let mut start = self.cursor;
        while start > 0 && (chars[start - 1].is_alphanumeric() || "_./-".contains(chars[start - 1])) {
            start -= 1;
        }
        (start, chars[start..self.cursor].iter().collect())
    }

    /// Replace the word before the cursor.
    pub fn replace_word(&mut self, start: usize, replacement: &str) {
        let b0 = self.byte_index(start);
        let b1 = self.byte_index(self.cursor);
        self.text.replace_range(b0..b1, replacement);
        self.cursor = start + replacement.chars().count();
    }

    pub fn handle(&mut self, ev: &KeyEvent) -> LineAction {
        if !ev.pressed {
            return LineAction::None;
        }
        let len = self.text.chars().count();
        match ev.code {
            KeyCode::Enter | KeyCode::NumpadEnter => {
                let line = std::mem::take(&mut self.text);
                self.cursor = 0;
                self.history_pos = None;
                LineAction::Submit(line)
            }
            KeyCode::Escape => LineAction::Cancel,
            KeyCode::Tab => LineAction::Tab,
            KeyCode::Left => {
                if ev.mods.ctrl {
                    self.cursor = self.prev_word();
                } else if self.cursor > 0 {
                    self.cursor -= 1;
                }
                LineAction::None
            }
            KeyCode::Right => {
                if ev.mods.ctrl {
                    self.cursor = self.next_word();
                } else if self.cursor < len {
                    self.cursor += 1;
                }
                LineAction::None
            }
            KeyCode::Home => {
                self.cursor = 0;
                LineAction::None
            }
            KeyCode::End => {
                self.cursor = len;
                LineAction::None
            }
            KeyCode::Backspace => {
                if ev.mods.ctrl {
                    let start = self.prev_word();
                    let b0 = self.byte_index(start);
                    let b1 = self.byte_index(self.cursor);
                    self.text.replace_range(b0..b1, "");
                    self.cursor = start;
                    LineAction::Changed
                } else if self.cursor > 0 {
                    let i = self.byte_index(self.cursor - 1);
                    self.text.remove(i);
                    self.cursor -= 1;
                    LineAction::Changed
                } else {
                    LineAction::None
                }
            }
            KeyCode::Delete => {
                if self.cursor < len {
                    let i = self.byte_index(self.cursor);
                    self.text.remove(i);
                    LineAction::Changed
                } else {
                    LineAction::None
                }
            }
            KeyCode::Up => {
                if self.history.is_empty() {
                    return LineAction::None;
                }
                let pos = match self.history_pos {
                    None => {
                        self.stash = self.text.clone();
                        self.history.len() - 1
                    }
                    Some(0) => 0,
                    Some(p) => p - 1,
                };
                self.history_pos = Some(pos);
                self.set(&self.history[pos].clone());
                LineAction::Changed
            }
            KeyCode::Down => {
                match self.history_pos {
                    None => {}
                    Some(p) if p + 1 < self.history.len() => {
                        self.history_pos = Some(p + 1);
                        self.set(&self.history[p + 1].clone());
                    }
                    Some(_) => {
                        self.history_pos = None;
                        let s = self.stash.clone();
                        self.set(&s);
                    }
                }
                LineAction::Changed
            }
            KeyCode::U if ev.mods.ctrl => {
                self.clear();
                LineAction::Changed
            }
            KeyCode::A if ev.mods.ctrl => {
                self.cursor = 0;
                LineAction::None
            }
            KeyCode::E if ev.mods.ctrl => {
                self.cursor = len;
                LineAction::None
            }
            _ => {
                if let Some(ch) = ev.text {
                    if !ch.is_control() && !ev.mods.ctrl && !ev.mods.alt {
                        self.insert_str(&ch.to_string());
                        return LineAction::Changed;
                    }
                }
                LineAction::None
            }
        }
    }

    fn prev_word(&self) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut i = self.cursor;
        while i > 0 && !chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        while i > 0 && chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        i
    }

    fn next_word(&self) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut i = self.cursor;
        while i < chars.len() && chars[i].is_alphanumeric() {
            i += 1;
        }
        while i < chars.len() && !chars[i].is_alphanumeric() {
            i += 1;
        }
        i
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_str(le: &mut LineEdit, s: &str) {
        for c in s.chars() {
            le.handle(&KeyEvent::typed(c));
        }
    }

    #[test]
    fn typing_editing_and_history() {
        let mut le = LineEdit::new();
        type_str(&mut le, "helo");
        le.handle(&KeyEvent::press(KeyCode::Left));
        type_str(&mut le, "l");
        assert_eq!(le.text, "hello");
        let LineAction::Submit(s) = le.handle(&KeyEvent::press(KeyCode::Enter)) else { panic!() };
        assert_eq!(s, "hello");
        le.push_history(&s);
        le.handle(&KeyEvent::press(KeyCode::Up));
        assert_eq!(le.text, "hello");
        le.handle(&KeyEvent::press(KeyCode::Down));
        assert_eq!(le.text, "");
    }

    #[test]
    fn word_before_cursor() {
        let mut le = LineEdit::new();
        type_str(&mut le, "run GAMES/po");
        let (start, w) = le.word_before_cursor();
        assert_eq!((start, w.as_str()), (4, "GAMES/po"));
        le.replace_word(start, "GAMES/pong");
        assert_eq!(le.text, "run GAMES/pong");
    }
}
