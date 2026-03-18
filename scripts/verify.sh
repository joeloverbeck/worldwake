#!/usr/bin/env bash
set -euo pipefail

strict=0
if [[ "${1-}" == "--strict" ]]; then
  strict=1
fi

echo "[verify] cargo test --workspace"
cargo test --workspace

echo "[verify] cargo clippy --workspace"
cargo clippy --workspace

if [[ "$strict" -eq 1 ]]; then
  echo "[verify] cargo clippy --workspace --all-targets -- -D warnings"
  cargo clippy --workspace --all-targets -- -D warnings
fi
