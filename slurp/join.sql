SELECT
  ticker
  FROM daily d
  LEFT OUTER JOIN ema_8 e8     ON d.id = e8.daily_id
  LEFT OUTER JOIN ema_21 e21   ON d.id = e21.daily_id
  LEFT OUTER JOIN ema_34 e34   ON d.id = e34.daily_id
  LEFT OUTER JOIN ema_89 e89   ON d.id = e89.daily_id
  LEFT OUTER JOIN sma_50 s50   ON d.id = s50.daily_id
  LEFT OUTER JOIN sma_200 s200 ON d.id = s200.daily_id
  WHERE 
    timestamp > (strftime('%s', 'now') - 3 * 86400) 
    AND ((e8.value < e21.value AND e21.value < e34.value AND e34.value < e89.value) OR
         (e8.value > e21.value AND e21.value > e34.value AND e34.value > e89.value))
  GROUP BY ticker
