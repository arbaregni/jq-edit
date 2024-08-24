use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[arg(long, default_value_t = log::LevelFilter::Off)]
    /// The level to log at.
    pub level_filter: log::LevelFilter
}
