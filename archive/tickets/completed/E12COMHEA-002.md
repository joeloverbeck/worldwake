# E12COMHEA-002: CombatProfile and DeadAt components with registration

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` authoritative components, generated component schema, delta surface, and world/component-table tests
**Deps**: E03 component registration machinery, E12COMHEA-001 (shared wound schema already landed), `specs/E12-combat-health.md`, `tickets/E12COMHEA-000-index.md`

## Problem

E12 needs two new authoritative agent components:

- `CombatProfile`: per-agent combat and bodily resilience parameters so combat and recovery behavior come from concrete per-agent state instead of hardcoded constants
- `DeadAt(Tick)`: explicit death-finality marker so later systems can exclude dead agents without archiving them

These components must be first-class registered components on `EntityKind::Agent`, exposed through the normal `World`/`WorldTxn` APIs, and serialized like the rest of authoritative state.

## Assumption Reassessment (2026-03-11)

1. `E12COMHEA-001` is already complete. `Wound`, `WoundCause::Combat`, and `CombatWeaponRef` exist today, so this ticket must not restate the old pre-E12 wound baseline.
2. The authoritative component surface is generated from `crates/worldwake-core/src/component_schema.rs`. Adding a new component there automatically affects `ComponentTables`, `World`, `WorldTxn`, `ComponentKind`, and `ComponentValue`.
3. Because `ComponentKind::ALL` and `component_samples()` are maintained explicitly in `crates/worldwake-core/src/delta.rs`, this ticket affects delta tests too. The original ticket understated that impact.
4. Existing agent-only component coverage follows a consistent pattern in:
   - `crates/worldwake-core/src/component_tables.rs`
   - `crates/worldwake-core/src/world.rs`
   - the component module itself
   The new components should extend those patterns instead of inventing bespoke tests.
5. `Permille`, `Tick`, and `NonZeroU32` already exist in `worldwake-core` and are the right scalar types here.
6. Existing per-agent profile components such as `MetabolismProfile` live in their own domain modules, not in `components.rs`. A dedicated `combat.rs` module in `worldwake-core` matches current architecture.
7. No current code defines `CombatProfile` or `DeadAt`. This is a schema addition, not a rename or cleanup of an existing path.

## Architecture Check

### Why These Changes Are Better Than The Current Architecture

1. `CombatProfile` is the right abstraction. Combat capacity, guard skill, natural clotting, and recovery should be explicit per-agent state so later systems can read the same authoritative profile instead of baking duplicate constants into combat, scheduler, healing, and AI code.
2. `DeadAt(Tick)` is cleaner than deriving death from helper functions everywhere. Fatality is caused by wound state, but once death occurs it becomes durable authoritative state with a concrete tick, which is exactly what later scheduling, looting, and event logic need.
3. Keeping these as registered components aligns with Principle 12: systems communicate through shared state, not through direct calls or side channels.

### Architectural Guardrails

1. `CombatProfile` should stay a flat data component with no cached derived values such as `wound_load`, `is_incapacitated`, or `is_dead`.
2. `DeadAt` should remain a single-purpose marker newtype around `Tick`, not a richer status enum or compatibility wrapper.
3. These types fit better in a dedicated `crates/worldwake-core/src/combat.rs` module than in `wounds.rs`. `wounds.rs` already owns the shared bodily-harm schema; `CombatProfile` and `DeadAt` are adjacent combat/life-cycle state, not wound provenance.
4. This ticket should not add compatibility aliases or alternate death representations. Later systems should adopt `DeadAt` directly.

## Revised Scope

Implement the new authoritative components and update the generated component surface and core tests that must evolve with it:

1. Add `CombatProfile` in a dedicated `worldwake-core` combat module.
2. Add `DeadAt(pub Tick)` in the same module.
3. Register both as agent-only authoritative components in `component_schema.rs`.
4. Expose them from `worldwake-core` so downstream crates can use them directly.
5. Update the explicit delta/component test fixtures that enumerate all authoritative components.
6. Extend world/component-table/component-module tests for CRUD, serialization, and kind-guard behavior.

## What to Change

### 1. Add `CombatProfile`

Create a new `worldwake-core` combat module with:

```rust
pub struct CombatProfile {
    pub wound_capacity: Permille,
    pub incapacitation_threshold: Permille,
    pub attack_skill: Permille,
    pub guard_skill: Permille,
    pub defend_bonus: Permille,
    pub natural_clot_resistance: Permille,
    pub natural_recovery_rate: Permille,
    pub unarmed_wound_severity: Permille,
    pub unarmed_bleed_rate: Permille,
    pub unarmed_attack_ticks: NonZeroU32,
}
impl Component for CombatProfile {}
```

The module should also provide local unit tests following the `needs.rs` / `wounds.rs` style.

### 2. Add `DeadAt`

```rust
pub struct DeadAt(pub Tick);
impl Component for DeadAt {}
```

Keep it minimal and authoritative.

### 3. Register both in `component_schema.rs`

Register both on `EntityKind::Agent` via the existing schema macro entries. This is the single source of truth for:

- `ComponentTables`
- world-facing insert/get/remove/count/query APIs
- transaction setters/clearers
- `ComponentKind` / `ComponentValue`

### 4. Wire module exports

Update `crates/worldwake-core/src/lib.rs` to declare and re-export the new combat module/types.

### 5. Update explicit delta/component fixtures

Because `delta.rs` keeps explicit authoritative-component coverage, update:

- `component_samples()`
- `ComponentKind::ALL` expectation
- any fixture lengths or assertions that depend on the full component set

### 6. Extend core tests where the generated surface matters

Update tests in:

- `crates/worldwake-core/src/combat.rs`
- `crates/worldwake-core/src/component_tables.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world.rs`

The original ticket’s single-file test scope was too narrow for a schema change of this kind.

## Files to Touch

- `crates/worldwake-core/src/combat.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify)

