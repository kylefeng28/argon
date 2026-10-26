//! Portable key names, key events and the per-frame input snapshot.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Layout-independent key codes (US physical positions), named the way
/// programs see them in `key("...")`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyCode {
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    #[serde(rename = "0")] Digit0, #[serde(rename = "1")] Digit1, #[serde(rename = "2")] Digit2,
    #[serde(rename = "3")] Digit3, #[serde(rename = "4")] Digit4, #[serde(rename = "5")] Digit5,
    #[serde(rename = "6")] Digit6, #[serde(rename = "7")] Digit7, #[serde(rename = "8")] Digit8,
    #[serde(rename = "9")] Digit9,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Up, Down, Left, Right,
    Space, Enter, Escape, Backspace, Tab, Delete, Insert, Home, End, PageUp, PageDown,
    Shift, Ctrl, Alt, Super, CapsLock,
    Minus, Equal, BracketLeft, BracketRight, Backslash, Semicolon, Quote, Comma, Period, Slash, Backquote,
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide, NumpadEnter, NumpadDecimal,
    Unknown,
}

impl KeyCode {
    /// The name used in Lua and in config files.
    pub fn name(self) -> &'static str {
        use KeyCode::*;
        match self {
            A => "a", B => "b", C => "c", D => "d", E => "e", F => "f", G => "g", H => "h", I => "i",
            J => "j", K => "k", L => "l", M => "m", N => "n", O => "o", P => "p", Q => "q", R => "r",
            S => "s", T => "t", U => "u", V => "v", W => "w", X => "x", Y => "y", Z => "z",
            Digit0 => "0", Digit1 => "1", Digit2 => "2", Digit3 => "3", Digit4 => "4",
            Digit5 => "5", Digit6 => "6", Digit7 => "7", Digit8 => "8", Digit9 => "9",
            F1 => "f1", F2 => "f2", F3 => "f3", F4 => "f4", F5 => "f5", F6 => "f6",
            F7 => "f7", F8 => "f8", F9 => "f9", F10 => "f10", F11 => "f11", F12 => "f12",
            Up => "up", Down => "down", Left => "left", Right => "right",
            Space => "space", Enter => "enter", Escape => "escape", Backspace => "backspace",
            Tab => "tab", Delete => "delete", Insert => "insert", Home => "home", End => "end",
            PageUp => "pageup", PageDown => "pagedown",
            Shift => "shift", Ctrl => "ctrl", Alt => "alt", Super => "super", CapsLock => "capslock",
            Minus => "-", Equal => "=", BracketLeft => "[", BracketRight => "]", Backslash => "\\",
            Semicolon => ";", Quote => "'", Comma => ",", Period => ".", Slash => "/", Backquote => "`",
            Numpad0 => "kp0", Numpad1 => "kp1", Numpad2 => "kp2", Numpad3 => "kp3", Numpad4 => "kp4",
            Numpad5 => "kp5", Numpad6 => "kp6", Numpad7 => "kp7", Numpad8 => "kp8", Numpad9 => "kp9",
            NumpadAdd => "kp+", NumpadSubtract => "kp-", NumpadMultiply => "kp*", NumpadDivide => "kp/",
            NumpadEnter => "kpenter", NumpadDecimal => "kp.",
            Unknown => "unknown",
        }
    }

    pub fn from_name(s: &str) -> Option<KeyCode> {
        let s = s.to_ascii_lowercase();
        ALL_KEYS.iter().copied().find(|k| k.name() == s).or(match s.as_str() {
            "esc" => Some(KeyCode::Escape),
            "return" => Some(KeyCode::Enter),
            "rshift" | "lshift" => Some(KeyCode::Shift),
            "lctrl" | "rctrl" | "control" => Some(KeyCode::Ctrl),
            _ => None,
        })
    }
}

pub const ALL_KEYS: &[KeyCode] = {
    use KeyCode::*;
    &[
        A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
        Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
        F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, Up, Down, Left, Right,
        Space, Enter, Escape, Backspace, Tab, Delete, Insert, Home, End, PageUp, PageDown,
        Shift, Ctrl, Alt, Super, CapsLock, Minus, Equal, BracketLeft, BracketRight, Backslash,
        Semicolon, Quote, Comma, Period, Slash, Backquote, Numpad0, Numpad1, Numpad2, Numpad3,
        Numpad4, Numpad5, Numpad6, Numpad7, Numpad8, Numpad9, NumpadAdd, NumpadSubtract,
        NumpadMultiply, NumpadDivide, NumpadEnter, NumpadDecimal,
    ]
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

/// A key press as delivered to the shell/editor and queued for programs.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KeyEvent {
    pub code: KeyCode,
    /// Printable text produced by this press (layout-dependent), if any.
    pub text: Option<char>,
    pub mods: Modifiers,
    pub pressed: bool,
    pub repeat: bool,
}

