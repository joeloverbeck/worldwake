# S72EVELOGEPO-002: Scenario config and compaction SystemFn

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new SystemFn for event log compaction; ScenarioDef gains compaction_interval
**Deps**: S72EVELOGEPO-001

## Problem

The checkpoint infrastructure from ticket 001 exists but is never invoked. This ticket wires the `compaction_interval` from scenario configuration to `EventLog`, implements the compaction `SystemFn` that periodically creates checkpoints and strips old deltas, and registers it in the tick dispatch pipeline.

## Assumption Reassessment (2026-04-08)

1. `ScenarioDef` at `crates/worldwake-cli/src/scenario/types.rs:22` has `pub seed: u64` and other fields. Adding `pub compaction_interval: u32` with `#[serde(default = "default_compaction_interval")]` is additive. Existing RON files will get the default (50) via serde.
2. `assemble_state()` in `crates/worldwake-cli/src/scenario/mod.rs` creates the `EventLog` and `SimulationState`. The `seed` is threaded from `ScenarioDef` through `seed_from_u64()` at line 440 to `DeterministicRng::new()` at line 441. The `compaction_interval` follows the same pattern: read from `def.compaction_interval`, call `event_log.set_compaction_interval(def.compaction_interval)` before constructing `SimulationState`.
3. `SystemFn` type at `crates/worldwake-sim/src/system_dispatch.rs:11` is `fn(SystemExecutionContext<'_>) -> Result<(), SystemError>`. `SystemExecutionContext` provides `world: &'a mut World`, `event_log: &'a mut EventLog`, `tick: Tick` — all needed by the compaction function.
4. `SystemDispatchTable` at `system_dispatch.rs:47-66` maps `SystemId` to `SystemFn`. Registration requires a new `SystemId` variant. `SystemId` at `crates/worldwake-sim/src/system_id.rs` (or equivalent) needs a `Compaction` variant.
5. Tick orchestration in `crates/worldwake-sim/src/tick_step.rs` defines the order systems run. The compaction SystemFn must run AFTER all other systems complete — it's a bookkeeping operation that should not interfere with game logic.
6. The compaction function needs `bincode::serialize(ctx.world)` — worldwake-sim already depends on `bincode` (check `Cargo.toml`). If not, the dependency must be added.
7. Cross-system ticket: worldwake-cli (config) -> worldwake-core (EventLog methods) -> worldwake-sim (SystemFn). The shared boundary is `EventLog`'s public API from ticket 001.

## Architecture Check

1. The compaction SystemFn uses the standard `SystemExecutionContext` interface — no special-case plumbing. It reads `event_log.compaction_interval()`, checks tick modulo, and calls `event_log.add_checkpoint()` / `event_log.strip_deltas_before()`. Clean integration with existing tick infrastructure.
2. No backward-compatibility shims. The `#[serde(default)]` on `ScenarioDef.compaction_interval` means all existing RON scenarios continue to work. The default value (50) enables compaction automatically for new scenarios.

## Verification Layers

1. Compaction triggers at correct ticks -> focused unit test (mock EventLog, verify add_checkpoint called at tick % interval == 0)
2. Compaction disabled when interval is 0 -> focused unit test
3. ScenarioDef deserialization with and without compaction_interval -> focused unit test
4. End-to-end compaction (events stripped after checkpoint) -> integration test using Harness
5. Cross-system boundary: ScenarioDef.compaction_interval reaches EventLog -> integration test verifying `event_log.compaction_interval()` matches scenario value after `assemble_state()`

## What to Change

### 1. Add `compaction_interval` to `ScenarioDef`

In `crates/worldwake-cli/src/scenario/types.rs`:

```rust
pub struct ScenarioDef {
    pub seed: u64,
    pub places: Vec<PlaceDef>,
    #[serde(default)]
    pub edges: Vec<EdgeDef>,
    #[serde(default)]
    pub agents: Vec<AgentDef>,
    #[serde(default)]
    pub items: Vec<ItemDef>,
    #[serde(default)]
    pub facilities: Vec<FacilityDef>,
    #[serde(default)]
    pub resource_sources: Vec<ResourceSourceDef>,
    /// Ticks between checkpoint snapshots. Default: 50. Set to 0 to disable.
    #[serde(default = "default_compaction_interval")]
    pub compaction_interval: u32,
}

fn default_compaction_interval() -> u32 { 50 }
```

