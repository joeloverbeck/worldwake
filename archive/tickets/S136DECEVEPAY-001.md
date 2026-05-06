# S136DECEVEPAY-001: Core types and payload field additions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-core::decision_event_payload`, `worldwake-sim::save_load`
**Deps**: spec `archive/specs/S136-decision-event-payload-extension.md` (D1, D2, D3, D4, D7)

## Problem

S110 made decision-history events always-on, but the always-on payloads carry only the chosen-goal/plan-ID/typed-`Discrepancy` surface. The failure path (`BlockerRecorded`, `ReplanTriggered`, `ExpectationMismatch`, `SourceExpectationFailure`) does not name the load-bearing beliefs/records/observations whose absence or contradiction would have flipped the outcome. The success path's `RejectedAlternativeSummary` records `rejection_reason` (a `GoalRejectionReason`) but not the *decisive ranking dimension* that ordered each rejected goal against the chosen one. Forensic reconstruction of "why did Agent X give up at tick 530?" still requires `enable_tracing()` or backward inference — incomplete per FND-29A.

This foundation ticket adds all the typed surfaces the rest of S136 will populate: 5 new core-side types, additive fields on 6 existing payload structs plus `RejectedAlternativeSummary`, and the `SAVE_FORMAT_VERSION` bump. All new fields use `#[serde(default)]` for omitted-field fixture/deserialization tolerance and default to empty `Vec` / `None` at construction sites, so the workspace builds and existing tests pass after the explicit construction-site updates. Current-format bincode saves are version-gated; v69 saves are rejected after the 70 bump rather than replayed forward. Subsequent tickets (002–005) populate the empty defaults with real data; ticket 006 adds golden coverage.

## Assumption Reassessment (2026-05-06)

1. `crates/worldwake-core/src/decision_event_payload.rs:156-409` defines 12 payload structs. Per the spec's per-tag field map, only 6 gain new fields in this ticket: `GoalCommittedPayload:156`, `PlanAdoptedPayload:214`, `BlockerRecordedPayload:399`, `ReplanTriggeredPayload:362`, `ExpectationMismatchPayload:290`, `SourceExpectationFailurePayload:300`. `RejectedAlternativeSummary` exists at `decision_event_payload.rs:164` with fields `goal_key`, `rejection_reason`, `score_gap`. `BeliefStatusTag` mirror exists at `decision_event_payload.rs:231` with derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize` — the new `RankedGoalComparisonDimensionTag` follows this precedent.
2. `RankedGoalComparisonDimension` enum lives in `crates/worldwake-ai/src/ranking.rs:2348` (cannot be relocated to core without inverting the workspace dependency graph). Reassessment corrected the drafted conversion-site premise: `worldwake-sim` cannot name the AI enum because it does not and must not depend on `worldwake-ai`. Ticket 001 lands the core mirror enum and current-format save/load roundtrip coverage; ticket 003 owns the AI-side runtime conversion when it populates `rejection_dimension`.
3. Boundary under audit: the `decision_event_payload` module's authoritative struct shapes — every consumer of these types must continue to construct them. Confirmed explicit-field construction sites across 4 crates with **0 spread-syntax usage** (no `..Default::default()` / `..Type::default()` patterns), so every site must explicitly initialize the new fields. Bincode save/load remains version-gated; v69 saves are rejected under v70.
4. Existing focused tests on the affected types: serde round-trip and `assert_value_bounds` coverage in `decision_event_payload.rs::tests` (e.g., `assert_value_bounds::<RejectedAlternativeSummary>` at line 635, `assert_copy_value_bounds::<BeliefStatusTag>` at line 650); `crates/worldwake-sim/src/save_load.rs::tests` covers per-version load paths. No goldens directly assert payload-field shape (existing goldens read `motive_score` / `goal_key` only). Ticket 006 adds the explicit field-shape golden coverage; ticket 001 adds focused serde round-trip coverage for the new fields.
5. `SAVE_FORMAT_VERSION` was 69 at `save_load.rs:6`; the dispatcher at `save_load.rs:129` uses `SAVE_FORMAT_VERSION => load_current_format(...)` so bumping the constant to 70 routes correctly without further dispatch changes. The existing wrong-version test proves v69 is rejected.

