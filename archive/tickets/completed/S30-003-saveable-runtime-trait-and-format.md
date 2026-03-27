# S30-003: SaveableRuntime trait and save format extension

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — save/load format, new trait in worldwake-sim, SAVE_FORMAT_VERSION bump
**Deps**: None (sim-crate only; independent of S30-001/002)

## Problem

`worldwake-sim` cannot reference `worldwake-ai` types due to the crate dependency direction (`ai → sim`, not `sim → ai`). To persist AI runtime state across save/load, the sim crate needs an opaque bytes interface. The save format must also be extended to carry an auxiliary AI payload alongside the existing `SimulationState` payload.

## Assumption Reassessment (2026-03-27)

Shared abstraction boundary under audit: `worldwake_sim::save_load::{save_to_bytes, load_from_bytes, save, load}` as the crate-boundary transport for an opaque autonomous-controller runtime payload, with `worldwake_ai::AgentTickDriver` as the first concrete runtime user.

1. `SAVE_FORMAT_VERSION` is currently `5` in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs#L6). Current format is exactly `[magic][version][bincode SimulationState payload]`; there is no length prefix on the sim payload.
2. The live error type is `SaveError`, not `SaveLoadError`, in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs#L11). This ticket must use the real type name and extend it rather than inventing a parallel error enum.
3. `save_to_bytes(state: &SimulationState) -> Result<Vec<u8>, SaveError>` and `load_from_bytes(bytes: &[u8]) -> Result<SimulationState, SaveError>` are the current signatures in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs#L66) and [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs#L76).
4. `save(state, path)` and `load(path)` are live file wrappers in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs#L56). Real consumers exist in [crates/worldwake-cli/src/handlers/persistence.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs), not `crates/worldwake-cli/src/main.rs`.
5. `AutonomousController` currently exposes only `name()`, `claims_agent()`, and `produce_agent_input()` in [crates/worldwake-sim/src/autonomous_controller.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/autonomous_controller.rs#L17). No persistence hook exists yet. A separate `SaveableRuntime` trait remains the right scope because persistence is optional controller behavior, not a requirement for every autonomous controller implementation.
6. The AI-side runtime promotion work described by the S30 spec is already substantially landed. `AgentDecisionRuntime`, `MaterializationBindings`, and `ExhaustionEntry` already exist and derive serde in [crates/worldwake-ai/src/decision_runtime.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs#L28). The ticket should not re-scope that already-delivered architecture.
7. The golden harness still saves only `SimulationState` and rebuilds a fresh driver in [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs#L1160) and [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs#L1168). The CLI load path likewise resets the driver after load in [crates/worldwake-cli/src/handlers/persistence.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/persistence.rs#L38).
8. v5 compatibility cannot be handled by reading an `ai_payload_len == 0` sentinel because v5 saves do not contain a sim-payload length field at all. The loader must branch on version first: parse version 5 with the legacy `[magic][version][sim payload]` layout, and parse version 6 with the new length-prefixed layout.
9. Adjacent contradictions classified during reassessment:
   - Required in-scope consequence: golden harness and CLI persistence call sites must be updated because they currently encode the “fresh driver after load” behavior.
   - Already-delivered adjacent work, out of scope here: runtime serde promotion and exhaustion-cache unification already exist in `worldwake-ai`.

## Architecture Check

1. Opaque runtime bytes are still the cleanest architecture. `worldwake-sim` remains ignorant of AI internals and only transports bytes across the save boundary, which preserves the crate dependency direction and Principle 24.
2. A dedicated `SaveableRuntime` trait is cleaner than adding persistence methods directly to `AutonomousController`. Persistence is not part of the minimum controller contract, and forcing every autonomous controller to implement save/restore would couple unrelated behavior into the dispatch interface.
3. The v6 format should be length-prefixed for both payloads. That gives deterministic framing, supports additional payloads later, and avoids “read rest of file” ambiguity.
4. Supporting legacy v5 loads via an explicit version branch is justified migration work, not a backward-compatibility alias path. The canonical format after this ticket is v6 only; v5 handling exists solely at the boundary to read old saves.

## Verification Layers

1. Save-format framing and legacy v5 migration -> focused `worldwake-sim` unit tests over `save_to_bytes`/`load_from_bytes`
2. AI runtime continuity across save/load -> focused `worldwake-ai` golden determinism coverage using the real harness resume path
3. CLI persistence semantics -> focused `worldwake-cli` handler/integration tests proving load restores state and preserved AI runtime instead of resetting it
4. Authoritative continuation after resume -> existing resumed-vs-uninterrupted golden assertions on scheduler, RNG, world hash, and event-log hash

## What to Change

### 1. Define `SaveableRuntime` trait

Add to `crates/worldwake-sim/src/saveable_runtime.rs` (new file):

```rust
use crate::save_load::SaveError;

/// Trait for autonomous controllers that support state persistence
/// across save/load boundaries. The state is serialized as opaque bytes
/// to preserve the crate dependency boundary (sim cannot depend on ai).
pub trait SaveableRuntime {
    fn save_runtime_state(&self) -> Result<Vec<u8>, SaveError>;
    fn restore_runtime_state(&mut self, bytes: &[u8]) -> Result<(), SaveError>;
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
) -> Result<Vec<u8>, SaveError>
```

- Serialize `SimulationState` via bincode → get sim payload bytes
- If `runtime` is `Some`, call `runtime.save_runtime_state()` → get AI payload bytes
- Write header + length-prefixed sim payload + length-prefixed AI payload (or `ai_payload_len = 0`)

### 4. Update `load_from_bytes()` return type

```rust
pub fn load_from_bytes(bytes: &[u8]) -> Result<(SimulationState, Option<Vec<u8>>), SaveError>
```

- Read header and branch by version
- For version 5: deserialize the remaining bytes as the legacy `SimulationState` payload and return `None`
- For version 6: read `sim_payload_len`, deserialize `SimulationState`, then read `ai_payload_len` and return `Some(ai_payload)` if len > 0
- Reject trailing/truncated v6 framing explicitly instead of silently accepting malformed payloads

### 5. Update file-based `save()` and `load()`

Update signatures to match the bytes-based functions:
- `save()` gains `runtime: Option<&dyn SaveableRuntime>`
- `load()` returns `(SimulationState, Option<Vec<u8>>)`

### 6. Bump `SAVE_FORMAT_VERSION` from 5 to 6

### 7. Add `SaveLoadError` variants if needed

Ensure `SaveError` has variants for runtime serialization/deserialization failures (for example `RuntimeSerialization` / `RuntimeDeserialization`) rather than overloading the existing simulation-state messages.

### 8. Wire the first real consumer through AI and CLI

- Implement `SaveableRuntime` for `worldwake_ai::AgentTickDriver`
- Add a post-load validation hook on `AgentTickDriver` that prunes stale runtime references and reinitializes derived fields against the loaded world
- Update the golden harness roundtrip helpers to save and restore the driver instead of reconstructing a fresh one
- Update CLI persistence handlers to save the active driver and restore it on load instead of resetting to `AgentTickDriver::new(PlanningBudget::default())`

## Files to Touch

- `crates/worldwake-sim/src/saveable_runtime.rs` (new — trait definition)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-export)
- `crates/worldwake-sim/src/save_load.rs` (modify — format extension, version bump, updated signatures)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — implement runtime save/restore and post-load validation)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — update `save_load_roundtrip()` helper to pass `None` for runtime and handle new return type)
- `crates/worldwake-cli/src/handlers/persistence.rs` (modify — save/load driver runtime at the real call site)
- `crates/worldwake-cli/tests/integration.rs` (modify if needed — assert resumed ticking still works via the real CLI flow)

