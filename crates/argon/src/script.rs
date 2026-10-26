//! Keystroke scripts for the headless runner and CI.
//!
//! Plain characters are typed. Braces hold special keys and directives:
//! `{enter}` `{esc}` `{f5}` `{tab}` `{up}` `{ctrl+s}` `{shift+right}`
//! `{wait:30}` (frames) `{hold:z}` `{release:z}`.

use argon_core::input::{KeyCode, KeyEvent, Modifiers};

#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Key(KeyEvent),
    Wait(u32),
    /// Move the pointer to virtual pixel coordinates.
    MouseMove(f32, f32),
    MouseButton(argon_core::input::MouseButton, bool),
    Wheel(f32),
}

pub fn parse(script: &str) -> Result<Vec<Step>, String> {
    let mut steps = Vec::new();
    let mut chars = script.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut tok = String::new();
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some(ch) => tok.push(ch),
                    None => return Err("unterminated '{' in key script".into()),
                }
            }
            steps.extend(parse_token(&tok)?);
        } else if c == '\n' {
            steps.push(Step::Key(KeyEvent::press(KeyCode::Enter)));
            steps.push(Step::Key(release(KeyCode::Enter)));
        } else {
            let ev = KeyEvent::typed(c);
            steps.push(Step::Key(ev.clone()));
            steps.push(Step::Key(KeyEvent { pressed: false, text: None, ..ev }));
        }
    }
    Ok(steps)
}

fn release(code: KeyCode) -> KeyEvent {
    KeyEvent { pressed: false, ..KeyEvent::press(code) }
}

fn parse_token(tok: &str) -> Result<Vec<Step>, String> {
    let t = tok.trim().to_ascii_lowercase();
    if let Some(n) = t.strip_prefix("wait:") {
        return Ok(vec![Step::Wait(n.trim().parse().map_err(|_| format!("bad wait '{tok}'"))?)]);
    }
    if let Some(xy) = t.strip_prefix("mouse:") {
        let (x, y) = xy.split_once(',').ok_or_else(|| format!("bad mouse position '{tok}'"))?;
        let x: f32 = x.trim().parse().map_err(|_| format!("bad mouse x '{tok}'"))?;
        let y: f32 = y.trim().parse().map_err(|_| format!("bad mouse y '{tok}'"))?;
        return Ok(vec![Step::MouseMove(x, y)]);
    }
    if let Some(b) = t.strip_prefix("click:") {
        let b = argon_core::input::MouseButton::from_name(b.trim()).ok_or_else(|| format!("bad mouse button '{tok}'"))?;
        return Ok(vec![Step::MouseButton(b, true), Step::MouseButton(b, false)]);
    }
    if let Some(b) = t.strip_prefix("mousedown:") {
        let b = argon_core::input::MouseButton::from_name(b.trim()).ok_or_else(|| format!("bad mouse button '{tok}'"))?;
        return Ok(vec![Step::MouseButton(b, true)]);
    }
    if let Some(b) = t.strip_prefix("mouseup:") {
        let b = argon_core::input::MouseButton::from_name(b.trim()).ok_or_else(|| format!("bad mouse button '{tok}'"))?;
        return Ok(vec![Step::MouseButton(b, false)]);
    }
    if let Some(n) = t.strip_prefix("wheel:") {
        return Ok(vec![Step::Wheel(n.trim().parse().map_err(|_| format!("bad wheel '{tok}'"))?)]);
    }
    if let Some(k) = t.strip_prefix("hold:") {
        let code = KeyCode::from_name(k.trim()).ok_or_else(|| format!("unknown key '{k}'"))?;
        return Ok(vec![Step::Key(KeyEvent::press(code))]);
    }
    if let Some(k) = t.strip_prefix("release:") {
        let code = KeyCode::from_name(k.trim()).ok_or_else(|| format!("unknown key '{k}'"))?;
        return Ok(vec![Step::Key(release(code))]);
    }
    let mut mods = Modifiers::default();
    let mut key = None;
    for part in t.split('+') {
        match part {
            "ctrl" => mods.ctrl = true,
            "shift" => mods.shift = true,
            "alt" => mods.alt = true,
            other => key = Some(KeyCode::from_name(other).ok_or_else(|| format!("unknown key '{other}' in {{{tok}}}"))?),
        }
    }
    let code = key.ok_or_else(|| format!("no key in {{{tok}}}"))?;
    // Letters with modifiers still carry text (so Ctrl+S has 's').
    let text = match code.name() {
        n if n.chars().count() == 1 && !mods.ctrl && !mods.alt => {
            let ch = n.chars().next().unwrap();
            Some(if mods.shift { ch.to_ascii_uppercase() } else { ch })
        }
        n if n.chars().count() == 1 => n.chars().next(),
        _ => None,
    };
    let text = if code == KeyCode::Space { Some(' ') } else { text };
    Ok(vec![
        Step::Key(KeyEvent { code, text, mods, pressed: true, repeat: false }),
        Step::Key(KeyEvent { code, text: None, mods, pressed: false, repeat: false }),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mixed_script() {
        let s = parse("ab{enter}{wait:3}{ctrl+s}{hold:z}").unwrap();
        assert_eq!(s.len(), 2 + 2 + 2 + 1 + 2 + 1);
        assert!(matches!(&s[0], Step::Key(k) if k.text == Some('a') && k.pressed));
        assert!(matches!(&s[6], Step::Wait(3)));
        assert!(matches!(&s[7], Step::Key(k) if k.code == KeyCode::S && k.mods.ctrl));
        assert!(matches!(&s[9], Step::Key(k) if k.code == KeyCode::Z && k.pressed));
    }
}