## Architecture Check

1. The new fields are additive and `#[serde(default)]`-annotated for omitted-field fixture/deserialization tolerance, but bincode current-format saves are still version-gated. Pre-bump v69 saves are rejected after the 70 bump per the repo's no-backward-compatibility rule. No live-authority shim or duplicate authoritative representation was added.
2. `RankedGoalComparisonDimensionTag` follows the established core-resident mirror pattern without relocating the source enum to core or inverting the `core ← ai` dependency. The mirror is mechanical: 1:1 variant correspondence with no semantic differences. The AI-side conversion is left for ticket 003.
3. The 4 typed reference structs (`BeliefRef`, `RecordRef`, `ObservationRef`, `PlanAssumptionRef`) carry stable typed addresses (entity ID + claim key + tick) rather than embedded value snapshots, preserving FND-29A's append-only history without payload bloat under contradiction-rich scenarios.
4. No new `EventTag` variant; payload widening only.

## Verification Layers

1. Type-shape correctness → `cargo test -p worldwake-core decision_event_payload` (existing serde round-trip suite extended with new field coverage).
2. Save/load current-format roundtrip and version gate → `cargo test -p worldwake-sim save_load` (current-format roundtrip preserves non-empty new fields; v69 is rejected as wrong version).
3. Construction-site updates → `cargo build --workspace` is the proof surface (compile-coherent state required across all 4 crates).
4. SAVE_FORMAT_VERSION bump → focused test in `save_load.rs::tests` asserting the new constant value and that the dispatcher routes `70` to `load_current_format`.

## What to Change

### 1. New core-side types in `decision_event_payload.rs`

