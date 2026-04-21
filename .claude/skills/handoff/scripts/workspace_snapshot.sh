#!/usr/bin/env bash
# Prints a compact, fenced markdown block describing the current workspace state.
# Designed to be inlined verbatim into §6 of the handoff packet.
# Degrades gracefully: outside a git repo, without git installed, or when
# sub-commands fail it still emits a block that clearly marks what is missing.

set -u

cwd="$(pwd)"

printf '```\n'
printf 'CWD: %s\n' "$cwd"

if ! command -v git >/dev/null 2>&1; then
  printf '(git not available)\n'
  printf '```\n'
  exit 0
fi

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  printf '(not a git repo)\n'
  printf '```\n'
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null || echo '(unavailable)')"
branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo '(unavailable)')"
head_sha="$(git rev-parse --short HEAD 2>/dev/null || echo '(unavailable)')"

printf 'Repo root: %s\n' "$repo_root"
printf 'Branch:    %s\n' "$branch"
printf 'HEAD:      %s\n' "$head_sha"

git_dir="$(git rev-parse --git-dir 2>/dev/null || true)"
progress=""
if [ -n "$git_dir" ]; then
  if [ -d "$git_dir/rebase-merge" ] || [ -d "$git_dir/rebase-apply" ]; then
    progress="${progress}rebase "
  fi
  if [ -f "$git_dir/MERGE_HEAD" ]; then
    progress="${progress}merge "
  fi
  if [ -f "$git_dir/CHERRY_PICK_HEAD" ]; then
    progress="${progress}cherry-pick "
  fi
  if [ -f "$git_dir/REVERT_HEAD" ]; then
    progress="${progress}revert "
  fi
  if [ -f "$git_dir/BISECT_LOG" ]; then
    progress="${progress}bisect "
  fi
fi
if [ -n "$progress" ]; then
  printf 'In progress: %s\n' "$progress"
fi

printf '\n-- git status --short --\n'
status_out="$(git status --short 2>/dev/null | head -n 30)"
if [ -z "$status_out" ]; then
  printf '(clean)\n'
else
  printf '%s\n' "$status_out"
fi

staged_stat="$(git diff --cached --shortstat 2>/dev/null | sed 's/^ //')"
unstaged_stat="$(git diff --shortstat 2>/dev/null | sed 's/^ //')"
printf '\n-- diff stats --\n'
printf 'Staged:   %s\n' "${staged_stat:-none}"
printf 'Unstaged: %s\n' "${unstaged_stat:-none}"

untracked_count="$(git ls-files --others --exclude-standard 2>/dev/null | wc -l | tr -d ' ')"
printf 'Untracked files: %s\n' "${untracked_count:-0}"

printf '\n-- recent commits --\n'
log_out="$(git log --oneline -n 5 2>/dev/null)"
if [ -z "$log_out" ]; then
  printf '(unavailable)\n'
else
  printf '%s\n' "$log_out"
fi

printf '```\n'
