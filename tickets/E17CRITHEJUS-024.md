# E17CRITHEJUS-024: Centralize the dynamic duration-backed planner snapshot contract

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` planner contract inventory and audit coverage
**Deps**: archive/tickets/completed/E17CRITHEJUS-021.md

## Problem

`E17CRITHEJUS-021` fixed the last known live hole in the planner snapshot boundary by preserving `TheftDispositionProfile`, and it added parity tests for the current dynamic duration-backed contract. That closed the bug, but the architecture is still spread across multiple sites:

- `crates/worldwake-sim/src/belief_view.rs::estimate_duration_from_beliefs()`
- `crates/worldwake-ai/src/planning_snapshot.rs::build_snapshot_entity()`
- `crates/worldwake-ai/src/planning_state.rs` `RuntimeBeliefView` accessors
- focused tests that restate the same inventory in assertions

That duplication means the contract is still implicit. A future `DurationExpr` addition can again land in runtime semantics first and only later be discovered missing from snapshot-backed planning. The next clean step is to make the planner-local dynamic duration dependency inventory explicit in one authoritative place and drive audit coverage from it.

## Assumption Reassessment (2026-03-26)

1. The live runtime contract is `crates/worldwake-sim/src/belief_view.rs::estimate_duration_from_beliefs()`. The current dynamic planner-relevant surfaces there are `ActorTradeDisposition`, `ActorTheftDisposition`, `ActorInvestigationDisposition`, `ActorDefendStance`, `ConsultRecord`, and `TravelToTarget`.
2. The current planner-side preservation is split across `crates/worldwake-ai/src/planning_snapshot.rs::build_snapshot_entity()` and the `RuntimeBeliefView for PlanningState` impl in `crates/worldwake-ai/src/planning_state.rs`. There is no single planner-local symbol that says “these are the dynamic duration-backed dependencies the snapshot must preserve.”
3. The shared abstraction boundary under audit is: `estimate_duration_from_beliefs()` runtime reads -> planner-local snapshot preservation in `PlanningSnapshot` -> snapshot-backed reads in `PlanningState` -> successor construction in `crates/worldwake-ai/src/search/transition.rs`.
4. The intended invariant is: every dynamic duration-backed dependency the planner supports is declared once in planner architecture, preserved once in snapshot/state form, and audited from that same declaration rather than by duplicated hand-maintained lists.
5. This is a planner/search ticket, not a crime-domain ticket. The live failure surface is generic successor construction in `search::transition`, not any one `GoalKind`. Current concrete goal families exercising the contract include `GoalKind::StealItem`, `GoalKind::InvestigateViolation`, `GoalKind::ReduceDanger`, and record/travel-driven goals that surface `ConsultRecord` and `Travel`.
6. Existing focused coverage now proves behavior, but it does not yet centralize the contract. `cargo test -p worldwake-ai -- --list` confirms `planning_state::tests::planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract` and `search::tests::build_successor_estimates_steal_ticks_from_theft_profile` exist; those tests should become consumers of a single planner-local inventory instead of re-encoding it ad hoc.
7. No heuristic is being removed. This ticket is about replacing duplicated architectural knowledge with one explicit contract source, which aligns with `docs/FOUNDATIONS.md` principles on concrete state, explainability, and rejecting workaround architecture.
8. The first failure boundary remains successor construction in `crates/worldwake-ai/src/search/transition.rs`, where missing snapshot-backed duration inputs degrade to `DurationEstimateFailed`. This ticket prevents that boundary from drifting out of sync again.
9. Adjacent contradictions belong to other tickets:
   - richer named prerequisite diagnostics remain in `tickets/E17CRITHEJUS-022.md`
   - planner contract documentation remains in `tickets/E17CRITHEJUS-023.md`
10. Mismatch + correction: no active ticket in `tickets/` currently owns this implementation cleanup. `E17CRITHEJUS-022` improves traceability, `E17CRITHEJUS-023` documents the contract, and `E17CRITHEJUS-021` only proved/parity-tested the current live set. A dedicated implementation ticket is required for the central inventory cleanup itself.

## Architecture Check

1. The clean fix is to introduce one planner-local contract inventory for dynamic duration-backed dependencies and make snapshot preservation plus audit tests read from it.
2. That is cleaner than continuing to mirror the same list manually across runtime semantics, snapshot fields, state accessors, tests, and future docs.
3. The contract should stay planner-local. `estimate_duration_from_beliefs()` is a broader runtime trait surface; the centralization needed here is “which of those runtime reads the planner snapshot must preserve,” not a new shared alias layer between crates.
4. No backwards-compatibility shims or fallback reads should be introduced. The planner should continue to depend on preserved snapshot state only.

## Verification Layers

1. Planner-local dynamic duration inventory matches the current runtime duration surface the planner supports -> focused unit coverage against the inventory and runtime parity tests
2. Snapshot build/state accessors preserve every inventory-declared dependency -> focused `planning_snapshot.rs` and/or `planning_state.rs` tests
3. Successor construction still succeeds for inventory-backed dynamic durations such as theft -> focused `search::tests`
4. Documentation and traceability tickets remain consumers of the centralized contract, not substitutes for it -> follow-up cross-ticket dependency review, not runtime proof
5. Golden scenarios are not the primary proof surface here because the contract failure happens earlier at planner successor construction

## What to Change

### 1. Introduce one authoritative planner-local inventory

Add a planner-local symbol or small module that names the dynamic duration-backed dependency classes the snapshot must preserve. It should distinguish dependency classes, not just raw `DurationExpr` syntax, so planner tests and future diagnostics can share the same vocabulary.

### 2. Drive snapshot audit coverage from that inventory

Refactor the current parity/audit tests so they iterate or otherwise derive assertions from the centralized inventory instead of retyping the list. Keep the proof surface strong at the snapshot/state and search layers.

### 3. Keep runtime semantics and planner semantics decoupled but aligned

Do not move planner concerns into `worldwake-sim`. Instead, add the minimal planner-local bridge that checks the runtime duration semantics the planner actually depends on and proves the snapshot boundary remains aligned to them.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/` planner-local contract module (new or modify existing nearby module)

## Out of Scope

- New named trace diagnostics for missing prerequisites (`E17CRITHEJUS-022`)
- Contributor/docs updates beyond pointing those docs at the final contract (`E17CRITHEJUS-023`)
- Broadening the planner to support new duration expressions not currently used by search
- Any authoritative action/system behavior changes in `worldwake-sim` or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. One planner-local contract inventory exists for the dynamic duration-backed snapshot dependencies the planner supports.
2. Snapshot/state parity coverage consumes that inventory rather than duplicating the dependency list manually.
3. Existing focused successor coverage still proves that inventory-backed dynamic duration estimation works for steal and other current planner-relevant cases.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The planner snapshot contract has one authoritative implementation source, not several drifting handwritten mirrors.
2. Future additions to planner-relevant dynamic durations must force a touch to the centralized contract and fail focused coverage if snapshot preservation is missing.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_state.rs` — refactor the current dynamic-duration parity test to derive its assertions from the centralized planner contract inventory.
2. `crates/worldwake-ai/src/search/tests.rs` — keep focused successor coverage for an inventory-backed case such as steal so the search layer still proves the contract matters operationally.
3. `crates/worldwake-ai/src/planning_snapshot.rs` and/or the new planner-local contract module — add focused inventory-shape tests if needed so the contract itself is directly asserted.

### Commands

1. `cargo test -p worldwake-ai planning_state::tests`
2. `cargo test -p worldwake-ai search::tests`
3. `cargo test -p worldwake-ai`
