# E17CRITHEJUS-021: Audit and enforce planning-snapshot completeness for planner-visible dependencies

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planning snapshot audit coverage and dependency preservation
**Deps**: archive/tickets/completed/E17CRITHEJUS-008.md

## Problem

`E17CRITHEJUS-008` exposed that planner-visible data can exist in the runtime belief view yet still be absent from `PlanningSnapshot`, causing search to fail later with `DurationEstimateFailed`. The immediate example was `ViolationDispositionProfile`: runtime duration estimation could see it, but snapshot-based planning initially could not.

That failure mode is architectural. `PlanningSnapshot` is supposed to be the planner’s durable, bounded belief-state encoding. If planner-visible duration inputs or other planning prerequisites can silently fall out of that boundary, search remains fragile and hard to extend. The correct fix is not to patch the next missing field ad hoc. It is to make snapshot completeness auditable and test it against the planner-visible dependency contract.

## Assumption Reassessment (2026-03-26)

1. `crates/worldwake-sim/src/belief_view.rs` defines the runtime contract used by duration estimation. `estimate_duration_from_beliefs()` consumes `RuntimeBeliefView` data such as `trade_disposition_profile()`, `violation_disposition_profile()`, travel distances, combat state, and other planner-visible belief inputs.
2. `crates/worldwake-ai/src/planning_snapshot.rs` now preserves `trade_disposition_profile`, `violation_disposition_profile`, `combat_profile`, record data, and actor consultation/tell state on snapshot entities. The immediate `ViolationDispositionProfile` hole exposed by `E17CRITHEJUS-008` is already fixed, but the broader duration-backed dependency contract is still implicit.
3. The shared abstraction boundary under audit is: `RuntimeBeliefView` -> `PlanningSnapshot::build()` / `snapshot_entity_for` -> `PlanningState` -> planner calls to `estimate_duration_from_beliefs()` and other planner-visible state readers.
4. The intended invariant is: any belief-visible datum that can affect root-candidate viability, duration estimation, or hypothetical transition legality inside planner search must either survive snapshot construction or be rejected explicitly by a named planner contract.
5. The live duration surfaces relevant to this ticket are `DurationExpr::ActorTradeDisposition`, `DurationExpr::ActorTheftDisposition`, `DurationExpr::ActorInvestigationDisposition`, `DurationExpr::ActorDefendStance`, `DurationExpr::ConsultRecord`, and `DurationExpr::TravelToTarget`. Reassessment against `crates/worldwake-sim/src/belief_view.rs::estimate_duration_from_beliefs()` shows that snapshot/planning-state coverage is already present for trade, investigation, defend/combat, consultation, and travel topology, but `ActorTheftDisposition` is still missing from the snapshot/state boundary.
6. This is a planner/search ticket, not a candidate-generation ticket. Focused `planning_snapshot.rs` unit coverage is necessary but not sufficient; at least one planner-level proof surface must show that snapshot-backed planning no longer fails from missing preserved state.
7. The current risk is drift, not total absence. `PlanningSnapshot` already preserves most dynamic duration inputs, but there is no explicit parity proof tying every live duration-backed planner dependency back to the runtime belief-view contract.
8. No heuristic is being removed here. The contradiction is a missing shared contract between runtime belief-view semantics and the planner snapshot boundary.
9. The first live failure boundary for the motivating regression was successor construction in `crates/worldwake-ai/src/search/transition.rs`, where duration estimation returned `None`; authoritative action start was downstream and not the source of the bug.
10. Existing runtime coverage already documents part of the failure envelope. `crates/worldwake-sim/src/belief_view.rs` includes `estimate_duration_from_beliefs_returns_none_for_missing_investigation_profile`, proving the runtime-side semantics. This ticket must add the planner-side parity proof, not rewrite the runtime contract.
11. Adjacent contradictions exposed during reassessment are separate tickets:
    - exact-goal root operator surfacing contract -> `E17CRITHEJUS-020`
    - trace diagnostics for omitted operators and named missing prerequisites -> `E17CRITHEJUS-022`
12. Mismatch + correction: the narrow bug “investigate needs `ViolationDispositionProfile` copied into snapshot” is already resolved. The corrected live scope is narrower and more concrete than the original broad wording: close the remaining `ActorTheftDisposition` snapshot hole and add explicit parity coverage for the full current set of dynamic duration-backed planner reads so future drift is caught by tests instead of by late search failures.

## Architecture Check

1. The clean fix is to treat dynamic duration-backed planner reads as an explicit snapshot contract, preserve the missing theft profile at that boundary, and add parity tests for the full live set.
2. That is cleaner than continuing to copy fields reactively after each regression, because the planner’s bounded belief-state contract becomes explicit and extensible without inventing a second alias layer or shadow duration API.
3. This aligns with `docs/FOUNDATIONS.md`: concrete state over abstract proxies, no magic hidden dependencies, and no workaround patches that leave the real boundary undefined.
4. No backwards-compatibility shim should be added. Preserve the real planner-visible state in snapshot form or reject the planner path explicitly.

