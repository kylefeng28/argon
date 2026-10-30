mod app;
mod demos;
mod headless;
mod keymap;
mod script;

use argon_core::config::{Config, Paths};
use argon_shell::{Console, ConsoleOptions};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "argon", version, about = "A fantasy console")]
struct Cli {
    /// Use this folder as the virtual disk instead of ~/.argon/disk
    #[arg(long, global = true)]
    disk: Option<PathBuf>,
    /// Keep all data next to the executable (argon-data/)
    #[arg(long, global = true)]
    portable: bool,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run a program (virtual path on the disk, e.g. DEMOS/pong)
    Run {
        program: String,
        #[command(flatten)]
        head: HeadlessArgs,
    },
    /// Boot to the shell headless, drive it with a key script, screenshot
    Shell {
        #[command(flatten)]
        head: HeadlessArgs,
    },
    /// Parse a program without running it
    Check { program: String },
    /// Print the API reference as Markdown
    Docs {
        /// One line per function (for prompts and quick reference)
        #[arg(long)]
        compact: bool,
    },
}

#[derive(clap::Args, Debug, Clone, Default)]
struct HeadlessArgs {
    /// Render offscreen without a window
    #[arg(long)]
    headless: bool,
    /// Number of frames to run before exiting (headless)
    #[arg(long, default_value_t = 60)]
    frames: u32,
    /// Write the final screen to this PNG (headless)
    #[arg(long)]
    screenshot: Option<PathBuf>,
    /// Keystroke script, e.g. "help{enter}{wait:10}" (headless)
    #[arg(long)]
    keys: Option<String>,
    /// Print the text layer to stdout at the end (headless)
    #[arg(long)]
    text: bool,
    /// Headless window size, e.g. 1920x1200 (shell fits it 16:10; native
    /// programs use it exactly)
    #[arg(long, default_value = "1280x800")]
    window: String,
    /// Write the raw framebuffer instead of the filtered display
    #[arg(long)]
    raw: bool,
    /// Also write frames to this folder as frame-00000.png, ... (headless),
    /// for turning into a GIF or video with ffmpeg
    #[arg(long)]
    record: Option<PathBuf>,
    /// With --record, keep every Nth frame (2 = 30 fps)
    #[arg(long, default_value_t = 2)]
    record_every: u32,
    /// With --record, skip this many frames first
    #[arg(long, default_value_t = 0)]
    record_from: u32,
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    let paths = Paths::detect(cli.portable, cli.disk.as_deref());
    let config = Config::load(&paths.config);

    match cli.cmd {
        None => {
            let console = make_console(paths, config, true)?;
            app::run(console, None)
        }
        Some(Cmd::Run { program, head }) => {
            let console = make_console(paths, config, !head.headless)?;
            if head.headless {
                headless::run(console, &head, Some(&program))
            } else {
                app::run(console, Some(program))
            }
        }
        Some(Cmd::Shell { head }) => {
            let console = make_console(paths, config, false)?;
            headless::run(console, &head, None)
        }
        Some(Cmd::Check { program }) => check(paths, &program),
        Some(Cmd::Docs { compact }) => {
            print!("{}", if compact { docs_compact() } else { docs_markdown() });
            Ok(())
        }
    }
}

fn make_console(paths: Paths, config: Config, audio: bool) -> anyhow::Result<Console> {
    Console::new(ConsoleOptions { paths, config, demos: demos::all(), audio }).map_err(|e| anyhow::anyhow!(e))
}

fn check(paths: Paths, program: &str) -> anyhow::Result<()> {
    use argon_core::vfs::{resolve_program, Jail};
    paths.ensure_dirs()?;
    argon_shell::console::install_demos(&paths, &demos::all());
    let disk = Jail::new(paths.disk.clone());
    let loc = resolve_program(&disk, "", program).ok_or_else(|| anyhow::anyhow!("program not found: {program}"))?;
    let src = disk.read_string(&loc.main)?;
    let result = match loc.kind {
        argon_core::vfs::ProgramKind::Lua => mlua_check(&src, &loc.main),
        argon_core::vfs::ProgramKind::Glsl => argon_core::shader::compile(&src).map(|_| ()).map_err(|e| format!("{}: {e}", loc.main)),
    };
    match result {
        Ok(()) => {
            println!("{}: OK", loc.main);
            Ok(())
        }
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn mlua_check(src: &str, name: &str) -> Result<(), String> {
    use argon_core::vfs::Jail;
    use argon_lua::machine::{MachineOptions, NoFlip};
    use argon_lua::runtime::Runtime;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    let tmp = std::env::temp_dir();
    let rt = Runtime::new(MachineOptions::default(), Jail::new(tmp), None, Arc::new(AtomicBool::new(false)), Box::new(NoFlip))
        .map_err(|e| e.to_string())?;
    rt.lua.load(src).set_name(format!("={name}")).into_function().map(|_| ()).map_err(|e| {
        let err = argon_lua::ArgonError::from_lua(&e, &rt.api, |_, l| src.lines().nth(l as usize - 1).map(|s| s.to_string()));
        err.render()
    })
}

fn docs_compact() -> String {
    use argon_core::api::Category;
    let api = argon_lua::docs::api_table();
    let mut out = String::new();
    for cat in Category::ALL {
        let entries: Vec<_> = api.in_category(cat).collect();
        if entries.is_empty() {
            continue;
        }
        out.push_str(&format!("## {}\n", cat.name()));
        for e in entries {
            out.push_str(&format!("{}  -- {}\n", e.signature, e.doc));
        }
        out.push('\n');
    }
    out
}

fn docs_markdown() -> String {
    use argon_core::api::Category;
    let api = argon_lua::docs::api_table();
    let mut out = String::from("# Argon API (api = 1)\n\n");
    for cat in Category::ALL {
        let entries: Vec<_> = api.in_category(cat).collect();
        if entries.is_empty() {
            continue;
        }
        out.push_str(&format!("## {}\n\n", cat.name()));
        for e in entries {
            out.push_str(&format!("### `{}`\n\n{}\n\n```lua\n{}\n```\n\n", e.signature, e.doc, e.example));
        }
    }
    out
}
