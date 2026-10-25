mod app;
mod demos;
mod keymap;

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
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let cli = Cli::parse();
    let paths = Paths::detect(cli.portable, cli.disk.as_deref());
    let config = Config::load(&paths.config);

    let console = make_console(paths, config)?;
    app::run(console, None)
}

fn make_console(paths: Paths, config: Config) -> anyhow::Result<Console> {
    Console::new(ConsoleOptions { paths, config, demos: demos::all() }).map_err(|e| anyhow::anyhow!(e))
}