### 2. Wire compaction_interval in `assemble_state()`

In `crates/worldwake-cli/src/scenario/mod.rs`, after creating the `EventLog` and before constructing `SimulationState`:

```rust
event_log.set_compaction_interval(def.compaction_interval);
```

### 3. Add `SystemId::Compaction` variant

In the file defining `SystemId` (likely `crates/worldwake-sim/src/system_id.rs` or equivalent), add a `Compaction` variant to the enum.

### 4. Implement compaction SystemFn

Create a new function (in `crates/worldwake-sim/src/` — either a new `compaction.rs` module or within an existing infrastructure module):

```rust
pub fn compact_event_log(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let interval = ctx.event_log.compaction_interval();
    if interval == 0 || ctx.tick.0 % u64::from(interval) != 0 {
        return Ok(());
    }

    // Serialize current World as checkpoint
    let snapshot = bincode::serialize(ctx.world)
        .expect("World serialization must not fail — all types are Serialize");
    ctx.event_log.add_checkpoint(ctx.tick, CheckpointData::new(snapshot));

    // Strip state_deltas from events older than this checkpoint
    ctx.event_log.strip_deltas_before(ctx.tick);

    // Drop oldest checkpoint if we now have > 2
    ctx.event_log.prune_old_checkpoints(2);

    Ok(())
}
```

### 5. Register in SystemDispatchTable and tick ordering

In `crates/worldwake-sim/src/system_dispatch.rs`, register `SystemId::Compaction -> compact_event_log`.

In `crates/worldwake-sim/src/tick_step.rs`, schedule `SystemId::Compaction` to run after all other systems complete. It must be the last system in the tick — it's a bookkeeping operation that should not interfere with any game system's view of `state_deltas` during the current tick.

### 6. Verify bincode dependency

Check `crates/worldwake-sim/Cargo.toml` for `bincode`. If missing, add it. worldwake-core already depends on bincode (used for canonical hashing).

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — add `compaction_interval` field and default fn)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — wire `compaction_interval` to EventLog in `assemble_state()`)
- `crates/worldwake-sim/src/system_dispatch.rs` (modify — register compaction SystemFn)
- `crates/worldwake-sim/src/tick_step.rs` (modify — schedule compaction as last system)
- `crates/worldwake-sim/src/system_id.rs` or equivalent (modify — add `Compaction` variant)
- `crates/worldwake-sim/src/compaction.rs` (new — compaction SystemFn implementation)
- `crates/worldwake-sim/src/lib.rs` (modify — declare `compaction` module)
- `crates/worldwake-sim/Cargo.toml` (modify — add `bincode` dependency if missing)

## Out of Scope

- Verification adaptation (ticket 003)
- Disk-backed storage
- Delta compaction by merging diffs
- Changing event emission frequency
- Modifying any existing SystemFn behavior

## Acceptance Criteria

### Tests That Must Pass

1. Compaction SystemFn creates checkpoint at tick 50 (with interval=50) and strips events before tick 50
2. Compaction SystemFn is a no-op at tick 49 (with interval=50)
3. Compaction SystemFn is a no-op when interval=0
4. `ScenarioDef` deserializes with default compaction_interval=50 when field is absent from RON
5. `ScenarioDef` deserializes with compaction_interval=0 when explicitly set
6. Existing suite: `cargo test --workspace`

### Invariants

1. Compaction never runs during game-logic systems — it is always the last system in the tick
2. Events at the current tick are never stripped (compaction strips events BEFORE the checkpoint tick)
3. Existing RON scenarios continue to deserialize without modification
4. `compact_event_log` is deterministic — same tick, same world state, same checkpoint bytes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/compaction.rs` (tests module) — compaction_triggers_at_interval, compaction_noop_at_non_interval, compaction_disabled_when_zero
2. `crates/worldwake-cli/src/scenario/types.rs` (or tests) — scenario_def_default_compaction_interval, scenario_def_explicit_zero

### Commands

1. `cargo test -p worldwake-sim` (targeted)
2. `cargo test -p worldwake-cli` (targeted)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
