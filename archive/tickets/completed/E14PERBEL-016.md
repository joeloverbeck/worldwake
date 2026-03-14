# E14PERBEL-016: Remove the Broad Runtime Read Escape Hatch and Make the Richer Planner Surface Explicit

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` read-trait naming/boundary cleanup, `worldwake-ai` planner/search/runtime integrations, and targeted regression coverage for compile-time and runtime boundary enforcement
**Deps**: `archive/tickets/completed/E14PERBEL-015.md`, `archive/specs/E14-perception-beliefs.md`, `specs/IMPLEMENTATION-ORDER.md`, `specs/S06-commodity-opportunity-valuation.md`, `specs/S07-care-intent-and-treatment-targeting.md`

## Problem

E14 and E14PERBEL-015 already established the first real boundary: goal-facing AI code no longer needs to type against the broad runtime trait. The remaining problem is narrower and more architectural.

Today, the codebase has two AI-facing read surfaces in practice, but only one of them is explicit:

- `GoalBeliefView` is the narrow surface already used by goal formation, pressure derivation, ranking, enterprise analysis, and goal explanation
- the broader runtime/planning surface still exists under the old `BeliefView` name and still mixes:
  - richer planning/search helpers such as reservations, queue state, and duration estimation
  - affordance/runtime queries used by snapshot building, search, revalidation, and failure handling
  - the same narrow reads that goal code also needs

That leaves one architectural escape hatch: the broad trait is still the de facto "real" surface, and `GoalBeliefView` is still obtained through a blanket adapter from it. That is workable, but it is not the clean long-term shape.

It creates three recurring risks:

1. future information-boundary bugs are still easy to introduce in tests or new modules because the broad trait can silently satisfy the narrow one
2. the old `BeliefView` name hides the fact that it is now the richer planning/runtime contract rather than the universal AI read surface
3. adding new AI-facing reads still encourages "just put it on the broad trait" instead of making the runtime-only boundary explicit

The code now needs an architectural cleanup that removes that escape hatch, makes the richer surface explicit, and preserves the already-good goal/runtime split.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-sim/src/belief_view.rs` already defines both `GoalBeliefView` and the broader `BeliefView`.
2. `crates/worldwake-ai` has already rewired the core goal-facing modules onto `GoalBeliefView`:
   - `candidate_generation`
   - `pressure`
   - `ranking`
   - `enterprise`
   - `goal_explanation`
3. `GoalBeliefView` is still only a blanket adapter over the broad `BeliefView`, so the separation is not yet enforced by the type graph.
4. `crates/worldwake-ai/src/planning_snapshot.rs` still builds `PlanningSnapshot` from the broad trait, which remains the richer planning/runtime surface in practice.
5. `archive/specs/E14-perception-beliefs.md` explicitly notes that the broad E14 trait surface is interim and that follow-up cleanup may split narrower goal-forming reads from broader affordance/search helpers.
6. `specs/S06-commodity-opportunity-valuation.md` already anticipates this cleanup and warns against blindly extending the mixed broad surface for future AI-facing reads.
7. No active ticket in `tickets/` currently owns the remaining runtime-surface cleanup itself.

## Architecture Check

1. The current architecture is already better than the ticket originally assumed because goal-forming code now depends on a narrow trait by signature.
2. The remaining cleanup should preserve that shape instead of reopening a larger split:
   - keep `GoalBeliefView` as the narrow goal-forming surface
   - rename or replace the broad `BeliefView` with an explicitly richer runtime/planning surface
   - remove the blanket adapter that makes the narrow surface an accidental alias of the broad one
3. This is cleaner than adding more fine-grained traits right now because the concrete unresolved pressure point is the ambiguous broad runtime trait, not the lack of more category traits.
4. This is more robust because new features must choose between the narrow goal surface and the explicitly richer runtime surface rather than silently inheriting both.
5. This is more extensible because future work such as S06 can add reads to the correct surface without contaminating goal formation.
6. No backwards-compatibility alias or shim is acceptable. The old broad `BeliefView` name should not remain as a deprecated umbrella or type alias beside the new surface.

## What to Change

### 1. Make the richer runtime surface explicit in `worldwake-sim`

Replace the ambiguous broad `BeliefView` integration surface with an explicitly richer runtime/planning trait. The exact name can be chosen during implementation, but it should communicate that this surface is broader than goal formation and is intended for planner/runtime use.

The key requirement is that `GoalBeliefView` must no longer be satisfied through a blanket implementation from the richer trait.

### 2. Make planning snapshot construction depend on an explicit snapshot-input surface

`PlanningSnapshot` should not be built from an ambiguously named "catch-all" trait.

Use the explicit richer runtime/planning surface for snapshot construction unless a genuinely smaller snapshot-only surface proves necessary during implementation.

The implementation should keep goal formation narrower than search/runtime simulation and should not widen every AI-facing interface just to satisfy snapshot building.

### 3. Rewire `worldwake-ai` modules onto the narrowest correct surfaces

Update call sites so each module uses the smallest defensible contract:

