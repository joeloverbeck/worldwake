# E14PERBEL-001: Remove FactId/KnowsFact/BelievesFact Scaffolding

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — removes unused relation types and ID type from core
**Deps**: None (standalone cleanup, no code depends on this scaffolding)

## Problem

`FactId`, `KnowsFact`, and `BelievesFact` are unused scaffolding from early design. Zero production code paths reference them, and no AI golden tests exercise them. E14 replaces the fact-based belief model with a state-snapshot model (`AgentBeliefStore`). This scaffolding must be removed cleanly before introducing the new types (Principle 26: No Backward Compatibility).

## Assumption Reassessment (2026-03-14)

1. `FactId` is defined in `ids.rs` via `worldwake_prefixed_id_type!` macro — confirmed at line 96.
2. `KnowsFact` and `BelievesFact` exist as variants in both `RelationKind` and `RelationValue` in `delta.rs` — confirmed at lines 68-69 and 126-133.
3. `RelationTables` has `knows_fact` and `believes_fact` fields in `relations.rs` — confirmed at lines 70-71.
4. `world/social.rs` has 6 fact-related methods (lines 318-401) — confirmed.
5. `world_txn.rs` has 4 fact-related transaction methods (lines 607-661) — confirmed.
6. `verification.rs` references `KnowsFact`/`BelievesFact` in `verify_completeness()` — confirmed at lines 350-356, 400.
7. `event_record.rs` uses `FactId` and `KnowsFact` in test fixtures — confirmed at lines 139, 295-298, 375-378.
8. No production code outside definitions and test fixtures uses these types — confirmed by grep.

## Architecture Check

1. Pure deletion of unused code — simplest possible change, no alternatives needed.
2. No backward-compatibility shims — these types have zero consumers, clean removal is correct.

## What to Change

### 1. Remove `FactId` from `ids.rs`

Delete the `worldwake_prefixed_id_type!` invocation for `FactId` (line 96 area). Remove any `FactId` re-export from `lib.rs` if present.

### 2. Remove `KnowsFact`/`BelievesFact` from `delta.rs`

Delete `KnowsFact` and `BelievesFact` variants from both `RelationKind` and `RelationValue` enums. Update any match arms in the same file that handle these variants. Fix test fixtures that reference them.

### 3. Remove fact fields from `RelationTables` in `relations.rs`

Delete `knows_fact` and `believes_fact` fields from the `RelationTables` struct. Remove `remove_fact_relations()` helper. Remove any `Default` or initialization code that sets these fields. Update `remove_all_relations()` or equivalent cleanup methods.

### 4. Remove fact methods from `world/social.rs`

Delete all 6 methods: `known_facts()`, `add_known_fact()`, `remove_known_fact()`, `believed_facts()`, `add_believed_fact()`, `remove_believed_fact()`.

### 5. Remove fact methods from `world_txn.rs`

Delete all 4 transaction methods: `add_known_fact()`, `remove_known_fact()`, `add_believed_fact()`, `remove_believed_fact()`.

### 6. Remove fact references from `verification.rs`

Remove `KnowsFact`/`BelievesFact` handling from `verify_completeness()` and any other verification functions.

### 7. Remove fact references from `event_record.rs`

Remove `FactId` import and update test fixtures that create sample `KnowsFact` relation deltas. These are test-only — production event records never contain fact relations.

## Files to Touch

- `crates/worldwake-core/src/ids.rs` (modify — remove `FactId`)
- `crates/worldwake-core/src/delta.rs` (modify — remove `KnowsFact`/`BelievesFact` variants + fix tests)
- `crates/worldwake-core/src/relations.rs` (modify — remove fact fields + helpers)
- `crates/worldwake-core/src/world/social.rs` (modify — remove 6 fact methods)
- `crates/worldwake-core/src/world_txn.rs` (modify — remove 4 fact txn methods)
- `crates/worldwake-core/src/verification.rs` (modify — remove fact verification)
- `crates/worldwake-core/src/event_record.rs` (modify — remove fact test fixtures)
- `crates/worldwake-core/src/lib.rs` (modify — remove `FactId` re-export if present)

## Out of Scope

- Adding any new types (that's E14PERBEL-002)
- Changing `RelationKind`/`RelationValue` variants other than `KnowsFact`/`BelievesFact`
- Modifying any crate other than `worldwake-core`
- Touching `BeliefView` trait or `OmniscientBeliefView`
- Modifying `WitnessData`, `VisibilitySpec`, or any event log infrastructure

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core` — all existing tests pass after removal
2. `cargo test --workspace` — no other crate breaks from the removal
3. `cargo clippy --workspace` — no new warnings
4. Grep for `FactId`, `KnowsFact`, `BelievesFact`, `knows_fact`, `believes_fact` across workspace returns zero matches (excluding archived specs and docs)

### Invariants

1. No production code path was broken (these types were unused scaffolding)
2. `RelationKind` and `RelationValue` remain exhaustive in all match arms
3. `RelationTables` still initializes correctly with remaining fields
4. Serialization format for `RelationKind`/`RelationValue` is not a concern — no persisted data uses these variants
5. Determinism invariant unaffected — `BTreeMap`/`BTreeSet` usage unchanged for remaining types

## Test Plan

### New/Modified Tests

1. No new tests needed — this is pure deletion of unused code
2. Existing tests in `delta.rs` and `event_record.rs` must be updated to remove fact-related fixtures

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
