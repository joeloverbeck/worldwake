# S147HTNMETDEC-015: Complete direct and escort HTN method goldens

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - selector direct-observation location bridge, selected-method plan provenance, typed method-failure producer, save format version 90, and golden coverage/docs.
**Deps**: `archive/tickets/S147HTNMETDEC-014.md` (report-backed `FulfillBountyInvestigation` golden slice)

## Problem

`S147HTNMETDEC-014` landed the stable report-backed `FulfillBountyInvestigation` D10 seam. The remaining original S147 non-production D10 narratives were `FulfillBountyDirect`, escort method selection/failure, and typed `Discrepancy::MethodFailure(MethodFailureContext)` production through a reachable planning/action failure path.

This ticket landed the direct-bounty and escort generated-candidate selector goldens and the typed method-failure producer substrate. The end-to-end method-failure golden remains split to `archive/tickets/S147HTNMETDEC-016.md` because this ticket's proof is lower-layer runtime producer coverage, not a full autonomous golden failure narrative.

## Assumption Reassessment (2026-05-17)

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` already proved `ProduceWithGather` selector/fallback, autonomous production method trace, and report-backed `FulfillBountyInvestigation` selector/autonomous trace coverage.
2. Direct-bounty reassessment showed the generated direct-observation bounty candidate carried bounty/target/place evidence, but `FulfillBountyDirect` could not select because `TargetLastSeenKnown` only saw `BelievedTargetLocation` and ignored the direct-observation `BelievedEntityState.last_known_place` already present in the actor-local belief state.
3. The selector fix is intentionally narrow: `MethodSelector` may fall back to `known_entity_beliefs(actor).last_known_place` only for `PerceptionSource::DirectObservation`. Report-backed bounty evidence still selects `FulfillBountyInvestigation` instead of being upgraded to a direct target-location claim.
4. Escort candidate reassessment showed a generated `EscortToSafety` offer can select `EscortToHome` once the selector reads the same direct-observation location bridge for the escortee and the fixture seeds local wound/location beliefs. The proof asserts a non-current escort destination instead of hard-coding one authored place.
5. Typed method failure needed a reachable producer before golden assertions. The selected method id is now persisted on `PlannedPlan.method_id` from the strategic `MethodPlanAttemptTrace`; when an unclassified failure occurs while a method-selected plan is active, `handle_plan_failure` emits `Discrepancy::MethodFailure(MethodFailureContext { kind: SubgoalUnachievable, subgoal_index: None, ... })` instead of the generic improper-planning-state discrepancy.
6. Because active `PlannedPlan` state is save-loaded through AI runtime state, the save format version moved from 89 to 90 with no compatibility shim.
7. Shared abstraction boundary under audit: generated `GoalOffer` evidence -> `MethodSelector` preconditions -> strategic `MethodPlanAttemptTrace` -> persisted `PlannedPlan.method_id` -> live action/failure producer.

## Architecture Check

1. The landed selector bridge preserves FND-14/FND-20 by reading only actor-relative direct observations; it does not query global world state or infer target location from reports.
2. Method failure attribution now has one canonical runtime carrier for selected-method provenance after planning: `PlannedPlan.method_id`.
3. No backwards-compatibility shim was added. Save format 89 is rejected, and the new version 90 encodes the changed persisted runtime shape.

## Verified Layers

1. `FulfillBountyDirect` selection -> generated direct-observation bounty candidate evidence plus snapshot-backed `MethodSelector` proof in `golden_htn_methods`.
2. Escort method selection -> generated escort candidate evidence plus snapshot-backed `MethodSelector` proof in `golden_htn_methods`.
3. Typed method failure -> focused `failure_handling` unit proof that a method-selected, otherwise-unclassified plan failure emits `Discrepancy::MethodFailure(MethodFailureContext)`.
4. Save format -> exact save-version, round-trip, and old-version rejection tests in `worldwake-sim`.
5. Golden metadata -> `python3 scripts/golden_inventory.py --write --check-docs`.
6. Affected AI behavior -> `cargo test -p worldwake-ai --test golden_htn_methods` and `cargo test -p worldwake-ai`.

## What Changed

### 1. Direct bounty selector bridge

`MethodSelector` now resolves method target location from `BeliefView::believed_target_location` first, then from direct-observation `BelievedEntityState.last_known_place`. Reported sources are deliberately excluded from the fallback so report-backed bounty evidence still selects investigation.

### 2. Escort method coverage

`golden_htn_methods` now includes generated escort-offer selector and deterministic replay coverage. The fixture seeds local actor knowledge for escortee injury and location and asserts method id `EscortToHome`.

### 3. Typed method-failure producer

`PlannedPlan` now carries the selected `MethodSchemaId`. Planning stores that id from the selected method trace, and failure handling uses it to produce a typed `Discrepancy::MethodFailure` for method-selected failures that are otherwise not classified by a more specific discrepancy.

## Files Touched

- `crates/worldwake-ai/src/htn/selector.rs` (modified)
- `crates/worldwake-ai/src/planner_ops.rs` (modified)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modified)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modified)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modified)
- `crates/worldwake-ai/src/failure_handling.rs` (modified)
- `crates/worldwake-ai/tests/golden_htn_methods.rs` (modified)
- `crates/worldwake-sim/src/save_load.rs` (modified)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/htn-methods.md` (regenerated)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `archive/tickets/S147HTNMETDEC-016.md` (new follow-up)
- `archive/specs/S147-htn-method-decomposition.md` (truth-sync)
- `specs/IMPLEMENTATION-ORDER.md` (truth-sync)

