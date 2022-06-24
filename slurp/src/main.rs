use chrono::prelude::*;
use chrono::Duration;
use std::collections::HashMap;
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

fn get_yahoo_quotes(mut db: db::Db) -> anyhow::Result<()> {
    let provider = yahoo::YahooConnector::new();
    let now = Utc::now();
    let four_yrs_ago = now - Duration::days(356 * 4);

    let stdin = io::stdin();
    for maybe_ticker in stdin.lock().lines() {
        let ticker = maybe_ticker?;
        println!("ticker: {}", &ticker);
        tokio_test::block_on(provider.get_quote_history(&ticker, four_yrs_ago, now))
            .map_err(|e| {
                eprintln!("{} not found", &ticker);
                anyhow::Error::from(e)
            })
            .and_then(|resp| {
                let quotes = resp.quotes().unwrap();
                println!("len: {}", quotes.len());
                db.insert_daily_quotes(&ticker, &quotes)
            })
            .ok();
    }
    Ok(())
}

fn us_stock(stk: &str) -> Contract {
    let mut contract = Contract::default();
    contract.symbol = stk.to_string();
    contract.exchange = "SMART".to_string();
    contract.sec_type = "STK".to_string();
    contract.currency = "USD".to_string();
    contract
}

struct Wrapper {
    pub client: EClient,
    pub req2ticker: HashMap<i32, String>,
    next_order_id: i32,
}

impl Wrapper {
    pub fn new() -> Self {
        Wrapper {
            client: EClient::new(),
            req2ticker: HashMap::new(),
            next_order_id: -1,
        }
    }

    fn error(&mut self, req_id: i32, error_code: i32, error_string: &str) {
        error!(
            "req_id: {} ,error_code: {} , error_string:{}",
            req_id, error_code, error_string
        );
    }

    pub fn process_ib_response(&mut self) -> Result<(), IBKRApiLibError> {
        match self.client.get_event()? {
            Some(ServerRspMsg::NextValidId  { order_id }) =>
            {
                self.next_order_id = order_id;
                info!("next_valid_id -- order_id: {}", order_id);
                // if self.start_requests().is_err() {
                //     panic!("start_requests failed!");
                // }
            },
            Some(ServerRspMsg::ErrMsg { req_id, error_code, error_str }) => self.error(req_id, error_code, &error_str),
            Some(ServerRspMsg::TickPrice { req_id, tick_type, price, tick_attr }) =>
                eprintln!("tick_size -- req_id: {}, tick_type: {}, price: {}, attrib: {}", req_id, tick_type, price, tick_attr),
            Some(ServerRspMsg::TickSize { req_id, tick_type, size }) =>
                eprintln!( "tick_size -- req_id: {}, tick_type: {}, size: {}", req_id, tick_type, size),
            Some(ServerRspMsg::CompletedOrder { contract, order, order_state }) =>
                eprintln!("completed_order -- contract: [{}], order: [{}], order_state: [{}]", contract, order, order_state),
            Some(ServerRspMsg::CompletedOrdersEnd)  => info!("completed_orders_end -- (no parameters for this message)"),
            Some(ServerRspMsg::OrderBound {req_id, api_client_id, api_order_id} ) =>
                eprintln!( "order_bound -- req_id: {}, api_client_id: {}, api_order_id: {}", req_id, api_client_id, api_order_id),
            Some(ServerRspMsg::MarketDataType {req_id, market_data_type}) =>
                eprintln!("market_data_type -- req_id: {}, market_data_type: {}", req_id, market_data_type),
            Some(ServerRspMsg::ManagedAccts { accounts_list }) =>
                eprintln!("managed_accounts -- accounts_list: {}", accounts_list),
            Some(ServerRspMsg::OpenOrderEnd) => info!("open_order_end. (no parameters passed)"),
            Some(ServerRspMsg::OpenOrder { order_id, contract, order, order_state }) =>
                eprintln!("open_order -- order_id: {}\n\n\t     contract: {}\n\t     order: {}\n\t    order_state: {}",
                      order_id, contract, order, order_state),
            Some(ServerRspMsg::OrderStatus { order_id, status, filled, remaining, avg_fill_price, perm_id, parent_id,
                          last_fill_price, client_id, why_held, mkt_cap_price}) =>
                          info!("order_status -- order_id: {}, status: {}, filled: {}, remaining: {}, avg_fill_price: {}, \
                    perm_id: {}, parent_id: {}, last_fill_price: {}, client_id: {}, why_held: {}, mkt_cap_price: {}",
                    order_id, status, filled, remaining, avg_fill_price, perm_id, parent_id, last_fill_price,
                    client_id, why_held, mkt_cap_price),
            Some(ServerRspMsg::ExecutionData { req_id, contract, execution }) =>
                eprintln!("exec_details -- req_id: {}, contract: {}, execution: {}", req_id, contract, execution),
            Some(ServerRspMsg::ExecutionDataEnd { req_id }) => info!("exec_details_end -- req_id: {}", req_id),
            Some(ServerRspMsg::HistoricalData { req_id, bar }) => { 
                eprintln!("{} - bar: {}", req_id, bar);
            },
            Some(ServerRspMsg::HistoricalDataEnd { req_id, start, end }) => eprintln!("{} {} {}", req_id, start, end),
            Some(ServerRspMsg::CommissionReport { commission_report }) =>
                eprintln!("commission_report -- commission_report: {}", commission_report),
            Some(i) => panic!("Received unhandled event! Exiting. Event: {}", i.to_string()),
            None => (),
        }

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let db = db::Db::init(None)?;
    // get_yahoo_quotes(db)?;

    let now = Utc::now();
    let four_yrs_ago = now - Duration::days(356 * 4);

    let mut app = Wrapper::new();
    // port 7497 for TWS or 4001 for IB Gateway, depending on the port you have set
    app.client.connect("127.0.0.1", 4001, 7274605)?;
    let dt = Utc::now();
    let query_time = dt.format("%Y%m%d %H:%M:%S").to_string();
    info!("Request Time:  {}", query_time);

    let contract = us_stock("AAPL");
    app.client.req_historical_data(
        4102,
        &contract,
        query_time.as_str(),
        "2 Y",
        "1 day",
        "TRADES",
        1,
        1,
        false,
        vec![],
    )?;
    eprintln!("sent req...");

    loop {
        match app.process_ib_response() {
            Ok(_) => continue,
            Err(e) => {
                eprintln!("{}", e.to_string());
                // error!("{}", e.to_string());
                break();
            }
        };
    }
    thread::sleep(time::Duration::new(2, 0));

    // m_pClient->reqHistoricalData(4003, ContractSamples::USStockAtSmart(), queryTime, "1 M", "1 day", "SCHEDULE", 1, 1, false, TagValueListSPtr());

    Ok(())
}
