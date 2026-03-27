# S30-004: Implement SaveableRuntime for AgentTickDriver

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AgentTickDriver gains save/restore capability
**Deps**: S30-002 (serde derives on AgentDecisionRuntime), S30-003 (SaveableRuntime trait exists)

## Problem

`AgentTickDriver` holds per-agent runtime state (`runtime_by_agent`, `budget`) that must survive save/load boundaries to maintain AI decision continuity (Principle 11). With serde derives on the runtime types (S30-002) and the `SaveableRuntime` trait in place (S30-003), this ticket wires the actual serialization.

## Assumption Reassessment (2026-03-27)

1. `AgentTickDriver` has private fields: `runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>`, `budget: PlanningBudget`, `semantics_cache: Option<(...)>`, `trace_sink: Option<DecisionTraceSink>` (`agent_tick/mod.rs:53-59`).
2. `PlanningBudget` already derives `Serialize, Deserialize` (`budget.rs:4`).
3. After S30-002, `AgentDecisionRuntime` will derive `Serialize, Deserialize`.
4. `semantics_cache` and `trace_sink` are derived/session-local — NOT serialized.
5. The internal `AgentTickDriverState` struct is private to `worldwake-ai` and never exposed to `worldwake-sim`.
6. `SaveableRuntime` trait (from S30-003) uses `Result<Vec<u8>, SaveLoadError>` — need to import `SaveLoadError` from `worldwake-sim`.

## Architecture Check

1. The `AgentTickDriverState` intermediary struct keeps serialization concerns isolated — only the fields that matter are included, and the struct is private.
2. Using bincode for the inner serialization matches the existing save format convention.
3. No shims — if a field is derived, it is excluded from the serialization struct entirely.

## Verification Layers

1. Round-trip fidelity → focused test: populate driver with runtimes → `save_runtime_state()` → `restore_runtime_state()` → assert runtimes match
2. Derived fields excluded → assert `semantics_cache` and `trace_sink` are `None` after restore
3. Single-layer ticket (AI crate serialization impl) — no cross-layer mapping needed.

## What to Change

### 1. Define private `AgentTickDriverState` struct

In `crates/worldwake-ai/src/agent_tick/mod.rs`:

```rust
#[derive(Serialize, Deserialize)]
struct AgentTickDriverState {
    budget: PlanningBudget,
    runtimes: BTreeMap<EntityId, AgentDecisionRuntime>,
}
```

### 2. Implement `SaveableRuntime` for `AgentTickDriver`

```rust
impl SaveableRuntime for AgentTickDriver {
    fn save_runtime_state(&self) -> Result<Vec<u8>, SaveLoadError> {
        let state = AgentTickDriverState {
            budget: self.budget.clone(),
            runtimes: self.runtime_by_agent.clone(),
        };
        bincode::serialize(&state).map_err(|e| SaveLoadError::...)
    }

    fn restore_runtime_state(&mut self, bytes: &[u8]) -> Result<(), SaveLoadError> {
        let state: AgentTickDriverState =
            bincode::deserialize(bytes).map_err(|e| SaveLoadError::...)?;
        self.budget = state.budget;
        self.runtime_by_agent = state.runtimes;
        self.semantics_cache = None;
        self.trace_sink = None;
        Ok(())
    }
}
```

### 3. Update golden harness to pass driver as runtime during save

In `golden_harness/mod.rs`, update `save_load_roundtrip()` to:
- Pass `Some(&self.driver as &dyn SaveableRuntime)` to `save_to_bytes()`
- After `load_from_bytes()`, if AI payload bytes are returned, call `driver.restore_runtime_state(&bytes)`

This means `from_simulation_state()` gains an `ai_runtime_bytes: Option<&[u8]>` parameter (or the restore is done externally after construction).

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — add `AgentTickDriverState`, impl `SaveableRuntime`)
- `crates/worldwake-ai/Cargo.toml` (modify — if `bincode` dep is missing for AI crate; likely already present transitively)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — update `save_load_roundtrip()` to use runtime save/restore)

## Out of Scope

- Post-load validation of entity references (S30-005)
- Removing the driver reset workaround in goldens (S30-006)
- Save format definition or `SaveableRuntime` trait design (S30-003)
- ExhaustionEntry struct or serde derives (S30-001, S30-002)
- Any behavioral changes to AI decision logic

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `AgentTickDriver` with multiple agent runtimes → `save_runtime_state()` → fresh driver → `restore_runtime_state()` → `runtime_by_agent` and `budget` match, `semantics_cache` is `None`, `trace_sink` is `None`
2. New focused test: `restore_runtime_state` with empty bytes (`[]`) → appropriate error returned
3. All golden tests pass with updated harness: `cargo test -p worldwake-ai`
4. `cargo clippy --workspace && cargo test --workspace`

### Invariants

1. `AgentTickDriverState` is private to `worldwake-ai` — never exposed in public API
2. Serialization is deterministic (BTreeMap ordering, no floats)
3. `semantics_cache` and `trace_sink` are always `None` after `restore_runtime_state()`
4. No new ECS components

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/mod.rs` (test module) — `test_driver_save_restore_round_trip`
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` — update `save_load_roundtrip()` and `from_simulation_state()` to wire runtime persistence

### Commands

1. `cargo test -p worldwake-ai agent_tick`
2. `cargo test -p worldwake-ai golden`
3. `cargo clippy --workspace && cargo test --workspace`
