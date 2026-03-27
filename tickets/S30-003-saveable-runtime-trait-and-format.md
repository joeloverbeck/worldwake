# S30-003: SaveableRuntime trait and save format extension

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — save/load format, new trait in worldwake-sim, SAVE_FORMAT_VERSION bump
**Deps**: None (sim-crate only; independent of S30-001/002)

## Problem

`worldwake-sim` cannot reference `worldwake-ai` types due to the crate dependency direction (`ai → sim`, not `sim → ai`). To persist AI runtime state across save/load, the sim crate needs an opaque bytes interface. The save format must also be extended to carry an auxiliary AI payload alongside the existing `SimulationState` payload.

## Assumption Reassessment (2026-03-27)

1. `SAVE_FORMAT_VERSION` is currently `5` (`save_load.rs:6`). Last bumped by S21.
2. Current save format: `[WWAK magic: 4 bytes][version: 4 bytes u32-LE][bincode SimulationState payload]` — no length-prefix on the sim payload; it consumes the rest of the file/buffer.
3. `save_to_bytes(state: &SimulationState) -> Vec<u8>` at `save_load.rs:66-73`. `load_from_bytes(bytes: &[u8]) -> Result<SimulationState, SaveLoadError>` at `save_load.rs:56-64`.
4. `save(state, path)` and `load(path)` are file-based wrappers.
5. `AutonomousController` trait at `autonomous_controller.rs:18-30` — currently has `name()`, `claims_agent()`, `produce_agent_input()`. The `SaveableRuntime` trait is separate from `AutonomousController` (not all controllers need persistence).
6. `SaveLoadError` exists in `save_load.rs` — confirm it has enough variants for runtime serialization errors.
7. The golden harness `save_load_roundtrip()` helper at `golden_harness/mod.rs` uses `save_to_bytes`/`load_from_bytes` — it will need updating to pass/receive the AI payload.

## Architecture Check

1. Opaque bytes is the correct pattern: `worldwake-sim` stores/retrieves raw bytes without knowing AI types. This preserves crate boundary (Principle 24).
2. Length-prefixed format allows detecting presence/absence of AI payload for backward compatibility.
3. No backward-compatibility shims — old v5 saves load cleanly because `ai_payload_len == 0` (or EOF after sim payload) triggers fresh driver start.

## Verification Layers

1. Format backward compat → focused test: v5-format bytes (no AI section) load without error, returning `None` for AI payload
2. Format round-trip → focused test: save with AI payload → load → AI payload bytes match
3. Single-layer ticket (sim crate serialization format) — no cross-layer mapping needed.

## What to Change

### 1. Define `SaveableRuntime` trait

Add to `crates/worldwake-sim/src/saveable_runtime.rs` (new file):

```rust
use crate::save_load::SaveLoadError;

/// Trait for autonomous controllers that support state persistence
/// across save/load boundaries. The state is serialized as opaque bytes
/// to preserve the crate dependency boundary (sim cannot depend on ai).
pub trait SaveableRuntime {
    fn save_runtime_state(&self) -> Result<Vec<u8>, SaveLoadError>;
    fn restore_runtime_state(&mut self, bytes: &[u8]) -> Result<(), SaveLoadError>;
}
```

Export from `crates/worldwake-sim/src/lib.rs`.

### 2. Extend save format to length-prefixed sim + optional AI payload

New format:
```
[magic: 4 bytes "WWAK"]
[version: 4 bytes u32-LE]
[sim_payload_len: 8 bytes u64-LE]    ← NEW
[sim_payload: N bytes]
[ai_payload_len: 8 bytes u64-LE]     ← NEW
[ai_payload: M bytes]                 ← NEW (0 bytes if no runtime)
```

### 3. Update `save_to_bytes()` signature

```rust
pub fn save_to_bytes(
    state: &SimulationState,
    runtime: Option<&dyn SaveableRuntime>,
) -> Result<Vec<u8>, SaveLoadError>
```

- Serialize `SimulationState` via bincode → get sim payload bytes
- If `runtime` is `Some`, call `runtime.save_runtime_state()` → get AI payload bytes
- Write header + length-prefixed sim payload + length-prefixed AI payload (or `ai_payload_len = 0`)

### 4. Update `load_from_bytes()` return type

```rust
pub fn load_from_bytes(bytes: &[u8]) -> Result<(SimulationState, Option<Vec<u8>>), SaveLoadError>
```

- Read header, validate magic + version
- Read `sim_payload_len`, deserialize `SimulationState`
- If remaining bytes exist, read `ai_payload_len` → return `Some(ai_payload)` if len > 0, else `None`
- If no remaining bytes (v5 compat), return `None`

### 5. Update file-based `save()` and `load()`

Update signatures to match the bytes-based functions:
- `save()` gains `runtime: Option<&dyn SaveableRuntime>`
- `load()` returns `(SimulationState, Option<Vec<u8>>)`

### 6. Bump `SAVE_FORMAT_VERSION` from 5 to 6

### 7. Add `SaveLoadError` variants if needed

Ensure `SaveLoadError` has variants for runtime serialization/deserialization failures (e.g., `RuntimeSerializationError`, `RuntimeDeserializationError`).

## Files to Touch

- `crates/worldwake-sim/src/saveable_runtime.rs` (new — trait definition)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-export)
- `crates/worldwake-sim/src/save_load.rs` (modify — format extension, version bump, updated signatures)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — update `save_load_roundtrip()` helper to pass `None` for runtime and handle new return type)
- `crates/worldwake-cli/src/main.rs` (modify — if CLI calls save/load directly, update call sites)

## Out of Scope

- Implementing `SaveableRuntime` for `AgentTickDriver` (S30-004)
- Serde derives on AI types (S30-002)
- Post-load validation (S30-005)
- ExhaustionEntry unification (S30-001)
- Changing any AI decision logic
- Backward-compatible loading of v4 or earlier saves (only v5→v6 transition)

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `save_to_bytes` with `runtime: None` produces loadable format; `load_from_bytes` returns `(state, None)`
2. New focused test: `save_to_bytes` with mock `SaveableRuntime` producing known bytes → `load_from_bytes` returns `(state, Some(expected_bytes))`
3. New focused test: v5-format bytes (no AI section, no sim length prefix) → `load_from_bytes` returns `(state, None)` without error — OR document that v5 compat is handled by version check + fresh driver
4. All golden tests pass (with updated harness passing `None`): `cargo test -p worldwake-ai`
5. `cargo clippy --workspace && cargo test --workspace`

### Invariants

1. `worldwake-sim` has zero knowledge of `worldwake-ai` types — opaque bytes only
2. Save format is deterministic (identical state + runtime → identical bytes)
3. Old saves without AI payload load without error
4. `SAVE_FORMAT_VERSION == 6`
5. No new ECS components

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs` (test module) — format round-trip tests with and without AI payload
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` — update `save_load_roundtrip()` to use new signatures (pass `None` for runtime initially)

### Commands

1. `cargo test -p worldwake-sim save_load`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace && cargo test --workspace`
