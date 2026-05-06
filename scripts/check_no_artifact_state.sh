#!/usr/bin/env bash
set -euo pipefail

matches="$(
  rg -l '\bArtifactState\b' crates/ 2>/dev/null || true
)"

if [ -n "$matches" ]; then
  echo "ArtifactState references found in:" >&2
  echo "$matches" >&2
  exit 1
fi

echo "ArtifactState removal verified: zero references in crates/"
