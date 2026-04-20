# S110DECHISEVE-006: Observer Decision History section

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — observer binary only; no simulation state or action behavior affected
**Deps**: archive/tickets/S110DECHISEVE-004.md (the first foundations-honest emission slice must be archived before the observer snapshot can rely on live decision events)

## Problem

S110's user-facing benefit is answering "why did this agent commit / reject / replan?" from world history without rerunning with tracing enabled. The observer binary already renders event-log summaries from `EventLog::events_by_tag(…)`. This ticket adds a dedicated "Decision History" section that renders the new decision events as a deterministic markdown table. The section replaces much of what the observer-behavioral-enrichment waves (`archive/specs/S85-observer-behavioral-enrichment.md`, `archive/specs/S98-observer-affordance-change-detection.md`) previously synthesized from heuristics — the synthesis is now grounded in authoritative log entries rather than heuristic inference.

## Assumption Reassessment (2026-04-20)

1. Observer binary lives at `crates/worldwake-cli/src/bin/observer.rs`. The observer already consumes `event_log: &worldwake_core::EventLog` (parameter of the render function around `observer.rs:2387`) and renders sections for Run Metadata (line 2401), Per-Agent Summary (line 2426), action counts, etc. The "Decision History" section is additive — insert a new section (positioned per the spec's implicit ordering after per-agent summary) that iterates decision-tagged events in log order.
2. The observer has an existing snapshot-test contract: the Markdown output is deterministic given a fixed scenario + seed + tick count. New rendering must preserve this determinism. Iteration must use a stable order (event-log insertion order, which is tick-sequential and deterministic).
3. Shared abstraction boundary under audit: the observer's Markdown output format. The "Decision History" section is a new header + table; it does not change any existing section. Downstream consumers of the observer dump (test harnesses, CI regression checks, external reviewers) see a strictly additive diff.

## Architecture Check

1. Rendering directly from `EventLog` (the authoritative append-only source) preserves FND-27 — the observer section is a derived view over the log, never a cache promoted to truth. Delete the observer binary output and the world's meaning is unchanged; the log remains the single source of causal history.
2. The one-line-per-event format with deterministic formatting is simpler than any intermediate aggregation. FND-29 (debuggability) is satisfied by presenting events in the order they occurred — no grouping, no summary that would hide the causal sequence.
3. Existing observer-enrichment archival specs (S85, S98) synthesized similar information heuristically. S110 supersedes that synthesis with authoritative log content; any heuristic that is no longer needed after this ticket should be flagged for removal in a follow-up ticket, not in this one. (This is a scope boundary: removing the old heuristic rendering is not in scope here.)

## Verification Layers

1. Rendering determinism → observer snapshot test on `survival-baseline.ron` at a fixed seed and tick count. The test asserts that the "Decision History" section matches a committed golden markdown fragment.
2. All-variant rendering coverage → a focused unit test (if the observer's rendering functions are testable in isolation) that constructs a small `EventLog` with one event per decision variant and asserts the rendered section contains one row per variant.
6. Single-layer ticket (rendering only) — no decision-trace, action-trace, or belief-view mapping. The observer is a derived view.

## What to Change

### 1. Add "Decision History" section to observer output

In `crates/worldwake-cli/src/bin/observer.rs`, add a new rendering function `render_decision_history_section(out: &mut String, event_log: &EventLog, agents: &[(EntityId, String)])` that:

- Writes header: `writeln!(out, "## Section — Decision History\n")`
- Writes column headers: `| Tick | Agent | Event | Payload Summary |` and the separator row.
- Iterates events in log-insertion order, filtered to the 11 new decision `EventTag` variants. For each event, formats a row:
  - `Tick`: `event.tick().0` as string
  - `Agent`: name lookup from the `agents` parameter (`actor_id` → name); fallback to `EntityId` if not found
  - `Event`: the `EventTag` variant's debug name (e.g., `GoalCommitted`)
  - `Payload Summary`: a deterministic one-line summary built per variant from the `decision_payload`. Examples:
    - `GoalOffered`: `goal={goal_key} emitter={emitter_tag}`
    - `GoalCommitted`: `goal={goal_key} motive={motive_score} alts={rejected_alternatives.len()}`
    - `PlanAdopted`: `goal={goal_key} steps={plan_step_count}`
    - `PlanInvalidated`: `goal={goal_key} reason={reason_variant_name}`
    - `BlockerRecorded`: `key={blocker_key} class={discrepancy_or_blocking_fact_variant} expires={expires_tick.0}`

Each payload summary is a stable one-line string — no newlines, no variable ordering of iteration over BTreeMaps or Vecs (use `.to_string()` on typed enum variants for stability).

Call `render_decision_history_section` from the main render function (the one that writes run metadata, per-agent summary, etc.) at the appropriate insertion point (after per-agent summary is a reasonable location — finalize at implementation time).

### 2. Add helper `decision_payload_summary(payload: &DecisionEventPayload) -> String`

Either in `observer.rs` as a module-local helper or alongside the `DecisionEventPayload` definition in `worldwake-core` (as `impl DecisionEventPayload { pub fn summary_line(&self) -> String { … } }`). Placement decision goes to the implementer — in-core is more discoverable for future tooling but ties the core crate to a formatting concern; observer-local is narrower but duplicates the match if another tool later wants the same summary. Recommendation: observer-local for now; refactor to core if a second consumer emerges.

### 3. Snapshot test on `survival-baseline.ron`

Add a snapshot-style test under `crates/worldwake-cli/tests/` (or extend the existing observer snapshot infrastructure if one exists — implementer greps `tests/*.rs` for observer snapshot patterns at implementation time) that:

- Runs `survival-baseline.ron` at seed 1 (or whatever the existing observer snapshot-test seed convention is) for a fixed tick count (e.g., 50 ticks).
- Captures the full observer dump.
- Asserts the "Decision History" section matches a committed golden markdown fragment.

The fragment lives alongside the test (committed to the repo). A tool / script to regenerate it on intentional format changes should be named in the commit message if implementer adds one.

### 4. Focused unit test on rendering

Add to `crates/worldwake-cli/src/bin/observer.rs` `#[cfg(test)]` block a test that constructs a small `EventLog` with one event per `DecisionEventPayload` variant and asserts `render_decision_history_section` output contains 11 data rows plus the header and separator.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — new rendering function, helper, and `#[cfg(test)]` unit test)
- `crates/worldwake-cli/tests/` (new or modify — observer snapshot test and golden fragment)

## Out of Scope

- Removing heuristic-based synthesis from older observer waves (S85, S98). That is a separate cleanup ticket. Coexistence is acceptable for this spec.
- Interactive filtering / sorting of the Decision History section. The table is rendered as a single deterministic pass in log order.
- Rendering of pre-S110 saved logs that lack `decision_payload`. Per FND-28, old logs are not decodable after ticket 002's `SAVE_FORMAT_VERSION` bump — the observer does not need a compatibility path.
- Exporting the Decision History as a separate file or JSON surface. The observer Markdown dump is the delivery target.
- Observer-side aggregation (commit rate per agent, rejection-reason histogram). The spec's Section H notes these are legitimate derived views but defers them to future tooling.

## Acceptance Criteria

### Tests That Must Pass

1. New unit test in `observer.rs` `#[cfg(test)]` — renders 11 decision variants, asserts table structure and one row per variant.
2. New snapshot test — `survival-baseline.ron` observer dump's "Decision History" section matches the committed golden fragment.
3. Existing observer tests (if any) continue to pass — the new section is additive.
4. `cargo test -p worldwake-cli` — targeted.
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Observer output remains deterministic given a fixed scenario + seed + tick count.
2. The "Decision History" section contains exactly the events tagged with one of the 11 new `EventTag` variants, in event-log insertion order, one row per event.
3. Every payload-summary string is newline-free and produced by a stable formatting function — no `HashMap` iteration, no float formatting, no wall-clock reads.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (`#[cfg(test)]`) — `render_decision_history_section_covers_all_variants`.
2. `crates/worldwake-cli/tests/observer_decision_history.rs` (new) — `survival_baseline_decision_history_section_matches_golden`.
3. Committed golden fragment file (path chosen by implementer; alongside other observer snapshots if they exist).

### Commands

1. `cargo test -p worldwake-cli observer_decision_history` — targeted.
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