- preserve `candidate_generation`, `pressure`, `ranking`, `enterprise`, `goal_explanation` on `GoalBeliefView`
- move `planning_snapshot`, `planning_state`, `search`, `planner_ops`, `plan_revalidation`, `failure_handling`, and affordance-driven runtime paths onto the explicit richer runtime/planning surface
- any remaining mixed usages should be evaluated and either narrowed or justified explicitly

The result should make it obvious, from the type signature alone, which stage of the AI pipeline is allowed to read which class of data.

### 4. Remove the architectural escape hatch

After migration:

- the old broad `BeliefView` escape hatch should be deleted or renamed into the explicit richer runtime/planning surface
- blanket implementations that effectively re-expose the full broad surface through `GoalBeliefView` should be removed
- tests should fail to compile if a goal-forming module tries to depend on runtime-only helpers or vice versa

### 5. Add regressions for boundary enforcement, not just behavior

Add tests that lock both behavior and architecture:

- runtime behavior regressions proving the split preserves current lawful behavior
- compile-time or module-boundary regressions proving goal modules cannot reach runtime-only helpers
- snapshot/search regressions proving richer planning surfaces still support queue, reservation, and duration logic without re-widening goal formation

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `archive/specs/E14-perception-beliefs.md` (modify only if the archived E14 outcome summary needs a clarification note about the follow-up cleanup)
- `specs/S06-commodity-opportunity-valuation.md` (modify if trait naming or extension guidance changes)

## Out of Scope

- new rumor/report propagation behavior
- changes to authoritative world validation rules
- introducing extra public-structure or snapshot-only traits unless the implementation proves they are necessary
- reworking unrelated planning algorithms
- adding compatibility wrappers so old broad-trait callers can continue indefinitely
- merchant selling behavior from S04 beyond read-boundary preparation

## Acceptance Criteria

### Tests That Must Pass

1. Goal-forming modules continue to depend only on `GoalBeliefView` and cannot reach reservation, queue, or duration-estimation helpers through that surface.
2. Planning snapshot/search/runtime modules read from an explicit richer runtime/planning contract rather than an ambiguously named catch-all trait.
3. Public route/place structure remains globally readable without making remote affordance-bearing entities globally discoverable.
4. Existing behavior regressions from E14PERBEL-015 continue to pass after the cleanup.
5. Existing suite: `cargo test -p worldwake-ai`
6. Existing suite: `cargo test --workspace`
7. Existing lint: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. No AI goal-forming path can gain extra authority merely because a broader runtime trait happened to be in scope.
2. Goal-forming reads and planning/runtime reads remain distinct concepts in the type system.
3. No backwards-compatibility alias path preserves the old broad trait name as the de facto integration surface.
4. Adding a new AI-facing read in future work requires choosing an explicit surface rather than defaulting to a catch-all trait.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` and related goal modules — update trait-boundary tests so goal formation compiles and behaves correctly on `GoalBeliefView` without relying on the richer runtime surface.
   Rationale: locks the main planner-entry boundary where accidental overreach is most dangerous.
2. `crates/worldwake-ai/src/planning_snapshot.rs` / `crates/worldwake-ai/src/search.rs` — add or update regressions proving the explicit richer runtime/planning surface still supports queue, reservation, and duration behavior after the cleanup.
   Rationale: preserves search/runtime correctness while separating it from goal formation.
3. `crates/worldwake-sim/src/belief_view.rs` or a dedicated trait-boundary test module — add compile-time or structural tests proving `GoalBeliefView` and the richer runtime/planning surface are distinct and non-accidentally-widened.
   Rationale: this ticket is architectural, so enforcement tests must cover the architecture itself.
4. `crates/worldwake-ai/tests/golden_trade.rs` and any other affected end-to-end suites — keep at least one remote-opportunity regression that proves the narrower goal surface still honors the lawful knowledge boundary.
   Rationale: verifies that the trait split does not regress the behavioral guarantees already established.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Implemented a narrower cleanup than the original draft proposed, because the current codebase had already completed the biggest split:

- kept `GoalBeliefView` as the explicit narrow goal-forming surface already used by candidate generation, ranking, pressure, enterprise analysis, and goal explanation
- replaced the ambiguous broad `BeliefView` integration surface with an explicitly richer `RuntimeBeliefView`
- removed the blanket trait escape hatch so types now opt into `GoalBeliefView` explicitly instead of getting it automatically from the richer runtime surface
- rewired planning snapshot, planning state, search, revalidation, failure handling, affordance/runtime paths, and supporting systems onto `RuntimeBeliefView`
- strengthened structural tests to assert the separate goal/runtime surfaces on `PerAgentBeliefView` and `PlanningState`, while updating goal-module test stubs to opt into the goal surface explicitly

Not implemented from the original draft:

- no extra public-structure-only trait
- no separate snapshot-only trait

Those were not justified by the current architecture after reassessment. The cleaner long-term win here was removing the broad-trait escape hatch and making the richer runtime surface explicit without adding unnecessary intermediate abstractions.
