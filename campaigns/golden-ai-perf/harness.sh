#!/usr/bin/env bash
set -euo pipefail

# Pre-compile both test binaries (outside timer)
echo "Pre-compiling test binaries..."
cargo test -p worldwake-ai --test golden_trade --no-run 2>&1
cargo test -p worldwake-ai --test golden_production --no-run 2>&1
echo "Compilation complete."

passed=0
total=4
combined_ms=0

tests=(
  "golden_trade golden_merchant_restock_return_stock"
  "golden_trade golden_merchant_restock_return_stock_replays_deterministically"
  "golden_production golden_resource_exhaustion_race"
  "golden_production golden_resource_exhaustion_race_replays_deterministically"
)

for entry in "${tests[@]}"; do
  binary="${entry%% *}"
  name="${entry##* }"

  start_ns=$(date +%s%N)
  if cargo test -p worldwake-ai --test "$binary" -- "$name" --exact 2>&1; then
    end_ns=$(date +%s%N)
    elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))
    combined_ms=$(( combined_ms + elapsed_ms ))
    passed=$(( passed + 1 ))
    echo "intermediate_ms=${elapsed_ms} test=${name}"
  else
    end_ns=$(date +%s%N)
    elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))
    combined_ms=$(( combined_ms + elapsed_ms ))
    echo "intermediate_ms=${elapsed_ms} test=${name} FAILED"
  fi
done

echo "combined_duration_ms=${combined_ms} pass=${passed} tests=${total}"

if [ "$passed" -eq "$total" ]; then
  exit 0
else
  exit 1
fi