## Out of Scope

- Reworking already-landed AI runtime serde or exhaustion-cache architecture
- Changing any AI decision logic
- Backward-compatible loading of v4 or earlier saves (only v5→v6 transition)

## Acceptance Criteria

### Tests That Must Pass

1. New focused test: `save_to_bytes(state, None)` produces loadable v6 bytes and `load_from_bytes` returns `(state, None)`
2. New focused test: `save_to_bytes(state, Some(mock_runtime))` round-trips an exact opaque runtime payload
3. New focused test: legacy v5 bytes still load as `(state, None)` via the explicit version-5 path
4. New focused test: malformed v6 framing is rejected with a deterministic `SaveError`
5. Golden save/load determinism coverage passes with restored driver runtime rather than a fresh driver
6. CLI persistence coverage passes with the updated save/load signatures
7. `cargo clippy --workspace && cargo test --workspace`

### Invariants

1. `worldwake-sim` has zero knowledge of `worldwake-ai` types — opaque bytes only
2. Save format is deterministic (identical state + runtime → identical bytes)
3. Old v5 saves load without error through an explicit version-5 migration branch
4. `SAVE_FORMAT_VERSION == 6`
5. No new ECS components

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/save_load.rs` (test module) — format round-trip tests with and without runtime payload, legacy v5 load coverage, malformed v6 framing coverage
2. `crates/worldwake-ai/src/agent_tick/mod.rs` or nearby focused AI runtime tests — runtime save/restore and post-load validation coverage
3. `crates/worldwake-ai/tests/golden_harness/mod.rs` / `crates/worldwake-ai/tests/golden_determinism.rs` — resumed golden flow now restores the driver rather than resetting it
4. `crates/worldwake-cli/src/handlers/persistence.rs` and/or `crates/worldwake-cli/tests/integration.rs` — save/load preserves CLI-driven simulation continuity with restored AI runtime

### Commands

1. `cargo test -p worldwake-sim save_load`
2. `cargo test -p worldwake-ai golden_save_load`
3. `cargo test -p worldwake-cli test_save_load_roundtrip`
4. `cargo clippy --workspace`
5. `cargo test --workspace`

## Outcome

Completion date: 2026-03-27

What actually changed:
- Added `worldwake_sim::SaveableRuntime` and extended the save format to v6 with length-prefixed simulation and runtime payloads.
- Kept explicit legacy v5 loading support via version-based parsing.
- Implemented runtime persistence and post-load validation for `worldwake_ai::AgentTickDriver`.
- Updated the golden harness and CLI persistence flow to restore the saved AI runtime instead of rebuilding a fresh driver.
- Promoted `bincode` to a normal `worldwake-ai` dependency because runtime serialization now happens in production code, not just tests.

Deviations from original plan:
- The ticket was corrected before implementation because the AI-side runtime serde and exhaustion-cache unification were already landed; this work narrowed to the actual remaining boundary integration.
- The real CLI call sites were `crates/worldwake-cli/src/handlers/persistence.rs` and `crates/worldwake-cli/src/handlers/mod.rs`, not `crates/worldwake-cli/src/main.rs`.
- The golden harness was updated by making `save_load_roundtrip()` restore the driver directly, rather than changing `from_simulation_state()` into a save/load-specific API.

Verification results:
- `cargo test -p worldwake-sim save_load`
- `cargo test -p worldwake-ai saveable_runtime_roundtrip_restores_persisted_driver_state`
- `cargo test -p worldwake-ai post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty`
- `cargo test -p worldwake-ai golden_save_load`
- `cargo test -p worldwake-ai golden_save_load_preserves_promoted_commitments`
- `cargo test -p worldwake-cli test_save_load_roundtrip`
- `cargo clippy --workspace`
- `cargo test --workspace`
