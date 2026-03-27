**Status**: PENDING

# S30: AI Runtime Save/Load Parity

## Summary

Promote performance-critical fields of `AgentDecisionRuntime` to serializable state so that save/load round-trips preserve AI decision continuity. Currently, transient caches (`search_exhausted_goals`, `exhaustion_counts`, observation snapshots, materialization bindings) are lost on load, causing behavioral divergence between uninterrupted and resumed execution. This spec defines which runtime fields are "derived caches" (reconstructable, per Principle 25) and which carry causal history that must survive boundaries (per Principle 11).

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
- `worldwake-sim` (save/load serialization boundary)
- `worldwake-core` (if any promoted fields become components)

## Dependencies

- None (can proceed independently, but synergizes with S29 for planning-state efficiency)

## Design Goals

1. **Save/load parity**: An agent's next decision after load is identical to what it would have been without the save/load boundary.
2. **Minimal serialization**: Only promote fields that carry non-reconstructable causal history. Derived caches that can be rebuilt from world state within 1-2 ticks should NOT be serialized.
3. **Determinism preserved**: Identical seeds + identical save/load sequences produce identical outcomes.
4. **No new components**: Runtime fields are serialized as part of the `AgentTickDriver` state, not as per-agent ECS components. The runtime is AI-layer state, not world state (Principle 24).
5. **Forward migration**: Old saves without runtime data load cleanly (empty/default runtime, convergent within a few ticks).

## Deliverables

### 1. Classification of `AgentDecisionRuntime` fields

| Field | Classification | Rationale |
|-------|---------------|-----------|
| `current_plan: Option<PlannedPlan>` | **Serialize** | Carries multi-tick commitment that agents must resume, not re-derive. Lost plan = agent restarts from scratch, changing behavior. |
| `current_step_index: usize` | **Serialize** | Position within the current plan. Losing this means re-executing already-completed steps. |
| `step_in_flight: bool` | **Serialize** | Whether an action request is pending. Losing this corrupts the action lifecycle. |
| `search_exhausted_goals: BTreeMap<GoalKey, Tick>` | **Serialize** | TTL-based exhaustion cache. Losing this causes expensive re-searches on load. |
| `exhaustion_counts: BTreeMap<GoalKey, u8>` | **Serialize** | Exponential backoff state. Losing this resets the penalty for chronically unsolvable goals. |
| `dirty: DirtySet` | **Derive** | Rebuilt from observation snapshot comparison on first tick after load. |
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

### 2. Serialization integration

Add `Serialize`/`Deserialize` derives to `AgentDecisionRuntime` (excluding derived fields via `#[serde(skip)]`).

Extend `AgentTickDriver` to serialize its `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>` as part of the save/load boundary:

```rust
#[derive(Serialize, Deserialize)]
pub struct AgentTickDriverState {
    pub budget: PlanningBudget,
    pub runtimes: BTreeMap<EntityId, AgentDecisionRuntime>,
}
```

`SimulationState` gains an optional `ai_runtime_state: Option<AgentTickDriverState>` field. Old saves without this field load with `None` (fresh runtimes, convergent behavior).

### 3. Save/load integration in `worldwake-sim`

- `save()` captures `AgentTickDriverState` from the input producer if it implements a new `SaveableRuntime` trait.
- `load()` restores the runtime state into the `AgentTickDriver` via `restore_runtime()`.
- If no runtime state is present in the save data, the driver starts fresh (backward compatible).

### 4. Remove golden_determinism driver reset workaround

After save/load parity is established, the `uninterrupted.driver = AgentTickDriver::new(PlanningBudget::default())` reset in `golden_save_load_round_trip_under_ai` becomes unnecessary and should be removed. The test should pass without it.

### 5. Increase EXHAUSTION_SKIP_TTL to optimal value

With save/load parity, the TTL can be increased beyond 16 without breaking determinism tests. Re-run the golden-perf harness to find the new optimal TTL (potentially 32+).

## Component Registration

No new ECS components. The AI runtime state is serialized as a separate top-level field in the save format, not as per-entity component data. This preserves the architectural boundary between world state (components) and AI-layer state (runtime).

