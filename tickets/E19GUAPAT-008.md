# E19GUAPAT-008: Replace overloaded patrol interval with explicit dwell timing fields

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `PatrolProfile` schema change, patrol duration contract update, snapshot/save/load/test updates
**Deps**: E19GUAPAT-001 (PatrolProfile exists), [archive/tickets/guard-patrol/E19GUAPAT-003.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-003.md), [specs/E19-guard-patrol.md](/home/joeloverbeck/projects/worldwake/specs/E19-guard-patrol.md)

## Problem

`PatrolProfile.base_patrol_interval` currently carries two different meanings: the spec names it as a patrol-leg cadence concept, but the delivered runtime patrol action uses it as the base dwell input for `DurationExpr::ActorPatrolProfile`. That overload is not a stable architecture. It hides two different world concepts behind one field, which makes the model harder to extend cleanly and pushes later patrol behavior toward magic reinterpretation instead of explicit state.

## Assumption Reassessment (2026-03-30)

1. The exact shared data contract under audit is `PatrolProfile` in [crates/worldwake-core/src/patrol.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs), the patrol duration resolver in [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs), the belief-side estimator in [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs), and the planning snapshot/runtime mirrors in [crates/worldwake-ai/src/planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs) and [crates/worldwake-ai/src/planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs).
2. The live patrol duration helper currently computes dwell as `base_patrol_interval + base_patrol_interval * vigilance / 1000` through `DurationExpr::ActorPatrolProfile`. That implementation is deterministic and valid as a temporary contract, but it conflates two distinct semantics: route cadence and per-stop observation dwell.
3. The active remaining patrol tickets do not own this schema correction. [tickets/E19GUAPAT-004.md](/home/joeloverbeck/projects/worldwake/tickets/E19GUAPAT-004.md) owns patrol candidate generation and motive scoring, [tickets/E19GUAPAT-005.md](/home/joeloverbeck/projects/worldwake/tickets/E19GUAPAT-005.md) owns a derived public-order view, and [tickets/E19GUAPAT-006.md](/home/joeloverbeck/projects/worldwake/tickets/E19GUAPAT-006.md) owns route adaptation. None of them should silently redefine `PatrolProfile` timing semantics as a side effect.
4. [tickets/E19GUAPAT-007.md](/home/joeloverbeck/projects/worldwake/tickets/E19GUAPAT-007.md) should depend on this ticket because golden patrol-cycle timing should reflect the final explicit dwell contract, not the current overloaded field.
5. The spec already says patrol dwell should be `base_dwell + (vigilance * dwell_scale / 1000)` and allows `base_dwell`/`dwell_scale` to be defined on `PatrolProfile` or derived from it. The clean correction is therefore to make those dwell inputs explicit in `PatrolProfile` itself instead of continuing to derive them from a field named for a different concept.
6. Under `docs/FOUNDATIONS.md`, this is a real architecture issue rather than cosmetic cleanup. It violates Principle 3 (`Concrete State Over Abstract Scores`) because one stored field currently stands in for two concrete timing dimensions, and it violates the repo’s explicit “no workaround” rule because leaving it in place would force future tickets to keep interpreting a misnamed field in different ways.
7. No backward-compatibility aliasing is allowed. This ticket must replace the overloaded field directly rather than keeping `base_patrol_interval` as a deprecated synonym beside new dwell fields.
8. Save/load, planning snapshots, and bincode-based component tests will need coordinated updates because `PatrolProfile` is already serialized and mirrored across crates. Those are required consequences of the schema correction, not adjacent bugs.
9. No additional live patrol cadence consumer currently exists in active code. If, after reassessment during implementation, a separate route-cadence field is still needed immediately for a concrete live caller, the ticket must update its scope explicitly and name that caller before implementation. It should not add a speculative second timing field “just in case.”

## Architecture Check

1. Replacing the overloaded interval field with explicit dwell timing fields is cleaner than preserving `base_patrol_interval` and layering interpretation rules on top of it. The profile should name the concrete timing values the runtime actually uses.
2. The recommended end state is:
   - `base_dwell_ticks: u32`
   - `dwell_vigilance_scale_ticks: u32`
   and `DurationExpr::ActorPatrolProfile` computes `base_dwell_ticks + vigilance * dwell_vigilance_scale_ticks / 1000`.
3. This aligns with the spec’s stated dwell formula, removes naming drift, and keeps the patrol action and planner duration surfaces fully deterministic and explicit.
4. If a separate full-leg cadence concept becomes necessary later, it should be introduced under its own concrete name for its own concrete consumer. That is cleaner than preserving an overloaded field now.
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

### 3. Update planner snapshot/runtime mirrors

Keep the snapshot/runtime parity surfaces aligned in:
- [crates/worldwake-ai/src/planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs)
- [crates/worldwake-ai/src/planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs)
- [crates/worldwake-ai/src/planner_duration_contract.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_duration_contract.rs) if any labels/tests need renaming

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
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify if sample/profile expectations change)
- `crates/worldwake-systems/src/patrol_actions.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
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
4. Snapshot/save/load/planner mirrors stay semantically aligned with the authoritative patrol profile schema

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/patrol.rs` and related component roundtrip tests — prove the patrol profile schema replacement is explicit and serializable
2. `crates/worldwake-sim/src/action_semantics.rs` and `crates/worldwake-sim/src/belief_view.rs` — prove authoritative and belief-side patrol duration resolution both use the new dwell fields
3. `crates/worldwake-systems/src/patrol_actions.rs` — prove patrol runtime behavior still holds under the corrected duration contract
4. `crates/worldwake-ai/src/planning_state.rs` and `crates/worldwake-ai/src/planner_duration_contract.rs` — prove planner snapshot/runtime duration parity remains exhaustive after the schema correction

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-systems`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`
