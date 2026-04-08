# S75BELVDECOM-001: Extract ControlBeliefView sub-trait (proof-of-concept)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — RuntimeBeliefView trait decomposition in worldwake-sim, impl blocks in worldwake-sim, worldwake-ai, worldwake-systems
**Deps**: S75 (Belief View Domain Decomposition spec)

## Problem

RuntimeBeliefView is a 113-method monolithic trait. Adding any new belief surface method requires a 4-file, 2-crate shotgun surgery pattern. This ticket extracts the smallest domain (ControlBeliefView, 4 methods) as a proof-of-concept to validate the supertrait pattern compiles correctly with `&dyn RuntimeBeliefView`.

## Assumption Reassessment (2026-04-08)

1. `RuntimeBeliefView` confirmed at `crates/worldwake-sim/src/belief_view.rs:363` with 113 methods. The 4 control methods exist: `believed_owner_of`, `believed_rights`, `can_control`, `has_control`.
2. 18 `impl RuntimeBeliefView for ...` blocks exist across the workspace: 2 production (`PerAgentBeliefView` at `per_agent_belief_view.rs:351`, `PlanningState` at `planning_state.rs:1102`) and 16 test mocks.
3. Shared boundary: `RuntimeBeliefView` trait in worldwake-sim is the sole abstraction under edit. All 128 `&dyn RuntimeBeliefView` call sites remain unchanged — supertrait composition preserves the trait object interface.
4. Auto-correction: ticket sketch said `believed_rights(&self, entity)` and `has_control(&self, agent, entity)`. Live signatures are `believed_rights(&self, actor, entity)` and `has_control(&self, entity)`. Corrected below because the live trait surface is unambiguous.
5. Auto-correction: `GoalBeliefView` currently retains `believed_owner_of`, `believed_rights`, and `can_control`, but not `has_control`. This proof-of-concept remains runtime-only; `GoalBeliefView` cleanup stays deferred to ticket 008.
6. Auto-correction: `impl_goal_belief_view!` currently delegates the three overlapping control reads via `RuntimeBeliefView::...`. After extraction those delegation calls must target `ControlBeliefView::...` inside `crates/worldwake-sim/src/belief_view.rs`. This is safe mechanical fallout from moving the method owner trait.
7. Reassessment after implementation: the owned structural change is present across `crates/worldwake-sim/src/belief_view.rs`, `crates/worldwake-sim/src/per_agent_belief_view.rs`, `crates/worldwake-ai/src/planning_state.rs`, and the expected mock/test implementors in `worldwake-ai`, `worldwake-sim`, and `worldwake-systems`. `cargo build --workspace`, `cargo test --workspace`, and package-level clippy for the three touched crates all pass on the live branch.
8. Adjacent contradiction exposed during verification: `cargo clippy --workspace --all-targets -- -D warnings` still fails outside the owned ControlBeliefView boundary with `E0463` resolving `worldwake_systems` from `crates/worldwake-ai/src/agent_tick/planning.rs` unit-test imports when the workspace-level clippy lib-test target is built. This is classified as separate follow-up cleanup, not a required consequence of the ControlBeliefView refactor; see `tickets/WSPCLIPAI-001-fix-workspace-clippy-worldwake-ai-test-linkage.md`.

## Architecture Check

1. The supertrait pattern (`trait RuntimeBeliefView: ControlBeliefView + ...`) is the cleanest approach because it preserves all 128 `&dyn RuntimeBeliefView` call sites without modification. The alternative (replacing `RuntimeBeliefView` with individual `&dyn SubTrait` parameters) would require changing 128 call sites across 27 files — unacceptable blast radius for a structural refactor.
2. No backward-compatibility shims. Methods move from RuntimeBeliefView to ControlBeliefView; the old location is deleted, not aliased.

## Verification Layers

1. Trait object coherence -> `cargo build --workspace` compiles with `&dyn RuntimeBeliefView` still functional
2. Method availability -> existing focused/unit and golden coverage continue calling the 4 control methods through `&dyn RuntimeBeliefView` and `ControlBeliefView` after the owner-trait move
3. Single-layer ticket — no cross-system invariant changed. Workspace-wide clippy restoration is tracked separately because the failing boundary is the `worldwake-ai` workspace lib-test linkage path, not the ControlBeliefView method move itself.

## What to Change

### 1. Define ControlBeliefView sub-trait

In `crates/worldwake-sim/src/belief_view.rs`, define:

```rust
pub trait ControlBeliefView {
    fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId>;
    fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight>;
    fn can_control(&self, agent: EntityId, entity: EntityId) -> bool;
    fn has_control(&self, entity: EntityId) -> bool;
}
```

Copy the method signatures (including default implementations if any) from `RuntimeBeliefView`.

### 2. Add supertrait bound to RuntimeBeliefView

