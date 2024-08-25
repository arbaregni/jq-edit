mod json;
mod cli;
mod jq;
mod ui;
mod app;
mod input;
mod my_line_editor;
mod scroll_text;
mod tokens;

use std::{
    fs::{self, File},
    io::{
        self,
        Read,
    },
    panic
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

// formats a number as a human readable size
fn format_size(size: usize) -> String {
    humansize::format_size(size, humansize::DECIMAL)
}

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

    // clean up the old files
    let mut read_dir = fs::read_dir(&log_folder)
        .with_context(|| format!("reading log folder at {}", log_folder.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("reading log folder at {}", log_folder.display()))?;

    read_dir.sort_by_key(|entry| {
        entry.path()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    });
    read_dir.reverse();

    let to_be_deleted = (MAX_LOG_RUNS_SAVED - 1)..read_dir.len();

    for i in to_be_deleted {
        let path = read_dir[i].path();
        fs::remove_file(&path)
            .with_context(|| format!("attempting to remove old log file"))?;
    }

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

fn read_source(cli: &cli::Cli) -> Result<String> {
    let mut buf = String::new();

    match &cli.input_filename {
        Some(filepath) => {
            // user has supplied a filepath to read from
            log::info!("reading input from {}", filepath.display());
            let mut f = File::open(filepath)
                .with_context(|| format!("opening input file {}", filepath.display()))?;
            f.read_to_string(&mut buf)?;
        },
        None => {
            // default to stdin
            log::info!("reading from stdin");
            io::stdin().read_to_string(&mut buf)?;
        }
    };

    Ok(buf)
}

// 16 kb = 16000 bytes
pub const MAX_STRING_SIZE_TO_PRINT: usize = 16_000;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let project_dirs = ProjectDirs::from("", "arbaregni", "jq-edit").expect("initialize project directories");

    let log_file = configure_logging(&cli, &project_dirs)?;

    let source = read_source(&cli)?;

    // since it's just going to be around for the entire life of the program,
    // just leak the string now and let the OS deal with it
    let source = source.leak();

    let mut app = crate::app::App::init(&cli, source);

    // submit the query once to jq; this will provide the formatting and colorization
    app.submit_query();
    
    // for testing purposes, if we self parse the json, do so now
    if cli.self_parse_json {
        let json_data = json::loads(source);
        println!("{json_data:?}");
    }

    run(&cli, &mut app)
        .expect("running app");

    if cli.print_log_file_path {
        println!("LOG_FILE: {}", log_file);
    }

    if app.filtered_content().len() < MAX_STRING_SIZE_TO_PRINT {
        println!("=============================================");
        println!("{}", app.filtered_content());
        println!("=============================================");
    } else {
        println!("Filtered content is too big to print: {}. Max size is {}.",
                 format_size(app.filtered_content().len()),
                 format_size(MAX_STRING_SIZE_TO_PRINT)
        );
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

