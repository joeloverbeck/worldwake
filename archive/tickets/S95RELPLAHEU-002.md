# S95RELPLAHEU-002: Add FF heuristic fields to SearchExpansionSummary

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — SearchExpansionSummary field additions and decision-trace formatting in worldwake-ai; observer test-constructor fallout in worldwake-cli
**Deps**: S95 spec

## Problem

The decision trace infrastructure lacks fields to record FF heuristic values and helpful-action counts during search expansions. Without these fields, the RPG heuristic is not observable in decision traces, violating P29 (Debuggability).

## Assumption Reassessment (2026-04-12)

1. `SearchExpansionSummary` exists at `crates/worldwake-ai/src/decision_trace.rs:814` with 17 fields. `ff_heuristic` and `helpful_action_count` do not yet exist. The struct currently derives `Clone, Debug` only, so this ticket owns additive field carriage and human-readable formatting, not serde-shape work.
2. The live explicit struct literal fallout is 8 sites across 4 files: `search/mod.rs` (2), `agent_tick/planning.rs` (2), `decision_trace.rs` (2), and `observer.rs` (2). Each must be updated with inert defaults (`ff_heuristic: None`, `helpful_action_count: 0`).
3. Existing parallel fields `landmark_heuristic: u32` and `preferred_candidates: u16` in `SearchExpansionSummary` establish the pattern for the new fields.
4. The canonical human-readable expansion-summary formatting surface is the plan-trace renderer in `crates/worldwake-ai/src/decision_trace.rs`, which currently prints `h_landmark=...`. `crates/worldwake-cli/src/bin/observer.rs` does not have a separate live landmark formatter; it only carries sample/test `SearchExpansionSummary` builders that must compile with the new fields.

## Architecture Check

1. Adding `Option<u32>` and `u16` fields to an existing trace struct follows the established pattern. `Option` for `ff_heuristic` correctly represents "FF not enabled or dead end detected." Zero for `helpful_action_count` is the natural default when FF is not active.
2. The canonical text rendering should stay on the existing decision-trace formatter path rather than inventing a second observer-only formatter. `observer.rs` only absorbs constructor fallout in this slice.
3. No backward-compatibility shims. All live construction sites are updated in this ticket.

## Verification Layers

1. New fields exist with correct types → compilation success
2. Default values (None, 0) at all construction sites → existing tests pass unchanged
3. Canonical human-readable trace output includes FF diagnostics when present → focused decision-trace render test
4. `observer.rs` sample/test builders compile with inert defaults → worldwake-cli tests
5. Single-layer ticket — trace infrastructure only, no cross-system mapping needed.

## What to Change

### 1. Add fields to SearchExpansionSummary

In `crates/worldwake-ai/src/decision_trace.rs`, add after the `landmark_heuristic` field:

```rust
/// The FF relaxed-plan heuristic value at this expansion, or `None` if
/// FF is disabled, no operators were available, or the RPG detected a
/// dead end.
pub ff_heuristic: Option<u32>,
/// Number of helpful actions identified from the relaxed plan.
pub helpful_action_count: u16,
```

### 2. Update all construction sites

Add `ff_heuristic: None, helpful_action_count: 0` to each of the 8 construction sites:

- `crates/worldwake-ai/src/search/mod.rs` — 2 sites (lines ~554, ~678)
- `crates/worldwake-ai/src/agent_tick/planning.rs` — 2 sites (lines ~2311, ~2349)
- `crates/worldwake-ai/src/decision_trace.rs` — 2 test construction sites (lines ~3953, ~4113)
- `crates/worldwake-cli/src/bin/observer.rs` — 2 `sample_summary` helper sites (lines ~2234, ~2261)

### 3. Decision-trace formatting

In `crates/worldwake-ai/src/decision_trace.rs`, extend the existing expansion-summary formatter so that when `ff_heuristic` is `Some(h)`, the rendered expansion line includes `h_ff={h}` and `helpful_actions={count}` alongside `h_landmark=...`.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-cli/src/bin/observer.rs` (modify sample/test constructors only)

## Out of Scope

- Populating `ff_heuristic` with actual RPG values (ticket 004)
- RPG algorithm (ticket 003)
- CognitiveProfile field (ticket 001)

## Acceptance Criteria

### Tests That Must Pass

1. All existing decision trace tests pass with new default field values
2. Decision-trace render output includes FF diagnostics when present
3. Observer test constructions compile with new fields
4. Existing suite: `cargo test --workspace`

### Invariants

1. `SearchExpansionSummary` remains `Clone + Debug`
2. All construction sites initialize the new fields to inert defaults (None, 0)
3. No behavioral change — fields are populated with actual values in ticket 004

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — extend existing expansion-summary and formatter tests to cover `ff_heuristic` and `helpful_action_count`
2. `crates/worldwake-cli/src/bin/observer.rs` — existing sample/test helpers compile with new inert defaults

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-cli`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-12.

- Added `ff_heuristic: Option<u32>` and `helpful_action_count: u16` to `SearchExpansionSummary` in `crates/worldwake-ai/src/decision_trace.rs`.
- Updated all live `SearchExpansionSummary` construction sites to initialize the new fields to inert defaults (`None`, `0`) in `search/mod.rs`, `agent_tick/planning.rs`, `decision_trace.rs` tests, and `observer.rs` sample/test helpers.
- Extended the canonical decision-trace expansion formatter in `crates/worldwake-ai/src/decision_trace.rs` so rendered expansion lines include `h_ff=...` and `helpful_actions=...` when FF diagnostics are present.
- Strengthened owning tests in `decision_trace.rs` to prove the new fields survive direct construction/debug formatting and appear in rendered trace output.

## Deviations

- Reassessment corrected the ticket's live boundary: there are 8 explicit construction sites, not 11; `SearchExpansionSummary` currently derives `Clone + Debug` rather than serde traits; and the canonical human-readable formatting surface is `decision_trace.rs`, while `observer.rs` only needed sample/test constructor fallout updates.
- Cargo test verification was run sequentially after an initial parallel attempt hit target-directory lock contention; the completed command set still matches the ticket's required proof surface.

## Verification Result

- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
