#!/bin/bash

if (( $# != 1 )); then 
  echo 'Usage slurp_ticker.sh "$TICKER"' >&2
  exit 1
fi

sqlite3 -tabs "$HOME/.local/stonks/db.sqlite3" "SELECT
  timestamp,
  close,
  e8.value   as ema_8,
  e21.value  as ema_21,
  e34.value  as ema_34,
  e89.value  as ema_89,
  s50.value  as sma_50,
  s200.value  as sma_200
  FROM daily d
  LEFT OUTER JOIN ema_8 e8     ON d.id = e8.daily_id
  LEFT OUTER JOIN ema_21 e21   ON d.id = e21.daily_id
  LEFT OUTER JOIN ema_34 e34   ON d.id = e34.daily_id
  LEFT OUTER JOIN ema_89 e89   ON d.id = e89.daily_id
  LEFT OUTER JOIN sma_50 s50   ON d.id = s50.daily_id
  LEFT OUTER JOIN sma_200 s200 ON d.id = s200.daily_id
  WHERE ticker = \"$1\"
  ORDER BY timestamp"
