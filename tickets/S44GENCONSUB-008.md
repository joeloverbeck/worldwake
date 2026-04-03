# S44GENCONSUB-008: Perception of contention state

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — perception system in worldwake-systems, belief types in worldwake-core
**Deps**: S44GENCONSUB-007

## Problem

Contention state (who holds the grant, how many are waiting) is authoritative world state but agents cannot yet perceive it. Per FOUNDATIONS P7 (Locality) and Canonical Scenario E, contention artifacts must be inspectable world state visible to co-located agents. Without perception, agents cannot factor observed queue length into their decisions, violating P21 (intentions are revisable commitments — agents should see "someone is already looting that corpse").

## Assumption Reassessment (2026-04-03)

1. Perception system at `crates/worldwake-systems/src/perception.rs:28`. Function `perception_system()`. Confirmed.
2. `observe_passive_local_entities()` at line 194 creates `DirectLocalObservationBatch` for co-located entities. This is where contention state observation should be added.
3. `BelievedEntityState` at `crates/worldwake-core/src/belief.rs` includes `believed_activity: Option<BelievedActivity>` (line 698). Contention state can extend this or use a new field.
4. `BelievedActivity` tracks `action_domain`, `target`, `observed_tick`. Contention state (grant holder, queue length) is conceptually different from activity — a new `BelievedContentionState` field on `BelievedEntityState` is cleaner.
5. `PerceptionProfile` controls observation fidelity. Contention state observation can use the same profile — no special perception needed.
6. Contention beliefs are subject to staleness per spec Section H.1. The `observed_tick` on `BelievedEntityState` handles this.

## Architecture Check

1. Adding contention perception through the existing `observe_passive_local_entities()` pathway is a clean extension — same mechanism as activity observation.
2. New `BelievedContentionState` field on `BelievedEntityState` keeps contention beliefs separate from activity beliefs, avoiding semantic overloading.
3. No backward-compatibility shims.

## Verification Layers

1. Co-located agent perceives grant on entity → agent's belief store updated → belief state check
2. Remote agent does NOT perceive contention state → belief store unchanged → belief state check
3. Stale contention belief persists until re-observation → belief freshness check
4. Cross-layer: perception system (systems) reads contention state (core), writes beliefs (core). State-mediated (P26).

## What to Change

### 1. Add BelievedContentionState to belief types

In `crates/worldwake-core/src/belief.rs`:
```rust
pub struct BelievedContentionState {
    pub grant_holder: Option<EntityId>,
    pub queue_length: u32,
    pub observed_tick: Tick,
}
```
Add `pub believed_contention: Option<BelievedContentionState>` to `BelievedEntityState`.

### 2. Extend perception to observe contention

In `crates/worldwake-systems/src/perception.rs`, within `observe_passive_local_entities()`: for each observed entity that has a `ContentionQueue`, project the grant holder and waiting count into `BelievedContentionState` on the observer's `BelievedEntityState` for that entity.

### 3. Update belief serialization

Ensure `BelievedContentionState` has proper derives (Clone, Debug, Eq, PartialEq, Serialize, Deserialize) and that `BelievedEntityState` serialization includes the new field with backward-compatible `Option` defaulting.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add BelievedContentionState, add field to BelievedEntityState)
- `crates/worldwake-systems/src/perception.rs` (modify — observe contention state)

## Out of Scope

- AI using contention beliefs for planning decisions (future refinement)
- Contention belief in testimony/gossip (future — standard testimony chains apply)
- Golden tests (S44GENCONSUB-009)

## Acceptance Criteria

### Tests That Must Pass

1. Agent at same place as entity with ContentionGrant observes grant_holder in beliefs
2. Agent at same place as entity with non-empty queue observes queue_length > 0
3. Agent at different place does NOT have BelievedContentionState for remote entity
4. Existing suite: `cargo test --workspace`

### Invariants

1. Contention perception requires co-location (P7 — no global queries)
2. BelievedContentionState may be stale — `observed_tick` tracks freshness
3. Perception system reads contention state but never writes it (P26)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` (tests) — contention state observation for co-located and remote agents
2. `crates/worldwake-core/src/belief.rs` (tests) — BelievedContentionState serialization

### Commands

1. `cargo test -p worldwake-systems perception`
2. `cargo test -p worldwake-core belief`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