## Verification Layers

1. Snapshot build preserves the full current dynamic duration-backed dependency set required by planner search -> focused `planning_snapshot.rs` and/or `planning_state.rs` unit coverage
2. Snapshot-backed duration estimation matches runtime belief-view duration estimation for trade, theft, investigation, defend, consult-record, and travel dependency classes -> focused parity tests in `worldwake-ai`
3. Planner successor construction no longer fails solely because a required runtime belief-view duration datum was dropped during snapshot construction -> focused `worldwake-ai` search tests
4. Traceability naming for missing prerequisites is not the proof surface here; if traces are still coarse, the stronger proof surface remains snapshot parity and focused planner coverage, with follow-up work in `E17CRITHEJUS-022`
5. Authoritative action handlers are not the primary proof surface for this ticket because the contradiction occurs before authoritative start.

## What to Change

### 1. Audit the dynamic duration-backed planner dependency contract

List the live planner/search consumers that depend on runtime belief-view data and identify which fields must survive `PlanningSnapshot`. For this ticket, the authoritative inventory is the current `estimate_duration_from_beliefs()` match surface plus the topology/record/profile reads it uses.

### 2. Make snapshot preservation intentional

Update `PlanningSnapshot` construction and supporting `PlanningState` accessors so each live dynamic duration-backed dependency is preserved. In the current code, that specifically includes adding `TheftDispositionProfile` preservation/exposure rather than relying on runtime-only access.

### 3. Add parity and regression tests

Add tests that compare runtime belief-view behavior with snapshot-backed planner behavior for the full live dynamic duration set, including the already-fixed investigation regression and the still-missing theft-disposition path.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify, if accessors or readers need cleanup)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (read for contract audit; modify only if shared parity helpers or clarifying tests are required)

## Out of Scope

- Root-candidate synthesis contract cleanup beyond what the snapshot audit needs to compile against
- Decision-trace schema work
- Candidate-generation profile gating changes
- Any authoritative system/action changes outside planner-visible belief access

## Acceptance Criteria

### Tests That Must Pass

1. Every current dynamic duration-backed planner dependency used by `estimate_duration_from_beliefs()` is preserved at the snapshot/state boundary when that dependency is planner-relevant.
2. Snapshot-backed planner duration estimation matches runtime belief-view behavior for trade, theft, investigation, defend, consult-record, and travel duration classes.
3. Planner-side regression coverage would fail if either `ViolationDispositionProfile` or `TheftDispositionProfile` dropped out of snapshot state again.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `PlanningSnapshot` remains a faithful bounded encoding of planner-visible belief state for the live dynamic duration-backed contract, not a lossy approximation with undocumented holes.
2. Planner search cannot silently depend on runtime belief-view duration data that the snapshot boundary does not preserve.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` and/or `crates/worldwake-ai/src/planning_state.rs` — add snapshot/state parity tests for the full live dynamic duration-backed dependency inventory, including `TheftDispositionProfile`.
2. `crates/worldwake-ai/src/search/tests.rs` — add focused successor/search coverage proving snapshot-backed planning no longer fails with `DurationEstimateFailed` when the theft duration dependency is present.
3. Reuse existing runtime duration semantics in `crates/worldwake-sim/src/belief_view.rs` as the contract reference; only add runtime-side tests if parity proof cannot stay entirely in `worldwake-ai`.

### Commands

1. `cargo test -p worldwake-ai planning_snapshot::tests`
2. `cargo test -p worldwake-ai search::tests`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-26
- What actually changed:
  - Reassessed the ticket against the live code and corrected the scope from a generic completeness audit to the real remaining gap: `DurationExpr::ActorTheftDisposition` was still reading runtime belief data that `PlanningSnapshot`/`PlanningState` did not preserve.
  - Preserved `TheftDispositionProfile` on `SnapshotEntity` and exposed it through `PlanningState` so snapshot-backed duration estimation now covers the full current dynamic duration-backed contract: trade, theft, investigation, defend, consult-record, and travel.
  - Added planner/runtime parity coverage for the full dynamic duration-backed contract and a focused successor regression proving steal planning no longer fails because the theft duration profile was dropped at the snapshot boundary.
  - Renamed two bindings in `crates/worldwake-ai/tests/planner_conformance.rs` to satisfy workspace clippy’s existing `similar_names` lint so the requested lint pass is real.
- Deviations from original plan:
  - No `worldwake-sim` production code changes were required; the runtime duration contract in `estimate_duration_from_beliefs()` was already correct and served as the audit reference.
  - The ticket narrowed from an open-ended “all planner-visible dependencies” audit to the stronger, current, and testable dynamic duration-backed contract that actually motivated the regression class.
- Verification results:
  - `cargo test -p worldwake-ai planning_state::tests::planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract` ✅
  - `cargo test -p worldwake-ai search::tests::build_successor_estimates_steal_ticks_from_theft_profile` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
