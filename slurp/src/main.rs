use chrono::prelude::*;
use std::io::{self, prelude::*};
use std::time::{Duration, UNIX_EPOCH};
use tokio_test;
use yahoo_finance_api as yahoo;

fn main() -> std::io::Result<()> {
    let provider = yahoo::YahooConnector::new();
    // get the latest quotes in 1 minute intervals

    let stdin = io::stdin();
    for ticker in stdin.lock().lines() {
        println!("ticker: {}", ticker?);
   }

    let start = Utc.ymd(1980, 12, 1).and_hms_milli(0, 0, 0, 0);
    let end = Utc.ymd(2022, 6, 21).and_hms_milli(23, 59, 59, 999);
    let resp = tokio_test::block_on(provider.get_quote_history("AAPL", start, end)).unwrap();
    // let resp = tokio_test::block_on(provider.get_quote_range("AAPL", "1d", "max")).unwrap();
    // let resp = tokio_test::block_on(provider.get_quote_range("AAPL", "1d", "2yrs")).unwrap();
    let quotes = resp.quotes().unwrap();
    println!("len: {}", quotes.len());
    println!("first : {:?}", quotes[0]);
    println!("last : {:?}", quotes[quotes.len()-1]);
    Ok(())
}

