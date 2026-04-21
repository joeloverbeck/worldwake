---
name: handoff
description: Generate a minimal continuation packet for restarting work in a fresh Claude Code session
disable-model-invocation: true
effort: low
---

# Handoff

Produce a compact, operational **RESUME PACKET** from the current live session so a fresh Claude Code session can continue immediately — without relying on `/compact`.

## Invocation

Manual only: `/handoff`

## Workflow

1. **Snapshot the workspace.** Run `.claude/skills/handoff/scripts/workspace_snapshot.sh` from the repo root via Bash. Capture its output verbatim for §6. If the script is unavailable or fails, mark §6 as `(unavailable)` — do NOT fabricate workspace state.
2. **Load the template.** Read `references/template.md`. It defines the 16-section schema, in exact order, that the packet MUST use.
3. **Load the content rules.** Read `references/checklist.md` before filling sections. Covers fact/guess separation, anti-goals, and what NOT to restate.
4. **Extract from the live session.** Walk the current conversation and fill each section. Use `Unknown` or `None` when a fact is not present. Never invent.
5. **Emit the packet in chat.** Copy-paste-ready markdown, starting at the `# RESUME PACKET` header. No preamble, no closing commentary — just the packet.
6. **Offer to save.** After emitting, ask once: "Save to `.claude/handoffs/latest.md` and a timestamped file?" If yes, write both (`latest.md` overwritten; timestamped file named `YYYY-MM-DDTHHMMSSZ-<slug>.md`). Slug format: kebab-case, ≤40 chars, summarizes the primary task (e.g., `s114-reassess-and-skill-audit`, `observer-perf-triage`). Avoid dates and filler words like "session" or "work".

## Rules (short)

- Facts come from the live session or the snapshot. Guesses go in §10. Never blur them.
- If a section has nothing to report, write `None` or `Unknown`. Do not fabricate filler.
- Do not restate root `CLAUDE.md` conventions unless this session overrode them. Auto-memory will reload them.
- Nested `CLAUDE.md`, path-scoped rules, and previously-invoked skills go in §14 so the new session knows to reload them manually.
- No giant code excerpts. No raw logs. File paths, symbols, commands, and short quotes only.
- Full rule set: `references/checklist.md`.

## Worked examples

For packets covering all five acceptance scenarios (coding with failed tests, debugging with dead ends, nested rules, non-git directory, fresh restart), see `references/examples.md`.

## Optional companion hooks

`references/hooks.md` contains opt-in `PostCompact` and `SessionStart` (matcher `compact`) snippets. The skill does NOT auto-modify `settings.json`; the user pastes them manually if desired.
