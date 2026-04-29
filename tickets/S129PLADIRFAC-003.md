# S129PLADIRFAC-003: TargetHasWashBasinClean precondition variant

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new variant on the cross-crate `Precondition` enum and matching arms in affordance/validation paths
**Deps**: archive/tickets/S129PLADIRFAC-001.md (provides `WashBasinState` for the precondition arm to read)

## Problem

S129's wash refactor (D7, owned by ticket 007) replaces today's `Precondition::TargetHasResourceSource { target_index: 1, commodity: Water, min_available: 1 }` (`needs_actions.rs:244–250`) with a basin-side gate so affordance generation can rule out empty basins early — currently the basin has no per-facility state to gate on. Without a basin-side precondition variant, the wash refactor would push gating into the commit handler alone, weakening the AI's affordance-discovery contract (FND-8 — preconditions belong in the action's declared surface). This ticket lands the precondition machinery so ticket 007 can swap one declared precondition for another in lockstep.

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `Precondition` enum at `crates/worldwake-sim/src/action_semantics.rs:47` is the cross-crate variant set. Existing variants relevant to this work: `TargetHasWorkstationTag { target_index, tag }` (line 64), `TargetHasResourceSource { target_index, commodity, min_available }` (line 68), `TargetHasConsumableEffect` (line 77), `TargetHasWounds(u8)` (line 81). The shape `TargetHasWashBasinClean { target_index: u8, min: u16 }` (one indexed target + a minimum unit count) mirrors `TargetHasResourceSource`'s arity.
2. Match arms exist at: `crates/worldwake-sim/src/action_semantics.rs:465, 469` (semantics layer), `crates/worldwake-sim/src/action_validation.rs:95, 99, 755, 764, 774` (validation layer), `crates/worldwake-sim/src/affordance_query.rs:322, 325, 460, 461` (affordance/discovery layer). Each arm reads target-resolved component state and decides pass/fail.
3. The shared abstraction boundary under audit is the `Precondition` enum's three downstream consumer surfaces — `action_semantics.rs` defines the variant; `action_validation.rs` validates a planned action against current world state; `affordance_query.rs` enumerates affordances during candidate generation. All three must gain matching arms in this ticket, otherwise the workspace fails to compile.
4. The new arm reads `WashBasinState.clean_water_units` from the target entity; per ticket 001, the `get_component_wash_basin_state` accessor exists. The arm returns "pass" when `clean_water_units >= min` and "fail" otherwise; missing component (basin facility somehow lacking the role-specific state) returns "fail" so candidates over un-stated basins are pruned.
5. Heuristic Removal Discipline (precision-rules §12): this ticket does **not** remove `TargetHasResourceSource` — that variant remains because other actions (e.g., `harvest`) still use it. It only adds a sibling variant. Ticket 007 swaps wash's specific use from one variant to the other.

## Architecture Check

1. Declaring the new variant as part of the cross-crate `Precondition` enum (rather than as a wash-private check inside `needs_actions.rs`) preserves the symmetry that all action preconditions are declared on a single surface. AI affordance discovery, plan validation, and authoritative validation all read the same enum — splitting wash into a private gate would create an information-locality leak (FND-26: systems interact through state, not direct calls into wash internals).
2. The variant carries `target_index: u8` rather than embedding the basin's `EntityId` because preconditions are declared statically at action-definition time before targets are resolved. `target_index` indexes into the resolved-targets array at validation/discovery time. Mirrors the existing `TargetHasWorkstationTag { target_index, tag }` precedent. No backward-compat shim: net-new variant.

## Verification Layers

