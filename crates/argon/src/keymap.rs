//! winit key events -> Argon key events.

use argon_core::input::{KeyCode, KeyEvent, Modifiers};
use winit::keyboard::{KeyCode as WK, PhysicalKey};

pub fn map_physical(key: PhysicalKey) -> KeyCode {
    let PhysicalKey::Code(k) = key else { return KeyCode::Unknown };
    use KeyCode::*;
    match k {
        WK::KeyA => A, WK::KeyB => B, WK::KeyC => C, WK::KeyD => D, WK::KeyE => E, WK::KeyF => F,
        WK::KeyG => G, WK::KeyH => H, WK::KeyI => I, WK::KeyJ => J, WK::KeyK => K, WK::KeyL => L,
        WK::KeyM => M, WK::KeyN => N, WK::KeyO => O, WK::KeyP => P, WK::KeyQ => Q, WK::KeyR => R,
        WK::KeyS => S, WK::KeyT => T, WK::KeyU => U, WK::KeyV => V, WK::KeyW => W, WK::KeyX => X,
        WK::KeyY => Y, WK::KeyZ => Z,
        WK::Digit0 => Digit0, WK::Digit1 => Digit1, WK::Digit2 => Digit2, WK::Digit3 => Digit3,
        WK::Digit4 => Digit4, WK::Digit5 => Digit5, WK::Digit6 => Digit6, WK::Digit7 => Digit7,
        WK::Digit8 => Digit8, WK::Digit9 => Digit9,
        WK::F1 => F1, WK::F2 => F2, WK::F3 => F3, WK::F4 => F4, WK::F5 => F5, WK::F6 => F6,
        WK::F7 => F7, WK::F8 => F8, WK::F9 => F9, WK::F10 => F10, WK::F11 => F11, WK::F12 => F12,
        WK::ArrowUp => Up, WK::ArrowDown => Down, WK::ArrowLeft => Left, WK::ArrowRight => Right,
        WK::Space => Space, WK::Enter => Enter, WK::Escape => Escape, WK::Backspace => Backspace,
        WK::Tab => Tab, WK::Delete => Delete, WK::Insert => Insert, WK::Home => Home, WK::End => End,
        WK::PageUp => PageUp, WK::PageDown => PageDown,
        WK::ShiftLeft | WK::ShiftRight => Shift,
        WK::ControlLeft | WK::ControlRight => Ctrl,
        WK::AltLeft | WK::AltRight => Alt,
        WK::SuperLeft | WK::SuperRight => Super,
        WK::CapsLock => CapsLock,
        WK::Minus => Minus, WK::Equal => Equal, WK::BracketLeft => BracketLeft, WK::BracketRight => BracketRight,
        WK::Backslash => Backslash, WK::Semicolon => Semicolon, WK::Quote => Quote, WK::Comma => Comma,
        WK::Period => Period, WK::Slash => Slash, WK::Backquote => Backquote,
        WK::Numpad0 => Numpad0, WK::Numpad1 => Numpad1, WK::Numpad2 => Numpad2, WK::Numpad3 => Numpad3,
        WK::Numpad4 => Numpad4, WK::Numpad5 => Numpad5, WK::Numpad6 => Numpad6, WK::Numpad7 => Numpad7,
        WK::Numpad8 => Numpad8, WK::Numpad9 => Numpad9, WK::NumpadAdd => NumpadAdd,
        WK::NumpadSubtract => NumpadSubtract, WK::NumpadMultiply => NumpadMultiply,
        WK::NumpadDivide => NumpadDivide, WK::NumpadEnter => NumpadEnter, WK::NumpadDecimal => NumpadDecimal,
        _ => Unknown,
    }
}

pub fn map_event(ev: &winit::event::KeyEvent, mods: winit::keyboard::ModifiersState) -> KeyEvent {
    let code = map_physical(ev.physical_key);
    let text = ev.text.as_ref().and_then(|t| t.chars().next()).filter(|c| !c.is_control());
    KeyEvent {
        code,
        text,
        mods: Modifiers {
            shift: mods.shift_key(),
            ctrl: mods.control_key(),
            alt: mods.alt_key(),
            super_key: mods.super_key(),
        },
        pressed: ev.state.is_pressed(),
        repeat: ev.repeat,
    }
}
