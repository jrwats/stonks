use anyhow::Context;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

const DEFAULT_FILE: &str = ".local/stonks/db.sqlite3";

use crate::quote::Quote;

#[derive(Debug)]
pub struct Calculations {
    pub table: String,
    pub values: Vec<(i32, f64)>,
}

#[derive(Debug)]
pub struct QuoteRow {
    pub id: i32,
    pub quote: Quote,
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

    /*
    pub fn get_all_daily_quotes(&self, ticker: &str) -> anyhow::Result<Vec<QuoteRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, open, close, high, low, avg, volume, count
         FROM daily
         WHERE ticker = ?
         ORDER BY timestamp ASC",
        )?;
        let mut rows = stmt.query([ticker])?;
        // rows.try_into
        let mut result = vec![];
        while let Some(row) = rows.next()? {
            result.push(row_to_quote(row)?);
        }
        Ok(result)
    }
    */

    pub fn get_daily_batch(
        &self,
        tickers: &[String],
    ) -> anyhow::Result<BTreeMap<String, Vec<QuoteRow>>> {
        let mut vars = "?,".repeat(tickers.len());
        vars.pop();
        let sql = format!(
            "SELECT id, timestamp, open, close, high, low, avg, volume, count, ticker
             FROM daily
             WHERE ticker IN ({})
             ORDER BY timestamp ASC",
            vars,
        );
        let mut stmt = self.conn.prepare(&sql)?;

        let mut rows = stmt.query(rusqlite::params_from_iter(tickers))?;
        let mut sym2quotes: BTreeMap<String, Vec<QuoteRow>> = BTreeMap::new();
        while let Some(row) = rows.next()? {
            let ticker: String = row.get(9)?;
            let quote = row_to_quote(row)?;
            sym2quotes.entry(ticker).or_default().push(quote);
        }
        Ok(sym2quotes)
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

    /*
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
    */
}
