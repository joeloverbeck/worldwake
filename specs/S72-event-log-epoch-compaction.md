# S72: Event Log Epoch Compaction

## Summary

S71 reduced per-event memory from ~300 KB to ~1-5 KB by storing structural diffs instead of full component snapshots. However, the event log (`Vec<EventRecord>`) remains strictly append-only with no mechanism to reclaim memory from old events. Even at 1-5 KB per delta, unbounded tick counts produce unbounded RAM growth — unacceptable for a simulation that will run indefinitely as a game backbone.

This spec introduces periodic **checkpoint snapshots** of the component store, after which the heavy `state_deltas` payloads on older events are stripped from memory. Event metadata (who, what, when, where, causal chain, tags, evidence, observed entities) is preserved indefinitely. Full component state at any historical tick remains reconstructible from the nearest checkpoint plus subsequent deltas.

## Phase

Phase 7: Consequence Carriers

## Status

DRAFT

## Motivation

After S71, per-event size dropped from ~150 KB to ~1-5 KB average. But the growth is still linear and unbounded:

| Ticks | Estimated Events | Estimated state_deltas RAM | Event Metadata RAM |
|-------|-----------------|---------------------------|-------------------|
| 300 | ~20,000 | ~60 MB | ~6 MB |
| 3,000 | ~200,000 | ~600 MB | ~60 MB |
| 30,000 | ~2,000,000 | ~6 GB | ~600 MB |

The `state_deltas` column dominates. Event metadata (all other fields) grows at ~300 bytes/event — manageable for millions of events. The `state_deltas` payload is the only field that needs compaction.

Codebase analysis confirms `state_deltas` is consumed at runtime only by test-only verification code (`#[cfg(test)]` in `verification.rs`). All production runtime systems — perception, AI planning, CLI display — use other event fields (`evidence`, `observed_entities`, `tags`, `witness_data`, etc.) and never read `state_deltas`.

## Crates

- `worldwake-core` (EventLog, EventRecord, EventPayload, checkpoint storage, stripping logic)
- `worldwake-cli` (ScenarioDef — adds `compaction_interval` field)

## Dependencies

S71 (event log delta compaction) — COMPLETED. S72 builds on the compact diff representation introduced by S71.

## Design Goals

- **Bounded RSS for state_deltas** — memory consumed by component deltas is capped at O(checkpoint_interval * events_per_tick * avg_delta_size), regardless of total tick count
- **Preserve event metadata** — all non-delta event fields persist for the full simulation lifetime (FND-29A)
- **Preserve causal reconstructibility** — component state at any historical tick can be reconstructed from the nearest prior checkpoint plus subsequent deltas (FND-12)
- **Preserve runtime correctness** — no production system reads `state_deltas`, so stripping has zero runtime impact
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
| FND-12 (Performance Compresses Computation, Never Causality) | Checkpoints and stripping change how component state is stored, not what the world means. Any historical component state remains derivable by replaying from the nearest checkpoint. The causal chain is unmodified. |
| FND-27 (Derived Summaries Are Caches) | Checkpoints are derived state — recomputable from initial world state plus the full event stream (before any stripping). After stripping, the checkpoint becomes the reconstruction base, but it is not authoritative truth; it is a performance cache that enables bounded memory. |
| FND-3 (Concrete State Over Abstract Scores) | No abstract scores introduced. Checkpoints store concrete `ComponentStore` state. |
| FND-28 (No Backward Compatibility) | Clean implementation. No shims or wrappers for pre-compaction event formats. |

## Information-Path Analysis

The compaction system introduces no new information paths. It operates purely on the event log's internal storage:

1. **Checkpoint creation**: reads the authoritative `ComponentStore` at tick T, serializes it via bincode, stores it on the `EventLog`. No agent, system, or perception path is involved.
2. **Delta stripping**: empties the `state_deltas` field on events with tick < T. Since no production runtime system reads `state_deltas`, this is invisible to all information paths.
3. **Reconstruction** (test/debug only): reads the nearest checkpoint, deserializes it, then replays `state_deltas` from checkpoint tick forward. This path exists only in test infrastructure and CLI debugging tools.

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
| Checkpoint `ComponentStore` snapshots | Derived state (cache). Recomputable from initial world state + full event replay. Stored for performance — enables bounded memory by making old deltas redundant. |

## Design

### 1. Checkpoint Storage

Add checkpoint infrastructure to `EventLog`:

```rust
/// Serialized component-store snapshot at a specific tick.
pub struct CheckpointData {
    /// Bincode-serialized ComponentStore state.
    component_snapshot: Vec<u8>,
}

pub struct EventLog {
    events: Vec<EventRecord>,
    // ... existing indices ...

    /// Periodic component-store snapshots. Key: tick.
    /// At most 2 retained (current + previous for safety).
    checkpoints: BTreeMap<Tick, CheckpointData>,
}
```

