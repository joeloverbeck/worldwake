# S30-005: Post-load validation for AgentTickDriver

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new post_load_validate() method on AgentTickDriver
**Deps**: S30-004 (restore_runtime_state must exist before validation can be called after it)

## Problem

After deserializing AI runtime state, entity references in the runtime may be stale — agents may have been despawned, items consumed, or places removed between save and the current world state. Derived fields (`dirty`, `semantics_cache`) need explicit initialization. Without post-load validation, the driver may hold references to dead entities, causing panics or incorrect decisions.

## Assumption Reassessment (2026-03-27)

1. `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` — agent keys may reference dead entities after load.
2. `exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>` — `GoalKey` contains `entity: Option<EntityId>` and `place: Option<EntityId>` (`goal.rs:87-91`), either of which may reference dead entities.
3. `materialization_bindings.hypothetical_to_authoritative: BTreeMap<HypotheticalEntityId, EntityId>` — authoritative `EntityId` values may reference dead entities.
4. `World::is_alive(EntityId) -> bool` — confirmed via grep; this is the standard liveness check.
5. `DirtySet::all()` — confirmed to exist from S24 implementation; sets all dirty bits so first tick forces full re-evaluation.
6. The spec mandates that `post_load_validate()` is called after every `restore_runtime_state()` — no code path may skip it.

## Architecture Check

1. Post-load validation is a separate method (not baked into `restore_runtime_state()`) because it requires `&World` which is only available after `SimulationState` is loaded. The restore happens from opaque bytes; validation happens once the world is available.
2. No shims — stale references are pruned, not preserved with workarounds.

## Verification Layers

1. Dead agent pruning → focused test: runtime with entry for dead agent → `post_load_validate()` → entry removed
2. Dead entity in exhaustion cache → focused test: GoalKey with dead entity → entry pruned
3. Dead entity in materialization bindings → focused test: authoritative EntityId dead → entry pruned
4. Dirty set initialization → focused test: after validate, `dirty == DirtySet::all()`
5. Single-layer ticket (AI runtime lifecycle) — no cross-layer mapping needed.

## What to Change

### 1. Add `post_load_validate()` to `AgentTickDriver`

In `crates/worldwake-ai/src/agent_tick/mod.rs`:

```rust
impl AgentTickDriver {
    /// Prune stale entity references and initialize derived state after load.
    /// MUST be called after every `restore_runtime_state()`.
    pub fn post_load_validate(&mut self, world: &World) {
        // 1. Remove runtimes for agents that no longer exist.
        self.runtime_by_agent.retain(|agent, _| world.is_alive(*agent));

        // 2. For each surviving runtime:
        for runtime in self.runtime_by_agent.values_mut() {
            // a. Prune exhaustion_cache entries referencing dead entities.
            runtime.exhaustion_cache.retain(|key, _| {
                key.entity.map_or(true, |e| world.is_alive(e))
                    && key.place.map_or(true, |e| world.is_alive(e))
            });

            // b. Prune materialization_bindings referencing dead entities.
            runtime.materialization_bindings
                .hypothetical_to_authoritative
                .retain(|_, auth| world.is_alive(*auth));

            // c. Set dirty to all-dirty for full re-evaluation on first tick.
            runtime.dirty = DirtySet::all();
        }

        // 3. Clear semantics_cache (rebuilt from ActionDefRegistry on first use).
        self.semantics_cache = None;
    }
}
```

### 2. Ensure golden harness calls post_load_validate after restore

In the save_load_roundtrip flow (updated in S30-004), add `driver.post_load_validate(&world)` after `driver.restore_runtime_state(&bytes)`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — add `post_load_validate` method)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — call `post_load_validate` after restore in roundtrip flow)

## Out of Scope

- `SaveableRuntime` trait or format changes (S30-003)
- Serialization implementation (S30-004)
- Removing the driver reset workaround (S30-006)
- Any behavioral changes to AI decision logic
- Validating `current_plan` step references against world state (plan steps contain `ActionDefId` which is stable; entity targets may be stale but are checked at action start time by the engine)

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: driver with runtime for dead agent → `post_load_validate` → runtime entry removed
2. New focused test: runtime with exhaustion_cache entry whose `GoalKey.entity` is dead → entry pruned after validate
3. New focused test: runtime with exhaustion_cache entry whose `GoalKey.place` is dead → entry pruned
4. New focused test: runtime with materialization binding to dead entity → binding pruned
5. New focused test: after `post_load_validate`, all surviving runtimes have `dirty == DirtySet::all()`
6. New focused test: `semantics_cache` is `None` after validate
7. `cargo test -p worldwake-ai`
8. `cargo clippy --workspace && cargo test --workspace`

### Invariants

1. After `post_load_validate()`, all `EntityId` references in runtime state are guaranteed live
2. After `post_load_validate()`, `dirty == DirtySet::all()` for all agent runtimes — first tick forces full re-evaluation
3. After `post_load_validate()`, `semantics_cache` is `None`
4. `post_load_validate()` is called after every `restore_runtime_state()` — no code path skips it

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/mod.rs` (test module) — `test_post_load_prunes_dead_agents`, `test_post_load_prunes_dead_exhaustion_refs`, `test_post_load_prunes_dead_materialization_bindings`, `test_post_load_sets_dirty_all`
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` — wire `post_load_validate` into roundtrip

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-ai golden`
3. `cargo clippy --workspace && cargo test --workspace`
