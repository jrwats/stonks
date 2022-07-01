use std::io::{self, prelude::*};
use structopt::StructOpt;

// use log::{error, info};

mod app;
mod cli;
mod db;
mod quote;
mod stoch;

use crate::cli::{Args, Command};
use crate::quote::Quote;
use app::App;

fn wait_loop(mut app: App) {
    loop {
        match app.process_ib_response() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", e.to_string());
                break ();
            }
        };
    }
}

fn main() -> anyhow::Result<()> {
    let db = db::Db::init(None)?;

    let args = Args::from_args();
    match args.command {
        Command::Full => {
            let mut app = App::new(db, false);
            // port 7497 for TWS or 4001 for IB Gateway, depending on the port you have set
            app.client.connect("127.0.0.1", 4001, 7274605)?;
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;
                app.add_ticker_to_request_queue(ticker);
            }
            let mut count = 0;
            while !app.ticker_request_queue.is_empty() && count < 20 {
                // 20 concurrent requests
                app.request_next_ticker()?;
                count += 1;
            }
            wait_loop(app);
        }
        Command::Incremental => {
            let mut app = App::new(db, true);
            // port 7497 for TWS or 4001 for IB Gateway, depending on the port you have set
            app.client.connect("127.0.0.1", 4001, 7274605)?;
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;
                app.add_ticker_to_request_queue(ticker);
            }
            let mut count = 0;
            while !app.ticker_request_queue.is_empty() && count < 20 {
                app.request_next_incremental_ticker()?;
                count += 1;
            }
            wait_loop(app);
        }
        Command::TrendCandidates {
            ref ema_period,
            ref stoch_k_len,
            ref stoch_k_smoothing,
            ref stoch_d_smoothing,
            ref stoch_threshold,
            ref adx_period,
        } => {
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;

                // get 2 months of data for
                eprintln!("{}", ticker);
                let metric_rows = db.get_metrics_for_ticker(&ticker, *ema_period)?;
                eprintln!("Gotten");
                if metric_rows.is_empty() {
                    eprintln!("No rows for {}", ticker);
                    continue;
                }
                let bull_trend = metric_rows.iter().all(|mr| {
                    let m = &mr.metrics;
                    m.ema_8 > m.ema_21 && m.ema_21 > m.ema_34 && m.ema_34 > m.ema_89
                });
                let bear_trend = metric_rows.iter().all(|mr| {
                    let m = &mr.metrics;
                    m.ema_8 < m.ema_21 && m.ema_21 < m.ema_34 && m.ema_34 < m.ema_89
                });

                let slow_stoch = stoch::get_slow_stoch(
                    *stoch_k_len,
                    *stoch_k_smoothing,
                    *stoch_d_smoothing,
                    &metric_rows,
                );
                let quotes: Vec<Quote> = metric_rows.into_iter().map(|mr| mr.quote).collect();
                let adx = stoch::get_adx(&quotes, 13, 13);
                println!("{}\t{}\t{}", ticker, slow_stoch, adx);
                if bull_trend && slow_stoch <= 40.0 || bear_trend && slow_stoch >= 60.0 {
                    println!("{}\t{}\t{}", ticker, slow_stoch, adx);
                }
            }
        }
        Command::CalculateMetrics => {
            let mut app = App::new(db, false);
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;
                app.calculate_and_insert_metrics(&ticker)?;
            }
        }
    }
    Ok(())
}
