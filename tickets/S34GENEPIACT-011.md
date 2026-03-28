# S34GENEPIACT-011: Replace standalone VerifyBelief goals with originating-goal epistemic progress barriers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-core` goal contract cleanup, `worldwake-ai` candidate/search/goal-model refactor, S34 spec correction, focused AI + golden coverage updates
**Deps**: [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md), [archive/tickets/not-implemented/S34GENEPIACT-008.md](/home/joeloverbeck/projects/worldwake/archive/tickets/not-implemented/S34GENEPIACT-008.md), [archive/tickets/completed/S34GENEPIACT-009.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-009.md)

## Problem

Live S34 epistemic behavior still routes deliberate verification through a standalone ranked `GoalKind::VerifyBelief`. That creates a structural mismatch with the foundations: the agent can want the originating world-condition goal and also emit a separate verification goal that competes with it, instead of keeping one intention whose stale assumptions are checked through explicit prerequisite work.

The clean fix is not to add a second prerequisite-barrier path beside the current model. The clean fix is to replace the standalone `VerifyBelief` goal path entirely so the canonical architecture becomes:

- originating goal remains the top-level intention
- stale supporting beliefs derive explicit `verify_belief` / `ask_witness` barrier steps inside that goal's plan
- epistemic actions stay first-class actions with duration, cost, aftermath, and typed traces
- no rival top-level verification goal remains live

## Assumption Reassessment (2026-03-28)

1. The live AI layer still emits standalone top-level verification goals. `emit_verify_belief_goals()` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) scans already-emitted candidates, derives `VerificationSubject` from stale evidence, and appends a distinct `GoalKind::VerifyBelief { subject, generation_tick }`.
2. The live ranking/suppression contract still treats verification as its own goal family. [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) gives `GoalKind::VerifyBelief` its own motive/provenance path, and [goal_policy.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_policy.rs) suppresses it as a distinct low-priority goal family.
3. The exact shared abstraction boundary under audit is mixed-layer but centered on the AI-internal stale-evidence contract:
   - AI/belief/planning layer: grounded-goal evidence, stale-belief detection, root-candidate synthesis, progress-barrier handling in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and [search/](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search)
   - authoritative action layer: explicit epistemic action payloads and commits in [epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs)
   - trace layer: typed epistemic lifecycle identity in [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs)
4. Existing focused coverage already proves the current standalone path, so this is not a "missing tests only" ticket:
   - candidate-generation coverage for standalone verification emission exists in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
   - planner coverage for `Travel -> VerifyBelief` and `AskWitness` under `GoalKind::VerifyBelief` exists in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
   - action-trace coverage for typed epistemic details exists in [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs)
5. Existing golden coverage proves adjacent recovery, not the desired deliberate barrier architecture. `golden_stale_prerequisite_belief_discovery_replan` in [golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) already proves live `GoalKind::RestockCommodity` can recover from a stale branch after passive local discovery, but it does not prove deliberate epistemic verification was selected as the prerequisite barrier.
6. The live S34 spec still describes the now-problematic architecture. [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md) still specifies a proactive `GoalKind::VerifyBelief`, standalone candidate generation in `emit_verify_belief_goals()`, and ranking of verification as its own top-level goal family.
7. The live goal-family surface under test for the motivating stale-source scenario is `GoalKind::RestockCommodity { commodity: Bread }`, not `ProduceCommodity`, and the live stale branch is a resource-source belief surfaced through candidate evidence in [golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs). Any refactor must preserve that live `RestockCommodity` contract rather than reintroducing a stale `ProduceCommodity` narrative.
8. This is an information-path refactor inside the AI layer. The same real-world fact currently has two lawful AI transport paths:
   - canonical today: top-level `VerifyBelief` candidate emitted from stale grounded-goal evidence
   - adjacent existing fallback: originating goal continues until passive discovery or start-failure forces same-goal branch replacement
   The canonical end state after this ticket should be one path only: originating goals derive explicit epistemic barrier steps from their own stale evidence, and the standalone top-level `VerifyBelief` path is removed in-scope.
9. Ordering-sensitive claims in this ticket are planner/action-lifecycle claims, not event-log-ordering claims:
   - originating goal selection and barrier insertion -> decision trace / focused planner tests
   - committed epistemic action identity -> action trace
   - belief refresh / contradiction aftermath -> authoritative belief and violation state
   Later downstream effects must not be used as a proxy for the earlier selection contract when lower-layer proof surfaces exist.
