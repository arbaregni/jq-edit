mod json;
mod cli;
mod jq;
mod ui;
mod app;
mod input;
mod my_line_editor;

use std::{
    io::{
        self,
        Read,
    },
    panic
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
    let cli = cli::Cli::parse();


    pretty_env_logger::formatted_timed_builder()
        .filter(None, cli.level_filter)
        .init();

    log::info!("reading from stdin");
    let source = read_stdin().unwrap();

    let mut app = crate::app::App::init(source);


    run(&cli, &mut app)
        .expect("running app");

    println!("QUERY: {}", app.query_content());
}

fn run(cli: &cli::Cli, app: &mut app::App) -> Result<()> {
    // Set up the terminal for rendering
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;

    // Restore the terminal on program failure
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        match cleanup() {
            Ok(()) => {},
            Err(e) => {
                eprintln!("during attempted cleanup, an error occured: {e}");
            }
        };
        default_hook(info);
    }));

    let backend = CrosstermBackend::new(io::stdout());
    let mut term = Terminal::new(backend)?;

    while app.run {
        ui::set_query_editor_styles(app);
        term.draw(|f| ui::render_app(app, f))?;

        input::handle_events(app)?;
        app.update(cli)?;
    }

    cleanup()?;

    Ok(())
}

fn cleanup() -> Result<()> {
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

