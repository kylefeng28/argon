//! Shell command parsing and formatting helpers. Execution lives in the
//! console, which owns the disk, the grid and the Lua state.

use argon_core::vfs::DirEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellCmd {
    NoOp,
    Unknown(String),
    Cls,
    Echo(Option<String>),
    Help(Option<String>),
    Exit,

    Dir(Option<String>),
    Run(Option<String>),
    Lua(Option<String>),
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
        "HELP" => ShellCmd::Help(one()),
        "EXIT" => ShellCmd::Exit,

        "RUN" => ShellCmd::Run(one()),
        "DIR" | "LS" => ShellCmd::Dir(one()),
        "LUA" | "EVAL" => ShellCmd::Lua(one()),

        _ => ShellCmd::Unknown(word.to_string()),
    }
}

/// Format a directory listing.
pub fn format_dir(entries: &[DirEntry], cols: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut files = 0;
    let mut bytes = 0u64;
    for e in entries {
        if e.is_dir {
            let tag = if e.is_cart { "<CART>" } else { "<DIR> " };
            out.push(format!("  {tag} {}", e.name));
        }
    }
    let mut names: Vec<String> = Vec::new();
    for e in entries {
        if !e.is_dir {
            files += 1;
            bytes += e.size;
            names.push(format!("{:<20} {:>8}", truncate_name(&e.name, 20), human_size(e.size)));
        }
    }
    let per_row = (cols / 32).max(1);
    for chunk in names.chunks(per_row) {
        out.push(format!("  {}", chunk.join("  ")));
    }
    out.push(format!("  {} file(s), {}", files, human_size(bytes)));
    out
}

fn truncate_name(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n - 1).collect();
        t.push('~');
        t
    }
}

pub fn human_size(b: u64) -> String {
    if b < 10_000 {
        format!("{b} B")
    } else if b < 10_000_000 {
        format!("{} KB", b / 1024)
    } else {
        format!("{} MB", b / (1024 * 1024))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commands_and_lua() {
        assert_eq!(parse("dir"), ShellCmd::Dir(None));
        assert_eq!(parse("RUN pong"), ShellCmd::Run(Some("pong".into())));
        assert_eq!(parse("LUA 2 + 2"), ShellCmd::Lua(Some("2 + 2".into())));
        assert_eq!(parse("EVAL 2 + 2"), ShellCmd::Lua(Some("2 + 2".into())));
        assert_eq!(parse("LUA print('dir')"), ShellCmd::Lua(Some("print('dir')".into())));
        assert_eq!(parse("LUA x = 1"), ShellCmd::Lua(Some("x = 1".into())));
    }
}
