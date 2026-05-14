#!/usr/bin/env bash
set -euo pipefail

# Run the canonical verification gates with settings that reduce local target/
# growth on WSL2/VM disks. This is for local development only; CI should keep
# calling scripts/verify.sh directly unless CI disk pressure becomes a problem.
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
export CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-line-tables-only}"
export CARGO_PROFILE_TEST_DEBUG="${CARGO_PROFILE_TEST_DEBUG:-line-tables-only}"

exec "$(dirname "$0")/verify.sh"