## Out of Scope

- Wound schema changes (already handled by E12COMHEA-001)
- Sword/Bow commodities and `CombatWeaponProfile` (E12COMHEA-003)
- Wound helper functions such as `is_incapacitated` / fatality derivation (E12COMHEA-006)
- Scheduler exclusion logic using `DeadAt` (E12COMHEA-008)
- Combat system, healing, or wound progression logic
- Any broader redesign of item/weapon architecture

## Acceptance Criteria

### Tests That Must Pass

1. `CombatProfile` satisfies `Component` trait bounds and round-trips through bincode.
2. `DeadAt` satisfies `Component` trait bounds and round-trips through bincode.
3. `CombatProfile` is accessible through the generated world/component-table APIs on agent entities.
4. `DeadAt` is accessible through the generated world/component-table APIs on agent entities.
5. Setting either component on a non-agent entity fails with the same invalid-operation pattern used by other agent-only components.
6. `ComponentKind::ALL` and `component_samples()` in `delta.rs` stay in sync after the schema addition.
7. `ComponentTables` serialization coverage includes the new component storage.
8. `cargo test -p worldwake-core` passes.
9. `cargo test --workspace` passes.
10. `cargo clippy --workspace --all-targets` passes.

### Invariants

1. No `f32`/`f64`; use `Permille` and `NonZeroU32` only.
2. No stored derived state in `CombatProfile`.
3. `DeadAt` is the sole authoritative death marker introduced here.
4. No compatibility aliases, wrappers, or deprecated parallel paths.
5. Registration follows the existing schema macro pattern exactly.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/combat.rs`
   - trait-bound coverage
   - field storage assertions
   - bincode round-trips for `CombatProfile` and `DeadAt`
2. `crates/worldwake-core/src/component_tables.rs`
   - empty/default iteration coverage for new tables
   - insert/get/remove and bincode coverage for both new components
3. `crates/worldwake-core/src/delta.rs`
   - explicit authoritative component enumeration stays complete
   - component sample coverage includes new variants
4. `crates/worldwake-core/src/world.rs`
   - CRUD roundtrip on agents
   - invalid operation on non-agent entities

### Commands

1. `cargo test -p worldwake-core -- combat`
2. `cargo test -p worldwake-core`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - corrected the ticket assumptions and scope before implementation so it matched the current E12 baseline, generated component architecture, and real test impact
  - added `crates/worldwake-core/src/combat.rs` with `CombatProfile`, `DeadAt`, and local serialization/trait coverage
  - registered both components in the authoritative schema and exposed them from `worldwake-core`
  - extended generated-surface coverage in `component_tables.rs`, `delta.rs`, and `world.rs`
  - updated the downstream schema expectation in `crates/worldwake-systems/tests/e09_needs_integration.rs` so the workspace acknowledged the intentional authoritative-component expansion
- Deviations from original plan:
  - the original ticket understated scope by omitting `delta.rs` and downstream schema-list tests; those had to change because the component schema is treated as a canonical explicit set in multiple places
  - kept the implementation narrow despite that extra test fallout; no combat logic, wound helpers, or scheduler changes were pulled in
- Verification results:
  - `cargo test -p worldwake-core -- combat` passed
  - `cargo test -p worldwake-core` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets` passed
