# E17CRITHEJUS-021: Audit and enforce planning-snapshot completeness for planner-visible dependencies

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planning snapshot audit coverage and dependency preservation
**Deps**: archive/tickets/completed/E17CRITHEJUS-008.md

## Problem

`E17CRITHEJUS-008` exposed that planner-visible data can exist in the runtime belief view yet still be absent from `PlanningSnapshot`, causing search to fail later with `DurationEstimateFailed`. The immediate example was `ViolationDispositionProfile`: runtime duration estimation could see it, but snapshot-based planning initially could not.

That failure mode is architectural. `PlanningSnapshot` is supposed to be the planner’s durable, bounded belief-state encoding. If planner-visible duration inputs or other planning prerequisites can silently fall out of that boundary, search remains fragile and hard to extend. The correct fix is not to patch the next missing field ad hoc. It is to make snapshot completeness auditable and test it against the planner-visible dependency contract.

## Assumption Reassessment (2026-03-26)

1. `crates/worldwake-sim/src/belief_view.rs` defines the runtime contract used by duration estimation. `estimate_duration_from_beliefs()` consumes `RuntimeBeliefView` data such as `trade_disposition_profile()`, `violation_disposition_profile()`, travel distances, combat state, and other planner-visible belief inputs.
2. `crates/worldwake-ai/src/planning_snapshot.rs` now preserves both `trade_disposition_profile` and `violation_disposition_profile` on `SnapshotEntity`, plus actor tell memory/profile state. The immediate `ViolationDispositionProfile` hole exposed by `E17CRITHEJUS-008` is already fixed, but the broader audit contract is still implicit.
3. The shared abstraction boundary under audit is: `RuntimeBeliefView` -> `PlanningSnapshot::build()` / `snapshot_entity_for` -> `PlanningState` -> planner calls to `estimate_duration_from_beliefs()` and other planner-visible state readers.
4. The intended invariant is: any belief-visible datum that can affect root-candidate viability, duration estimation, or hypothetical transition legality inside planner search must either survive snapshot construction or be rejected explicitly by a named planner contract.
5. The live duration surfaces relevant to this ticket include at least `DurationExpr::ActorTradeDisposition`, `DurationExpr::ActorTheftDisposition`, `DurationExpr::ActorInvestigationDisposition`, `DurationExpr::ActorDefendStance`, `DurationExpr::TravelToTarget`, and `DurationExpr::ConsultRecord`. Reassessment must audit the full live set rather than only the investigation path that happened to fail first.
6. This is a planner/search ticket, not a candidate-generation ticket. Focused `planning_snapshot.rs` unit coverage is necessary but not sufficient; at least one planner-level proof surface must show that snapshot-backed planning no longer fails from missing preserved state.
7. The current risk is drift, not total absence. `PlanningSnapshot` already preserves many fields, but there is no explicit completeness audit tying planner-visible dependencies back to the runtime belief-view contract.
8. No heuristic is being removed here. The contradiction is a missing shared contract between runtime belief-view semantics and the planner snapshot boundary.
9. The first live failure boundary for the motivating regression was successor construction in `crates/worldwake-ai/src/search/transition.rs`, where duration estimation returned `None`; authoritative action start was downstream and not the source of the bug.
10. Existing runtime coverage already documents part of the failure envelope. `crates/worldwake-sim/src/belief_view.rs` includes `estimate_duration_from_beliefs_returns_none_for_missing_investigation_profile`, proving the runtime-side semantics. This ticket must add the planner-side parity proof, not rewrite the runtime contract.
11. Adjacent contradictions exposed during reassessment are separate tickets:
    - exact-goal root operator surfacing contract -> `E17CRITHEJUS-020`
    - trace diagnostics for omitted operators and named missing prerequisites -> `E17CRITHEJUS-022`
12. Mismatch + correction: the narrow bug “investigate needs `ViolationDispositionProfile` copied into snapshot” is already resolved. The corrected scope is broader and architectural: make snapshot completeness auditable so future planner-visible dependency drift is caught by tests instead of by late search failures.

## Architecture Check

1. The clean fix is to define and test a planner-visible dependency inventory at the snapshot boundary, then keep `PlanningSnapshot` faithful to that inventory.
2. That is cleaner than continuing to copy fields reactively after each regression, because the planner’s bounded belief-state contract becomes explicit and extensible.
3. This aligns with `docs/FOUNDATIONS.md`: concrete state over abstract proxies, no magic hidden dependencies, and no workaround patches that leave the real boundary undefined.
4. No backwards-compatibility shim should be added. Preserve the real planner-visible state in snapshot form or reject the planner path explicitly.

## Verification Layers

1. Snapshot build preserves all planner-visible dependency fields required by the audited contract -> focused `planning_snapshot.rs` unit coverage
2. Snapshot-backed duration estimation matches runtime belief-view duration estimation for audited dependency classes -> focused parity tests in `worldwake-ai` and/or `worldwake-sim`
3. Planner search no longer fails solely because a required runtime belief-view datum was dropped during snapshot construction -> focused `worldwake-ai` search tests
4. Traceability naming for missing prerequisites is not the proof surface here; if traces are still coarse, the stronger proof surface remains snapshot parity and focused planner coverage, with follow-up work in `E17CRITHEJUS-022`
5. Authoritative action handlers are not the primary proof surface for this ticket because the contradiction occurs before authoritative start.

## What to Change

### 1. Audit the planner-visible dependency contract

List the live planner/search consumers that depend on runtime belief-view data and identify which fields must survive `PlanningSnapshot`. At minimum, cover all `DurationExpr` variants that read `RuntimeBeliefView`, plus any non-duration planner readers that would become unsound if omitted from snapshot state.

### 2. Make snapshot preservation intentional

Update `PlanningSnapshot` construction and supporting accessors so each audited dependency is preserved or intentionally rejected with a named reason. Avoid one-off field patches without contract coverage.

### 3. Add parity and regression tests

Add tests that compare runtime belief-view behavior with snapshot-backed planner behavior for the audited dependency classes, including the investigation-disposition regression that motivated this work.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify, if accessors or readers need cleanup)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify only if shared parity helpers or clarifying tests are required)

## Out of Scope

- Root-candidate synthesis contract cleanup beyond what the snapshot audit needs to compile against
- Decision-trace schema work
- Candidate-generation profile gating changes
- Any authoritative system/action changes outside planner-visible belief access

## Acceptance Criteria

### Tests That Must Pass

1. Every audited planner-visible dependency required by search or duration estimation is either preserved in `PlanningSnapshot` or rejected explicitly by contract.
2. Snapshot-backed planner duration estimation matches runtime belief-view behavior for the audited dependency classes.
3. The investigation-disposition regression is covered by a planner-side test that would fail if `ViolationDispositionProfile` dropped out of snapshot state again.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `PlanningSnapshot` remains a faithful bounded encoding of planner-visible belief state, not a lossy approximation with undocumented holes.
2. Planner search cannot silently depend on runtime belief-view data that the snapshot boundary does not preserve.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` — add snapshot completeness tests for audited planner-visible dependency fields and regressions around `ViolationDispositionProfile`.
2. `crates/worldwake-ai/src/search/tests.rs` — add focused search coverage proving snapshot-backed planning no longer fails with `DurationEstimateFailed` when the audited dependency is present.
3. `crates/worldwake-sim/src/belief_view.rs` — strengthen shared duration-estimation tests only if needed to prove runtime/snapshot parity against the same live dependency classes.

### Commands

1. `cargo test -p worldwake-ai planning_snapshot::tests`
2. `cargo test -p worldwake-ai search::tests`
3. `cargo test -p worldwake-ai`
