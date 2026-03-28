#!/usr/bin/env bash
# Golden test performance harness — dynamically discovers and measures the 5 slowest suites.
# Output format: combined_duration_ms=XXXXX pass=YY tests=ZZ
# Intermediate output per suite for partial signal tracking.
set -euo pipefail

TOP_N=5

# Discover all golden test suites by listing test binaries.
mapfile -t ALL_SUITES < <(
    find crates/worldwake-ai/tests -maxdepth 1 -name 'golden_*.rs' -printf '%f\n' \
        | sed 's/\.rs$//' \
        | sort
)

if [ "${#ALL_SUITES[@]}" -eq 0 ]; then
    echo "HARNESS_ERROR: no golden_*.rs test files found"
    exit 1
fi

# Run every suite once and record per-suite timing.
declare -A suite_ms suite_pass suite_tests
total_pass=0
total_tests=0

for suite in "${ALL_SUITES[@]}"; do
    start_ns=$(date +%s%N)
    output=$(cargo test -p worldwake-ai --test "$suite" -- --test-threads=1 2>&1) || {
        echo "HARNESS_ERROR: $suite failed"
        exit 1
    }
    end_ns=$(date +%s%N)

    elapsed_ms=$(( (end_ns - start_ns) / 1000000 ))

    passed=$(echo "$output" | grep -oP '\d+ passed' | grep -oP '\d+' || echo 0)
    suite_t=$passed

    suite_ms[$suite]=$elapsed_ms
    suite_pass[$suite]=$passed
    suite_tests[$suite]=$suite_t

    total_pass=$((total_pass + passed))
    total_tests=$((total_tests + suite_t))

    echo "intermediate: suite=$suite duration_ms=$elapsed_ms pass=$passed tests=$suite_t"
done

# Rank suites by duration descending and pick the top N.
mapfile -t ranked < <(
    for suite in "${ALL_SUITES[@]}"; do
        echo "${suite_ms[$suite]} $suite"
    done | sort -rn | head -n "$TOP_N"
)

combined_ms=0
combined_pass=0
combined_tests=0
top_suites=()
for entry in "${ranked[@]}"; do
    ms=${entry%% *}
    suite=${entry#* }
    combined_ms=$((combined_ms + ms))
    combined_pass=$((combined_pass + suite_pass[$suite]))
    combined_tests=$((combined_tests + suite_tests[$suite]))
    top_suites+=("$suite")
done

echo "top_${TOP_N}_suites=${top_suites[*]}"
echo "combined_duration_ms=$combined_ms pass=$combined_pass tests=$combined_tests"
