# E16OFFSUCFAC-009: Political AI Integration for Offices

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — belief view boundary, planner ops, goal model, candidate generation, ranking in `worldwake-ai` / `worldwake-sim`
**Deps**: E16OFFSUCFAC-004, E16OFFSUCFAC-006, E16OFFSUCFAC-007, E16OFFSUCFAC-008

## Problem

Most E16 office substrate already exists in core/systems, but AI still cannot participate in office politics end to end.

The remaining gap is not just "emit a couple of goals." The current AI stack is missing the political query surface and scoring needed to plan and choose political behavior without violating the belief boundary:

1. `PlannerOpKind` still does not classify `bribe`, `threaten`, or `declare_support`.
2. `GoalKindTag` already includes `ClaimOffice` and `SupportCandidateForOffice`, but `GoalKindPlannerExt` still exposes no relevant ops for them.
3. `candidate_generation.rs` does not emit political goals.
4. `ranking.rs` gives both political goals motive `0`, so even emitted goals would be filtered out.
5. `GoalBeliefView` has no office/faction/loyalty/support query surface, so political candidate generation currently cannot be implemented cleanly without either world cheating or brittle special cases.

## Assumption Reassessment (2026-03-15)

### Confirmed existing code

1. `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice` already exist in [goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs).
2. `GoalKindTag::{ClaimOffice, SupportCandidateForOffice}` already exist in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs).
3. `OfficeData`, `FactionData`, `SuccessionLaw`, `EligibilityRule`, `support_declarations`, and `UtilityProfile.courage` already exist in core.
4. `bribe`, `threaten`, and `declare_support` actions already exist in [office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs).
5. `succession_system()` and `public_order()` already exist in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs).

### Confirmed remaining gaps

1. `PlannerOpKind` in [planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) still has only 16 variants and intentionally excludes `bribe`, `threaten`, and `declare_support`.
2. `POLITICAL_OFFICE_OPS` in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) is still empty.
3. `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice` are still treated as never satisfied and non-progressing in the planner model.
4. `generate_candidates()` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) emits no political candidates.
5. `ranking.rs` currently assigns both political goals `GoalPriorityClass::Low` and motive `0`, which suppresses them via the existing zero-motive filter.
6. `GoalBeliefView` in [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs) does not expose office metadata, office holder visibility, faction membership, loyalty, or support declarations.

### Discrepancies from the prior ticket draft

1. The prior draft treated core office/faction types, actions, succession, and public order as future work for this ticket. They are already implemented and are out of scope here.
2. The prior draft assumed political AI could be added entirely inside `worldwake-ai`. That is false: the missing architectural seam is the belief/runtime view boundary in `worldwake-sim`.
3. The prior draft omitted ranking changes. Without ranking work, political goals still never execute.

## Architecture Check

### Required architectural approach

Political AI must read through a narrow AI-facing belief/runtime boundary, not by reaching into `World` directly from `worldwake-ai`.

That means this ticket must first extend `GoalBeliefView`/`RuntimeBeliefView` with the minimum political query surface needed by:

1. candidate generation
2. ranking
3. goal-model satisfaction/progress checks
4. planner payload construction where required

### Why this is better than the current architecture

This is more robust than adding ad hoc office lookups directly in AI code because:

1. It preserves the existing architectural rule that AI compiles against belief views, not `World`.
2. It keeps political planning composable with `PerAgentBeliefView`, `PlanningState`, and test stubs instead of adding one-off escape hatches.
3. It creates the correct seam for later E16b/E19 work, which will need office/control knowledge too.

### Known architectural limitation to note

The current belief model does not yet store first-class institutional beliefs such as "who this agent believes holds office X" or "who this agent believes belongs to faction Y." This ticket should not invent a parallel alias layer or planner-only cache.

For now, the implementation may expose the minimum public-structure / subjective query surface needed through the belief traits, but any such choice should stay explicit and narrow. If a cleaner future architecture emerges, it should be to extend institutional belief state directly, not to grow hidden AI-side shortcuts.

## What to Change

### 1. Extend the AI belief boundary for political queries

In [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs), add the minimum political queries required for AI:

1. office discovery / office data visibility
2. office-holder knowledge needed to reason about vacancy
3. faction membership visibility needed for eligibility checks
4. loyalty visibility needed for support-candidate generation and ranking
5. support declaration visibility needed to avoid repetitive declarations and support satisfaction/progress checks

Implement these queries in:

1. [per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs)
2. [planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs)
3. AI test belief-view stubs affected by the trait expansion

