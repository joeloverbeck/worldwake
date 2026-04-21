# S114PLASTGUA-003: PlannedStep extension — guard, expectations, and accessor methods

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — two new fields on `PlannedStep` (runtime-only); `impl PlannedStep` accessor block.
**Deps**: `archive/tickets/S114PLASTGUA-001.md`

## Problem

Every plan-guard downstream ticket (006 build functions, 007 revalidation, 008 plan adoption, 009 AI tick step) reads `step.guard`, `step.expectations`, and accessor methods on `PlannedStep`. Landing the extension in a dedicated ticket lets reviewers focus on construction-site correctness without conflating it with behavioral logic.

## Assumption Reassessment (2026-04-21)

1. `PlannedStep` is declared at `crates/worldwake-ai/src/planner_ops.rs:814` with seven existing fields (`def_id`, `targets`, `payload_override`, `estimated_ticks`, plus three more — the struct has no `impl` block today per the handoff's evidence-backed facts). `PlannedPlan` at `planner_ops.rs:924` owns `steps: Vec<PlannedStep>`. Runtime-only: no `Serialize` / `Deserialize` derives, not saved/loaded.
2. S114 spec D2 at `specs/S114-plan-step-guards.md:141-170` defines the two new fields (`guard: Option<PlanGuard>`, `expectations: Vec<PlanExpectation>`) and four accessor methods (`primary_target`, `target_place`, `target_claim`, `expected_complete_tick`). Each accessor returns `Option<_>` because the backing field may be absent for untargeted actions.
3. Shared boundary under audit: the `PlannedStep` struct itself. Construction sites counted via `rg -c 'PlannedStep \{' crates/worldwake-ai` = 102 occurrences across 15 files. No `Default` impl on `PlannedStep` exists, so every literal construction site must either add `guard: None, expectations: vec![]` explicitly or be migrated to use a new `..Default::default()` spread (out of scope — the spec does not mandate a `Default` impl).
4. Live `GoalKind` coverage: all planning-layer goals construct `PlannedStep` through the planner (`planner_ops::make_step`, `search::transition`, etc.) — not through scenario authoring. Construction-site updates are mechanical and contained within worldwake-ai.
5. No existing test in `plan_revalidation.rs`, `search/`, or `agent_tick/` currently asserts the absence of `guard` / `expectations` fields, so additive fields don't break any test — they only force literal updates.

## Architecture Check

1. Pure additive to a runtime-only struct. No save format impact (no `Serialize` derive). No impact on the authoritative event log or sim-layer types.
2. Accessor methods centralize binding logic used by ticket 006's `build_plan_guard` / `build_plan_expectations` — without accessors, the build functions would re-implement target extraction at every call site.

## Verification Layers

1. Struct-field addition (compile-time) → workspace `cargo check -p worldwake-ai` after all 102 construction sites are updated.
2. Accessor correctness (`primary_target`, `target_place`, `target_claim`, `expected_complete_tick`) → focused unit tests in a new `#[cfg(test)]` block appended to `planner_ops.rs` or a new `plan_step_accessors.rs` module.
3. No behavioral change → existing decision-trace and agent_tick tests (`crates/worldwake-ai/tests/*.rs`) stay green byte-for-byte — the additive fields default to `None` / `Vec::new()` and are read by no one yet.
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

    pub fn target_place(&self) -> Option<EntityId> {
        // Implementation derives the place from the primary target by
        // inspecting `PlanningEntityRef` variants (Authoritative vs Hypothetical)
        // — exact path per live `PlanningEntityRef` shape.
    }

    pub fn target_claim(&self) -> Option<BeliefClaimKey> {
        // Derived from `targets` + payload_override where applicable.
    }

    pub fn expected_complete_tick(&self, start_tick: Tick) -> Tick {
        Tick(start_tick.0.saturating_add(self.estimated_ticks as u64))
    }
}
```

### 3. Update every `PlannedStep { ... }` construction site

Touch every file listed in the Files to Touch block; add `guard: None, expectations: vec![],` to each literal. This is mechanical but voluminous (~102 sites).

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

1. `PlannedStep` remains runtime-only — no `Serialize` / `Deserialize` derive added.
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
