use chrono::prelude::*;
use chrono::Duration;
use std::collections::{HashMap, VecDeque};
use std::io::{self, prelude::*};
use std::time;
use tokio_test;
use yahoo_finance_api as yahoo;

use ibtwsapi::core::client::EClient;
use ibtwsapi::core::contract::Contract;
use ibtwsapi::core::errors::*;
use ibtwsapi::core::messages::ServerRspMsg;
use log::{error, info};
use std::thread;

mod db;
mod quote;

use db::QuoteRow;
use quote::Quote;

#[derive(Debug)]
struct TickerQuote {
    ticker: String,
    quote: Quote,
}

fn get_yahoo_quotes(mut db: db::Db) -> anyhow::Result<()> {
    let provider = yahoo::YahooConnector::new();
    let now = Utc::now();
    let four_yrs_ago = now - Duration::days(356 * 4);

    let stdin = io::stdin();
    for maybe_ticker in stdin.lock().lines() {
        let ticker = maybe_ticker?;
        tokio_test::block_on(provider.get_quote_history(&ticker, four_yrs_ago, now))
            .map_err(|e| {
                eprintln!("{} not found", &ticker);
                anyhow::Error::from(e)
            })
            .and_then(|resp| {
                let yahoo_quotes = resp.quotes().unwrap();
                let quotes: Vec<Quote> = yahoo_quotes.into_iter().map(|y| y.into()).collect();
                println!("len: {}", quotes.len());
                db.insert_daily_quotes(&ticker, &quotes)
            })
            .ok();
    }
    Ok(())
}

fn us_stock(stk: &str, primary_exchange: Option<String>) -> Contract {
    let mut contract = Contract::default();
    contract.symbol = stk.to_string();
    contract.exchange = "SMART".to_string();
    contract.sec_type = "STK".to_string();
    contract.currency = "USD".to_string();
    if let Some(pe) = primary_exchange {
        contract.primary_exchange = pe;
    }
    contract
}

struct Wrapper {
    pub client: EClient,
    pub db: db::Db,
    pub ticker_request_queue: VecDeque<String>,
    pub open_requests: HashMap<i32, String>,
    pub quotes: VecDeque<TickerQuote>,
    pub req_id: i32,
    next_order_id: i32,
}

impl Wrapper {
    pub fn new(db: db::Db) -> Self {
        Wrapper {
            client: EClient::new(),
            db,
            open_requests: HashMap::new(),
            ticker_request_queue: VecDeque::new(),
            quotes: VecDeque::with_capacity(2048),
            next_order_id: -1,
            req_id: 1,
        }
    }

    fn error(&mut self, req_id: i32, error_code: i32, error_string: &str) {
        let maybe_ticker = self.open_requests.get(&req_id);
        eprintln!(
            "{} => {:?}, {}, {}",
            req_id, maybe_ticker, error_code, error_string
        );
        if let Some(ticker) = maybe_ticker {
            self.open_requests.remove(&req_id);
            self.request_next_ticker();
        }
        error!(
            "req_id: {} ,error_code: {} , error_string: {}",
            req_id, error_code, error_string
        );
    }

    pub fn request_ticker(&mut self, ticker: &str) -> anyhow::Result<()> {
        let exchange = self.db.get_exchange(ticker)?;
        if let Some(ref e) = exchange {
            eprintln!("{} exchange: {}", ticker, e);
        }
        let contract = us_stock(ticker, exchange);
        let dt = Utc::now();
        let query_time = dt.format("%Y%m%d %H:%M:%S").to_string();
        self.req_id += 1;
        self.open_requests.insert(self.req_id, ticker.to_string());
        eprintln!("requesting {}", ticker);
        Ok(self.client.req_historical_data(
            self.req_id,
            &contract,
            query_time.as_str(),
            "2 Y",
            "1 day",
            "TRADES",
            1,
            1,
            false,
            vec![],
        )?)
    }

    pub fn add_ticker_to_request_queue(&mut self, ticker: String) {
        self.ticker_request_queue.push_back(ticker);
    }

    pub fn request_next_ticker(&mut self) -> bool {
        if let Some(ticker) = self.ticker_request_queue.pop_front() {
            self.request_ticker(&ticker);
            return true;
        }
        false
    }

