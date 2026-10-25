//! A single-line editor used by the shell prompt and the editor's prompts
//! (find, go to line, confirmations).

use argon_core::input::{KeyCode, KeyEvent};

#[derive(Clone, Debug, Default)]
pub struct LineEdit {
    pub text: String,
    /// Cursor position in chars.
    pub cursor: usize,
}

pub enum LineAction {
    None,
    Changed,
    Submit(String),
    Cancel,
}

impl LineEdit {
    pub fn new() -> LineEdit {
        LineEdit::default()
    }

    pub fn set(&mut self, s: &str) {
        self.text = s.to_string();
        self.cursor = self.text.chars().count();
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    fn byte_index(&self, char_idx: usize) -> usize {
        self.text.char_indices().nth(char_idx).map(|(i, _)| i).unwrap_or(self.text.len())
    }

    pub fn insert_str(&mut self, s: &str) {
        let i = self.byte_index(self.cursor);
        self.text.insert_str(i, s);
        self.cursor += s.chars().count();
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
                LineAction::Submit(line)
            }
            KeyCode::Escape => LineAction::Cancel,
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
    fn typing_editing() {
        let mut le = LineEdit::new();
        type_str(&mut le, "helo");
        le.handle(&KeyEvent::press(KeyCode::Left));
        type_str(&mut le, "l");
        assert_eq!(le.text, "hello");
        let LineAction::Submit(s) = le.handle(&KeyEvent::press(KeyCode::Enter)) else { panic!() };
        assert_eq!(s, "hello");
    }
}
