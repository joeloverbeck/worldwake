# S52EVIDAFT-003: Evidence decay system

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new SystemId variant, new system function
**Deps**: S52EVIDAFT-001, S52EVIDAFT-002

## Problem

Evidence entries accumulate on Place entities but never decay. Without a decay system, evidence persists forever, violating the design goal that evidence represents physical aftermath that fades through natural processes.

## Assumption Reassessment (2026-04-05)

1. `SystemId` is defined via `define_system_ids!` in `crates/worldwake-sim/src/system_manifest.rs`, but the authoritative per-tick sequencing now lives in `SystemManifest::canonical()`, while `SystemManifest::pre_action()` is reserved for pre-input transitions such as `ArtifactLifecycle`. `EvidenceDecay` belongs in the canonical per-tick order, not the pre-action manifest.
2. The dense dispatch surface is `SystemId::ALL` plus `worldwake-systems` `dispatch_table()` in `crates/worldwake-systems/src/lib.rs`. Adding a new closed-system variant requires updating both.
3. The canonical ordering should place `EvidenceDecay` after `Perception` and before `Patrol`, so same-tick observers can still perceive newly emitted evidence before later cleanup starts.
4. `SceneEvidence` on places stores `Vec<EvidenceEntry>` with `created_at: Tick` and `decay_ticks: u32` per entry. The live execution context field is `ctx.tick`, not `ctx.current_tick`. Decay condition remains `tick - created_at >= decay_ticks`.
5. Default decay rates still come from emission-time entries owned by `S52EVIDAFT-002`; this ticket only enforces stored decay timing, not per-kind policy.

## Architecture Check

1. Decay system is a pure state-transformation system — reads current tick and SceneEvidence, removes expired entries, cleans up empty components. No cross-system calls per P26.
2. Decay is monotonic — entries only age, never refresh. This prevents evidence from becoming permanent anchors.
3. No backward-compatibility shims.

## Verification Layers

1. Evidence entry removed when `current_tick - created_at >= decay_ticks` → authoritative world state assertion
2. Evidence entries with `current_tick - created_at < decay_ticks` survive → authoritative world state
3. SceneEvidence component removed from place when all entries expired → authoritative world state
4. System ordering: decay runs after Perception and before Patrol in `SystemManifest::canonical()` → system manifest ordering test
5. Single-layer ticket (decay system only) — no cross-system verification beyond ordering.

## What to Change

### 1. Add SystemId::EvidenceDecay

In `crates/worldwake-sim/src/system_manifest.rs`:
- Add `(EvidenceDecay, "evidence_decay")` to `define_system_ids!` macro.
- Add to canonical ordering: after Perception and before Patrol.

### 2. Implement evidence_decay_system

Create `crates/worldwake-systems/src/evidence_decay.rs`:

```rust
pub fn evidence_decay_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError> {
    let current_tick = ctx.tick;
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

## Outcome

Implemented the evidence decay system on the live scheduler and dispatch surfaces.

- Added `SystemId::EvidenceDecay` and inserted it into `SystemManifest::canonical()` after `Perception` and before `Patrol` in `crates/worldwake-sim/src/system_manifest.rs`.
- Added `crates/worldwake-systems/src/evidence_decay.rs` with `evidence_decay_system`, which removes expired `SceneEvidence` entries at `tick - created_at >= decay_ticks`, preserves surviving entries without resetting `next_entry_id`, and clears the component when the final entry expires.
- Registered the new system in `crates/worldwake-systems/src/lib.rs`.
- Added focused proof for exact expiry boundaries, selective multi-entry decay, component cleanup, dispatch routing, and canonical ordering.

Verification completed:

1. `cargo test -p worldwake-systems evidence_decay_system_ -- --nocapture`
2. `cargo test -p worldwake-systems dispatch_table_routes_evidence_decay_system -- --nocapture`
3. `cargo test -p worldwake-sim canonical_manifest_matches_fixed_scheduler_order -- --nocapture`
4. `cargo test -p worldwake-sim system_id_all_matches_canonical_variant_order -- --nocapture`
5. `cargo test -p worldwake-systems`
6. `cargo test -p worldwake-sim`
7. `cargo test --workspace`
8. `cargo clippy --workspace --all-targets -- -D warnings`
