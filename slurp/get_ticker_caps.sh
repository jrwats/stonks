#!/bin/bash

if [[ ! -f '.raw.csv' ]]; then
  curl https://companiesmarketcap.com/usa/largest-companies-in-the-usa-by-market-cap/?download=csv -o .raw.csv
fi

< .raw.csv python -c '
import csv, json, sys; 
print(json.dumps([dict(r) for r in csv.DictReader(sys.stdin)]))' | 
  jq -r '
    .[1:] | .[] | to_entries | .[] | .value | 
    select(.[3] | tonumber > 1000000000) | "\(.[2])\t\(.[3])"' |
    sed -r 's/^BRK-B/BRK B/;s/^BF-A/BF A/' | # IKBR-specific filter/renames
    tee ticker_mkt_cap.tsv |
    cut -d $'\t' -f1 | sort |
    comm -23 - unavailable.conf > tickers.list 

