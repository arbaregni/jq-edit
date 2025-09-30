use std::path::PathBuf;

use clap::{ArgAction, Parser};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[arg()]
    // Supply an optional parameter to start the query on
    pub query: Option<String>,

    #[arg(long)]
    pub test_streaming: bool,

    #[arg(short = 'f', long)]
    /// Supply an optional parameter to read the input from a file, instead of stdin
    pub input_filename: Option<PathBuf>,

    #[arg(long, default_value_t = log::LevelFilter::Info)]
    /// The level to log at.
    pub log_level: log::LevelFilter,

    /// Print logs to stdout
    #[arg(long)]
    pub print_logs: bool,

    #[arg(long)]
    /// Supply this flag to print the log file as part of the program exit summary
    pub print_log_file_path: bool,

    #[arg(long, default_value_t = true, value_parser=parse_bool, action=ArgAction::Set)]
    /// Supply this flag to colorize the json output
    pub colorize: bool,

    #[arg(long)]
    /// Testing flag, supply it to use the homegrown json parsing solution rather than delagating to JQ
    pub self_parse_json: bool,
}


fn parse_bool(s: &str) -> Result<bool, &'static str> {

    match s.to_lowercase().as_str() {
        "true" | "yes" => Ok(true),
        "false" | "no" => Ok(false),
        _ => Err("expected `true` or `false`")
    }

}
