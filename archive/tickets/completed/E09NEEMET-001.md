# E09NEEMET-001: Shared Phase 2 body-harm schema (Wound, WoundCause, BodyPart, WoundList)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new core types, component registration
**Deps**: E08 (Phase 1 complete)

## Problem

E09 (deprivation wounds) and E12 (combat wounds) both need a shared body-harm representation. The spec mandates these types live in `worldwake-core` as shared Phase 2 schema so that starvation, dehydration, and combat all converge on the same consequence carrier. Without this, E09 cannot implement deprivation consequences.

## Assumption Reassessment (2026-03-10)

1. No `Wound`, `WoundCause`, `BodyPart`, `DeprivationKind`, or `WoundList` types exist in the codebase yet — confirmed against the current workspace.
2. The current `worldwake-core` architecture does not treat new authoritative components as local-only additions. The inventory of authoritative components is split across:
   - `component_schema.rs` for typed world/table APIs
   - `delta.rs` for `ComponentKind` / `ComponentValue`
   - `world_txn.rs` for create-time component delta capture
   - `verification.rs` for event-log-to-world coverage checks
3. Kind restrictions are not enforced by `ComponentTables`; they are enforced by `World`'s generated typed insertion API in `crates/worldwake-core/src/world.rs`.
4. The crate already has strong test patterns for new authoritative components:
   - component storage behavior in `component_tables.rs`
   - kind-restricted world insertion and query behavior in `world.rs`
   - component delta inventory coverage in `delta.rs`
   - event-log/world-state reconciliation in `verification.rs`
5. The IMPLEMENTATION-ORDER prerequisite remains valid: Phase 2 needs a shared wound schema in `worldwake-core` before E09 deprivation harm and E12 combat harm can converge on a common consequence carrier.

## Architecture Check

1. Placing wound types in `worldwake-core` follows the crate dependency graph — both `worldwake-systems` (E09 needs system) and future E12 combat can import from core without circular deps.
2. `WoundList` as a component on Agent entities means wounds are authoritative stored state, consistent with Principle 3.
3. A dedicated `wounds.rs` module is cleaner than overloading `components.rs`. `components.rs` currently holds Phase 1 generic components (`Name`, `AgentData`), while items, topology, relations, and other domains already live in focused modules.
4. `WoundCause::Deprivation(DeprivationKind)` is more extensible than encoding starvation/dehydration directly into `Wound` fields. It preserves a single harm carrier while leaving space for later combat and environmental causes without aliases or compatibility shims.
5. The current architecture has one weakness worth calling out: authoritative component inventories are duplicated manually in several files. For this ticket, the robust path is to wire `WoundList` through every required inventory so it is first-class everywhere. A deeper deduplication refactor could come later, but this ticket should not leave wounds as a partial special case.
6. No backwards-compatibility shims, aliases, or temporary parallel schemas are justified here.

## Scope Correction

This ticket should:

1. Add the shared wound domain types in `worldwake-core`.
2. Register `WoundList` as an authoritative agent-only component in the same way as other core components.
3. Extend component delta typing and world-state verification so wounds are first-class in auditing and event-log coverage.
4. Add or extend tests following the current `worldwake-core` patterns.

This ticket should not:

1. Implement deprivation progression, healing, pain derivation, or combat logic.
2. Introduce compatibility aliases or temporary duplicate harm models.
3. Refactor the entire authoritative-component architecture beyond what is needed to make `WoundList` first-class and well-tested.

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

### 4. Extend authoritative component inventories

Update the explicit inventories that mirror authoritative components so `WoundList` participates in the same infrastructure as existing components:

- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-core/src/world_txn.rs`
- `crates/worldwake-core/src/verification.rs`

## Files to Touch

- `crates/worldwake-core/src/wounds.rs` (new)
- `crates/worldwake-core/src/component_schema.rs` (modify — add WoundList registration)
- `crates/worldwake-core/src/lib.rs` (modify — add module + re-exports)
- `crates/worldwake-core/src/delta.rs` (modify — add `WoundList` component kind/value coverage)
- `crates/worldwake-core/src/world.rs` (modify — generated API / tests exercised through new schema entry)
- `crates/worldwake-core/src/world_txn.rs` (modify — include `WoundList` in component delta capture)
- `crates/worldwake-core/src/verification.rs` (modify — include `WoundList` in actual-world component collection)

## Out of Scope

- Wound progression / healing logic (E12)
- Combat wound sources (E12)
- Pain derivation from wounds (E13)
- Wound effects on action capacity (E12)
- Any fear/danger scoring

## Acceptance Criteria

### Tests That Must Pass

1. `WoundList` can be inserted, retrieved, and removed in `ComponentTables`.
2. `WoundList` insertion through `World` is accepted for `EntityKind::Agent` and rejected for non-agent kinds.
3. `WoundList` participates in world query/count helpers the same way existing authoritative components do.
4. `Wound` with `WoundCause::Deprivation(DeprivationKind::Starvation)` round-trips through bincode.
5. `BodyPart`, `WoundCause`, and `DeprivationKind` satisfy the required enum trait bounds.
6. `delta.rs` recognizes `WoundList` as an authoritative component kind/value.
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. All types are deterministic (`Ord` + no `HashMap`/`HashSet`).
2. `WoundList` is authoritative stored state per Principle 3.
3. No floating-point types used.
4. Component kind predicate restricts `WoundList` to `EntityKind::Agent`.
5. Wounds are first-class in authoritative auditing; they are not invisible to component delta typing or verification.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/wounds.rs` — construction, serialization, trait bounds
2. `crates/worldwake-core/src/component_tables.rs` — table CRUD coverage for `WoundList`
3. `crates/worldwake-core/src/world.rs` — world insertion/query/count behavior and wrong-kind rejection
4. `crates/worldwake-core/src/delta.rs` — authoritative component kind/value inventory coverage updated for `WoundList`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

Completion date: 2026-03-10

What actually changed:

1. Added `crates/worldwake-core/src/wounds.rs` with `BodyPart`, `DeprivationKind`, `WoundCause`, `Wound`, and `WoundList`.
2. Registered `WoundList` as an agent-only authoritative component and re-exported the wound types from `worldwake-core`.
3. Extended `delta.rs`, `world_txn.rs`, and `verification.rs` so wounds are first-class in component inventories, create-time delta capture, and event-log/world-state verification.
4. Reduced the architectural duplication that originally made new authoritative components easy to miss: `ComponentKind`/`ComponentValue` now derive from `component_schema.rs`, and `world_txn.rs` plus `verification.rs` now consume schema-driven `World::component_values(...)` instead of maintaining their own component inventories.
5. Added focused tests for wound traits/serialization, component-table storage, world kind restrictions/query helpers, delta inventory coverage, and verification coverage.

Differences from the original plan:

1. The original ticket understated the integration surface. `WoundList` required changes outside `component_schema.rs` so it would not be invisible to auditing and verification.
2. The ticket ended up including a narrow architecture refactor because the duplication was real technical debt and the schema-driven cleanup was small, coherent, and improved long-term robustness without introducing compatibility layers.

Verification results:

1. `cargo test -p worldwake-core` passed.
2. `cargo clippy --workspace --all-targets -- -D warnings` passed.
3. `cargo test --workspace` passed.
