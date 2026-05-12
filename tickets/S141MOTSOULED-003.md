# S141MOTSOULED-003: `RankedGoalSummary.motive_source_contributions` field

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai::decision_trace::RankedGoalSummary` field extension
**Deps**: `archive/tickets/S141MOTSOULED-001.md` (uses `MotiveSourceRef`)

## Problem

S141's decision-trace deliverable (D5) requires per-motive-source contribution scoring to be inspectable in the decision trace so observer Section 3b (owned by 006) can render the breakdown per `GoalCommitted`. The existing `RankedGoalSummary` at `crates/worldwake-ai/src/decision_trace.rs:529` carries `motive_score: u32` but no per-source decomposition.

This ticket adds the field with empty default. Population by `score_motive_source` is owned by 004 (the motive_score body refactor). The transient state (field exists but is empty until 004 lands) is acceptable because the trace surface is not authoritative — it's a derived read model per FND-27.

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RankedGoalSummary` at `crates/worldwake-ai/src/decision_trace.rs:529` currently carries `motive_score: u32`, `provenance`, discounts, `acquisition_quantity`, and `artifact_axes` (per S141 reassessment). The 8 construction sites flagged by Step 2 sub-check (d) are mostly inside decision_trace.rs and adjacent test surfaces. No existing focused/unit, runtime, or golden test currently asserts `motive_source_contributions` — it's a net-new trace field.
2. `MotiveSourceRef` lives in `worldwake-core::motive_source` after `archive/tickets/S141MOTSOULED-001.md` landed. `worldwake-ai/Cargo.toml` already depends on `worldwake-core`, so importing the new type is a one-line `use` change.
3. Shared abstraction boundary: `RankedGoalSummary` is the per-candidate trace shape consumed by `DecisionTraceSink` and ultimately by observer Section 3b. Its field set is the data contract under audit. Adding `motive_source_contributions: Vec<(MotiveSourceRef, u32)>` is purely additive; existing decision-trace consumers ignore the new field until 006 wires the rendering.
4. The field is populated by 004's `score_motive_source` returning `(MotiveSourceRef, u32)` tuples; this ticket only adds the field with empty default. The empty-vec transient state is FND-28-compliant because the field is a derived view (FND-27 cache), not authoritative state.

## Architecture Check

1. Adding a typed per-source contribution slot on `RankedGoalSummary` makes "Agent X chose Y because they cared about Z" (FND-20) literally inspectable from the trace — observer Section 3b can render `NeedPressure(Hunger) → 14200, Greed(market_opportunity#42) → 4220` without re-computing from scratch. The alternative (recompute contributions in the observer from `offer.motive_sources` + `RankingContext`) would re-do the scoring work and risk drift between observer math and ranking math.
2. The new field defaults to `Vec::new()` — no `#[serde(default)]` needed because the field is in-memory trace state, not serialized save state. (If decision-trace state is later serialized, `Vec::new()` is already its `Default` value.)

## Verification Layers

1. RankedGoalSummary shape → focused unit test in `crates/worldwake-ai/src/decision_trace.rs#[cfg(test)]` asserting `RankedGoalSummary::default().motive_source_contributions.is_empty()`.
2. Trace consumer compatibility → existing decision-trace tests continue to pass without modification (the new field defaults to empty; consumers that don't read it are unaffected).
3. Single-layer ticket — population by `score_motive_source` is owned by 004; rendering by observer is owned by 006. Cross-layer verification belongs in those tickets.

## What to Change

### 1. Extend `RankedGoalSummary` struct

At `crates/worldwake-ai/src/decision_trace.rs:529` add the new field:

```rust
pub struct RankedGoalSummary {
    // existing fields preserved
    pub motive_source_contributions: Vec<(MotiveSourceRef, u32)>,
}
```

Add `use worldwake_core::motive_source::MotiveSourceRef;` at the top of the file (or rely on the crate-root re-export established by `archive/tickets/S141MOTSOULED-001.md`).

### 2. Update the 8 `RankedGoalSummary { ... }` construction sites

Each site adds `motive_source_contributions: Vec::new(),` to the literal. The 8 sites are inside `crates/worldwake-ai/src/decision_trace.rs` and its sibling consumers — confirm exact locations via `rg -n "RankedGoalSummary\s*\{" crates/worldwake-ai/` during the implementation phase.

### 3. Update any explicit field-by-field equality test fixtures

If decision-trace tests construct expected `RankedGoalSummary` values with explicit literals (rather than via factory helpers), add the new field with an empty vec.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — field + construction sites + tests)
- Likely: additional 1–2 files containing the remaining `RankedGoalSummary { ... }` sites; confirm with `rg -n "RankedGoalSummary\s*\{" crates/worldwake-ai/` during reassessment.

## Out of Scope

- Population of `motive_source_contributions` by `score_motive_source` — owned by 004.
- Observer rendering of the field — owned by 006.
- `SAVE_FORMAT_VERSION` bump — owned by `archive/tickets/S141MOTSOULED-002.md` (this field is in-memory trace state, not serialized; if later serialized, it rides under version 78 via `Vec::new()` default).

## Acceptance Criteria

### Tests That Must Pass

1. `RankedGoalSummary::default().motive_source_contributions` is an empty `Vec` (or equivalent assertion against the explicit-construction path used by existing tests).
2. Existing suite: `cargo test -p worldwake-ai` (decision-trace tests continue to pass; no consumer reads the new field yet).

### Invariants

1. The field type is exactly `Vec<(MotiveSourceRef, u32)>` — matches the spec D5 prose and aligns with the existing `decisive_beliefs: Vec<BeliefRef>` convention in `decision_event_payload.rs:346`.
2. The field defaults to an empty `Vec`; no construction site emits non-empty values until 004 lands the population.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs#[cfg(test)]` — add a focused test asserting the new field's empty-default behavior and that bincode round-trip (if applicable) preserves emptiness.

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test -p worldwake-ai` (full crate sweep; existing decision-trace tests confirm no regression)
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
