# S72: Event Log Epoch Compaction

## Summary

S71 reduced per-event memory from ~300 KB to ~1-5 KB by storing structural diffs instead of full component snapshots. However, the event log (`Vec<EventRecord>`) remains strictly append-only with no mechanism to reclaim memory from old events. Even at 1-5 KB per delta, unbounded tick counts produce unbounded RAM growth — unacceptable for a simulation that will run indefinitely as a game backbone.

This spec introduces periodic **checkpoint snapshots** of the `World` state, after which the heavy `state_deltas` payloads on older events are stripped from memory. Event metadata (who, what, when, where, causal chain, tags, evidence, observed entities) is preserved indefinitely. Full world state at any historical tick remains reconstructible from the nearest checkpoint plus subsequent deltas.

## Phase

Phase 7: Consequence Carriers

## Status

COMPLETED

## Motivation

After S71, per-event size dropped from ~150 KB to ~1-5 KB average. But the growth is still linear and unbounded:

| Ticks | Estimated Events | Estimated state_deltas RAM | Event Metadata RAM |
|-------|-----------------|---------------------------|-------------------|
| 300 | ~20,000 | ~60 MB | ~6 MB |
| 3,000 | ~200,000 | ~600 MB | ~60 MB |
| 30,000 | ~2,000,000 | ~6 GB | ~600 MB |

The `state_deltas` column dominates. Event metadata (all other fields) grows at ~300 bytes/event — manageable for millions of events. The `state_deltas` payload is the only field that needs compaction.

Runtime analysis of `state_deltas` consumers:

- **Perception** (`perception.rs:999, 1034`) reads `state_deltas` to extract institutional claims and force control claims from Political events. However, perception only processes current-tick events via `events_at_tick(tick)` (perception.rs:47). Compaction only strips events **before** the checkpoint tick, so perception never encounters stripped deltas.
- **Verification** (`verification.rs:174`, `#[cfg(test)]`) replays all `state_deltas` to reconstruct world state. This is test-only infrastructure.
- **CLI display** (`handlers/events.rs:177-182`) prints delta counts and details. After compaction, old events show "deltas: (none)" — benign.
- **Observer binary** (`observer.rs:600, 623`) logs delta counts per event.

No runtime system reads `state_deltas` from events older than the current tick. Stripping old deltas is invisible to all simulation systems.

## Crates

- `worldwake-core` (EventLog, EventRecord, EventPayload, checkpoint storage, stripping logic)
- `worldwake-sim` (SystemFn registration in `system_dispatch.rs`, tick orchestration in `tick_step.rs`)
- `worldwake-cli` (ScenarioDef — adds `compaction_interval` field, passes it to EventLog during `assemble_state()`)

## Dependencies

S71 (event log delta compaction) — COMPLETED. S72 builds on the compact diff representation introduced by S71.

## Design Goals

- **Bounded RSS for state_deltas** — memory consumed by component deltas is capped at O(compaction_interval * events_per_tick * avg_delta_size), regardless of total tick count
- **Preserve event metadata** — all non-delta event fields persist for the full simulation lifetime (FND-29A)
- **Preserve causal reconstructibility** — world state at any historical tick can be reconstructed from the nearest prior checkpoint plus subsequent deltas (FND-12)
- **Preserve runtime correctness** — no production system reads `state_deltas` from old events, so stripping has zero runtime impact
- **Deterministic** — checkpoint creation and stripping are deterministic functions of tick number

## Non-Goals

