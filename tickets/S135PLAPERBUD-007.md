# S135PLAPERBUD-007: Golden tests for perception omission scenarios

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No — test-only
**Deps**: `archive/tickets/S135PLAPERBUD-001.md`, `archive/tickets/S135PLAPERBUD-002.md`, `archive/tickets/S135PLAPERBUD-003.md`, S135PLAPERBUD-004, S135PLAPERBUD-005

## Problem

Per S135's "Validation and Falsification" section, three goldens prove the spec's contract: (1) crowded place + budget cap → typed `OverBudget` records, no planner-side discard, (2) need-weighted policy under hunger pressure → priority preserved, (3) action revalidation against omitted entity → `Discrepancy::Omission(reason)`. These tests close the loop on the architectural change and protect against regression in long-running survival scenarios.

## Assumption Reassessment (2026-05-05)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Target file `crates/worldwake-ai/tests/golden_perception_omission.rs` does not exist (validated during S135 reassessment). Existing perception-related golden test: `crates/worldwake-ai/tests/golden_perception_exposure.rs::golden_observation_budget_prioritizes_agents_and_facilities_over_waste` (line 606). It exercises per-tick priority truncation but not omission records or planner-side cap removal. This new golden file is additive to that file's coverage.
2. Determinism: spec Goal 5 requires omission entries emit in `BTreeMap`-stable order with FIFO ring-buffer eviction. Tests assert against deterministic ordering, not just count-based aggregates.
3. Three survival goldens (`scenarios/survival-baseline.ron`, `scenarios/survival-scattered.ron`, `scenarios/survival-contested.ron`) at ≤4 agents each have low entity density and produce identical canonical state hashes pre/post S135 (per spec's regression bound). This ticket runs them as a determinism regression check, not as part of the new file.
4. Live `GoalKind` under test (Test 3): an action handler's revalidation path runs against an omitted entity. The exact `GoalKind` chosen for the revalidation test should be one whose handler reaches `effect_sink_hypothetical.rs` revalidation — likely a needs-driven goal (`SatisfyNeed`) since those use `effect_sink_hypothetical.rs` heavily. Confirm during implementation that the chosen `GoalKind` actually flows through the revalidation surface modified by ticket 004.
5. Harness boundary (per `docs/precision-rules.md` Rule 3): tests 1 and 2 (perception write paths) can use a local needs-only harness because they only exercise perception. Test 3 (revalidation) needs full action registries because the failure path runs through the action handler revalidation — confirm via reassessment of `effect_sink_hypothetical.rs` integration with the test harness.
6. `Discrepancy::Omission` (ticket 004), `RootCandidateTrace.omitted_anchor` (ticket 005), and `ObservationOmissionLog` (tickets 001, 003) all need to exist for the new goldens to compile.

## Architecture Check

1. The three scenarios test orthogonal axes: (1) the budget-truncation write path (tickets 001+003), (2) the priority composition (tickets 001+003 + S105's existing `compute_observation_priority`), (3) the revalidation attribution (tickets 002+003+004+005). Combined, they demonstrate the "single perception cap" contract end to end.
2. Negative-test coverage: each scenario also asserts the spec's negative invariant — an agent's `ObservationOmissionLog` never contains an entity that is also in their `BeliefStore` for the same tick.
3. Determinism bound: the existing 1440-tick survival goldens (`survival-baseline.ron`, etc.) are not modified. They stay green post-S135 because their per-place entity densities are well under both `observation_budget` and the (deleted) `max_snapshot_entities_per_place`. The hash-regression guard in this ticket proves it.
4. **Phase distinction**: tests cover candidate-emission (Test 3 via `RootCandidateTrace.omitted_anchor`) and revalidation (Test 3 via `Discrepancy::Omission`) as distinct surfaces — both attribution paths are exercised separately.
5. **Verification surface mapping** (per `docs/precision-rules.md` Rule 5):
   - perception write contract → focused belief-store assertion (Test 1)
   - priority composition → priority-class observation order (Test 2)
   - candidate emission attribution → decision-trace `RootCandidateTrace.omitted_anchor` (Test 3)
   - revalidation attribution → action-trace / handler error variant `Discrepancy::Omission` (Test 3)

## Verification Layers

1. Crowded-place scenario produces N `OverBudget` records → golden assertion on `ObservationOmissionLog.entries` count and discriminant (focused belief-store layer).
2. Need-weighted scenario preserves priority → golden assertion comparing observed-entity ordering against the priority-class expectation under hunger pressure (focused perception layer).
3. Revalidation scenario produces `Discrepancy::Omission(reason)` → golden assertion through the action-trace handler-error surface AND decision-trace `RootCandidateTrace.omitted_anchor` field (cross-layer: action-trace and decision-trace).
4. Existing survival goldens produce identical canonical state hashes pre/post S135 → golden hash regression check.

## What to Change

### 1. Create `golden_perception_omission.rs`

New file at `crates/worldwake-ai/tests/golden_perception_omission.rs`. Three test functions plus a determinism regression check:

#### Test 1: `golden_perception_omission_overbudget_writes`

- Setup: agent with `PerceptionProfile { observation_budget: 24, omission_log_capacity: 50, salience_policy: SaliencePolicy::PriorityWithNeedBoost, ..default }`. Co-located with 60 non-agent entities (mix of items, facilities, dropped corpses).
- Run for 1 tick.
- Assertions:
  - `ObservationOmissionLog.entries.len() == 36`
  - All 36 entries have `reason: OmissionReason::OverBudget { budget: 24, candidates_seen: 60 }`
  - The 24 observed entities (in `BeliefStore.known_entity_beliefs`) and the 36 omitted entities are disjoint sets
  - Planner snapshot's per-place entity list equals exactly the 24 observed entities (no re-truncation, no missing entities)
  - Omission entries are in deterministic order (sorted by `omitted_entity` ascending within the tick)

#### Test 2: `golden_perception_omission_need_weighted_priority`

- Setup: agent with hunger above the spec's salience threshold and `salience_policy: PriorityWithNeedBoost`. Co-located with 30 entities including 10 food-source items and 20 unrelated items, with `observation_budget: 12`.
- Run for 1 tick.
- Assertions:
  - All 10 food-source items appear in `BeliefStore.known_entity_beliefs` (priority class + need boost wins them slots)
  - The 18 lowest-priority unrelated items end up in `ObservationOmissionLog` with `OmissionReason::OverBudget`
  - Priority composition: each food-item's `compute_observation_priority` score (computed against the agent's hunger-elevated `need_salience_boost`) exceeds each unrelated-item's score

#### Test 3: `golden_perception_omission_revalidation_typed_reason`

- Setup: agent with `omission_log_capacity: 16`. Pre-populate `ObservationOmissionLog` with one current bounded-log entry for entity X while X is absent from the planning snapshot. Configure the agent with a goal whose action handler revalidates against X (likely a `SatisfyNeed` goal whose target requires X).
- Run for 1 tick.
- Assertions:
  - Action lifecycle (action-trace surface) returns `Err(Discrepancy::Omission(reason))` matching the log entry's reason at the revalidation site
  - Decision-trace surface: `RootCandidateTrace.omitted_anchor == Some(reason)` for the discarded candidate that referenced X
  - Negative invariant: X is NOT in `BeliefStore.known_entity_beliefs` at any tick (the omission entry persists as the cause)

### 2. Determinism regression check

Add a determinism regression test in the same file (or extend an existing harness) that runs `survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron` for the canonical 1440 ticks and asserts the canonical state hash matches the pre-S135 baseline. This proves the change is a no-op for low-density scenarios. Use the existing `golden_harness/soak_world.rs` infrastructure.

### 3. Negative invariant check

In each test, after running for the test's tick budget, assert: for every agent and every tick, `ObservationOmissionLog.entries.iter().map(|e| e.omitted_entity).collect::<BTreeSet<_>>()` and `BeliefStore.known_entity_beliefs.keys().collect::<BTreeSet<_>>()` are disjoint sets.

## Files to Touch

- `crates/worldwake-ai/tests/golden_perception_omission.rs` (new)

## Out of Scope

- Modifying any existing golden test — only the new file plus the determinism regression check.
- Adding new scenarios to `scenarios/*.ron` — the new goldens construct their setup in-line via the test builder.
- Cross-agent omission correlation — defer to future specs.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_perception_omission` passes (all three test functions + determinism regression check).
2. `cargo test -p worldwake-ai` passes (full AI suite — including existing survival goldens unchanged).
3. `./scripts/verify.sh` passes.

### Invariants

1. After perception runs, `ObservationOmissionLog.entries.iter().map(|e| e.omitted_entity)` and `BeliefStore.known_entity_beliefs.keys()` are disjoint sets, for every agent at every tick.
2. The planner snapshot's per-place entity list equals the agent's accumulated belief observations at that place — no per-place cap (proves ticket 003's cap removal).
3. Survival goldens produce identical canonical state hashes pre/post S135 (proves the spec's regression bound).
4. `Discrepancy::Omission(reason)` carried through revalidation matches `RootCandidateTrace.omitted_anchor` carried through candidate emission for the same agent/entity pair (cross-phase consistency).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_perception_omission.rs::golden_perception_omission_overbudget_writes` — proves D1+D2+D3 (perception write path).
2. `crates/worldwake-ai/tests/golden_perception_omission.rs::golden_perception_omission_need_weighted_priority` — proves S105 priority composition unchanged under `SaliencePolicy::PriorityWithNeedBoost` and S135's omission-write integration.
3. `crates/worldwake-ai/tests/golden_perception_omission.rs::golden_perception_omission_revalidation_typed_reason` — proves D4+D5+D7 (typed `Discrepancy::Omission` attribution + `RootCandidateTrace.omitted_anchor` annotation).
4. `crates/worldwake-ai/tests/golden_perception_omission.rs::golden_perception_omission_survival_hash_regression` (or equivalent name) — runs the three survival goldens for 1440 ticks and asserts canonical state hash unchanged from pre-S135 baseline.

### Commands

1. `cargo test -p worldwake-ai --test golden_perception_omission`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`