Add these definitions adjacent to existing payload types. `RankedGoalComparisonDimensionTag` follows `BeliefStatusTag`'s derives; the four ref structs match each other.

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RankedGoalComparisonDimensionTag {
    // 1:1 with worldwake-ai::ranking::RankedGoalComparisonDimension variants
    PriorityClass,
    SubstitutePreferenceOrder,
    MotiveScore,
    SourceComposite,
    Feasibility,
    GoalSpecificity,
    OpportunityStrength,
    // ... add every variant present in ranking.rs:2348 (enumerate at implementation time)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct BeliefRef {
    pub claim_key: BeliefClaimKey,
    pub claim_held_at_tick: Tick,
    pub status: BeliefStatusTag,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct RecordRef {
    pub record_entity: EntityId,
    pub recorded_at_tick: Tick,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ObservationRef {
    pub observed_entity: EntityId,
    pub aspect: EntityBeliefAspect,
    pub observed_tick: Tick,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlanAssumptionRef {
    pub assumption: FrameAssumption,
    pub introduced_at_step: u8,
}
```

Re-export each from `crates/worldwake-core/src/lib.rs` alongside the existing `BeliefStatusTag` re-exports.

### 2. Widen `RejectedAlternativeSummary` with `rejection_dimension`

Add as a final field with serde default (placeholder; ticket 003 wires real data):

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RejectedAlternativeSummary {
    pub goal_key: GoalKey,
    pub rejection_reason: GoalRejectionReason,
    pub score_gap: i32,
    #[serde(default)]
    pub rejection_dimension: Option<RankedGoalComparisonDimensionTag>,  // NEW
}
```

### 3. Failure-path payload widening

Add three `#[serde(default)] Vec<…>` fields to:
- `BlockerRecordedPayload` (`decision_event_payload.rs:399`)
- `ReplanTriggeredPayload` (`decision_event_payload.rs:362`)
- `ExpectationMismatchPayload` (`decision_event_payload.rs:290`)
- `SourceExpectationFailurePayload` (`decision_event_payload.rs:300`)

```rust
#[serde(default)]
pub decisive_beliefs: Vec<BeliefRef>,
#[serde(default)]
pub decisive_records: Vec<RecordRef>,
#[serde(default)]
pub decisive_world_observations: Vec<ObservationRef>,
```

All four payloads gain all three Vecs (per spec D3). All default to empty.

### 4. `assumptions` field on success-path and most failure-path payloads

Add `#[serde(default)] pub assumptions: Vec<PlanAssumptionRef>` to:
- `GoalCommittedPayload` (`decision_event_payload.rs:156`)
- `PlanAdoptedPayload` (`decision_event_payload.rs:214`)
- `BlockerRecordedPayload` (also gains `decisive_*` per #3)
- `ReplanTriggeredPayload` (also gains `decisive_*`)
- `ExpectationMismatchPayload` (also gains `decisive_*`)

`SourceExpectationFailurePayload` does NOT gain `assumptions` (per spec D4 — no active-plan frame at source-expectation failure time).

### 5. Update all 64 construction sites

For each affected struct, every existing `<StructName> { ... }` literal in the workspace must add the new fields with empty defaults — placeholder values that subsequent tickets replace with real data. No spread-syntax shortcut available (0 sites use `..Default::default()`).

Defaults to insert:
- `RejectedAlternativeSummary`: `rejection_dimension: None` (replaced by ticket 003 at `build_rejected_alternatives`).
- Failure-path payloads: `decisive_beliefs: Vec::new(), decisive_records: Vec::new(), decisive_world_observations: Vec::new()` (replaced by ticket 004).
- 5 payloads gaining `assumptions`: `assumptions: Vec::new()` (replaced by ticket 002).

Sites identified at ticket-write time (per-struct counts: `GoalCommittedPayload` 7; `PlanAdoptedPayload` 6; `BlockerRecordedPayload` 11; `ReplanTriggeredPayload` 7; `ExpectationMismatchPayload` 11; `SourceExpectationFailurePayload` 8; `RejectedAlternativeSummary` ~14):
- `crates/worldwake-core/src/decision_event_payload.rs` — 2 `RejectedAlternativeSummary` + assorted payload tests.
- `crates/worldwake-ai/src/agent_tick/planning.rs` — 5 `RejectedAlternativeSummary` (lines 994, 3642, 3647, 3934, 3939) + payload sites.
- `crates/worldwake-ai/src/agent_tick/execution.rs` — failure-path emission sites (lines 140, 222, 448, 503).
- `crates/worldwake-ai/src/agent_tick/observation.rs` — `ExpectationMismatch` emission (line 123).
- `crates/worldwake-ai/src/agent_tick/mod.rs` — emission sites (lines 476, 497, 516, 621, 682, 696, 882, 1774, 1815).
- `crates/worldwake-ai/src/agent_tick/tests.rs` — test fixtures.
- `crates/worldwake-sim/src/save_load.rs` — 1 `RejectedAlternativeSummary` (line 856) + per-version load test fixtures.
- `crates/worldwake-cli/src/bin/observer.rs` — 2 `RejectedAlternativeSummary` (lines 4470, 5750) + decision-history test fixtures.

Re-grep at implementation time to confirm no construction sites have moved.

### 6. `RankedGoalComparisonDimension → Tag` conversion

Reassessment corrected this drafted step. `worldwake-sim` cannot name `worldwake-ai::ranking::RankedGoalComparisonDimension` without an invalid dependency edge. This ticket therefore adds only the core mirror enum and serde/bincode shape coverage. Ticket 003 owns the AI-side conversion at the runtime emission site.

### 7. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`, change `pub const SAVE_FORMAT_VERSION: u32 = 69;` to `70`. Confirm dispatcher at `save_load.rs:129` continues to route via the named constant (no literal-value match arm to update).

## Files to Touch

- `crates/worldwake-core/src/decision_event_payload.rs` (modify — new types, field additions, test fixtures)
- `crates/worldwake-core/src/lib.rs` (modify — re-export new types)
- `crates/worldwake-sim/src/save_load.rs` (modify — version bump, current-format test fixtures)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — construction sites)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — construction sites)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — construction sites)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — construction sites)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — test fixture construction sites)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — test fixture construction sites)

## Out of Scope