impl KeyEvent {
    pub fn press(code: KeyCode) -> KeyEvent {
        KeyEvent { code, text: None, mods: Modifiers::default(), pressed: true, repeat: false }
    }

    pub fn typed(ch: char) -> KeyEvent {
        let code = KeyCode::from_name(&ch.to_ascii_lowercase().to_string()).unwrap_or(KeyCode::Unknown);
        let mods = Modifiers { shift: ch.is_ascii_uppercase(), ..Default::default() };
        KeyEvent { code, text: Some(ch), mods, pressed: true, repeat: false }
    }

    pub fn with_ctrl(mut self) -> KeyEvent {
        self.mods.ctrl = true;
        self
    }

    pub fn with_shift(mut self) -> KeyEvent {
        self.mods.shift = true;
        self
    }

    /// The value `read_key()` returns: the character for printable keys,
    /// otherwise the key name.
    pub fn read_key_name(&self) -> String {
        match self.text {
            Some(c) if !c.is_control() => c.to_string(),
            _ => self.code.name().to_string(),
        }
    }
}

/// The abstract game pad (FR-I1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PadButton {
    Up, Down, Left, Right, A, B, X, Y, L, R, Start, Select,
}

impl PadButton {
    pub const ALL: [PadButton; 12] = [
        PadButton::Up, PadButton::Down, PadButton::Left, PadButton::Right,
        PadButton::A, PadButton::B, PadButton::X, PadButton::Y,
        PadButton::L, PadButton::R, PadButton::Start, PadButton::Select,
    ];

    pub fn from_name(s: &str) -> Option<PadButton> {
        match s.to_ascii_lowercase().as_str() {
            "up" => Some(PadButton::Up),
            "down" => Some(PadButton::Down),
            "left" => Some(PadButton::Left),
            "right" => Some(PadButton::Right),
            "a" => Some(PadButton::A),
            "b" => Some(PadButton::B),
            "x" => Some(PadButton::X),
            "y" => Some(PadButton::Y),
            "l" => Some(PadButton::L),
            "r" => Some(PadButton::R),
            "start" => Some(PadButton::Start),
            "select" => Some(PadButton::Select),
            _ => None,
        }
    }
}

/// Keyboard -> pad mapping, configurable in config.toml (D14).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyMap {
    pub pad: HashMap<PadButton, Vec<KeyCode>>,
}

impl Default for KeyMap {
    fn default() -> Self {
        use KeyCode::*;
        let mut pad = HashMap::new();
        pad.insert(PadButton::Up, vec![Up]);
        pad.insert(PadButton::Down, vec![Down]);
        pad.insert(PadButton::Left, vec![Left]);
        pad.insert(PadButton::Right, vec![Right]);
        pad.insert(PadButton::A, vec![Z]);
        pad.insert(PadButton::B, vec![X]);
        pad.insert(PadButton::X, vec![A]);
        pad.insert(PadButton::Y, vec![S]);
        pad.insert(PadButton::L, vec![Q]);
        pad.insert(PadButton::R, vec![W]);
        pad.insert(PadButton::Start, vec![Enter]);
        pad.insert(PadButton::Select, vec![Shift]);
        KeyMap { pad }
    }
}

/// Mouse buttons as programs name them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    pub fn from_name(s: &str) -> Option<MouseButton> {
        match s.to_ascii_lowercase().as_str() {
            "left" | "1" => Some(MouseButton::Left),
            "right" | "2" => Some(MouseButton::Right),
            "middle" | "3" => Some(MouseButton::Middle),
            _ => None,
        }
    }
}

/// Mouse state as seen by a program (FR-I6). Coordinates are in the
/// program's virtual pixels; the shell and editor never look at this.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MouseState {
    pub x: f32,
    pub y: f32,
    /// True while the pointer is over the virtual screen.
    pub inside: bool,
    pub down: HashSet<MouseButton>,
    pub pressed: HashSet<MouseButton>,
    pub released: HashSet<MouseButton>,
    /// Wheel movement this frame in notches (positive = away from the user).
    pub wheel: f32,
    pub wheel_x: f32,
}

/// Live keyboard and mouse state kept on the main thread and sampled into
/// snapshots.
#[derive(Clone, Debug, Default)]
pub struct InputState {
    pub down: HashSet<KeyCode>,
    pressed: HashSet<KeyCode>,
    released: HashSet<KeyCode>,
    typed: Vec<KeyEvent>,
    mouse: MouseState,
}

impl InputState {
    /// Pointer moved; coordinates already mapped to virtual pixels (may be
    /// outside the screen when over the letterbox).
    pub fn mouse_move(&mut self, x: f32, y: f32, inside: bool) {
        self.mouse.x = x;
        self.mouse.y = y;
        self.mouse.inside = inside;
    }