## Out of Scope

- Reworking production-method and report-backed `FulfillBountyInvestigation` coverage already landed by earlier S147 tickets.
- Adding story-beat methods or method-only goals.
- Fabricating method failures directly in traces without a live producer.
- Full end-to-end method-failure golden coverage; that remaining D10 narrative is owned by `archive/tickets/S147HTNMETDEC-016.md`.

## Completed Acceptance

### Tests That Passed

1. Stable goldens land for the direct and escort method paths that live substrate can honestly prove.
2. The remaining end-to-end method-failure golden is assigned to `archive/tickets/S147HTNMETDEC-016.md` with the exact missing proof boundary.
3. `python3 scripts/golden_inventory.py --write --check-docs` passes after golden metadata changes.
4. `cargo test -p worldwake-ai --test golden_htn_methods` passes.
5. `cargo test -p worldwake-ai` passes.

### Invariants

1. Methods remain lawful pursuit patterns, not story beats.
2. Generated candidates carry lawful evidence; methods do not query global world state to compensate.
3. Method failure proof uses a live typed failure producer, not a fabricated trace.
4. Flat GOAP fallback remains available when methods are disabled or no method preconditions match.

## Test Plan Result

### Landed Tests

1. `crates/worldwake-ai/tests/golden_htn_methods.rs` - generated direct-bounty and escort method selection coverage plus deterministic replay.
2. `crates/worldwake-ai/src/failure_handling.rs` - lower-layer typed method-failure producer coverage.
3. `crates/worldwake-sim/src/save_load.rs` - save format version and no-shim rejection coverage for the persisted plan provenance change.

### Observed Commands

1. `cargo test -p worldwake-ai --test golden_htn_methods`
2. `cargo test -p worldwake-ai failure_handling::tests::method_selected_unclassified_failure_records_method_failure_discrepancy -- --exact`
3. `cargo test -p worldwake-sim --lib save_load::tests::save_format_version_is_90_after_s147_method_plan_provenance_landing -- --exact`
4. `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`
5. `cargo test -p worldwake-sim --lib save_load::tests::load_rejects_pre_s147_method_plan_provenance_version_89_without_migration_shim -- --exact`
6. `python3 scripts/golden_inventory.py --write --check-docs`
7. `cargo test -p worldwake-ai`
8. `cargo test -p worldwake-sim`

## Outcome

Completed on 2026-05-17.

- Landed generated-candidate direct-bounty selector coverage for `FulfillBountyDirect`.
- Landed generated-candidate escort selector coverage for `EscortToHome`.
- Persisted selected method provenance on active plans and connected unclassified method-selected failures to `Discrepancy::MethodFailure(MethodFailureContext)`.
- Bumped save format version 89 to 90 because active plan provenance is persisted runtime state.
- Regenerated golden inventory/docs for the new HTN method scenario metadata.

## Deviations

- The original ticket grouped direct, escort, and method-failure goldens. Reassessment found the direct and escort paths were ready for generated-candidate selector goldens, while method failure first needed runtime producer substrate. The producer landed here; the full end-to-end method-failure golden is split to `archive/tickets/S147HTNMETDEC-016.md`.
- A draft combined Cargo command with multiple test filters was invalid and was discarded as non-proof. The save-format checks were rerun as separate exact tests and those observed runs are recorded below.

## Verification Result

- Passed `cargo test -p worldwake-ai --test golden_htn_methods` (38 tests).
- Passed `cargo test -p worldwake-ai failure_handling::tests::method_selected_unclassified_failure_records_method_failure_discrepancy -- --exact`.
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_format_version_is_90_after_s147_method_plan_provenance_landing -- --exact`.
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`.
- Passed `cargo test -p worldwake-sim --lib save_load::tests::load_rejects_pre_s147_method_plan_provenance_version_89_without_migration_shim -- --exact`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs` (`53 files, 53 contributing files, 253 tests, 195 scenario blocks`).
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo test -p worldwake-sim`.
