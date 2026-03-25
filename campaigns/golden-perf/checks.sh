#!/usr/bin/env bash
# Correctness guard for golden-perf campaign.
# Ensures optimizations don't break functionality.
set -euo pipefail

echo "[checks] Running full workspace tests..."
cargo test --workspace

echo "[checks] Running clippy with strict warnings..."
cargo clippy --workspace --all-targets -- -D warnings

echo "[checks] All correctness checks passed."
