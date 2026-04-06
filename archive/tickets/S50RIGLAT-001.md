# S50RIGLAT-001: Rights types and world query functions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — core type additions plus shared control-query refactor
**Deps**: None

## Problem

`can_exercise_control()` returns a boolean-equivalent `Result<(), WorldError>`. Callers that need to know *why* an actor has control (e.g., justice system distinguishing lawful confiscation from theft) have no way to ask. This ticket adds the typed rights vocabulary and query functions without changing any existing callers.

## Assumption Reassessment (2026-04-05)

1. `can_exercise_control()` at `crates/worldwake-core/src/world/ownership.rs:156` returns `Result<(), WorldError>` and checks: direct possession, unpossessed ownership, faction delegation, office delegation, container traversal. It does not consult office jurisdiction today. Verified this session.
2. `possessed_by`, `owned_by` are `BTreeMap<EntityId, EntityId>` in `RelationTables` at `crates/worldwake-core/src/relations.rs:68-70`. `factions_of()` at `social.rs:60`, `offices_held_by()` at `social.rs:210`. All verified.
3. No existing `RightKind`, `EffectiveRight`, `effective_rights`, or `has_right` symbols in the codebase. Confirmed via grep.
4. This is a single-layer ticket (core types + world queries). No AI, planning, or cross-system interaction.

## Architecture Check

1. The ticket remains core-local, but it is not literally "pure additions": `effective_rights()` should share the authoritative control walk with `can_exercise_control()` instead of duplicating logic. That is a bounded refactor inside `world/ownership.rs`, not a cross-crate behavior change.
2. No backward-compatibility shims. `can_exercise_control()` is preserved as-is; `effective_rights()` is a new parallel API. Per P28, once downstream callers migrate to `has_right()` where needed, the old function can be left as the convenience boolean check (it is not a shim — it is a valid simpler API).

## Verification Layers

1. `RightKind` enum covers all 5 existing `can_exercise_control()` paths, plus reserved `JurisdictionalAuthority` for ticket 002 → focused unit tests enumerate the 5 live paths and confirm no jurisdictional right is surfaced yet
2. `effective_rights()` returns correct rights for each control path → focused unit test per path (possession, ownership, faction, office, container)
3. `has_right()` is consistent with `effective_rights()` → unit test asserting `has_right(a, e, k) == effective_rights(a, e).iter().any(|r| r.kind == k)`
4. Single-layer ticket (core only) — no cross-system verification needed

## What to Change

### 1. Add RightKind and EffectiveRight types

Create types in `crates/worldwake-core/src/rights.rs` (new file):

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RightKind {
    PhysicalPossession,
    Ownership,
    FactionAuthority,
    OfficeAuthority,
    JurisdictionalAuthority,
    ContainerAccess,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectiveRight {
    pub kind: RightKind,
    pub via: Option<EntityId>,
}
```

Export from `crates/worldwake-core/src/lib.rs`.

### 2. Add effective_rights() and has_right() on World

In `crates/worldwake-core/src/world/ownership.rs`, add:

```rust
pub fn effective_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
    // Follow same logic as can_exercise_control but collect all matching rights:
    // 1. Container traversal: if entity is in container, recurse on container
    // 2. Direct possession → PhysicalPossession
    // 3. Unpossessed ownership → Ownership
    // 4. Faction delegation → FactionAuthority { via: faction }
    // 5. Office delegation → OfficeAuthority { via: office }
    // 6. JurisdictionalAuthority is intentionally not surfaced in ticket 001.
    //    Ticket 002 owns the OfficeData.jurisdiction migration and the first
    //    live implementation of jurisdiction-backed rights.
}

