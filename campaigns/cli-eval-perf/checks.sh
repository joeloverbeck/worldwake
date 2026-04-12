#!/usr/bin/env bash
set -euo pipefail

echo "[checks] Running full workspace tests..."
cargo test --workspace

echo "[checks] Running strict workspace clippy..."
cargo clippy --workspace --all-targets -- -D warnings

echo "[checks] All correctness checks passed."
