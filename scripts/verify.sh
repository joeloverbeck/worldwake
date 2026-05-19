#!/usr/bin/env bash
set -euo pipefail

# Disable incremental compilation for broad gates — full-workspace runs rebuild
# many crate × target × profile combinations, where incremental caches grow
# fast and amortize poorly. Interactive dev still gets incremental compilation
# via Cargo's normal defaults.
export CARGO_INCREMENTAL=0

echo "[verify] cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "[verify] cargo test --workspace"
cargo test --workspace

echo "[verify] bash scripts/check_active_goal_removed.sh"
bash scripts/check_active_goal_removed.sh

echo "[verify] bash scripts/check_no_artifact_state.sh"
bash scripts/check_no_artifact_state.sh

echo "[verify] bash scripts/check_no_debug_view_in_ai.sh"
bash scripts/check_no_debug_view_in_ai.sh

echo "[verify] cargo clippy --workspace"
cargo clippy --workspace

echo "[verify] cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "[verify] cargo run -p worldwake-cli --bin scenario-coverage -- --check"
cargo run -p worldwake-cli --bin scenario-coverage -- --check
