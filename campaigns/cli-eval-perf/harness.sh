#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# Pre-build to exclude compilation time from measurement.
# Debug mode: matches simulation-observer skill workflow (the actual pain point).
# Release mode completes in ~21s; debug mode is ~330s due to unoptimized
# allocation/cloning in the planner trace infrastructure.
cargo build -p worldwake-cli --bin observer 2>/dev/null

start_ms=$(($(date +%s%N) / 1000000))
cargo run -p worldwake-cli --bin observer -- \
  scenarios/cli-evaluation.ron --ticks 1440 --output /dev/null 2>/dev/null
end_ms=$(($(date +%s%N) / 1000000))

duration_ms=$((end_ms - start_ms))
echo "duration_ms=${duration_ms}"