The `checkpoints` BTreeMap stores at most 2 entries. After a new checkpoint is created and stripping against it succeeds, the oldest checkpoint (if 3 exist) is removed.

### 2. Compaction Interval Configuration

Add an optional `compaction_interval` field to `ScenarioDef`:

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

A new `SystemFn` registered in the tick scheduler, running after all other systems complete:

```
fn compact_event_log(world: &mut World, tick: Tick):
    let interval = world.compaction_interval();
    if interval == 0 || tick.0 % interval != 0 { return; }

    // 1. Serialize current ComponentStore as checkpoint
    let snapshot = bincode::serialize(world.component_store());
    world.event_log_mut().add_checkpoint(tick, CheckpointData { component_snapshot: snapshot });

    // 2. Strip state_deltas from events older than this checkpoint
    let cutoff_tick = tick;
    for event_id in events_before(cutoff_tick) {
        event.strip_state_deltas();  // Replaces Vec<StateDelta> with empty Vec
    }

    // 3. Drop oldest checkpoint if we now have > 2
    world.event_log_mut().prune_old_checkpoints(max_checkpoints: 2);
```

**Stripping implementation**: `EventRecord` gains a `strip_state_deltas(&mut self)` method that calls `self.payload.state_deltas.clear()` followed by `self.payload.state_deltas.shrink_to_fit()` to release the backing allocation.

**No new enum variants**: Since `state_deltas` is a `Vec<StateDelta>`, an empty Vec is the natural representation of "stripped." No marker variant is needed. Code that iterates `state_deltas()` will simply see an empty slice for compacted events.

### 4. Verification Adaptation

The test-only `VerificationState::from_event_log` in `verification.rs` currently replays ALL state_deltas. After compaction, events before the checkpoint have empty state_deltas. The verification path becomes:

```
fn from_event_log(event_log: &EventLog) -> Self:
    // If checkpoints exist, start from the latest one
    if let Some((checkpoint_tick, data)) = event_log.latest_checkpoint() {
        let components = bincode::deserialize(&data.component_snapshot);
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
- Bincode serialization of the same `ComponentStore` produces the same bytes (deterministic types, `BTreeMap` iteration order)
- Stripping is a pure function of tick numbers
- The simulation state after compaction is identical to the state without compaction — only the event log's internal storage changes

**Important**: World-state hashing (used for save/load determinism checks) must exclude checkpoint data and must account for stripped deltas. The hash should cover event metadata and the live component store, not the event log's internal storage optimizations.

## Files to Touch

- `crates/worldwake-core/src/event_log.rs` — add `checkpoints` field, `add_checkpoint`, `latest_checkpoint`, `prune_old_checkpoints`, `strip_deltas_before` methods
- `crates/worldwake-core/src/event_record.rs` — add `strip_state_deltas(&mut self)` method on `EventRecord` and `EventPayload`
- `crates/worldwake-core/src/verification.rs` — adapt `from_event_log` to use checkpoint-based reconstruction
- `crates/worldwake-cli/src/scenario/types.rs` — add `compaction_interval` to `ScenarioDef`
- `crates/worldwake-cli/src/scenario/mod.rs` — pass `compaction_interval` to world initialization
- `crates/worldwake-sim/src/scheduler.rs` (or wherever `SystemFn` registration lives) — register the compaction system function

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
- Verification roundtrip succeeds: checkpoint-based reconstruction produces the same final component state as full-replay reconstruction (tested by running both paths on the same event log)

### Determinism

- Two runs with the same seed and compaction_interval produce identical checkpoint bytes at every checkpoint tick
- World-state hash is identical with and without compaction enabled

### Configuration

- `compaction_interval: 0` disables compaction (no checkpoints, no stripping)
- Default `compaction_interval` is 50 ticks
- Existing scenario RON files work without modification (serde default)

## Test Plan

### New Tests

1. **Checkpoint roundtrip** (`event_log.rs`) — serialize ComponentStore, deserialize, compare to original
2. **Stripping correctness** (`event_log.rs`) — after `strip_deltas_before(tick)`, events before tick have empty `state_deltas`, events at/after tick are unchanged
3. **Verification from checkpoint** (`verification.rs`) — `from_event_log` with compacted log produces same result as `from_event_log` with uncompacted log
4. **Checkpoint pruning** (`event_log.rs`) — after 3 checkpoints, only 2 remain (oldest dropped)
5. **Disabled compaction** — `compaction_interval: 0` produces no checkpoints and no stripping
6. **Determinism** — two identical runs produce identical checkpoint bytes

### Commands

1. `cargo test -p worldwake-core` (checkpoint, stripping, verification tests)
2. `cargo test -p worldwake-ai` (golden tests for determinism)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
