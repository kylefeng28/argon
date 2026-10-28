//! A small Lua tokenizer for syntax highlighting, one line at a time.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tok {
    Plain,
    Keyword,
    String,
    Number,
    Comment,
    Builtin,
}

pub const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local", "nil", "not", "or",
    "repeat", "return", "then", "true", "until", "while", "continue", "self",
];

/// Source language for highlighting.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    #[default]
    Lua,
    Glsl,
}

impl Lang {
    pub fn from_path(path: &str) -> Lang {
        if path.to_lowercase().ends_with(".glsl") { Lang::Glsl } else { Lang::Lua }
    }
}

pub const GLSL_KEYWORDS: &[&str] = &[
    "void", "float", "int", "uint", "bool", "vec2", "vec3", "vec4", "ivec2", "ivec3", "ivec4", "bvec2", "bvec3", "bvec4",
    "mat2", "mat3", "mat4", "sampler2D", "uniform", "in", "out", "inout", "const", "if", "else", "for", "while", "do",
    "return", "break", "continue", "discard", "true", "false", "struct", "define", "precision", "highp", "mediump", "lowp",
];

pub const GLSL_BUILTINS: &[&str] = &[
    "mainImage", "iResolution", "iTime", "iTimeDelta", "iFrame", "iFrameRate", "iMouse", "iDate", "iChannel0", "iChannel1",
    "iChannel2", "iChannel3", "iChannelResolution", "fragColor", "fragCoord", "gl_FragCoord", "vUv", "vColor",
    "sin", "cos", "tan", "asin", "acos", "atan", "pow", "exp", "log", "exp2", "log2", "sqrt", "inversesqrt", "abs", "sign",
    "floor", "ceil", "fract", "mod", "min", "max", "clamp", "mix", "step", "smoothstep", "length", "distance", "dot",
    "cross", "normalize", "reflect", "refract", "texture", "textureLod", "texelFetch", "textureSize", "textureTopDown",
    "dFdx", "dFdy", "fwidth", "radians", "degrees", "sinh", "cosh", "tanh", "round", "trunc", "transpose", "inverse",
    "determinant", "any", "all", "not", "lessThan", "greaterThan", "equal",
];

/// State carried between lines: whether we are inside a long comment/string.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LineState {
    #[default]
    Normal,
    /// Inside `--[[ ... ]]` (level = number of `=`).
    LongComment(u8),
    LongString(u8),
}

/// Classify each char of `line` for the given language. `is_builtin` says
/// whether an identifier is an API function (Lua).
pub fn highlight_line_lang(lang: Lang, line: &str, state: LineState, is_builtin: &dyn Fn(&str) -> bool) -> (Vec<Tok>, LineState) {
    match lang {
        Lang::Lua => highlight_line(line, state, is_builtin),
        Lang::Glsl => highlight_glsl_line(line, state),
    }
}

/// GLSL: `//` and `/* */` comments, numbers, keywords, well-known builtins.
pub fn highlight_glsl_line(line: &str, state: LineState) -> (Vec<Tok>, LineState) {
    let chars: Vec<char> = line.chars().collect();
    let mut toks = vec![Tok::Plain; chars.len()];
    let mut i = 0;
    let mut state = state;
    while i < chars.len() {
        if let LineState::LongComment(_) = state {
            // Inside /* ... */
            let mut j = i;
            let mut closed = None;
            while j + 1 < chars.len() {
                if chars[j] == '*' && chars[j + 1] == '/' {
                    closed = Some(j + 2);
                    break;
                }
                j += 1;
            }
            match closed {
                Some(end) => {
                    for t in &mut toks[i..end] {
                        *t = Tok::Comment;
                    }
                    i = end;
                    state = LineState::Normal;
                }
                None => {
                    for t in &mut toks[i..] {
                        *t = Tok::Comment;
                    }
                    return (toks, state);
                }
            }
            continue;
        }
        let c = chars[i];
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            for t in &mut toks[i..] {
                *t = Tok::Comment;
            }
            return (toks, LineState::Normal);
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            state = LineState::LongComment(0);
            toks[i] = Tok::Comment;
            toks[i + 1] = Tok::Comment;
            i += 2;
            continue;
        }
        if c == '#' {
            // Preprocessor line: keyword color for the directive.
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_alphabetic() {
                i += 1;
            }
            for t in &mut toks[start..i] {
                *t = Tok::Keyword;
            }
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.') {
                i += 1;
            }
            for t in &mut toks[start..i] {
                *t = Tok::Number;
            }
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let tok = if GLSL_KEYWORDS.contains(&word.as_str()) {
                Tok::Keyword
            } else if GLSL_BUILTINS.contains(&word.as_str()) {
                Tok::Builtin
            } else {
                Tok::Plain
            };
            for t in &mut toks[start..i] {
                *t = tok;
            }
            continue;
        }
        i += 1;
    }
    (toks, state)
}

