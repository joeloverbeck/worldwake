# Context Audit Handoff

This file lets a fresh Claude Code session pick up a prior context-cleanup effort without redoing
the analysis. The user will paste the **Prompt to paste** section below into the new session.

---

## Prompt to paste into fresh session

> Read `.claude/CONTEXT-AUDIT-HANDOFF.md` in this repo. That file captures what was already
> removed from global/project Claude Code configuration in a prior session. Then run a fresh
> context audit:
>
> 1. Run `/context` (the user will show you the output) and enumerate what is currently loaded:
>    system tools, user agents, plugin agents, memory files, skills, MCP servers.
> 2. For each item >50 tokens, apply the "truly global vs. repo-specific" test — is it useful in
>    any Rust/Cargo/CLI repo? Is it likely useful in at least some of this user's other repos?
> 3. Surface any NEW waste not already addressed (the prior cleanup is listed in this file — do
>    not re-flag those items). Look especially for:
>    - Plugin agents tied to languages/stacks the user doesn't use
>    - Duplicate imports or dead `@import` references in `~/.claude/CLAUDE.md`
>    - Skill descriptions >500 tokens that could be shortened
>    - Recently-added plugins/agents since the prior audit
> 4. Report as a sortable table (action, target, tokens, rationale) with explicit "remove /
>    flag / keep" recommendations. Under auto mode, apply safe reversible removals
>    (moves to `.disabled/` backup dirs) and ask before deleting.
>
> Expected baseline after prior cleanup: ~45–50k context vs. 91k pre-cleanup. If `/context`
> shows substantially more than 50k, we missed something.

---

## What the prior session already did (don't redo)

### Project-level changes (in this repo)
- **`.claude/settings.json`** created with these plugins disabled for Worldwake:
  - `frontend-design@claude-code-plugins` — no frontend in this project
  - `typescript-lsp@claude-plugins-official` — no TS
  - `voltagent-core-dev@voltagent-subagents` — frontend/mobile/electron/graphql/websocket
  - `voltagent-lang@voltagent-subagents` — 26 language agents; only `rust-engineer` was relevant and
    user confirmed never invoking it explicitly

### Global-level changes (affect all projects)
- **SuperClaude framework uninstalled** by the user (~35k tokens of memory files + ~17 agents gone).
- **`voltagent-research@voltagent-subagents` plugin disabled globally** via `/plugin` menu (~700 tokens).
- **`~/.claude/CLAUDE.md` cleaned** — removed 37 lines of dead `@import`s (SuperClaude files no longer
  exist). Current file contains only the "Workflow Contracts" section.
- **9 user agents moved** to `~/.claude/agents.disabled/` (reversible with `mv`):
  - Language/tool-specific: `build-error-resolver` (TS), `e2e-runner` (Playwright/Vercel),
    `go-build-resolver`, `go-reviewer`, `refactor-cleaner` (knip/depcheck/ts-prune),
    `test-failure-resolver` (Jest)
  - Convention-specific: `doc-updater` (docs/CODEMAPS/), `spec-assumption-validator` (superseded by
    project's `/reassess-spec` skill), `workflow-alignment-validator` (workflows/ folder convention
    not used here)
- **Remaining 5 user agents** (all truly global): `architect`, `code-reviewer`, `planner`,
  `security-reviewer`, `tdd-guide`

### Custom statusLine (unrelated to audit but part of the session)
- `~/.claude/statusline-command.sh` rewritten to show cost + delta + 5h + 7d rate limits with color
  thresholds. Original backed up to `~/.claude/statusline-command.sh.backup`.
- State file pattern: `/tmp/claude-statusline/${session_id}.cost` for cost-delta tracking.

---

## Second-pass audit (2026-04-21)

### Project-level changes (in this repo)
- Added 8 more plugin disables to `.claude/settings.json`:
  - `voltagent-qa-sec@voltagent-subagents` (~1.4k tokens, 14 agents)
  - `voltagent-meta@voltagent-subagents` (~1.0k tokens, 10 agents)
  - `voltagent-dev-exp@voltagent-subagents` (~1.4k tokens, 13 agents — user confirmed never used)
  - `hookify@claude-plugins-official` (~150 tokens, 5 skills + 1 agent)
  - `claude-md-management@claude-plugins-official` (~120 tokens)
  - `claude-code-setup@claude-plugins-official` (~100 tokens)
  - `feature-dev@claude-code-plugins` (~275 tokens, 3 agents + 1 skill)
  - `code-review@claude-code-plugins` (~15 tokens, 1 skill)

### Global-level changes
- User deleted `~/.claude/skills/` contents directly (no longer uses global skills — project-local skills evolved via `skill-audit` are authoritative).
- `~/.claude/settings.json` hooks narrowed: console.log grep in PostToolUse Edit and Stop hooks now scoped to `.ts|.tsx|.js|.jsx` files — no longer fires subprocess on every Rust edit.

### Post-second-pass baseline
- `/context` before: 33k (config) + 45.5k (messages) = 78.5k
- Custom agents dropped from 5.2k → expected ~500 tokens (user agents + superpowers:code-reviewer only)
- Expected new overhead: ~28k, leaving ~17k fresh-session headroom inside the 45–50k target.

## Still-flagged items (review if baseline drifts upward)

None. All prior deferred items (`voltagent-meta`, `claude-code-setup`, `voltagent-dev-exp`) addressed in the second pass.

---

## Project context (Worldwake)

- Rust workspace, 5 crates (`worldwake-core`, `-sim`, `-systems`, `-ai`, `-cli`)
- CLI/text prototype, no frontend, no TypeScript, no web, no mobile
- Toolchain: cargo, clippy, git, bash scripts, python3 for small dev scripts only
- Heavy use of: `specs/`, `tickets/`, `.claude/skills/`, golden E2E tests, observer scenarios
- Solo developer; actively monitoring token costs (hence custom statusLine)
- Active skills pattern: project-native `/brainstorm`, `/reassess-spec`, `/implement-ticket`,
  `/post-ticket-review`, `/scenario-analysis`, etc. User does NOT use SuperClaude `/sc:*` variants.

---

## Reversibility cheat-sheet

- Restore a disabled agent: `mv ~/.claude/agents.disabled/<name>.md ~/.claude/agents/`
- Restore a disabled plugin (project-level): edit `.claude/settings.json` and set to `true` or
  remove the key
- Restore a globally-disabled plugin: `/plugin` menu → enable
- Restore old statusLine: `mv ~/.claude/statusline-command.sh.backup ~/.claude/statusline-command.sh`
