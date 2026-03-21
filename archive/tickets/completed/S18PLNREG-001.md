# S18PLNREG-001: Reassess planner regression coverage for stale-belief branch replacement

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: No additional production or test changes required
**Deps**: `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, `tickets/README.md`, `specs/S18-prerequisite-aware-emergent-chain-goldens.md`, `archive/tickets/completed/S18PREAWAEME-003.md`

## Problem

This ticket was opened to add a concentrated planner/runtime regression cluster for stale-belief branch replacement after `S18PREAWAEME-003`. Reassessment against the live code and test surface showed that the intended production fixes and the key focused and golden regressions are already present. The active ticket was stale and was incorrectly proposing duplicate implementation work.

## Assumption Reassessment (2026-03-21)

1. The production architecture this ticket was meant to protect is already live:
   - depleted prerequisite-source filtering in [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
   - depleted acquisition-evidence filtering in [`candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
   - non-suppressing `SourceDepleted` blocker semantics in [`blocked_intent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs)
   - stale-plan rejection in [`plan_selection.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs)
2. The focused tests this ticket described as missing already exist:
   - `goal_model::tests::prerequisite_places_produce_commodity_exclude_depleted_resource_sources`
   - `candidate_generation::tests::depleted_resource_sources_are_excluded_from_produce_goal_evidence`
   - `blocked_intent::tests::source_depleted_does_not_block_goal_generation`
   - `plan_selection::tests::stale_current_plan_is_not_retained_when_current_goal_has_no_plan`
3. The downstream goldens this ticket described as future work already exist and pass in [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs):
   - `golden_merchant_restocks_via_prerequisite_aware_craft`
   - `golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically`
   - `golden_stale_prerequisite_belief_discovery_replan`
   - `golden_stale_prerequisite_belief_discovery_replan_replays_deterministically`
4. The stale-belief golden is already asserting the earlier planner boundary this ticket wanted. It checks:
   - initial stale Orchard-branch selection from seeded belief
   - fresh-search fallback selection to Bandit Camp after local perception
   - same-goal branch replacement provenance
   - candidate evidence inclusion/exclusion and root prerequisite guidance
5. The active ticket’s proposed file-touch list was stale. Adding new coverage in [`agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs) is not currently required to prove the invariant because the live proof surface is split cleanly between focused symbol tests and the supply-chain golden.
6. No current failing behavior or uncovered invariant was found during reassessment. This is a ticket-correction and archival task, not an engine-change task.

## Architecture Check

1. The current architecture is cleaner than the ticket’s proposed expansion. The focused unit tests lock each causal rule at the owning symbol, while the stale-belief golden proves the cross-layer recovery chain with decision traces. That separation is robust and easier to maintain than adding another near-duplicate planner-runtime cluster.
2. Adding new `agent_tick` regression tests right now would mostly duplicate assertions already covered more directly in `plan_selection` and more realistically in `golden_supply_chain`. That would increase maintenance cost without materially improving architectural confidence.
3. The only architectural refinement worth noting is conditional, not immediate: if a second stale-belief scenario appears that needs the same trace assertions, factor shared semantic helpers into the golden harness then. Doing it now would be premature abstraction.

## Verification Layers

1. depleted prerequisite-source places are excluded from goal guidance -> focused unit coverage in [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
2. depleted acquisition evidence is excluded from candidate generation -> focused unit coverage in [`candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
3. `SourceDepleted` does not suppress same-goal regeneration -> focused unit coverage in [`blocked_intent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs)
4. stale current plans are not retained when the current goal has no viable fresh plan -> focused unit coverage in [`plan_selection.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs)
5. stale local belief is corrected and replaced by a fresh fallback branch -> decision trace assertions in [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)
6. downstream craft-restock recovery and deterministic replay remain intact -> authoritative world-state, action-trace, and replay assertions in [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)

## Scope Correction

No additional production changes are justified.

No additional test additions are justified at this time.

This ticket is complete once its stale assumptions are corrected, verification is rerun, and the ticket is archived.

## Tests

### New/Modified Tests

1. None.
Rationale: the reassessment found that the intended focused and golden regressions already existed in the live tree and already cover the invariant this ticket was opened to add.

### Verification Commands

1. `cargo test -p worldwake-core source_depleted_does_not_block_goal_generation`
2. `cargo test -p worldwake-ai prerequisite_places_produce_commodity_exclude_depleted_resource_sources`
3. `cargo test -p worldwake-ai depleted_resource_sources_are_excluded_from_produce_goal_evidence`
4. `cargo test -p worldwake-ai stale_current_plan_is_not_retained_when_current_goal_has_no_plan`
5. `cargo test -p worldwake-ai --test golden_supply_chain golden_stale_prerequisite_belief_discovery_replan`
6. `cargo test -p worldwake-ai --test golden_supply_chain golden_stale_prerequisite_belief_discovery_replan_replays_deterministically`
7. `cargo test -p worldwake-ai`
8. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - corrected the ticket to match the live architecture and test surface
  - verified the existing focused regressions, stale-belief goldens, `worldwake-ai` package tests, and `worldwake-ai` clippy target all pass
- Deviations from original plan:
  - no new production code was required
  - no new tests were added because the intended regressions were already present
  - the ticket was closed as stale-planning cleanup rather than as a new implementation ticket
- Verification results:
  - `cargo test -p worldwake-core source_depleted_does_not_block_goal_generation` passed
  - `cargo test -p worldwake-ai prerequisite_places_produce_commodity_exclude_depleted_resource_sources` passed
  - `cargo test -p worldwake-ai depleted_resource_sources_are_excluded_from_produce_goal_evidence` passed
  - `cargo test -p worldwake-ai stale_current_plan_is_not_retained_when_current_goal_has_no_plan` passed
  - `cargo test -p worldwake-ai --test golden_supply_chain golden_stale_prerequisite_belief_discovery_replan` passed
  - `cargo test -p worldwake-ai --test golden_supply_chain golden_stale_prerequisite_belief_discovery_replan_replays_deterministically` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings` passed
