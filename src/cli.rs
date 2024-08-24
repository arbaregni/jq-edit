use clap::Parser;

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
    pub print_run_log_file: bool,

    #[arg(long)]
    /// Supply this flag to colorize the json output
    pub colorize: bool,

}
