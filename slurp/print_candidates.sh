#!/bin/bash

set -euo pipefail

sqlite3 "$HOME/.local/stonks/db.sqlite3" < join.sql | 
  cargo run --release -- trend-candidates
