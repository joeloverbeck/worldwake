# S59EXPOBLSUB-007: report_missing action

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new action in worldwake-systems plus minimal action-payload fallout
**Deps**: S59EXPOBLSUB-002, S59EXPOBLSUB-005, S59EXPOBLSUB-006

## Problem

Once an expectation becomes overdue, the owner has no action-level path to formalize that overdue record into the existing violation workflow. The first honest slice is a `report_missing` action that turns an overdue expectation into `ViolationKind::EntityMissing`, so later search/investigation behavior can reuse the existing violation substrate.

## Assumption Reassessment (2026-04-06)

1. Action registration still follows the `register_*_action()` pattern called from `register_all_actions()` in [action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs).
2. The shared boundary under audit is: overdue expectation state in `ExpectationStore` (`crates/worldwake-core/src/expectation.rs`) -> action affordance/payload in `worldwake-sim` -> violation recording in `ViolationMemory` / `ViolationKind::EntityMissing` (`crates/worldwake-core/src/violation.rs`).
3. `ViolationKind::EntityMissing { entity, expected_place }` exists and already drives downstream investigate behavior in [investigate_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/investigate_actions.rs).
4. `ExpectationState` only has `Active | Overdue | Resolved | Expired`; there is no intermediate “reported” state. The original ticket claim that `report_missing` updates expectation state from `Overdue` is stale and cannot land honestly against the live shared enum.
5. `report_found` is not implementable on the current branch. `SearchResult` exists only as a shared enum in [expectation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/expectation.rs); there is no stored search-result carrier, no `ActionPayload` variant, and no live action/state surface that can authoritatively prove “actor has resolved a search result.”
6. The office-record path in the original draft is also stale. `InstitutionalClaim` only supports office, faction, force-control, accusation, and verdict records in [institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs); there is no missing-person institutional claim type to append to a record without widening shared architecture.
7. Existing tell/social-observation code in [tell_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/tell_actions.rs) can propagate social facts, but reusing that path for `report_found` or richer `report_missing` communication would broaden the ticket beyond its first lawful carrier.
8. `ViolationMemory` is not a universal agent component today, but tell/investigate already lawfully create or mutate it when an actor has `ViolationDispositionProfile`. The honest first slice is therefore: `report_missing` requires `ViolationDispositionProfile` and writes `ViolationMemory` on demand.
9. Mismatch + correction: the original ticket bundled `report_missing` and `report_found`, expectation-state mutation, and office-record creation. Live code only supports `report_missing -> ViolationMemory` cleanly. This ticket is narrowed to that first slice, and `report_found` plus richer report propagation move to follow-up ticket `S59EXPOBLSUB-013`.

## Architecture Check

1. Reusing the existing violation workflow is cleaner than inventing a second “missing report” carrier before search/report follow-through exists. It gives overdue expectations a concrete downstream consequence without widening shared institutional or tell substrates prematurely.
2. No backward-compatibility shims or duplicate authoritative paths are introduced. The ticket adds one action and one payload shape for selecting which overdue expectation to formalize.

## Verification Layers

1. Overdue expectation yields a `report_missing` affordance with the correct expectation payload -> focused runtime affordance test
2. Committing `report_missing` records `ViolationKind::EntityMissing` in authoritative `ViolationMemory` -> authoritative world state
3. Active or already-recorded expectations do not produce a lawful `report_missing` affordance -> focused runtime affordance test
4. The new action is fully registered in the action catalog -> registry completeness test
5. Single-layer ticket after reassessment: no separate action-trace or institutional-record proof surface is required because target-side report propagation is explicitly deferred

## What to Change

### 1. Create report_missing action

Create `crates/worldwake-systems/src/report_actions.rs` with a new `report_missing` action:

