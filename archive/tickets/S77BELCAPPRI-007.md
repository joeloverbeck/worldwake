# S77BELCAPPRI-007: Reconcile hidden-event isolation proof with mandatory current-place observation

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Yes — `e15` hidden-event isolation proof corrected to the live current-place observation contract
**Deps**: S77BELCAPPRI-004, S77BELCAPPRI-006

## Problem

After `S77BELCAPPRI-006` restored self-local carried-item knowledge and the blocked `e09` path no longer stops the crate run early, broad `cargo test -p worldwake-systems` now reaches `crates/worldwake-systems/tests/e15_information_integration.rs::hidden_event_at_empty_location_remains_isolated_from_remote_agents` and fails on `assert!(store.known_entities.is_empty())`. This ticket must determine whether the hidden-event isolation proof is stale under the live mandatory current-place observation contract from `S77BELCAPPRI-004` or whether a production information leak now exists.

## Assumption Reassessment (2026-04-09)

1. `cargo test -p worldwake-systems` now fails in `hidden_event_at_empty_location_remains_isolated_from_remote_agents` at `crates/worldwake-systems/tests/e15_information_integration.rs:749`.
2. The failing assertion is about the remote observer's `AgentBeliefStore` being entirely empty after one tick, not about the hidden event's origin place or event payload specifically.
3. `S77BELCAPPRI-004` intentionally made agents always observe the place they currently occupy, so a fully empty `known_entities` assertion may now be stale even when hidden remote events remain isolated.
4. This is a mixed-boundary ticket. The exact contract under audit is hidden-event isolation in `e15_information_integration` versus the live perception/current-place observation boundary in `worldwake-systems::perception`.
5. Focused reproduction proved the failure is a stale proof, not a production leak. After one tick the remote observer's `known_entities` contains only `destination`, with a direct-observation place belief; `origin` remains absent and `social_observations` remain empty.
6. Auto-correction applied: ticket said either the `e15` proof or `worldwake-systems::perception` might need changes; live code has no unexpected remote information transfer, only a stale `is_empty()` assertion in `crates/worldwake-systems/tests/e15_information_integration.rs`. Correction applied: narrow owned scope to the integration proof. Safe because focused reproduction directly established the post-tick belief contents and they align with the live current-place observation contract from `S77BELCAPPRI-004`.

## Architecture Check

1. Correcting the proof to the strongest honest isolation invariant is cleaner than preserving an `is_empty()` assertion that may have been invalidated by later lawful perception changes.
2. No backward-compatibility shims. The ticket should tighten the test to the true hidden-event contract rather than preserving an outdated global-ignorance assertion.

## Verification Layers

1. The remote observer does not learn about the hidden remote event or its origin place -> focused `e15` integration proof
2. Existing suite: `cargo test -p worldwake-systems --test e15_information_integration`

## What to Change

### 1. Reassess the actual post-tick remote belief contents

Inspect which entities or claims are present in the remote observer's `AgentBeliefStore` after the hidden-event scenario and classify each as lawful current-place knowledge versus unexpected remote leakage.

### 2. Correct the owning boundary

Update the `e15` proof to assert the real invariant: the remote observer may lawfully know only its occupied destination place through direct observation, but must not learn the hidden event's origin place or payload.

## Files to Touch

- `crates/worldwake-systems/tests/e15_information_integration.rs` (modify)

## Out of Scope

- Reverting `S77BELCAPPRI-004` current-place observation
- Care/request-resolution behavior from `S77BELCAPPRI-006`
- Broad tell/planner/golden expansion beyond the hidden-event isolation proof

## Acceptance Criteria

### Tests That Must Pass

1. Focused: `cargo test -p worldwake-systems --test e15_information_integration hidden_event_at_empty_location_remains_isolated_from_remote_agents`
2. Existing suite: `cargo test -p worldwake-systems --test e15_information_integration`

### Invariants

1. Remote agents do not learn about hidden events at other places without a lawful information path.
2. The proof does not require the remote agent to be globally ignorant of their own current place if current-place observation is now a live contract.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/e15_information_integration.rs` — `hidden_event_at_empty_location_remains_isolated_from_remote_agents` now asserts that only the remote agent's occupied destination place is known, while the hidden origin remains absent

### Commands

1. `cargo test -p worldwake-systems --test e15_information_integration hidden_event_at_empty_location_remains_isolated_from_remote_agents`
2. `cargo test -p worldwake-systems --test e15_information_integration`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Reassessed the `e15` failure against the live current-place observation contract from `S77BELCAPPRI-004`.
- Confirmed there is no hidden-event leak: after one tick the remote observer lawfully knows only `destination` via direct place observation, while `origin` remains unknown and no social observations are created.
- Updated `hidden_event_at_empty_location_remains_isolated_from_remote_agents` to assert that exact contract instead of requiring `store.known_entities` to be globally empty.

## Verification Result

- Passed `cargo test -p worldwake-systems --test e15_information_integration hidden_event_at_empty_location_remains_isolated_from_remote_agents`
- Passed `cargo test -p worldwake-systems --test e15_information_integration`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
