# S71EVELOGDEL-002: Add `CompactSet` variant to `ComponentDelta`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new variant on `ComponentDelta` enum in `worldwake-core`
**Deps**: S71EVELOGDEL-001

## Problem

To store compact structural diffs instead of full snapshots, `ComponentDelta` needs a new `CompactSet` variant that carries a `ComponentDiff` enum (initially wrapping `BeliefStoreDiff`). This variant must satisfy the same derive bounds as the existing enum and serialize correctly with bincode.

## Assumption Reassessment (2026-04-08)

1. `ComponentDelta` defined at `crates/worldwake-core/src/delta.rs:180` with derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` and `#[allow(clippy::large_enum_variant)]`.
2. `StateDelta` wraps `ComponentDelta` at `delta.rs:228`. Adding a variant to `ComponentDelta` does not require changes to `StateDelta`.
3. Exhaustive match sites on `ComponentDelta::Set`: `verification.rs:196`, `perception.rs:1049`, `world_txn.rs` (construction only), `event_record.rs` tests, `delta.rs` tests. All use `..` wildcard or `_ =>` catch-all except verification which destructures `Set` explicitly.
4. `BeliefStoreDiff` will exist after S71EVELOGDEL-001 at `belief.rs` with matching derive bounds.
5. This ticket only adds the variant and wrapper enum. No match sites are updated yet (that's tickets 004, 005). The workspace will compile because existing matches either use `_ =>` wildcards or will be updated in the same PR/branch.
6. Not a planner/golden-driven ticket.
14. No mismatches found.

## Architecture Check

1. A `ComponentDiff` enum (with initially one variant `BeliefStore(BeliefStoreDiff)`) is cleaner than putting `BeliefStoreDiff` directly into `CompactSet`, because it provides a natural extension point when other component types get compact diffs later. The wrapper adds one level of indirection but keeps `ComponentDelta` agnostic to specific component diff types.
2. No backward-compatibility shims. Old `Set` variant remains for components that don't have custom diff logic. FND-28 is satisfied because both representations coexist by design (not as a compat layer) — `Set` is the default, `CompactSet` is used where diff logic exists.

## Verification Layers

1. New variant satisfies same trait bounds as existing variants -> compile-time verification (derives)
2. Serialization roundtrip -> focused unit test (bincode encode/decode)
3. Single-layer ticket (type definition + serialization test within `worldwake-core`); match-site updates are in subsequent tickets.

## What to Change

### 1. Define `ComponentDiff` enum

In `crates/worldwake-core/src/delta.rs`, add:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComponentDiff {
    BeliefStore(BeliefStoreDiff),
}
```

### 2. Add `CompactSet` variant to `ComponentDelta`

```rust
pub enum ComponentDelta {
    Set {
        entity: EntityId,
        component_kind: ComponentKind,
        before: Option<ComponentValue>,
        after: ComponentValue,
    },
    CompactSet {
        entity: EntityId,
        component_kind: ComponentKind,
        diff: ComponentDiff,
    },
    Removed {
        entity: EntityId,
        component_kind: ComponentKind,
        before: ComponentValue,
    },
}
```

### 3. Add accessor methods on `ComponentDelta`

Add `entity()` and `component_kind()` methods that work across all variants, if they don't already exist. This avoids duplicating match logic in every consumer.

### 4. Export new types

Ensure `ComponentDiff` and `BeliefStoreDiff` are re-exported from the crate root or the delta module's public API so downstream crates can reference them.

## Files to Touch

- `crates/worldwake-core/src/delta.rs` (modify — add `ComponentDiff` enum, `CompactSet` variant, accessor methods)
- `crates/worldwake-core/src/lib.rs` (modify — re-export `ComponentDiff` if needed)

## Out of Scope

- Emitting `CompactSet` from `WorldTxn` (ticket 003)
- Updating match sites in verification, perception, CLI (tickets 004, 005)
- Diff types for non-belief-store components

## Acceptance Criteria

### Tests That Must Pass

1. `ComponentDelta::CompactSet` serialization roundtrip: bincode encode then decode produces equal value
2. `ComponentDiff::BeliefStore` serialization roundtrip
3. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `ComponentDelta` continues to satisfy `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
2. Adding `CompactSet` does not change the serialized representation of existing `Set` and `Removed` variants (bincode uses variant index; new variant appended after existing ones preserves existing indices)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/delta.rs` — serialization roundtrip test for `ComponentDelta::CompactSet` with a sample `BeliefStoreDiff`
2. `crates/worldwake-core/src/delta.rs` — verify existing `ComponentDelta::Set` serialization is unchanged (regression guard)

### Commands

1. `cargo test -p worldwake-core delta` (targeted)
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
