# E16DPOLPLAN-007: Golden harness extensions — office/faction helpers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — golden test harness
**Deps**: None

## Problem

The `GoldenHarness` in `golden_harness/mod.rs` lacks helpers for creating offices, factions, faction membership, loyalty, and courage — all needed by golden E2E scenarios 11-20.

## Assumption Reassessment (2026-03-18)

1. `GoldenHarness` is in `crates/worldwake-ai/tests/golden_harness/mod.rs` — confirmed
2. `OfficeData` component exists with `succession_law`, `succession_period`, `eligibility_rule` fields — confirmed (from E16 spec)
3. `FactionData` component exists — confirmed
4. Loyalty is a relation in `RelationTables` — confirmed
5. `UtilityProfile.courage` field exists — confirmed

## Architecture Check

1. Follows existing harness helper patterns (e.g., `seed_agent`, `seed_bread`, `seed_topology`)
2. Helpers are thin wrappers around world mutation APIs — no business logic in harness

## What to Change

### 1. `seed_office(place, succession_law, succession_period, eligibility_rule) -> EntityId`

Create Office entity with `OfficeData` component at a jurisdiction.

### 2. `seed_faction(name) -> EntityId`

Create Faction entity with `FactionData` component.

### 3. `add_faction_membership(agent, faction)`

Add `member_of` relation between agent and faction.

### 4. `set_loyalty(subject, target, value: Permille)`

Seed loyalty relation between two agents.

### 5. `set_courage(agent, value: Permille)`

Update agent's `UtilityProfile.courage` field.

### 6. `enterprise_weighted_utility(enterprise: Permille) -> UtilityProfile`

Create utility profile with high enterprise weight for political goal generation.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)

## Out of Scope

- Golden test scenarios themselves (separate tickets E16DPOLPLAN-008 through E16DPOLPLAN-021)
- Changes to `OfficeData`, `FactionData`, or relation APIs
- Changes to production code in any crate

## Acceptance Criteria

### Tests That Must Pass

1. Harness compiles: `cargo test -p worldwake-ai --no-run`
2. `seed_office` creates an entity with `OfficeData` component retrievable via world query
3. `seed_faction` creates an entity with `FactionData` component
4. `add_faction_membership` creates a queryable relation
5. `set_loyalty` creates a queryable loyalty relation
6. `set_courage` updates the `UtilityProfile.courage` field
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No business logic in harness — helpers are pure state setup
2. Existing golden tests unaffected (no API changes to existing helpers)

## Test Plan

### New/Modified Tests

1. Harness helpers verified implicitly by golden tests in subsequent tickets

### Commands

1. `cargo test -p worldwake-ai --no-run`
2. `cargo clippy --workspace`
