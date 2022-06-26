use anyhow::Context;
use chrono::format::{self, strftime::StrftimeItems, Parsed};
use chrono::prelude::*;
use ibtwsapi::core::common::BarData;

#[derive(Debug, Default)]
pub struct Quote {
    pub timestamp: i64,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub avg: f64, // weighted avg price
    pub volume: i64,
    pub count: i32, // number of trades during the bar's timespan (day)
}

fn to_timestamp(daily: &str) -> anyhow::Result<i64> {
    // "20220623" => 1654781400
    let mut p = Parsed::default();
    format::parse(&mut p, daily, StrftimeItems::new("%Y%m%d"))
        .with_context(|| format!("parsing {}", daily))?;
    p.hour_mod_12 = Some(4); // 4:00
    p.hour_div_12 = Some(1); // PM
    p.minute = Some(0);
    p.second = Some(0);

    // Convert parsed information into a DateTime in the EDT timezone
    let edt_tz = FixedOffset::west(4 * 3600);
    let dt = p.to_datetime_with_timezone(&edt_tz)?;
    let utc_dt = dt.with_timezone(&Utc);

    // let naive_date = NaiveDate::parse_from_str(daily, "%Y%m%d")?;
    // let naive_dt = naive_date.and_hms(4, 0, 0); // 4PM EST
    // let offset = chrono::FixedOffset::west(5 * 3600); // EST TZ
    // let ts: DateTime<Utc> = chrono::DateTime::from_utc(naive_dt, TimeZone::from_offset(&offset));
    Ok(utc_dt.timestamp())
}

impl TryFrom<BarData> for Quote {
    type Error = anyhow::Error;
    fn try_from(bar: BarData) -> anyhow::Result<Self> {
        let timestamp = to_timestamp(&bar.date)?;
        Ok(Quote {
            timestamp,
            high: bar.high,
            low: bar.low,
            open: bar.open,
            close: bar.close,
            volume: bar.volume,
            avg: bar.average,
            count: bar.bar_count,
        })
    }
}

impl From<yahoo_finance_api::Quote> for Quote {
    fn from(yq: yahoo_finance_api::Quote) -> Self {
        Quote {
            timestamp: yq.timestamp as i64,
            high: yq.high,
            low: yq.low,
            open: yq.open,
            close: yq.adjclose, // accounts for splits AND dividends!
            volume: yq.volume as i64,
            avg: -1.0,
            count: -1,
        }
    }
}
