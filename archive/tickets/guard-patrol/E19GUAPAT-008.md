# E19GUAPAT-008: Replace overloaded patrol interval with explicit dwell timing fields

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `PatrolProfile` schema change, patrol dwell contract update, core/sim/AI fixture and parity test updates
**Deps**: E19GUAPAT-001 (PatrolProfile exists), [archive/tickets/guard-patrol/E19GUAPAT-003.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-003.md), [specs/E19-guard-patrol.md](/home/joeloverbeck/projects/worldwake/specs/E19-guard-patrol.md)

## Problem

`PatrolProfile.base_patrol_interval` currently carries two different meanings: the spec names it as a patrol-leg cadence concept, but the delivered runtime patrol action uses it as the base dwell input for `DurationExpr::ActorPatrolProfile`. That overload is not a stable architecture. It hides two different world concepts behind one field, which makes the model harder to extend cleanly and pushes later patrol behavior toward magic reinterpretation instead of explicit state.

## Assumption Reassessment (2026-03-30)

1. The exact shared data contract under audit is `PatrolProfile` in [crates/worldwake-core/src/patrol.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs), the authoritative dwell resolver `patrol_duration_ticks()` in [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs), and the belief-side estimator in [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs). The AI layer does not define a second patrol-timing schema; [crates/worldwake-ai/src/planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs) and [crates/worldwake-ai/src/planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs) carry whole `PatrolProfile` values, so they need fixture/parity updates rather than a separate architectural rewrite.
2. The live patrol duration helper currently computes dwell as `base_patrol_interval + base_patrol_interval * vigilance / 1000` through `DurationExpr::ActorPatrolProfile`. That implementation is deterministic, but it conflates two different concepts: a named patrol-leg interval and the only live runtime consumer, which is per-stop dwell.
3. There is no second live consumer for patrol cadence in current code. The patrol action is explicitly the dwell phase, and no runtime or planner surface currently reads a separate leg-cadence contract. Introducing a new cadence field now would therefore be speculative architecture.
4. The active spec is internally inconsistent: [specs/E19-guard-patrol.md](/home/joeloverbeck/projects/worldwake/specs/E19-guard-patrol.md) still names `base_patrol_interval` as cadence in the component shape, but its patrol-action section already defines dwell as `base_dwell + (vigilance * dwell_scale / 1000)`. This ticket should align implementation with the explicit dwell formula and record the spec contradiction, not preserve the overloaded field.
5. The active remaining patrol tickets do not own this schema correction. [tickets/E19GUAPAT-004.md](/home/joeloverbeck/projects/worldwake/tickets/E19GUAPAT-004.md) owns patrol candidate generation and motive scoring, [tickets/E19GUAPAT-005.md](/home/joeloverbeck/projects/worldwake/tickets/E19GUAPAT-005.md) owns a derived public-order view, and [tickets/E19GUAPAT-006.md](/home/joeloverbeck/projects/worldwake/tickets/E19GUAPAT-006.md) owns route adaptation. None of them should silently redefine patrol timing semantics as a side effect.
6. [tickets/E19GUAPAT-007.md](/home/joeloverbeck/projects/worldwake/tickets/E19GUAPAT-007.md) correctly depends on this ticket because future patrol goldens should lock in the final explicit dwell contract, not the current overloaded field.
7. Under [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md), this is a real architecture issue rather than cosmetic cleanup. One stored field currently stands in for two concrete timing dimensions, which is exactly the kind of semantic overload the repo forbids.
8. No backward-compatibility aliasing is allowed. This ticket must replace the overloaded field directly rather than keeping `base_patrol_interval` as a deprecated synonym beside new dwell fields.
9. The required downstream updates are current-code consequences of the schema change: `PatrolProfile` sample constructors, component/delta roundtrips in `worldwake-core`, authoritative/belief duration tests in `worldwake-sim`, patrol action tests in `worldwake-systems`, and planner parity fixtures in `worldwake-ai`. There is no dedicated patrol save/load test surface to update beyond the existing serde/bincode/component proof surfaces.

## Architecture Check

