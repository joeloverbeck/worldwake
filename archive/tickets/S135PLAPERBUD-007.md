# S135PLAPERBUD-007: Golden tests for perception omission scenarios

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No — test-only
**Deps**: `archive/tickets/S135PLAPERBUD-001.md`, `archive/tickets/S135PLAPERBUD-002.md`, `archive/tickets/S135PLAPERBUD-003.md`, `archive/tickets/S135PLAPERBUD-004.md`, `archive/tickets/S135PLAPERBUD-005.md`

## Problem

Per S135's "Validation and Falsification" section, three goldens prove the spec's contract: (1) crowded place + budget cap -> typed `OverBudget` records and no planner-side cap, (2) need-weighted policy under hunger pressure -> priority preserved, (3) revalidation against an omitted entity -> `Discrepancy::Omission(reason)`. These tests close the loop on the architectural change without changing survival scenario behavior.

## Assumption Reassessment (2026-05-05)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Target file `crates/worldwake-ai/tests/golden_perception_omission.rs` does not exist (validated during S135 reassessment). Existing perception-related golden test: `crates/worldwake-ai/tests/golden_perception_exposure.rs::golden_observation_budget_prioritizes_agents_and_facilities_over_waste` (line 606). It exercises per-tick priority truncation but not omission records or planner-side cap removal. This new golden file is additive to that file's coverage.
2. Determinism: spec Goal 5 requires omission entries emit in `BTreeMap`-stable order with FIFO ring-buffer eviction. Tests assert against deterministic ordering, not just count-based aggregates.
3. The live repo has no checked-in older baseline hash for comparing long-run survival state across the S135 migration. Existing survival goldens own the 1440-tick survival contracts; this ticket owns the new omission-specific golden coverage.
4. Live `GoalKind` under test (Test 3): implementation uses a `ShareBelief` goal exact-bound to an omitted listener to exercise `RootCandidateTrace.omitted_anchor`, then exercises `effect_sink_hypothetical.rs` revalidation directly with the same omitted entity.
5. Harness boundary (per `docs/precision-rules.md` Rule 3): tests 1 and 2 (perception write paths) can use a local needs-only harness because they only exercise perception. Test 3 (revalidation) needs full action registries because the failure path runs through the action handler revalidation — confirm via reassessment of `effect_sink_hypothetical.rs` integration with the test harness.
6. `Discrepancy::Omission` (ticket 004), `RootCandidateTrace.omitted_anchor` (ticket 005), and `ObservationOmissionLog` (tickets 001, 003) all need to exist for the new goldens to compile.

## Assumption Reassessment (2026-05-06)

1. The S135 substrate is already live: `ObservationOmissionLog`, `OmissionReason`, `Discrepancy::Omission`, `GoalBeliefView::observation_omission_log`, `RootCandidateTrace.omitted_anchor`, observer omission rendering, and the removed `CognitiveProfile.max_snapshot_entities_per_place` path all exist on the branch.
2. The drafted same-place snapshot assertion is stale. `docs/planner-contracts.md` states that the actor's current place remains an authoritative planner-visible local surface, so the live no-second-cap proof is that all 60 co-located local entities remain snapshot-visible while perception still records the 36 attention-budget omissions.
3. The drafted older-baseline state-hash regression is not an executable live proof because the repo does not carry a separate older S135 hash baseline for this test. Existing survival goldens already own the long-run survival contracts; this ticket owns the new S135 omission golden file plus generated golden inventory/docs.
4. The revalidation proof is bound to the live carrier chain rather than a full autonomous action lifecycle: `search_plan` exposes `RootCandidateTrace.omitted_anchor`, and `HypotheticalEffectSink` revalidation returns the matching `Discrepancy::Omission(reason)` for the same omitted entity.

## Architecture Check