- Populating `decisive_*` fields with real failure-input data (ticket 004).
- Populating `assumptions` from `frame.assumptions` (ticket 002).
- Populating `rejection_dimension` from `RankedGoalComparisonOutcome::decisive_dimension` (ticket 003).
- Reordering `emit_plan_selection_events` (ticket 002).
- Observer Section 3 rendering of new fields (ticket 005).
- Golden test coverage (ticket 006).
- Promoting decisive classification from `decision_trace.rs` to always-on for success-path tags (Non-Goal in spec).

## Acceptance Criteria

### Tests That Must Pass

1. New: serde round-trip on each modified payload type with both empty-default and non-empty-payload values (`cargo test -p worldwake-core decision_event_payload`).
2. New: `RankedGoalComparisonDimensionTag` core mirror type satisfies the expected copy/ordering/serde bounds; runtime source-enum conversion remains ticket 003 scope.
3. Existing: `save_load.rs::tests::load_rejects_wrong_version` asserts that v69 bytes are rejected under v70; `save_to_bytes_roundtrip_preserves_decision_event_payloads` now includes non-empty new fields.
4. Existing focused suite passes: `cargo test -p worldwake-core`, `cargo test -p worldwake-sim`, `cargo test -p worldwake-ai`, `cargo test -p worldwake-cli`.
5. Workspace builds cleanly: `cargo build --workspace`.

### Invariants

1. Current-format saves roundtrip with the new fields; v69 saves are rejected after the bump per the repo's no-backward-compatibility policy.
2. `RankedGoalComparisonDimensionTag` is mechanically 1:1 with `RankedGoalComparisonDimension` — no semantic shifts, no merging, no narrowing.
3. No new authoritative state; all new fields are additive observability surfaces.
4. `SAVE_FORMAT_VERSION` strictly increases (69 → 70).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/decision_event_payload.rs::tests` — extend serde round-trip module with new-field coverage for the 6 affected payload structs and `RejectedAlternativeSummary`.
2. `crates/worldwake-core/src/decision_event_payload.rs::tests::decision_event_payload_types_satisfy_required_bounds` — covers the core mirror/reference type bounds.
3. `crates/worldwake-sim/src/save_load.rs::tests::save_to_bytes_roundtrip_preserves_decision_event_payloads` — extended current-format fixture with non-empty new fields; `load_rejects_wrong_version` confirms v69 rejection under v70.

### Commands

1. `cargo test -p worldwake-core decision_event_payload`
2. `cargo test -p worldwake-sim save_load`
3. `cargo build --workspace` (compile-coherence proof for the 64 construction-site updates)
4. `cargo test --workspace`
5. `./scripts/verify.sh`

Merge note: Ticket 001 bumps `SAVE_FORMAT_VERSION` 69→70; tickets 002–006 deliberately avoid additional bumps because they populate fields added by this ticket without changing serialized shape again.

## Outcome

Completed on 2026-05-06.

- Added `RankedGoalComparisonDimensionTag`, `BeliefRef`, `RecordRef`, `ObservationRef`, and `PlanAssumptionRef` in `worldwake-core::decision_event_payload`, with crate-root re-exports.
- Added `rejection_dimension`, `decisive_*`, and `assumptions` fields to the S136-owned payload structs, with placeholder empty/`None` construction across core, AI, sim, and CLI constructors.
- Bumped `SAVE_FORMAT_VERSION` from 69 to 70 and extended save/load current-format decision-payload fixtures to include non-empty new fields.
- Corrected the drafted save/load compatibility and conversion-site claims: v69 saves are rejected, and AI-side enum conversion is ticket 003 scope.

## Deviations

- Did not add a `worldwake-sim` conversion from `RankedGoalComparisonDimension` to `RankedGoalComparisonDimensionTag`; that would require `worldwake-sim` to depend on `worldwake-ai`, which violates the workspace crate graph.
- Did not add a pre-v70 forward-load test. The live save/load contract rejects older format versions after a format bump.

## Verification Result

- Passed `cargo test -p worldwake-core --lib decision_event_payload -- --list`
- Passed `cargo test -p worldwake-core --lib decision_event_payload`
- Passed `cargo test -p worldwake-sim --lib save_load -- --list`
- Passed `cargo test -p worldwake-sim --lib save_load`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `./scripts/verify.sh`
