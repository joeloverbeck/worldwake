# S59EXPOBLSUB-013: report_found and missing-report propagation

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — follow-on report action and any required shared report/result carriers
**Deps**: S59EXPOBLSUB-007, S59EXPOBLSUB-009

## Problem

The S59 roadmap still needs the second half of the report lifecycle after `search_place` exists: found-person results and richer missing-report propagation are not yet owned by an active ticket. Without an explicit owner, `report_found` and any lawful office/agent propagation path would remain unstated follow-up work.

## Assumption Reassessment (2026-04-06)

1. `report_found` cannot land before `search_place` because the current branch has no authoritative stored/search-action carrier proving that an actor resolved a search.
2. The current institutional substrate has no missing-person claim type, so any office-record path must either add one or explicitly narrow to an existing lawful carrier.
3. `tell_actions.rs` already owns the canonical listener-memory update path for relayed beliefs/social observations; this ticket should reuse or extend that path instead of duplicating it.
4. This ticket exists because `S59EXPOBLSUB-007` was narrowed to the first honest slice: overdue expectation -> local `ViolationMemory`.
5. `S59EXPOBLSUB-009` has now landed `search_place` as a direct-subject action in `crates/worldwake-systems/src/search_actions.rs`, with `SearchPlaceActionPayload` and action-trace identity in `worldwake-sim`, expectation resolution on successful finds, and `LastSeenMemory` updates for found subjects. It still does not persist a reusable search-result/report carrier, so `report_found` must reassess from that narrower live boundary rather than assume a stored `SearchResult`.
6. Live correction: `search_place` already writes a reusable authoritative carrier for the owner-searcher slice, but it is not a standalone `SearchResult` component. The concrete current-branch carrier is the actor's resolved `ExpectationStore` record plus matching `LastSeenMemory` entry in `crates/worldwake-systems/src/search_actions.rs`.
7. Live mismatch: `PlannerOpKind::ReportFound` is already reserved in `crates/worldwake-ai/src/planner_ops.rs`, but there is no `ReportFoundActionPayload`, no `report_found` action registration in `crates/worldwake-systems/src/action_registry.rs`, and no `GoalKind::ReportFound` variant in `crates/worldwake-core/src/goal.rs`. This ticket can land the runtime action surface without widening into a new AI goal family.
8. Honest scope correction: this ticket owns the first lawful `report_found` runtime slice for direct colocated agent propagation from the reporter's resolved expectation outcome to another agent's `LastSeenMemory` / matching overdue expectations. Office or institutional missing-person record propagation remains a follow-up because `InstitutionalClaim` still has no missing/found-person record shape.

## Architecture Check

1. Reusing the actor's resolved `ExpectationStore` outcome plus `LastSeenMemory` is cleaner than inventing a second stored `SearchResult` authority path. It keeps one canonical post-search truth carrier for the direct agent-propagation slice.
2. Keeping institutional/office propagation separate prevents this ticket from widening shared record architecture speculatively when the live `InstitutionalClaim` surface still lacks a lawful missing-person claim type.
3. No backward-compatibility shims should be introduced when the final record/report path is chosen.

## Verification Layers

1. Resolved-expectation-backed `report_found` admission -> focused runtime/action test
2. `report_found` updates listener `LastSeenMemory` through one canonical hearsay path -> authoritative world state
3. `report_found` resolves listener overdue expectations for the reported subject through one canonical expectation-state path -> authoritative world state
4. Existing missing violations for the listener are cleared through `ViolationMemory` rather than left unresolved after a found report -> authoritative world state
5. Action catalog/payload fallout for `report_found` is wired cleanly -> focused payload/registry tests
6. Institutional/office missing-person record propagation is out of scope for this ticket after reassessment and must remain owned by a follow-up ticket

## What to Change

### 1. Implement report_found after search_place lands

- Add a lawful `report_found` action for actors who have already resolved a search into a found outcome in their own `ExpectationStore`
- Back the action with the concrete current-branch carrier: resolved `ExpectationStore` outcome plus matching `LastSeenMemory`, not a new stored `SearchResult` component
- Propagate the found result to a colocated interested agent by updating that listener's `LastSeenMemory`, resolving matching overdue expectations for the same subject, and clearing any matching active `ViolationKind::EntityMissing`

### 2. Reassess richer report_missing propagation

- Keep direct agent propagation in `report_found` as the canonical current slice
- Defer office/institutional missing-person record propagation to a follow-up ticket because the live institutional substrate still lacks a lawful missing/found-person claim carrier

## Files to Touch

- `crates/worldwake-systems/src/report_actions.rs` (modify — add `report_found` alongside the existing `report_missing` surface)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register `report_found`)
- `crates/worldwake-systems/src/lib.rs` (modify — re-export `report_found` registration)
- `crates/worldwake-sim/src/action_payload.rs` (modify — add `ReportFoundActionPayload` and `ActionPayload` accessors/tests)
- `crates/worldwake-sim/src/action_trace.rs` (modify — add `report_found` payload detail formatting/tests)
- `crates/worldwake-sim/src/lib.rs` (modify — re-export payload type)

## Out of Scope

- Initial overdue-to-violation formalization already owned by `S59EXPOBLSUB-007`
- New `GoalKind::ReportFound` AI admission/ranking/candidate work
- Institutional missing/found-person claim types or office-record propagation

## Acceptance Criteria

### Tests That Must Pass

1. `report_found` is only lawful when backed by the reporter's resolved found-outcome expectation plus matching `LastSeenMemory` carrier
2. Committing `report_found` updates the listener's `LastSeenMemory` through a single canonical hearsay path
3. Committing `report_found` resolves the listener's matching overdue expectation(s) for the reported subject with the same found outcome
4. Committing `report_found` resolves any matching active `ViolationKind::EntityMissing` for the listener
5. Action registry completeness includes `report_found`
3. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. No telepathic or implicit global propagation of found-person results
2. No duplicate authority path for the same report/result fact
3. This ticket uses the resolved expectation outcome as the canonical current-branch report carrier; it does not add a parallel stored `SearchResult` authority path

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/report_actions.rs` — focused affordance/commit tests for `report_found` admission and direct agent propagation
2. `crates/worldwake-systems/src/action_registry.rs` — action catalog includes `report_found`
3. `crates/worldwake-sim/src/action_payload.rs` and `crates/worldwake-sim/src/action_trace.rs` — payload/trace fallout coverage

### Commands

1. `cargo test -p worldwake-systems report_found`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed on 2026-04-07.

- Added the bounded `report_found` runtime slice in `crates/worldwake-systems/src/report_actions.rs` using the reporter's resolved found expectation plus matching `LastSeenMemory` as the canonical current-branch carrier
- Wired `report_found` through `worldwake-sim` payload and action-trace surfaces plus `worldwake-systems` action registration/export
- Kept listener overdue-expectation gating authoritative rather than belief-side, preserving locality: the actor can surface a colocated report affordance from its own lawful evidence, while start/commit still reject targets that do not actually hold an overdue expectation for the subject
- Deferred institutional missing/found-person record propagation to follow-up ticket `S59EXPOBLSUB-016`

## Verification Result

- Passed `cargo test -p worldwake-sim action_payload`
- Passed `cargo test -p worldwake-sim action_trace`
- Passed `cargo test -p worldwake-systems report_actions`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test -p worldwake-systems`
