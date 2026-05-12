# S141MOTSOULED-003: `RankedGoalSummary.motive_source_contributions` field

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai::decision_trace::RankedGoalSummary` field extension
**Deps**: `archive/tickets/S141MOTSOULED-001.md` (uses `MotiveSourceRef`)

## Problem

S141's decision-trace deliverable (D5) requires per-motive-source contribution scoring to be inspectable in the decision trace so observer Section 3b (owned by 006) can render the breakdown per `GoalCommitted`. The existing `RankedGoalSummary` at `crates/worldwake-ai/src/decision_trace.rs:529` carries `motive_score: u32` but no per-source decomposition.

This ticket added the field with empty default. Population by `score_motive_source` remains owned by 004 (the motive_score body refactor). The transient state (field exists but is empty until 004 lands) is acceptable because the trace surface is not authoritative — it's a derived read model per FND-27.

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `RankedGoalSummary` at `crates/worldwake-ai/src/decision_trace.rs:529` carried `motive_score: u32`, `provenance`, discounts, `acquisition_quantity`, and `artifact_axes` before this ticket. The live constructor sweep found 21 explicit construction/helper sites once test-only forensic helpers and the production `summarize_ranked_goal` path were included. No existing focused/unit, runtime, or golden test asserted `motive_source_contributions` before this ticket — it was a net-new trace field.
2. `MotiveSourceRef` lives in `worldwake-core::motive_source` after `archive/tickets/S141MOTSOULED-001.md` landed. `worldwake-ai/Cargo.toml` already depends on `worldwake-core`, so importing the new type is a one-line `use` change.
3. Shared abstraction boundary: `RankedGoalSummary` is the per-candidate trace shape consumed by `DecisionTraceSink` and ultimately by observer Section 3b. Its field set is the data contract under audit. Adding `motive_source_contributions: Vec<(MotiveSourceRef, u32)>` is purely additive; existing decision-trace consumers ignore the new field until 006 wires the rendering.
4. The field is populated by 004's `score_motive_source` returning `(MotiveSourceRef, u32)` tuples; this ticket only adds the field with empty default. The empty-vec transient state is FND-28-compliant because the field is a derived view (FND-27 cache), not authoritative state.

## Architecture Check

1. Adding a typed per-source contribution slot on `RankedGoalSummary` makes "Agent X chose Y because they cared about Z" (FND-20) literally inspectable from the trace — observer Section 3b can render `NeedPressure(Hunger) → 14200, Greed(market_opportunity#42) → 4220` without re-computing from scratch. The alternative (recompute contributions in the observer from `offer.motive_sources` + `RankingContext`) would re-do the scoring work and risk drift between observer math and ranking math.
2. The new field defaults to `Vec::new()` — no `#[serde(default)]` needed because the field is in-memory trace state, not serialized save state. (If decision-trace state is later serialized, `Vec::new()` is already its `Default` value.)

## Verified Layers

1. RankedGoalSummary shape → focused unit test in `crates/worldwake-ai/src/decision_trace.rs#[cfg(test)]` asserts `RankedGoalSummary::default().motive_source_contributions.is_empty()`.
2. Trace consumer compatibility → existing decision-trace tests passed without consumer changes; consumers that do not read the field are unaffected.
3. Single-layer ticket — population by `score_motive_source` remains owned by 004; rendering by observer remains owned by 006. Cross-layer verification belongs in those tickets.

## What Changed

### 1. Extended `RankedGoalSummary` struct

At `crates/worldwake-ai/src/decision_trace.rs:529`, the struct now includes:

```rust
pub struct RankedGoalSummary {
    pub motive_source_contributions: Vec<(MotiveSourceRef, u32)>,
}
```

The implementation imports `MotiveSourceRef` from `worldwake_core`.

### 2. Updated `RankedGoalSummary { ... }` construction sites

Each explicit construction/helper site now sets `motive_source_contributions: Vec::new(),`. The live patch updated the production summarizer in `crates/worldwake-ai/src/agent_tick/planning.rs`, `decision_trace.rs` unit fixtures, `survival_forensics.rs` unit fixtures, and the golden-harness synthetic helper.

### 3. Updated explicit field-by-field test fixtures

Explicit `RankedGoalSummary` test literals now carry the new field with an empty vec.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — field + construction sites + tests)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — production `summarize_ranked_goal` empty staged field)
- `crates/worldwake-ai/src/survival_forensics.rs` (modify — unit fixture fallout)
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs` (modify — synthetic helper fallout)

## Out of Scope

- Population of `motive_source_contributions` by `score_motive_source` — owned by 004.
- Observer rendering of the field — owned by 006.
- `SAVE_FORMAT_VERSION` bump — owned by `archive/tickets/S141MOTSOULED-002.md` (this field is in-memory trace state, not serialized; if later serialized, it rides under version 78 via `Vec::new()` default).

## Acceptance Result

### Tests

1. `RankedGoalSummary::default().motive_source_contributions` is an empty `Vec`, proved by the focused unit test.
2. Existing suite: `cargo test -p worldwake-ai` passed; no consumer reads the new field yet.

### Invariants

1. The field type is exactly `Vec<(MotiveSourceRef, u32)>` — matches the spec D5 prose and aligns with the existing `decisive_beliefs: Vec<BeliefRef>` convention in `decision_event_payload.rs:346`.
2. The field defaults to an empty `Vec`; no construction site emits non-empty values until 004 lands the population.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs#[cfg(test)]` — added a focused test asserting the new field's empty-default behavior. No bincode round-trip was added because `RankedGoalSummary` is an in-memory trace summary, not a serialized save payload.

### Commands

1. `cargo test -p worldwake-ai --lib decision_trace::tests::ranked_goal_summary_default_has_empty_motive_source_contributions -- --exact`
2. `cargo test -p worldwake-ai` (full crate sweep; existing decision-trace tests confirm no regression)
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-12.

- Added `RankedGoalSummary.motive_source_contributions: Vec<(MotiveSourceRef, u32)>` and a `Default` impl whose staged contribution vector is empty.
- Updated the production `summarize_ranked_goal` trace construction and all explicit test/synthetic `RankedGoalSummary` literals to seed the field with `Vec::new()`.
- Added focused unit coverage for the empty-default behavior. Population by motive-source scoring remains owned by `tickets/S141MOTSOULED-004.md`.

## Deviations

- The live constructor sweep found 21 explicit construction/helper sites, not the drafted 8. The additional touched files were constructor fallout for the same shared trace carrier.
- No save-format bump or bincode test landed because `RankedGoalSummary` is an in-memory derived trace summary, not a persisted save payload.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib decision_trace::tests::ranked_goal_summary_default_has_empty_motive_source_contributions -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