1. The new variant is exhaustively matched in all three consumer files → `cargo build --workspace` failure if any arm is missing.
2. Affordance discovery prunes wash candidates when the basin is empty → focused unit test in `affordance_query.rs`'s test module: seed a basin with `WashBasinState { clean_water_units: 0, ... }`, declare a precondition `TargetHasWashBasinClean { target_index: 0, min: 1 }`, assert `get_affordances` for an action with that precondition returns no candidate over the empty basin.
3. Validation accepts wash on basins with sufficient water → focused unit test in `action_validation.rs`'s test module: same seed but with `clean_water_units: 5, min: 2` — assert validation returns `Ok(())`.
4. Validation rejects wash on basins with insufficient water → focused unit test: `clean_water_units: 1, min: 2` — assert validation returns the appropriate `Err` (mirror the `TargetHasResourceSource` failure type).

## What to Change

### 1. `crates/worldwake-sim/src/action_semantics.rs`

Add a new variant to the `Precondition` enum at line 47:

```rust
TargetHasWashBasinClean {
    target_index: u8,
    min: u16,
},
```

Position the variant adjacent to `TargetHasResourceSource` (line 68) for grouping by domain. Add a matching arm to the semantic-formatter or display impl at lines 465–469 (mirroring `TargetHasResourceSource`).

### 2. `crates/worldwake-sim/src/action_validation.rs`

Add validation arm(s) at line 95 (and any other exhaustive-match site, per the grep output at lines 99, 755, 764, 774). The arm reads:

```rust
Precondition::TargetHasWashBasinClean { target_index, min } => {
    let basin = targets.get(*target_index as usize).ok_or(...)?;
    let state = world.get_component_wash_basin_state(*basin).ok_or(...)?;
    if state.clean_water_units >= *min {
        Ok(())
    } else {
        Err(...)
    }
}
```

Match the exact error type used by `TargetHasResourceSource`'s rejection path; that variant already returns the canonical "target lacks required quantity" error.

### 3. `crates/worldwake-sim/src/affordance_query.rs`

Add affordance-discovery arms at lines 322 and 460 (the two exhaustive-match sites). The discovery arm mirrors validation but is called during candidate enumeration; it returns whether the affordance is reachable for a given target.

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (modify — new variant + display/format arm)
- `crates/worldwake-sim/src/action_validation.rs` (modify — validation arms at all exhaustive-match sites)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — discovery arms at all exhaustive-match sites)

## Out of Scope

- Wiring the new precondition into the `wash` action's precondition list (deferred to ticket 007).
- Removing `TargetHasResourceSource` from wash (deferred to ticket 007 — that variant stays in the enum because other actions use it).
- Any AI-side affordance changes beyond the discovery arm (those flow through ticket 007's per-basin candidate emission).

## Acceptance Criteria

### Tests That Must Pass

1. New focused test in `action_validation.rs` test block: `target_has_wash_basin_clean_passes_when_units_meet_min`.
2. New focused test in `action_validation.rs`: `target_has_wash_basin_clean_rejects_when_units_below_min`.
3. New focused test in `affordance_query.rs` test block: `wash_basin_clean_precondition_prunes_empty_basin_candidates`.
4. New focused test: `wash_basin_clean_precondition_passes_through_filled_basin_candidates`.
5. Existing suite: `cargo test -p worldwake-sim` (all preexisting precondition tests must continue to pass — no regressions).

### Invariants

1. `Precondition::TargetHasWashBasinClean` is a sibling variant alongside `TargetHasResourceSource`; both remain on the enum. Removing either is a separate refactor.
2. Validation, affordance discovery, and the semantic formatter all match the new variant exhaustively — `cargo build --workspace` succeeds with no missing-arm warnings.
3. The validation arm reads `WashBasinState` via the read-only accessor (`get_component_wash_basin_state`) — no mutation in validation/discovery paths (preserves the read/write separation that affordance discovery is side-effect-free).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_validation.rs` — two new focused tests on the validation arm's pass/fail semantics.
2. `crates/worldwake-sim/src/affordance_query.rs` — two new focused tests on the discovery arm's prune/admit semantics.

### Commands

1. `cargo test -p worldwake-sim action_validation`
2. `cargo test -p worldwake-sim affordance_query`
3. `cargo build --workspace`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