- Domain: `ActionDomain::Social`
- Target: actor place (`TargetSpec::ActorPlace`)
- Preconditions: actor is alive, not incapacitated, has `ViolationDispositionProfile`, and has an overdue expectation selected by payload
- Duration: short fixed communication-style action
- on_commit: record `ViolationKind::EntityMissing { entity: subject, expected_place }` into actor `ViolationMemory`, creating the component if absent
- Affordance payloads: enumerate overdue expectations that are not already represented by an active `EntityMissing` violation

### 2. Add minimal shared payload fallout

Extend `worldwake-sim` action payload/trace surfaces with the narrow payload needed to identify which overdue expectation is being reported.

### 3. Register the action

Register `report_missing` from `register_all_actions()` and include it in the action-catalog completeness test.

## Files to Touch

- `crates/worldwake-systems/src/report_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify)
- `crates/worldwake-systems/src/action_registry.rs` (modify)
- `crates/worldwake-sim/src/action_payload.rs` (modify)
- `crates/worldwake-sim/src/action_trace.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- `report_found` action and any search-result-driven follow-through — follow-up `S59EXPOBLSUB-013`
- Office/institutional missing-person record creation
- Expectation lifecycle changes beyond the existing `Overdue` state
- Candidate generation for `ReportMissing` goal — ticket `S59EXPOBLSUB-011`

## Acceptance Criteria

### Tests That Must Pass

1. Overdue expectation produces a `report_missing` affordance with the correct expectation payload
2. Committing `report_missing` records `ViolationKind::EntityMissing` with the correct subject and expected place
3. Active expectations do not produce `report_missing`
4. Already-recorded `EntityMissing` violations suppress duplicate `report_missing` affordances
5. Action registry completeness test includes `"report_missing"`
6. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. `report_missing` only formalizes overdue expectations; it does not resolve or otherwise mutate expectation lifecycle state
2. Missing-person reporting reuses the canonical violation workflow instead of introducing a parallel missing-report authority path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/report_actions.rs` — focused affordance and commit tests for overdue-to-violation formalization
2. `crates/worldwake-systems/src/action_registry.rs` — updated completeness test
3. `crates/worldwake-sim/src/action_payload.rs` and `crates/worldwake-sim/src/action_trace.rs` — focused payload/trace fallout coverage

### Commands

1. `cargo test -p worldwake-systems report_missing`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed on 2026-04-06.

Reassessment correction applied before coding:
- The original bundled `report_found` with `report_missing`, but live code had no lawful search-result carrier, no expectation “reported” state, and no institutional missing-person claim type.
- This ticket therefore landed the first honest slice only: `report_missing` now formalizes overdue expectations into `ViolationMemory::EntityMissing`.
- Follow-up ticket `S59EXPOBLSUB-013` now owns `report_found` plus any richer missing-report propagation once `search_place` lands.

Implemented work:
- Added `report_missing` in `crates/worldwake-systems/src/report_actions.rs` as a self-targeted social action over actor place.
- Added `ReportMissingActionPayload` in `crates/worldwake-sim/src/action_payload.rs`, re-exported it from `crates/worldwake-sim/src/lib.rs`, and extended `ActionTraceDetail` in `crates/worldwake-sim/src/action_trace.rs`.
- Registered `report_missing` in `crates/worldwake-systems/src/lib.rs` and `crates/worldwake-systems/src/action_registry.rs`.

Behavioral result:
- Overdue expectations now yield a lawful `report_missing` affordance when the actor has `ViolationDispositionProfile`.
- Committing the action records `ViolationKind::EntityMissing { entity, expected_place }` in authoritative `ViolationMemory`.
- The action intentionally does not resolve or otherwise mutate `ExpectationRecord` state.

## Verification Result

Passed:
1. `cargo test -p worldwake-systems report_missing`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test -p worldwake-ai --test golden_integration`

Attempted but environment-killed:
1. `cargo test --workspace`

The full workspace run was terminated by `SIGKILL` while executing `worldwake-ai`'s `golden_integration` binary. I reran that exact target directly and it passed (`45 passed`), which indicates the new `report_missing` slice did not introduce a deterministic regression in that suite.
