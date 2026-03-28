# S34GENEPIACT-010: Golden E2E coverage for deliberate epistemic prerequisite chains

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Tests only expected
**Deps**: [archive/tickets/completed/S34GENEPIACT-012.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-012.md), [archive/tickets/completed/S34GENEPIACT-011.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-011.md), [archive/tickets/completed/S34GENEPIACT-009.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-009.md), [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md)

## Problem

The live S34 architecture now has a fact-sensitive epistemic barrier contract:

- arrival-observable stale facts refresh through travel plus ordinary lawful local perception
- social information transfer uses explicit `ask_witness`

Focused planner/runtime coverage exists for both paths, and existing goldens already cover the arrival-observable branch. The missing end-to-end proof is the explicit social epistemic branch: a grounded goal selecting `AskWitness`, committing the action, updating belief provenance via report transfer, and then continuing from the new knowledge.

## Assumption Reassessment (2026-03-28)

1. The ticket's original framing is stale. [archive/tickets/completed/S34GENEPIACT-012.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-012.md) has already landed, so this ticket must target the post-012 architecture rather than wait for it.
2. The exact mixed-layer boundary under audit here is:
   - decision-trace proof that an originating grounded goal selected the canonical epistemic barrier path
   - action-trace proof of committed `ask_witness`
   - authoritative belief-store proof that the transferred fact arrived as `PerceptionSource::Report { from: witness, chain_len: 1 }`
   - later decision/action proof that the originating branch continues from the new belief
3. Existing golden coverage already proves the arrival-observable branch this ticket originally listed as missing:
   - [golden_stale_prerequisite_belief_discovery_replan in golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) covers stale supply-source travel, lawful arrival refresh, contradiction recording, and replan under live `GoalKind::RestockCommodity`
   - [golden_stale_belief_travel_reobserve_replan in golden_social.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs) covers stale entity/resource belief travel and direct re-observation
   - [golden_rumor_leads_to_wasted_trip_then_discovery in golden_social.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs) covers a reported/rumored stale branch that ends in lawful arrival correction
4. Existing focused coverage already proves the planner surfaces this ticket depends on:
   - `goal_model::tests::search_restock_goal_returns_travel_barrier_for_remote_stale_source`
   - `goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload`
5. No current golden uses `ask_witness` as the end-to-end canonical barrier path. Repository search across `crates/worldwake-ai/tests` shows only one negative assertion in [golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) that `ask_witness` does **not** commit on the arrival-observable stale-source branch.
6. Corrected scope: this ticket should add the missing `ask_witness` golden plus its deterministic replay companion, and should not duplicate the already-covered arrival-observable stale-belief or stale-supply replan scenarios.
7. Scenario isolation for the new golden must be explicit:
   - intended branch: originating goal -> `AskWitness` progress barrier -> report-sourced belief update -> continuation from the new knowledge
   - competing lawful branches to control: direct observation of the target fact, unrelated tell traffic, unrelated need satisfaction, and alternative non-epistemic satisfiers
   - excluded branches are test setup choices only; the production architecture remains fact-sensitive and unchanged
8. Adjacent contradiction classification:
   - required consequence of this ticket: fill the missing end-to-end proof surface for the explicit social epistemic path
   - separate already-resolved architecture work: fact-sensitive barrier selection and removal of duplicate arrival-observable `VerifyBelief` paths belong to S34GENEPIACT-012, not here

## Architecture Check

1. Adding one focused `AskWitness` golden is cleaner than adding more arrival-refresh goldens, because the arrival-refresh branch is already represented at both focused and golden layers.
2. This is cleaner than inventing a dormant `VerifyBelief`-style golden branch, because the current architecture intentionally made `AskWitness` the only live explicit epistemic action in the canonical path for these fact classes.
3. The durable design remains sound:
   - arrival-observable facts use travel plus passive perception
   - social knowledge transfer uses explicit `ask_witness`
   - no duplicate canonical path exists for the same fact class
4. If later work introduces a true inspection-only fact class, it should arrive as a new ticket/spec with its own authoritative contract rather than by widening this ticket's scope.

