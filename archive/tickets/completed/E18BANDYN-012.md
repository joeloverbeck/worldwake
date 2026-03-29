# E18BANDYN-012: Bandit regroup and camp-establishment omission traceability

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` candidate-generation diagnostics and decision-trace surfacing for bandit regroup / establish-camp omissions
**Deps**: archive/tickets/completed/E18BANDYN-009.md, archive/tickets/completed/E18BANDYN-011.md, archive/tickets/completed/AITRACE-001-decision-trace-candidate-pipeline-history.md

## Problem

The T22 closeout proved the live E18 behavior, but the debugging contract is still weaker than the architecture deserves. Today a developer can prove that `GoalKind::RegroupWithFaction` or `GoalKind::EstablishBanditCamp` was eventually selected, and focused tests can prove the authoritative `establish_camp` legality rules, but the decision trace does not explain why those bandit-specific candidates were omitted at the candidate boundary. That forces source-diving into `emit_regroup_with_faction_goals()` when the real question is architectural: did the agent lack rally doctrine, stay suppressed because it was already safe in an active camp, or reach rally without lawful local controlled supplies?

## Assumption Reassessment (2026-03-30)

1. The live goal families under audit are [`GoalKind::RegroupWithFaction`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) and [`GoalKind::EstablishBanditCamp`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs). Their current candidate-generation surface is centralized in [`emit_regroup_with_faction_goals()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), not in planner search or in the authoritative action layer.
2. The current candidate-generation diagnostics only store [`omitted_political`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) and [`omitted_social`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs). There is no parallel omission surface for bandit goals, so bandit candidate absence falls through to a generic “not generated” result in decision-trace status instead of a concrete reason.
3. The current decision-trace status helper only maps omission reasons for political and social families through [`omitted_political_reason_for_goal()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) and [`omitted_social_reason_for_goal()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs). The live runtime trace path also threads diagnostics through [`ReadPhaseResult`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/observation.rs) into [`CandidateTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs). Bandit omission traceability therefore needs end-to-end plumbing through the existing candidate-trace pipeline, not just a helper in `decision_trace.rs`.
4. The authoritative legality boundary is already explicit in [`validate_establish_camp()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/bandit_camp_actions.rs): faction membership, place tag, existing active camp rules, minimum regroup count, and concrete locally controlled edible supplies at the place. Focused tests already prove that lower layer, including `bandit_camp_actions::tests::establish_camp_accepts_ground_supplies_under_local_control` and `bandit_camp_actions::tests::establish_camp_reuse_expands_same_place_camp_capacity_for_new_supplies`.
5. The shared abstraction boundary under audit is bandit rally doctrine at the candidate layer plus lawful camp re-establishment prerequisites at the authoritative layer. This ticket is about restoring explanation at the candidate boundary; it should not duplicate the authoritative legality proof already owned by focused `bandit_camp_actions` coverage.
6. The motivating mixed-layer scenario is [`golden_t22_bandit_camp_destruction`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs). The intended invariant is: when regroup or re-establishment does not appear, the trace should identify the concrete bandit-family omission reason without requiring ad hoc debug output or code inspection.
7. The live bandit candidate surface depends on three concrete substrates, all visible in `emit_regroup_with_faction_goals()`: institutional rally belief, locally observed same-faction active camp presence, and local controlled edible supplies. If omission reasoning is added, it must talk in those concrete terms rather than abstract “cannot regroup” buckets.
8. This ticket should not broaden into route-choice traceability. The downstream merchant-route explanation gap is separate and belongs in its own planner traceability ticket.
9. This ticket should not broaden into generic request-resolution or action-trace redesign. `establish_camp` legality is already best proven at the focused authoritative layer today; the missing explanation surface is earlier, at candidate generation.
10. Mismatch + correction: the repo already has good lower-layer tests for `establish_camp`, and `worldwake-ai` already has focused emission/suppression tests for the live bandit goals (`regroup_with_faction_requires_rally_point_belief`, `regroup_with_faction_is_suppressed_while_agent_stands_in_active_faction_camp`, `establish_bandit_camp_requires_local_controlled_edible_supplies`). The right follow-up is to strengthen those focused tests with omission assertions and then thread the new omission data through the existing `agent_tick` -> `CandidateTrace` -> `goal_status()` pipeline.

