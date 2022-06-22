use chrono::prelude::*;
use chrono::Duration;
use std::io::{self, prelude::*};
// use std::time::{Duration, UNIX_EPOCH};
use tokio_test;
use yahoo_finance_api as yahoo;

mod db;

fn main() -> anyhow::Result<()> {
    let provider = yahoo::YahooConnector::new();
    // get the latest quotes in 1 minute intervals

    let _db = db::Db::init(None)?;
    let stdin = io::stdin();

    let now = Utc::now();
    let four_yrs_ago = now - Duration::days(356 * 4);

    for maybe_ticker in stdin.lock().lines() {
        let ticker = maybe_ticker?;
        println!("ticker: {}", &ticker);
        let resp = tokio_test::block_on(provider.get_quote_history(&ticker, four_yrs_ago, now)).unwrap();
        let quotes = resp.quotes().unwrap();
        println!("len: {}", quotes.len());
        println!("first : {:?}", quotes[0]);
        println!("last : {:?}", quotes[quotes.len()-1]);
   }

    Ok(())
}

