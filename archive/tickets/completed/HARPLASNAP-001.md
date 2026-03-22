# HARPLASNAP-001: Define and enforce planner snapshot fidelity for institutional domain data

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — focused `worldwake-ai` snapshot/parity regression coverage; no new production architecture
**Deps**: `docs/FOUNDATIONS.md`, `specs/E16b-force-legitimacy-and-jurisdiction-control.md`, `archive/tickets/completed/E16BFORLEGJURCON-009.md`, `archive/tickets/E16DPOLPLAN-023.md`

## Problem

The motivating force-law regression already exposed the real architectural risk: planner snapshotting can silently diverge from the live belief view if a planner-read semantic component gets projected into a partial shadow shape. That specific office-data bug is already fixed in production, but the lower-layer focused coverage is still thinner than the architecture warrants. This ticket should now lock down the planner/local-boundary invariant without reopening already-delivered force-legitimacy architecture.

## Assumption Reassessment (2026-03-22)

1. The ticket's original architecture-change framing is stale. `PlanningSnapshot` already stores full `OfficeData` in `SnapshotEntity.office_data`, populated directly from `RuntimeBeliefView::office_data()` inside `build_snapshot_entity()` in [crates/worldwake-ai/src/planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs). `PlanningState` already returns that preserved value through `impl RuntimeBeliefView for PlanningState` in [crates/worldwake-ai/src/planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs).
2. The live belief/planner affordance path that originally regressed is also already restored. `enumerate_press_force_claim_payloads()` in [crates/worldwake-systems/src/office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs) reads `view.office_data(office)`, and the current golden `golden_force_claim_ai_installation` in [crates/worldwake-ai/tests/golden_offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs) already proves live `PerAgentBeliefView` and planner `PlanningState` both surface `press_force_claim`.
3. The live goal family remains `GoalKind::ClaimOffice { office }`, and the exact operator/affordance surface is still `PlannerOpKind::PressForceClaim` plus `get_affordances()` over `enumerate_press_force_claim_payloads()`. The ticket does not need to change planner semantics, candidate generation, or goal modeling.
4. This is now a focused boundary-coverage ticket, not a production redesign or golden-gap ticket. The correct proof surface is lower-layer snapshot/planning-state regression coverage, with existing goldens retained as end-to-end confirmation rather than primary proof.
5. The relevant contract is semantic parity between the live belief view and the planner snapshot for components the planner reads as truth. This is not event ordering, action lifecycle ordering, or authoritative world-state ordering.
6. The current code already follows the cleaner architecture for the original office bug: it preserves the whole semantic component instead of hand-copying a subset of fields. Adding a second planner-domain abstraction layer or external snapshot schema would duplicate truth and increase drift risk rather than improving extensibility.
7. The first failure boundary in the motivating regression was planner-local affordance reproduction through `RuntimeBeliefView::office_data()`. The exact shared symbols now verified are `build_snapshot_entity()`, `PlanningSnapshot::office_data()`, `PlanningState::office_data()`, and `get_affordances()`.
8. The political closure boundary here remains the AI-layer affordance/planning boundary before any `PressForceClaim` commit, control-state mutation, or office-holder installation. Those authoritative layers were already covered by the completed E16b work and its goldens.
9. No `ControlSource`, queued-input, driver reset, or action-start recovery behavior is in scope.
10. Mismatch corrected: the ticket no longer claims a new design note or production contract document is required. The live code already carries the essential contract inline by preserving full `OfficeData`; the remaining deficiency is focused regression coverage, not missing architecture.
11. Mismatch corrected: `archive/tickets/completed/E16DPOLPLAN-023.md` was a stale dependency path. The actual archived ticket is [archive/tickets/E16DPOLPLAN-023.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E16DPOLPLAN-023.md).
12. The survivability envelope remains qualitative: future planner fidelity regressions are most likely when a new planner-read semantic component is partially projected. The clean prevention pattern is whole-component snapshot preservation plus targeted live-vs-snapshot parity tests at the exact read boundary.

## Architecture Check

1. The current architecture is better than the ticket's original open-ended “fidelity contract” proposal: preserve semantically complete components at the planner boundary when planner logic reads them as truth, and prove parity with focused tests. That keeps truth in one concrete shape and avoids a second planner-specific domain model.
2. A separate documentation-only contract file would be more likely to drift than help. The durable architectural artifact here is code that clones the same semantic component from the belief view plus tests that fail if someone regresses that behavior.
3. No backwards-compatibility aliasing, shadow office models, or duplicated institution abstractions should be introduced.