    pub fn mouse_button(&mut self, b: MouseButton, pressed: bool) {
        if pressed {
            if self.mouse.down.insert(b) {
                self.mouse.pressed.insert(b);
            }
        } else if self.mouse.down.remove(&b) {
            self.mouse.released.insert(b);
        }
    }

    pub fn mouse_wheel(&mut self, dx: f32, dy: f32) {
        self.mouse.wheel += dy;
        self.mouse.wheel_x += dx;
    }

    pub fn key_event(&mut self, ev: &KeyEvent) {
        if ev.pressed {
            if !ev.repeat && self.down.insert(ev.code) {
                self.pressed.insert(ev.code);
            }
            self.typed.push(ev.clone());
        } else {
            if self.down.remove(&ev.code) {
                self.released.insert(ev.code);
            }
        }
    }

    /// Produce the snapshot for a frame and clear the edge sets.
    pub fn snapshot(&mut self, keymap: &KeyMap) -> InputSnapshot {
        let mut buttons = HashSet::new();
        let mut buttons_pressed = HashSet::new();
        let mut buttons_released = HashSet::new();
        for (b, keys) in &keymap.pad {
            if keys.iter().any(|k| self.down.contains(k)) {
                buttons.insert(*b);
            }
            if keys.iter().any(|k| self.pressed.contains(k)) {
                buttons_pressed.insert(*b);
            }
            if keys.iter().any(|k| self.released.contains(k)) {
                buttons_released.insert(*b);
            }
        }
        let mouse = MouseState {
            x: self.mouse.x,
            y: self.mouse.y,
            inside: self.mouse.inside,
            down: self.mouse.down.clone(),
            pressed: std::mem::take(&mut self.mouse.pressed),
            released: std::mem::take(&mut self.mouse.released),
            wheel: std::mem::take(&mut self.mouse.wheel),
            wheel_x: std::mem::take(&mut self.mouse.wheel_x),
        };
        InputSnapshot {
            down: self.down.clone(),
            pressed: std::mem::take(&mut self.pressed),
            released: std::mem::take(&mut self.released),
            typed: std::mem::take(&mut self.typed),
            buttons,
            buttons_pressed,
            buttons_released,
            mouse,
        }
    }

    pub fn clear_all(&mut self) {
        self.down.clear();
        self.pressed.clear();
        self.released.clear();
        self.typed.clear();
        let (x, y, inside) = (self.mouse.x, self.mouse.y, self.mouse.inside);
        self.mouse = MouseState { x, y, inside, ..Default::default() };
    }
}

/// Input as seen by a program for one frame (FR-I4).
#[derive(Clone, Debug, Default)]
pub struct InputSnapshot {
    pub down: HashSet<KeyCode>,
    pub pressed: HashSet<KeyCode>,
    pub released: HashSet<KeyCode>,
    /// Key presses in order, for `read_key()`/`input()`.
    pub typed: Vec<KeyEvent>,
    pub buttons: HashSet<PadButton>,
    pub buttons_pressed: HashSet<PadButton>,
    pub buttons_released: HashSet<PadButton>,
    pub mouse: MouseState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names_roundtrip() {
        for k in ALL_KEYS {
            assert_eq!(KeyCode::from_name(k.name()), Some(*k), "{k:?}");
        }
    }

    #[test]
    fn mouse_edges_and_wheel_accumulate_per_frame() {
        let mut st = InputState::default();
        let km = KeyMap::default();
        st.mouse_move(10.0, 20.0, true);
        st.mouse_button(MouseButton::Left, true);
        st.mouse_wheel(0.0, 1.0);
        st.mouse_wheel(0.0, 2.0);
        let s1 = st.snapshot(&km);
        assert_eq!((s1.mouse.x, s1.mouse.y), (10.0, 20.0));
        assert!(s1.mouse.pressed.contains(&MouseButton::Left) && s1.mouse.down.contains(&MouseButton::Left));
        assert_eq!(s1.mouse.wheel, 3.0);
        let s2 = st.snapshot(&km);
        assert!(!s2.mouse.pressed.contains(&MouseButton::Left) && s2.mouse.down.contains(&MouseButton::Left));
        assert_eq!(s2.mouse.wheel, 0.0);
    }

    #[test]
    fn edges_are_one_frame() {
        let mut st = InputState::default();
        let km = KeyMap::default();
        st.key_event(&KeyEvent::press(KeyCode::Z));
        let s1 = st.snapshot(&km);
        assert!(s1.buttons.contains(&PadButton::A) && s1.buttons_pressed.contains(&PadButton::A));
        let s2 = st.snapshot(&km);
        assert!(s2.buttons.contains(&PadButton::A) && !s2.buttons_pressed.contains(&PadButton::A));
        st.key_event(&KeyEvent { pressed: false, ..KeyEvent::press(KeyCode::Z) });
        let s3 = st.snapshot(&km);
        assert!(!s3.buttons.contains(&PadButton::A) && s3.buttons_released.contains(&PadButton::A));
    }
}
