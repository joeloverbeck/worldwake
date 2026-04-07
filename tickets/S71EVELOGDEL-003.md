# S71EVELOGDEL-003: Wire compact diff into `WorldTxn::replace_simple_component`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — delta emission path in `worldwake-core` world transaction
**Deps**: S71EVELOGDEL-001, S71EVELOGDEL-002

## Problem

`replace_simple_component` at `world_txn.rs:1016` unconditionally emits `ComponentDelta::Set` with full before/after `ComponentValue` snapshots. For `AgentBeliefStore` this means ~300 KB per perception event per agent. This ticket wires the compact diff path so that belief-store component sets emit `ComponentDelta::CompactSet` with a `BeliefStoreDiff` instead.

## Assumption Reassessment (2026-04-08)

1. `replace_simple_component` at `world_txn.rs:1016-1049` is a generic function parameterized by `Get`, `Remove`, `Insert`, `Wrap` closures plus a `ComponentKind` discriminant. Delta is pushed at line 1042-1047.
2. The function receives `component_kind: ComponentKind` as a parameter. Branching on `ComponentKind::AgentBeliefStore` to choose the compact path is straightforward.
3. The `wrap` closure produces `ComponentValue` from the raw component. For the compact path, we need the raw `AgentBeliefStore` values (before and after) to compute the diff. The `before` value is obtained via `get()` at line 1033 as type `T` (generic). The compact path needs to downcast or specialize for `AgentBeliefStore`.
4. The function signature is generic over `T`. The compact diff branch can be implemented by adding a specialized code path for `set_component_agent_belief_store` (the public caller) rather than modifying the generic `replace_simple_component` itself. Alternatively, the generic function can check `component_kind` and compute the diff from the `before`/`after` `T` values when `T` is `AgentBeliefStore`.
5. `set_component_agent_belief_store` is the public method that calls `replace_simple_component` with `ComponentKind::AgentBeliefStore`. This is the natural specialization point.
6. Not a planner/golden-driven ticket.
14. No mismatches found.

## Architecture Check

1. Specializing at the `set_component_agent_belief_store` call site (or within `replace_simple_component` via a `component_kind` check) is cleaner than adding a generic diff trait, because only one component type needs compact diffs initially. A generic trait can be introduced later if more components need it (YAGNI).
2. No backward-compatibility shims. The `Set` variant is still used for all other component kinds. `CompactSet` replaces `Set` for belief stores only.

## Verification Layers

1. Belief store sets emit `CompactSet` instead of `Set` -> focused unit test constructing a `WorldTxn`, setting a belief store, and inspecting the emitted delta
2. Non-belief-store sets still emit `Set` -> focused unit test confirming other component kinds are unchanged
3. Single-layer ticket (authoritative delta emission within `worldwake-core`); downstream consumer handling is in tickets 004, 005.

## What to Change

### 1. Specialize belief-store delta emission

In the `set_component_agent_belief_store` method (or within `replace_simple_component` when `component_kind == ComponentKind::AgentBeliefStore`):

- When `before` is `Some`, compute `BeliefStoreDiff::compute(&before, &after)` and emit `ComponentDelta::CompactSet { entity, component_kind, diff: ComponentDiff::BeliefStore(diff) }`.
- When `before` is `None` (first-time set), emit the standard `ComponentDelta::Set` since there is no base state to diff against (or emit a `CompactSet` with a diff that represents "add everything" — implementation choice, but first-time sets are rare and small).

### 2. Preserve the equality short-circuit

The existing early return at line 1034 (`if before.as_ref() == Some(&component) { return Ok(()); }`) must remain. If `before == after`, no delta is emitted regardless of component kind.

## Files to Touch

- `crates/worldwake-core/src/world_txn.rs` (modify — specialize delta emission for `AgentBeliefStore`)

## Out of Scope

- Updating verification reconstruction (ticket 004)
- Updating CLI display or other consumers (ticket 005)
- Compact diffs for non-belief-store components
- Changing the equality short-circuit behavior

## Acceptance Criteria

### Tests That Must Pass

1. Setting an `AgentBeliefStore` on a `WorldTxn` emits `ComponentDelta::CompactSet` (not `Set`) when `before` is `Some`
2. Setting a non-belief-store component still emits `ComponentDelta::Set`
3. Setting an identical belief store emits no delta (equality short-circuit preserved)
4. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `ComponentDelta::CompactSet` is emitted only for `ComponentKind::AgentBeliefStore`
2. The equality short-circuit (`before == after` -> no delta) is preserved for all component kinds
3. First-time sets (no previous value) still produce a valid delta

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world_txn.rs` — focused test: set belief store on entity, inspect emitted `StateDelta` to confirm `CompactSet` variant
2. `crates/worldwake-core/src/world_txn.rs` — focused test: set non-belief-store component, confirm `Set` variant
3. `crates/worldwake-core/src/world_txn.rs` — focused test: set identical belief store, confirm no delta emitted

### Commands

1. `cargo test -p worldwake-core world_txn` (targeted)
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace --all-targets -- -D warnings`
