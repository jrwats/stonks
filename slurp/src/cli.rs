use structopt::{self, StructOpt};

#[derive(StructOpt, Debug)]
#[structopt(name = "slurp", global_setting = structopt::clap::AppSettings::ColoredHelp)]
pub struct Args {
    #[structopt(subcommand)]
    pub command: Command,
}

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Iterate all newline-delimitted tickers read from stdin and fill the DB with 2 years of
    /// daily candles
    Full,

    /// Iterate all newline-delimiitted tickers and append the last day's candle to the data
    Incremental,

    /// Calculate and store simple moving averages, exponential moving averages, ATRs, etc
    /// for all stored tickers
    CalculateMetrics,
}
