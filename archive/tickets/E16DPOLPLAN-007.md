# E16DPOLPLAN-007: Golden harness extensions — office/faction helpers

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — golden test harness
**Deps**: None

## Problem

The `GoldenHarness` in `golden_harness/mod.rs` lacks helpers for creating offices, factions, faction membership, loyalty, and courage — all needed by golden E2E scenarios 11-20.

## Assumption Reassessment (2026-03-18, corrected)

1. `GoldenHarness` is in `crates/worldwake-ai/tests/golden_harness/mod.rs` — confirmed
2. `OfficeData` component exists with fields: `title: String`, `jurisdiction: EntityId`, `succession_law: SuccessionLaw`, `eligibility_rules: Vec<EligibilityRule>`, `succession_period_ticks: u64`, `vacancy_since: Option<Tick>` — confirmed
3. `FactionData` component exists with fields: `name: String`, `purpose: FactionPurpose` — confirmed
4. Loyalty is a relation in `RelationTables` (`loyal_to` / `loyalty_from`) — confirmed. API: `txn.set_loyalty(subject, target, strength)`
5. `UtilityProfile.courage` field exists — confirmed
6. `WorldTxn::create_office(name)` creates Office entity with `Name` component — confirmed. `set_component_office_data` sets the `OfficeData` component separately.
7. `WorldTxn::create_faction(name)` creates Faction entity with `Name` component — confirmed. `set_component_faction_data` sets the `FactionData` component separately.
8. `WorldTxn::add_member(member, faction)` adds `member_of` relation — confirmed

## Architecture Check

1. Follows existing harness helper patterns (e.g., `seed_agent`, `seed_bread`, `seed_topology`)
2. Helpers are thin wrappers around world mutation APIs — no business logic in harness

## What to Change

### 1. `seed_office(title, jurisdiction, succession_law, succession_period_ticks, eligibility_rules) -> EntityId`

Create Office entity with `OfficeData` component. Sets `vacancy_since: Some(Tick(0))` to mark initially vacant. Parameters: `title: &str`, `jurisdiction: EntityId`, `succession_law: SuccessionLaw`, `succession_period_ticks: u64`, `eligibility_rules: Vec<EligibilityRule>`.

### 2. `seed_faction(name, purpose) -> EntityId`

Create Faction entity with `FactionData` component. Parameters: `name: &str`, `purpose: FactionPurpose`.

### 3. `add_faction_membership(agent, faction)`

Add `member_of` relation between agent and faction via `txn.add_member(member, faction)`.

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

## Outcome

- **Completion date**: 2026-03-18
- **What changed**: Added 6 helper functions to `crates/worldwake-ai/tests/golden_harness/mod.rs`: `seed_office`, `seed_faction`, `add_faction_membership`, `set_loyalty`, `set_courage`, `enterprise_weighted_utility`. Added imports for `EligibilityRule`, `FactionData`, `FactionPurpose`, `OfficeData`, `SuccessionLaw`.
- **Deviations from original plan**: Corrected helper signatures to match actual codebase APIs — `seed_office` takes `title`, `jurisdiction`, `succession_period_ticks`, `eligibility_rules: Vec<EligibilityRule>` (not the original singular/shortened names). `seed_faction` takes `purpose: FactionPurpose` in addition to `name`.
- **Verification**: `cargo test -p worldwake-ai` — 11 tests pass. `cargo clippy -p worldwake-ai` — clean.
