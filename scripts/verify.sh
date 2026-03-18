#!/usr/bin/env bash
set -euo pipefail

echo "[verify] cargo test --workspace"
cargo test --workspace

echo "[verify] cargo clippy --workspace"
cargo clippy --workspace

echo "[verify] cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings
