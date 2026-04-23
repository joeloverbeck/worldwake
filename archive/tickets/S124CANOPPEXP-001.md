# S124CANOPPEXP-001: Add OpportunityExpectationKind metadata to committed plan carrier

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI committed-plan runtime carrier, save format version
**Deps**: `archive/specs/S124-canonical-opportunity-expectation-failure.md`, `archive/tickets/S124OPEXFAL-001.md` (landed)

## Problem

`S124OPEXFAL-001` landed `PlannedPlan.committed_source: Option<SourceKey>` at [`crates/worldwake-ai/src/planner_ops.rs:962`](../../crates/worldwake-ai/src/planner_ops.rs) so the AI-layer retained-plan path can preserve concrete source identity. That carrier tells the attribution path *which* source was trusted, but not *what kind of expectation* the plan committed to. Today the three AI-layer detection sites (observation, candidate generation, planning) and the single AI-layer writer (`apply_source_reliability_failure_observations`) exchange only `BTreeSet<SourceKey>`, so no detection site can tell the writer whether the violated expectation was `AcquireCommodity`-from-source or `RestockCommodity`-from-source, and no future non-acquisition expectation kind can share the substrate without a second normalization pass.

This ticket extends the committed-plan carrier with an explicit `OpportunityExpectationKind` tag and defines that enum in `worldwake-ai`. It also bumps `SAVE_FORMAT_VERSION` because `PlannedPlan` is persisted through the runtime save path.

## Assumption Reassessment (2026-04-23)

1. The committed-plan carrier exists at [`crates/worldwake-ai/src/planner_ops.rs:958`](../../crates/worldwake-ai/src/planner_ops.rs) with fields `goal`, `opportunity`, `committed_source: Option<SourceKey>` (line 962), `steps`, `total_estimated_ticks`, `terminal_kind`. The adoption site that populates `committed_source` is at [`crates/worldwake-ai/src/agent_tick/planning.rs:1405-1410`](../../crates/worldwake-ai/src/agent_tick/planning.rs), which calls `committed_source_for_offer(...)` from [`planner_ops.rs:1018`](../../crates/worldwake-ai/src/planner_ops.rs). Existing focused coverage: `adopt_selected_plan_populates_expected_commodity_assumption_immediately` at [`planning.rs:2621`](../../crates/worldwake-ai/src/agent_tick/planning.rs) and `refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection` at [`agent_tick/tests.rs:6180`](../../crates/worldwake-ai/src/agent_tick/tests.rs).
2. `SAVE_FORMAT_VERSION` is currently `42` at [`crates/worldwake-sim/src/save_load.rs:6`](../../crates/worldwake-sim/src/save_load.rs). `S124OPEXFAL-001` bumped 41 → 42 when it added `committed_source`; this ticket must bump 42 → 43 because the persisted `PlannedPlan` shape changes again. The version-check unit test `save_to_bytes_writes_current_format_version` exists in `save_load::tests` and will need its expected version updated.
3. Shared abstraction boundary under audit: the runtime carrier that travels from plan adoption in `planning.rs` through retained-plan observation (`observation.rs`), same-goal failure attribution (`planning.rs`), and the AI-layer reliability writer (`agent_tick/mod.rs`). Adding `expectation_kind: Option<OpportunityExpectationKind>` extends this carrier without introducing a parallel runtime path.
4. Live construction-site correction: the current branch constructs `PlannedPlan` through the canonical `PlannedPlan::new(...)` helper at call sites; there are zero direct `PlannedPlan { ... }` literals to patch across the workspace. The real field fallout is therefore the defining struct + constructor/builder in `planner_ops.rs`, the adoption path in `agent_tick/planning.rs`, and focused tests that must assert the new field's semantics.
5. Verification-surface correction: `worldwake-sim/src/save_load.rs` only round-trips opaque runtime bytes. It can truthfully prove the format-version bump, but it cannot by itself prove `PlannedPlan.expectation_kind` survives runtime decode. The honest positive round-trip seam for the new field is the existing `AgentDecisionRuntime` bincode round-trip test in [`crates/worldwake-ai/src/decision_runtime.rs`](../../crates/worldwake-ai/src/decision_runtime.rs), because that test serializes and deserializes the actual carrier containing `PlannedPlan`.
6. Mismatch + correction: spec's Q1a approval (reassessment session) kept D1 scoped to adding expectation-kind metadata; it did NOT add `supporting_place`/`supporting_entity` as separate fields beyond `SourceKey.entity`, because the existing `plan.opportunity.anchor` already provides place vs. entity distinction at the detection sites.

## Architecture Check

1. Extending the already-landed `PlannedPlan.committed_source` carrier with a sibling `expectation_kind: Option<OpportunityExpectationKind>` field keeps all committed provenance on one runtime path. Introducing a separate `CommittedOpportunityProvenance` struct would create a second runtime carrier for the same facts and force every reader to correlate two paths, violating FND-28.
2. No backwards-compatibility shim is introduced. The save format bumps to v43; old saves at v42 are rejected at load time via the existing version-mismatch path, consistent with project policy.
3. The new enum lives in `worldwake-ai` (not `worldwake-core`) because it is agent-decision runtime metadata — not durable cross-crate state or a scenario-authored profile. `SourceKey`, `OpportunityKey`, and `EntityId` (the core types the enum interacts with) already live in core.

## Verification Layers

1. `PlannedPlan.expectation_kind` is populated at adoption for source-backed acquisition opportunities and `None` for non-source-backed commits -> focused unit coverage at [`crates/worldwake-ai/src/agent_tick/planning.rs`](../../crates/worldwake-ai/src/agent_tick/planning.rs) extending the existing `adopt_selected_plan_populates_expected_commodity_assumption_immediately` test.
2. Save format v43 is written by the save header and the actual persisted runtime carrier round-trips the new field -> focused coverage split across [`crates/worldwake-sim/src/save_load.rs`](../../crates/worldwake-sim/src/save_load.rs) `save_to_bytes_writes_current_format_version` with `SAVE_FORMAT_VERSION = 43`, plus the existing `AgentDecisionRuntime` bincode round-trip test in [`crates/worldwake-ai/src/decision_runtime.rs`](../../crates/worldwake-ai/src/decision_runtime.rs) updated to exercise a `PlannedPlan` with `expectation_kind = Some(...)`.
3. Retained-plan read path continues to treat the same committed source identity as before (no behavioral regression) -> existing `refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection` at [`agent_tick/tests.rs:6180`](../../crates/worldwake-ai/src/agent_tick/tests.rs) and the survival golden at [`crates/worldwake-ai/tests/golden_survival_preferences.rs`](../../crates/worldwake-ai/tests/golden_survival_preferences.rs).
4. Single-layer ticket beyond those three surfaces — action-trace and event-log delta coverage is not applicable because no authoritative-action or event-log surfaces change; the carrier addition is observed only by the AI-layer runtime and the persisted save format.

## What to Change

### 1. Define `OpportunityExpectationKind` in worldwake-ai