### 2. Add planner-op support for office actions

In [planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs):

1. add `PlannerOpKind::{Bribe, Threaten, DeclareSupport}`
2. classify `bribe`, `threaten`, and `declare_support`
3. define correct planner semantics for each

### 3. Finish political goal-model wiring

In [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs):

1. replace `POLITICAL_OFFICE_OPS = &[]` with real op sets
2. ensure political goals can build payload overrides when necessary
3. mark the appropriate political step(s) as progress barriers
4. add satisfaction/progress checks that prevent pointless re-declaration loops where the visible state already reflects the intended political stance

### 4. Emit political candidates

In [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs):

1. add `emit_political_candidates()`
2. generate `ClaimOffice` only when the agent has enough visible information to believe the office is vacant and the agent is eligible
3. generate `SupportCandidateForOffice` only when the agent has enough visible information to believe the office is vacant, the candidate is eligible, and loyalty is non-zero
4. respect `BlockedIntentMemory`
5. avoid duplicate or already-satisfied political goals where visible support state already matches the intended declaration

### 5. Rank political candidates

In [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs):

1. assign `ClaimOffice` the intended political priority class and non-zero motive based on `enterprise_weight`
2. assign `SupportCandidateForOffice` the intended political priority class and motive derived from `social_weight` and visible loyalty strength
3. keep existing suppression behavior under high danger / urgent self-care unless the design clearly warrants a change

## Files to Touch

- [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs)
- [per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs)
- [planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs)
- [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
- [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
- [planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs)

## Out of Scope

1. Reworking core office/faction schemas
2. Re-implementing office actions, succession, or public order
3. Adding a new institutional belief-storage subsystem unless strictly required to satisfy the boundary cleanly
4. E16b legitimacy/control-state work
5. Full multi-agent political scenario coverage beyond the targeted tests needed here

## Acceptance Criteria

### Behavior

1. `bribe`, `threaten`, and `declare_support` classify to planner ops and appear in planner semantics.
2. `ClaimOffice` exposes the relevant political op set instead of an empty slice.
3. `SupportCandidateForOffice` exposes the relevant political op set instead of an empty slice.
4. Political candidate generation reads through `GoalBeliefView` instead of directly depending on `World`.
5. Political goals are emitted only when visible political state supports them.
6. Political goals are not emitted when the office does not appear vacant to the acting agent.
7. Political goals are not emitted when eligibility fails.
8. Political ranking produces non-zero motive for valid political goals.
9. `ClaimOffice` can reach a planner terminal through a meaningful political step sequence.
10. `SupportCandidateForOffice` can reach a planner terminal through a meaningful political step sequence.
11. AI does not keep re-emitting or re-planning redundant political declarations when the visible support state already matches the goal.

### Invariants

1. AI-side political logic compiles against the belief/runtime-view boundary, not `World`.
2. No backward-compatibility alias layer is added.
3. Deterministic containers and integer arithmetic remain intact.
4. Existing non-political goal generation and ranking behavior remain stable.

## Test Plan

### New/Modified Tests

1. [planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs)
   Validate action classification and semantics for `bribe`, `threaten`, and `declare_support`.
2. [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
   Validate political op sets, progress barriers, payload overrides, and satisfaction/progress behavior.
3. [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
   Validate political emission for visible vacancy, eligibility, loyalty, blocked-intent suppression, and already-declared support.
4. [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
   Validate political priority/motive scoring and the zero-motive regression.
5. [per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs)
   Validate new political belief/runtime queries expose only the intended surface.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Extended the AI-facing belief/runtime boundary with narrow political queries for office data, visible office holders, faction membership, actor-private loyalty, and actor-private support declarations
  - Wired `PlannerOpKind::{Bribe, Threaten, DeclareSupport}` into action classification and planner semantics
  - Finished political goal modeling for `ClaimOffice` and `SupportCandidateForOffice`, including payload construction, progress barriers, and support-declaration satisfaction
  - Added political candidate generation and non-zero ranking so office goals can actually surface and compete
  - Seeded planning snapshots/planning state with support-declaration state and updated downstream exhaustive matches in AI runtime code
- Deviations from original plan:
  - The key architectural change was broader than the earlier draft assumed because the real missing seam was in `worldwake-sim`, not only in `worldwake-ai`
  - The implementation stayed intentionally narrow and did not introduce a separate institutional-belief cache or alias layer
  - `ClaimOffice` still does not model full campaign dynamics or institutional-memory richness; that remains future architecture work rather than hidden planner shortcuts
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo test -p worldwake-sim` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