    pub fn process_ib_response(&mut self) -> anyhow::Result<()> {
        match self.client.get_event()? {
            Some(ServerRspMsg::NextValidId { order_id }) => {
                self.next_order_id = order_id;
                info!("next_valid_id -- order_id: {}", order_id);
            }
            Some(ServerRspMsg::ErrMsg {
                req_id,
                error_code,
                error_str,
            }) => self.error(req_id, error_code, &error_str),
            // Some(ServerRspMsg::TickPrice { req_id, tick_type, price, tick_attr }) =>
            //     eprintln!("tick_size -- req_id: {}, tick_type: {}, price: {}, attrib: {}", req_id, tick_type, price, tick_attr),
            // Some(ServerRspMsg::TickSize { req_id, tick_type, size }) =>
            //     eprintln!( "tick_size -- req_id: {}, tick_type: {}, size: {}", req_id, tick_type, size),
            // Some(ServerRspMsg::CompletedOrder { contract, order, order_state }) =>
            //     eprintln!("completed_order -- contract: [{}], order: [{}], order_state: [{}]", contract, order, order_state),
            // Some(ServerRspMsg::CompletedOrdersEnd)  => info!("completed_orders_end -- (no parameters for this message)"),
            // Some(ServerRspMsg::OrderBound {req_id, api_client_id, api_order_id} ) =>
            //     eprintln!( "order_bound -- req_id: {}, api_client_id: {}, api_order_id: {}", req_id, api_client_id, api_order_id),
            // Some(ServerRspMsg::MarketDataType {req_id, market_data_type}) =>
            //     eprintln!("market_data_type -- req_id: {}, market_data_type: {}", req_id, market_data_type),
            Some(ServerRspMsg::ManagedAccts { accounts_list }) => {
                eprintln!("managed_accounts -- accounts_list: {}", accounts_list)
            }
            // Some(ServerRspMsg::OpenOrderEnd) => info!("open_order_end. (no parameters passed)"),
            // Some(ServerRspMsg::OpenOrder { order_id, contract, order, order_state }) =>
            //     eprintln!("open_order -- order_id: {}\n\n\t     contract: {}\n\t     order: {}\n\t    order_state: {}",
            //           order_id, contract, order, order_state),
            // Some(ServerRspMsg::OrderStatus { order_id, status, filled, remaining, avg_fill_price, perm_id, parent_id,
            //               last_fill_price, client_id, why_held, mkt_cap_price}) =>
            //               info!("order_status -- order_id: {}, status: {}, filled: {}, remaining: {}, avg_fill_price: {}, \
            //         perm_id: {}, parent_id: {}, last_fill_price: {}, client_id: {}, why_held: {}, mkt_cap_price: {}",
            //         order_id, status, filled, remaining, avg_fill_price, perm_id, parent_id, last_fill_price,
            //         client_id, why_held, mkt_cap_price),
            // Some(ServerRspMsg::ExecutionData { req_id, contract, execution }) =>
            //     eprintln!("exec_details -- req_id: {}, contract: {}, execution: {}", req_id, contract, execution),
            // Some(ServerRspMsg::ExecutionDataEnd { req_id }) => info!("exec_details_end -- req_id: {}", req_id),
            Some(ServerRspMsg::HistoricalData { req_id, bar }) => {
                let quote = bar.try_into()?;
                let ticker = self
                    .open_requests
                    .get(&req_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown req_id {}", req_id))?;
                let tq = TickerQuote {
                    ticker: ticker.clone(),
                    quote,
                };
                self.quotes.push_back(tq);
            }
            Some(ServerRspMsg::HistoricalDataEnd { req_id, start, end }) => {
                eprintln!("end: {} {} {}", req_id, start, end);
                eprintln!("{} quotes", self.quotes.len());
                let ticker = self
                    .open_requests
                    .remove(&req_id)
                    .ok_or_else(|| anyhow::anyhow!("unexpected {}", req_id))?;
                self.request_next_ticker();
                let mut tick2quotes: HashMap<String, Vec<Quote>> = HashMap::new();
                for tq in self.quotes.drain(0..) {
                    let qs = tick2quotes.entry(tq.ticker).or_insert(vec![]);
                    qs.push(tq.quote);
                }
                for (ticker, quotes) in tick2quotes {
                    eprintln!("{} for {}", quotes.len(), ticker);
                    self.db.insert_daily_quotes(&ticker, &quotes)?;
                }
                self.calculate_and_insert_metrics(&ticker)?;
            }
            Some(ServerRspMsg::CommissionReport { commission_report }) => eprintln!(
                "commission_report -- commission_report: {}",
                commission_report
            ),
            Some(i) => panic!(
                "Received unhandled event! Exiting. Event: {}",
                i.to_string()
            ),
            None => {
                eprintln!("waiting... {:?}", self.open_requests);
                thread::sleep(time::Duration::new(2, 0));
            }
        }

        Ok(())
    }

    fn calculate_and_insert_metrics(&mut self, ticker: &str) -> anyhow::Result<()> {
        let quotes = self.db.get_all_daily_quotes(ticker)?;
        self.insert_simple_moving_avgs(200, &quotes)?;
        self.insert_simple_moving_avgs(50, &quotes)?;
        for ema_window in [8, 21, 34, 89] {
            self.insert_emas(ema_window, &quotes)?;
        }
        Ok(())
    }

    fn calculate_moving_avgs(window: usize, quotes: &[QuoteRow]) -> Vec<(i32, f64)> {
        let mut sum: f64 = quotes[0..window].iter().map(|q| q.quote.close).sum();
        let mut avgs = Vec::with_capacity(quotes.len() - window);
        avgs.push((quotes[window - 1].id, sum / (window as f64)));
        let mut drop_idx = 0;
        for quote in quotes[window..].iter() {
            sum -= quotes[drop_idx].quote.close;
            sum += quote.quote.close;
            avgs.push((quote.id, sum / (window as f64)));
            drop_idx += 1;
        }
        avgs
    }

    fn insert_simple_moving_avgs(
        &mut self,
        window: usize,
        quotes: &[QuoteRow],
    ) -> anyhow::Result<()> {
        let vals = Self::calculate_moving_avgs(200, quotes);
        let table = format!("sma_{}", window);
        self.db.insert_calculations(&table, &vals)
    }

    fn calculate_exp_moving_avgs(window: usize, quotes: &[QuoteRow]) -> Vec<(i32, f64)> {
        let sum: f64 = quotes[0..window].iter().map(|q| q.quote.close).sum();
        let mut avg: f64 = sum / (window as f64);
        let mut avgs = Vec::with_capacity(quotes.len() - window);
        avgs.push((quotes[window - 1].id, avg));
        let smoothing: f64 = 2.0 / (window as f64 + 1.0);
        for quote in quotes[window..].iter() {
            avg = (quote.quote.close - avg) * smoothing + avg;
            avgs.push((quote.id, avg));
        }
        avgs
    }

    fn insert_emas(&mut self, window: usize, quotes: &[QuoteRow]) -> anyhow::Result<()> {
        let vals = Self::calculate_exp_moving_avgs(200, quotes);
        let table = format!("ema_{}", window);
        self.db.insert_calculations(&table, &vals)
    }
}

fn main() -> anyhow::Result<()> {
    let db = db::Db::init(None)?;

    let mut app = Wrapper::new(db);

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_moving_avgs() {
        let mut id = -1;
        let mocks: Vec<QuoteRow> = [2.0, 3.0, 4.0, 5.5, 6.0, 7.0]
            .map(|f| {
                let mut quote = Quote::default();
                quote.close = f;
                id += 1;
                QuoteRow { id, quote }
            })
            .into_iter()
            .collect();
        let avgs = Wrapper::calculate_moving_avgs(3, &mocks);
        //  [(2, 3.0), (3, 4.166666666666667), (4, 5.166666666666667), (5, 6.166666666666667)]
        assert_eq!(
            avgs,
            vec![(2, 3.0), (3, 12.5 / 3.0), (4, 15.5 / 3.0), (5, 18.5 / 3.0)]
        );
    }

    #[test]
    fn test_exp_avgs() {
        let mut id = -1;
        let mocks: Vec<QuoteRow> = [2.0, 3.0, 4.0, 5.5, 6.0, 7.0]
            .map(|f| {
                let mut quote = Quote::default();
                quote.close = f;
                id += 1;
                QuoteRow { id, quote }
            })
            .into_iter()
            .collect();
        let avgs = Wrapper::calculate_exp_moving_avgs(3, &mocks);
        // [(2, 3.0), (3, 4.25), (4, 5.125), (5, 6.0625)]
        assert_eq!(
            avgs,
            vec![
                (2, 3.0),
                (3, 3.0 + 0.5 * 2.5),
                (4, 4.25 + 1.75 * 0.5),
                (5, 5.125 + 1.875 * 0.5)
            ]
        );
    }
}
