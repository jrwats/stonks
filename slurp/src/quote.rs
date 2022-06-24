use chrono::prelude::*;
use ibtwsapi::core::common::BarData;

pub struct Quote {
    timestamp: i64,
    high: f64,
    low: f64,
    open: f64,
    close: f64,
    volume: i64,
    avg: f64, // weighted avg price
    count: i32 // number of trades during the bar's timespan (day)
}

fn to_timestamp(daily: &str) -> anyhow::Result<i64> {
    // "20220623" => 1654781400
    let naive_date = NaiveDate::parse_from_str(daily, "%Y%m%d")?;
    let naive_dt = naive_date.and_hms(4, 0, 0); // 4PM EST
    let offset = chrono::FixedOffset::west(5 * 3600); // EST TZ
    let ts: DateTime<Utc> = chrono::DateTime::from_utc(naive_dt, TimeZone::from_offset(&offset));
    Ok(ts.timestamp())
}

impl Quote {
    fn try_from_daily_ib_bar(bar: &BarData) -> anyhow::Result<Self> {
        let timestamp = to_timestamp(&bar.date)?;
        Ok(Quote {
            timestamp,
            high: bar.high,
            low: bar.low,
            open: bar.open,
            close: bar.close,
            volume: bar.volume,
            avg: bar.average,
            count: bar.bar_count
        })
    }
}

