# E16OFFSUCFAC-003: Add `support_declarations` Social Relation

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new relation storage and relation delta/value plumbing in `worldwake-core`
**Deps**: E16OFFSUCFAC-001

## Problem

E16 needs a public `support_declarations` relation so agents can declare support for a candidate for a given office without changing underlying loyalty. The E16 spec intentionally separates public support from private loyalty so coercion, bribery, and hidden allegiance can coexist.

The current codebase already has the general social-relation architecture this feature should follow:

- canonical storage in `RelationTables`
- low-level world mutation/query helpers in `crates/worldwake-core/src/world/social.rs`
- generic relation event deltas via `RelationDelta::{Added, Removed}` plus `RelationKind` / `RelationValue`
- archive blocking/cleanup through `archive_dependencies`, archive preparation, and `remove_all`

This ticket should extend that architecture cleanly rather than introduce a one-off relation path.

## Assumption Reassessment (2026-03-15)

### Confirmed

1. `RelationTables` already stores the existing social relations (`member_of`, `loyal_to`, `office_holder`, `hostile_to`) with deterministic `BTreeMap` / `BTreeSet` storage.
2. Social relation mutations are exposed through world helpers in `crates/worldwake-core/src/world/social.rs` and transaction wrappers in `crates/worldwake-core/src/world_txn.rs`.
3. `RelationDelta` is generic. Relation-specific changes are represented by `RelationDelta::{Added, Removed}` carrying `RelationKind` and `RelationValue`.
4. `archive_dependencies`, `archive_mutation_snapshot`, `verification`, and relation collection logic all participate in the canonical relation model.
5. The E16 spec stores `support_declarations` as a single authoritative forward map: `(supporter, office) -> candidate`.

### Incorrect assumptions in the previous draft

1. `RelationDelta` does **not** get relation-specific enum variants such as `DeclareSupport` or `ClearDeclarationsForOffice`. The correct extension point is `RelationKind` / `RelationValue`.
2. A second authoritative reverse index such as `declarers_for_office` is **not** implied by the existing architecture and is not justified yet. It would add synchronization burden and duplicate source-of-truth state before there is evidence that scans are a bottleneck.
3. Archive cleanup should not be bolted on ad hoc. If support declarations can block archive the same way other inbound social relations do, they must integrate with the existing archive dependency / preparation model.
4. The original file list was incomplete. Adding a new canonical relation affects more than `relations.rs`, `delta.rs`, and `world_txn.rs`.

## Architecture Decision

### Keep

1. Add `support_declarations` as a first-class canonical relation. This is consistent with E16 and improves the long-term model because public support is distinct world state, not an alias for loyalty.
2. Store it as the spec defines: `BTreeMap<(EntityId, EntityId), EntityId>` keyed by `(supporter, office)` with a single candidate value.
3. Expose mutation/query helpers through the same world and transaction APIs used by the other social relations.

### Reject

1. Do **not** add a reverse index in this ticket. Counting and iteration for an office can derive from the canonical map. If profiling later proves this hot, a derived cache can be introduced deliberately.
2. Do **not** add bespoke relation delta variants. That would fork the event model away from the generic relation architecture already in place.

### Benefit vs current architecture

This change is more beneficial than the current architecture because E16 currently lacks a world-state carrier for public political support. Without it, succession logic would have to overload loyalty or invent handler-local state, both of which are architecturally worse:

- overloading loyalty would collapse two distinct causal concepts into one
- handler-local or system-local state would violate the project’s preference for explicit world state
- a canonical relation keeps this inspectable, serializable, deterministic, and event-sourced

The robust version of this feature is therefore: add the new relation cleanly inside the existing relation framework, with no compatibility layer and no duplicate storage.

## Correct Scope

### 1. Add canonical relation storage

In `crates/worldwake-core/src/relations.rs`:

```rust
// (supporter, office) -> candidate
pub(crate) support_declarations: BTreeMap<(EntityId, EntityId), EntityId>,
```

`remove_all()` must remove rows where the archived/purged entity appears as:

- `supporter`
- `office`
- `candidate`

Archive dependency reporting must also account for inbound declarations where appropriate, consistent with the existing social-relation archive model.

### 2. Extend canonical relation enums

In `crates/worldwake-core/src/delta.rs`:

- add `RelationKind::SupportDeclaration`
- add `RelationValue::SupportDeclaration { supporter, office, candidate }`
- update any `ALL`, `kind()`, sample, serde, or exhaustive-match tests accordingly

### 3. Add world-level support declaration APIs

In `crates/worldwake-core/src/world/social.rs`:

- `declare_support(supporter, office, candidate) -> Result<(), WorldError>`
- `clear_support_declaration(supporter, office) -> Result<(), WorldError>`
- `support_declaration(supporter, office) -> Option<EntityId>`
- `support_declarations_for_office(office) -> Vec<(EntityId, EntityId)>`
- `count_support_declarations_for_candidate(office, candidate) -> usize`
- `clear_support_declarations_for_office(office) -> Result<(), WorldError>`

