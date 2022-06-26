use std::io::{self, prelude::*};
use std::time;

use log::{error, info};
use std::thread;

mod app;
mod db;
mod quote;

use app::App;

fn main() -> anyhow::Result<()> {
    let db = db::Db::init(None)?;

    let mut app = App::new(db);

    // port 7497 for TWS or 4001 for IB Gateway, depending on the port you have set
    app.client.connect("127.0.0.1", 4001, 7274605)?;

    for io_ticker in io::stdin().lock().lines() {
        let ticker = io_ticker?;
        app.add_ticker_to_request_queue(ticker);
    }

    let mut count = 0;
    while !app.ticker_request_queue.is_empty() && count < 20 {
        app.request_next_ticker();
        count += 1;
    }

    eprintln!("sent req...");
    loop {
        match app.process_ib_response() {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", e.to_string());
                break ();
            }
        };
    }
    thread::sleep(time::Duration::new(2, 0));

    Ok(())
}
