use chrono::prelude::*;
use chrono::Duration;
use std::collections::{HashMap, VecDeque};
use std::time;

use ibtwsapi::core::client::EClient;
use ibtwsapi::core::contract::Contract;
use ibtwsapi::core::messages::ServerRspMsg;
use log::{error, info};
use std::thread;

use crate::db::Db;
use crate::quote::Quote;

const CONCURRENCY_LIMIT: usize = 40;
const CONCURRENCY_BUFFER: usize = 10;

#[derive(Debug)]
pub struct TickerQuote {
    ticker: String,
    quote: Quote,
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

pub struct App {
    pub client: EClient,
    pub db: Db,
    pub req_limit: usize,
    pub force: bool,
    pub full_ticker_queue: VecDeque<String>,
    pub incremental_ticker_queue: VecDeque<String>,
    pub open_requests: HashMap<i32, (bool, String)>,
    pub quotes: VecDeque<TickerQuote>,
    pub req_id: i32,
    next_order_id: i32,
}

/// Avoid requesting daily tickers in the middle of trading day
fn close_time(mut dt: DateTime<Utc>) -> DateTime<Utc> {
    // If the hours given is less than 4:30PM, return yesterday's 4:30PM time for query input.
    // TimeZone.from_offset9
    let edt_tz = FixedOffset::west(4 * 3600);
    let edt = dt.with_timezone(&edt_tz);
    if (edt.hour() > 9 || edt.hour() == 9 && edt.minute() >= 30)
        && (edt.hour() < 16 || edt.hour() == 16 && edt.minute() < 30)
    {
        dt = dt - Duration::days(1);
        return Utc.ymd(dt.year(), dt.month(), dt.day()).and_hms(11, 59, 0);
    }
    dt
}

impl App {
    pub fn new(db: Db, req_limit: usize, force: bool) -> Self {
        App {
            client: EClient::new(),
            req_limit,
            db,
            force,
            open_requests: HashMap::new(),
            full_ticker_queue: VecDeque::new(),
            incremental_ticker_queue: VecDeque::new(),
            quotes: VecDeque::with_capacity(2048),
            next_order_id: -1,
            req_id: 1,
        }
    }

    fn error(&mut self, req_id: i32, error_code: i32, error_string: &str) {
        let ticker = self.open_requests.get(&req_id);
        eprintln!(
            "{} => {:?}, {}, {}",
            req_id, ticker, error_code, error_string
        );
        if ticker.is_some() {
            self.open_requests.remove(&req_id);
            self.request_next_ticker().ok();
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
        let dt = close_time(Utc::now());
        let query_time = dt.format("%Y%m%d %H:%M:%S").to_string();
        self.req_id += 1;
        self.open_requests
            .insert(self.req_id, (false, ticker.to_string()));
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

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut count = 0;
        while !self.full_ticker_queue.is_empty() && count < self.req_limit {
            self.request_next_ticker()?;
            count += 1;
        }
        while !self.incremental_ticker_queue.is_empty() && count < self.req_limit {
            self.request_next_incremental_ticker()?;
            count += 1;
        }
        self.wait_loop();
        Ok(())
    }

    fn wait_loop(&mut self) {
        loop {
            match self.process_ib_response() {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{}", e.to_string());
                    break ();
                }
            };
        }
    }

    pub fn request_next_incremental_ticker(&mut self) -> anyhow::Result<bool> {
        if let Some(ticker) = self.incremental_ticker_queue.pop_front() {
            let last_quote = self.db.get_last_quote(&ticker)?;
            let dt = close_time(Utc::now());
            // eprintln!("dt: {:?}", dt);
            let query_time = dt.format("%Y%m%d %H:%M:%S").to_string();
            let last_quote = Utc.timestamp(last_quote.quote.timestamp, 0);
            let num_days = (dt - last_quote).num_days();
            if num_days == 0 && !self.force {
                eprintln!("skipping up-to-date {}", &ticker);
                self.request_next_incremental_ticker()?;
                return Ok(true);
            }
            let day_str = format!("{} D", num_days + 2);
            self.req_id += 1;
            let contract = us_stock(&ticker, self.db.get_exchange(&ticker)?);
            eprintln!(
                "requesting '{}' for {}, req_id: {}",
                &day_str, &ticker, self.req_id
            );
            self.open_requests
                .insert(self.req_id, (true, ticker.to_string()));
            self.client.req_historical_data(
                self.req_id,
                &contract,
                query_time.as_str(),
                &day_str,
                "1 day",
                "TRADES",
                1,
                1,
                false,
                vec![],
            )?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn add_incremental_ticker(&mut self, ticker: String) {
        self.incremental_ticker_queue.push_back(ticker);
    }

    pub fn add_ticker_to_request_queue(&mut self, ticker: String) {
        self.full_ticker_queue.push_back(ticker);
    }

    pub fn request_next_ticker(&mut self) -> anyhow::Result<bool> {
        if let Some(ticker) = self.full_ticker_queue.pop_front() {
            self.request_ticker(&ticker)?;
            return Ok(true);
        }
        Ok(false)
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
            Some(ServerRspMsg::NewsBulletins { .. }) => info!("news bulletin ignored"),
            Some(ServerRspMsg::HistoricalData { req_id, bar }) => {
                let quote = bar.try_into()?;
                let (_incremental, ticker) = self
                    .open_requests
                    .get(&req_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown req_id {}", req_id))?;
                let tq = TickerQuote {
                    ticker: ticker.clone(),
                    quote,
                };
                // eprintln!("tq: {:?}", tq);
                self.quotes.push_back(tq);
            }
            Some(ServerRspMsg::HistoricalDataEnd { req_id, start, end }) => {
                eprintln!("end: {} {} {}", req_id, start, end);
                let (incremental, ticker) = self
                    .open_requests
                    .remove(&req_id)
                    .ok_or_else(|| anyhow::anyhow!("unexpected {}", req_id))?;
                eprintln!("{} - {} quotes", ticker, self.quotes.len());
                if incremental {
                    self.request_next_incremental_ticker()?;
                } else {
                    self.request_next_ticker()?;
                }
                let mut tick2quotes: HashMap<String, Vec<Quote>> = HashMap::new();
                for tq in self.quotes.drain(0..) {
                    let qs = tick2quotes.entry(tq.ticker).or_insert(vec![]);
                    qs.push(tq.quote);
                }
                for (ticker, quotes) in tick2quotes {
                    if incremental {
                        // check for updated close numbers and request new 2-year data if so
                        let first_quote = &quotes[0];
                        let maybe_cached_quote = self
                            .db
                            .get_quote_with_timestamp(&ticker, first_quote.timestamp)?;
                        if let Some(cached_quote) = maybe_cached_quote {
                            if cached_quote.quote.close != first_quote.close {
                                eprintln!(
                                    "{} != {} for {}",
                                    cached_quote.quote.close, first_quote.close, ticker
                                );
                                self.add_ticker_to_request_queue(ticker);
                                if self.open_requests.len() < CONCURRENCY_LIMIT + CONCURRENCY_BUFFER
                                {
                                    self.request_next_ticker()?;
                                }
                                continue;
                            }
                        } else {
                            eprintln!(
                                "No row for {} with timestamp {}",
                                ticker, first_quote.timestamp
                            );
                            continue;
                        }
                    }
                    // eprintln!("inserting {:?}", quotes);
                    self.db.insert_daily_quotes(&ticker, &quotes)?;
                }
                // let start = time::Instant::now();
                // self.db.calculate_and_insert_metrics(&ticker)?;
                // eprintln!("calculate & insert metrics in: {:?}", start.elapsed());
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
}
