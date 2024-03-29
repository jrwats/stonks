use structopt::{self, StructOpt};

#[derive(StructOpt, Debug)]
#[structopt(name = "slurp", global_setting = structopt::clap::AppSettings::ColoredHelp)]
pub struct Args {
    #[structopt(subcommand)]
    pub command: Command,

    #[structopt(long, default_value = "127.0.0.1")]
    pub ip: String,

    #[structopt(long, default_value = "4001")]
    pub port: u32,

    /// Max number of concurrent requests
    #[structopt(long, default_value = "40")]
    pub req_limit: usize,
}

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Iterate all newline-delimitted tickers read from stdin and fill the DB with 2 years of
    /// daily candles
    Full,

    /// Iterate all newline-delimiitted tickers and append the days of candles since the last row
    /// we have for that ticker
    Incremental {
        /// Don't rely on DB cache - always add latest days from IBKR
        #[structopt(long)]
        force: bool,
    },

    /// Find all tickers (of the ones provided) for whom the last 30-days of metrics abide by the
    /// EMA 8 < EMA 21 < EMA 34 < EMA 89 OR
    /// EMA 8 > EMA 21 > EMA 34 > EMA 89 rules
    TrendCandidates {
        /// Spit out metrics regardless of whether the ticker passes the filter
        #[structopt(long)]
        force: bool,

        /// Only compare 8 ema with 34 ema
        #[structopt(long)]
        loose: bool,

        /// Period (days) for which to require that EMAs abide by strictly consistent ordering
        #[structopt(long, default_value = "42")]
        ema_period: usize,

        /// Default period for the stochastic slow filter
        #[structopt(long, default_value = "8")]
        stoch_k_len: usize,

        /// Smoothing of the k
        #[structopt(long, default_value = "3")]
        stoch_k_smoothing: usize,

        /// D Smoothing (slow smoothing of the actual stochastic value)
        #[structopt(long, default_value = "3")]
        stoch_d_smoothing: usize,

        /// The minimum absolute threshold (added 50) at which we consider this overbought (in downtrend) or
        /// oversold (in uptrend). i.e. a value of 10 means values above 60 will be considered
        /// overbought and values below 40 will be oversold
        #[structopt(long, default_value = "10.0")]
        stoch_threshold: f64,

        #[structopt(long, default_value = "13")]
        adx_period: usize,
    },
}