1. Replacing the overloaded interval field with explicit dwell timing fields is cleaner than preserving `base_patrol_interval` and layering interpretation rules on top of it. The profile should name the concrete timing values the runtime actually uses.
2. The recommended end state is:
   - `base_dwell_ticks: u32`
   - `dwell_vigilance_scale_ticks: u32`
   and `DurationExpr::ActorPatrolProfile` computes `base_dwell_ticks + vigilance * dwell_vigilance_scale_ticks / 1000`.
3. This aligns with the spec’s stated dwell formula, removes naming drift, and keeps the patrol action and planner duration surfaces fully deterministic and explicit.
4. This is materially better than the current architecture because it removes semantic aliasing without inventing a second unused transport path. If a separate full-leg cadence concept becomes necessary later, it should be introduced under its own concrete name for its own concrete consumer.
5. No backwards-compatibility shims or alias fields.

## Verification Layers

1. `PatrolProfile` schema replacement remains serialization-safe and explicit -> focused `worldwake-core` component/bincode tests
2. Patrol duration uses the new explicit dwell fields authoritatively -> focused `worldwake-sim` duration resolution tests
3. Belief-side duration estimation matches authoritative patrol duration -> focused `belief_view` / planning-state parity tests
4. Planner duration dependency inventory remains exhaustive after the schema change -> focused `worldwake-ai` planner duration contract test
5. Patrol runtime behavior still advances routes correctly under the new dwell contract -> focused `worldwake-systems` patrol action tests

## What to Change

### 1. Replace the overloaded timing field in `PatrolProfile`

Update [crates/worldwake-core/src/patrol.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs) so `PatrolProfile` stores explicit dwell timing inputs instead of `base_patrol_interval`.

Recommended shape:

```rust
pub struct PatrolProfile {
    pub base_dwell_ticks: u32,
    pub dwell_vigilance_scale_ticks: u32,
    pub vigilance: Permille,
    pub route_adaptation_sensitivity: Permille,
    pub patrol_motive_weight: Permille,
}
```

### 2. Update the shared patrol duration contract

Change [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs) and [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) so `DurationExpr::ActorPatrolProfile` resolves from the explicit dwell fields, not from a renamed cadence field.

### 3. Update AI parity fixtures, not a second schema

Keep the runtime/planner parity surface aligned in:
- [crates/worldwake-ai/src/planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs)
- [crates/worldwake-ai/src/planner_duration_contract.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_duration_contract.rs)

No dedicated patrol-timing field mapping exists in `planning_snapshot.rs`; if that file changes at all, it should only be because test fixtures or imported sample values changed.

### 4. Update patrol tests and sample data

Adjust sample `PatrolProfile` values and assertions everywhere they rely on the old field name or its overloaded formula, including core component tests, sim duration tests, and patrol action tests.

### 5. Update downstream patrol ticket assumptions if needed

If implementation changes the expected patrol dwell semantics materially, update [tickets/E19GUAPAT-007.md](/home/joeloverbeck/projects/worldwake/tickets/E19GUAPAT-007.md) so the future goldens assert against the final explicit dwell contract.

## Files to Touch

- `crates/worldwake-core/src/patrol.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/world_txn.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-sim/src/action_semantics.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-systems/src/patrol_actions.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/planner_duration_contract.rs` (modify if required by renamed labels or tests)
- `tickets/E19GUAPAT-007.md` (modify only if final dwell semantics need ticket assumption updates)

## Out of Scope

- Patrol candidate generation or patrol motive arithmetic beyond whatever compile/test updates the schema replacement requires
- Route adaptation logic
- Public-order guard presence factor
- Golden patrol scenarios themselves
- Introducing a speculative separate cadence field without a named live consumer

## Acceptance Criteria

### Tests That Must Pass

1. `PatrolProfile` no longer stores the overloaded `base_patrol_interval` field
2. Patrol dwell resolves from explicit dwell fields and `vigilance`
3. Belief-side duration estimation matches authoritative patrol duration after the schema change
4. Existing patrol action tests still prove commit/abort route-progress invariants under the new dwell contract
5. Existing planner duration inventory/parity tests still pass after the schema change
6. Existing suite: `cargo test -p worldwake-core`
7. Existing suite: `cargo test -p worldwake-sim`
8. Existing suite: `cargo test -p worldwake-systems`
9. Existing suite: `cargo test -p worldwake-ai`
10. `cargo clippy --workspace`

