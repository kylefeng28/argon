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
    Ver,
    Exit,

    Theme(Option<String>),
    Filter(Option<String>),
    Font(Option<String>),

    Cd(Option<String>),
    Dir(Option<String>),
    Type(Option<String>),
    Run(Option<String>),
    Lua(Option<String>),

    New(Option<String>),
    Edit(Option<String>),

    Mkdir(Option<String>),
    Copy(Option<String>, Option<String>),
    Ren(Option<String>, Option<String>),
    Del(Option<String>),
    Export(Option<String>),
    Import(Option<String>),
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
    let two = || -> (Option<String>, Option<String>) {
        match rest {
            None => (None, None),
            Some(r) => {
                let mut p = r.splitn(2, char::is_whitespace);
                let a = p.next().map(|s| s.to_string());
                let b = p.next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
                (a, b)
            }
        }
    };

    match word.to_ascii_uppercase().as_str() {
        "CLS" => ShellCmd::Cls,
        "ECHO" => ShellCmd::Echo(one()),
        "HELP" => ShellCmd::Help(one()),
        "VER" | "VERSION" => ShellCmd::Ver,
        "EXIT" => ShellCmd::Exit,

        "THEME" => ShellCmd::Theme(one()),
        "FILTER" => ShellCmd::Filter(one()),
        "FONT" => ShellCmd::Font(one()),

        "CD" => ShellCmd::Cd(one()),
        "DIR" | "LS" => ShellCmd::Dir(one()),
        "TYPE" | "CAT" => ShellCmd::Type(one()),
        "RUN" => ShellCmd::Run(one()),
        "LUA" | "EVAL" => ShellCmd::Lua(one()),

        "NEW" => ShellCmd::New(one()),
        "EDIT" => ShellCmd::Edit(one()),

        "MKDIR" => ShellCmd::Mkdir(one()),
        "COPY" | "CP" => {
            let (a, b) = two();
            ShellCmd::Copy(a, b)
        }
        "REN" | "RENAME" | "MV" => {
            let (a, b) = two();
            ShellCmd::Ren(a, b)
        }
        "DEL" | "RM" | "DELETE" => ShellCmd::Del(one()),

        "EXPORT" => ShellCmd::Export(one()),
        "IMPORT" => ShellCmd::Import(one()),

        _ => ShellCmd::Unknown(word.to_string()),
    }
}

/// DOS-style wildcard match (`*` and `?`), case-insensitive.
pub fn wildcard_match(pattern: &str, name: &str) -> bool {
    fn rec(p: &[char], n: &[char]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some('*'), _) => rec(&p[1..], n) || (!n.is_empty() && rec(p, &n[1..])),
            (Some('?'), Some(_)) => rec(&p[1..], &n[1..]),
            (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => rec(&p[1..], &n[1..]),
            _ => false,
        }
    }
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    rec(&p, &n)
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

/// Template for `NEW name`.
pub fn new_program_template(name: &str) -> String {
    format!(
        "-- {name}\n-- Press F5 to run, Esc to stop, F1 for the catalog.\n\ncls()\nprint(\"Hello from {name}!\")\n\nwhile true do\n  fill_circle(320 + 100 * math.sin(time()), 200, 40, rgb(255, 200, 0))\n  flip()\nend\n"
    )
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

    #[test]
    fn wildcards() {
        assert!(wildcard_match("*.lua", "pong.lua"));
        assert!(wildcard_match("P?NG*", "pong.lua"));
        assert!(!wildcard_match("*.png", "pong.lua"));
        assert!(wildcard_match("*", "anything"));
    }
}