pub fn has_right(&self, actor: EntityId, entity: EntityId, kind: RightKind) -> bool {
    self.effective_rights(actor, entity).iter().any(|r| r.kind == kind)
}
```

### 3. Add focused unit tests

In `crates/worldwake-core/src/world/ownership.rs` (test module) or a new `rights_test.rs`:

- `test_effective_rights_possession` — actor possesses entity → returns `PhysicalPossession`
- `test_effective_rights_ownership` — actor owns unpossessed entity → returns `Ownership`
- `test_effective_rights_faction_authority` — actor in faction that owns entity → returns `FactionAuthority { via: faction }`
- `test_effective_rights_office_authority` — actor holds office that owns entity → returns `OfficeAuthority { via: office }`
- `test_effective_rights_container_access` — entity in container controlled by actor → returns `ContainerAccess`
- `test_effective_rights_no_rights` — actor has no relationship → returns empty vec
- `test_effective_rights_does_not_surface_jurisdictional_authority_yet` — reserved variant is not emitted before ticket 002
- `test_has_right_consistency` — `has_right()` agrees with `effective_rights()`

## Files to Touch

- `crates/worldwake-core/src/rights.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add `pub mod rights` and re-export)
- `crates/worldwake-core/src/world/ownership.rs` (modify — add shared control-right helpers, `effective_rights`, `has_right`)
- `crates/worldwake-core/src/world.rs` (modify — add focused world-level control-right tests if the existing ownership tests remain there)

## Out of Scope

- Changing `can_exercise_control()` return type or signature
- Modifying any existing callers of `can_exercise_control()`
- `OfficeData.jurisdiction` migration (ticket 002)
- Belief-facing queries (ticket 003)
- Justice candidate generation (ticket 004)
- Any live `JurisdictionalAuthority` result (requires ticket 002 for jurisdiction substrate first)

## Acceptance Criteria

### Tests That Must Pass

1. `test_effective_rights_possession` — possession yields `PhysicalPossession`
2. `test_effective_rights_ownership` — ownership yields `Ownership`
3. `test_effective_rights_faction_authority` — faction membership yields `FactionAuthority`
4. `test_effective_rights_office_authority` — office holding yields `OfficeAuthority`
5. `test_effective_rights_container_access` — container control yields `ContainerAccess`
6. `test_effective_rights_no_rights` — no relationship yields empty vec
7. `test_effective_rights_does_not_surface_jurisdictional_authority_yet` — no jurisdictional right is emitted before ticket 002
8. `test_has_right_consistency` — `has_right` agrees with `effective_rights`
9. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `effective_rights(a, e)` is non-empty if and only if `can_exercise_control(a, e).is_ok()` — the two functions must agree on the boolean outcome
2. `RightKind` variants map 1:1 to the existing `can_exercise_control()` check paths; `JurisdictionalAuthority` remains reserved and unsurfaced in ticket 001

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/world.rs` or `crates/worldwake-core/src/world/ownership.rs` — 8 focused unit tests for rights enumeration, reserved-jurisdiction behavior, and consistency

### Commands

1. `cargo test -p worldwake-core -- effective_rights`
2. `cargo test -p worldwake-core -- has_right`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completed: 2026-04-05
- What changed:
  - Added [`RightKind`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/rights.rs) and [`EffectiveRight`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/rights.rs) in a new core model file and re-exported them from [`lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/lib.rs)
  - Added [`World::effective_rights()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/ownership.rs) and [`World::has_right()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/ownership.rs)
  - Refactored [`can_exercise_control()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/ownership.rs) to share one authoritative control-right walk with the new typed query path
  - Added focused rights tests in [`world.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs) covering possession, ownership, faction authority, office authority, container access, no-rights, reserved jurisdiction absence, and boolean consistency
- Deviations from original plan:
  - `JurisdictionalAuthority` landed as a reserved `RightKind` variant but is intentionally not surfaced yet; ticket `S50RIGLAT-002` still owns the first live jurisdiction-backed right result
  - The ticket required a bounded internal helper refactor in `world/ownership.rs`; it was not purely additive in implementation shape
- Verification:
  - `cargo test -p worldwake-core -- effective_rights`
  - `cargo test -p worldwake-core has_right_consistency`
  - `cargo test -p worldwake-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
