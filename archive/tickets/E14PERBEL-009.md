# E14PERBEL-009: Split Goal-Formation Belief Reads from Affordance/Search Helpers

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI/sim read-model boundary cleanup across `worldwake-ai` and `worldwake-sim`
**Deps**: `archive/tickets/E14PERBEL-006.md`, `archive/tickets/completed/E14PERBEL-007.md`, `specs/E14-perception-beliefs.md`, `specs/S02-goal-decision-policy-unification.md`, `specs/S03-planner-target-identity-and-affordance-binding.md`

## Problem

`BeliefView` is currently doing two different jobs:

1. it is the AI-facing read surface for goal formation, pressure derivation, ranking, and explanation
2. it is also the broader affordance/search/runtime-feasibility surface used by snapshot building, hypothetical planning state, affordance enumeration, duration estimation, queue checks, reservation checks, and other action-level helpers

Those two jobs overlap, but they are not the same architectural concern.

The current ticket overstates the issue by framing this as "AI versus execution helpers." That is not precise enough for the current codebase. Search, affordance enumeration, and planning snapshots are still planner concerns, but they need a broader helper surface than goal formation does. The real architectural problem is that the AI modules that should only reason from subjective/self/public knowledge still compile directly against the entire broad trait.

That makes the boundary too loose for long-term maintenance:

- goal-forming modules can casually start depending on reservation/queue/runtime helpers they do not need
- future spec work can keep extending the broad trait even when only the narrow goal-forming surface should grow
- the codebase has no explicit place to classify which reads belong to subjective goal formation versus broader action-feasibility planning

## Assumption Reassessment (2026-03-14)

1. `BeliefView` in `crates/worldwake-sim/src/belief_view.rs` is still a mixed trait — confirmed.
2. `PerAgentBeliefView` in `crates/worldwake-sim/src/per_agent_belief_view.rs` still answers some methods from beliefs, some from self-authoritative world state, some from public structure, and some from broader authoritative helpers — confirmed.
3. `archive/tickets/completed/E14PERBEL-004.md` was correct to defer the split. It implemented an honest interim adapter, not the final architecture — confirmed.
4. `archive/tickets/E14PERBEL-006.md` removed `OmniscientBeliefView`, but it explicitly did not solve this broader boundary problem — confirmed.
5. `archive/tickets/completed/E14PERBEL-007.md` added integration coverage for belief isolation, but it also explicitly documented that the trait-boundary cleanup remained open here — confirmed.
6. The active E14 spec still says "No `BeliefView` trait changes required." That statement is now stale relative to the codebase and remaining cleanup work — confirmed.
7. The broad trait is not only used by AI goal formation. It is also used by:
   - `crates/worldwake-sim/src/affordance_query.rs`
   - `crates/worldwake-ai/src/planning_snapshot.rs`
   - `crates/worldwake-ai/src/planning_state.rs`
   - `crates/worldwake-ai/src/search.rs`
   - `crates/worldwake-sim/src/tick_step.rs`
8. Because of (7), a full replacement of `BeliefView` in one ticket would overreach. The honest cleanup for current scope is to introduce a narrower goal-forming trait, migrate the goal-forming AI modules to it, and leave the broader affordance/search surface in place until a later dedicated redesign is warranted.

## Architecture Check

1. The most robust current improvement is a two-layer read model:
   - a narrow AI-facing goal-forming trait for subjective/self/public reads
   - the existing broader affordance/search helper trait for action-feasibility planning
2. This is more beneficial than the current architecture because it adds a real compile-time boundary where the current code has none, while avoiding a larger speculative rewrite of search and affordance infrastructure.
3. This ticket should not pretend that all planning concerns are subjective. Search snapshots, queue/range checks, and duration/affordance helpers still need a broader surface today.
4. This ticket should also not preserve alias paths or compatibility shims around the old unconstrained AI read path. Goal-forming modules should move to the new narrow trait directly.
5. The ideal long-term architecture is still cleaner than what this ticket can finish:
   - goal formation depends on a belief-local surface
   - action-feasibility planning depends on a distinct planning/affordance surface
   - execution uses authoritative `World`/`WorldTxn`
   This ticket should move toward that shape without rewriting the entire planning stack.

## What to Change

### 1. Introduce and document a narrow goal-forming trait

In `crates/worldwake-sim/src/belief_view.rs`, introduce a new trait for the AI read phase. The exact name may vary, but it should reflect goal formation rather than generic belief access. Preferred name:

```rust
pub trait GoalBeliefView { ... }
```

This trait should contain only the reads required by:

- candidate generation
- pressure derivation
- enterprise/read-side market signals
- ranking
- goal explanation

The file should also explicitly document the classification boundary:

- subjective goal-forming reads
- self-authoritative reads
- public topology / public structure reads
- broader affordance/search/runtime-feasibility helpers that remain on `BeliefView`

### 2. Keep `BeliefView` as the broader affordance/search surface for now

Do **not** try to replace the full `BeliefView` trait in this ticket.

`BeliefView` should continue to own methods still required by:

- `affordance_query.rs`
- `planning_snapshot.rs`
- `planning_state.rs`
- `search.rs`
- `tick_step.rs`
- other broader action-feasibility helpers

This ticket is successful if the goal-forming AI modules no longer compile against that full broad trait.

