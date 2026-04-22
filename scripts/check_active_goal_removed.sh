#!/usr/bin/env bash
set -euo pipefail

matches="$(
  rg -l \
    'ActiveGoal|get_component_active_goal|set_component_active_goal|has_component_active_goal|insert_component_active_goal|remove_component_active_goal|iter_active_goals|entities_with_active_goal|query_active_goal|count_with_active_goal' \
    crates/ 2>/dev/null || true
)"

if [ -n "$matches" ]; then
  echo "ActiveGoal references found in:" >&2
  echo "$matches" >&2
  exit 1
fi

echo "ActiveGoal removal verified: zero references in crates/"
