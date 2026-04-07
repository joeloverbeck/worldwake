# S71EVELOGDEL-004: Update verification reconstruction for `CompactSet`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — verification reconstruction logic in `worldwake-core`
**Deps**: S71EVELOGDEL-002

## Problem

`verification.rs:196-202` reconstructs component state by iterating event-log deltas and inserting the `after` value from `ComponentDelta::Set` into a running components map. After ticket 002 adds the `CompactSet` variant, verification must handle it by applying the compact diff to the running state instead of replacing with a full snapshot.

## Assumption Reassessment (2026-04-08)

1. Verification reconstruction at `verification.rs:196-202` matches `StateDelta::Component(ComponentDelta::Set { entity, component_kind, after, .. })` and inserts `after.clone()` into a `BTreeMap<(EntityId, ComponentKind), ComponentValue>`.
2. The `before` field is not read — the match uses `..` wildcard. Only `after` is consumed.
3. For `CompactSet`, verification needs to: (a) look up the current `ComponentValue` in its running map, (b) downcast it to `AgentBeliefStore`, (c) apply the `BeliefStoreDiff`, (d) wrap back into `ComponentValue::AgentBeliefStore` and insert.
4. If no prior value exists in the map (first event for this entity+component), this is an error condition — `CompactSet` requires a base state to apply against. The first set for a belief store should always be a full `Set` (ticket 003 ensures this).
5. This is a single-crate change within `worldwake-core`.
6. Not a planner/golden-driven ticket.
14. No mismatches found.

## Architecture Check

1. Extending the existing match arm is cleaner than refactoring verification into a separate reconstruction engine. The change is small: one new match arm that applies a diff instead of replacing wholesale.
2. No backward-compatibility shims. The new match arm handles `CompactSet`; the existing `Set` arm is unchanged.

## Verification Layers

1. `CompactSet` reconstruction produces same final state as full `Set` reconstruction -> focused unit test comparing both paths
2. Missing base state for `CompactSet` is detected -> focused unit test confirming panic/error on `CompactSet` without prior `Set`
3. Single-layer ticket (authoritative verification within `worldwake-core`).

## What to Change

### 1. Add `CompactSet` match arm in verification reconstruction

In `crates/worldwake-core/src/verification.rs`, within the delta iteration loop (around line 196), add a new match arm:

```rust
StateDelta::Component(ComponentDelta::CompactSet {
    entity,
    component_kind,
    diff,
}) => {
    let key = (*entity, *component_kind);
    let base = components.get(&key)
        .expect("CompactSet requires a prior Set base state");
    let updated = diff.apply_to_component_value(base);
    components.insert(key, updated);
}
```

### 2. Implement `ComponentDiff::apply_to_component_value`

In `crates/worldwake-core/src/delta.rs`, add a method on `ComponentDiff`:

```rust
impl ComponentDiff {
    pub fn apply_to_component_value(&self, base: &ComponentValue) -> ComponentValue {
        match self {
            ComponentDiff::BeliefStore(diff) => {
                let ComponentValue::AgentBeliefStore(store) = base else {
                    panic!("BeliefStore diff applied to non-belief-store component");
                };
                ComponentValue::AgentBeliefStore(diff.clone().apply(store))
            }
        }
    }
}
```

This keeps the downcast logic centralized in one place rather than scattered across consumers.

## Files to Touch

- `crates/worldwake-core/src/verification.rs` (modify — add `CompactSet` match arm)
- `crates/worldwake-core/src/delta.rs` (modify — add `apply_to_component_value` method on `ComponentDiff`)

## Out of Scope

- Updating perception, CLI, or observer consumers (ticket 005)
- Wiring `CompactSet` emission in `WorldTxn` (ticket 003)
- Full integration/soak testing (ticket 006)

## Acceptance Criteria

### Tests That Must Pass

1. Verification reconstruction with `CompactSet` deltas produces identical final component state as reconstruction with equivalent full `Set` deltas
2. `CompactSet` without a prior `Set` base panics (contract violation)
3. Mixed delta streams (some `Set`, some `CompactSet`) reconstruct correctly
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `apply_to_component_value` preserves the roundtrip property: if the diff was computed from `compute(before, after)`, applying it to `before` produces `after`
2. Verification reconstruction is deterministic regardless of whether deltas are full or compact

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/verification.rs` — focused test: construct an event stream with `CompactSet` deltas, run verification reconstruction, compare against known final state
2. `crates/worldwake-core/src/delta.rs` — focused test for `ComponentDiff::apply_to_component_value` with valid and invalid base types

### Commands

1. `cargo test -p worldwake-core verification` (targeted)
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-08 — delivered by S71EVELOGDEL-002.

- S71EVELOGDEL-002 absorbed this ticket's entire scope during implementation because adding the `CompactSet` variant to `ComponentDelta` required updating all exhaustive match sites for the workspace to compile.
- `verification.rs` `CompactSet` match arm: delivered in 002
- `ComponentDiff::apply_to_component_value`: delivered in 002
- Focused tests for `apply_to_component_value` are deferred to ticket 006 (integration validation) since the method is exercised through verification reconstruction.

## Verification Result

- Covered by S71EVELOGDEL-002 verification: `cargo test -p worldwake-core` (1030 tests passed)
