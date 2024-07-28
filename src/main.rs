mod cli;
mod jq;
mod ui;
mod app;
mod events;

use std::io::{
    self,
    Read,
};

use anyhow::Result;
use clap::Parser;

use ratatui::{
    crossterm::{
        terminal::{
            disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
        },
        ExecutableCommand,
    },
    prelude::*,
};


fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}


fn main() {
    pretty_env_logger::formatted_timed_builder()
        .filter(None, log::LevelFilter::Info)
        .init();

    let cli = cli::Cli::parse();

    log::info!("reading from stdin");
    let source = read_stdin().unwrap();

    log::info!("read: {source}");

    let mut app = crate::app::App::init(source);

    run(&cli, &mut app)
        .expect("running app");
}

fn run(cli: &cli::Cli, app: &mut app::App) -> Result<()> {
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut term = Terminal::new(backend)?;

    while app.run {
        term.draw(|f| ui::render_app(app, f))?;
        events::handle_events(app)?;
        app.update(cli)?;
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