1. The three scenarios test orthogonal axes: (1) the budget-truncation write path (tickets 001+003), (2) the priority composition (tickets 001+003 + S105's existing `compute_observation_priority`), (3) the revalidation attribution (tickets 002+003+004+005). Combined, they demonstrate the "single perception cap" contract end to end.
2. Negative-test coverage: each scenario also asserts the spec's negative invariant — an agent's `ObservationOmissionLog` never contains an entity that is also in their `BeliefStore` for the same tick.
3. Determinism bound: the new over-budget golden uses equal-priority lots and asserts deterministic omission ordering. Existing 1440-tick survival goldens remain unchanged and continue to own the long-run survival contracts; no older-baseline state-hash fixture is added here.
4. **Phase distinction**: tests cover candidate-emission (Test 3 via `RootCandidateTrace.omitted_anchor`) and revalidation (Test 3 via `Discrepancy::Omission`) as distinct surfaces — both attribution paths are exercised separately.
5. **Verification surface mapping** (per `docs/precision-rules.md` Rule 5):
   - perception write contract → focused belief-store assertion (Test 1)
   - priority composition → priority-class observation order (Test 2)
   - candidate emission attribution → decision-trace `RootCandidateTrace.omitted_anchor` (Test 3)
   - revalidation attribution → `HypotheticalEffectSink` revalidation error variant `Discrepancy::Omission` (Test 3)

## Verification Layers

1. Crowded-place scenario produces N `OverBudget` records → golden assertion on `ObservationOmissionLog.entries` count and discriminant (focused belief-store layer).
2. Need-weighted scenario preserves priority → golden assertion comparing observed-entity ordering against the priority-class expectation under hunger pressure (focused perception layer).
3. Revalidation scenario produces `Discrepancy::Omission(reason)` → golden assertion through the hypothetical revalidation surface AND decision-trace `RootCandidateTrace.omitted_anchor` field (cross-layer: revalidation and decision-trace).
4. Generated golden inventory/docs include the new S135 scenarios.

## What to Change

### 1. Create `golden_perception_omission.rs`

New file at `crates/worldwake-ai/tests/golden_perception_omission.rs`. Three test functions:

#### Test 1: `golden_perception_omission_overbudget_writes`

- Setup: agent with `PerceptionProfile { observation_budget: 24, omission_log_capacity: 64, salience_policy: SaliencePolicy::PriorityWithNeedBoost, ..default }`. Co-located with 60 equal-priority waste lots.
- Run for 1 tick.
- Assertions:
  - `ObservationOmissionLog.entries.len() == 36`
  - All 36 entries have `reason: OmissionReason::OverBudget { budget: 24, candidates_seen: 60 }`
  - The observed entities (in `BeliefStore.known_entities`) and the 36 omitted entities are disjoint sets
  - Same-place planner snapshot still includes all 60 co-located lots, proving no second local entity cap
  - Omission entries are in deterministic order (sorted by `omitted_entity` ascending within the tick)

#### Test 2: `golden_perception_omission_need_weighted_priority`

- Setup: agent with hunger above the spec's salience threshold and `salience_policy: PriorityWithNeedBoost`. Co-located with 30 entities including 10 food-source items and 20 unrelated items, with `observation_budget: 12`.
- Run for 1 tick.
- Assertions:
  - All 10 food-source items appear in `BeliefStore.known_entities` (priority class + need boost wins them slots)
  - The 18 lowest-priority unrelated items end up in `ObservationOmissionLog` with `OmissionReason::OverBudget`
  - Priority composition is proved through the final retained/omitted surface: food lots are retained while only waste lots are omitted.

#### Test 3: `golden_perception_omission_revalidation_typed_reason`

- Setup: pre-populate `ObservationOmissionLog` with one current bounded-log entry for entity X while X is absent from the planning snapshot. Configure a `ShareBelief` goal exact-bound to X and run planner search over the live full action registries, then run hypothetical co-location revalidation against X.
- Assertions:
  - `HypotheticalEffectSink` revalidation returns `Err(Discrepancy::Omission(reason))` matching the log entry's reason at the revalidation site
  - Decision-trace surface: `RootCandidateTrace.omitted_anchor == Some(reason)` for the discarded candidate that referenced X
  - Negative invariant: X is NOT in `BeliefStore.known_entities` (the omission entry persists as the cause)

### 2. Negative invariant check

In perception tests, after running for the test's tick budget, assert: `ObservationOmissionLog.entries.iter().map(|e| e.omitted_entity).collect::<BTreeSet<_>>()` and `BeliefStore.known_entities.keys().collect::<BTreeSet<_>>()` are disjoint sets.

## Files to Touch

- `crates/worldwake-ai/tests/golden_perception_omission.rs` (new)
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-index.md`
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-scenario-details/perception-omission.md`
- Existing generated golden detail pages with refreshed source-line references from `scripts/golden_inventory.py`
- `archive/specs/S135-planner-perception-budget.md`

## Out of Scope

- Modifying existing golden test source files.
- Adding new scenarios to `scenarios/*.ron` — the new goldens construct their setup in-line via the test builder.
- Cross-agent omission correlation — defer to future specs.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_perception_omission` passes (all three S135 test functions plus shared golden harness tests compiled into that integration binary).
2. `cargo test -p worldwake-ai` passes (full AI suite — including existing survival goldens unchanged).
3. `./scripts/verify.sh` passes, or the live wrapper-equivalent gate set is recorded if the wrapper is not run.

### Invariants

1. After perception runs, `ObservationOmissionLog.entries.iter().map(|e| e.omitted_entity)` and `BeliefStore.known_entities.keys()` are disjoint sets for the tested observer.
2. The same-place planner snapshot includes all 60 co-located local lots — no per-place cap (proves ticket 003's cap removal under the live current-place planner contract).
3. New generated golden inventory/docs include scenarios 381-383.
4. `Discrepancy::Omission(reason)` carried through revalidation matches `RootCandidateTrace.omitted_anchor` carried through candidate emission for the same agent/entity pair (cross-phase consistency).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_perception_omission.rs::golden_perception_omission_overbudget_writes` — proves D1+D2+D3 (perception write path).
2. `crates/worldwake-ai/tests/golden_perception_omission.rs::golden_perception_omission_need_weighted_priority` — proves S105 priority composition unchanged under `SaliencePolicy::PriorityWithNeedBoost` and S135's omission-write integration.
3. `crates/worldwake-ai/tests/golden_perception_omission.rs::golden_perception_omission_revalidation_typed_reason` — proves D4+D5+D7 (typed `Discrepancy::Omission` attribution + `RootCandidateTrace.omitted_anchor` annotation).

### Commands

1. `cargo test -p worldwake-ai --test golden_perception_omission`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-06.

- Added `crates/worldwake-ai/tests/golden_perception_omission.rs` with scenarios 381-383.
- Regenerated golden inventory/docs, including the new `docs/generated/golden-scenario-details/perception-omission.md` page and refreshed generated source-line references.
- Truth-synced this ticket and `archive/specs/S135-planner-perception-budget.md` to the live current-place planner contract and removed the non-executable older-baseline hash-regression claim.
- Resolved the post-ticket-review generated-doc blocker by making the source scenario metadata generator-friendly and regenerating complete `Setup`, `Proves`, and `Cross-system chain` prose for scenarios 381-383.

## Deviations

- The over-budget test uses 60 equal-priority waste lots rather than a mixed entity set so deterministic omission ordering is the owned assertion rather than incidental priority-class ordering.
- Same-place planner proof asserts that all 60 co-located local lots remain planner-visible. This follows `docs/planner-contracts.md`; omitted entities can still attribute remote/absent snapshot anchors through `RootCandidateTrace.omitted_anchor`.
- The revalidation proof uses `HypotheticalEffectSink` directly plus `search_plan` root-candidate traces, not a fully autonomous action lifecycle.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_perception_omission -- --list`.
- Passed `cargo test -p worldwake-ai --test golden_perception_omission`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `cargo test -p worldwake-ai`.
- Passed `./scripts/verify.sh` (live gates: `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo run -p worldwake-cli --bin scenario-coverage -- --check`).
- Passed `python3 scripts/golden_inventory.py --write --check-docs` after scenario metadata cleanup.
- Inspected regenerated `docs/generated/golden-scenario-details/perception-omission.md` and `docs/generated/golden-scenario-index.md`; scenarios 381-383 now contain complete `Setup`, `Proves`, and `Cross-system chain` prose.
- Passed `cargo test -p worldwake-ai --test golden_perception_omission` after scenario metadata cleanup.
