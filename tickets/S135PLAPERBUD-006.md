# S135PLAPERBUD-006: Observer perception summary omission rendering

**Status**: PENDING
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: No — observer-only (read-only consumer)
**Deps**: `archive/tickets/S135PLAPERBUD-001.md`, S135PLAPERBUD-003

## Problem

Per S135 D6, the observer's "Perception Trace Summary" sub-section inside Section 5 "Raw Event Sample" should render per-agent top-K omissions grouped by `OmissionReason` discriminant, so a reader can answer "why did this agent ignore the dragon next to them?" without re-running the simulation. This ticket extends the existing sub-heading and adds a `--top-omissions <K>` CLI flag (default K=5).

## Assumption Reassessment (2026-05-05)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Observer "### Perception Trace Summary" sub-heading lives at `crates/worldwake-cli/src/bin/observer.rs:3091` inside Section 5 "## Section 5 — Raw Event Sample" (`observer.rs:2981`). The total observer file is 6509+ lines. Existing perception rendering shows per-agent event bins (observed counts, entity tallies) over 100-tick windows.
2. `--top-omissions` CLI flag does not currently exist (validated during S135 reassessment). The observer's `Cli` struct carries other top-N flags (e.g., `--top-stuck-windows`, `--top-critical-windows` per the file's command-line arg parsing area near top). Match the existing convention.
3. Shared abstraction boundary under audit: the observer's read-only consumption of `AgentBeliefStore.observation_omission_log` snapshots replayed from the event log via the existing `BeliefStoreDiff` reconstruction path (added in ticket 001's paired-field extension). No simulation state mutation; no AI-side logic change. This matches the "Read-Only Tooling Consumer" pattern documented in `.claude/skills/reassess-spec/references/worldwake-validation-patterns.md`.
4. Existing observer tests in `observer.rs` cfg-test block: `assert!(out.contains("## Section 3 — Decision History"));` and similar at lines 5422, 5508, 5510, 5546, 5575, 5605. Add new assertions for the omission block in the same style.

## Architecture Check

1. The new rendering is purely additive — the existing "### Perception Trace Summary" sub-heading is preserved, and the omissions block appears after the existing per-agent event bins as a sub-block. No regression risk to existing observer output.
2. CLI flag follows the existing top-N convention: `--top-omissions <K>` with default 5, matching the `--top-critical-windows` style. Clap derive plus `#[arg(long, default_value_t = 5)]`.
3. The aggregation reads `ObservationOmissionLog` through the event-log delta replay path (no direct world-state reads). Per the "Read-Only Tooling Consumer" pattern, this matches the existing observer architecture (e.g., the perception trace already replays through events).
4. Determinism: top-K entries must be ordered by `observed_tick` descending with `BTreeMap`-stable tie-break (sorted by `omitted_entity` ascending). This matches S135 Goal 5's determinism requirement.

## Verification Layers

1. The new top-K block renders correctly with non-empty omission logs → focused unit test in `observer.rs` cfg-test block (named-render output assertion).
2. The new block is absent (or shows "no omissions") when every agent's log is empty → focused unit test.
3. Existing observer Section 5 output is unchanged for runs where no entities are dropped → existing observer cfg-test sufficient.
4. CLI flag parsing works → focused unit test on `Cli` struct parsing (or run `--help` and grep for the flag).
5. **Single-layer ticket** — observer is a read-only diagnostic surface. No simulation state mutation, so no decision-trace or action-trace layer mapping needed. The proof surface is the observer's rendered output.

## What to Change

### 1. Add `--top-omissions` CLI flag

In `crates/worldwake-cli/src/bin/observer.rs`, locate the `Cli` struct (likely near top of file with `clap::Parser` derive). Add:

```rust
#[arg(long, default_value_t = 5)]
top_omissions: usize,
```

Match the visibility and field-naming convention of sibling top-N flags (search for `top_critical_windows`, `top_stuck_windows`, etc. in the same struct).

### 2. Render top-K omissions per agent

In `observer.rs` near line 3091 (`### Perception Trace Summary`), after the existing per-agent event bins, add a per-agent block. Suggested format:

```
#### Top observation omissions

| Agent | OverBudget | SalienceBelowFloor | Top entries |
|-------|-----------|--------------------|-------------|
| <name> | <count>  | <count>            | <K entries: entity, reason, tick> |
```

The "Top entries" column shows up to `cli.top_omissions` entries from each agent's `ObservationOmissionLog`, ordered by `observed_tick` descending with `BTreeMap`-stable tie-break (sorted by `omitted_entity` ascending). Each row lists `entity_display_name(world, omitted_entity)` (per the existing helper at `crates/worldwake-cli/src/display.rs`), the reason variant discriminant, and the tick.

For agents whose log is empty, render `— (no omissions recorded)`.

### 3. Aggregate counts by discriminant

For each agent, count occurrences of `OmissionReason::OverBudget` and `OmissionReason::SalienceBelowFloor` separately across their `ObservationOmissionLog.entries`. Display these counts in the table.

### 4. Cap K to log capacity

If `cli.top_omissions` exceeds an agent's entries count, render only the available entries. If `cli.top_omissions == 0`, render an empty "Top entries" column for that row but still show the count columns.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify) — `Cli` struct extension and Section 5 sub-block rendering

## Out of Scope

- Mutating `ObservationOmissionLog` — that's perception's job (ticket 003).
- Cross-agent correlation (e.g., "all agents in place X have similar omissions") — defer to a future spec if needed.
- Section reorganization — Section 5's existing structure is preserved; the omissions block is purely additive.
- New goldens → ticket 007.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli --bin observer` passes (new block render assertions + existing observer tests at lines 5422, 5508, 5510, 5546, 5575, 5605).
2. `cargo run -p worldwake-cli --bin observer -- --help` shows the new `--top-omissions` flag.
3. `cargo build --workspace` succeeds.

### Invariants

1. The new block renders as part of `### Perception Trace Summary` and never replaces or reorders the existing per-agent event bins (additive only).
2. Top-K omission entries are sorted by `observed_tick` descending, with deterministic tie-break by `omitted_entity` ascending (`BTreeMap`-stable order — matches spec Goal 5).
3. The flag default (5) is respected; explicit values override; values exceeding an agent's entry count gracefully render only available entries.
4. Per-agent counts of `OverBudget` and `SalienceBelowFloor` discriminants sum to ≤ that agent's `ObservationOmissionLog.entries.len()`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` cfg-test block — new test: synthesized run with one agent and 8 dropped entities, render observer output, assert "Top observation omissions" block contains 5 entries (top-K capped at default 5), `OverBudget` count of 8, `SalienceBelowFloor` count of 0.
2. `crates/worldwake-cli/src/bin/observer.rs` cfg-test block — new test: run with no dropped entities, assert the omissions block reads `— (no omissions recorded)` for every agent.
3. `crates/worldwake-cli/src/bin/observer.rs` cfg-test block — new test: `--top-omissions 3` overrides default; assert the block shows 3 entries.
4. `crates/worldwake-cli/src/bin/observer.rs` cfg-test block — new test: deterministic ordering — two agents with identical omission counts but different `(observed_tick, omitted_entity)` pairs render in stable order across re-runs.

### Commands

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo run -p worldwake-cli --bin observer -- --help | grep top-omissions`
3. `./scripts/verify.sh`
