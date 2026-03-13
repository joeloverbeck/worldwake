# FND02-005b: Add explain_goal() Debuggability API to AI Crate

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new module in worldwake-ai
**Deps**: Phase 2 complete, ranking.rs and candidate_generation.rs stable

## Problem

No structured goal inspection API exists. The AI system produces emergent agent decisions but provides no programmatic way to answer "why did this agent choose this goal?" The ranking and candidate generation logic exists but has no public entry point for introspection. This is a core debuggability requirement per Principle 27 (Debuggability Is a Product Feature).

## Assumption Reassessment (2026-03-13)

1. `ranking.rs` has `RankedGoal { grounded, priority_class, motive_score }` — confirmed.
2. `RankingContext` holds all inputs needed for ranking (view, agent, utility, needs, thresholds, danger_pressure) — confirmed.
3. `PriorityClass` enum: Background, Low, Medium, High, Critical — confirmed.
4. `candidate_generation.rs` has `generate_candidates()` returning candidates with evidence — confirmed.
5. `BlockedIntentMemory` has `is_blocked(key, tick)` — confirmed.
6. `RecipeRegistry` is available for production-related goal context — confirmed.
7. No `goal_explanation.rs` exists in worldwake-ai — confirmed, must be created.
8. `worldwake-ai/src/lib.rs` has `mod enterprise` (private) — need to add new public module.
9. Motive scoring uses `u32` (integer), not floats — confirmed.

## Architecture Check

1. This is a **derived read-model** (Principle 25) — it recomputes the ranking for a specific goal and returns the explanation. No state stored.
2. Reuses existing `rank_candidates()` and `generate_candidates()` internally — no duplication of ranking logic.
3. No backwards-compatibility shims — new API addition.

## What to Change

### 1. Create `crates/worldwake-ai/src/goal_explanation.rs`

Define the explanation struct:

```rust
pub struct GoalExplanation {
    /// The goal being explained.
    pub goal: GoalKind,
    /// Assigned priority class for this goal.
    pub priority_class: PriorityClass,
    /// Motive value (integer score from ranking).
    pub motive_value: u32,
    /// Entity evidence that contributed to this goal's emission.
    pub evidence_entities: Vec<EntityId>,
    /// Place evidence relevant to this goal.
    pub evidence_places: Vec<EntityId>,
    /// Other goals competing for the agent's attention, with their ranking.
    pub competing_goals: Vec<(GoalKind, PriorityClass, u32)>,
}
```

Implement the explanation function:

```rust
pub fn explain_goal(
    view: &dyn BeliefView,
    agent: EntityId,
    goal: &GoalKind,
    blocked: &BlockedIntentMemory,
    recipes: &RecipeRegistry,
    current_tick: Tick,
) -> Option<GoalExplanation>
```

Implementation:
- Run `generate_candidates()` for the agent to get all current candidates.
- Find the candidate matching the requested `goal`.
- If not found, return `None` (goal not currently emittable).
- Run `rank_candidates()` to get priority and motive for all candidates.
- Extract the target goal's ranking and evidence.
- Collect competing goals from the ranked list.
- Return structured `GoalExplanation`.

### 2. Wire into `crates/worldwake-ai/src/lib.rs`

Add `pub mod goal_explanation;` to the module declarations.

## Files to Touch

- `crates/worldwake-ai/src/goal_explanation.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add module declaration)

## Out of Scope

- Do NOT implement `trace_event_cause()` — that is FND02-005a.
- Do NOT modify `ranking.rs`, `candidate_generation.rs`, or `enterprise.rs` logic.
- Do NOT add CLI integration or display formatting.
- Do NOT modify worldwake-core or worldwake-sim crates.
- Do NOT add stored state — this must remain a derived read-model.
- Do NOT add Permille-based confidence scores — motive values are `u32` integers.

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: Create an agent with hunger pressure, call `explain_goal()` for `ConsumeOwnedCommodity` — verify explanation includes correct goal kind, priority class, and non-empty evidence.
2. Unit test: Call `explain_goal()` for a goal that the agent cannot currently emit — returns `None`.
3. Unit test: Verify `competing_goals` is populated with other ranked candidates.
4. Unit test: Verify explanation is deterministic — same inputs produce same output.
5. Existing suite: `cargo test -p worldwake-ai`
6. Full suite: `cargo test --workspace`

### Invariants

1. Function is a pure derived read-model — no state stored, no mutations.
2. No `HashMap`, `HashSet`, `f32`, `f64` in new code.
3. Existing ranking and candidate generation behavior unchanged.
4. Deterministic — same inputs always return same explanation.
5. Motive values are `u32` (matching `ranking.rs`), not `Permille`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_explanation.rs` (inline test module) — unit tests for explanation generation, missing goals, competing goals, determinism.

### Commands

1. `cargo test -p worldwake-ai -- goal_explanation` — targeted tests
2. `cargo test -p worldwake-ai` — full AI crate suite
3. `cargo clippy --workspace` — lint check
4. `cargo test --workspace` — full workspace suite
