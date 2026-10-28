//! Lua bindings for the Argon API. Every function registered here must have
//! an entry in [`crate::docs::API_DOCS`]; a test enforces it.

use crate::error::BREAK_MARKER;
use crate::machine::{FlipError, Machine};
use argon_audio::{AudioCmd, ToneParams, Wave};
use argon_core::draw::{BlendMode, DrawCmd, ImageId, Rect, ShaderId, ShaderInputs, SpriteOpts};
use argon_core::input::{KeyCode, MouseButton, PadButton};
use argon_core::shader::{CompiledShader, UniformType, MAX_USER_UNIFORMS};

use argon_core::{Color, CELL_H, CELL_W};
use mlua::{FromLua, Lua, LuaSerdeExt, MultiValue, Table, UserData, UserDataFields, UserDataMethods, Value};
use std::cell::RefCell;
use std::rc::Rc;

pub const EXIT_MARKER: &str = "__argon_exit__";

type M = Rc<RefCell<Machine>>;

fn rt(msg: impl Into<String>) -> mlua::Error {
    mlua::Error::RuntimeError(msg.into())
}

fn break_err() -> mlua::Error {
    rt(BREAK_MARKER)
}

fn flip_result(r: Result<(), FlipError>) -> mlua::Result<()> {
    match r {
        Ok(()) => Ok(()),
        Err(FlipError::Break) => Err(break_err()),
        Err(FlipError::NotSupported) => Err(rt("flip() needs a running program. Save this as a program and use RUN.")),
    }
}

/// Pixel coordinate from a Lua number (floor, like most retro APIs).
fn px(v: f64) -> i32 {
    if v.is_finite() {
        v.floor().clamp(-1e6, 1e6) as i32
    } else {
        0
    }
}

fn col(m: &Machine, v: Option<f64>) -> Color {
    match v {
        Some(c) if c.is_finite() => Color::from_u32(c.max(0.0).min(u32::MAX as f64) as u32),
        _ => m.color,
    }
}

fn color_num(c: Color) -> f64 {
    c.to_u32() as f64
}

/// Image handle exposed to Lua.
#[derive(Clone, Copy)]
#[allow(clippy::too_many_arguments)]
pub struct LuaImage {
    pub id: ImageId,
    pub w: u32,
    pub h: u32,
}

impl UserData for LuaImage {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("width", |_, i| Ok(i.w));
        fields.add_field_method_get("height", |_, i| Ok(i.h));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__tostring", |_, i, ()| Ok(format!("image({}x{})", i.w, i.h)));
    }
}

impl FromLua for LuaImage {
    fn from_lua(v: Value, _lua: &Lua) -> mlua::Result<Self> {
        match &v {
            Value::UserData(ud) => Ok(*ud.borrow::<LuaImage>()?),
            other => Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "image".to_string(),
                message: Some("expected an image from load_image or new_image".to_string()),
            }),
        }
    }
}

/// Sound handle exposed to Lua.
#[derive(Clone, Copy)]
pub struct LuaSound {
    pub id: u32,
    pub duration: f32,
}

impl UserData for LuaSound {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("duration", |_, s| Ok(s.duration));
    }
}

impl FromLua for LuaSound {
    fn from_lua(v: Value, _lua: &Lua) -> mlua::Result<Self> {
        match &v {
            Value::UserData(ud) => Ok(*ud.borrow::<LuaSound>()?),
            other => Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "sound".to_string(),
                message: Some("expected a sound from load_sound".to_string()),
            }),
        }
    }
}

/// Shader handle exposed to Lua.
#[derive(Clone)]
pub struct LuaShader {
    pub id: ShaderId,
    pub compiled: std::sync::Arc<CompiledShader>,
}

impl UserData for LuaShader {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method("__tostring", |_, s, ()| Ok(format!("shader({} uniforms)", s.compiled.uniforms.len())));
    }
}

impl FromLua for LuaShader {
    fn from_lua(v: Value, _lua: &Lua) -> mlua::Result<Self> {
        match &v {
            Value::UserData(ud) => Ok(ud.borrow::<LuaShader>()?.clone()),
            other => Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "shader".to_string(),
                message: Some("expected a shader from load_shader".to_string()),
            }),
        }
    }
}

/// Convert a Lua value into a vec4 uniform slot according to its declared type.
fn uniform_value(v: &Value, ty: UniformType, name: &str) -> mlua::Result<[f32; 4]> {
    let mut out = [0.0f32; 4];
    match v {
        Value::Number(n) => out[0] = *n as f32,
        Value::Integer(i) => out[0] = *i as f32,
        Value::Boolean(b) => out[0] = if *b { 1.0 } else { 0.0 },
        Value::Table(t) => {
            for (i, slot) in out.iter_mut().enumerate() {
                if let Some(x) = t.get::<Option<f64>>(i + 1)? {
                    *slot = x as f32;
                }
            }
        }
        other => return Err(rt(format!("uniform '{name}': expected a number or {{x, y, z, w}} table, got {}", other.type_name()))),
    }
    let _ = ty;
    Ok(out)
}

