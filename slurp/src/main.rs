use std::io::{self, prelude::*};
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use rayon::prelude::*;

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
            println!("{}\t{}\t{}\t{}", "ticker", "stoch", "ADX", "RSI");
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;

                // get 2 months of data for
                // eprintln!("{}", ticker);
                // let start = std::time::Instant::now();
                let metric_rows = db.get_metrics_for_ticker(&ticker, None)?;
                // eprintln!("fetched in: {:?}", start.elapsed());
                if metric_rows.is_empty() {
                    eprintln!("No rows for {}", ticker);
                    continue;
                }
                if *ema_period > metric_rows.len() {
                    continue;
                }
                let ema_start_idx = metric_rows.len() - ema_period;
                let bull_trend = metric_rows[ema_start_idx..].iter().all(|mr| {
                    let m = &mr.metrics;
                    m.ema_8 > m.ema_21 && m.ema_21 > m.ema_34 && m.ema_34 > m.ema_89 ||
                        *loose && m.ema_8 > m.ema_34
                });
                let bear_trend = metric_rows[ema_start_idx..].iter().all(|mr| {
                    let m = &mr.metrics;
                    m.ema_8 < m.ema_21 && m.ema_21 < m.ema_34 && m.ema_34 < m.ema_89 ||
                        *loose && m.ema_8 < m.ema_34
                });

                let slow_stoch = stoch::get_slow_stoch(
                    *stoch_k_len,
                    *stoch_k_smoothing,
                    *stoch_d_smoothing,
                    &metric_rows,
                );
                let quotes: Vec<Quote> = metric_rows.into_iter().map(|mr| mr.quote).collect();
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
        Command::CalculateMetrics => {
            let adb = Arc::new(Mutex::new(db));
            let mut tickers: Vec<String> = Vec::with_capacity(2048);
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;
                tickers.push(ticker.clone());
                // app.calculate_and_insert_metrics(&ticker)?;
            }
            tickers.par_iter_mut().map(|ref sym| {
                let quotes = {
                    let db = adb.lock().unwrap();
                    db.get_all_daily_quotes(sym)?
                };
 
                for window in db::SMA_WINDOWS {
                    let smas = calc::get_moving_avgs(window, &quotes);
                    let table = format!("sma_{}", window);
                    {
                        let mut db = adb.lock().unwrap();
                        db.insert_calculations(&table, &smas)?;
                    }
                }

                for window in db::EMA_WINDOWS {
                    let emas = calc::get_exp_moving_avgs(window, &quotes);
                    let table = format!("ema_{}", window);
                    {
                        let mut db = adb.lock().unwrap();
                        db.insert_calculations(&table, &emas)?;
                    }
                }
                Ok(())
            }).collect::<anyhow::Result<Vec<()>>>()?;
        }
    }
    Ok(())
}
