#!/usr/bin/env bash
set -euo pipefail

echo "[checks] Running full workspace tests..."
cargo test --workspace

echo "[checks] Running strict workspace clippy..."
cargo clippy --workspace --all-targets -- -D warnings

echo "[checks] Running focused one-seed soak verification..."
cargo run --release -p worldwake-ai --bin soak_seed_perf -- 0

echo "[checks] All correctness checks passed."
