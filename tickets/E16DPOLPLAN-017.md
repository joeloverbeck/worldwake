# E16DPOLPLAN-017: Succession resolution verification test

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

The Politics system `succession_system()` resolution path (vacancy detection → support counting → office holder installation) is assumed by all golden scenarios but never directly tested in isolation.

## Assumption Reassessment (2026-03-18)

1. `succession_system()` is in `crates/worldwake-systems/src/offices.rs` — confirmed
2. Support declarations are relations in `RelationTables` — confirmed
3. `office_holder` relation is set on installation — confirmed
4. `succession_period` on `OfficeData` controls delay before resolution — confirmed

## Architecture Check

1. Isolated unit test of succession resolution — no AI involvement
2. Directly tests the system function with manually seeded state

## What to Change

### 1. Add unit test in offices.rs test module

Tests:
1. Create a vacant office with Support succession law (period=5)
2. Add support declarations from two agents for different candidates (2 for A, 1 for B)
3. Run `succession_system()` for the required succession period
4. Verify the candidate with more support (A) is installed as holder
5. Verify the `office_holder` relation is correctly set

## Files to Touch

- `crates/worldwake-systems/src/offices.rs` (modify — test module)

## Out of Scope

- Force succession testing (covered by E16DPOLPLAN-015 golden test)
- AI-driven support generation
- Golden E2E tests
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `succession_resolution_installs_majority_candidate` — A installed with 2 vs 1 support
2. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Candidate with most support wins
2. `office_holder` relation correctly set after installation
3. Resolution only happens after `succession_period` ticks

## Test Plan

### New/Modified Tests

1. `offices.rs::tests::succession_resolution_installs_majority_candidate`

### Commands

1. `cargo test -p worldwake-systems offices`
2. `cargo test -p worldwake-systems`
