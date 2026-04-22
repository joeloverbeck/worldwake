# S114PLASTGUA-003: PlannedStep extension — guard, expectations, and accessor methods

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — two new fields on `PlannedStep`; `impl PlannedStep` accessor block; existing derive surface preserved.
**Deps**: `archive/tickets/S114PLASTGUA-001.md`

## Problem

Every plan-guard downstream ticket (006 build functions, 007 revalidation, 008 plan adoption, 009 AI tick step) reads `step.guard`, `step.expectations`, and accessor methods on `PlannedStep`. Landing the extension in a dedicated ticket lets reviewers focus on construction-site correctness without conflating it with behavioral logic.

## Assumption Reassessment (2026-04-21)

1. `PlannedStep` is declared at `crates/worldwake-ai/src/planner_ops.rs` with seven existing fields and no accessor `impl` block on the live branch. Reassessment drift: despite the original draft calling it runtime-only, the live type already derives `Ord + PartialOrd + Serialize + Deserialize`, and `PlannedPlan` round-trip tests serialize it through bincode. The added fields therefore had to preserve that existing derive surface rather than narrow it.
2. S114 spec D2 at `specs/S114-plan-step-guards.md:141-170` defines the two new fields (`guard: Option<PlanGuard>`, `expectations: Vec<PlanExpectation>`) and four accessor methods (`primary_target`, `target_place`, `target_claim`, `expected_complete_tick`). Each accessor returns `Option<_>` because the backing field may be absent for untargeted actions.
3. Shared boundary under audit: the `PlannedStep` struct itself. Live constructor fallout was narrower than the draft's 102-count estimate once helper-backed and spread-based sites were accounted for, but it still crossed the same `worldwake-ai` modules named in this ticket. No `Default` impl on `PlannedStep` exists, so touched explicit literals now initialize `guard: None` and `expectations: Vec::new()` directly.
4. Live `GoalKind` coverage: all planning-layer goals construct `PlannedStep` through the planner (`planner_ops::make_step`, `search::transition`, etc.) — not through scenario authoring. Construction-site updates are mechanical and contained within worldwake-ai.
5. No existing test in `plan_revalidation.rs`, `search/`, or `agent_tick/` currently asserts the absence of `guard` / `expectations` fields, so additive fields don't break any test — they only force literal updates.

## Architecture Check

1. Pure additive to the existing `PlannedStep` carrier. No authoritative event-log or sim-layer schema changed, but the live serde/ordering contract on `PlannedStep` remained intact, so ticket 001's runtime guard/expectation types and core predicate enums were widened enough to satisfy the pre-existing derive chain.
2. Accessor methods centralize binding logic used by ticket 006's `build_plan_guard` / `build_plan_expectations`. On the current branch, the strongest honest `target_claim()` seam is the primary authoritative target's location claim key; the method does not invent payload-specific claim taxonomy ahead of ticket 006.

## Verification Layers

1. Struct-field addition / derive preservation → `cargo test -p worldwake-ai --no-run` after constructor fallout is updated.
2. Accessor correctness (`primary_target`, `target_place`, `target_claim`, `expected_complete_tick`) → focused unit tests in a new `#[cfg(test)]` block appended to `planner_ops.rs` or a new `plan_step_accessors.rs` module.
3. No behavioral change → existing `cargo test -p worldwake-ai` suite stays green — the additive fields default to `None` / `Vec::new()` and are not consumed yet.
4. Single-layer (AI-crate runtime only); downstream action-trace and event-log impacts arrive in tickets 007 / 009.

## What to Change

### 1. Extend `PlannedStep`

In `crates/worldwake-ai/src/planner_ops.rs:814`, append to the struct:

```rust
pub guard: Option<PlanGuard>,
pub expectations: Vec<PlanExpectation>,
```

Import `PlanGuard` and `PlanExpectation` from the ticket 001 module.

### 2. Add accessor methods

In a new `impl PlannedStep` block immediately after the struct:

```rust
impl PlannedStep {
    pub fn primary_target(&self) -> Option<EntityId> {
        self.targets.first().and_then(PlanningEntityRef::entity)
    }

    pub fn target_place(&self) -> Option<EntityId> { /* primary authoritative target today */ }

    pub fn target_claim(&self) -> Option<BeliefClaimKey> { /* primary target location claim */ }

    pub fn expected_complete_tick(&self, start_tick: Tick) -> Tick {
        Tick(start_tick.0.saturating_add(self.estimated_ticks as u64))
    }
}
```

### 3. Update every `PlannedStep { ... }` construction site

Touch every affected explicit `PlannedStep { ... }` literal in `worldwake-ai`; initialize `guard: None` and `expectations: Vec::new()` while leaving helper-backed spread sites alone when they already inherit the updated fields.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — struct + impl)
- `crates/worldwake-ai/src/plan_selection.rs` (modify)
- `crates/worldwake-ai/src/feasibility_probe.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — 9 test-literal sites)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/side_benefit.rs` (modify)
- `crates/worldwake-ai/src/interrupts.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — 16 test sites)
- `crates/worldwake-ai/src/search/transition.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

(The precise per-file count is the grep result `rg -c 'PlannedStep \{' crates/worldwake-ai`; at implementation time, re-run to confirm no new site has landed between reassessment and the edit.)

## Out of Scope

- Populating `guard` / `expectations` with non-default values — construction logic lives in ticket 006 (`build_plan_guard` / `build_plan_expectations`).
- Reading `guard` in revalidation (ticket 007).
- Reading `expectations` at plan adoption (ticket 008).
- Any change to golden tests — those do not construct `PlannedStep` literals directly; they exercise planner output.

## Acceptance Criteria

### Tests That Must Pass

1. New unit tests in `planner_ops.rs` tests module covering:
   - `PlannedStep::primary_target` returns `Some(entity)` when `targets[0]` is an `Authoritative(entity)` reference, `None` for `Hypothetical(_)` references or empty `targets`.
   - `PlannedStep::expected_complete_tick(Tick(10))` for a step with `estimated_ticks: 3` returns `Tick(13)`.
   - Default construction (`PlannedStep { ..., guard: None, expectations: vec![] }`) compiles and passes `PartialEq` roundtrip.
2. Existing suite: `cargo test -p worldwake-ai` stays green.

### Invariants

1. `PlannedStep` keeps its pre-existing `Ord + PartialOrd + Serialize + Deserialize` derive surface; this ticket does not narrow or remove that contract.
2. Accessor methods always return `Option<_>` where the backing field may be absent; callers must handle `None` explicitly.
3. `expected_complete_tick` uses `saturating_add` so a pathological `estimated_ticks` cannot overflow.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` tests module (new test functions) — accessor coverage.
2. No changes to existing tests beyond the mechanical field-addition to literal construction sites.

### Commands

1. `cargo test -p worldwake-ai planned_step`
2. `cargo test -p worldwake-ai` (full crate suite)
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-22.

- Extended `PlannedStep` with `guard: Option<PlanGuard>` and `expectations: Vec<PlanExpectation>` in `crates/worldwake-ai/src/planner_ops.rs`.
- Added `PlannedStep::{primary_target,target_place,target_claim,expected_complete_tick}` with the live-honest contract: authoritative primary-target extraction, authoritative-primary-target place passthrough, and a location-scoped `BeliefClaimKey` for `target_claim`.
- Updated the affected `PlannedStep` construction sites across `worldwake-ai` to seed the new fields with `None` / `Vec::new()`.
- Preserved the existing derive boundary on `PlannedStep` by widening the ticket-001 runtime guard types and the core predicate enums to satisfy `Ord` and serde requirements already present on the live branch.

## Deviations

- Reassessment disproved the draft's "runtime-only / no serde" assumption for `PlannedStep`; the implementation preserved the live derive surface instead of trying to narrow it.
- `target_claim()` landed as the strongest honest current seam: the primary authoritative target's `EntityBeliefAspect::Location` claim key. Payload-specific claim derivation remains for later ticketed guard-building work.

## Verification Result

- Passed `cargo test -p worldwake-ai --no-run`
- Passed `cargo test -p worldwake-ai planned_step`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
