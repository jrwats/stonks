PRAGMA foreign_keys=OFF;
BEGIN TRANSACTION;
CREATE TABLE IF NOT EXISTS ticker_exchange (
           ticker TEXT PRIMARY KEY NOT NULL,
           primary_exchange TEXT
         );
REPLACE INTO ticker_exchange VALUES('ABNB','NASDAQ');
REPLACE INTO ticker_exchange VALUES('APLS','NASDAQ');
REPLACE INTO ticker_exchange VALUES('AZPN','NASDAQ');
REPLACE INTO ticker_exchange VALUES('CASH','NASDAQ');
REPLACE INTO ticker_exchange VALUES('CAT','NYSE');
REPLACE INTO ticker_exchange VALUES('CDEV','NASDAQ');
REPLACE INTO ticker_exchange VALUES('CSCO','NASDAQ');
REPLACE INTO ticker_exchange VALUES('FANG','NASDAQ');
REPLACE INTO ticker_exchange VALUES('FBRT','NYSE');
REPLACE INTO ticker_exchange VALUES('FIVE','NASDAQ');
REPLACE INTO ticker_exchange VALUES('FRGE','NYSE');
REPLACE INTO ticker_exchange VALUES('GPRO','NASDAQ');
REPLACE INTO ticker_exchange VALUES('KEYS','NYSE');
REPLACE INTO ticker_exchange VALUES('LPLA','NASDAQ');
REPLACE INTO ticker_exchange VALUES('META','NASDAQ');
REPLACE INTO ticker_exchange VALUES('PLAY','NASDAQ');
REPLACE INTO ticker_exchange VALUES('ROLL','NASDAQ');
REPLACE INTO ticker_exchange VALUES('SMTC','NASDAQ');
REPLACE INTO ticker_exchange VALUES('SPCE','NYSE');
REPLACE INTO ticker_exchange VALUES('TREX','NYSE');
REPLACE INTO ticker_exchange VALUES('TVTY','NYSE');
REPLACE INTO ticker_exchange VALUES('WELL','NYSE');
REPLACE INTO ticker_exchange VALUES('WING','NASDAQ');
REPLACE INTO ticker_exchange VALUES('WLL','NYSE');
REPLACE INTO ticker_exchange VALUES('WNG','NYSE');
COMMIT;
