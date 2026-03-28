# S33OPPSCOGOAIDE-002: Refactor GroundedGoal to carry OpportunityAnchor and emit per-opportunity

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `GroundedGoal`, candidate generation, blocker matching, ranking tie-breaks, facility-queue abandonment bookkeeping
**Deps**: S33OPPSCOGOAIDE-001, `specs/S33-opportunity-scoped-goal-identity.md`

## Problem

`worldwake-core` already had `OpportunityAnchor` / `OpportunityKey`, but the AI layer was still collapsing multiple concrete opportunities for one `GoalKey` into a single `GroundedGoal`. The live merge point was inside candidate generation, where multiple concrete sources unioned evidence into one candidate and blocker checks ran before the full opportunity set existed.

That made same-desire opportunities alias each other in ways that weakened blocker precision, obscured evidence provenance, and let ordering depend on incidental merged state.

## Assumption Reassessment (2026-03-28)

1. `OpportunityAnchor` and `OpportunityKey` were already present in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs). This ticket did not need to add them.
2. `GroundedGoal` in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) still lacked `anchor`, and candidate generation still merged by `GoalKey`.
3. The public candidate-generation boundary already returned `Vec<GroundedGoal>`; the real problem was the internal desire-level merge in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs).
4. The planning layer still built one merged snapshot from unioned candidate evidence in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs). Live golden coverage showed that removing that union immediately would regress legitimate plans, so full per-candidate planning snapshots were not safe to ship in this ticket.
5. `AgentDecisionRuntime.exhaustion_cache` and `PlannedPlan` remain `GoalKey`-scoped. Opportunity-keyed exhaustion and plan persistence stay in later S33 tickets.

## Architecture Decision

1. Adding `anchor: OpportunityAnchor` directly to `GroundedGoal` is the right boundary. `GroundedGoal` is already the concrete ranked/planned candidate payload, so a wrapper type would only create another alias surface.
2. Emitting one `GroundedGoal` per concrete opportunity is better than the old `GoalKey` merge because it preserves desire identity in `GoalKey` while making opportunity identity explicit and testable.
3. Post-emission blocker filtering is better than pre-emission global suppression, but anchor alone is not enough. Some blockers are scoped by concrete evidence entities such as facilities or sources, so the blocker matcher must consult candidate evidence as well as the anchor.
4. The merged planning snapshot remains deferred. The cleaner long-term architecture is still per-candidate planning scope, but live code is not ready for that cut yet. Shipping the candidate identity split without destabilizing planning was the better tradeoff here.
5. The ranking change needed here is narrow: keep existing motive-score contracts stable, but add a deterministic self-care opportunity-strength tie-break so stronger self-care outputs win over weaker substitutes when all higher-order ranking dimensions tie.
6. Queue patience handling is cleaner when the authoritative abandonment step also writes the blocked-facility memory and clears queued-facility intent state, rather than relying on later observation inference.

## What Changed

### 1. `GroundedGoal` now carries `anchor`

[`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) now includes `pub anchor: OpportunityAnchor` on `GroundedGoal`, and constructor sites across `worldwake-ai` were updated explicitly.

### 2. Candidate generation now emits per-opportunity candidates

[`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) no longer merges candidates internally by `GoalKey`. It now emits one `GroundedGoal` per concrete opportunity, including distinct anchors for:

- concrete places for multi-source acquire / produce / cargo opportunities
- concrete entities for targeted social, combat, theft, justice, corpse, and care opportunities
- `OpportunityAnchor::None` for self-only goals like direct consume, sleep, relieve, and wash

### 3. Blocker filtering moved to post-emission and now matches real opportunity scope

Candidate filtering now happens after emission. Matching is no longer based only on a pre-emission global `GoalKey` query. The live filter checks:

- `GroundedGoal.anchor`
- `GroundedGoal.evidence_places`
- `GroundedGoal.evidence_entities`

This preserves facility/source/seller-specific suppression instead of collapsing everything that shares a desire.

### 4. Ranking kept its motive contract and gained a narrower tie-break

[`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) keeps the existing motive-score contract for direct self-care and downstream recipe-input ranking. To avoid incidental ordering between equally-scored self-care commodity opportunities, it now carries relief magnitude in drive provenance and applies an `OpportunityStrength` tie-break after motive score, rather than redefining motive scores globally.

### 5. Planning scope stayed merged for now, with explicit guardrails

[`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) still builds a merged evidence snapshot. This ticket instead added deterministic first-per-`GoalKey` planning dedup so per-opportunity generation does not explode identical desire-level search attempts while the planner remains `GoalKey`-scoped.

### 6. Queue patience abandonment now records blockers at the authoritative change point

[`crates/worldwake-ai/src/agent_tick/candidates.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/candidates.rs) now persists `ExclusiveFacilityUnavailable` blocked intent memory and clears queued-facility intent state in the same abandonment transaction. This removed a fragile observation-time dependency and stabilized the patience-expiry path.

## Files Touched

- [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
- [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
- [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs)
- [`crates/worldwake-ai/src/agent_tick/candidates.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/candidates.rs)
- [`crates/worldwake-ai/src/agent_tick/observation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/observation.rs)
- [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs)
- [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs)
- [`crates/worldwake-ai/tests/golden_production.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs)

## Out of Scope

- introducing `OpportunityAnchor` / `OpportunityKey` in `worldwake-core`
- opportunity-keyed exhaustion cache / invalidation
- `PlannedPlan.opportunity` persistence
- replacing merged planning snapshots with fully per-candidate planning snapshots
- save/load version changes tied to persisted opportunity identity

## Acceptance Criteria

1. `GroundedGoal` carries explicit opportunity identity through `anchor`.
2. Candidate generation no longer merges by `GoalKey`.
3. Multi-source opportunities emit distinct candidates with isolated evidence.
4. Blocker filtering suppresses matching concrete opportunities rather than all desire-level siblings.
5. Ranking remains stable for existing self-care contracts while preferring stronger tied commodity opportunities deterministically.
6. Queue abandonment writes blocked-facility memory at the authoritative transition point.
7. `cargo test -p worldwake-ai`, `cargo test --workspace`, and `cargo clippy --workspace` pass.

## Test Plan

### New/Modified Tests

1. `candidate_generation::tests::remote_harvest_source_within_travel_horizon_emits_produce_goal`
2. `candidate_generation::tests::acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`
3. `candidate_generation::tests::blocked_acquire_place_only_suppresses_matching_opportunity`
4. `agent_tick::tests::abandoned_queue_then_records_standard_exclusive_facility_blocker`
5. `golden_contested_harvest_start_failure_recovers_via_remote_fallback`
6. `golden_facility_queue_patience_timeout`

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai candidate_generation::tests::`
3. `cargo test -p worldwake-ai --test golden_production golden_materialized_output_ownership_prevents_theft -- --nocapture`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - added `GroundedGoal.anchor`
  - removed internal desire-level candidate merge
  - switched blocker filtering to post-emission opportunity matching using anchors plus evidence scope
  - added deterministic opportunity-strength tie-breaks for tied self-care commodity opportunities
  - persisted queue-abandonment blockers at the authoritative abandonment step
  - updated focused and golden coverage around multi-source opportunity identity, blocker scope, queue abandonment, and production recovery
- Deviations from original plan:
  - did not replace merged planning snapshots with per-candidate snapshots; live golden coverage showed that was not yet safe
  - added queue-abandonment persistence cleanup and a narrow ranking tie-break because the opportunity split exposed those weaknesses in live behavior
  - corrected the patience-timeout golden to match the finite blocked-intent dampener contract rather than requiring permanent reroute to the alternate facility
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
