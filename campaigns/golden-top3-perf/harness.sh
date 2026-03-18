#!/usr/bin/env bash
set -euo pipefail

# golden-top3-perf harness
# Measures combined wall time of the 3 slowest golden test suites.
# Exits 0 if all pass, 1 if any fail.

SUITES=(golden_determinism golden_production golden_combat)
TOTAL_MS=0
PASS_COUNT=0
TEST_COUNT=${#SUITES[@]}

# Pre-compile all test binaries (outside timer)
echo "Pre-compiling test binaries..."
cargo test -p worldwake-ai --no-run 2>&1 | tail -1

for suite in "${SUITES[@]}"; do
    START_NS=$(date +%s%N)
    if cargo test -p worldwake-ai --test "$suite" -- --test-threads=1 2>&1 | tail -3; then
        STATUS="PASS"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        STATUS="FAIL"
    fi
    END_NS=$(date +%s%N)
    ELAPSED_MS=$(( (END_NS - START_NS) / 1000000 ))
    TOTAL_MS=$((TOTAL_MS + ELAPSED_MS))
    echo "intermediate_ms=${ELAPSED_MS} suite=${suite} status=${STATUS}"
done

echo "combined_duration_ms=${TOTAL_MS} pass=${PASS_COUNT} tests=${TEST_COUNT}"

if [ "$PASS_COUNT" -eq "$TEST_COUNT" ]; then
    exit 0
else
    exit 1
fi
