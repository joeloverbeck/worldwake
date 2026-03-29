# E18BANDYN-007: Planner contract alignment for raid and regroup goals

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` exact-goal terminal surfacing, focused planner/ranking/policy coverage
**Deps**: `specs/E18-bandit-dynamics.md`, `docs/planner-contracts.md`, E18BANDYN-002, E18BANDYN-006

## Problem

The original ticket narrative assumes raid/regroup still need new planner-op wiring, search-terminal branches, and custom ranking/switch logic. Live code already wires most of that through the shared goal-dispatch / goal-policy / planner-search architecture, but reassessment exposes one real discrepancy: combat exact-goal root synthesis currently treats `PlannerOpKind::Attack` as synthesizable from goal identity, which can bypass the co-location affordance boundary for both `GoalKind::RaidTarget` and the shared `GoalKind::EngageHostile` surface. The clean fix is to correct the ticket to the live architecture first, then remove the invalid shared combat synthesis path and lock the intended contracts with focused tests.

## Assumption Reassessment (2026-03-29)

1. The shared abstraction boundary under audit is exact-goal terminal surfacing for combat/travel goals across [`GoalKindPlannerExt::relevant_op_kinds()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), [`GroundedGoal::synthesized_root_candidate_targets()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), [`search::candidates::goal_synthesized_candidates()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs), and [`search::transition::terminal_kind()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/transition.rs).
2. `RegroupWithFaction` is already planner-supported through the live dispatch surface, not through new `planner_ops.rs` semantics entries. [`REGROUP_WITH_FACTION_OPS`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) already maps the goal to `PlannerOpKind::Travel`, [`GoalKind::is_satisfied()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) already terminates the goal at the believed rally place, and [`search_regroup_goal_uses_believed_rally_point_as_travel_destination()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) already proves the travel plan surface.
3. `RaidTarget` is already mapped to the existing combat operator family, but the live operator is `PlannerOpKind::Attack`, not a generic `Combat` planner op. [`RAID_TARGET_OPS`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) is attack-only, and [`GoalKind::is_satisfied()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) treats the goal as satisfied only once the target is dead or no longer a visible hostile.
4. The original claim that remote raid planning should be `Travel + Raid` does not match the live planner contract. `RaidTarget` currently has no `Travel` operator in its relevant-op surface, candidate generation only emits local raid targets in [`emit_raid_candidates()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), and [`docs/planner-contracts.md`](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md) does not list attack as a valid synthesized terminal family. If remote pursuit is desired later, it should be modeled as a separate pursuit/approach contract, not by smuggling travel into the exact local combat terminal.
5. Current code contradicts that planner contract by still allowing synthesized `Attack` root targets from goal identity inside [`GroundedGoal::synthesized_root_candidate_targets()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs). That shared path affects both `RaidTarget` and `EngageHostile`, so correcting it is an in-scope required consequence of fixing raid planning, not unrelated cleanup.
6. Ranking is also different from the original narrative. `RaidTarget` is declared as a danger-provenance goal in [`DECL_RAID_TARGET`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs), so the live ranked result is driven by danger pressure and `danger_weight`, not enterprise weight, via [`ranked_priority_class()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) and [`ranked_motive_score()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs). The fallback `priority_class()` / `motive_score()` arms for `RaidTarget` in [`ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) are not the authoritative live path while that provenance family remains `Danger`.
7. `RegroupWithFaction` does not currently sit in a dedicated class above enterprise goals. Live ranking assigns it `GoalPriorityClass::Medium` with motive from `social_weight` in [`ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs). This is cleaner than introducing a one-off priority class here; if future behavior needs a stronger class, that should be a principled faction-coordination ranking change shared across similar goals.
8. Suppression and interrupt policy are already centralized in [`goal_policy.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs), not goal-specific branches in `goal_switching.rs`. `RaidTarget` is a normal unsuppressed combat/enterprise-family goal. `RegroupWithFaction` is suppressed when stress is at or above `High` through the shared goal-family policy, not only at “Critical danger” as the original ticket stated.
9. Goal switching is class/role-driven. [`compare_goal_switch()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_switching.rs) is intentionally generic, while [`evaluate_interrupt()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/interrupts.rs) gates free interrupts through `FreeInterruptRole`. This means survival/reactive goals can interrupt raid goals when they outrank them, while raid/regroup themselves remain normal goals and cannot preempt reactive survival work.
10. Existing focused coverage proves pieces of the live contract but leaves the newly exposed shared combat-synthesis bug under-covered. Current coverage includes regroup travel search, goal-op dispatch, and binding checks in [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), but there is no focused test proving that raid/combat exact goals do not synthesize remote attack commitments.
11. Mismatch + correction: this ticket is not about adding new planner op kinds, new search terminal branches, or new goal-switch kinds. It is about aligning the bandit goals to the live shared planner contract, removing the invalid synthesized combat shortcut, and adding focused tests for the correct planner/ranking/policy behavior.

## Architecture Check

1. Reusing the existing `Travel` and `Attack` operator families is cleaner than inventing raid-specific planner ops. `RegroupWithFaction` is just destination travel; `RaidTarget` is just local combat commitment once lawful combat is actually available.
2. Removing synthesized `Attack` root candidates is cleaner than teaching the planner to fabricate local combat legality from goal identity. Co-location and combat availability should come from real affordances or from a future explicit pursuit substrate, not from an alias path in root synthesis.
3. Keeping ranking and interrupt behavior inside the shared goal-policy / provenance system is more robust than adding raid/regroup-specific special cases. If the project later wants a distinct “faction coordination” class, that should be a shared architecture change, not a one-ticket bandit exception.
4. No backwards-compatibility shims or alias paths: the old synthesized combat shortcut should be removed outright rather than preserved behind conditional fallbacks.

## Verification Layers

1. `RegroupWithFaction` remains a belief-backed travel goal -> focused planner search test on [`search_plan()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/mod.rs) plus exact planned-step assertions.
2. `RaidTarget` only commits through local combat, not synthesized remote attack -> focused goal-model/root-synthesis test plus focused planner search test proving no remote combat commitment plan is fabricated.
3. `RaidTarget` ranking follows danger provenance rather than enterprise weight -> focused ranking unit test inspecting `priority_class`, `motive_score`, and provenance.
4. `RegroupWithFaction` ranking/policy follow the shared medium/social/high-stress-suppressed contract -> focused ranking and goal-policy unit tests.
5. Survival/reactive work can interrupt raid while raid/regroup remain normal goals -> focused interrupt/goal-policy tests rather than scenario-level inference.
6. No broader regression in AI planning surfaces -> `cargo test -p worldwake-ai`, `cargo clippy --workspace`, and `cargo build --workspace`.