## Architecture Check

1. Adding a bounded `BanditCandidateOmission` surface is cleaner than relying on source-diving because it keeps bandit-family reasoning inside the existing candidate-generation diagnostics architecture instead of inventing a one-off debug helper.
2. The clean design is concrete and local: omission reasons should talk about rally-belief absence, observed same-faction camp presence, safe-in-camp suppression, and missing locally controlled edible supplies. Those are real world/belief facts, not abstract AI mood labels.
3. This aligns with `docs/FOUNDATIONS.md`: explainable emergence requires developer-visible causal explanations, and locality requires the explanation to point at explicit carriers such as institutional beliefs and local observations rather than hidden planner magic.
4. No backwards-compatibility aliasing/shims introduced. This ticket should extend the existing omission-diagnostics path, not add a parallel “bandit debug mode.”

## Verification Layers

1. Bandit candidate absence because the actor lacks rally doctrine -> focused candidate-generation test plus decision-trace goal-status omission reason.
2. Bandit candidate absence because the actor is already safe in an observed active same-faction camp -> focused candidate-generation test plus decision-trace goal-status omission reason.
3. Bandit candidate absence because the actor reached rally without lawful local controlled edible supplies -> focused candidate-generation test plus decision-trace goal-status omission reason.
4. Authoritative `establish_camp` legality remains unchanged -> existing focused `bandit_camp_actions` tests and, if needed, one additional focused authoritative regression.
5. Golden T22 remains the mixed-layer proof surface only for “trace now explains the omitted bandit branch without source-diving,” not for re-proving all lower-layer legality branches.
6. If traces still prove the outcome but not enough provenance after this ticket, the remaining gap should become a separate request/action-trace ticket rather than broadening this one.

## What to Change

### 1. Add structured bandit omission diagnostics

Extend `CandidateGenerationDiagnostics` with a bandit-family omission channel analogous to the existing social/political omission surfaces.

The omission reasons should stay concrete and bounded to the live candidate surface, for example:

- missing rally belief for the faction
- already safe in a locally observed active same-faction camp
- already at rally while a locally observed same-faction camp is active there
- already at rally but lacking lawful local controlled edible supplies

Do not introduce abstract “regroup blocked” or “camping impossible” summary enums that hide the actual causal fact.

### 2. Surface those omissions through the existing decision-trace pipeline

Extend the existing `agent_tick`/decision-trace plumbing so `GoalKind::RegroupWithFaction` and `GoalKind::EstablishBanditCamp` can report concrete omission reasons the same way political/social goals already do.

The trace summary and `goal_status()` output should make it possible to answer “why didn’t this bandit goal appear?” directly from the trace.

### 3. Strengthen focused and mixed-layer coverage

Strengthen the existing focused candidate-generation regressions to assert the omission reasons directly, add one end-to-end `agent_tick` trace assertion for `goal_status()`, and add the narrowest golden assertion needed in T22 to prove the improved debugging contract without bloating the scenario.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify)

## Out of Scope

- changing bandit behavior, ranking, or `establish_camp` legality
- changing route-threat math or merchant route selection
- generic request-resolution trace redesign
- ad hoc debug dumps or test-only logging helpers

## Acceptance Criteria

### Tests That Must Pass

