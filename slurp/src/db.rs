use rusqlite::{params, Connection, Transaction};
use std::env;
use std::path::{Path, PathBuf};
use yahoo_finance_api as yahoo;

const DEFAULT_FILE: &str = ".local/stonks/db.sqlite3";

use crate::quote::Quote;

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

    for ema_period in [8, 21, 34, 89] {
        let sql = format!(
            r#"CREATE TABLE IF NOT EXISTS ema_{} (
           daily_id INTEGER,
           value REAL,
           FOREIGN KEY ([daily_id]) REFERENCES "daily" ([id])
         )"#,
            ema_period,
        );
        conn.execute(&sql, [])?;
    }

    for sma_period in [50, 200] {
        let sql = format!(
            r#"CREATE TABLE IF NOT EXISTS sma_{} (
           daily_id INTEGER,
           value REAL,
           FOREIGN KEY ([daily_id]) REFERENCES "daily" ([id])
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

// struct QuoteWriter<'a> {
//     tx: Transaction<'a>,
//     stmt:
// }
//
// impl QuoteWriter<'a> {
//     pub fn insert_row(q: &Quote) -> anyhow::Result<()> {
//     }
// }

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

    pub fn insert_daily_quotes(
        &mut self,
        ticker: &str,
        daily_quotes: &[Quote],
    ) -> anyhow::Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO daily 
                  (ticker, timestamp, high, low, open, close, avg, volume, count) 
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
    //
    //     pub fn get_writer() -> QuoteWriter {
    //     }
}
