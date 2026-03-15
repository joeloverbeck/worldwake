# E16OFFSUCFAC-007: Implement Succession System (Per-Tick)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new system function in worldwake-systems, system dispatch wiring
**Deps**: E16OFFSUCFAC-002, E16OFFSUCFAC-003, E16OFFSUCFAC-006

## Problem

E16 requires a per-tick `succession_system()` that detects office vacancies when holders die, manages timed succession periods, and resolves succession via either Support (vote counting) or Force (territorial control). The Politics system slot already exists in `SystemManifest` and `SystemDispatch` as a noop — this ticket implements it.

## Assumption Reassessment (2026-03-15)

1. `SystemManifest` in `system_manifest.rs` already has a Politics system slot — confirmed (index 6, currently noop).
2. `SystemDispatch` dispatch table maps Politics to `noop_system` — confirmed, will replace with `succession_system`.
3. `OfficeData` component with `vacancy_since`, `succession_law`, `succession_period_ticks` will exist (E16OFFSUCFAC-002).
4. `support_declarations` relation with count/clear APIs will exist (E16OFFSUCFAC-003).
5. `office_holder`/`offices_held` relation APIs already exist — confirmed.
6. Entity alive status can be checked via existing APIs — confirmed.
7. Place-based entity queries exist (`entities_at`) — confirmed, needed for Force succession.
8. `WorldTxn` atomic commit is the mechanism for safe holder changes — confirmed.

## Architecture Check

1. Replacing the noop Politics system with `succession_system` follows the established pattern — the slot was pre-allocated for this purpose.
2. The system runs after needs and combat, before AI decisions — correct ordering since AI needs to know about succession outcomes.
3. All mutations go through `WorldTxn` for atomicity — prevents two-holder race conditions.
4. Office uniqueness (invariant 9.13) is enforced by setting the new holder and clearing vacancy atomically.
5. No backward-compatibility shims.

## What to Change

### 1. Create `crates/worldwake-systems/src/offices.rs`

New module with `succession_system()` function:

```
pub fn succession_system(world: &World, txn: &mut WorldTxn, tick: Tick)

For each Office entity with OfficeData:
  IF holder is alive -> continue (office is stable)
  IF holder is dead AND vacancy_since is None:
    -> Set vacancy_since = current_tick via txn
    -> Clear office_holder relation via txn
    -> Emit VacancyEvent (visibility: SamePlace at jurisdiction)
  IF vacancy_since is Some(start_tick):
    Match succession_law:
      Support:
        IF current_tick - start_tick >= succession_period_ticks:
          -> Count support_declarations for this office
          -> Candidate with most declarations wins
          -> If tie: extend period by succession_period_ticks / 2
          -> If winner: set office_holder via txn, clear vacancy_since,
             clear support_declarations for this office,
             emit InstallationEvent
          -> If no declarations: extend period
      Force:
        -> Check if any eligible agent is alone at jurisdiction
           (no other eligible claimants present)
           AND has been present for >= succession_period_ticks / 2
        -> If so: install them via txn, emit InstallationEvent
        -> Combatants at jurisdiction resolve naturally via existing combat system
```

### 2. Add helper functions

- `offices_with_jurisdiction(place, world)` — find all offices governing a place
- `office_is_vacant(office, world) -> bool` — check if office has no living holder
- `eligible_agents_at(office, place, world) -> Vec<EntityId>` — agents at place who satisfy eligibility rules
- `count_hostile_faction_pairs_at(place, world) -> u32` — needed for public order (shared helper for E16OFFSUCFAC-008)

### 3. Replace noop in dispatch table

Update `crates/worldwake-systems/src/lib.rs` dispatch table to replace `noop_system` with `succession_system` for the Politics slot.

### 4. Wire module into lib.rs

Add `pub mod offices;` and exports.

## Files to Touch

- `crates/worldwake-systems/src/offices.rs` (new — succession system + helpers)
- `crates/worldwake-systems/src/lib.rs` (modify — add module, replace noop in dispatch table)

## Out of Scope

- Bribe/Threaten/DeclareSupport action handlers (E16OFFSUCFAC-006 — already done by dependency)
- Public order function (E16OFFSUCFAC-008)
- AI goal generation or planner ops (E16OFFSUCFAC-009)
- Belief-mediated vacancy awareness in the AI layer (E16OFFSUCFAC-009)
- Event visibility propagation mechanics (already handled by E14/E15)
- Modifying the SystemManifest ordering — Politics slot already exists at the right position

## Acceptance Criteria

### Tests That Must Pass

1. **Vacancy detection**: When office holder dies, `vacancy_since` is set to current tick.
2. **Vacancy detection**: `office_holder` relation is cleared when holder dies.
3. **Vacancy detection**: `VacancyEvent` emitted with `SamePlace` visibility at jurisdiction.
4. **Support succession**: Candidate with most declarations wins after period expires.
5. **Support succession**: Tied vote extends period by `succession_period_ticks / 2`.
6. **Support succession**: No declarations extends period.
7. **Support succession**: Winner installed atomically — `office_holder` set, `vacancy_since` cleared, declarations cleared.
8. **Support succession**: `InstallationEvent` emitted on successful installation.
9. **Force succession**: Eligible agent alone at jurisdiction for required ticks is installed.
10. **Force succession**: Multiple eligible claimants at jurisdiction prevents installation (combat resolves naturally).
11. **Office uniqueness (9.13)**: Succession NEVER produces two simultaneous holders.
12. **Stable office**: Living holder means no succession activity.
13. Politics system replaces noop in dispatch table.
14. `cargo clippy --workspace --all-targets -- -D warnings`
15. `cargo test --workspace`

### Invariants

1. **Office uniqueness (9.13)**: At most one holder per office at any tick — enforced by `WorldTxn` atomic commit.
2. No scripted succession — outcomes emerge from agent actions (spec section 8).
3. All events use `SamePlace` visibility — no information teleportation.
4. Determinism: `BTreeMap` iteration order, integer arithmetic only.
5. No forward dependencies on E17/E19 — extension points are commented but not implemented.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` — comprehensive tests for vacancy detection, Support resolution, Force resolution, tie-breaking, period extension, office uniqueness.
2. Integration test verifying Politics system runs in the tick loop (dispatch table test).

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