## Verification Layers

1. originating goal selects the `AskWitness` prerequisite barrier under the live grounded-goal contract -> decision trace
2. `ask_witness` starts and commits with the expected payload identity -> action trace
3. actor belief updates to a witness-reported fact with `PerceptionSource::Report { from: witness, chain_len: 1 }` -> authoritative belief state
4. post-query continuation uses the new knowledge rather than stalling on the stale branch -> later decision trace and/or action trace, depending on the cleanest contract boundary in the final scenario
5. deterministic replay of the whole chain -> replay companion golden

## What to Change

Add one new golden scenario, likely in [crates/worldwake-ai/tests/golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) or [crates/worldwake-ai/tests/golden_social.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs), that proves:

1. an originating grounded goal depends on stale fact knowledge
2. a co-located witness lawfully knows the relevant subject
3. decision traces show the originating goal selects `AskWitness` as the canonical progress barrier
4. action traces show committed `ask_witness`
5. the actor's belief updates with `PerceptionSource::Report { from: witness, chain_len: 1 }`
6. the actor then continues from the new knowledge toward the originating goal's downstream branch

Add a deterministic replay companion for the same scenario.

## Files to Touch

- [crates/worldwake-ai/tests/golden_social.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs) or another fitting `golden_*.rs` suite if reassessment during implementation shows a better semantic home

## Out of Scope

- changing planner, candidate-generation, or action-handler production behavior
- duplicating existing arrival-refresh or stale-supply replan goldens
- reviving generic `VerifyBelief` golden expectations for arrival-observable facts
- unrelated social/tell/investigation refactors

## Acceptance Criteria

### Tests That Must Pass

1. new golden proving the `AskWitness` prerequisite chain
2. replay companion for that golden
3. `cargo test -p worldwake-ai`

### Invariants

1. The golden asserts the live canonical explicit epistemic path: `AskWitness`, not an obsolete generic `VerifyBelief` path.
2. The scenario proves the earliest strong causal surfaces for this branch: decision trace for barrier choice, action trace for action identity, authoritative belief state for report transfer.
3. The originating goal continues from the new knowledge rather than forking into a standalone verification goal.

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` or `crates/worldwake-ai/tests/golden_supply_chain.rs` — `AskWitness` prerequisite-chain golden
   Rationale: prove the only missing end-to-end canonical explicit epistemic branch after S34GENEPIACT-012.
2. same suite — deterministic replay companion
   Rationale: preserve the golden determinism contract for the new epistemic chain.

### Commands

1. `cargo test -p worldwake-ai goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload -- --exact`
2. `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan -- --exact`
3. `cargo test -p worldwake-ai golden_stale_belief_travel_reobserve_replan -- --exact`
4. `cargo test -p worldwake-ai golden_rumor_leads_to_wasted_trip_then_discovery -- --exact`
5. `cargo test -p worldwake-ai`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - corrected the ticket scope to the live post-S34GENEPIACT-012 architecture instead of duplicating already-covered arrival-observable goldens
  - added `golden_stale_prerequisite_ask_witness_chain` and `golden_stale_prerequisite_ask_witness_chain_replays_deterministically` in [crates/worldwake-ai/tests/golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)
  - proved the end-to-end `AskWitness` path with decision-trace barrier selection, action-trace commit identity, report-sourced belief transfer, and post-query continuation toward the originating `RestockCommodity` branch
- Deviations from original plan:
  - the original ticket proposed three new golden families, but reassessment showed that the stale arrival-refresh and stale supply-depletion replan branches were already covered by existing goldens and focused tests
  - the new golden uses an apples-restock scenario with the scheduler advanced to `Tick(50)` so the seeded belief is genuinely stale under the live confidence contract; no production code change was required
- Verification results:
  - `cargo test -p worldwake-ai goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload -- --exact` passed
  - `cargo test -p worldwake-ai golden_stale_prerequisite_ask_witness_chain -- --exact` passed
  - `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan -- --exact` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
