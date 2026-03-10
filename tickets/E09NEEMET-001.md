# E09NEEMET-001: Shared Phase 2 body-harm schema (Wound, WoundCause, BodyPart)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new core types, component registration
**Deps**: E08 (Phase 1 complete)

## Problem

E09 (deprivation wounds) and E12 (combat wounds) both need a shared body-harm representation. The spec mandates these types live in `worldwake-core` as shared Phase 2 schema so that starvation, dehydration, and combat all converge on the same consequence carrier. Without this, E09 cannot implement deprivation consequences.

## Assumption Reassessment (2026-03-10)

1. No `Wound`, `WoundCause`, `BodyPart`, or `WoundList` types exist in the codebase yet — confirmed by grep.
2. The IMPLEMENTATION-ORDER.md Step 7a ("Phase 2 shared schema extraction") explicitly lists "wounds / deprivation-harm schema" as a prerequisite before E09/E10/E12 can begin.
3. The component registration macro pattern in `component_schema.rs` requires 15 method names per component plus a kind predicate.

## Architecture Check

1. Placing wound types in `worldwake-core` follows the crate dependency graph — both `worldwake-systems` (E09 needs system) and future E12 combat can import from core without circular deps.
2. `WoundList` as a component on Agent entities means wounds are authoritative stored state, consistent with Principle 3.
3. No backwards-compatibility shims needed — this is greenfield.

## What to Change

### 1. New module `crates/worldwake-core/src/wounds.rs`

Define the shared body-harm types:

```rust
/// Body part targeted by a wound.
pub enum BodyPart {
    Head,
    Torso,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

/// What caused a wound.
pub enum WoundCause {
    Deprivation(DeprivationKind),
    // E12 will add: Combat(CombatWoundSource), etc.
}

pub enum DeprivationKind {
    Starvation,
    Dehydration,
}

/// A single wound on an agent's body.
pub struct Wound {
    pub body_part: BodyPart,
    pub cause: WoundCause,
    pub severity: Permille,
    pub inflicted_at: Tick,
}

/// Authoritative list of wounds on an agent.
pub struct WoundList {
    pub wounds: Vec<Wound>,
}
impl Component for WoundList {}
```

All types must derive: `Clone, Debug, Serialize, Deserialize`. Enum types also derive `Copy, Eq, PartialEq, Ord, PartialOrd, Hash`.

### 2. Register `WoundList` in `component_schema.rs`

Add a new block to the `with_authoritative_components!` macro for `WoundList` on `EntityKind::Agent`.

### 3. Export from `crates/worldwake-core/src/lib.rs`

Add `pub mod wounds;` and re-export key types.

## Files to Touch

- `crates/worldwake-core/src/wounds.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify — add WoundList registration)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Wound progression / healing logic (E12)
- Combat wound sources (E12)
- Pain derivation from wounds (E13)
- Wound effects on action capacity (E12)
- Any fear/danger scoring

## Acceptance Criteria

### Tests That Must Pass

1. `WoundList` can be inserted, retrieved, and removed on Agent entities via component table methods.
2. `WoundList` insertion is rejected for non-Agent entity kinds.
3. `Wound` with `WoundCause::Deprivation(Starvation)` can be constructed and serialized round-trip (bincode).
4. `BodyPart`, `WoundCause`, `DeprivationKind` derive all required traits (`Copy, Clone, Eq, Ord, Hash, Debug, Serialize, Deserialize`).
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All types are deterministic (`Ord` + no `HashMap`/`HashSet`).
2. `WoundList` is authoritative stored state per Principle 3.
3. No floating-point types used.
4. Component kind predicate restricts `WoundList` to `EntityKind::Agent`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/wounds.rs` (unit tests) — construction, serialization, trait bounds
2. `crates/worldwake-core/tests/` or inline — component table integration for WoundList

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
