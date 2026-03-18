# E16DPOLPLAN-018: Affordance payload enumeration verification tests

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

`enumerate_bribe_payloads` and `enumerate_threaten_payloads` are used by the action framework but have no direct unit tests verifying their payload generation semantics.

## Assumption Reassessment (2026-03-18)

1. `enumerate_bribe_payloads` is in `crates/worldwake-systems/src/office_actions.rs` ~line 214 — confirmed
2. `enumerate_threaten_payloads` is in `crates/worldwake-systems/src/office_actions.rs` ~line 291 — confirmed
3. Bribe payloads offer full commodity stock per payload — confirmed
4. No self-bribe or self-threaten — confirmed

## Architecture Check

1. Unit tests verifying payload generation — no AI or action execution involved
2. Tests the data contract that the planner and action framework depend on

## What to Change

### 1. Bribe payload tests in office_actions.rs

1. Verify full commodity quantity is offered per payload (5 bread → `offered_quantity: Quantity(5)`)
2. Verify no self-bribe (actor == target → empty Vec)
3. Verify empty when agent has no commodities
4. Verify multiple payloads for agent with multiple commodity types

### 2. Threaten payload tests in office_actions.rs

1. Verify correct target enumeration for agents at same location
2. Verify no self-threaten (actor excluded from targets)
3. Verify empty when no valid targets present

## Files to Touch

- `crates/worldwake-systems/src/office_actions.rs` (modify — test module)

## Out of Scope

- Bribe/Threaten action execution
- Planning semantics
- Golden tests
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `bribe_payload_offers_full_stock` — 5 bread → Quantity(5)
2. `bribe_payload_no_self_bribe` — empty Vec for self
3. `bribe_payload_empty_without_commodities` — empty Vec
4. `bribe_payload_multiple_commodity_types` — one payload per type
5. `threaten_payload_enumerates_colocated_targets` — correct targets
6. `threaten_payload_no_self_threaten` — actor excluded
7. `threaten_payload_empty_without_targets` — empty Vec
8. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Full stock offered per bribe payload (not partial)
2. Self-targeting never produced
3. Only co-located agents targeted

## Test Plan

### New/Modified Tests

1. `office_actions.rs::tests::bribe_payload_offers_full_stock`
2. `office_actions.rs::tests::bribe_payload_no_self_bribe`
3. `office_actions.rs::tests::bribe_payload_empty_without_commodities`
4. `office_actions.rs::tests::bribe_payload_multiple_commodity_types`
5. `office_actions.rs::tests::threaten_payload_enumerates_colocated_targets`
6. `office_actions.rs::tests::threaten_payload_no_self_threaten`
7. `office_actions.rs::tests::threaten_payload_empty_without_targets`

### Commands

1. `cargo test -p worldwake-systems office_actions`
2. `cargo test -p worldwake-systems`
