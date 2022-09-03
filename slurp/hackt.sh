#!/bin/bash

set -uo pipefail

# Worrkaround for populating the DB when it seems to silently do nothing
# Likely bug is TWS client isn't seeing messages from TWS

tofetch="$(mktemp -p /tmp tofetch_XXX)"
todo() {
  comm -13 <(sqlite3 ~/.local/stonks/db.sqlite3 'SELECT DISTINCT(ticker) FROM daily' | sort) tickers.list
}

todo > "$tofetch"
while [[ -s "$tofetch" ]]; do
  echo "$(wc -l /tmp/tofetch.list) tickers" >&2
  timeout 120 </tmp/tofetch.list cargo run --release -- "$@" full
  exit_code=$?
  echo "exit code: $exit_code" >&2
  todo > "$tofetch"
done
