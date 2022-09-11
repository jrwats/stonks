use std::io::{self, prelude::*};
use std::collections::HashMap;
use structopt::StructOpt;

mod app;
mod calc;
mod cli;
mod db;
mod quote;
mod stoch;

use crate::cli::{Args, Command};
use crate::quote::Quote;
use app::App;

fn main() -> anyhow::Result<()> {
    let db = db::Db::init(None)?;

    let args = Args::from_args();
    match args.command {
        Command::Full => {
            let mut app = App::new(db, args.req_limit, false);
            // port 7497 for TWS or 4001 for IB Gateway, depending on the port you have set
            app.client.connect(&args.ip, args.port, 7274605)?;
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;
                app.add_ticker_to_request_queue(ticker);
            }
            app.run()?;
        }
        Command::Incremental { force } => {
            let mut app = App::new(db, args.req_limit, force);
            // port 7497 for TWS or 4001 for IB Gateway, depending on the port you have set
            app.client.connect(&args.ip, args.port, 7274605)?;
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;
                app.add_incremental_ticker(ticker);
            }
            app.run()?;
        }
        Command::TrendCandidates {
            ref force,
            ref ema_period,
            ref stoch_k_len,
            ref stoch_k_smoothing,
            ref stoch_d_smoothing,
            ref stoch_threshold,
            ref loose,
            ref adx_period,
        } => {
            let mut tickers: Vec<String> = Vec::with_capacity(2048);
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;
                tickers.push(ticker.clone());
            }
            let sym2quotes = db.get_daily_batch(&tickers)?;
            if sym2quotes.len() != tickers.len() {
                for ticker in tickers {
                    if !sym2quotes.contains_key(&ticker) {
                        eprintln!("missing quotes for: {}", ticker);
                    }
                }
            }
            println!("{}\t{}\t{}\t{}", "ticker", "stoch", "ADX", "RSI");
            for (ticker, quotes) in sym2quotes {
                let ema_8: HashMap<i32, f64> =
                    calc::get_exp_moving_avgs(8, &quotes).into_iter().collect();
                let ema_21: HashMap<i32, f64> =
                    calc::get_exp_moving_avgs(21, &quotes).into_iter().collect();
                let ema_34: HashMap<i32, f64> =
                    calc::get_exp_moving_avgs(34, &quotes).into_iter().collect();
                let ema_89: HashMap<i32, f64> =
                    calc::get_exp_moving_avgs(89, &quotes).into_iter().collect();

                // get 2 months of data for
                if *ema_period > quotes.len() {
                    continue;
                }
                let ema_start_idx = quotes.len() - ema_period;
                let bull_trend = quotes[ema_start_idx..].iter().all(|q| {
                    let i = q.id;
                    ema_8.get(&i) > ema_21.get(&i)
                        && ema_21.get(&i) > ema_34.get(&i)
                        && ema_34.get(&i) > ema_89.get(&i)
                        || *loose && ema_8.get(&i) > ema_34.get(&i)
                });
                let bear_trend = quotes[ema_start_idx..].iter().all(|q| {
                    let i = q.id;
                    ema_8.get(&i) < ema_21.get(&i)
                        && ema_21.get(&i) < ema_34.get(&i)
                        && ema_34.get(&i) < ema_89.get(&i)
                        || *loose && ema_8.get(&i) < ema_34.get(&i)
                });

                let slow_stoch = stoch::get_slow_stoch(
                    *stoch_k_len,
                    *stoch_k_smoothing,
                    *stoch_d_smoothing,
                    &quotes,
                );
                let quotes: Vec<Quote> = quotes.into_iter().map(|qr| qr.quote).collect();
                let adxr = stoch::get_adxr(&quotes, *adx_period, 1);

                if *force
                    || (bull_trend && slow_stoch <= (50.0 - stoch_threshold)
                        || bear_trend && slow_stoch >= (50.0 + stoch_threshold))
                        && adxr > 20.0
                {
                    let rsi = stoch::get_last_rsi(&quotes, 2);
                    println!("{}\t{}\t{}\t{}", ticker, slow_stoch, adxr, rsi);
                }
            }
        }
    }
    Ok(())
}