## SystemFn Integration

### `worldwake-sim`

- Extend `SimulationState` with `ai_runtime_state: Option<AgentTickDriverState>`.
- Extend `save()` to capture AI runtime state via trait.
- Extend `load()` to restore AI runtime state.
- Add `SaveableRuntime` trait for autonomous controllers that support state persistence.

### `worldwake-ai`

- Add `Serialize`/`Deserialize` to `AgentDecisionRuntime`, `PlannedPlan`, `PlannedStep`, `MaterializationBindings`, and all nested types.
- Implement `SaveableRuntime` for `AgentTickDriver`.
- Add `AgentTickDriver::restore_runtime()` method.

## Cross-System Interactions (Principle 12)

The AI runtime state is NOT world state. It is a derived optimization layer. Systems continue to interact through world state (components, relations, events). The runtime serialization is a boundary concern (Principle 11) — it preserves the AI layer's internal state across the save/load boundary without affecting how systems read or write world state.

## FND-01 Section H

### H.1 Information-Path Analysis

No information paths are changed. The serialized fields already exist in memory during normal execution. Save/load parity means the post-load information state is identical to what it would have been without the boundary.

### H.2 Positive-Feedback Analysis

No feedback loops introduced. The change affects state persistence, not state evolution.

### H.3 Concrete Dampeners

N/A — no feedback loops.

### H.4 Stored vs Derived State

- **Stored (in save data)**: `current_plan`, `current_step_index`, `step_in_flight`, `search_exhausted_goals`, `exhaustion_counts`, `last_effective_place`, `last_needs`, `last_wounds`, `last_commodity_signature`, `last_unique_item_signature`, `last_facility_access_signature`, `last_in_transit`, `materialization_bindings`.
- **Derived (not stored)**: `dirty` (rebuilt from snapshot comparison), `last_priority_class` (rebuilt from ranking), `last_frame_clear_reason` (diagnostic only), `semantics_cache` (rebuilt from action defs).

All stored fields are either commitments (plan state) or observation anchors (snapshot baselines for dirty detection). None are derived values promoted to truth — they are the ground truth of the AI layer's internal state.

## Invariants

1. `save(state) → load(data) → advance 1 tick` produces identical world state to `advance 1 tick` without save/load (given identical seed and inputs).
2. Old saves without `ai_runtime_state` load successfully with empty runtimes (backward compatibility).
3. `AgentDecisionRuntime` serialization is deterministic (BTreeMap ordering preserved, no floats).
4. The runtime serialization does NOT include any reference to world state (`&World`, `&EventLog`) — it is self-contained.

## Tests

- [ ] Save/load round-trip with AI runtime: uninterrupted and resumed runs produce identical world hashes WITHOUT driver reset.
- [ ] Old save format loads successfully (no `ai_runtime_state` field).
- [ ] Exhaustion cache survives save/load: agent that exhausted a goal before save still skips it after load.
- [ ] Multi-step plan continuation: agent mid-plan at save resumes at the correct step after load.
- [ ] `golden_save_load_round_trip_under_ai` passes without the driver reset workaround.
- [ ] `golden_save_load_preserves_promoted_commitments` continues to pass.

## Acceptance Criteria

1. The driver reset workaround in `golden_save_load_round_trip_under_ai` is removed and the test passes.
2. EXHAUSTION_SKIP_TTL can be increased to 32+ without breaking any save/load determinism test.
3. All 2700+ workspace tests pass.
4. Save format backward compatibility: old saves load without error.
5. Profiling shows no regression from serialization overhead (the fields are small).

## References

- golden-perf campaign: exp-005 (indefinite accumulation broke save/load), exp-016 (TTL=32 broke save/load)
- `crates/worldwake-ai/src/decision_runtime.rs` — `AgentDecisionRuntime`
- `crates/worldwake-ai/src/agent_tick/mod.rs` — `AgentTickDriver`
- `crates/worldwake-sim/src/save_load.rs` — current serialization
- `docs/FOUNDATIONS.md` Principle 11, Principle 28 item 11
