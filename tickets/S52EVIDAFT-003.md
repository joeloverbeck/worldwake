# S52EVIDAFT-003: Evidence decay system

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new SystemId variant, new system function
**Deps**: S52EVIDAFT-001

## Problem

Evidence entries accumulate on Place entities but never decay. Without a decay system, evidence persists forever, violating the design goal that evidence represents physical aftermath that fades through natural processes.

## Assumption Reassessment (2026-04-05)

1. `SystemId` defined via `define_system_ids!` macro at `crates/worldwake-sim/src/system_manifest.rs:52-63`. Currently 10 variants. Adding `EvidenceDecay` follows the same pattern.
2. System dispatch table at `crates/worldwake-systems/src/lib.rs:68-80`. New system added to handlers array.
3. Canonical system ordering at `system_manifest.rs:97+`. EvidenceDecay should run after Perception (so evidence created this tick is perceivable before decay runs) and before next tick's action systems.
4. `SceneEvidence` component on Place entities contains `Vec<EvidenceEntry>` with `created_at: Tick` and `decay_ticks: u32` per entry. Decay condition: `current_tick - created_at >= decay_ticks`.
5. Default decay rates per spec: ContainerTampered 200, BloodTrail 100, DisturbanceMarker 50, MovementTrace 30.

## Architecture Check

1. Decay system is a pure state-transformation system — reads current tick and SceneEvidence, removes expired entries, cleans up empty components. No cross-system calls per P26.
2. Decay is monotonic — entries only age, never refresh. This prevents evidence from becoming permanent anchors.
3. No backward-compatibility shims.

## Verification Layers

1. Evidence entry removed when `current_tick - created_at >= decay_ticks` → authoritative world state assertion
2. Evidence entries with `current_tick - created_at < decay_ticks` survive → authoritative world state
3. SceneEvidence component removed from place when all entries expired → authoritative world state
4. System ordering: decay runs after Perception → system manifest ordering test
5. Single-layer ticket (decay system only) — no cross-system verification beyond ordering.

## What to Change

### 1. Add SystemId::EvidenceDecay

In `crates/worldwake-sim/src/system_manifest.rs`:
- Add `(EvidenceDecay, "evidence_decay")` to `define_system_ids!` macro.
- Add to canonical ordering: after Perception.

### 2. Implement evidence_decay_system

Create `crates/worldwake-systems/src/evidence_decay.rs`:

```rust
pub fn evidence_decay_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let current_tick = ctx.current_tick;
    // Iterate all entities with SceneEvidence
    // For each: retain only entries where current_tick - created_at < decay_ticks
    // If no entries remain, remove the SceneEvidence component
}
```

### 3. Register in dispatch table

In `crates/worldwake-systems/src/lib.rs`:
- Add `pub mod evidence_decay;`
- Add `evidence_decay_system` to `dispatch_table()` handlers array.

## Files to Touch

- `crates/worldwake-sim/src/system_manifest.rs` (modify)
- `crates/worldwake-systems/src/evidence_decay.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify)

## Out of Scope

- Evidence emission — ticket 002
- Evidence perception — ticket 004
- Golden tests — ticket 005
- Weather-based or traffic-based variable decay rates (spec uses fixed per-kind rates for now)

## Acceptance Criteria

### Tests That Must Pass

1. Evidence entry removed at exactly `created_at + decay_ticks` tick
2. Evidence entries with remaining decay time survive
3. SceneEvidence component removed from place when last entry decays
4. Multiple entries on same place decay independently
5. Existing suite: `cargo test --workspace`

### Invariants

1. Decay is monotonic — entries only age, never refresh
2. Empty SceneEvidence components are cleaned up — no ghost components on places
3. Decay is tick-based, not wall-clock-based — correct across save/load

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/evidence_decay.rs` — Unit tests for single-entry decay, multi-entry independent decay, component cleanup on empty

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