/// Build the per-draw shader inputs from the opts table.
fn shader_inputs(mm: &Machine, sh: &LuaShader, opts: Option<&Table>) -> mlua::Result<(ShaderInputs, Option<Rect>)> {
    let mut inputs = ShaderInputs {
        time: mm.time() as f32,
        time_delta: mm.dt as f32,
        frame: mm.frame as i32,
        frame_rate: mm.fps as f32,
        mouse: mm.imouse(),
        ..Default::default()
    };
    if let Some(u) = mm.shader_uniforms.get(&sh.id.0) {
        inputs.user = *u;
    }
    let mut rect = None;
    if let Some(t) = opts {
        if let Some(ch) = t.get::<Option<Table>>("channels")? {
            for i in 0..4 {
                if let Some(img) = ch.get::<Option<LuaImage>>(i + 1)? {
                    inputs.channels[i] = Some(img.id);
                }
            }
        }
        if let Some(r) = t.get::<Option<Table>>("rect")? {
            let g = |i: i64| -> mlua::Result<f32> { Ok(r.get::<Option<f64>>(i)?.unwrap_or(0.0) as f32) };
            rect = Some(Rect::new(g(1)?, g(2)?, g(3)?, g(4)?));
        }
        if let Some(u) = t.get::<Option<Table>>("uniforms")? {
            for pair in u.pairs::<String, Value>() {
                let (name, val) = pair?;
                match sh.compiled.uniforms.iter().find(|x| x.name == name) {
                    Some(decl) => inputs.user[decl.slot] = uniform_value(&val, decl.ty, &name)?,
                    None => return Err(rt(format!("shader has no uniform '{name}' (declare it: uniform float {name};)"))),
                }
            }
        }
    }
    Ok((inputs, rect))
}

/// Playing-voice handle.
#[derive(Clone, Copy)]
pub struct LuaVoice(pub u32);

impl UserData for LuaVoice {}

impl FromLua for LuaVoice {
    fn from_lua(v: Value, _lua: &Lua) -> mlua::Result<Self> {
        match &v {
            Value::UserData(ud) => Ok(*ud.borrow::<LuaVoice>()?),
            Value::Integer(i) => Ok(LuaVoice(*i as u32)),
            Value::Number(n) => Ok(LuaVoice(*n as u32)),
            other => Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "voice".to_string(),
                message: Some("expected the value returned by play_sound".to_string()),
            }),
        }
    }
}

fn tone_params(freq: Option<f64>, opts: Option<&Table>) -> mlua::Result<ToneParams> {
    let mut p = ToneParams::default();
    if let Some(f) = freq {
        p.freq = f as f32;
    }
    if let Some(t) = opts {
        if let Some(f) = t.get::<Option<f32>>("freq")? {
            p.freq = f;
        }
        if let Some(w) = t.get::<Option<String>>("wave")? {
            p.wave = Wave::parse(&w).ok_or_else(|| rt(format!("unknown wave '{w}'; use square, triangle, saw, sine or noise")))?;
        }
        if let Some(v) = t.get::<Option<f32>>("volume")? {
            p.volume = v.clamp(0.0, 1.0);
        }
        if let Some(v) = t.get::<Option<f32>>("duty")? {
            p.duty = v.clamp(0.0, 1.0);
        }
        if let Some(v) = t.get::<Option<f32>>("slide")? {
            p.slide = v;
        }
        if let Some(v) = t.get::<Option<f32>>("attack")? {
            p.attack = v.max(0.0);
        }
        if let Some(v) = t.get::<Option<f32>>("release")? {
            p.release = v.max(0.0);
        }
        if let Some(v) = t.get::<Option<f32>>("pan")? {
            p.pan = v.clamp(-1.0, 1.0);
        }
    }
    p.freq = p.freq.clamp(1.0, 20000.0);
    Ok(p)
}

/// "A4" -> 440, "C#5", "Eb3", "A4+" (ignored suffix). Octave defaults to 4.
pub fn note_to_freq(name: &str) -> Option<f32> {
    let s = name.trim();
    let mut chars = s.chars();
    let letter = chars.next()?.to_ascii_uppercase();
    let base = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let (acc, rest) = if let Some(r) = rest.strip_prefix('#') {
        (1, r.to_string())
    } else if let Some(r) = rest.strip_prefix('b') {
        (-1, r.to_string())
    } else {
        (0, rest)
    };
    let octave: i32 = if rest.is_empty() { 4 } else { rest.parse().ok()? };
    let midi = (octave + 1) * 12 + base + acc;
    Some(440.0 * 2f32.powf((midi - 69) as f32 / 12.0))
}

fn mouse_button_arg(name: Option<String>) -> mlua::Result<MouseButton> {
    match name {
        None => Ok(MouseButton::Left),
        Some(n) => MouseButton::from_name(&n).ok_or_else(|| rt(format!("unknown mouse button '{n}'; use left, right or middle"))),
    }
}