## Verification Layers

1. Snapshot preserves semantically complete office metadata -> focused `planning_snapshot.rs` regression test on captured `OfficeData`
2. `PlanningState` exposes the same office metadata as the live belief view -> focused `planning_state.rs` parity test on `RuntimeBeliefView::office_data()`
3. Live and planner-local force-claim affordances agree -> focused `planning_state.rs` affordance parity test using `get_affordances()` plus existing `golden_force_claim_ai_installation`
4. Additional authoritative or golden layering is not the primary proof target because the production force-legitimacy architecture has already been delivered and verified elsewhere

## What to Change

### 1. Add focused office-data snapshot regression coverage

In [crates/worldwake-ai/src/planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs), add a focused test that seeds a full `OfficeData` value on the live belief view, builds a `PlanningSnapshot`, and asserts the captured snapshot value is semantically complete rather than field-subset or `None`.

### 2. Add focused live-vs-planning parity coverage

In [crates/worldwake-ai/src/planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs), add focused tests that:

- compare live `RuntimeBeliefView::office_data()` against `PlanningState`
- prove `get_affordances()` returns the same `press_force_claim` payload on both the live belief view and the derived planning state for an eligible local force-law office

## Files to Touch

- `tickets/HARPLASNAP-001.md` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)

## Out of Scope

- New planner architecture or generic snapshot-schema infrastructure
- Changes to force-control semantics, `GoalKind::ClaimOffice`, or planner operator mapping
- Additional golden scenarios beyond the already-completed E16b force-law coverage
- New documentation files in `docs/` for a contract already embodied in code and tests

## Acceptance Criteria

### Tests That Must Pass

1. Focused test proves `PlanningSnapshot` preserves full `OfficeData` from the live belief view
2. Focused test proves `PlanningState` returns the same `OfficeData` as the originating live belief view
3. Focused parity test proves live and planner-local affordance search both expose `PressForceClaim` for the same eligible local force-law office
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Planner snapshot state must preserve semantically complete source components when planner logic reads those components as truth
2. The planner must not rely on a lower-fidelity shadow representation when the live belief view already exposes richer semantic data

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` — add a focused `OfficeData` preservation regression test
2. `crates/worldwake-ai/src/planning_state.rs` — add focused live-vs-snapshot `OfficeData` and `press_force_claim` affordance parity tests

### Commands

1. `cargo test -p worldwake-ai --lib planning_snapshot::tests::snapshot_preserves_full_office_data_for_planner_semantics -- --exact`
2. `cargo test -p worldwake-ai --lib planning_state::tests::planning_state_matches_live_office_data_and_force_claim_affordances -- --exact`
3. `cargo test -p worldwake-ai --test golden_offices golden_force_claim_ai_installation -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

## Outcome

Completion date: 2026-03-22

What actually changed:
- Reassessed the ticket against live `E16b` force-law code and corrected the scope from “add new planner snapshot architecture” to “add focused regression coverage for the already-delivered architecture.”
- Added `planning_snapshot::tests::snapshot_preserves_full_office_data_for_planner_semantics` in [crates/worldwake-ai/src/planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs) to prove the snapshot preserves full `OfficeData`.
- Added `planning_state::tests::planning_state_matches_live_office_data_and_force_claim_affordances` in [crates/worldwake-ai/src/planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs) to prove live-vs-planner parity for `office_data()` and `press_force_claim` affordances.
- Corrected the stale dependency path to [archive/tickets/E16DPOLPLAN-023.md](/home/joeloverbeck/projects/worldwake/archive/tickets/E16DPOLPLAN-023.md).

Deviations from original plan:
- No production architecture changes were made because the clean whole-component snapshot design was already live.
- No new `docs/` design note was added because it would duplicate the actual contract already enforced by code plus focused tests.

Verification results:
- `cargo test -p worldwake-ai --lib planning_snapshot::tests::snapshot_preserves_full_office_data_for_planner_semantics -- --exact`
- `cargo test -p worldwake-ai --lib planning_state::tests::planning_state_matches_live_office_data_and_force_claim_affordances -- --exact`
- `cargo test -p worldwake-ai --test golden_offices golden_force_claim_ai_installation -- --exact`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace`
