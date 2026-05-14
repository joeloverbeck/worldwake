#!/usr/bin/env bash
set -euo pipefail

matches="$(
  rg -l 'DebugWorldView' crates/worldwake-ai/src 2>/dev/null || true
)"

if [ -n "$matches" ]; then
  echo "DebugWorldView illegally imported in worldwake-ai:" >&2
  echo "$matches" >&2
  exit 1
fi

echo "DebugWorldView import check verified: zero references in crates/worldwake-ai/src"