Add a new enum in `crates/worldwake-ai/src/` (either in `planner_ops.rs` adjacent to the carrier definition, or a small new module like `opportunity_expectation.rs` re-exported from `lib.rs`):

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum OpportunityExpectationKind {
    AcquireCommodityFromConcreteSource,
    RestockCommodityFromConcreteSource,
}
```

The enum must derive `Serialize`/`Deserialize` because it becomes persisted runtime state on `PlannedPlan`. `Copy` + `Ord` + `Hash` keep it cheap to pass around alongside `SourceKey`.

### 2. Add `expectation_kind` field to `PlannedPlan`

Extend the struct at [`planner_ops.rs:958`](../../crates/worldwake-ai/src/planner_ops.rs):

```rust
pub struct PlannedPlan {
    pub goal: GoalKey,
    pub opportunity: worldwake_core::OpportunityKey,
    pub committed_source: Option<SourceKey>,
    pub expectation_kind: Option<OpportunityExpectationKind>,  // NEW
    pub steps: Vec<PlannedStep>,
    pub total_estimated_ticks: u32,
    pub terminal_kind: PlanTerminalKind,
}
```

Default to `None` in the constructor at [`planner_ops.rs:979`](../../crates/worldwake-ai/src/planner_ops.rs). Add a builder analogous to `with_committed_source(...)`: `with_expectation_kind(mut self, kind: Option<OpportunityExpectationKind>) -> Self`.

### 3. Derive the expectation kind at plan adoption

Extend [`planner_ops.rs:1018`](../../crates/worldwake-ai/src/planner_ops.rs) (`committed_source_for_offer`) or add a sibling helper `expectation_kind_for_offer(offer: &GoalOffer) -> Option<OpportunityExpectationKind>` that maps `GoalKind::AcquireCommodity` → `AcquireCommodityFromConcreteSource` and `GoalKind::RestockCommodity` → `RestockCommodityFromConcreteSource` when the offer has exactly one concrete source entity; returns `None` otherwise.

Update the adoption block at [`agent_tick/planning.rs:1400-1410`](../../crates/worldwake-ai/src/agent_tick/planning.rs) to populate both `committed_source` and `expectation_kind` from the ranked candidate's offer in the same conditional block.

### 4. Update the canonical constructor/builder and focused runtime fixtures

Because the live branch constructs `PlannedPlan` through `PlannedPlan::new(...)` rather than direct struct literals, the real implementation work is:

- `crates/worldwake-ai/src/planner_ops.rs` (struct field, constructor default, builder, offer helper)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (adoption path sets the field from the ranked offer)
- focused runtime/test fixtures that should exercise a non-`None` value (notably `crates/worldwake-ai/src/decision_runtime.rs`)

No repo-wide direct literal sweep is required on the live branch.

### 5. Bump `SAVE_FORMAT_VERSION`

Change `SAVE_FORMAT_VERSION: u32 = 42` at [`crates/worldwake-sim/src/save_load.rs:6`](../../crates/worldwake-sim/src/save_load.rs) to `43`. Update the `save_to_bytes_writes_current_format_version` unit test in `save_load::tests` to assert `43`.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — struct, constructor, builder, adoption helper)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export `OpportunityExpectationKind` if defined in a new module)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — runtime round-trip coverage for non-`None` expectation kind)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — adoption + focused adoption tests)
- `crates/worldwake-sim/src/save_load.rs` (modify — SAVE_FORMAT_VERSION bump + version assertion)

## Out of Scope

- The `OpportunityExpectationFailureIncident`, `ExpectationFailurePhase`, and `ExpectationFailureCause` runtime types — they are introduced in `S124CANOPPEXP-002` alongside their first consumers (the detection sites and writer).
- Changing detection-site output shape — detection sites still emit `BTreeSet<SourceKey>` after this ticket; the incident pipeline lands in `S124CANOPPEXP-002`.
- Evolving `apply_source_reliability_failure_observations(...)` — its signature stays `&BTreeSet<SourceKey>` after this ticket; the evolution lands in `S124CANOPPEXP-002`.
- Extending the enum to non-acquisition expectation kinds (e.g., `ConcreteTargetPresence`) — intentionally deferred until a concrete motivating deliverable arrives.
- Moving provenance onto `IntentionFrame` — expectation_kind stays on `PlannedPlan`, matching `committed_source`.

## Acceptance Criteria

### Tests That Must Pass

1. A new or extended unit test in `crates/worldwake-ai/src/agent_tick/planning.rs` (or its sibling test module) proves that plan adoption for `GoalKind::AcquireCommodity` with a single concrete source populates `PlannedPlan.expectation_kind = Some(OpportunityExpectationKind::AcquireCommodityFromConcreteSource)` alongside the already-set `committed_source`.
2. A new or extended unit test proves that plan adoption for a non-source-backed opportunity (no concrete source entity) leaves `expectation_kind = None`.
3. `save_to_bytes_writes_current_format_version` in `save_load::tests` asserts `SAVE_FORMAT_VERSION == 43`.
4. The existing `agent_decision_runtime_bincode_round_trip_preserves_all_fields` test is extended so the runtime's `current_plan` carries `expectation_kind = Some(AcquireCommodityFromConcreteSource)` and the value survives encode/decode.
5. Existing regression: `cargo test -p worldwake-ai --lib agent_tick::tests::refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection -- --exact`
6. Existing regression: `cargo test -p worldwake-ai --lib agent_tick::planning::tests::adopt_selected_plan_populates_expected_commodity_assumption_immediately -- --exact`
7. Existing regression: `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The canonical `PlannedPlan::new(...)` constructor initializes `expectation_kind` to `None`, and the adoption path is the only production writer that upgrades it to `Some(...)` for source-backed acquire/restock opportunities.
2. When a plan adopts a `GoalKind::AcquireCommodity` or `GoalKind::RestockCommodity` opportunity with exactly one concrete source entity in `evidence_entities`, `expectation_kind` is populated with the corresponding variant and is in lockstep with `committed_source = Some(SourceKey { entity, commodity })`. A plan cannot have `committed_source = Some(...)` and `expectation_kind = None` for these two goal kinds, or vice versa.
3. Save format v43 is the only version accepted by the current loader; v42 saves fail at version-check time via the existing `SaveError::VersionMismatch` path.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/planning.rs` (tests module) — extend or add focused coverage so plan adoption asserts `expectation_kind` alongside `committed_source` for both source-backed and non-source-backed opportunities.
2. `crates/worldwake-sim/src/save_load.rs` (tests module) — update `save_to_bytes_writes_current_format_version` to expect v43.
3. `crates/worldwake-ai/src/decision_runtime.rs` (tests module) — extend `agent_decision_runtime_bincode_round_trip_preserves_all_fields` so the runtime's `current_plan` exercises `expectation_kind = Some(AcquireCommodityFromConcreteSource)` and preserves it through bincode round-trip.

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::adopt_selected_plan_populates_expected_commodity_assumption_immediately -- --exact`
2. `cargo test -p worldwake-ai --lib decision_runtime::tests::agent_decision_runtime_bincode_round_trip_preserves_all_fields -- --exact`
3. `cargo test -p worldwake-ai --lib agent_tick::tests::refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection -- --exact`
4. `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_writes_current_format_version -- --exact`
5. `cargo test -p worldwake-ai`
6. `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
7. `scripts/verify.sh`

## Outcome

Completed on 2026-04-23.

- Added `OpportunityExpectationKind` in `worldwake-ai` and extended `PlannedPlan` with `expectation_kind: Option<OpportunityExpectationKind>`.
- Kept the committed-opportunity metadata on the canonical runtime carrier: `PlannedPlan::new(...)` defaults the new field to `None`, `with_expectation_kind(...)` supports explicit fixtures, and plan adoption now derives both `committed_source` and `expectation_kind` from the same ranked offer.
- Bumped `SAVE_FORMAT_VERSION` from `42` to `43`.
- Extended focused proof at the live seams: adoption tests in `agent_tick/planning.rs`, runtime bincode round-trip in `decision_runtime.rs`, save-header version assertion in `save_load.rs`, plus the existing read-phase regression and broader AI/workspace verification.

## Deviations

- Reassessment corrected the drafted round-trip seam: `worldwake-sim/src/save_load.rs` only proves the save-header version because it stores opaque runtime bytes. The truthful positive round-trip proof for `PlannedPlan.expectation_kind` lands in `worldwake-ai/src/decision_runtime.rs`, where the actual runtime carrier is serialized and deserialized.
- Reassessment also corrected the drafted constructor fallout: the live branch uses `PlannedPlan::new(...)` at call sites, so no repo-wide direct `PlannedPlan { ... }` literal sweep was required.

## Verification Result

- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::adopt_selected_plan_populates_expected_commodity_assumption_immediately -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::planning::tests::adopt_selected_plan_leaves_expectation_kind_empty_without_concrete_source -- --exact`
- Passed `cargo test -p worldwake-ai --lib decision_runtime::tests::agent_decision_runtime_bincode_round_trip_preserves_all_fields -- --exact`
- Passed `cargo test -p worldwake-ai --lib agent_tick::tests::refresh_runtime_for_read_phase_uses_committed_source_for_local_failure_detection -- --exact`
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_writes_current_format_version -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-ai --test golden_survival_preferences survival_preferences_keeps_proactive_diversification_alive_under_survival -- --ignored --exact --test-threads=1`
- Passed `./scripts/verify.sh`
