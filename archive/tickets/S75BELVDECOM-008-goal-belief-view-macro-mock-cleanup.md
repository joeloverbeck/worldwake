# S75BELVDECOM-008: GoalBeliefView decomposition + macro update + mock cleanup

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: Yes — GoalBeliefView trait composition, goal bridge traits, blanket impl path, test mock ergonomics
**Deps**: archive/tickets/S75BELVDECOM-007-snapshot-entity-sub-struct-decomposition.md

## Problem

After all RuntimeBeliefView sub-traits are extracted and SnapshotEntity is decomposed, GoalBeliefView still remains monolithic even though its method surface now maps almost entirely onto the extracted domain traits. This ticket finishes that cleanup by introducing goal-only bridge traits for the partial domains, replacing the runtime macro delegation path with blanket impls, and keeping GoalBeliefView as the stable AI-facing facade over that split.

## Assumption Reassessment (2026-04-09)

1. `GoalBeliefView` is still monolithic in `crates/worldwake-sim/src/belief_view.rs`, but its 92 methods now map directly onto the extracted domain traits plus three partial domains: `SpatialBeliefView`, `TemporalBeliefView`, and `ControlBeliefView`.
2. `impl_goal_belief_view!` is still only a mechanical delegation layer for `RuntimeBeliefView` implementors (`PerAgentBeliefView`, `PlanningState`, and several test doubles). Because the runtime types already implement the underlying domain traits, the live cleanup shape is to replace the macro with blanket impls rather than keep a second delegation mechanism.
3. Broad default implementations on the existing runtime sub-traits would be the wrong cleanup surface. They would weaken compile-time guarantees for production implementors by turning required runtime methods into silently-defaultable behavior. The lawful live cleanup is narrower: compose full-coverage domains directly, add goal-only bridge traits for the partial domains, replace runtime macro delegation with blanket impls, and update the small set of direct `GoalBeliefView` stubs to those narrower traits.

## Architecture Check

1. GoalBeliefView should stay the stable planning facade, but it should stop depending on a separate per-type delegation macro. The partial domains are `Spatial`, `Temporal`, and `Control`.
2. The clean live boundary is: add three narrow goal-only bridge traits for the partial domains, keep GoalBeliefView as the facade trait, and provide blanket impls from the richer runtime traits to those goal bridge traits plus a blanket GoalBeliefView impl for runtime implementors.
3. With that shape in place, runtime implementors derive GoalBeliefView automatically through blanket impls, the macro and its invocation sites disappear, and only the remaining direct goal-view stubs need local cleanup.

## Verification Layers

1. GoalBeliefView composition -> `cargo build --workspace` (compile-time proof)
2. Blanket-impl replacement correctness -> all golden and planner-facing tests pass
3. Direct GoalBeliefView stub cleanup -> test compilation succeeds with the narrowed goal-facing trait set

## What to Change

### 1. Audit GoalBeliefView method-to-trait mapping

Partition the 92 GoalBeliefView methods into:
- full-coverage domains that can be inherited directly from the existing runtime sub-traits
- partial domains that need a goal-only bridge trait because the runtime trait still has extra required methods outside the goal surface

### 2. Define goal-only bridge traits for the partial domains

Add narrow goal-facing bridge traits for the partial surfaces:
- goal spatial reads (`effective_place`, `entities_at`, `locally_observed_entities_at`, `adjacent_places_with_travel_ticks`, `route_experience`, `patrol_route`)
- goal temporal reads (`current_tick`)
- goal control reads (`believed_owner_of`, `believed_rights`, `can_control`)

Provide blanket impls from the richer runtime traits (`SpatialBeliefView`, `TemporalBeliefView`, `ControlBeliefView`) to those goal bridge traits.

### 3. Keep GoalBeliefView as the facade over the split

Keep the existing GoalBeliefView call contract, but route runtime implementors through the new bridge traits and blanket impl path instead of the old macro.

### 4. Replace the runtime macro delegation path with blanket impls

Once GoalBeliefView is compositional, make any type implementing the required goal-facing traits automatically implement GoalBeliefView. Delete the old macro invocation sites from runtime implementors and test doubles.

### 5. Simplify existing direct GoalBeliefView stubs

Update the remaining direct `impl GoalBeliefView for ...` test stubs to implement only the narrower goal-facing traits they actually use. RuntimeBeliefView test doubles should no longer need a separate GoalBeliefView macro invocation once the blanket impl lands.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — goal bridge traits, GoalBeliefView facade cleanup, blanket impls, macro removal)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — remove obsolete macro invocation)
- `crates/worldwake-ai/src/planning_state.rs` (modify — remove obsolete macro invocation)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify — remove obsolete macro invocation)
- `crates/worldwake-ai/src/pressure.rs` (modify — remove obsolete macro invocation)
- `crates/worldwake-ai/src/ranking.rs` (modify — remove obsolete macro invocation)
- `crates/worldwake-ai/src/enterprise.rs` (modify — remove obsolete macro invocation)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify — remove obsolete macro invocation)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — remove obsolete macro invocation)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — remove obsolete macro invocation)
- `crates/worldwake-sim/src/commodity_opportunity.rs` (modify — direct GoalBeliefView stub split)
- `crates/worldwake-ai/src/feasibility.rs` (modify — direct GoalBeliefView stub split)
- `crates/worldwake-ai/src/exhaustion.rs` (modify — direct GoalBeliefView stub split)
- `crates/worldwake-ai/src/pursuit_belief.rs` (modify — direct GoalBeliefView stub split)
- `crates/worldwake-sim/src/belief_view.rs` (modify — direct GoalBeliefView test stub split)

## Out of Scope

- Further sub-trait extraction (completed in 001-006)
- SnapshotEntity changes (completed in 007)
- Splitting belief_view.rs into multiple files/modules

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace`
2. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. GoalBeliefView remains usable at all existing `&dyn GoalBeliefView` call sites.
2. Replacing the runtime macro delegation path with blanket impls does not change runtime behavior.
3. Direct GoalBeliefView stubs and runtime test doubles compile against the narrowed goal-facing trait boundary without adding defaultable production runtime methods.
4. No behavioral change.

## Outcome

Completed on 2026-04-09.

Added `GoalSpatialBeliefView`, `GoalTemporalBeliefView`, and `GoalControlBeliefView` in `crates/worldwake-sim/src/belief_view.rs`; removed `impl_goal_belief_view!`; and replaced the old runtime delegation path with blanket impls that keep `GoalBeliefView` usable at existing call sites while deriving from the decomposed runtime traits. Macro invocation fallout was removed from the runtime/test implementors, and the remaining direct goal-view stubs were brought back to a compiling shape under the new facade boundary.

Deviation from the original draft: the live cleanup preserved `GoalBeliefView` as the stable AI-facing facade and only decomposed the implementation path beneath it, so not every direct goal-view test double needed to be rewritten onto narrower sub-traits.

Verification passed with:
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Test Plan

### New/Modified Tests

1. None — pure structural refactor. The trait and stub simplification is validated by existing tests compiling and passing.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
