#!/usr/bin/env bash
set -euo pipefail

echo "[verify] cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "[verify] cargo test --workspace"
cargo test --workspace

echo "[verify] bash scripts/check_active_goal_removed.sh"
bash scripts/check_active_goal_removed.sh

echo "[verify] bash scripts/check_no_artifact_state.sh"
bash scripts/check_no_artifact_state.sh

echo "[verify] cargo clippy --workspace"
cargo clippy --workspace

echo "[verify] cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "[verify] cargo run -p worldwake-cli --bin scenario-coverage -- --check"
cargo run -p worldwake-cli --bin scenario-coverage -- --check
