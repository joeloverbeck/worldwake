#!/usr/bin/env bash
# Golden test performance harness — measures combined wall time of 5 slowest suites.
# Output format: combined_duration_ms=XXXXX pass=YY tests=ZZ
# Intermediate output per suite for partial signal tracking and early abort.
set -euo pipefail

SUITES=(
    golden_determinism
    golden_trade
    golden_care
    golden_supply_chain
    golden_production
)

total_ms=0
total_pass=0
total_tests=0
best_ms="${BEST_MS:-999999999}"
abort_threshold="${ABORT_THRESHOLD:-0.05}"

for suite in "${SUITES[@]}"; do
    # Run suite and capture output + timing
    start_ns=$(date +%s%N)
    output=$(cargo test -p worldwake-ai --test "$suite" -- --test-threads=1 2>&1) || {
        echo "HARNESS_ERROR: $suite failed"
        exit 1
    }
    end_ns=$(date +%s%N)

    # Compute wall time in milliseconds
    elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))

    # Parse test counts from "test result: ok. X passed; Y failed; ..."
    passed=$(echo "$output" | grep -oP '\d+ passed' | grep -oP '\d+' || echo 0)
    tests_run=$(echo "$output" | grep -oP 'filtered out; finished' | wc -l || echo 0)
    # Total tests = passed (since we exit on failure)
    suite_tests=$passed

    total_ms=$((total_ms + elapsed_ms))
    total_pass=$((total_pass + passed))
    total_tests=$((total_tests + suite_tests))

    # Intermediate output for partial signal tracking
    echo "intermediate: suite=$suite duration_ms=$elapsed_ms pass=$passed tests=$suite_tests"

    # Early abort check: if running total already exceeds best by abort_threshold
    abort_limit=$(echo "$best_ms * (1 + $abort_threshold)" | bc -l | cut -d. -f1)
    if [ "$total_ms" -gt "$abort_limit" ] 2>/dev/null; then
        echo "EARLY_ABORT: running total ${total_ms}ms exceeds abort limit ${abort_limit}ms after $suite"
        echo "combined_duration_ms=$total_ms pass=$total_pass tests=$total_tests"
        exit 0
    fi
done

echo "combined_duration_ms=$total_ms pass=$total_pass tests=$total_tests"
