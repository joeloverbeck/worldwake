# S59EXPOBLSUB-013: report_found and missing-report propagation

**Status**: PENDING
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

## Architecture Check

1. Keeping this follow-on work separate prevents `S59EXPOBLSUB-007` from inventing speculative search-result or institutional carriers before they exist.
2. No backward-compatibility shims should be introduced when the final report/result carrier is chosen.

## Verification Layers

1. Search-result-driven `report_found` admission -> focused runtime/action test
2. Owner/office propagation path -> authoritative world state and/or action trace at the concrete carrier chosen during implementation
3. Single-purpose follow-up ticket: exact proof surface depends on the reassessed final carrier

## What to Change

### 1. Implement report_found after search_place lands

- Add the lawful action admission path for actors who have just resolved a search
- Reuse existing tell/report infrastructure when possible
- Update expectation outcome and any grounded downstream records using a concrete current-branch carrier

### 2. Reassess richer report_missing propagation

- Decide whether office/agent notification belongs in `report_missing`, `report_found`, or a shared report carrier once search results exist
- Add any required shared institutional/tell carrier only if current live code actually needs it

## Files to Touch

- Reassess from the landed `search_place` boundary in `crates/worldwake-systems/src/search_actions.rs`, plus any required report/result carrier files discovered during implementation.

## Out of Scope

- Initial overdue-to-violation formalization already owned by `S59EXPOBLSUB-007`

## Acceptance Criteria

### Tests That Must Pass

1. `report_found` is only lawful when backed by the concrete search-result carrier chosen during implementation
2. Expectation resolution and downstream propagation use one canonical authoritative path
3. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. No telepathic or implicit global propagation of found-person results
2. No duplicate authority path for the same report/result fact

## Test Plan

### New/Modified Tests

1. Focused runtime/action tests at the concrete report/result carrier chosen after reassessment against the landed `search_place` boundary

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