- Disk-backed event storage (future extension, not needed now)
- Compacting event metadata (small enough to remain in RAM indefinitely)
- Reducing event emission rate (orthogonal concern)
- Delta compaction by merging adjacent diffs (adds algebraic complexity without clear need)
- Changing the append-only invariant of the event log (FND-29A is non-negotiable)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-29A (Causal History Is Append-Only) | **Core enabler.** FND-29A explicitly allows events to be "summarized, indexed, or compacted for storage" while mandating that "the authoritative history must behave as append-only" and that the fact an event occurred is never erased. This spec compacts storage (strips heavy delta payloads) while preserving every event record and its metadata. No event is deleted. |
| FND-12 (Performance Compresses Computation, Never Causality) | Checkpoints and stripping change how world state is stored, not what the world means. Any historical world state remains derivable by replaying from the nearest checkpoint. The causal chain is unmodified. The checkpoint serializes the entire `World` struct (entities, components, relations, topology) to ensure complete reconstructibility of everything encoded in state_deltas. |
| FND-27 (Derived Summaries Are Caches) | Checkpoints are derived state — recomputable from initial world state plus the full event stream (before any stripping). After stripping, the checkpoint becomes the reconstruction base, but it is not authoritative truth; it is a performance cache that enables bounded memory. |
| FND-3 (Concrete State Over Abstract Scores) | No abstract scores introduced. Checkpoints store concrete `World` state. |
| FND-28 (No Backward Compatibility) | Clean implementation. No shims or wrappers for pre-compaction event formats. |

## Information-Path Analysis

The compaction system introduces no new information paths. It operates purely on the event log's internal storage:

1. **Checkpoint creation**: reads the authoritative `World` state at tick T, serializes it via bincode, stores it on the `EventLog`. No agent, system, or perception path is involved.
2. **Delta stripping**: empties the `state_deltas` field on events with tick < T. Since no production runtime system reads `state_deltas` from old events (perception only reads current-tick events at `perception.rs:47`), this is invisible to all information paths.
3. **Reconstruction** (test/debug only): reads the nearest checkpoint, deserializes the `World`, then replays `state_deltas` from checkpoint tick forward. This path exists only in test infrastructure and CLI debugging tools.

No agent information path is affected. Perception, beliefs, AI planning, and all social information flows are unchanged.

## Positive-Feedback Analysis

No amplifying loops. Checkpoint creation and delta stripping are fixed-interval operations triggered by tick count modulo K. They do not affect event emission rates, belief update frequency, or any other simulation dynamic.

## Concrete Dampeners

N/A — no positive feedback loops introduced.

## Stored State vs. Derived Read-Model

| Item | Classification |
|------|---------------|
| Event metadata (tick, cause, actor, targets, evidence, observed_entities, visibility, witness_data, tags) | Authoritative stored state. Preserved indefinitely. |
| `state_deltas` on recent events (tick >= last checkpoint) | Authoritative stored state. Active window, not yet compacted. |
| `state_deltas` on old events (tick < last checkpoint) | **Stripped after compaction.** Was authoritative, now derivable from checkpoint + forward replay. The causal fact "component X changed" is still recorded in the event metadata; the exact delta payload is recoverable from the checkpoint. |
| Checkpoint `World` snapshots | Derived state (cache). Recomputable from initial world state + full event replay. Stored for performance — enables bounded memory by making old deltas redundant. |

## Design

### 1. Checkpoint Storage

Add checkpoint infrastructure to `EventLog` (`event_log.rs`):

```rust
/// Serialized World snapshot at a specific tick.
pub struct CheckpointData {
    /// Bincode-serialized World state.
    world_snapshot: Vec<u8>,
}

pub struct EventLog {
    events: Vec<EventRecord>,
    // ... existing indices ...

    /// Periodic World snapshots. Key: tick.
    /// At most 2 retained (current + previous for safety).
    checkpoints: BTreeMap<Tick, CheckpointData>,

    /// Ticks between checkpoint snapshots. 0 = disabled.
    compaction_interval: u32,
}
```

The `checkpoints` BTreeMap stores at most 2 entries. After a new checkpoint is created and stripping against it succeeds, the oldest checkpoint (if 3 exist) is removed.

