use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[arg(long, default_value_t = log::LevelFilter::Off)]
    /// The level to log at.
    pub log_level: log::LevelFilter,

    #[arg(long)]
    /// Supply this flag to refresh jq every frame
    pub refresh_jq_every_frame: bool,

    #[arg(long)]
    /// Supply this flag to colorize the json output
    pub colorize: bool
}