/// Classify each char of a Lua `line`. `is_builtin` says whether an identifier is
/// an API function.
pub fn highlight_line(line: &str, state: LineState, is_builtin: &dyn Fn(&str) -> bool) -> (Vec<Tok>, LineState) {
    let chars: Vec<char> = line.chars().collect();
    let mut toks = vec![Tok::Plain; chars.len()];
    let mut i = 0;
    let mut state = state;

    while i < chars.len() {
        match state {
            LineState::LongComment(level) | LineState::LongString(level) => {
                let tok = if matches!(state, LineState::LongComment(_)) { Tok::Comment } else { Tok::String };
                // Look for closing ]=*]
                let mut closed_at = None;
                let mut j = i;
                while j < chars.len() {
                    if chars[j] == ']' && closes_long(&chars, j, level) {
                        closed_at = Some(j + level as usize + 2);
                        break;
                    }
                    j += 1;
                }
                match closed_at {
                    Some(end) => {
                        for t in &mut toks[i..end.min(chars.len())] {
                            *t = tok;
                        }
                        i = end;
                        state = LineState::Normal;
                    }
                    None => {
                        for t in &mut toks[i..] {
                            *t = tok;
                        }
                        return (toks, state);
                    }
                }
            }
            LineState::Normal => {
                let c = chars[i];
                if c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
                    if let Some(level) = opens_long(&chars, i + 2) {
                        state = LineState::LongComment(level);
                        let end = (i + 4 + level as usize).min(chars.len());
                        for t in &mut toks[i..end] {
                            *t = Tok::Comment;
                        }
                        i = end;
                        continue;
                    }
                    for t in &mut toks[i..] {
                        *t = Tok::Comment;
                    }
                    return (toks, LineState::Normal);
                }
                if c == '[' {
                    if let Some(level) = opens_long(&chars, i) {
                        state = LineState::LongString(level);
                        let end = (i + 2 + level as usize).min(chars.len());
                        for t in &mut toks[i..end] {
                            *t = Tok::String;
                        }
                        i = end;
                        continue;
                    }
                }
                if c == '"' || c == '\'' {
                    let start = i;
                    i += 1;
                    while i < chars.len() && chars[i] != c {
                        if chars[i] == '\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    i = (i + 1).min(chars.len());
                    for t in &mut toks[start..i] {
                        *t = Tok::String;
                    }
                    continue;
                }
                if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
                    let start = i;
                    while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                        i += 1;
                    }
                    for t in &mut toks[start..i] {
                        *t = Tok::Number;
                    }
                    continue;
                }
                if c.is_alphabetic() || c == '_' {
                    let start = i;
                    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                        i += 1;
                    }
                    let word: String = chars[start..i].iter().collect();
                    let preceded_by_dot = start > 0 && (chars[start - 1] == '.' || chars[start - 1] == ':');
                    let tok = if KEYWORDS.contains(&word.as_str()) {
                        Tok::Keyword
                    } else if !preceded_by_dot && is_builtin(&word) {
                        Tok::Builtin
                    } else {
                        Tok::Plain
                    };
                    for t in &mut toks[start..i] {
                        *t = tok;
                    }
                    continue;
                }
                i += 1;
            }
        }
    }
    (toks, state)
}

/// If `[` at `i` starts a long bracket, return its level.
fn opens_long(chars: &[char], i: usize) -> Option<u8> {
    if i >= chars.len() || chars[i] != '[' {
        return None;
    }
    let mut j = i + 1;
    let mut level = 0u8;
    while j < chars.len() && chars[j] == '=' {
        level = level.saturating_add(1);
        j += 1;
    }
    if j < chars.len() && chars[j] == '[' {
        Some(level)
    } else {
        None
    }
}

fn closes_long(chars: &[char], i: usize, level: u8) -> bool {
    let mut j = i + 1;
    for _ in 0..level {
        if j >= chars.len() || chars[j] != '=' {
            return false;
        }
        j += 1;
    }
    j < chars.len() && chars[j] == ']'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_tokens() {
        let (t, st) = highlight_line("local x = fill_rect(1, 'a') -- hi", LineState::Normal, &|w| w == "fill_rect");
        assert_eq!(st, LineState::Normal);
        assert_eq!(t[0], Tok::Keyword);
        assert_eq!(t[10], Tok::Builtin);
        assert_eq!(t[20], Tok::Number);
        assert_eq!(t[23], Tok::String);
        assert_eq!(t[28], Tok::Comment);
        assert_eq!(t[t.len() - 1], Tok::Comment);
    }

    #[test]
    fn glsl_tokens() {
        let (t, st) = highlight_glsl_line("vec2 uv = fragCoord / iResolution.xy; // hi", LineState::Normal);
        assert_eq!(st, LineState::Normal);
        assert_eq!(t[0], Tok::Keyword);
        assert_eq!(t[10], Tok::Builtin);
        assert_eq!(t[t.len() - 1], Tok::Comment);
        let (_, st) = highlight_glsl_line("/* open", LineState::Normal);
        assert_eq!(st, LineState::LongComment(0));
        let (t, st) = highlight_glsl_line("still */ float x = 1.0;", st);
        assert_eq!(st, LineState::Normal);
        assert_eq!(t[0], Tok::Comment);
        assert_eq!(t[9], Tok::Keyword);
    }

    #[test]
    fn long_comment_spans_lines() {
        let (_, st) = highlight_line("--[[ start", LineState::Normal, &|_| false);
        assert_eq!(st, LineState::LongComment(0));
        let (t, st) = highlight_line("still ]] x", st, &|_| false);
        assert_eq!(st, LineState::Normal);
        assert_eq!(t[0], Tok::Comment);
        assert_eq!(t[9], Tok::Plain);
    }
}
