//! Normalized, human-readable errors (FR-R8).

use argon_core::api::ApiTable;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgonError {
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
    pub hint: Option<String>,
    pub source_line: Option<String>,
    /// True when the user pressed Esc (not a real error).
    pub is_break: bool,
}

pub const BREAK_MARKER: &str = "__argon_break__";

impl ArgonError {
    pub fn simple(message: impl Into<String>) -> ArgonError {
        ArgonError { file: None, line: None, message: message.into(), hint: None, source_line: None, is_break: false }
    }

    pub fn user_break() -> ArgonError {
        ArgonError { message: "Break".to_string(), is_break: true, ..ArgonError::simple("") }
    }

    /// Parse an mlua error into file/line/message and attach hints.
    pub fn from_lua(err: &mlua::Error, api: &ApiTable, source_lookup: impl Fn(&str, u32) -> Option<String>) -> ArgonError {
        let raw = match err {
            mlua::Error::RuntimeError(s) | mlua::Error::SyntaxError { message: s, .. } => s.clone(),
            mlua::Error::MemoryError(s) => format!("Program exceeded memory limit ({s})"),
            mlua::Error::CallbackError { traceback: _, cause } => {
                return ArgonError::from_lua(cause, api, source_lookup);
            }
            mlua::Error::ExternalError(e) => e.to_string(),
            mlua::Error::WithContext { context, cause } => {
                let mut inner = ArgonError::from_lua(cause, api, source_lookup);
                if !inner.is_break {
                    inner.message = format!("{context}: {}", inner.message);
                }
                return inner;
            }
            other => other.to_string(),
        };
        if raw.contains(BREAK_MARKER) {
            return ArgonError::user_break();
        }
        let mut e = parse_location(&raw);
        // Drop Luau's traceback noise after the first line.
        if let Some(i) = e.message.find("\nstack traceback") {
            e.message.truncate(i);
        }
        if let (Some(f), Some(l)) = (&e.file, e.line) {
            e.source_line = source_lookup(f, l);
        }
        e.hint = hint_for(&e.message, e.source_line.as_deref(), api);
        e
    }

    /// Multi-line rendering for the shell.
    pub fn render(&self) -> String {
        if self.is_break {
            return "Break".to_string();
        }
        let mut out = String::from("ERROR");
        if let Some(f) = &self.file {
            out.push_str(&format!(" in {f}"));
            if let Some(l) = self.line {
                out.push_str(&format!(":{l}"));
            }
        }
        out.push_str(": ");
        out.push_str(&self.message);
        if let Some(h) = &self.hint {
            out.push_str(&format!("\n  {h}"));
        }
        if let (Some(l), Some(src)) = (self.line, &self.source_line) {
            out.push_str(&format!("\n  {l:>4} | {}", src.trim_end()));
        }
        out
    }

    /// One-line rendering for the editor status bar.
    pub fn one_line(&self) -> String {
        if self.is_break {
            return "Break".to_string();
        }
        let mut s = match (self.file.as_deref(), self.line) {
            (Some(f), Some(l)) => format!("{f}:{l}: "),
            _ => String::new(),
        };
        match &self.hint {
            Some(h) => s.push_str(&format!("{h} ({})", self.message)),
            None => s.push_str(&self.message),
        }
        s
    }
}

/// Split `file:line: message` (Luau format with chunk name `=file`).
fn parse_location(raw: &str) -> ArgonError {
    let first = raw.lines().next().unwrap_or(raw);
    // Strip `[string "name"]:` form into `name:`.
    let cleaned = if let Some(rest) = first.strip_prefix("[string \"") {
        if let Some(i) = rest.find("\"]") {
            format!("{}{}", &rest[..i], &rest[i + 2..])
        } else {
            first.to_string()
        }
    } else {
        first.to_string()
    };
    let mut parts = cleaned.splitn(3, ':');
    let a = parts.next().unwrap_or("");
    let b = parts.next();
    let c = parts.next();
    if let (Some(b), Some(c)) = (b, c) {
        if let Ok(line) = b.trim().parse::<u32>() {
            let mut msg = c.trim().to_string();
            // Keep any continuation lines (e.g. multi-line syntax errors).
            for l in raw.lines().skip(1) {
                if l.starts_with("stack traceback") {
                    break;
                }
                msg.push('\n');
                msg.push_str(l);
            }
            return ArgonError { file: Some(a.trim().to_string()), line: Some(line), message: msg, hint: None, source_line: None, is_break: false };
        }
    }
    ArgonError::simple(raw.trim().to_string())
}

fn hint_for(message: &str, source_line: Option<&str>, api: &ApiTable) -> Option<String> {
    // PUC-style: attempt to call a nil value (global 'fil_rect')
    if let Some(i) = message.find("global '") {
        let rest = &message[i + 8..];
        let name = rest.split('\'').next()?;
        if let Some(s) = api.suggest(name) {
            if s != name {
                return Some(format!("Did you mean {s}?"));
            }
        }
        return Some(format!("'{name}' is not defined. Type HELP for the list of functions."));
    }
    // Luau does not name the variable; look at the source line for calls to
    // names that are not API functions but are close to one.
    if message.contains("attempt to call a nil value") {
        let line = source_line?;
        for name in called_identifiers(line) {
            if api.get(&name).is_some() || LUA_KEYWORDS.contains(&name.as_str()) {
                continue;
            }
            if let Some(s) = api.suggest(&name) {
                return Some(format!("Did you mean {s}?"));
            }
        }
    }
    None
}

const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local", "nil", "not", "or",
    "repeat", "return", "then", "true", "until", "while", "continue",
];

/// Identifiers directly followed by `(` that are not fields or methods.
fn called_identifiers(line: &str) -> Vec<String> {
    let bytes: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_alphabetic() || bytes[i] == '_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_alphanumeric() || bytes[i] == '_') {
                i += 1;
            }
            let name: String = bytes[start..i].iter().collect();
            let mut j = i;
            while j < bytes.len() && bytes[j] == ' ' {
                j += 1;
            }
            let preceded_by_field = start > 0 && (bytes[start - 1] == '.' || bytes[start - 1] == ':');
            if j < bytes.len() && bytes[j] == '(' && !preceded_by_field {
                out.push(name);
            }
        } else if bytes[i] == '"' || bytes[i] == '\'' {
            // Skip string literals.
            let q = bytes[i];
            i += 1;
            while i < bytes.len() && bytes[i] != q {
                if bytes[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
        } else if bytes[i] == '-' && i + 1 < bytes.len() && bytes[i + 1] == '-' {
            break;
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_luau_location() {
        let e = parse_location("main.lua:12: attempt to call a nil value (global 'fil_rect')");
        assert_eq!(e.file.as_deref(), Some("main.lua"));
        assert_eq!(e.line, Some(12));
        assert!(e.message.starts_with("attempt to call"));
        let e2 = parse_location("[string \"main.lua\"]:3: boom");
        assert_eq!(e2.file.as_deref(), Some("main.lua"));
        assert_eq!(e2.line, Some(3));
        assert_eq!(e2.message, "boom");
    }
}
