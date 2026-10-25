mod app;
mod keymap;

use argon_shell::Console;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let console = Console::new().map_err(|e| anyhow::anyhow!(e))?;
    app::run(console, None)
}
