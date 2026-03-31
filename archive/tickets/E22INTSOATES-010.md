# E22INTSOATES-010: T31 — Stress with Frequent Disruptions

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: None
**Deps**: E22INTSOATES-009

## Problem

No existing test verifies that the simulation handles arbitrary mid-run disruptions gracefully. T31 injects random disruptions (agent death, item destruction, workstation removal, agent teleportation) every 100 ticks and verifies that all per-tick invariants hold, no panics occur, and save/load roundtrips produce identical hashes.

## Assumption Reassessment (2026-03-31)

1. `WorldTxn` supports all disruption types: adding `DeadAt`, removing entities, removing components, mutating placement relations — confirmed.
2. `DeterministicRng` exists for deterministic disruption selection — confirmed.
3. `save_to_bytes()` and `load_from_bytes()` exist — confirmed in `crates/worldwake-sim/src/save_load.rs`.
4. `hash_world()` exists — confirmed.
5. `EntityId` allocator uses generational slots — confirmed in `crates/worldwake-core/src/allocator.rs`. No duplicate reuse is an allocator invariant.
6. T31 reuses T30's population and topology builders — depends on 009.
7. T31 runs 2880 ticks (2 days) with disruptions every 100 ticks = 28 disruptions.
8. All T30 per-tick invariants apply to T31.
9. No adjacent contradictions.
10. T31 is `#[ignore]` like T30.

## Architecture Check

1. T31 reuses T30's world setup and per-tick invariant checking. The disruption injection uses standard `WorldTxn` mutations — no special disruption API needed. Disruption type is selected deterministically from `DeterministicRng` for reproducibility.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. All T30 per-tick invariants → same verification as 009
2. No panic → test completes without unwrap failure (implicit in test passing)
3. No duplicate EntityId reuse → allocator generation integrity (checked via allocator API or entity existence queries)
4. Save/load roundtrip fidelity → `save_to_bytes()` → `load_from_bytes()` at tick 2880 produces identical `hash_world()`
5. Determinism → state hash comparison across runs with same seed

## What to Change

### 1. Add T31 stress test to `crates/worldwake-ai/tests/golden_integration.rs`

- Reuse T30 population/topology builder
- Every 100 ticks, inject one random disruption via `WorldTxn`:
  - Kill a random living agent (add `DeadAt` component)
  - Destroy a random `ItemLot` (remove entity)
  - Remove `WorkstationTag` from a random facility
  - Teleport a random agent to a random place (via relation mutation)
- Disruption type selected deterministically from `DeterministicRng`
- Run 2880 ticks
- Check all T30 per-tick invariants at every tick
- At tick 2880: `save_to_bytes()` → `load_from_bytes()` → compare `hash_world()`
- `fn run_t31_stress(seed: Seed)` — runs single seed, panics on invariant violation
- Single `#[test]` `#[ignore]` function: `t31_stress_disruptions`

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Performance optimization
- Changes to any engine or system code
- Non-`#[ignore]` test variants

## Acceptance Criteria

### Tests That Must Pass

1. `t31_stress_disruptions` (run via `cargo test -p worldwake-ai --test golden_integration -- --ignored t31`) — 2880 ticks with 28 disruptions, zero invariant violations, no panics
2. All T30 per-tick invariants hold despite disruptions
3. No duplicate `EntityId` reuse
4. `save_to_bytes()` → `load_from_bytes()` roundtrip at tick 2880 produces identical `hash_world()`
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Simulation handles all disruption types gracefully — no panics or unwrap failures
2. Conservation holds despite item destruction (destruction is an explicit sink)
3. Allocator generation integrity — no EntityId reuse

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs::t31_stress_disruptions` — stress test with frequent disruptions

### Commands

1. `cargo test -p worldwake-ai --test golden_integration -- --ignored t31`
2. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-31
- **What changed**: Added `run_t31_stress(seed)` and `t31_stress_disruptions` (`#[test] #[ignore]`) to `crates/worldwake-ai/tests/golden_integration.rs`. Added `EntityKind` import. Fixed pre-existing bug in `build_t30_world()`: moved `BanditCamp` component from bandit agent entities to `PLACE_T30_BANDIT_CAMP` (place entity) and `BanditFactionPolicy` from bandit agents to `bandit_faction` (faction entity), aligning with component schema constraints and T22R's correct pattern.
- **Deviations**: T30's builder had stale component placement (`BanditCamp` on agents, `BanditFactionPolicy` on agents) that violated the current component schema. Fixed per Principle 28 (No Backward Compatibility). No ticket-level deviations.
- **Verification**: `cargo test -p worldwake-ai --test golden_integration -- --ignored t31` passed (456s). `cargo test -p worldwake-ai` 36/36 passed. `cargo test --workspace` all passed. `cargo clippy --workspace` clean.