The `compaction_interval` field is set during initialization via `EventLog::set_compaction_interval()`, called from `assemble_state()` in `worldwake-cli`'s scenario module. This keeps the configuration path clean: `ScenarioDef.compaction_interval` → `assemble_state()` → `EventLog::set_compaction_interval()`.

The `World` struct (`world.rs:119`) already derives `Serialize, Deserialize`, making bincode serialization straightforward. Serializing the entire `World` (entity allocator, component tables, relation tables, topology) ensures complete reconstruction of everything encoded in `state_deltas` — entity creation/archival, component sets/removals, relation adds/removes, and reservations.

### 2. Compaction Interval Configuration

Add an optional `compaction_interval` field to `ScenarioDef` (`scenario/types.rs`):

```rust
pub struct ScenarioDef {
    pub seed: u64,
    // ... existing fields ...
    /// Ticks between checkpoint snapshots. Default: 50.
    /// Set to 0 to disable compaction.
    #[serde(default = "default_compaction_interval")]
    pub compaction_interval: u32,
}

fn default_compaction_interval() -> u32 { 50 }
```

This is an operational parameter like `seed` — it controls infrastructure behavior, not simulation semantics. It does not affect world state, agent behavior, or deterministic replay of the active window.

### 3. Stripping Mechanism

A new `SystemFn` registered in `SystemDispatchTable` (`system_dispatch.rs`), using the standard `SystemExecutionContext` signature. It runs after all other systems complete during tick orchestration (`tick_step.rs`):

```
fn compact_event_log(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>:
    let interval = ctx.event_log.compaction_interval();
    if interval == 0 || ctx.tick.0 % u64::from(interval) != 0 { return Ok(()); }

    // 1. Serialize current World as checkpoint
    let snapshot = bincode::serialize(ctx.world);
    ctx.event_log.add_checkpoint(ctx.tick, CheckpointData { world_snapshot: snapshot });

    // 2. Strip state_deltas from events older than this checkpoint
    ctx.event_log.strip_deltas_before(ctx.tick);

    // 3. Drop oldest checkpoint if we now have > 2
    ctx.event_log.prune_old_checkpoints(2);

    Ok(())
```

**Stripping implementation**: `EventRecord` gains a `strip_state_deltas(&mut self)` method that calls `self.payload.state_deltas.clear()` followed by `self.payload.state_deltas.shrink_to_fit()` to release the backing allocation.

**No new enum variants**: Since `state_deltas` is a `Vec<StateDelta>`, an empty Vec is the natural representation of "stripped." No marker variant is needed. Code that iterates `state_deltas()` will simply see an empty slice for compacted events.

### 4. Verification Adaptation

The test-only `ExpectedWorldState::from_event_log` in `verification.rs` (line 174) currently replays ALL state_deltas. After compaction, events before the checkpoint have empty state_deltas. The verification path becomes:

```
fn from_event_log(event_log: &EventLog) -> Self:
    // If checkpoints exist, start from the latest one
    if let Some((checkpoint_tick, data)) = event_log.latest_checkpoint() {
        let world: World = bincode::deserialize(&data.world_snapshot);
        // Extract entity_states, components, relations, reservations from world
        // Replay only state_deltas from checkpoint_tick forward
        for event in events_from(checkpoint_tick) {
            apply_state_deltas(event);
        }
    } else {
        // No checkpoint — replay all (pre-compaction or short simulation)
        for event in all_events {
            apply_state_deltas(event);
        }
    }
```

This is a test-infrastructure change only. It does not affect simulation correctness.

### 5. Determinism Guarantee

Compaction is deterministic:
- Checkpoint creation happens at tick T where `T % K == 0` — same tick every run with the same K
- Bincode serialization of the same `World` produces the same bytes (deterministic types, `BTreeMap` iteration order)
- Stripping is a pure function of tick numbers
- The simulation state after compaction is identical to the state without compaction — only the event log's internal storage changes