1. A focused candidate-generation test proves `RegroupWithFaction` omission reports missing rally doctrine when the agent is a faction member but lacks the institutional belief.
2. A focused candidate-generation test proves `RegroupWithFaction` omission reports safe-in-camp suppression when the agent is in a locally observed active same-faction camp with no wounds and no visible hostiles.
3. A focused candidate-generation test proves `EstablishBanditCamp` omission reports missing lawful local controlled edible supplies when the agent is at rally without a same-faction camp there.
4. A runtime `agent_tick` trace test proves `goal_status()` surfaces a bandit-family omission reason end-to-end through the live candidate-trace pipeline.
5. `golden_t22_bandit_camp_destruction` proves the trace can explain at least one intentionally omitted bandit branch without source-diving.
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Bandit omission reasons remain concrete state / belief facts, not abstract strategic labels.
2. The canonical rally-doctrine information path remains institutional belief -> candidate generation -> planning. The traceability work must not add a second behavioral path.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs::regroup_with_faction_requires_rally_point_belief` — strengthened.
Rationale: proves candidate-generation omission records `MissingRallyBelief` at the earliest causal boundary instead of falling through to generic non-generation.
2. `crates/worldwake-ai/src/candidate_generation.rs::regroup_with_faction_is_suppressed_while_agent_stands_in_active_faction_camp` — strengthened.
Rationale: proves safe-in-camp suppression is preserved as an explicit bandit omission reason rather than only as an absent candidate.
3. `crates/worldwake-ai/src/candidate_generation.rs::establish_bandit_camp_requires_local_controlled_edible_supplies` — strengthened.
Rationale: proves rally-arrival without lawful local controlled edible supplies records the specific establishment omission reason.
4. `crates/worldwake-ai/src/agent_tick/tests.rs::trace_bandit_regroup_missing_rally_omission_reason` — new.
Rationale: proves the live `ReadPhaseResult` -> `CandidateTrace` -> `goal_status()` pipeline surfaces the omission reason end-to-end during runtime tracing.
5. `crates/worldwake-ai/src/decision_trace.rs::goal_status_reports_bandit_omission_reason` — new.
Rationale: keeps goal-to-omission lookup coverage local and deterministic for the new bandit-family helper.
6. `crates/worldwake-ai/src/decision_trace.rs::bandit_omission_helper_only_matches_bandit_goal_families` — new.
Rationale: prevents cross-family false positives in omission lookup as the trace surface grows.
7. `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` — strengthened.
Rationale: proves the mixed-layer debugging contract directly in T22 by checking that the outsider’s missing-rally branch is explained in decision traces without source-diving.

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::regroup_with_faction_requires_rally_point_belief -- --nocapture`
2. `cargo test -p worldwake-ai candidate_generation::tests::regroup_with_faction_is_suppressed_while_agent_stands_in_active_faction_camp -- --nocapture`
3. `cargo test -p worldwake-ai candidate_generation::tests::establish_bandit_camp_requires_local_controlled_edible_supplies -- --nocapture`
4. `cargo test -p worldwake-ai golden_t22_bandit_camp_destruction -- --nocapture`
5. `cargo test -p worldwake-systems bandit_camp_actions::tests::establish_camp_reuse_expands_same_place_camp_capacity_for_new_supplies -- --nocapture`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-30
- What actually changed:
  - Added a typed `BanditCandidateOmission` trace surface with concrete omission reasons for bandit regroup and camp-establishment candidates.
  - Threaded the new omission diagnostics through the live `agent_tick` read pipeline into `CandidateTrace` and `GoalTraceStatus`.
  - Strengthened focused candidate-generation coverage, added runtime and helper trace coverage, and tightened T22 so the outsider’s missing-rally branch is explained directly by decision traces.
- Deviations from original plan:
  - The ticket was corrected before implementation to acknowledge the required `agent_tick` plumbing. This was a real scope dependency in the live architecture, not optional follow-up cleanup.
  - Existing focused candidate-generation tests were strengthened instead of replaced, because the live repo already had the right candidate-level proof surfaces.
- Verification results:
  - Passed `cargo test -p worldwake-ai candidate_generation::tests::regroup_with_faction_requires_rally_point_belief -- --nocapture`
  - Passed `cargo test -p worldwake-ai candidate_generation::tests::regroup_with_faction_is_suppressed_while_agent_stands_in_active_faction_camp -- --nocapture`
  - Passed `cargo test -p worldwake-ai candidate_generation::tests::establish_bandit_camp_requires_local_controlled_edible_supplies -- --nocapture`
  - Passed `cargo test -p worldwake-ai agent_tick::tests::trace_bandit_regroup_missing_rally_omission_reason -- --nocapture`
  - Passed `cargo test -p worldwake-ai decision_trace::tests::goal_status_reports_bandit_omission_reason -- --nocapture`
  - Passed `cargo test -p worldwake-ai golden_t22_bandit_camp_destruction -- --nocapture`
  - Passed `cargo test -p worldwake-systems bandit_camp_actions::tests::establish_camp_reuse_expands_same_place_camp_capacity_for_new_supplies -- --nocapture`
  - Passed `cargo test -p worldwake-ai`
  - Passed `cargo clippy --workspace --all-targets -- -D warnings`