10. The clean architecture likely requires removing `GoalKind::VerifyBelief` from the live goal model, not merely stopping its candidate emission. Leaving the variant, ranking path, or suppression family alive as an alternate route would preserve the same dual-path contradiction under a different name.
11. Adjacent contradiction classification:
   - required consequence of this ticket: S34 spec text and focused planner tests must be rewritten to the originating-goal barrier contract
   - future follow-up, not in scope here: new deliberate epistemic goldens remain in [S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md) once this canonical architecture exists
12. Mismatch + correction: the archived [S34GENEPIACT-008.md](/home/joeloverbeck/projects/worldwake/archive/tickets/not-implemented/S34GENEPIACT-008.md) correctly identified the architectural direction, but it understated the breadth of the live contract and the amount of existing proof already in the repo. This ticket corrects that scope into a one-shot replacement instead of an incremental patch.

## Architecture Check

1. Replacing standalone `VerifyBelief` with originating-goal epistemic barriers is cleaner than tuning ranking, motive weights, or suppression around the current competing-goal model. That current model splits one intention into two rival top-level desires, which is weaker than the foundations' revisable-commitment model.
2. The cleanest robust design is:
   - derive an AI-internal stale-evidence barrier requirement from `GroundedGoal` evidence and the actor's belief state
   - allow originating goals to surface `verify_belief` / `ask_witness` as explicit progress barriers when that requirement exists
   - keep epistemic action payloads and traces unchanged as the explicit world-action layer
   - remove the standalone `VerifyBelief` goal family entirely so no alias path survives
3. This aligns with [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md):
   - Principle 18: goals name desired world conditions and include enabling subchains
   - Principle 19: commitments are revisable when assumptions break
   - Principle 24: epistemic actions remain explicit state-mediated actions, not planner magic
4. No backwards-compatibility aliasing or dual-path coexistence is acceptable here. The old standalone verification goal path must be removed in-scope if the new barrier path lands.

## Verification Layers

1. Standalone top-level verification candidate path no longer exists -> focused candidate-generation tests in `worldwake-ai`
2. Originating goals synthesize `verify_belief` / `ask_witness` as explicit progress barriers from stale evidence -> focused planner/search tests and decision-trace-facing runtime tests in `worldwake-ai`
3. Committed epistemic action identity remains explicit and typed -> `action_trace` tests in `worldwake-sim` plus any focused runtime consumption tests in `worldwake-systems`
4. Belief refresh or contradiction aftermath still mutates the same authoritative belief / violation state -> focused epistemic action tests in `worldwake-systems`
5. Existing stale-prerequisite recovery scenarios still work under the new canonical path -> updated focused/golden regression in `worldwake-ai`
6. New end-to-end deliberate epistemic scenarios remain out of scope here and are owned by [S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md)

## What to Change

### 1. Correct the S34 spec to the canonical barrier model

Update [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md) so it no longer treats `GoalKind::VerifyBelief` as the canonical proactive goal architecture.

The corrected spec should:

- state that deliberate verification remains explicit action-level behavior
- define stale-evidence verification as an originating-goal planning barrier, not a separate top-level desire
- name the canonical information path and explicitly remove the old standalone verification path in-scope
- update test expectations so focused/golden coverage targets the originating-goal barrier contract

### 2. Remove the standalone top-level verification goal path

Refactor the live code so the AI no longer emits or ranks standalone `GoalKind::VerifyBelief` candidates. If the cleanest implementation is to delete `GoalKind::VerifyBelief` from [goal.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) and its downstream AI use sites, do that rather than leaving a dormant or parallel path behind.

At minimum this replacement must remove the old authority path from:

- `emit_verify_belief_goals()` candidate emission in `worldwake-ai`
- ranking/suppression/provenance handling that treats verification as its own top-level goal family
- goal-model planner tests and operator surfaces that only exist to support standalone verification goals

### 3. Add a canonical AI-internal stale-evidence barrier substrate

Introduce one canonical AI-internal helper or data contract that derives explicit epistemic barrier requirements from a grounded goal's stale evidence. The exact type name can vary, but it must:

- derive from the actor's current belief state plus the grounded goal's evidence
- synthesize lawful `VerificationSubject` / witness-topic requirements for `verify_belief` and `ask_witness`
- be consumed by the originating goal's search/root-candidate logic rather than by a separate top-level goal family
- remain explainable in decision traces and focused planner tests