### 3. Migrate the goal-forming AI modules to the narrow trait

Update the production signatures in:

- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/enterprise.rs`
- `crates/worldwake-ai/src/goal_explanation.rs`
- `crates/worldwake-ai/src/pressure.rs`
- `crates/worldwake-ai/src/ranking.rs`

and any directly related helper surface they call so they accept the new narrow trait instead of `&dyn BeliefView`.

The migration should be direct. Do not add a renamed wrapper function that still takes `&dyn BeliefView` merely to preserve the old call shape.

### 4. Update active specs that would otherwise keep extending the wrong surface

At minimum:

- `specs/E14-perception-beliefs.md`
  - remove the stale "no trait changes required" assumption
  - document that E14 established the interim broad `BeliefView`, while this follow-up adds a narrower goal-forming trait
- `specs/S06-commodity-opportunity-valuation.md`
  - do not instruct future work to extend the broad mixed trait blindly
  - future commodity-opportunity reads should target the narrow goal-forming AI surface when they are part of value/ranking logic

### 5. Add boundary tests

Add or strengthen tests that prove:

- the goal-forming modules compile against the narrow trait
- the broader `BeliefView` remains available where affordance/search code still needs it
- future changes to the goal-forming surface require an explicit decision about whether the new method belongs on the narrow trait or the broad one

## Files to Touch

- `tickets/E14PERBEL-009.md` (modify — corrected assumptions and scope)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add/document narrow trait)
- `crates/worldwake-sim/src/lib.rs` (modify — export the new trait)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/enterprise.rs` (modify)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify)
- `crates/worldwake-ai/src/pressure.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — strengthen boundary tests)
- `crates/worldwake-ai/src/lib.rs` (modify — export/test dependency availability as needed)
- `specs/E14-perception-beliefs.md` (modify)
- `specs/S06-commodity-opportunity-valuation.md` (modify)

## Out of Scope

- Reintroducing `OmniscientBeliefView` or any compatibility alias
- Replacing the full `BeliefView` / affordance / search / planning-state stack in one pass
- Moving action execution to read through the new narrow trait
- Reworking `planning_snapshot.rs` and `search.rs` into a brand-new planning-query architecture
- Rumor/report propagation (E15 scope)
- Crime/theft systems (E17 scope)
- Passive local observation (`E14PERBEL-011`)

## Acceptance Criteria

### Tests That Must Pass

1. Goal-forming AI modules compile against the new narrow trait instead of `&dyn BeliefView`
2. `BeliefView` remains the broader surface for affordance/search code that still needs it
3. The boundary is documented in code/specs so future additions are not ad hoc
4. Active specs no longer say "no trait changes required" for this boundary
5. `cargo test -p worldwake-ai`
6. `cargo test -p worldwake-sim`
7. `cargo clippy --workspace`
8. `cargo test --workspace`

### Invariants

1. Goal formation depends on a narrower read surface than affordance/search runtime helpers
2. No backwards-compatibility alias preserves the old unconstrained AI read path
3. The code remains honest about the current interim architecture:
   - narrow trait for goal-forming reads
   - broad trait for affordance/search/runtime-feasibility helpers
4. Future read-surface additions must make an explicit classification choice instead of defaulting to the broad trait

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs`
   - strengthen the compile-time/source-level boundary check so the goal-forming modules are verified against the narrow trait usage
2. `crates/worldwake-ai/src/lib.rs`
   - dependency-availability test updated to include the new exported trait if needed
3. Any directly affected unit tests in the migrated AI modules
   - updated only as needed to compile against the new trait

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - corrected the ticket scope from a too-broad "planner vs execution" split to the real current architectural seam: goal formation versus broader affordance/search helpers
  - added `GoalBeliefView` in `crates/worldwake-sim/src/belief_view.rs` and exported it from `worldwake-sim`
  - migrated the goal-forming AI modules (`candidate_generation`, `enterprise`, `goal_explanation`, `pressure`, `ranking`) to compile against `GoalBeliefView`
  - kept the broader `BeliefView` in place for `affordance_query`, `planning_snapshot`, `planning_state`, `search`, and related runtime-feasibility helpers
  - updated `specs/E14-perception-beliefs.md` and `specs/S06-commodity-opportunity-valuation.md` so future work does not assume the broad mixed trait is the final interface
  - added a regression test in `crates/worldwake-ai/src/agent_tick.rs` that source-checks the goal-forming modules for `GoalBeliefView` usage
  - updated the dependency-availability test in `crates/worldwake-ai/src/lib.rs` to cover the new exported trait
- Deviations from original plan:
  - did not replace the full `BeliefView` / affordance / search surface in one pass because the current code still uses that broader trait honestly for snapshot/search/runtime-feasibility work
  - did not move `failure_handling`, `planning_snapshot`, `planning_state`, or `search` to the new trait because they still depend on broader helpers such as reservations, queues, duration estimation, and other action-level queries
  - kept `has_production_job` on the narrow goal-forming surface for now because candidate generation still uses it to avoid emitting obviously blocked production opportunities; further cleanup can revisit whether that belongs in goal formation or in later search/failure stages
- Verification results:
  - `cargo test -p worldwake-ai --no-fail-fast`
  - `cargo test -p worldwake-sim --no-fail-fast`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace --no-fail-fast`
