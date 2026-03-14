# E14PERBEL-001: Remove FactId/KnowsFact/BelievesFact Scaffolding

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — removes obsolete fact identifiers and fact-relation plumbing from `worldwake-core`
**Deps**: None

## Problem

`FactId`, `KnowsFact`, and `BelievesFact` are legacy scaffolding from an earlier fact-centric belief direction. E14’s accepted architecture replaces that direction with per-agent state snapshots (`AgentBeliefStore`, `BelievedEntityState`, `PerceptionSource`) rather than opaque fact handles. Keeping both models alive violates Principle 26 (`docs/FOUNDATIONS.md`): no backward compatibility, no alias paths, no dead abstractions.

The important correction is that this scaffolding is not merely “unused definitions.” It is still wired into core relation storage, archive snapshots, transaction delta emission, verification, serialization tests, and archive/purge tests. The ticket must remove that plumbing cleanly rather than treating it as a no-op deletion.

## Assumption Reassessment (2026-03-14)

### Confirmed Current State

1. `FactId` exists in `crates/worldwake-core/src/ids.rs` and is re-exported from `crates/worldwake-core/src/lib.rs`.
2. `RelationKind::{KnowsFact, BelievesFact}` and `RelationValue::{KnowsFact, BelievesFact}` exist in `crates/worldwake-core/src/delta.rs`.
3. `RelationTables` stores fact relations in `crates/worldwake-core/src/relations.rs` via `knows_fact` and `believes_fact`.
4. `crates/worldwake-core/src/world/social.rs` exposes public world APIs for fact relations: `add_known_fact`, `remove_known_fact`, `known_facts`, `add_believed_fact`, `remove_believed_fact`, `believed_facts`.
5. `crates/worldwake-core/src/world_txn.rs` exposes transactional wrappers for the same fact relation APIs and emits relation deltas for them.
6. `crates/worldwake-core/src/verification.rs` treats fact relations as part of the expected live-world relation set.
7. `crates/worldwake-core/src/world/lifecycle.rs` includes fact relations in `ArchiveMutationSnapshot`, so archive teardown currently models them as part of entity mutation state.
8. `crates/worldwake-core/src/world.rs`, `relations.rs`, `delta.rs`, `world_txn.rs`, `event_record.rs`, and `world/lifecycle.rs` all contain tests or fixtures that currently exercise this scaffolding.

### Discrepancies From The Previous Ticket Version

1. The previous ticket said “no production code path” uses this scaffolding. That is too strong. No gameplay system depends on fact-based beliefs, but core engine code still carries and mutates these relations through archive, verification, and transaction infrastructure.
2. The previous ticket omitted `crates/worldwake-core/src/world.rs` and `crates/worldwake-core/src/world/lifecycle.rs`, both of which must change.
3. The previous ticket understated test impact. This work requires updating a substantial set of existing tests, not just `delta.rs` and `event_record.rs`.
4. Workspace grep currently finds one follow-on spec mismatch: `specs/E15-rumor-witness-discovery.md` still mentions `BelievesFact`. That contradiction should be corrected in later E15/E14 spec alignment work, but this ticket remains code-focused.

## Architecture Reassessment

### Is Removal Better Than The Current Architecture?

Yes.

The current architecture keeps a second, abstract belief representation alive inside the authoritative relation layer even though E14 explicitly moves belief state toward concrete per-agent snapshots. That duplication is harmful:

1. It preserves a dead abstraction in authoritative state.
2. It forces archive, verification, and transaction code to carry relation types that should no longer exist.
3. It increases future migration risk by making it easier for later work to accidentally reuse `FactId` instead of building on `AgentBeliefStore`.

Pure removal is the correct direction. No compatibility shim, alias, or “temporary bridge” should be introduced.

### Ideal-Architecture Note

While doing this cleanup, one architectural smell becomes clearer: archive teardown snapshots in `world/lifecycle.rs` manually mirror many relation categories one-by-one. That is acceptable for this ticket, but it means obsolete relation kinds leak widely when added. A later cleanup could consider consolidating archive snapshot capture around relation categories or reusable helpers so future relation-model changes have a smaller blast radius. That is not in scope here.

## What To Change

### 1. Remove `FactId`

- Delete `FactId` from `crates/worldwake-core/src/ids.rs`.
- Remove the `FactId` re-export from `crates/worldwake-core/src/lib.rs`.
- Remove `FactId`-specific tests in `ids.rs`.

### 2. Remove fact relation kinds and values

- Delete `KnowsFact` and `BelievesFact` from `RelationKind`.
- Delete `KnowsFact { agent, fact }` and `BelievesFact { agent, fact }` from `RelationValue`.
- Update `RelationKind::ALL`, `RelationValue::kind()`, and all delta tests/fixtures accordingly.

### 3. Remove fact storage from `RelationTables`

- Delete `knows_fact` and `believes_fact` from `RelationTables`.
- Remove fact-specific cleanup helpers and `remove_all()` handling.
- Update relation-table serialization and purge tests to reflect the remaining authoritative relation set.

