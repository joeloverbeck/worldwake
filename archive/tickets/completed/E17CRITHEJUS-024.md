# E17CRITHEJUS-024: Centralize the dynamic duration-backed planner snapshot contract

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` planner contract inventory and audit coverage
**Deps**: archive/tickets/completed/E17CRITHEJUS-021.md

## Problem

`E17CRITHEJUS-021` fixed the last known live hole in the planner snapshot boundary by preserving `TheftDispositionProfile`, and it added parity tests for part of the current planner-visible duration contract. That closed the bug, but the architecture is still spread across multiple sites:

- `crates/worldwake-sim/src/belief_view.rs::estimate_duration_from_beliefs()`
- `crates/worldwake-ai/src/planning_snapshot.rs::build_snapshot_entity()`
- `crates/worldwake-ai/src/planning_state.rs` `RuntimeBeliefView` accessors
- focused tests that restate the same inventory in assertions

That duplication means the contract is still implicit. A future planner-visible `DurationExpr` addition can again land in runtime semantics first and only later be discovered missing from snapshot-backed planning. The next clean step is to make the planner-local non-fixed duration dependency inventory explicit in one authoritative place and drive audit coverage from it.

## Assumption Reassessment (2026-03-26)

1. The live runtime contract is `crates/worldwake-sim/src/belief_view.rs::estimate_duration_from_beliefs()`. The current planner-relevant non-fixed surfaces there are `TargetConsumable`, `ActorMetabolism`, `ActorTradeDisposition`, `ActorTheftDisposition`, `ActorInvestigationDisposition`, `ActorDefendStance`, `CombatWeapon`, `TargetTreatment`, `ConsultRecord`, and `TravelToTarget`.
2. The current planner-side preservation is split across `crates/worldwake-ai/src/planning_snapshot.rs::build_snapshot_entity()` and the `RuntimeBeliefView for PlanningState` impl in `crates/worldwake-ai/src/planning_state.rs`. There is no single planner-local symbol that says “these are the dynamic duration-backed dependencies the snapshot must preserve.”
3. The shared abstraction boundary under audit is: `estimate_duration_from_beliefs()` runtime reads -> planner-local snapshot preservation in `PlanningSnapshot` -> snapshot-backed reads in `PlanningState` -> successor construction in `crates/worldwake-ai/src/search/transition.rs`.
4. The intended invariant is: every planner-supported non-fixed duration dependency class is declared once in planner architecture, preserved once in snapshot/state form, and audited from that same declaration rather than by duplicated hand-maintained lists.
5. This is a planner/search ticket, not a crime-domain ticket. The live failure surface is generic successor construction in `search::transition`, not any one `GoalKind`. Current concrete goal families exercising the contract include `GoalKind::ConsumeOwnedCommodity`, `GoalKind::Relieve`, `GoalKind::Wash`, `GoalKind::StealItem`, `GoalKind::InvestigateViolation`, `GoalKind::ReduceDanger`, `GoalKind::TreatWounds`, `GoalKind::EngageHostile`, and record/travel-driven goals that surface `ConsultRecord` and `Travel`.
6. Existing focused coverage proves part of the behavior, but it does not yet centralize the contract and it does not cover the full live planner-visible non-fixed set. `cargo test -p worldwake-ai -- --list` confirms `planning_state::tests::planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract`, `planning_snapshot::tests::*`, and `search::tests::build_successor_estimates_steal_ticks_from_theft_profile` exist; those tests should become consumers of a single planner-local inventory instead of re-encoding only a subset ad hoc.
7. No heuristic is being removed. This ticket is about replacing duplicated architectural knowledge with one explicit contract source, which aligns with `docs/FOUNDATIONS.md` principles on concrete state, explainability, and rejecting workaround architecture.
8. The first failure boundary remains successor construction in `crates/worldwake-ai/src/search/transition.rs`, where missing snapshot-backed duration inputs degrade to `DurationEstimateFailed`. This ticket prevents that boundary from drifting out of sync again across the full planner-visible non-fixed duration surface, not just the crime-adjacent subset.
9. Adjacent contradictions belong to other tickets:
   - richer named prerequisite diagnostics remain in `tickets/E17CRITHEJUS-022.md`
   - planner contract documentation remains in `tickets/E17CRITHEJUS-023.md`
10. Mismatch + correction: no active ticket in `tickets/` currently owns this implementation cleanup. `E17CRITHEJUS-022` improves traceability, `E17CRITHEJUS-023` documents the contract, and `E17CRITHEJUS-021` only proved/parity-tested the current live set. A dedicated implementation ticket is required for the central inventory cleanup itself.

## Architecture Check

1. The clean fix is to introduce one planner-local contract inventory for planner-supported non-fixed duration dependencies and make snapshot preservation plus audit tests read from it.
2. That is cleaner than continuing to mirror the same list manually across runtime semantics, snapshot fields, state accessors, tests, and future docs. It also corrects the current architectural drift where the parity test itself already encodes only a subset of the live contract.
3. The contract should stay planner-local. `estimate_duration_from_beliefs()` is a broader runtime trait surface; the centralization needed here is “which of those runtime reads the planner snapshot must preserve for planner-supported operators,” not a new shared alias layer between crates.
4. No backwards-compatibility shims or fallback reads should be introduced. The planner should continue to depend on preserved snapshot state only.

## Verification Layers

1. Planner-local non-fixed duration inventory matches the current runtime duration surface the planner supports -> focused unit coverage against the inventory and runtime parity tests
2. Snapshot build/state accessors preserve every inventory-declared dependency -> focused `planning_snapshot.rs` and/or `planning_state.rs` tests
3. Successor construction still succeeds for inventory-backed dynamic durations such as theft -> focused `search::tests`
4. Documentation and traceability tickets remain consumers of the centralized contract, not substitutes for it -> follow-up cross-ticket dependency review, not runtime proof
5. Golden scenarios are not the primary proof surface here because the contract failure happens earlier at planner successor construction

## What to Change

### 1. Introduce one authoritative planner-local inventory

Add a planner-local symbol or small module that names the planner-supported non-fixed duration dependency classes the snapshot must preserve. It should distinguish dependency classes, not just raw `DurationExpr` syntax, so planner tests and future diagnostics can share the same vocabulary.

### 2. Drive snapshot audit coverage from that inventory

Refactor the current parity/audit tests so they iterate or otherwise derive assertions from the centralized inventory instead of retyping the list. Keep the proof surface strong at the snapshot/state and search layers.

### 3. Keep runtime semantics and planner semantics decoupled but aligned

Do not move planner concerns into `worldwake-sim`. Instead, add the minimal planner-local bridge that checks the runtime duration semantics the planner actually depends on and proves the snapshot boundary remains aligned to them. The inventory audit should derive from the live planner operator surface so `TargetConsumable`, `ActorMetabolism`, `CombatWeapon`, and `TargetTreatment` stay in scope alongside the crime-adjacent duration classes.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify to register the planner-local contract module)
- `crates/worldwake-ai/src/` planner-local contract module (new or modify existing nearby module)

## Out of Scope

- New named trace diagnostics for missing prerequisites (`E17CRITHEJUS-022`)
- Contributor/docs updates beyond pointing those docs at the final contract (`E17CRITHEJUS-023`)
- Broadening the planner to support new duration expressions not currently used by search
- Any authoritative action/system behavior changes in `worldwake-sim` or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. One planner-local contract inventory exists for the planner-supported non-fixed duration snapshot dependencies.
2. Snapshot/state parity coverage consumes that inventory rather than duplicating the dependency list manually.
3. The inventory covers the full current planner-visible non-fixed duration surface, including `TargetConsumable`, `ActorMetabolism`, `CombatWeapon`, and `TargetTreatment` in addition to the previously audited crime-adjacent cases.
4. Existing focused successor coverage still proves that inventory-backed duration estimation works for steal and other current planner-relevant cases.
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The planner snapshot contract has one authoritative implementation source, not several drifting handwritten mirrors.
2. Future additions to planner-relevant non-fixed durations must force a touch to the centralized contract and fail focused coverage if snapshot preservation is missing.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_state.rs` — refactor the current duration parity test to derive its assertions from the centralized planner contract inventory and cover the full live non-fixed planner set.
2. `crates/worldwake-ai/src/search/tests.rs` — keep focused successor coverage for an inventory-backed case such as steal so the search layer still proves the contract matters operationally.
3. `crates/worldwake-ai/src/planning_snapshot.rs` and/or the new planner-local contract module — add focused inventory-shape tests proving the contract matches the live planner operator surface.