## What to Change

### 1. Remove synthesized combat root targets

In [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs):
- remove the `PlannerOpKind::Attack` root-synthesis path from `GroundedGoal::synthesized_root_candidate_targets()`
- leave `RaidTarget` / `EngageHostile` payload and binding logic intact for real affordances
- keep regroup travel behavior unchanged

### 2. Add focused contract tests

Add or strengthen tests to prove:
- `RaidTarget` co-located search still yields local `Attack` combat commitment when a lawful attack affordance exists
- remote `RaidTarget` does not get a fabricated synthesized attack plan
- shared combat exact-goal synthesis is absent at the root-synthesis layer
- `RaidTarget` ranking uses danger provenance
- `RegroupWithFaction` uses medium/social ranking and high-stress suppression through shared policy

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify tests)
- `crates/worldwake-ai/src/goal_policy.rs` (modify tests)
- `crates/worldwake-ai/src/interrupts.rs` (modify tests)
- `tickets/E18BANDYN-007.md` (modify)

## Out of Scope

- Adding a new `Raid` action or raid-specific planner op
- Making `RaidTarget` a remote pursuit goal with `Travel + Attack`
- Introducing a new priority class just for `RegroupWithFaction`
- Candidate generation mechanics from E18BANDYN-006 beyond proving the planner contract they target
- Route danger estimation or camp mechanics from other E18 tickets

## Acceptance Criteria

### Tests That Must Pass

1. `RegroupWithFaction` still produces a one-step travel plan to the believed rally place
2. Co-located `RaidTarget` still produces a local `Attack` combat-commitment plan
3. Remote or no-longer-local `RaidTarget` does not produce a fabricated synthesized attack commitment
4. `GroundedGoal::synthesized_root_candidate_targets()` no longer synthesizes `Attack` for combat exact goals
5. `RaidTarget` ranks from danger provenance rather than enterprise weight
6. `RegroupWithFaction` remains medium-priority with social-weight motive and high-stress suppression
7. Survival/reactive goals still outrank and can interrupt raid work through the existing shared interrupt rules
8. Existing suite: `cargo test -p worldwake-ai`
9. Existing suite: `cargo clippy --workspace`
10. Existing suite: `cargo build --workspace`

### Invariants

1. Exact local combat legality comes from lawful affordances, not synthesized target aliases
2. `RegroupWithFaction` remains a belief-only travel goal using the faction rally-point information path
3. `RaidTarget` and `EngageHostile` continue sharing the same combat terminal boundary; this ticket must not fix raid by leaving a broken parallel path on engage-hostile
4. No new planner op kinds, switch kinds, or backwards-compatibility shims

## Test Plan

### New/Modified Tests

1. [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) — prove combat goals no longer synthesize `Attack` root targets and that local/remote raid search obeys the corrected planner contract
2. [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) — prove `RaidTarget` uses danger provenance and `RegroupWithFaction` uses the live medium/social contract
3. [`crates/worldwake-ai/src/goal_policy.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs) — prove the live suppression / free-interrupt contract for raid and regroup rather than the stale ticket narrative
4. [`crates/worldwake-ai/src/interrupts.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/interrupts.rs) — prove reactive danger response interrupts raid work while raid does not preempt danger response

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace`
3. `cargo build --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - corrected the ticket to the live planner architecture instead of the stale “new planner op / search branch / enterprise raid ranking” narrative
  - removed `PlannerOpKind::Attack` root-target synthesis for `RaidTarget` and `EngageHostile`, so local combat legality now comes from lawful affordances rather than synthesized aliases
  - added focused tests for raid/regroup planner, ranking, goal-policy, and interrupt behavior
- Deviations from original plan:
  - did not add new planner-op semantics, search-terminal branches, or goal-switch kinds because those were already delivered or architecturally incorrect for the live system
  - did not implement remote `Travel + Raid`; the clean architecture remains local combat goals today, with any future pursuit behavior deferred to a separate explicit approach/pursuit contract
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace` passed
  - `cargo build --workspace` passed