fn sprite_opts(opts: Option<Table>) -> mlua::Result<SpriteOpts> {
    let mut o = SpriteOpts::default();
    let Some(t) = opts else { return Ok(o) };
    if let Some(s) = t.get::<Option<f32>>("scale")? {
        o.scale_x = s;
        o.scale_y = s;
    }
    if let Some(s) = t.get::<Option<f32>>("scale_x")? {
        o.scale_x = s;
    }
    if let Some(s) = t.get::<Option<f32>>("scale_y")? {
        o.scale_y = s;
    }
    if let Some(r) = t.get::<Option<f32>>("rotation")? {
        o.rotation = r;
    }
    o.flip_x = t.get::<Option<bool>>("flip_x")?.unwrap_or(false);
    o.flip_y = t.get::<Option<bool>>("flip_y")?.unwrap_or(false);
    if let Some(tint) = t.get::<Option<f64>>("tint")? {
        o.tint = Color::from_u32(tint.max(0.0) as u32);
    }
    if let Some(a) = t.get::<Option<f32>>("alpha")? {
        o.tint.a = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
    }
    if let Some(v) = t.get::<Option<f32>>("origin_x")? {
        o.origin_x = v;
    }
    if let Some(v) = t.get::<Option<f32>>("origin_y")? {
        o.origin_y = v;
    }
    Ok(o)
}

fn points_from_table(t: &Table) -> mlua::Result<Vec<[f32; 2]>> {
    let n = t.raw_len();
    let mut pts = Vec::with_capacity(n / 2);
    let mut i = 1;
    while i < n {
        let x: f64 = t.raw_get(i)?;
        let y: f64 = t.raw_get(i + 1)?;
        pts.push([x as f32, y as f32]);
        i += 2;
    }
    Ok(pts)
}

fn values_to_string(vals: &MultiValue) -> mlua::Result<String> {
    let mut parts = Vec::with_capacity(vals.len());
    for v in vals.iter() {
        parts.push(match v {
            Value::String(s) => s.to_str()?.to_string(),
            Value::Nil => "nil".to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::Number(n) => fmt_number(*n),
            other => other.to_string()?,
        });
    }
    Ok(parts.join("\t"))
}