These should follow current patterns:

- validate live entities / correct office kind
- overwrite the prior declaration for the same `(supporter, office)` pair
- avoid recording redundant no-op changes at higher layers

### 4. Add transaction wrappers using generic relation deltas

In `crates/worldwake-core/src/world_txn.rs`:

- wrap world mutations with the same before/after comparison pattern already used for loyalty and office assignment
- emit generic `RelationDelta::{Added, Removed}` entries carrying `RelationKind::SupportDeclaration`
- ensure overwrite emits `Removed(old)` then `Added(new)` for the same `(supporter, office)` pair when the candidate changes
- include support declarations in archive mutation snapshots / removed relation deltas so event log teardown remains truthful

### 5. Update world verification / relation collection

Because this becomes a first-class canonical relation, the usual world verification plumbing must be updated:

- relation collection in `verification.rs`
- relation liveness checks in `verification.rs`
- any exhaustive relation matching in `world_txn.rs` or related tests

## Files To Touch

- `crates/worldwake-core/src/relations.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world/social.rs`
- `crates/worldwake-core/src/world/lifecycle.rs`
- `crates/worldwake-core/src/world.rs`
- `crates/worldwake-core/src/world_txn.rs`
- `crates/worldwake-core/src/verification.rs`

## Out Of Scope

- `DeclareSupport` action handler behavior (E16OFFSUCFAC-006)
- succession resolution logic that consumes declarations (E16OFFSUCFAC-007)
- AI planner ops or goal generation
- event payload definitions in `worldwake-sim`
- speculative indexing/caching beyond the canonical forward map

## Acceptance Criteria

### Tests That Must Pass

1. Declaring support stores `(supporter, office) -> candidate` and makes it queryable.
2. Re-declaring support for the same `(supporter, office)` overwrites the previous candidate.
3. `support_declarations_for_office(office)` returns all `(supporter, candidate)` pairs for that office in deterministic order.
4. `count_support_declarations_for_candidate(office, candidate)` returns the correct count.
5. `clear_support_declaration(supporter, office)` removes only that declaration.
6. `clear_support_declarations_for_office(office)` removes all declarations for that office.
7. `WorldTxn` records `Added` support-declaration relation deltas on first declaration.
8. `WorldTxn` records `Removed(old)` then `Added(new)` when a declaration is overwritten.
9. Archive/purge cleanup removes stale support declarations when the archived entity is the supporter, office, or candidate.
10. Archive dependency / snapshot behavior remains consistent with the rest of the social relation model.
11. Save/load roundtrip preserves support declarations.
12. `cargo clippy --workspace --all-targets -- -D warnings`
13. `cargo test --workspace`

### Invariants

1. All authoritative storage remains deterministic (`BTreeMap` / `BTreeSet` only).
2. At most one support declaration exists for each `(supporter, office)` pair.
3. Support declarations remain distinct from loyalty; no aliasing or compatibility wrapper is introduced.
4. Relation deltas remain generic; this feature does not introduce relation-specific delta enums.
5. Any archive semantics for inbound declarations follow the existing archive dependency model rather than an ad hoc side path.

## Test Plan

### New / Modified Tests

1. `crates/worldwake-core/src/relations.rs`
   - storage roundtrip and `remove_all()` coverage for supporter / office / candidate positions
2. `crates/worldwake-core/src/world.rs`
   - world-level support declaration query / overwrite / clear behavior
3. `crates/worldwake-core/src/world_txn.rs`
   - transaction delta behavior for add / overwrite / clear / archive teardown
4. `crates/worldwake-core/src/delta.rs`
   - relation kind/value coverage and serde roundtrip updates
5. `crates/worldwake-core/src/verification.rs`
   - world verification captures support declarations as canonical relations

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completed: 2026-03-15
- Actually changed:
  - Added canonical `support_declarations` storage to `RelationTables`
  - Extended the generic relation model with `RelationKind::SupportDeclaration` and `RelationValue::SupportDeclaration`
  - Added world and transaction support-declaration APIs for declare/query/clear/count operations
  - Integrated archive dependency, archive snapshot, teardown delta, and world verification support for the new relation
  - Added and updated unit tests across `relations.rs`, `delta.rs`, `world.rs`, and `world_txn.rs`
- Deviations from original plan:
  - Did not add a reverse index such as `declarers_for_office`; the implementation kept a single authoritative forward map to avoid duplicated source-of-truth state
  - Did not add bespoke `RelationDelta` variants; the implementation followed the existing generic `RelationKind` / `RelationValue` architecture
  - Expanded the touched-file scope to include lifecycle and verification plumbing required by the actual canonical relation architecture
- Verification:
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
