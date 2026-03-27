**Status**: PENDING

# S30: AI Runtime Save/Load Parity

## Summary

Promote performance-critical fields of `AgentDecisionRuntime` to serializable state so that save/load round-trips preserve AI decision continuity. Currently, transient caches (exhaustion cache, observation snapshots, materialization bindings) are lost on load, causing behavioral divergence between uninterrupted and resumed execution. This spec defines which runtime fields are "derived caches" (reconstructable, per Principle 25) and which carry causal history that must survive boundaries (per Principle 11).

Additionally, this spec merges the separate `search_exhausted_goals` and `exhaustion_counts` maps into a unified `ExhaustionEntry` struct, preparing the data model for S31's goal-aware invalidation conditions.

## Why This Exists

The golden-perf campaign exposed a fundamental tension between AI optimization and save/load determinism:

1. **exp-005**: Indefinite exhaustion cache accumulation (the correct optimization) broke `golden_save_load_round_trip_under_ai` because the resumed run started with an empty cache while the uninterrupted run had accumulated state.
2. **exp-016**: TTL=32 broke the same test for the same reason — the divergence window exceeded the test's 30-tick resume period.
3. **Workaround**: The golden_determinism test had to reset the uninterrupted driver at the save boundary, masking the underlying problem.

Principle 11 states: "boundaries may change encoding, batching, or scheduling strategy, never world meaning." A save/load boundary currently DOES change world meaning because it erases the AI's learned search history, causing different action choices post-load.

Principle 28, item 11 requires specs to declare "what must survive save/load, replay, and offscreen compression without changing world meaning."

## Phase

Phase 3+: AI Architecture Overhaul

## Crates

- `worldwake-ai`
- `worldwake-sim` (save/load serialization boundary, `SaveableRuntime` trait)

## Dependencies

- None (can proceed independently, but synergizes with S29 for planning-state efficiency)

## Design Goals

1. **Save/load parity**: An agent's next decision after load is identical to what it would have been without the save/load boundary.
2. **Minimal serialization**: Only promote fields that carry non-reconstructable causal history. Derived caches that can be rebuilt from world state within 1-2 ticks should NOT be serialized.
3. **Determinism preserved**: Identical seeds + identical save/load sequences produce identical outcomes.
4. **No new components**: Runtime fields are serialized as part of the `AgentTickDriver` state, not as per-agent ECS components. The runtime is AI-layer state, not world state (Principle 24).
5. **Forward migration**: Old saves without runtime data load cleanly (empty/default runtime, convergent within a few ticks).
6. **Crate boundary respect**: `worldwake-sim` cannot depend on `worldwake-ai`. The serialization boundary uses opaque bytes and a trait abstraction.
7. **Post-load integrity**: After restoring runtime state, validate that entity references are still live and initialize derived fields.

## Deliverables

### 1. Classification of `AgentDecisionRuntime` fields

| Field | Classification | Rationale |
|-------|---------------|-----------|
| `current_plan: Option<PlannedPlan>` | **Serialize** | Carries multi-tick commitment that agents must resume, not re-derive. Lost plan = agent restarts from scratch, changing behavior. |
| `current_step_index: usize` | **Serialize** | Position within the current plan. Losing this means re-executing already-completed steps. |
| `step_in_flight: bool` | **Serialize** | Whether an action request is pending. Losing this corrupts the action lifecycle. |
| `exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>` | **Serialize** | Unified exhaustion state (see Deliverable 3). Losing this causes expensive re-searches and resets backoff penalties. |
| `dirty: DirtySet` | **Derive** | Initialized to `DirtySet::all()` after load, forcing full re-evaluation on first post-load tick. Normal operation rebuilds it from observation snapshot comparison. |
| `last_priority_class` | **Derive** | Rebuilt from first candidate generation after load. |
| `last_effective_place` | **Serialize** | Observation snapshot anchor. Without it, first tick after load has no baseline for dirty detection — all bits fire, causing full re-search. |
| `last_needs` | **Serialize** | Same as above — observation snapshot anchor. |
| `last_wounds` | **Serialize** | Same. |
| `last_commodity_signature` | **Serialize** | Same. |
| `last_unique_item_signature` | **Serialize** | Same. |
| `last_facility_access_signature` | **Serialize** | Same. |
| `last_in_transit` | **Serialize** | Controls POSITION dirty-bit clearing logic for multi-leg travel. |
| `materialization_bindings` | **Serialize** | Maps hypothetical entities from planning to authoritative entities created during execution. Losing this breaks multi-step plan execution. |
| `last_frame_clear_reason` | **Derive** | Diagnostic only; does not affect decision logic. |

