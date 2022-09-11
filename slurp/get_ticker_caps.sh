#!/bin/bash

if [[ ! -f '.raw.csv' ]]; then
  curl https://companiesmarketcap.com/usa/largest-companies-in-the-usa-by-market-cap/?download=csv -o .raw.csv
fi

< .raw.csv python3 -c '
import csv, json, sys; 
print(json.dumps([dict(r) for r in csv.DictReader(sys.stdin)]))' | 
  jq -r '.[1:] | .[] | select(.marketcap | tonumber > 1000000000) | "\(.Symbol)\t\(.marketcap)"' |
    sed -r '/^ACC/d;s/^BRK-B/BRK B/;s/TVTY/NLSN/;s/^BF-A/BF A/' | # IKBR-specific filter/renames
    tee ticker_mkt_cap.tsv |
    cut -d $'\t' -f1 | sort |
    comm -23 - unavailable.conf > tickers.list