**Hashing note**: `hash_world()` (`canonical.rs:59`) produces identical hashes with and without compaction enabled, since compaction does not affect `World` state. `hash_event_log()` (`canonical.rs:63`) will differ between compacted and non-compacted runs because the event log's internal structure changes (stripped deltas, added checkpoints). This is expected and correct — the event log hash reflects storage state, while the world hash reflects simulation state.

## Files to Touch

- `crates/worldwake-core/src/event_log.rs` — add `checkpoints` field, `compaction_interval` field, `add_checkpoint`, `latest_checkpoint`, `prune_old_checkpoints`, `strip_deltas_before`, `set_compaction_interval`, `compaction_interval` methods
- `crates/worldwake-core/src/event_record.rs` — add `strip_state_deltas(&mut self)` method on `EventRecord` and `EventPayload`
- `crates/worldwake-core/src/verification.rs` — adapt `ExpectedWorldState::from_event_log` to use checkpoint-based reconstruction
- `crates/worldwake-cli/src/scenario/types.rs` — add `compaction_interval` to `ScenarioDef`
- `crates/worldwake-cli/src/scenario/mod.rs` — pass `compaction_interval` to `EventLog::set_compaction_interval()` during `assemble_state()`
- `crates/worldwake-sim/src/system_dispatch.rs` — register the compaction system function in `SystemDispatchTable`
- `crates/worldwake-sim/src/tick_step.rs` — schedule compaction SystemFn after all other systems

## Out of Scope

- Disk-backed event log (future extension for truly massive histories)
- Compacting non-state_deltas event fields (metadata is small enough to remain in RAM)
- Delta compaction by merging adjacent diffs (algebraic complexity not justified)
- Changing event emission frequency
- Modifying the append-only invariant

## Acceptance Criteria

### Memory

- RSS growth from `state_deltas` is bounded: after the first compaction cycle, the total memory consumed by `state_deltas` across all events stays below `compaction_interval * events_per_tick * avg_delta_size` regardless of total tick count
- At 3,000 ticks (seed 0, 20 agents), RSS is under 200 MB (vs. estimated 600 MB without compaction)
- Soak binary (10,080 ticks) completes without OOM on 8 GB RAM

### Correctness

- All golden tests pass with identical world-state hashes (compaction does not affect simulation outcomes)
- `cargo test --workspace` passes
- Verification roundtrip succeeds: checkpoint-based reconstruction via `ExpectedWorldState::from_event_log` produces the same final state as full-replay reconstruction (tested by running both paths on the same event log before compaction)

### Determinism

- Two runs with the same seed and compaction_interval produce identical checkpoint bytes at every checkpoint tick
- `hash_world()` is identical with and without compaction enabled (compaction does not affect World state)
- `hash_event_log()` is expected to differ between compacted and non-compacted runs (different internal storage, same simulation outcomes)

### Configuration

- `compaction_interval: 0` disables compaction (no checkpoints, no stripping)
- Default `compaction_interval` is 50 ticks
- Existing scenario RON files work without modification (serde default)

## Test Plan

### New Tests

1. **Checkpoint roundtrip** (`event_log.rs`) — serialize World, deserialize, compare to original
2. **Stripping correctness** (`event_log.rs`) — after `strip_deltas_before(tick)`, events before tick have empty `state_deltas`, events at/after tick are unchanged
3. **Verification from checkpoint** (`verification.rs`) — `ExpectedWorldState::from_event_log` with compacted log produces same result as with uncompacted log
4. **Checkpoint pruning** (`event_log.rs`) — after 3 checkpoints, only 2 remain (oldest dropped)
5. **Disabled compaction** — `compaction_interval: 0` produces no checkpoints and no stripping
6. **Determinism** — two identical runs produce identical checkpoint bytes and identical `hash_world()` output

### Commands

1. `cargo test -p worldwake-core` (checkpoint, stripping, verification tests)
2. `cargo test -p worldwake-ai` (golden tests for determinism)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
