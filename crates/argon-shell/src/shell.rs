//! Shell command parsing and formatting helpers. Execution lives in the
//! console, which owns the disk, the grid and the Lua state.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellCmd {
    NoOp,
    Unknown(String),
    Cls,
    Echo(Option<String>),
    Help,
    Exit,
}

/// Parse a line. Commands are case-insensitive.
pub fn parse(line: &str) -> ShellCmd {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ShellCmd::NoOp;
    }
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let word = parts.next().unwrap_or("");
    let rest = parts.next().map(|s| s.trim()).filter(|s| !s.is_empty());
    let one = || rest.map(|s| s.to_string());

    match word.to_ascii_uppercase().as_str() {
        "CLS" => ShellCmd::Cls,
        "ECHO" => ShellCmd::Echo(one()),
        "HELP" => ShellCmd::Help,
        "EXIT" => ShellCmd::Exit,
        _ => ShellCmd::Unknown(word.to_string()),
    }
}
