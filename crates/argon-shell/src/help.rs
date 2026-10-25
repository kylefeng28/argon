//! HELP text for shell commands and API lookups.

use argon_core::api::{ApiTable, Category};

pub struct CommandDoc {
    pub name: &'static str,
    pub usage: &'static str,
    pub doc: &'static str,
}

pub static COMMANDS: &[CommandDoc] = &[
    CommandDoc { name: "CLS", usage: "CLS", doc: "Clear the screen. Alias: CLEAR." },
    CommandDoc { name: "ECHO", usage: "ECHO [message]", doc: "Prints a message to the screen." },
    CommandDoc { name: "VARCLEAR", usage: "VARCLEAR", doc: "Forget all variables and functions defined at the prompt." },

    CommandDoc { name: "HELP", usage: "HELP [topic]", doc: "Help on a command, a function or a category (e.g. HELP draw)." },
    CommandDoc { name: "VER", usage: "VER", doc: "Show the Argon version." },
    CommandDoc { name: "EXIT", usage: "EXIT", doc: "Quit Argon." },

    CommandDoc { name: "CD", usage: "CD folder | CD .. | CD", doc: "Change folder. CD alone returns to the root." },
    CommandDoc { name: "DIR", usage: "DIR [pattern]", doc: "List programs and files in the current folder. Alias: LS." },
    CommandDoc { name: "TYPE", usage: "TYPE name", doc: "Print a file to the screen. Alias: CAT" },
    CommandDoc { name: "RUN", usage: "RUN [name]", doc: "Run a program. With no name, re-runs the last one. Esc stops it." },
    CommandDoc { name: "LUA", usage: "LUA [name]", doc: "Evaluate a lua expression or command. Alias: EVAL." },

    CommandDoc { name: "NEW", usage: "NEW name", doc: "Create a new program from a template and open it in the editor." },
    CommandDoc { name: "EDIT", usage: "EDIT [name]", doc: "Open a program in the editor (creates it if missing)." },

    CommandDoc { name: "MKDIR", usage: "MKDIR name", doc: "Create a folder." },
    CommandDoc { name: "COPY", usage: "COPY from to", doc: "Copy a file or folder. Alias: CP" },
    CommandDoc { name: "REN", usage: "REN from to", doc: "Rename a file or folder. Alias: MV" },
    CommandDoc { name: "DEL", usage: "DEL name", doc: "Delete a file or folder. Alias: RM" },
];

pub fn command_doc(name: &str) -> Option<&'static CommandDoc> {
    let n = name.to_ascii_uppercase();
    let n = match n.as_str() { "LS" => "DIR".to_string(), "CLEAR" => "CLS".to_string(), _ => n };
    COMMANDS.iter().find(|c| c.name == n)
}

/// Lines to print for `HELP [topic]`.
pub fn help_lines(topic: Option<&str>, api: &ApiTable, cols: usize) -> Vec<String> {
    let mut out = Vec::new();
    match topic {
        None => {
            out.push("Shell commands:".to_string());
            let names: Vec<&str> = COMMANDS.iter().map(|c| c.name).collect();
            out.extend(columns(&names, cols));
            out.push(String::new());
            out.push("Function categories (HELP <category>):".to_string());
            let cats: Vec<String> = Category::ALL.iter().map(|c| c.name().to_lowercase()).collect();
            let refs: Vec<&str> = cats.iter().map(|s| s.as_str()).collect();
            out.extend(columns(&refs, cols));
            out.push(String::new());
            out.push("Type any Lua at the prompt: 2^10, print('hi'), fill_circle(320, 200, 50)".to_string());
            out.push("HELP <name> explains a command or function. In the editor, F1 is the catalog.".to_string());
        }
        Some(t) => {
            if let Some(c) = command_doc(t) {
                out.push(c.usage.to_string());
                out.push(format!("  {}", c.doc));
                return out;
            }
            if let Some(e) = api.get(&t.to_ascii_lowercase()) {
                out.push(e.signature.to_string());
                out.push(format!("  {}", e.doc));
                out.push(format!("  Example: {}", e.example));
                return out;
            }
            if let Some(cat) = Category::ALL.iter().find(|c| c.name().eq_ignore_ascii_case(t)) {
                out.push(format!("{} functions:", cat.name()));
                for e in api.in_category(*cat) {
                    let sig = truncate(e.signature, 42);
                    out.push(truncate(&format!("  {sig:<42} {}", e.doc), cols.saturating_sub(1)));
                }
                return out;
            }
            match api.suggest(&t.to_ascii_lowercase()) {
                Some(s) => out.push(format!("No help for '{t}'. Did you mean {s}?")),
                None => out.push(format!("No help for '{t}'. Type HELP for a list of topics.")),
            }
        }
    }
    out
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n.saturating_sub(3)).collect();
        t.push_str("...");
        t
    }
}

/// Lay out short words in columns.
pub fn columns(words: &[&str], cols: usize) -> Vec<String> {
    let w = words.iter().map(|s| s.chars().count()).max().unwrap_or(1) + 2;
    let per_row = (cols.max(w) / w).max(1);
    words
        .chunks(per_row)
        .map(|row| {
            let mut s = String::from("  ");
            for (i, word) in row.iter().enumerate() {
                s.push_str(word);
                if i + 1 < row.len() {
                    for _ in 0..(w - word.chars().count()) {
                        s.push(' ');
                    }
                }
            }
            s
        })
        .collect()
}
