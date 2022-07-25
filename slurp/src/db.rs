use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension};
use std::env;
use std::path::{Path, PathBuf};

const DEFAULT_FILE: &str = ".local/stonks/db.sqlite3";

use crate::quote::Quote;

pub const SMA_WINDOWS: [usize; 2] = [50, 200];
pub const EMA_WINDOWS: [usize; 4] = [8, 21, 34, 89];

#[derive(Debug)]
pub struct QuoteRow {
    pub id: i32,
    pub quote: Quote,
}

#[derive(Debug)]
pub struct MetricRow {
    pub id: i32,
    pub quote: Quote,
    pub metrics: Metrics,
}

#[derive(Debug)]
pub struct Metrics {
    pub ema_8: Option<f64>,
    pub ema_21: Option<f64>,
    pub ema_34: Option<f64>,
    pub ema_89: Option<f64>,
    pub sma_50: Option<f64>,
    pub sma_200: Option<f64>,
}

pub struct Db {
    conn: Connection,
}

fn init_tables(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS daily (
           id INTEGER PRIMARY KEY NOT NULL,
           ticker TEXT,
           timestamp INTEGER,
           high REAL,
           low REAL,
           open REAL,
           close REAL,
           avg REAL,
           adjclose REAL,
           volume INTEGER,
           count INTEGER
         )",
        [],
    )?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS daily_idx ON daily (ticker, timestamp)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS ticker_exchange (
           ticker TEXT PRIMARY KEY NOT NULL,
           primary_exchange TEXT
         )",
        [],
    )?;

    for ema_period in EMA_WINDOWS {
        let sql = format!(
            r#"CREATE TABLE IF NOT EXISTS ema_{} (
           daily_id INTEGER PRIMARY KEY NOT NULL,
           value REAL,
           FOREIGN KEY ([daily_id]) REFERENCES "daily" ([id]) ON DELETE CASCADE
         )"#,
            ema_period,
        );
        conn.execute(&sql, [])?;
    }

    for sma_period in SMA_WINDOWS {
        let sql = format!(
            r#"CREATE TABLE IF NOT EXISTS sma_{} (
           daily_id INTEGER PRIMARY KEY NOT NULL,
           value REAL,
           FOREIGN KEY ([daily_id]) REFERENCES "daily" ([id]) ON DELETE CASCADE
         )"#,
            sma_period,
        );
        conn.execute(&sql, [])?;
    }
    Ok(())
}

