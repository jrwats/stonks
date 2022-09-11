#!/bin/bash


tmp="$(mktemp -d)"
echo "$tmp" >&2

sqlite3 "$HOME/.local/stonks/db.sqlite3" 'SELECT DISTINCT ticker FROM daily' |
  cargo run --release trend-candidates --loose |
  tail -n +2 > "$tmp/out.tsv"

filename="Bounce $(date '+%F').txt"
mkdir -p "$HOME/watchlists"

awk '$2 == "false" {print $1}' "$tmp/out.tsv" | paste -sd , - | awk '{print "###Stocks," $0}' > "$HOME/watchlists/$filename"
awk '$2 == "true" {print $1}' "$tmp/out.tsv" | paste -sd , - | awk '{print "###Loose," $0}' >> "$HOME/watchlists/$filename"