This substrate belongs in `worldwake-ai`, not `worldwake-core`, because it is an AI planning/read-model concern rather than world identity.

### 4. Rewire planner/search behavior to use originating-goal epistemic barriers

Update planner/search logic so originating goals with stale evidence can lawfully choose:

- `AskWitness` when a co-located witness payload matches the stale subject
- `Travel -> VerifyBelief` when the stale subject must be checked at a remote place

These steps should remain explicit progress barriers under the originating goal, not hidden planner transitions. The planner should still stop at the epistemic barrier and replan after the action commits, preserving the current explicit-action contract.

### 5. Replace focused proof surfaces to the new contract

Rewrite the focused tests so they prove:

- stale evidence on an originating goal yields epistemic barrier candidates under that same goal
- standalone verification goals are gone
- explicit epistemic action identity and aftermath are unchanged
- existing stale-prerequisite recovery scenarios still succeed under the new canonical path

## Files to Touch

- `specs/S34-general-epistemic-actions.md` (modify — correct the canonical S34 architecture)
- `crates/worldwake-core/src/goal.rs` (modify — remove or repurpose standalone `VerifyBelief` goal support if no longer canonical)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — remove standalone verification emission and introduce originating-goal stale-evidence barrier derivation)
- `crates/worldwake-ai/src/goal_model.rs` (modify — update relevant op surfaces, payload overrides, barrier semantics, and focused tests)
- `crates/worldwake-ai/src/search/` (modify — wire originating-goal search/root-candidate handling to the barrier substrate)
- `crates/worldwake-ai/src/ranking.rs` (modify — remove standalone verification ranking/provenance path)
- `crates/worldwake-ai/src/goal_policy.rs` (modify — remove standalone verification family policy if it no longer exists)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` or another fitting `golden_*.rs` suite (modify — keep stale-prerequisite recovery coverage aligned with the new canonical path)

## Out of Scope

- adding the new deliberate epistemic E2E goldens tracked in [S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md)
- changing epistemic action handler semantics unless the barrier migration exposes a real production bug there
- adding a backwards-compatible compatibility layer that supports both standalone verification goals and originating-goal barriers
- unrelated passive-discovery, tell, or violation-response refactors

## Acceptance Criteria

### Tests That Must Pass

1. Focused AI coverage proves stale evidence is handled as an originating-goal epistemic barrier rather than a standalone top-level verification goal
2. Focused AI coverage proves `AskWitness` and `Travel -> VerifyBelief` can be selected as explicit progress barriers under originating goals
3. Focused AI coverage proves the standalone `VerifyBelief` goal path no longer exists
4. Existing stale-prerequisite recovery coverage still passes under the new canonical contract
5. Existing typed epistemic action-trace coverage still passes
6. `cargo test -p worldwake-ai`
7. `cargo test -p worldwake-sim action_trace`
8. `cargo test -p worldwake-systems epistemic_actions`
9. `cargo clippy -p worldwake-ai -p worldwake-sim -p worldwake-systems --all-targets -- -D warnings`

### Invariants

1. The canonical top-level intention remains the originating world-condition goal, not a rival verification-only goal
2. `verify_belief` and `ask_witness` remain explicit world actions with duration, cost, and aftermath
3. No dual-path coexistence remains for the same deliberate-verification contract
4. The canonical information path for deliberate verification is explainable through focused planner/trace surfaces without falling back to indirect downstream inference

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — replace standalone verification-emission assertions with originating-goal stale-evidence barrier assertions
2. `crates/worldwake-ai/src/goal_model.rs` and `crates/worldwake-ai/src/search/` — add focused planner/search tests proving originating-goal `AskWitness` and `Travel -> VerifyBelief` barrier insertion
3. `crates/worldwake-ai/tests/golden_supply_chain.rs` or another fitting `golden_*.rs` suite — update stale-prerequisite recovery coverage so it still proves the live `RestockCommodity` contract under the new architecture
4. `specs/S34-general-epistemic-actions.md` — correct the S34 architectural narrative and proof-surface expectations

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::verify_belief`
2. `cargo test -p worldwake-ai goal_model::tests::search_verify_belief`
3. `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo test -p worldwake-sim action_trace`
6. `cargo test -p worldwake-systems epistemic_actions`
7. `cargo clippy -p worldwake-ai -p worldwake-sim -p worldwake-systems --all-targets -- -D warnings`