### 4. Remove fact APIs from `World`

- Delete public/query mutation APIs in `crates/worldwake-core/src/world/social.rs`:
  - `add_known_fact`
  - `remove_known_fact`
  - `known_facts`
  - `add_believed_fact`
  - `remove_believed_fact`
  - `believed_facts`
- Remove fact-only helper logic used exclusively by those APIs.
- Update `crates/worldwake-core/src/world.rs` tests that currently validate fact behavior or fact cleanup.

### 5. Remove fact APIs from `WorldTxn`

- Delete transactional wrappers that stage fact mutations and emit relation deltas.
- Delete archive-teardown delta helpers for removed fact relations.
- Update transaction tests that currently assert fact relation deltas or archive cleanup deltas.

### 6. Remove fact handling from archive snapshots and verification

- Delete `known_facts` and `believed_facts` from `ArchiveMutationSnapshot`.
- Remove any snapshot helper logic that only exists for fact capture.
- Remove fact relation collection from `verify_completeness()` and related liveness checks.

### 7. Remove fact-only test fixtures from event/serialization coverage

- Update `crates/worldwake-core/src/event_record.rs` roundtrip tests to use still-valid relation deltas instead of fact relations.
- Update any other serialization fixtures that currently rely on `FactId` only to populate a “sample relation.”

## Files To Touch

- `crates/worldwake-core/src/ids.rs`
- `crates/worldwake-core/src/lib.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/relations.rs`
- `crates/worldwake-core/src/world/social.rs`
- `crates/worldwake-core/src/world/lifecycle.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/world_txn.rs`
- `crates/worldwake-core/src/verification.rs`
- `crates/worldwake-core/src/event_record.rs`

## Out Of Scope

- Implementing `AgentBeliefStore`, `BelievedEntityState`, `PerceptionSource`, or `PerAgentBeliefView`
- Editing AI/planner behavior
- Broad refactors of the archive system beyond removing fact-specific branches
- Keeping any compatibility wrapper for `FactId` or fact relations
- Full spec cleanup outside this ticket; note the E15 contradiction but do not expand this ticket into unrelated spec rewrites

## Acceptance Criteria

### Behavior / Architecture

1. `worldwake-core` no longer defines or exports `FactId`.
2. The authoritative relation model no longer contains fact-based belief relations.
3. Archive, verification, purge, and transaction code no longer mention fact relations.
4. No compatibility aliases, wrappers, or placeholder dead types are introduced.

### Code Search

1. Workspace grep for `FactId`, `KnowsFact`, `BelievesFact`, `knows_fact`, and `believes_fact` returns no code matches outside archived material and non-updated future specs.
2. Remaining spec references outside this ticket are explicitly noted if they still exist after code cleanup.

## Test Plan

### New / Modified Tests Required

This is not “no new tests needed.” The bug here is architectural drift: obsolete relation kinds still participate in authoritative state. Existing tests must be strengthened or updated so they verify the cleaned architecture rather than silently preserving the dead path.

Required coverage:

1. Update ID serialization/bounds tests to remove `FactId`.
2. Update relation kind/value tests so they assert the cleaned relation set.
3. Update relation-table roundtrip and `remove_all()` tests to prove the remaining relation storage still serializes and purges correctly after fact-row removal.
4. Update archive/purge tests in `world.rs` and `world/lifecycle.rs` so they continue covering teardown behavior without any fact fields.
5. Update `WorldTxn` tests so relation-delta coverage still exists using live relation kinds only.
6. Update event-record roundtrip tests to use valid non-fact relation deltas.
7. Remove fact-specific API tests and preserve the surrounding invariants those tests were really checking, where still relevant.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Completion Notes

When implementation is complete:

1. Mark this ticket `COMPLETED`.
2. Add an `Outcome` section summarizing what actually changed versus the original plan.
3. Archive the completed ticket under `archive/` in a fitting E14/phase-aligned location.

## Outcome

- **Completion date**: 2026-03-14
- **What actually changed**: Removed `FactId` from core IDs and exports; deleted `KnowsFact` / `BelievesFact` from delta enums and relation storage; removed fact APIs from `World` and `WorldTxn`; removed fact handling from archive snapshots and verification; updated serialization, archive, purge, and transaction tests to use only live relation kinds.
- **Deviations from original plan**: The original ticket understated the blast radius. The actual implementation also required removing fact fields from `ArchiveMutationSnapshot`, updating `world.rs` and `world/lifecycle.rs`, and repairing a broader set of tests than originally listed.
- **Verification results**:
  - `cargo test -p worldwake-core` ✅
  - `cargo test --workspace` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `rg -n "FactId|KnowsFact|BelievesFact|knows_fact|believes_fact|known_facts|believed_facts|add_known_fact|add_believed_fact|remove_known_fact|remove_believed_fact" crates/worldwake-core/src` ✅ no matches
- **Follow-on note**: `specs/E15-rumor-witness-discovery.md` still mentions `BelievesFact`; that spec contradiction remains to be corrected in later spec-alignment work.
