PRAGMA foreign_keys=OFF;
BEGIN TRANSACTION;
CREATE TABLE ticker_exchange (
           ticker TEXT PRIMARY KEY NOT NULL,
           primary_exchange TEXT
         );
INSERT INTO ticker_exchange VALUES('ABNB','NASDAQ');
INSERT INTO ticker_exchange VALUES('AZPN','NASDAQ');
INSERT INTO ticker_exchange VALUES('META','NASDAQ');
INSERT INTO ticker_exchange VALUES('FANG','NASDAQ');
INSERT INTO ticker_exchange VALUES('APLS','NASDAQ');
INSERT INTO ticker_exchange VALUES('CASH','NASDAQ');
INSERT INTO ticker_exchange VALUES('CAT','NYSE');
INSERT INTO ticker_exchange VALUES('FIVE','NASDAQ');
INSERT INTO ticker_exchange VALUES('CSCO','NASDAQ');
INSERT INTO ticker_exchange VALUES('FRGE','NYSE');
INSERT INTO ticker_exchange VALUES('LPLA','NASDAQ');
INSERT INTO ticker_exchange VALUES('KEYS','NYSE');
INSERT INTO ticker_exchange VALUES('ROLL','NASDAQ');
INSERT INTO ticker_exchange VALUES('SMTC','NASDAQ');
INSERT INTO ticker_exchange VALUES('SPCE','NYSE');
INSERT INTO ticker_exchange VALUES('TREX','NYSE');
INSERT INTO ticker_exchange VALUES('TVTY','NYSE');
INSERT INTO ticker_exchange VALUES('WELL','NYSE');
INSERT INTO ticker_exchange VALUES('WING','NASDAQ');
INSERT INTO ticker_exchange VALUES('WNG','NYSE');
INSERT INTO ticker_exchange VALUES('WLL','NYSE');
INSERT INTO ticker_exchange VALUES('FBRT','NYSE');
INSERT INTO ticker_exchange VALUES('PLAY','NASDAQ');
COMMIT;
