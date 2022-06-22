use yahoo_finance_api as yahoo;
use std::time::{Duration, UNIX_EPOCH};
use chrono::prelude::*;
use tokio_test;

fn main() {
    let provider = yahoo::YahooConnector::new();
    // get the latest quotes in 1 minute intervals

    let start = Utc.ymd(2020, 1, 1).and_hms_milli(0, 0, 0, 0);
    let end = Utc.ymd(2020, 1, 31).and_hms_milli(23, 59, 59, 999);
    // let resp = tokio_test::block_on(provider.get_quote_history("AAPL", start, end)).unwrap();
    // let resp = tokio_test::block_on(provider.get_quote_range("AAPL", "1d", "max")).unwrap();
    let resp = tokio_test::block_on(provider.get_quote_range("AAPL", "1d", "2yrs")).unwrap();
    let quotes = resp.quotes().unwrap();
    println!("Apple's quotes in January: {:?}", quotes);
}