### Commands

1. `cargo test -p worldwake-ai planning_state::tests::planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract`
2. `cargo test -p worldwake-ai planning_snapshot::tests`
3. `cargo test -p worldwake-ai search::tests::build_successor_estimates_steal_ticks_from_theft_profile`
4. `cargo test -p worldwake-ai`

## Outcome

- Completion date: 2026-03-26
- What actually changed:
  - Reassessed the ticket against the live planner operator surface and corrected the scope from a six-case crime-adjacent subset to the full planner-visible non-fixed duration contract: `TargetConsumable`, `ActorMetabolism`, `ActorTradeDisposition`, `ActorTheftDisposition`, `ActorInvestigationDisposition`, `ActorDefendStance`, `CombatWeapon`, `TargetTreatment`, `ConsultRecord`, and `TravelToTarget`.
  - Added a planner-local contract module in `worldwake-ai` that names those dependency classes and maps live `DurationExpr` values into that inventory.
  - Added focused coverage proving that the inventory matches the live planner action registry rather than a handwritten partial list.
  - Refactored the existing `planning_state` runtime-vs-snapshot parity test to iterate the centralized inventory and verify snapshot parity across the full non-fixed planner-supported duration surface.
- Deviations from original plan:
  - No production `worldwake-sim` changes were needed; the runtime duration semantics were already correct and remained the audit reference.
  - `crates/worldwake-ai/src/search/tests.rs` did not require modification because the existing steal successor regression still provided the needed operational proof once the central inventory and broadened parity coverage landed.
  - `crates/worldwake-ai/src/planning_snapshot.rs` did not require direct code changes; the architectural gap was the missing planner-local contract source, not additional snapshot field preservation.
- Verification results:
  - `cargo test -p worldwake-ai planning_state::tests::planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract` ✅
  - `cargo test -p worldwake-ai planner_duration_contract::tests::planner_duration_inventory_matches_live_non_fixed_planner_surface` ✅
  - `cargo test -p worldwake-ai search::tests::build_successor_estimates_steal_ticks_from_theft_profile` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
