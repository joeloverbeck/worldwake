# S44GENCONSUB-008: Perception of contention state

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — perception system in worldwake-systems, belief types in worldwake-core
**Deps**: S44GENCONSUB-007, S44GENCONSUB-010

## Problem

Contention state (who holds the grant, how many are waiting) is authoritative world state but agents cannot yet perceive it. Per FOUNDATIONS P7 (Locality) and Canonical Scenario E, contention artifacts must be inspectable world state visible to co-located agents. Without perception, agents cannot factor observed queue length into their decisions, violating P21 (intentions are revisable commitments — agents should see "someone is already looting that corpse").

## Assumption Reassessment (2026-04-03)

1. Perception system at `crates/worldwake-systems/src/perception.rs:28`. Function `perception_system()`. Confirmed.
2. `observe_passive_local_entities()` at line 194 creates `DirectLocalObservationBatch` for co-located entities, but the actual passive snapshot payload comes from `build_believed_entity_state(...)` via `collect_direct_local_observation_batch()`. The clean extension point is the shared snapshot builder, not a perception-only side channel.
3. `process_witness_event()` also rebuilds `BelievedEntityState` from `ObservedEntitySnapshot::to_believed_entity_state(...)`. If contention state is only added to passive local observation, later direct-witness updates can lawfully overwrite a fresh contention belief back to `None`. The snapshot pipeline must therefore carry contention state too.
4. `BelievedEntityState` at `crates/worldwake-core/src/belief.rs` includes `believed_activity: Option<BelievedActivity>`. Contention state (grant holder, queue length) is conceptually different from activity — a new `BelievedContentionState` field on `BelievedEntityState` is cleaner.
5. `PerceptionProfile` controls observation fidelity. Contention state observation can use the same profile — no special perception component is needed.
6. After `S44GENCONSUB-010`, ground `UniqueItem` pickup is also a live contention domain. This ticket should perceive generalized contention state on any observed entity carrying `ContentionQueue`, not only corpses/patients/facilities.
7. Contention beliefs are subject to staleness per spec Section H.1. The per-field `observed_tick` still matters because contention can change faster than the rest of the entity snapshot.

## Architecture Check

1. Adding contention perception through the existing entity snapshot pipeline is a clean extension — same mechanism as other direct local entity facts, without a second bespoke projection path.
2. New `BelievedContentionState` field on `BelievedEntityState` keeps contention beliefs separate from activity beliefs, avoiding semantic overloading.
3. The clean boundary is generalized local contention perception for any entity with `ContentionQueue`, including the newly live unique-item race domain.
4. No backward-compatibility shims.

## Verification Layers

1. Co-located agent perceives grant on entity → agent's belief store updated → belief state check
2. Co-located agent perceives queue length on contention-managed entity, including unique-item race/grant cases when applicable
3. Remote agent does NOT perceive contention state → belief store unchanged → belief state check
4. Direct witness/event observation does not erase a freshly observable contention belief through a narrower snapshot path
5. Stale contention belief persists until re-observation → belief freshness check
6. Cross-layer: perception system (systems) reads contention state (core), writes beliefs (core). State-mediated (P26).

## What to Change

### 1. Add BelievedContentionState to belief types and snapshot builders

In `crates/worldwake-core/src/belief.rs`:
```rust
pub struct BelievedContentionState {
    pub grant_holder: Option<EntityId>,
    pub queue_length: u32,
    pub observed_tick: Tick,
}
```
Add `pub believed_contention: Option<BelievedContentionState>` to `BelievedEntityState`.

Also extend the shared direct-observation snapshot path so `build_observed_entity_snapshot(...)`, `ObservedEntitySnapshot`, and `ObservedEntitySnapshot::to_believed_entity_state(...)` can preserve contention state instead of dropping it on direct witness/event updates.

### 2. Extend perception to observe generalized contention state
In `crates/worldwake-systems/src/perception.rs`, keep using the existing passive/direct observation flow, but ensure every observed local entity carrying `ContentionQueue` projects grant holder and waiting count into `BelievedContentionState` on the observer's `BelievedEntityState`. This should cover corpse/patient/facility queues and the now-live unique-item race/grant state through the same builder path.

### 3. Update belief serialization and fixture fallout
Ensure `BelievedContentionState` has proper derives (Clone, Debug, Eq, PartialEq, Serialize, Deserialize) and that `BelievedEntityState` / `ObservedEntitySnapshot` serialization includes the new field with `Option` defaulting. Update hand-written `BelievedEntityState` / `ObservedEntitySnapshot` fixture literals that compile against the shared belief shape.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add BelievedContentionState, extend belief/snapshot builders and serialization tests)
- `crates/worldwake-systems/src/perception.rs` (modify — ensure local/direct observation preserves projected contention state; add focused tests)
- shared fixture/test sites constructing `BelievedEntityState` or `ObservedEntitySnapshot` directly (modify as needed)

## Out of Scope

- AI using contention beliefs for planning decisions (future refinement)
- Contention belief in testimony/gossip (future — standard testimony chains apply)
- Golden tests (S44GENCONSUB-009)

## Acceptance Criteria

### Tests That Must Pass

1. Agent at same place as entity with `ContentionGrant` observes `grant_holder` in beliefs
2. Agent at same place as entity with non-empty queue observes `queue_length > 0`
3. Agent at same place as contention-managed unique item can observe race/grant state through the same belief field
4. Agent at different place does NOT have `BelievedContentionState` for remote entity
5. Existing suite: `cargo test --workspace`

### Invariants

1. Contention perception requires co-location (P7 — no global queries)
2. BelievedContentionState may be stale — `observed_tick` tracks freshness
3. Perception system reads contention state but never writes it (P26)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` (tests) — contention state observation for co-located and remote agents, plus no loss through direct observation snapshot updates
2. `crates/worldwake-core/src/belief.rs` (tests) — `BelievedContentionState` / `ObservedEntitySnapshot` serialization and builder projection

### Commands

1. `cargo test -p worldwake-systems perception`
2. `cargo test -p worldwake-core belief`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

**Completed**: 2026-04-04

Implemented generalized contention perception through the shared direct-observation snapshot pipeline instead of only patching `observe_passive_local_entities()`. `BelievedContentionState` was added to `BelievedEntityState`, carried through `ObservedEntitySnapshot`, projected from authoritative `ContentionQueue` state in `build_observed_entity_snapshot(...)` / `build_believed_entity_state(...)`, and preserved by direct witness/event updates via `ObservedEntitySnapshot::to_believed_entity_state(...)`.

The live scope broadened slightly during reassessment because a perception-only patch would have been incomplete: `process_witness_event()` also rebuilds `BelievedEntityState` through the shared snapshot carrier, so the ticket was corrected to the shared projection boundary. The ticket was also updated to include the newly live `UniqueItem` contention domain from `S44GENCONSUB-010`.

Real fallout was broader than the original two-file expectation but still mechanical and bounded: cross-crate fixture and helper literals that construct `BelievedEntityState` or `ObservedEntitySnapshot` directly were updated in core, systems, sim, and AI, and the new shared belief type was re-exported from `worldwake-core` so downstream imports remained coherent.

**Verification**

- `cargo test -p worldwake-core belief`
- `cargo test -p worldwake-systems perception`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
