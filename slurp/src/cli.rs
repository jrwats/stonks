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

    /// Find all tickers (of the ones provided) for whom the last 30-days of metrics abide by the
    /// EMA 8 < EMA 21 < EMA 34 < EMA 89 OR
    /// EMA 8 > EMA 21 > EMA 34 > EMA 89 rules
    TrendCandidates,

}
