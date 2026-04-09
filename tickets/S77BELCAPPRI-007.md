# S77BELCAPPRI-007: Reconcile hidden-event isolation proof with mandatory current-place observation

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Yes — `e15` hidden-event isolation proof or its owning read surface must be corrected
**Deps**: S77BELCAPPRI-004, S77BELCAPPRI-006

## Problem

After `S77BELCAPPRI-006` restored self-local carried-item knowledge and the blocked `e09` path no longer stops the crate run early, broad `cargo test -p worldwake-systems` now reaches `crates/worldwake-systems/tests/e15_information_integration.rs::hidden_event_at_empty_location_remains_isolated_from_remote_agents` and fails on `assert!(store.known_entities.is_empty())`. This ticket must determine whether the hidden-event isolation proof is stale under the live mandatory current-place observation contract from `S77BELCAPPRI-004` or whether a production information leak now exists.

## Assumption Reassessment (2026-04-09)

1. `cargo test -p worldwake-systems` now fails in `hidden_event_at_empty_location_remains_isolated_from_remote_agents` at `crates/worldwake-systems/tests/e15_information_integration.rs:749`.
2. The failing assertion is about the remote observer's `AgentBeliefStore` being entirely empty after one tick, not about the hidden event's origin place or event payload specifically.
3. `S77BELCAPPRI-004` intentionally made agents always observe the place they currently occupy, so a fully empty `known_entities` assertion may now be stale even when hidden remote events remain isolated.
4. This is a mixed-boundary ticket. The exact contract under audit is hidden-event isolation in `e15_information_integration` versus the live perception/current-place observation boundary in `worldwake-systems::perception`.
5. Root cause has not yet been proved. The remote agent may now lawfully know only their own current place, or an actual remote information leak may exist. That distinction must be established before any test or production change.

## Architecture Check

1. Correcting the proof to the strongest honest isolation invariant is cleaner than preserving an `is_empty()` assertion that may have been invalidated by later lawful perception changes.
2. No backward-compatibility shims. The ticket should either tighten the test to the true hidden-event contract or fix a real production leak at the owning information boundary.

## Verification Layers

1. The remote observer does not learn about the hidden remote event or its origin place -> focused `e15` integration proof
2. If reassessment finds a production leak, the strongest lower-layer proof must identify the exact perception or event-projection boundary that leaks
3. Existing suite: `cargo test -p worldwake-systems --test e15_information_integration`

## What to Change

### 1. Reassess the actual post-tick remote belief contents

Inspect which entities or claims are present in the remote observer's `AgentBeliefStore` after the hidden-event scenario and classify each as lawful current-place knowledge versus unexpected remote leakage.

### 2. Correct the owning boundary

If the hidden-event isolation contract still holds and only the blanket-empty assertion is stale, update the `e15` proof to assert the real invariant. If unexpected remote knowledge leaked in, fix production code at the owning perception/event boundary instead.

## Files to Touch

- `crates/worldwake-systems/tests/e15_information_integration.rs` (modify if the proof is stale)
- `crates/worldwake-systems/src/perception.rs` (modify only if reassessment proves a production leak)

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

1. `crates/worldwake-systems/tests/e15_information_integration.rs` — tighten the hidden-event isolation assertion to the real contract after reassessment

### Commands

1. `cargo test -p worldwake-systems --test e15_information_integration hidden_event_at_empty_location_remains_isolated_from_remote_agents`
2. `cargo test -p worldwake-systems --test e15_information_integration`
3. `cargo clippy --workspace --all-targets -- -D warnings`
