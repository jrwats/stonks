use std::io::{self, prelude::*};
use structopt::StructOpt;

// use log::{error, info};

mod app;
mod cli;
mod db;
mod quote;

use crate::cli::{Args, Command};
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
            let mut app = App::new(db);
            // port 7497 for TWS or 4001 for IB Gateway, depending on the port you have set
            app.client.connect("127.0.0.1", 4001, 7274605)?;
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;
                app.add_ticker_to_request_queue(ticker);

                let mut count = 0;
                while !app.ticker_request_queue.is_empty() && count < 20 {
                    // 20 concurrent requests
                    app.request_next_ticker()?;
                    count += 1;
                }
            }
            wait_loop(app);
        }
        Command::Incremental => {
            eprintln!("Unimplemented");
        }
        Command::TrendCandidates => {
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;

                // let 2 months of data for
                let metric_rows = db.get_metrics_for_ticker(&ticker, 42)?;
                let bull_trend = metric_rows.iter().all(|mr| {
                    let m = &mr.metrics;
                    m.ema_8 > m.ema_21 && m.ema_21 > m.ema_34 && m.ema_34 > m.ema_89.unwrap_or(f64::NEG_INFINITY)
                });
                let bear_trend = metric_rows.iter().all(|mr| {
                    let m = &mr.metrics;
                    m.ema_8 < m.ema_21 && m.ema_21 < m.ema_34 && m.ema_34 < m.ema_89.unwrap_or(f64::INFINITY)
                });
                if bull_trend || bear_trend {
                    println!("{}", ticker);
                }

            }
        }
        Command::CalculateMetrics => {
            let mut app = App::new(db);
            for io_ticker in io::stdin().lock().lines() {
                let ticker = io_ticker?;
                app.calculate_and_insert_metrics(&ticker)?;
            }
        }
    }
    Ok(())
}