fn fmt_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Register every API function into `lua`'s globals.
#[allow(clippy::too_many_arguments)]
pub fn install(lua: &Lua, m: &M) -> mlua::Result<Vec<&'static str>> {
    let g = lua.globals();
    let mut names: Vec<&'static str> = Vec::new();

    macro_rules! def {
        ($name:literal, |$mm:ident $(, $arg:ident : $ty:ty)*| $body:block) => {{
            let m2: M = m.clone();
            names.push($name);
            g.set($name, lua.create_function(move |_lua, ($($arg,)*): ($($ty,)*)| {
                #[allow(unused_mut)]
                let mut $mm = m2.borrow_mut();
                $body
            })?)?;
        }};
        ($name:literal, lua |$lua:ident, $mm:ident $(, $arg:ident : $ty:ty)*| $body:block) => {{
            let m2: M = m.clone();
            names.push($name);
            g.set($name, lua.create_function(move |$lua, ($($arg,)*): ($($ty,)*)| {
                #[allow(unused_mut)]
                let mut $mm = m2.borrow_mut();
                $body
            })?)?;
        }};
    }

    // ---- System ----
    def!("flip", |mm| { flip_result(mm.do_flip()) });
    def!("cls", |mm, c: Option<f64>| {
        let color = match c { Some(v) => Color::from_u32(v.max(0.0) as u32), None => Color::BLACK };
        mm.cmds.push(DrawCmd::Clear(color));
        let (fg, bg) = (mm.grid.fg, mm.grid.bg);
        mm.grid.clear_with(fg, bg);
        Ok(())
    });
    def!("wait", |mm, secs: f64| {
        let mut left = secs.max(0.0);
        while left > 0.0 {
            flip_result(mm.do_flip())?;
            left -= mm.dt;
        }
        Ok(())
    });
    def!("time", |mm| { Ok(mm.time()) });
    def!("delta_time", |mm| { Ok(mm.dt) });
    def!("frame", |mm| { Ok(mm.frame) });
    def!("fps", |mm, n: Option<f64>| {
        if let Some(n) = n {
            mm.fps = n.clamp(1.0, 240.0) as u32;
        }
        Ok(mm.fps)
    });
    def!("screen_size", |mm| { Ok((mm.width, mm.height)) });
    def!("set_resolution", |mm, a: Value, b: Option<f64>| {
        let (w, h) = match (&a, b) {
            (Value::String(s), None) => {
                let name = s.to_str()?.to_string();
                argon_core::resolution_preset(&name)
                    .ok_or_else(|| rt(format!("unknown resolution '{name}'; use tiny, retro, classic, hd or WxH")))?
            }
            (Value::Number(w), Some(h)) => (*w as u32, h as u32),
            (Value::Integer(w), Some(h)) => (*w as u32, h as u32),
            _ => return Err(rt("set_resolution(w, h) or set_resolution('preset')")),
        };
        if !(16..=8192).contains(&w) || !(16..=8192).contains(&h) {
            return Err(rt("resolution must be between 16x16 and 8192x8192"));
        }
        mm.set_resolution(w, h);
        Ok(())
    });
    def!("alt_screen", |mm| {
        if mm.immediate {
            return Err(rt("alt_screen() needs a running program."));
        }
        mm.enter_alt_screen();
        Ok(())
    });
    def!("main_screen", |mm| {
        mm.leave_alt_screen();
        Ok(())
    });
    def!("exit", |mm| {
        mm.exit_requested = true;
        Err::<(), _>(rt(EXIT_MARKER))
    });
    def!("rgb", |_mm, r: f64, gg: f64, b: f64, a: Option<f64>| {
        let c = |v: f64| v.clamp(0.0, 255.0).round() as u8;
        Ok(color_num(Color::rgba(c(r), c(gg), c(b), c(a.unwrap_or(255.0)))))
    });
    def!("hsv", |_mm, h: f64, s: f64, v: f64, a: Option<f64>| {
        Ok(color_num(Color::hsv(h as f32, s as f32, v as f32, a.unwrap_or(255.0).clamp(0.0, 255.0).round() as u8)))
    });
    def!("color", |mm, c: Option<f64>| {
        if let Some(c) = c {
            mm.color = Color::from_u32(c.max(0.0) as u32);
        }
        Ok(color_num(mm.color))
    });

    // ---- Math ----
    def!("random", |mm, a: Option<f64>, b: Option<f64>| {
        Ok(match (a, b) {
            (None, _) => mm.rng.next_f64(),
            (Some(n), None) => mm.rng.range(1, n.floor() as i64) as f64,
            (Some(lo), Some(hi)) => mm.rng.range(lo.floor() as i64, hi.floor() as i64) as f64,
        })
    });
    def!("seed", |mm, n: f64| {
        mm.rng = crate::machine::Pcg32::new(n.to_bits());
        Ok(())
    });
    def!("clamp", |_mm, x: f64, lo: f64, hi: f64| { Ok(x.max(lo).min(hi)) });
    def!("lerp", |_mm, a: f64, b: f64, t: f64| { Ok(a + (b - a) * t) });
    def!("round", |_mm, x: f64| { Ok(x.round()) });
    def!("sign", |_mm, x: f64| { Ok(if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 }) });
    def!("dist", |_mm, x1: f64, y1: f64, x2: f64, y2: f64| { Ok(((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()) });
    def!("angle", |_mm, x1: f64, y1: f64, x2: f64, y2: f64| { Ok((y2 - y1).atan2(x2 - x1)) });
    // math.random/randomseed route through our PRNG for determinism.
    {
        let math: Table = g.get("math")?;
        math.set("random", g.get::<Value>("random")?)?;
        math.set("randomseed", g.get::<Value>("seed")?)?;
    }

    // ---- Text ----
    def!("print", |mm, vals: MultiValue| {
        let s = values_to_string(&vals)?;
        mm.grid.println(&s);
        Ok(())
    });
    def!("write", |mm, vals: MultiValue| {
        let s = values_to_string(&vals)?;
        mm.grid.write(&s);
        Ok(())
    });
    def!("locate", |mm, c: f64, r: f64| {
        mm.grid.locate(px(c).max(0) as usize, px(r).max(0) as usize);
        Ok(())
    });
    def!("text_color", |mm, fg: f64, bg: Option<f64>| {
        mm.grid.fg = Color::from_u32(fg.max(0.0) as u32);
        mm.grid.bg = bg.map(|b| Color::from_u32(b.max(0.0) as u32));
        Ok(())
    });
    def!("put_char", |mm, c: f64, r: f64, ch: String, fg: Option<f64>, bg: Option<f64>| {
        let ch = ch.chars().next().unwrap_or(' ');
        let fg = fg.map(|v| Color::from_u32(v.max(0.0) as u32)).unwrap_or(mm.grid.fg);
        let bg = match bg { Some(v) => Some(Color::from_u32(v.max(0.0) as u32)), None => mm.grid.bg };
        mm.grid.put_char_colored(px(c).max(0) as usize, px(r).max(0) as usize, ch, fg, bg);
        Ok(())
    });
    def!("get_char", |mm, c: f64, r: f64| {
        match mm.grid.get(px(c).max(0) as usize, px(r).max(0) as usize) {
            Some(cell) => Ok((Some(cell.ch.to_string()), Some(color_num(cell.fg)), cell.bg.map(color_num))),
            None => Ok((None, None, None)),
        }
    });
    def!("text_size", |mm| { Ok((mm.grid.cols(), mm.grid.rows())) });
    def!("cursor", |mm, v: bool| {
        mm.grid.cursor_visible = v;
        Ok(())
    });
    def!("clear_text", |mm| {
        let (fg, bg) = (mm.grid.fg, mm.grid.bg);
        mm.grid.clear_with(fg, bg);
        Ok(())
    });

    // ---- Input ----
    def!("input", |mm, prompt: Option<String>| {
        if let Some(p) = &prompt {
            mm.grid.write(p);
        }
        let was_visible = mm.grid.cursor_visible;
        mm.grid.cursor_visible = true;
        let mut buf = String::new();
        let result = loop {
            if mm.immediate {
                break Err(rt("input() needs a running program. Save this as a program and use RUN."));
            }
            if let Err(e) = flip_result(mm.do_flip()) {
                break Err(e);
            }
            let mut done = false;
            while let Some(ev) = mm.key_queue.pop_front() {
                match ev.code {
                    KeyCode::Enter | KeyCode::NumpadEnter => {
                        done = true;
                        break;
                    }
                    KeyCode::Backspace => {
                        if buf.pop().is_some() {
                            if mm.grid.cursor_col > 0 {
                                mm.grid.cursor_col -= 1;
                            } else if mm.grid.cursor_row > 0 {
                                mm.grid.cursor_row -= 1;
                                mm.grid.cursor_col = mm.grid.cols() - 1;
                            }
                            let (c, r) = (mm.grid.cursor_col, mm.grid.cursor_row);
                            mm.grid.put_char(c, r, ' ');
                        }
                    }
                    _ => {
                        if let Some(ch) = ev.text {
                            if !ch.is_control() && !ev.mods.ctrl {
                                buf.push(ch);
                                mm.grid.write(&ch.to_string());
                            }
                        }
                    }
                }
            }
            if done {
                mm.grid.newline();
                break Ok(buf.clone());
            }
        };
        mm.grid.cursor_visible = was_visible;
        result
    });
    def!("wait_key", |mm| {
        loop {
            if let Some(ev) = mm.key_queue.pop_front() {
                return Ok(ev.read_key_name());
            }
            if mm.immediate {
                return Err(rt("wait_key() needs a running program. Save this as a program and use RUN."));
            }
            flip_result(mm.do_flip())?;
        }
    });
    def!("read_key", |mm| { Ok(mm.key_queue.pop_front().map(|ev| ev.read_key_name())) });
    def!("button", |mm, name: String, _player: Option<f64>| {
        let b = PadButton::from_name(&name).ok_or_else(|| rt(format!("unknown button '{name}'; use up down left right a b x y l r start select")))?;
        Ok(mm.input.buttons.contains(&b))
    });
    def!("button_pressed", |mm, name: String, _player: Option<f64>| {
        let b = PadButton::from_name(&name).ok_or_else(|| rt(format!("unknown button '{name}'")))?;
        Ok(mm.input.buttons_pressed.contains(&b))
    });
    def!("button_released", |mm, name: String, _player: Option<f64>| {
        let b = PadButton::from_name(&name).ok_or_else(|| rt(format!("unknown button '{name}'")))?;
        Ok(mm.input.buttons_released.contains(&b))
    });
    def!("mouse", |mm| { Ok((mm.input.mouse.x as f64, mm.input.mouse.y as f64, mm.input.mouse.inside)) });
    def!("mouse_button", |mm, name: Option<String>| {
        let b = mouse_button_arg(name)?;
        Ok(mm.input.mouse.down.contains(&b))
    });
    def!("mouse_pressed", |mm, name: Option<String>| {
        let b = mouse_button_arg(name)?;
        Ok(mm.input.mouse.pressed.contains(&b))
    });
    def!("mouse_released", |mm, name: Option<String>| {
        let b = mouse_button_arg(name)?;
        Ok(mm.input.mouse.released.contains(&b))
    });
    def!("mouse_wheel", |mm| { Ok((mm.input.mouse.wheel as f64, mm.input.mouse.wheel_x as f64)) });
    def!("key", |mm, name: String| {
        let k = KeyCode::from_name(&name).ok_or_else(|| rt(format!("unknown key '{name}'")))?;
        Ok(mm.input.down.contains(&k))
    });
    def!("key_pressed", |mm, name: String| {
        let k = KeyCode::from_name(&name).ok_or_else(|| rt(format!("unknown key '{name}'")))?;
        Ok(mm.input.pressed.contains(&k))
    });
    def!("key_released", |mm, name: String| {
        let k = KeyCode::from_name(&name).ok_or_else(|| rt(format!("unknown key '{name}'")))?;
        Ok(mm.input.released.contains(&k))
    });

    // ---- Draw ----
    def!("pixel", |mm, x: f64, y: f64, c: Option<f64>| {
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Pixel { x: px(x), y: px(y), color });
        Ok(())
    });
    def!("line", |mm, x1: f64, y1: f64, x2: f64, y2: f64, c: Option<f64>| {
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Line { x1: px(x1), y1: px(y1), x2: px(x2), y2: px(y2), color });
        Ok(())
    });
    def!("line_width", |mm, w: f64| {
        mm.cmds.push(DrawCmd::SetLineWidth(w.clamp(1.0, 256.0) as u32));
        Ok(())
    });
    def!("rect", |mm, x: f64, y: f64, w: f64, h: f64, c: Option<f64>| {
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Rect { x: px(x), y: px(y), w: px(w), h: px(h), color, fill: false });
        Ok(())
    });
    def!("fill_rect", |mm, x: f64, y: f64, w: f64, h: f64, c: Option<f64>| {
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Rect { x: px(x), y: px(y), w: px(w), h: px(h), color, fill: true });
        Ok(())
    });
    def!("circle", |mm, x: f64, y: f64, r: f64, c: Option<f64>| {
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Circle { x: px(x), y: px(y), r: px(r), color, fill: false });
        Ok(())
    });
    def!("fill_circle", |mm, x: f64, y: f64, r: f64, c: Option<f64>| {
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Circle { x: px(x), y: px(y), r: px(r), color, fill: true });
        Ok(())
    });
    def!("ellipse", |mm, x: f64, y: f64, rx: f64, ry: f64, c: Option<f64>| {
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Ellipse { x: px(x), y: px(y), rx: px(rx), ry: px(ry), color, fill: false });
        Ok(())
    });
    def!("fill_ellipse", |mm, x: f64, y: f64, rx: f64, ry: f64, c: Option<f64>| {
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Ellipse { x: px(x), y: px(y), rx: px(rx), ry: px(ry), color, fill: true });
        Ok(())
    });
    def!("triangle", |mm, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, c: Option<f64>| {
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Triangle { pts: [[x1 as f32, y1 as f32], [x2 as f32, y2 as f32], [x3 as f32, y3 as f32]], color, fill: false });
        Ok(())
    });
    def!("fill_triangle", |mm, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, c: Option<f64>| {
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Triangle { pts: [[x1 as f32, y1 as f32], [x2 as f32, y2 as f32], [x3 as f32, y3 as f32]], color, fill: true });
        Ok(())
    });
    def!("polygon", |mm, pts: Table, c: Option<f64>| {
        let color = col(&mm, c);
        let pts = points_from_table(&pts)?;
        mm.cmds.push(DrawCmd::Polygon { pts, color, fill: false });
        Ok(())
    });
    def!("fill_polygon", |mm, pts: Table, c: Option<f64>| {
        let color = col(&mm, c);
        let pts = points_from_table(&pts)?;
        mm.cmds.push(DrawCmd::Polygon { pts, color, fill: true });
        Ok(())
    });
    def!("draw_text", |mm, x: f64, y: f64, text: MultiValue| {
        // text may be followed by color and scale
        let mut it = text.into_iter();
        let s = match it.next() {
            Some(Value::String(s)) => s.to_str()?.to_string(),
            Some(Value::Nil) | None => String::new(),
            Some(v) => match v {
                Value::Integer(i) => i.to_string(),
                Value::Number(n) => fmt_number(n),
                Value::Boolean(b) => b.to_string(),
                other => other.to_string()?,
            },
        };
        let c = match it.next() { Some(Value::Number(n)) => Some(n), Some(Value::Integer(i)) => Some(i as f64), _ => None };
        let scale = match it.next() { Some(Value::Number(n)) => n.max(1.0) as u32, Some(Value::Integer(i)) => i.max(1) as u32, _ => 1 };
        let color = col(&mm, c);
        mm.cmds.push(DrawCmd::Text { x: px(x), y: px(y), text: s, color, scale });
        Ok(())
    });
    def!("clip", |mm, x: Option<f64>, y: Option<f64>, w: Option<f64>, h: Option<f64>| {
        let r = match (x, y, w, h) {
            (Some(x), Some(y), Some(w), Some(h)) => Some(Rect::new(x as f32, y as f32, w as f32, h as f32)),
            _ => None,
        };
        mm.cmds.push(DrawCmd::SetClip(r));
        Ok(())
    });
    def!("camera", |mm, dx: Option<f64>, dy: Option<f64>| {
        mm.cmds.push(DrawCmd::SetCamera { dx: dx.unwrap_or(0.0) as f32, dy: dy.unwrap_or(0.0) as f32 });
        Ok(())
    });
    def!("blend", |mm, mode: String| {
        let b = BlendMode::parse(&mode).ok_or_else(|| rt(format!("unknown blend mode '{mode}'; use alpha, add, multiply or replace")))?;
        mm.cmds.push(DrawCmd::SetBlend(b));
        Ok(())
    });

    // ---- Image ----
    def!("load_image", |mm, path: String| {
        let bytes = mm.jail.read(&path).map_err(|e| rt(format!("load_image: {e}")))?;
        let img = image::load_from_memory(&bytes).map_err(|e| rt(format!("load_image: {path}: {e}")))?.to_rgba8();
        let (w, h) = img.dimensions();
        let id = mm.alloc_image(w, h);
        mm.cmds.push(DrawCmd::CreateImage { id, width: w, height: h, pixels: img.into_raw() });
        Ok(LuaImage { id, w, h })
    });
    def!("new_image", |mm, w: f64, h: f64, c: Option<f64>| {
        let (w, h) = (px(w).clamp(1, 8192) as u32, px(h).clamp(1, 8192) as u32);
        let color = c.map(|v| Color::from_u32(v.max(0.0) as u32)).unwrap_or(Color::TRANSPARENT);
        let id = mm.alloc_image(w, h);
        let pixels = color.to_bytes().repeat((w * h) as usize);
        mm.cmds.push(DrawCmd::CreateImage { id, width: w, height: h, pixels });
        Ok(LuaImage { id, w, h })
    });
    def!("image_size", |_mm, img: LuaImage| { Ok((img.w, img.h)) });
    def!("draw_image", |mm, img: LuaImage, x: f64, y: f64, opts: Option<Table>| {
        let opts = sprite_opts(opts)?;
        mm.cmds.push(DrawCmd::Sprite { image: img.id, src: Rect::new(0.0, 0.0, img.w as f32, img.h as f32), x: x as f32, y: y as f32, opts });
        Ok(())
    });
    def!("draw_sprite", |mm, img: LuaImage, sx: f64, sy: f64, sw: f64, sh: f64, x: f64, y: f64, opts: Option<Table>| {
        let opts = sprite_opts(opts)?;
        mm.cmds.push(DrawCmd::Sprite { image: img.id, src: Rect::new(sx as f32, sy as f32, sw as f32, sh as f32), x: x as f32, y: y as f32, opts });
        Ok(())
    });
    def!("draw_image_quad", |mm, img: LuaImage, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, x4: f64, y4: f64| {
        mm.cmds.push(DrawCmd::ImageQuad {
            image: img.id,
            pts: [[x1 as f32, y1 as f32], [x2 as f32, y2 as f32], [x3 as f32, y3 as f32], [x4 as f32, y4 as f32]],
            uvs: [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            tint: Color::WHITE,
        });
        Ok(())
    });
    def!("set_target", |mm, img: Option<LuaImage>| {
        mm.cmds.push(DrawCmd::SetTarget(img.map(|i| i.id)));
        Ok(())
    });

    // ---- GPU ----
    def!("load_shader", |mm, src: String| {
        // A path if it has no newline and ends in .glsl/.frag; otherwise source.
        let source = if !src.contains('\n') && (src.ends_with(".glsl") || src.ends_with(".frag") || src.ends_with(".fs")) {
            mm.jail.read_string(&src).map_err(|e| rt(format!("load_shader: {e}")))?
        } else {
            src
        };
        let compiled = argon_core::shader::compile(&source).map_err(|e| rt(format!("load_shader: {e}")))?;
        let id = ShaderId(mm.next_shader);
        mm.next_shader += 1;
        mm.shader_uniforms.insert(id.0, [[0.0; 4]; MAX_USER_UNIFORMS]);
        mm.cmds.push(DrawCmd::CreateShader { id, shader: compiled.clone() });
        Ok(LuaShader { id, compiled })
    });
    def!("draw_shader", |mm, sh: LuaShader, opts: Option<Table>| {
        let (inputs, rect) = shader_inputs(&mm, &sh, opts.as_ref())?;
        mm.cmds.push(DrawCmd::DrawShader { id: sh.id, rect, inputs });
        Ok(())
    });
    def!("set_shader", |mm, sh: Option<LuaShader>, opts: Option<Table>| {
        match sh {
            Some(sh) => {
                let (inputs, _) = shader_inputs(&mm, &sh, opts.as_ref())?;
                mm.cmds.push(DrawCmd::SetShader { id: Some(sh.id), inputs });
            }
            None => mm.cmds.push(DrawCmd::SetShader { id: None, inputs: ShaderInputs::default() }),
        }
        Ok(())
    });
    def!("set_uniform", |mm, sh: LuaShader, name: String, value: Value| {
        let decl = sh
            .compiled
            .uniforms
            .iter()
            .find(|x| x.name == name)
            .ok_or_else(|| rt(format!("shader has no uniform '{name}' (declare it: uniform float {name};)")))?;
        let v = uniform_value(&value, decl.ty, &name)?;
        let slots = mm.shader_uniforms.entry(sh.id.0).or_insert([[0.0; 4]; MAX_USER_UNIFORMS]);
        slots[decl.slot] = v;
        Ok(())
    });
    def!("shader_source", |_mm, name: String| {
        argon_core::shader::builtin_shader(&name)
            .map(|s| s.to_string())
            .ok_or_else(|| rt(format!("no built-in shader '{name}'; try {}", argon_core::shader::BUILTIN_SHADERS.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", "))))
    });

    // ---- Sound ----
    def!("beep", |mm, freq: f64, duration: f64, opts: Option<Table>| {
        let params = tone_params(Some(freq), opts.as_ref())?;
        mm.audio.push(AudioCmd::Beep { params, duration: duration.clamp(0.0, 60.0) as f32 });
        Ok(())
    });
    def!("tone", |mm, channel: f64, opts: Option<Table>| {
        let ch = channel as i64;
        if !(1..=8).contains(&ch) {
            return Err(rt("tone: channel must be 1 to 8"));
        }
        let params = match opts {
            Some(t) => Some(tone_params(None, Some(&t))?),
            None => None,
        };
        mm.audio.push(AudioCmd::Tone { channel: (ch - 1) as u8, params });
        Ok(())
    });
    def!("note_freq", |_mm, name: String| {
        note_to_freq(&name).ok_or_else(|| rt(format!("note_freq: bad note name '{name}' (try 'A4' or 'C#5')")))
    });
    def!("load_sound", |mm, path: String| {
        let bytes = mm.jail.read(&path).map_err(|e| rt(format!("load_sound: {e}")))?;
        let data = argon_audio::decode(&bytes).map_err(|e| rt(format!("load_sound: {path}: {e}")))?;
        let id = mm.next_sound;
        mm.next_sound += 1;
        let duration = data.duration();
        mm.audio.push(AudioCmd::LoadSound { id, data });
        Ok(LuaSound { id, duration })
    });
    def!("play_sound", |mm, snd: LuaSound, opts: Option<Table>| {
        let mut volume = 1.0f32;
        let mut pan = 0.0f32;
        let mut pitch = 1.0f32;
        let mut looping = false;
        if let Some(t) = opts {
            volume = t.get::<Option<f32>>("volume")?.unwrap_or(1.0);
            pan = t.get::<Option<f32>>("pan")?.unwrap_or(0.0);
            pitch = t.get::<Option<f32>>("pitch")?.unwrap_or(1.0);
            looping = t.get::<Option<bool>>("loop")?.unwrap_or(false);
        }
        let voice = mm.next_voice;
        mm.next_voice += 1;
        mm.audio.push(AudioCmd::Play { voice, sound: snd.id, volume, pan, pitch, looping });
        Ok(LuaVoice(voice))
    });
    def!("set_voice", |mm, v: LuaVoice, opts: Table| {
        mm.audio.push(AudioCmd::SetVoice {
            voice: v.0,
            volume: opts.get::<Option<f32>>("volume")?,
            pan: opts.get::<Option<f32>>("pan")?,
            pitch: opts.get::<Option<f32>>("pitch")?,
        });
        Ok(())
    });
    def!("stop_sound", |mm, v: LuaVoice| {
        mm.audio.push(AudioCmd::StopVoice(v.0));
        Ok(())
    });
    def!("stop_all_sounds", |mm| {
        mm.audio.push(AudioCmd::StopAll);
        Ok(())
    });
    def!("set_volume", |mm, v: f64| {
        mm.audio.push(AudioCmd::SetMaster(v.clamp(0.0, 1.0) as f32));
        Ok(())
    });

    // ---- File ----
    def!("read_file", |mm, path: String| {
        match mm.jail.read_string(&path) {
            Ok(s) => Ok((Some(s), None::<String>)),
            Err(e) => Ok((None, Some(e.to_string()))),
        }
    });
    def!("write_file", |mm, path: String, text: String| {
        mm.jail.write(&path, text.as_bytes()).map_err(|e| rt(format!("write_file: {e}")))
    });
    def!("append_file", |mm, path: String, text: String| {
        mm.jail.append(&path, text.as_bytes()).map_err(|e| rt(format!("append_file: {e}")))
    });
    def!("file_exists", |mm, path: String| { Ok(mm.jail.exists(&path)) });
    def!("delete_file", |mm, path: String| {
        mm.jail.delete(&path).map_err(|e| rt(format!("delete_file: {e}")))
    });
    def!("list_files", lua |lua, mm, dir: Option<String>| {
        let entries = mm.jail.list(dir.as_deref().unwrap_or("")).map_err(|e| rt(format!("list_files: {e}")))?;
        let t = lua.create_table()?;
        for (i, e) in entries.iter().enumerate() {
            t.set(i + 1, e.name.clone())?;
        }
        Ok(t)
    });
    def!("save", lua |lua, mm, key: String, value: Value| {
        let json: serde_json::Value = lua.from_value(value)?;
        let mut store = read_store(&mm);
        store.insert(key, json);
        write_store(&mm, &store)
    });
    def!("load", lua |lua, mm, key: String, default: Value| {
        let store = read_store(&mm);
        match store.get(&key) {
            Some(v) => lua.to_value(v),
            None => Ok(default),
        }
    });
    {
        // require: load `name.lua` from the program folder once.
        let m2: M = m.clone();
        names.push("require");
        let cache = lua.create_table()?;
        let cache_key = lua.create_registry_value(cache)?;
        g.set("require", lua.create_function(move |lua, name: String| {
            let cache: Table = lua.registry_value(&cache_key)?;
            if let Some(v) = cache.get::<Option<Value>>(name.as_str())? {
                return Ok(v);
            }
            let clean = name.trim().trim_end_matches(".lua").replace('.', "/");
            let candidates = [format!("{clean}.lua"), format!("{clean}/init.lua")];
            let (path, src) = {
                let mm = m2.borrow();
                let mut found = None;
                for c in &candidates {
                    if let Ok(s) = mm.jail.read_string(c) {
                        found = Some((c.clone(), s));
                        break;
                    }
                }
                found.ok_or_else(|| rt(format!("require: module '{name}' not found (looked for {})", candidates.join(", "))))?
            };
            let chunk_name = format!("={}", path);
            let v: Value = lua.load(&src).set_name(&chunk_name).call(())?;
            let v = if v == Value::Nil { Value::Boolean(true) } else { v };
            cache.set(name.as_str(), v.clone())?;
            Ok(v)
        })?)?;
    }

    Ok(names)
}

fn store_path(m: &Machine) -> String {
    let name = m.main_name.trim_end_matches(".lua").replace('/', "_");
    format!("{name}.json")
}

fn read_store(m: &Machine) -> serde_json::Map<String, serde_json::Value> {
    let Some(j) = &m.save_jail else { return Default::default() };
    j.read_string(&store_path(m))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

fn write_store(m: &Machine, store: &serde_json::Map<String, serde_json::Value>) -> mlua::Result<()> {
    let Some(j) = &m.save_jail else { return Err(rt("save: no save folder available")) };
    let s = serde_json::to_string_pretty(&serde_json::Value::Object(store.clone())).map_err(|e| rt(e.to_string()))?;
    j.write(&store_path(m), s.as_bytes()).map_err(|e| rt(format!("save: {e}")))
}

/// Cell size exposed for `draw_text` layout math.
pub const FONT_CELL: (u32, u32) = (CELL_W, CELL_H);