#### `AgentTickDriver` fields

| Field | Classification | Rationale |
|-------|---------------|-----------|
| `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` | **Serialize** | Core per-agent runtime map. |
| `budget: PlanningBudget` | **Serialize** | Planning budget parameters (already has serde). |
| `semantics_cache: Option<(usize, BTreeMap<ActionDefId, PlannerOpSemantics>)>` | **Derive** | Rebuilt from `ActionDefRegistry` on first use after load. |
| `trace_sink: Option<DecisionTraceSink>` | **Derive** | Session-local diagnostic tool; never persisted. |

### 2. Serialization boundary design

#### Crate boundary constraint

`SimulationState` lives in `worldwake-sim`. `AgentTickDriver` lives in `worldwake-ai`. The crate dependency graph (`sim → core` only, `ai → core + sim + systems`) means `worldwake-sim` cannot reference AI types directly.

#### Solution: Opaque bytes with `SaveableRuntime` trait

Add a `SaveableRuntime` trait to `worldwake-sim`:

```rust
/// Trait for autonomous controllers that support state persistence
/// across save/load boundaries. The state is serialized as opaque bytes
/// to preserve the crate dependency boundary (sim cannot depend on ai).
pub trait SaveableRuntime {
    /// Serialize internal runtime state to opaque bytes (bincode format).
    fn save_runtime_state(&self) -> Result<Vec<u8>, SaveLoadError>;

    /// Restore internal runtime state from opaque bytes.
    /// Caller MUST call post_load_validate() afterward.
    fn restore_runtime_state(&mut self, bytes: &[u8]) -> Result<(), SaveLoadError>;
}
```

#### Save format extension

The save format gains an auxiliary AI runtime payload after the `SimulationState` payload:

```
[magic: 4 bytes "WWAK"]
[version: 4 bytes u32-LE]
[sim_payload_len: 8 bytes u64-LE]
[sim_payload: N bytes]
[ai_payload_len: 8 bytes u64-LE]    ← NEW
[ai_payload: M bytes]                ← NEW (0 bytes if no runtime)
```

- `save_to_bytes()` gains a `runtime: Option<&dyn SaveableRuntime>` parameter.
- `load_from_bytes()` returns `(SimulationState, Option<Vec<u8>>)` — the caller passes the opaque bytes to the driver's `restore_runtime_state()`.
- If no AI payload is present (old saves, `ai_payload_len == 0`), the driver starts fresh.

#### Internal serialization type

Within `worldwake-ai`, `AgentTickDriver` serializes its persistable state through an internal struct:

```rust
#[derive(Serialize, Deserialize)]
struct AgentTickDriverState {
    budget: PlanningBudget,
    runtimes: BTreeMap<EntityId, AgentDecisionRuntime>,
}
```

This type is private to `worldwake-ai` — it is never exposed to `worldwake-sim`.

### 3. `ExhaustionEntry` unification

Replace the separate `search_exhausted_goals: BTreeMap<GoalKey, Tick>` and `exhaustion_counts: BTreeMap<GoalKey, u8>` with a unified struct:

```rust
/// Per-goal exhaustion state: records when a goal's search was exhausted
/// and how many times it has been exhausted (for exponential backoff).
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExhaustionEntry {
    /// Tick at which the goal was marked exhausted.
    pub exhausted_at: Tick,
    /// Number of consecutive exhaustion events (drives exponential backoff).
    pub count: u8,
}
```

`AgentDecisionRuntime` gains:

```rust
pub exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>,
```

All call sites that previously read/wrote `search_exhausted_goals` or `exhaustion_counts` are updated to use `exhaustion_cache`. The TTL skip logic (`is_exhausted()`) reads `entry.exhausted_at`; backoff logic reads `entry.count`.

**S31 note**: S31 will add an `invalidation: Option<ExhaustionInvalidationCondition>` field to `ExhaustionEntry` and bump the save format version. This spec intentionally keeps the struct minimal.

### 4. Serde derives

