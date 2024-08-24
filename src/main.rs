mod json;
mod cli;
mod jq;
mod ui;
mod app;
mod input;
mod my_line_editor;
mod parse;

use std::{
    fs::{self, File}, io::{
        self,
        Read,
    }, panic, time::SystemTime
};

use anyhow::{Context, Result};
use clap::Parser;

use directories::ProjectDirs;
use ratatui::{
    crossterm::{
        terminal::{
            disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
        },
        ExecutableCommand,
    },
    prelude::*,
};

const LOG_FOLDER_NAME: &str = "logs";
/// How many old runs to keep in the log folder
const MAX_LOG_RUNS_SAVED: usize = 20;

fn configure_logging(cli: &cli::Cli, project_dirs: &ProjectDirs) -> Result<String> {
    // ~/.cache/jq-edit
    let log_folder = project_dirs.cache_dir().to_path_buf().join(LOG_FOLDER_NAME);

    // ensure it exists
    if !log_folder.exists() {
        fs::create_dir_all(&log_folder)
            .with_context(|| format!("creating log folder at {}", log_folder.display()))?;
    }

    // create the newest log run
    let now = chrono::Utc::now();
    let filename = format!("run-{}.log", now.format("%Y-%m-%dT%H:%M:%SZ"));
    let filepath = log_folder.join(filename);

    let log_filename = format!("{}", filepath.display());

    let log_file = fern::log_file(filepath)
        .with_context(|| format!("creating new log in {}", log_folder.display()))?;

    fern::Dispatch::new()
        .format(move |out, message, record| {
            let now = chrono::Utc::now();
            out.finish(format_args!(
                "  {} [{}]  {} > {}",
                now.format("%Y-%m-%dT%H:%M:%SZ"),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(cli.log_level)
        .chain(log_file)
        .apply()?;

    Ok(log_filename)
}

fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}


fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let project_dirs = ProjectDirs::from("", "arbaregni", "jq-edit").expect("initialize project directories");

    let log_file = configure_logging(&cli, &project_dirs)?;

    log::info!("reading from stdin");
    let source = read_stdin().unwrap();

    // since it's just going to be around for the entire life of the program,
    // just leak the string now and let the OS deal with it
    let source = source.leak();

    let mut app = crate::app::App::init(source);

    run(&cli, &mut app)
        .expect("running app");

    if cli.print_run_log_file {
        println!("LOG_FILE: {}", log_file);
    }

    if cli.print_filtered_content {
        println!("{}", app.filtered_content());
    }

    println!("QUERY: {}", app.query_content());

    Ok(())
}

fn run(cli: &cli::Cli, app: &mut app::App) -> Result<()> {
    // Set up the terminal for rendering
    log::info!("enabling raw terminal mode");
    enable_raw_mode()?;

    log::info!("entering alternate screen");
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

    log::info!("entering app loop");

    while app.is_running {
        ui::set_query_editor_styles(app);

        term.draw(|f| ui::render_app(app, f))?;

        input::handle_events(app)?;
        app.update(cli)?;

        if app.clear_screen {
            term.clear()?;
            app.clear_screen = false;
        }
    }

    cleanup()?;

    Ok(())
}

fn cleanup() -> Result<()> {
    log::info!("cleaning up");
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

