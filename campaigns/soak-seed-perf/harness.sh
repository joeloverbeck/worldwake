#!/usr/bin/env bash
set -euo pipefail

CAMPAIGN_DIR="$(cd "$(dirname "$0")" && pwd)"
RESULTS_TSV="$CAMPAIGN_DIR/results.tsv"
SEEDS=(0 1 2 3 4)

if [ ! -f "$RESULTS_TSV" ]; then
  echo "HARNESS_ERROR: missing results.tsv"
  exit 1
fi

line_count=$(wc -l < "$RESULTS_TSV")
experiment_count=$(( line_count > 0 ? line_count - 1 : 0 ))
seed_index=$(( experiment_count % ${#SEEDS[@]} ))
seed_id=${SEEDS[$seed_index]}

start_ns=$(date +%s%N)
cargo run --release -p worldwake-ai --bin soak_seed_perf -- "$seed_id" \
  >/tmp/soak-seed-perf-harness.log 2>&1 || {
    cat /tmp/soak-seed-perf-harness.log
    echo "HARNESS_ERROR: one-seed soak run failed for seed_id=$seed_id"
    exit 1
  }
end_ns=$(date +%s%N)

runner_duration_ms=$(grep -oP '^duration_ms=\K\d+' /tmp/soak-seed-perf-harness.log | tail -n 1 || true)
if [ -z "$runner_duration_ms" ]; then
  cat /tmp/soak-seed-perf-harness.log
  echo "HARNESS_ERROR: soak_seed_perf runner did not emit duration_ms"
  exit 1
fi

wrapper_duration_ms=$(( (end_ns - start_ns) / 1000000 ))

echo "seed_id=$seed_id"
echo "duration_ms=$runner_duration_ms"
echo "wrapper_duration_ms=$wrapper_duration_ms"
echo "seed_cycle_index=$seed_index"