- Add `Serialize, Deserialize` to `AgentDecisionRuntime` with `#[serde(skip)]` on derived fields (`dirty`, `last_priority_class`, `last_frame_clear_reason`).
- Add `Serialize, Deserialize` to `MaterializationBindings`.
- `DirtySet` does NOT need serde derives (it is skipped and reconstructed).
- `PlannedPlan`, `PlannedStep`, `GoalKey`, `PlanningBudget`, and all nested types already have serde derives — no changes needed.

### 5. Post-load validation

After deserializing AI runtime state, validate entity references against the loaded world:

```rust
impl AgentTickDriver {
    /// Prune stale entity references and initialize derived state after load.
    pub fn post_load_validate(&mut self, world: &World) {
        // 1. Remove runtimes for agents that no longer exist in the world.
        self.runtime_by_agent.retain(|agent, _| world.is_alive(*agent));

        // 2. For each surviving runtime:
        for runtime in self.runtime_by_agent.values_mut() {
            // a. Prune exhaustion_cache entries whose GoalKey references dead entities.
            runtime.exhaustion_cache.retain(|key, _| {
                key.entity.map_or(true, |e| world.is_alive(e))
                    && key.place.map_or(true, |e| world.is_alive(e))
            });

            // b. Prune materialization_bindings referencing dead entities.
            runtime.materialization_bindings
                .hypothetical_to_authoritative
                .retain(|_, auth| world.is_alive(*auth));

            // c. Set dirty to all-dirty so first tick forces full re-evaluation.
            runtime.dirty = DirtySet::all();
        }

        // 3. Clear semantics_cache (will be rebuilt from ActionDefRegistry).
        self.semantics_cache = None;
    }
}
```

### 6. `SAVE_FORMAT_VERSION` bump

Bump `SAVE_FORMAT_VERSION` from 5 to 6. Version 5 saves (without AI payload) are detected by `ai_payload_len == 0` or by the absence of the AI payload section (EOF after sim payload). The driver starts fresh in this case.

### 7. Remove golden_determinism driver reset workaround

After save/load parity is established, the `AgentTickDriver::new(PlanningBudget::default())` reset in `GoldenHarness::from_simulation_state()` becomes unnecessary and should be removed. The test should pass without it because the driver's runtime state is now preserved across the save/load boundary.

### 8. Increase EXHAUSTION_SKIP_TTL to optimal value

With save/load parity, the TTL can be increased beyond 16 without breaking determinism tests. Re-run the golden-perf harness to find the new optimal TTL (potentially 32+).

**S31 note**: S31 will eventually remove TTL entirely in favor of condition-based invalidation. This TTL increase is an interim optimization.

## Component Registration

No new ECS components. The AI runtime state is serialized as opaque bytes appended to the save format, not as per-entity component data. This preserves the architectural boundary between world state (components) and AI-layer state (runtime).

## SystemFn Integration

### `worldwake-sim`

- Add `SaveableRuntime` trait to `autonomous_controller.rs` (or a new `saveable_runtime.rs` module).
- Extend `save_to_bytes()` to accept `Option<&dyn SaveableRuntime>` and append AI payload.
- Extend `load_from_bytes()` to return `(SimulationState, Option<Vec<u8>>)` with the AI payload.
- Bump `SAVE_FORMAT_VERSION` from 5 to 6.

### `worldwake-ai`

- Add `Serialize`/`Deserialize` to `AgentDecisionRuntime` and `MaterializationBindings`.
- Introduce `ExhaustionEntry` struct and refactor `exhaustion_cache` field.
- Implement `SaveableRuntime` for `AgentTickDriver`.
- Add `AgentTickDriver::post_load_validate()` method.
- Update all call sites for the exhaustion cache unification.

## Cross-System Interactions (Principle 12)

The AI runtime state is NOT world state. It is a derived optimization layer. Systems continue to interact through world state (components, relations, events). The runtime serialization is a boundary concern (Principle 11) — it preserves the AI layer's internal state across the save/load boundary without affecting how systems read or write world state.

The opaque bytes approach reinforces Principle 24 (systems interact through state, not through each other): `worldwake-sim` has no knowledge of `worldwake-ai` types. It stores and retrieves raw bytes. The AI layer is solely responsible for interpreting those bytes.

## FND-01 Section H

### H.1 Information-Path Analysis