### Invariants

1. Patrol timing state is explicit and concrete; one field must not stand in for both cadence and dwell semantics
2. No backwards-compatibility alias field for `base_patrol_interval`
3. No `f32`/`f64`; all dwell arithmetic remains deterministic integer / `Permille` math
4. Authoritative, belief-side, and planner parity surfaces stay semantically aligned with the authoritative patrol profile schema

## Test Plan

### New/Modified Tests

1. [crates/worldwake-core/src/patrol.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs), [crates/worldwake-core/src/world.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs), [crates/worldwake-core/src/component_tables.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_tables.rs), [crates/worldwake-core/src/world_txn.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs), and [crates/worldwake-core/src/delta.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs) — update patrol-profile component roundtrip and delta fixtures to the explicit dwell schema
2. [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs)
   `action_semantics::tests::patrol_duration_ticks_uses_explicit_base_and_scale_fields` — new regression proving base dwell and vigilance scale are independent inputs rather than one overloaded field
3. [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs)
   `action_semantics::tests::duration_expr_resolves_trade_and_combat_driven_ticks_from_authoritative_state` — updated authoritative patrol duration expectation under the explicit dwell schema
4. [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs)
   `belief_view::tests::estimate_duration_from_beliefs_uses_patrol_profile_duration_contract` — updated belief-side patrol duration parity coverage
5. [crates/worldwake-systems/src/patrol_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/patrol_actions.rs)
   `patrol_actions::tests::patrol_duration_scales_with_vigilance` — updated runtime patrol action duration coverage with the new explicit dwell fields
6. [crates/worldwake-ai/src/planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs) and [crates/worldwake-ai/src/planner_duration_contract.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_duration_contract.rs)
   `planning_state::tests::planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract` and `planner_duration_contract::tests::planner_duration_inventory_matches_live_non_fixed_planner_surface` — updated planner/runtime duration parity coverage after the schema correction

### Commands

1. `cargo test -p worldwake-core patrol_profile`
2. `cargo test -p worldwake-sim action_semantics::tests::patrol_duration_ticks_uses_explicit_base_and_scale_fields -- --exact`
3. `cargo test -p worldwake-sim duration_expr_resolves_trade_and_combat_driven_ticks_from_authoritative_state`
4. `cargo test -p worldwake-sim estimate_duration_from_beliefs_uses_patrol_profile_duration_contract`
5. `cargo test -p worldwake-systems patrol_actions::tests::patrol_duration_scales_with_vigilance -- --exact`
6. `cargo test -p worldwake-ai planning_state::tests::planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract -- --exact`
7. `cargo test -p worldwake-ai planner_duration_contract::tests::planner_duration_inventory_matches_live_non_fixed_planner_surface -- --exact`
8. `cargo test -p worldwake-core`
9. `cargo test -p worldwake-sim`
10. `cargo test -p worldwake-systems`
11. `cargo test -p worldwake-ai`
12. `cargo clippy --workspace`

## Outcome

- Completed: 2026-03-30
- What actually changed: replaced `PatrolProfile.base_patrol_interval` with explicit `base_dwell_ticks` and `dwell_vigilance_scale_ticks`, updated `patrol_duration_ticks()` to use the explicit dwell contract, and updated the affected core/sim/system/AI fixtures and parity tests.
- Deviations from original plan: no separate cadence field was added because reassessment found no live cadence consumer; `planning_snapshot.rs` and `tickets/E19GUAPAT-007.md` did not need changes; the active E19 spec contradiction was recorded here but not edited in-scope.
- Verification results: focused patrol/profile/parity commands passed, then `cargo test -p worldwake-core`, `cargo test -p worldwake-sim`, `cargo test -p worldwake-systems`, `cargo test -p worldwake-ai`, and `cargo clippy --workspace` all passed.