Change:
```rust
pub trait RuntimeBeliefView {
```
To:
```rust
pub trait RuntimeBeliefView: ControlBeliefView {
```

Remove the 4 control methods from RuntimeBeliefView's body (they now live on ControlBeliefView).

### 3. Update all 18 impl blocks

For each `impl RuntimeBeliefView for T` block, extract the 4 control methods into a separate `impl ControlBeliefView for T` block. The production implementations:

- `crates/worldwake-sim/src/per_agent_belief_view.rs:351` — `impl ControlBeliefView for PerAgentBeliefView<'_>`
- `crates/worldwake-ai/src/planning_state.rs:1102` — `impl ControlBeliefView for PlanningState<'_>`

And the 16 test mock implementations (see Files to Touch for full list).

### 4. Export the new sub-trait

Ensure `ControlBeliefView` is exported from `worldwake-sim`'s `lib.rs` so downstream crates can import it.

### 5. Update delegation and UFCS fallout

In `crates/worldwake-sim/src/belief_view.rs`, switch `impl_goal_belief_view!` delegation for the overlapping control reads to `ControlBeliefView::...`. Update any direct UFCS call sites like `RuntimeBeliefView::believed_owner_of(&view, ...)` to use `ControlBeliefView::...` (or equivalent method-call syntax) because those methods no longer live on `RuntimeBeliefView` itself.

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — define sub-trait, add supertrait bound, remove methods)
- `crates/worldwake-sim/src/lib.rs` (modify — export ControlBeliefView)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — split impl block)
- `crates/worldwake-ai/src/planning_state.rs` (modify — split impl block)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` tests (modify — UFCS fallout for moved method owner trait)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — split impl block)
- `crates/worldwake-ai/src/search/tests.rs` (modify — split impl block)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — split impl block)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — split impl block)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — split impl block)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — split impl block)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify — split impl block)
- `crates/worldwake-ai/src/goal_model.rs` (modify — split impl block)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — split impl block)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — split impl block)
- `crates/worldwake-ai/src/pressure.rs` (modify — split impl block)
- `crates/worldwake-ai/src/ranking.rs` (modify — split impl block)
- `crates/worldwake-ai/src/enterprise.rs` (modify — split impl block)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — split impl block)
- `crates/worldwake-sim/src/trade_valuation.rs` (modify — split impl block)

## Out of Scope

- Moving methods from other domains (Entity, Spatial, etc.) — separate tickets
- SnapshotEntity sub-struct decomposition — ticket 007
- GoalBeliefView decomposition — ticket 008
- Splitting belief_view.rs into multiple files/modules

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build --workspace` — trait-object composition still compiles at workspace scope
2. `cargo test --workspace` — all existing tests pass without behavioral changes
3. `cargo clippy -p worldwake-sim --all-targets -- -D warnings`
4. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

### Invariants

1. `&dyn RuntimeBeliefView` remains usable at all 128 existing call sites.
2. All 4 control methods are callable through both `&dyn ControlBeliefView` and `&dyn RuntimeBeliefView`.
3. `impl_goal_belief_view!` still delegates the three planning-visible control reads without behavior change.
4. No behavioral change — method bodies are moved, not modified.
5. Workspace-level clippy failure ownership is not widened into this ticket; the separate `worldwake-ai` unit-test linkage defect remains tracked by `tickets/WSPCLIPAI-001-fix-workspace-clippy-worldwake-ai-test-linkage.md`.

## Test Plan

### New/Modified Tests

1. None — this is a pure structural refactor. All existing tests serve as the behavior proof. No new test logic needed.

### Commands

1. `cargo build --workspace` (compilation is the primary proof)
2. `cargo test --workspace`
3. `cargo clippy -p worldwake-sim --all-targets -- -D warnings`
4. `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
5. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-08.

- Extracted `ControlBeliefView` from `RuntimeBeliefView` in `worldwake-sim`, exported it publicly, and updated the runtime supertrait plus `impl_goal_belief_view!` delegation to route overlapping control reads through the new owner trait.
- Split the control-domain implementation surface out of the production belief views (`PerAgentBeliefView`, `PlanningState`) and the affected test/mock belief views across `worldwake-ai`, `worldwake-sim`, and `worldwake-systems`.
- Updated moved-method UFCS call sites to use `ControlBeliefView::...` where the methods no longer live on `RuntimeBeliefView`.
- Deviation from the original plan: the ticket now records package-level clippy on the three touched crates as its owned verification surface. During handoff, `cargo clippy --workspace --all-targets -- -D warnings` exposed an adjacent `worldwake-ai` workspace test-linkage defect unrelated to the control-view extraction, so that work was split into `WSPCLIPAI-001` instead of widening this ticket.

## Verification Result

- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy -p worldwake-sim --all-targets -- -D warnings`
- Passed `cargo clippy -p worldwake-systems --all-targets -- -D warnings`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
