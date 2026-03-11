# E12COMHEA-002: CombatProfile and DeadAt components with registration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-core components + component_schema + component_tables
**Deps**: E03 (component registration machinery)

## Problem

E12 requires two new components registered on `EntityKind::Agent`:
- `CombatProfile`: per-agent combat/bodily resilience parameters (Principle 11 — no magic numbers)
- `DeadAt(Tick)`: marks the tick at which an agent died (death finality, invariant 9.14)

## Assumption Reassessment (2026-03-11)

1. Component registration uses macro in `component_schema.rs` — confirmed, pattern well-established.
2. `component_tables.rs` uses `define_component_tables!` macro — confirmed.
3. `EntityKind::Agent` already accepts `WoundList`, `HomeostaticNeeds`, `MetabolismProfile`, etc. — confirmed.
4. `Permille` and `Tick` types exist in `worldwake-core` — confirmed.
5. `NonZeroU32` is already used in existing components (e.g., `MetabolismProfile`) — confirmed.

## Architecture Check

1. Both components live in `worldwake-core` because they are authoritative state read by multiple crates.
2. `CombatProfile` is a flat struct with all `Permille`/`NonZeroU32` fields — no derived state stored.
3. `DeadAt` is a newtype around `Tick` — minimal, precise, queryable.

## What to Change

### 1. Create `CombatProfile` struct

New file or section in an existing module (follow project pattern for component placement):

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

### 2. Create `DeadAt` struct

```rust
pub struct DeadAt(pub Tick);
impl Component for DeadAt {}
```

### 3. Register both in component_schema.rs

Add entries to `define_component_schema!` macro for `CombatProfile` and `DeadAt`, both on `EntityKind::Agent`.

### 4. Add to component_tables.rs

Add entries to `define_component_tables!` macro.

### 5. Re-export from world.rs / lib.rs

Ensure both types are publicly accessible from `worldwake_core`.

## Files to Touch

- `crates/worldwake-core/src/combat.rs` (new — CombatProfile, DeadAt definitions)
- `crates/worldwake-core/src/component_schema.rs` (modify — register both)
- `crates/worldwake-core/src/component_tables.rs` (modify — add storage)
- `crates/worldwake-core/src/lib.rs` (modify — declare module, re-export)

## Out of Scope

- Wound struct changes (E12COMHEA-001)
- Sword/Bow commodities (E12COMHEA-003)
- Action definitions (E12COMHEA-010/011/012/013)
- Wound helper functions (E12COMHEA-006)
- Scheduler DeadAt exclusion logic (E12COMHEA-008)
- Any combat system logic

## Acceptance Criteria

### Tests That Must Pass

1. `CombatProfile` satisfies `Component` trait bounds (`Clone + Debug + Serialize + Deserialize`)
2. `DeadAt` satisfies `Component` trait bounds
3. `CombatProfile` round-trips through bincode
4. `DeadAt` round-trips through bincode
5. Can set/get/remove `CombatProfile` on an Agent entity via World API
6. Can set/get/remove `DeadAt` on an Agent entity via World API
7. Setting `CombatProfile` on non-Agent entity fails appropriately
8. Setting `DeadAt` on non-Agent entity fails appropriately
9. Different `CombatProfile` values produce distinct serialized bytes
10. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. No `f32`/`f64` — all numeric fields use `Permille` or `NonZeroU32`
2. No stored derived state (wound_load, is_incapacitated, etc. are NOT fields)
3. Component registration follows existing macro pattern exactly
4. All new types derive required traits per project conventions

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/combat.rs` — trait bounds, bincode roundtrip, World API CRUD

### Commands

1. `cargo test -p worldwake-core -- combat`
2. `cargo test --workspace && cargo clippy --workspace`