No information paths are changed. The serialized fields already exist in memory during normal execution. Save/load parity means the post-load information state is identical to what it would have been without the boundary.

### H.2 Positive-Feedback Analysis

No feedback loops introduced. The change affects state persistence, not state evolution.

### H.3 Concrete Dampeners

N/A — no feedback loops.

### H.4 Stored vs Derived State

- **Stored (in save data)**: `current_plan`, `current_step_index`, `step_in_flight`, `exhaustion_cache` (unified `ExhaustionEntry` map), `last_effective_place`, `last_needs`, `last_wounds`, `last_commodity_signature`, `last_unique_item_signature`, `last_facility_access_signature`, `last_in_transit`, `materialization_bindings`, `budget`.
- **Derived (not stored)**: `dirty` (initialized to `DirtySet::all()` after load), `last_priority_class` (rebuilt from ranking), `last_frame_clear_reason` (diagnostic only), `semantics_cache` (rebuilt from action defs), `trace_sink` (session-local diagnostic).

All stored fields are either commitments (plan state), observation anchors (snapshot baselines for dirty detection), or optimization state (exhaustion cache). None are derived values promoted to truth — they are the ground truth of the AI layer's internal state.

## Invariants

1. `save(state) → load(data) → advance 1 tick` produces identical world state to `advance 1 tick` without save/load (given identical seed and inputs).
2. Old saves without AI runtime payload load successfully with empty runtimes (backward compatibility).
3. `AgentDecisionRuntime` serialization is deterministic (BTreeMap ordering preserved, no floats).
4. The runtime serialization does NOT include any reference to world state (`&World`, `&EventLog`) — it is self-contained.
5. After `post_load_validate()`, all entity references in the runtime are guaranteed live.
6. After `post_load_validate()`, `dirty` is `DirtySet::all()` — the first tick always re-evaluates.

## Tests

- [ ] Save/load round-trip with AI runtime: uninterrupted and resumed runs produce identical world hashes WITHOUT driver reset.
- [ ] Old save format (v5) loads successfully with no AI payload.
- [ ] Exhaustion cache survives save/load: agent that exhausted a goal before save still skips it after load.
- [ ] Multi-step plan continuation: agent mid-plan at save resumes at the correct step after load.
- [ ] `golden_save_load_round_trip_under_ai` passes without the driver reset workaround.
- [ ] `golden_save_load_preserves_promoted_commitments` continues to pass.
- [ ] Post-load validation prunes a runtime entry referencing a dead agent.
- [ ] Post-load validation prunes exhaustion cache entries referencing dead entities.
- [ ] `ExhaustionEntry` unification: existing TTL skip and backoff logic works identically with the unified struct.

## Acceptance Criteria

1. The driver reset workaround in `golden_save_load_round_trip_under_ai` is removed and the test passes.
2. EXHAUSTION_SKIP_TTL can be increased to 32+ without breaking any save/load determinism test.
3. All workspace tests pass.
4. Save format backward compatibility: old saves (v5) load without error.
5. Profiling shows no regression from serialization overhead (the fields are small).
6. `post_load_validate()` is called after every `restore_runtime_state()` — no code path skips it.
7. `ExhaustionEntry` unification does not change any observable behavior (pure refactoring).

## References

- golden-perf campaign: exp-005 (indefinite accumulation broke save/load), exp-016 (TTL=32 broke save/load)
- `crates/worldwake-ai/src/decision_runtime.rs` — `AgentDecisionRuntime`, `MaterializationBindings`
- `crates/worldwake-ai/src/agent_tick/mod.rs` — `AgentTickDriver`
- `crates/worldwake-sim/src/save_load.rs` — current serialization, `SAVE_FORMAT_VERSION`
- `crates/worldwake-sim/src/autonomous_controller.rs` — `AutonomousController` trait
- `crates/worldwake-ai/tests/golden_harness/mod.rs` — driver reset workaround (line ~1181)
- `crates/worldwake-ai/tests/golden_determinism.rs` — `golden_save_load_round_trip_under_ai`
- `docs/FOUNDATIONS.md` Principle 11 (boundaries don't change world meaning), Principle 24 (systems interact through state), Principle 25 (derived summaries are caches), Principle 28 item 11 (declare what survives save/load)
- S31 (goal-aware exhaustion invalidation) — depends on this spec; will extend `ExhaustionEntry`