fn ensure_parent(db_path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

fn row_to_quote(row: &rusqlite::Row) -> rusqlite::Result<QuoteRow> {
    let id: i32 = row.get(0)?;
    let timestamp: i64 = row.get(1)?;
    let open: f64 = row.get(2)?;
    let close: f64 = row.get(3)?;
    let high: f64 = row.get(4)?;
    let low: f64 = row.get(5)?;
    let avg: f64 = row.get(6)?;
    let volume: i64 = row.get(7)?;
    let count: i32 = row.get(8)?;
    let quote = Quote {
        timestamp,
        open,
        close,
        high,
        low,
        avg,
        volume,
        count,
    };
    Ok(QuoteRow { id, quote })
}

impl Db {
    pub fn init(file: Option<PathBuf>) -> anyhow::Result<Self> {
        let home = env::var("HOME")?;
        let db_path = file.unwrap_or(PathBuf::from(home).join(DEFAULT_FILE));
        ensure_parent(&db_path)?;
        let mut conn = Connection::open(&db_path)?;
        init_tables(&mut conn)?;
        let instance = Db { conn };
        Ok(instance)
    }

    pub fn get_exchange(&self, ticker: &str) -> anyhow::Result<Option<String>> {
        let row: Option<String> = self
            .conn
            .query_row(
                "SELECT primary_exchange FROM ticker_exchange WHERE ticker = ?",
                [ticker],
                |row| row.get(0),
            )
            .optional()?;
        Ok(row)
    }

    pub fn get_all_daily_quotes(&self, ticker: &str) -> anyhow::Result<Vec<QuoteRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, open, close, high, low, avg, volume, count
         FROM daily
         WHERE ticker = ?
         ORDER BY timestamp ASC",
        )?;
        let mut rows = stmt.query([ticker])?;
        let mut result = vec![];
        while let Some(row) = rows.next()? {
            result.push(row_to_quote(row)?);
        }
        Ok(result)
    }

    pub fn get_last_quote(&self, ticker: &str) -> anyhow::Result<QuoteRow> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, open, close, high, low, avg, volume, count
         FROM daily
         WHERE ticker = ?
         ORDER BY timestamp DESC
         LIMIT 1",
        )?;
        let quote_row = stmt
            .query_row([ticker], |row| Ok(row_to_quote(row)?))
            .with_context(|| format!("No row for {}", ticker))?;
        Ok(quote_row)
    }

    pub fn get_quote_with_timestamp(
        &self,
        ticker: &str,
        timestamp: i64,
    ) -> anyhow::Result<Option<QuoteRow>> {
        eprintln!("{} {}", ticker, timestamp);
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, open, close, high, low, avg, volume, count
             FROM daily
             WHERE ticker = ? AND timestamp = ?",
        )?;
        let quote_row = stmt
            .query_row(params![ticker, timestamp], |row| Ok(row_to_quote(row)?))
            .optional()?;
        Ok(quote_row)
    }

    pub fn get_metrics_for_ticker(
        &self,
        ticker: &str,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<MetricRow>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT * FROM (
              SELECT
                id, timestamp, open, close, high, low, avg, volume, count,
                e8.value as ema_8, e21.value as ema_21, e34.value as ema_34,
                e89.value as ema_89, s50.value as sma_50, s200.value as sma_200
              FROM daily d
              LEFT OUTER JOIN ema_8 e8     ON d.id = e8.daily_id
              LEFT OUTER JOIN ema_21 e21   ON d.id = e21.daily_id
              LEFT OUTER JOIN ema_34 e34   ON d.id = e34.daily_id
              LEFT OUTER JOIN ema_89 e89   ON d.id = e89.daily_id
              LEFT OUTER JOIN sma_50 s50   ON d.id = s50.daily_id
              LEFT OUTER JOIN sma_200 s200 ON d.id = s200.daily_id
              WHERE ticker = ?
              ORDER BY timestamp DESC
              LIMIT ?
            ) ORDER BY timestamp ASC",
        )?;
        let mut rows = stmt.query(params![ticker, limit.unwrap_or(9999)])?;
        let mut result = vec![];
        while let Some(row) = rows.next()? {
            let id: i32 = row.get(0)?;
            let timestamp: i64 = row.get(1)?;
            let open: f64 = row.get(2)?;
            let close: f64 = row.get(3)?;
            let high: f64 = row.get(4)?;
            let low: f64 = row.get(5)?;
            let avg: f64 = row.get(6)?;
            let volume: i64 = row.get(7)?;
            let count: i32 = row.get(8)?;
            let quote = Quote {
                timestamp,
                open,
                close,
                high,
                low,
                avg,
                volume,
                count,
            };
            let ema_8: Option<f64> = row.get(9)?;
            let ema_21: Option<f64> = row.get(10)?;
            let ema_34: Option<f64> = row.get(11)?;
            let ema_89: Option<f64> = row.get(12)?;
            let sma_50: Option<f64> = row.get(13)?;
            let sma_200: Option<f64> = row.get(14)?;
            let metrics = Metrics {
                ema_8,
                ema_21,
                ema_34,
                ema_89,
                sma_50,
                sma_200,
            };
            result.push(MetricRow { id, quote, metrics });
        }
        Ok(result)
    }

    pub fn insert_daily_quotes(
        &mut self,
        ticker: &str,
        daily_quotes: &[Quote],
    ) -> anyhow::Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO daily
                  (ticker, timestamp, open, close, high, low, avg, volume, count)
                VALUES
                  (?,      ?,         ?,    ?,   ?,    ?,     ?,   ?,      ?)",
            )?;
            for quote in daily_quotes {
                stmt.execute(params![
                    ticker,
                    quote.timestamp,
                    &quote.open,
                    &quote.close,
                    &quote.high,
                    &quote.low,
                    &quote.avg,
                    &quote.volume,
                    &quote.count
                ])?;
            }
        }
        Ok(tx.commit()?)
    }

    pub fn insert_calculations(
        &mut self,
        table: &str,
        values: &[(i32, f64)],
    ) -> anyhow::Result<()> {
        let tx = self.conn.transaction()?;
        {
            let fmt_stmt = format!(
                "INSERT OR REPLACE INTO {} (daily_id, value) VALUES (?, ?)",
                table
            );
            let mut stmt = tx.prepare(&fmt_stmt)?;
            for row in values {
                stmt.execute(params![row.0, row.1])?;
            }
        }
        Ok(tx.commit()?)
    }
}
