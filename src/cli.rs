use clap::{ArgAction, Parser};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[arg(short, long)]
    /// Supply this flag to print the filtered content
    pub print_filtered_content: bool,

    #[arg(long, default_value_t = log::LevelFilter::Info)]
    /// The level to log at.
    pub log_level: log::LevelFilter,

    #[arg(long)]
    /// Supply this flag to print the log file as part of the program exit summary
    pub print_log_file_path: bool,

    #[arg(long, default_value_t = true, value_parser=parse_bool, action=ArgAction::Set)]
    /// Supply this flag to colorize the json output
    pub colorize: bool,

}

fn parse_bool(s: &str) -> Result<bool, &'static str> {

    match s.to_lowercase().as_str() {
        "true" | "yes" => Ok(true),
        "false" | "no" => Ok(false),
        _ => Err("expected `true` or `false`")
    }

}
