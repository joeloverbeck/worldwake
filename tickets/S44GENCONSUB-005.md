# S44GENCONSUB-005: ContentionStatus + Affordance annotation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new field on Affordance struct (worldwake-sim), new enum in worldwake-core
**Deps**: S44GENCONSUB-003

## Problem

The planner currently has no visibility into contention state when evaluating affordances. An agent may plan for a contention-full target with no signal that the resource is contested. Adding a `ContentionStatus` annotation to `Affordance` lets the planner filter or deprioritize contested targets, satisfying FOUNDATIONS P21 (intentions are not entitlements).

## Assumption Reassessment (2026-04-03)

1. `Affordance` struct at `crates/worldwake-sim/src/affordance.rs:7-13` has fields: `def_id`, `actor`, `bound_targets`, `payload_override`, `explanation`. No contention field. Confirmed.
2. `get_affordances()` at `crates/worldwake-sim/src/affordance_query.rs:9` returns `Vec<Affordance>`. All callers must handle the new field. Confirmed.
3. `get_affordances_for_defs()` at line 60 is the filtered variant — also returns `Vec<Affordance>`.
4. Affordance consumers in worldwake-ai read def_id, actor, bound_targets, payload_override. Adding a field with a sensible default (`Unmanaged`) should require minimal consumer changes.
5. `ContentionStatus` is a derived value computed per-query from queue state — not stored. Confirmed in spec Section H.4.

## Architecture Check

1. Adding `contention_status: ContentionStatus` to `Affordance` with default `Unmanaged` is backward-compatible for existing unmanaged affordances — they get the same default value.
2. No backward-compatibility shims — this is a new field, not a wrapper around an old one.

## Verification Layers

1. Affordance for unmanaged entity has `ContentionStatus::Unmanaged` → unit test
2. Affordance for entity with grant held by actor has `Granted` → unit test
3. Affordance for entity with full queue has `Full` → unit test
4. Affordance for entity with available queue has `Available` → unit test
5. Cross-layer: affordance query reads contention state, AI consumes it. Both are read-only on contention state (P26).

## What to Change

### 1. Add ContentionStatus enum to worldwake-core

In `crates/worldwake-core/src/contention.rs`:
```rust
pub enum ContentionStatus {
    Unmanaged,
    Granted,
    Queued { position: u32 },
    Available,
    Full,
}
```
Derive: Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize. Implement Default as `Unmanaged`.

### 2. Add contention_status field to Affordance

In `crates/worldwake-sim/src/affordance.rs`, add `pub contention_status: ContentionStatus` to `Affordance`. Update all construction sites to include the field (default `Unmanaged` for non-contention paths).

### 3. Compute contention status in get_affordances()

In `crates/worldwake-sim/src/affordance_query.rs`, after enumerating affordances, check if the target entity has a `ContentionQueue`. If so, compute `ContentionStatus` based on: actor holds grant → `Granted`; actor in queue → `Queued { position }`; queue not full → `Available`; queue full → `Full`.

### 4. Update Affordance consumers

Update any code in worldwake-ai that constructs or pattern-matches on `Affordance` to include the new field.

## Files to Touch

- `crates/worldwake-core/src/contention.rs` (modify — add ContentionStatus)
- `crates/worldwake-core/src/lib.rs` (modify — add re-export)
- `crates/worldwake-sim/src/affordance.rs` (modify — add field)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — compute status)
- `crates/worldwake-ai/src/*.rs` (modify — update Affordance construction/matching as needed)

## Out of Scope

- Planner feasibility scoring based on ContentionStatus (future refinement)
- Action validation gating (S44GENCONSUB-006)

## Acceptance Criteria

### Tests That Must Pass

1. Affordance for entity without ContentionQueue has `Unmanaged` status
2. Affordance for entity where actor holds grant has `Granted` status
3. Affordance for entity with full queue (max_waiters reached) has `Full` status
4. Existing suite: `cargo test --workspace`

### Invariants

1. `ContentionStatus` is always derived from current queue state, never stored
2. Default `Unmanaged` for all non-contention-managed entities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/affordance_query.rs` (tests) — contention status computation
2. `crates/worldwake-core/src/contention.rs` (tests) — ContentionStatus bounds and serialization

### Commands

1. `cargo test -p worldwake-sim affordance`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
