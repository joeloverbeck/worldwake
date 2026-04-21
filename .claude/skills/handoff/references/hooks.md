# Optional Companion Hooks

These hooks are OPT-IN. The `/handoff` skill does NOT modify `settings.json`. To enable, paste the snippets below into `.claude/settings.json` (project-local) or `~/.claude/settings.json` (user-level), then restart Claude Code.

## Why add them

Auto-compact can fire mid-session without manual `/handoff`. These hooks:

1. Save Claude's own `compact_summary` to a timestamped file whenever compaction happens, so the fact trail survives.
2. On a fresh session matched to `compact` resumption, inject ONLY a tiny pointer (`HANDOFF POINTER: .claude/handoffs/latest.md`) — NOT the full summary — keeping token burn near zero. The model then decides whether to `cat` the file.

If you invoke `/handoff` manually before every `/clear`, you don't need these. They're insurance for the forget-to-run-it case.

## Snippet: `PostCompact` hook

```json
{
  "hooks": {
    "PostCompact": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "mkdir -p .claude/handoffs && ts=$(date -u +%Y-%m-%dT%H%M%SZ) && printf '%s\\n' \"${CLAUDE_COMPACT_SUMMARY:-}\" > \".claude/handoffs/autocompact-${ts}.md\" && ln -sf \"autocompact-${ts}.md\" .claude/handoffs/latest.md"
          }
        ]
      }
    ]
  }
}
```

The environment variable name (`CLAUDE_COMPACT_SUMMARY`) is a placeholder — check your Claude Code version's hook documentation for the actual variable that exposes the compact summary. The `/update-config` skill or Claude Code docs are authoritative.

## Snippet: `SessionStart` hook with matcher `compact`

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "compact",
        "hooks": [
          {
            "type": "command",
            "command": "test -f .claude/handoffs/latest.md && printf 'HANDOFF POINTER: %s — read this file for full packet.\\n' '.claude/handoffs/latest.md'"
          }
        ]
      }
    ]
  }
}
```

This injects a single line, NOT the whole packet.

## Why not inject the full summary

Injecting the full summary on every compact-matched session restart doubles the steady-state token load and defeats the "low token burn" goal. The pointer pattern lets the model pull the packet only when it actually needs it — usually only on the first user turn after restart.
