# S18PREAWAEME-003: Golden test — stale prerequisite belief discovery and replan

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner prerequisite-source filtering, candidate-evidence filtering, blocked-intent generation semantics, and stale-plan retention
**Deps**: `archive/specs/S18-prerequisite-aware-emergent-chain-goldens.md`

## Problem

No golden had been proving the stale prerequisite chain end-to-end for the live craft-restock architecture:

1. the agent plans from a stale nearer prerequisite-source belief
2. local perception corrects that belief on arrival
3. the planner drops the stale branch and selects a lawful fallback source
4. the same restock goal still succeeds

Reassessment showed this was not a tests-only gap. Current code still treated zero-quantity `ResourceSource` beliefs as actionable in prerequisite discovery and acquisition evidence, and `BlockedIntent::SourceDepleted` still suppressed whole-goal generation even when an alternative lawful source existed.

## Assumption Reassessment (2026-03-21)

1. `GoalKind::RestockCommodity` already delegates to recipe-input prerequisite discovery in [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs). The parent S18 spec text that described this as missing was stale.
2. The already-delivered craft-restock golden is `golden_merchant_restocks_via_prerequisite_aware_craft` in [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs).
3. The original ticket story around `ProduceCommodity { Bake Bread }` plus harvest-based fallback was wrong for the live planner. `PRODUCE_OPS` does not include `Harvest`, so a harvest-backed stale-source chain belongs under `RestockCommodity`, not `ProduceCommodity`, in [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs).
4. The cleaner golden is single-agent, not a two-agent depletion drama. We can seed a stale Orchard belief directly, start Orchard already empty, keep Bandit Camp valid, and prove the same causal invariant without adding Alice as unrelated noise.
5. The live recovery boundary in the golden is earlier than the old ticket claimed. After arrival and local perception, the agent fresh-searches and selects the Bandit Camp fallback before any stale harvest step is enqueued. The golden should assert that earlier search-selection boundary, not force a `StartFailed` or revalidation failure that the engine does not naturally produce in this scenario.
6. `BlockedIntentMemory` still matters architecturally, but it is better locked by focused unit coverage here than by forcing the golden to hinge on a later failure boundary that the live runtime no longer reaches in this setup.

## Architecture Check

1. Filtering depleted sources out of both `prerequisite_places()` and acquisition evidence is cleaner than leaving empty shells in planning surfaces and teaching tests to tolerate stuck or misleading plans.
2. Keeping `SourceDepleted` recordable but non-suppressing is cleaner than goal-level vetoes. It preserves traceability without blocking lawful same-goal regeneration.
3. The plan-selection fix is architecturally correct because a fresh search result must outrank a stale retained current plan when the current goal no longer has any viable plan.
4. The final golden uses the earliest causal boundary the live engine actually guarantees: stale route selected first, local belief corrected on arrival, then a fresh fallback search selection. That is stronger and less brittle than asserting a later artificial failure surface.

## Verification Layers

1. Depleted prerequisite-source places are excluded from goal guidance -> focused unit coverage in [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
2. Depleted resource-source places and entities are excluded from actionable acquisition evidence -> focused unit coverage in [`candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
3. `SourceDepleted` no longer suppresses whole-goal generation -> focused unit coverage in [`blocked_intent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs)
4. A stale current plan is not retained when the current goal has no fresh viable plan -> focused unit coverage in [`plan_selection.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs)
5. Initial stale branch selects Orchard Farm from seeded beliefs -> decision trace in [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)
6. Local perception corrects the Orchard source belief and triggers a fresh Bandit Camp fallback plan -> decision trace plus belief-state assertion in [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)
7. Orchard and Bandit Camp are both visited lawfully, Bandit Camp is consumed, and bread is restocked at the home market -> authoritative world state plus action trace in [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)
8. Determinism and conservation remain intact -> replay companion plus per-tick conservation assertions in [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)

## Files Touched

- [`blocked_intent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs)
- [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
- [`candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
- [`plan_selection.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs)
- [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)

## Tests

### New/Modified Tests

1. [`blocked_intent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs): `source_depleted_does_not_block_goal_generation`
Rationale: locks the corrected rule that `SourceDepleted` remains recordable without vetoing same-goal regeneration.

2. [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs): `prerequisite_places_produce_commodity_exclude_depleted_resource_sources`
Rationale: proves zero-quantity resource sources no longer survive prerequisite-place discovery.

3. [`candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs): `depleted_resource_sources_are_excluded_from_produce_goal_evidence`
Rationale: proves actionable acquisition evidence no longer includes empty source places or entities.

4. [`plan_selection.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs): `stale_current_plan_is_not_retained_when_current_goal_has_no_plan`
Rationale: locks the fresh-search-over-stale-plan rule needed for robust recovery after stale-branch invalidation.

5. [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs): `golden_stale_prerequisite_belief_discovery_replan`
Rationale: proves the full stale-belief restock chain under the live architecture: stale Orchard selection, local belief correction, fresh Bandit Camp replan, and successful bread restock.

6. [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs): `golden_stale_prerequisite_belief_discovery_replan_replays_deterministically`
Rationale: locks deterministic replay for the new stale-belief supply-chain scenario.

### Verification Commands

1. `cargo test -p worldwake-core source_depleted_does_not_block_goal_generation`
2. `cargo test -p worldwake-ai prerequisite_places_produce_commodity_exclude_depleted_resource_sources`
3. `cargo test -p worldwake-ai depleted_resource_sources_are_excluded_from_produce_goal_evidence`
4. `cargo test -p worldwake-ai stale_current_plan_is_not_retained_when_current_goal_has_no_plan`
5. `cargo test -p worldwake-ai --test golden_supply_chain golden_merchant_restocks_via_prerequisite_aware_craft`
6. `cargo test -p worldwake-ai --test golden_supply_chain golden_stale_prerequisite_belief_discovery_replan`
7. `cargo test -p worldwake-ai --test golden_supply_chain golden_stale_prerequisite_belief_discovery_replan_replays_deterministically`
8. `cargo test -p worldwake-ai --test golden_supply_chain`
9. `cargo test -p worldwake-ai`
10. `cargo test -p worldwake-core`
11. `cargo test --workspace`
12. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - `places_with_resource_source()` now excludes depleted resource sources from prerequisite-place discovery.
  - acquisition-path evidence now excludes depleted resource-source places and entities instead of leaking empty places through `Evidence::with_place`.
  - `BlockedIntent::SourceDepleted` no longer suppresses whole-goal generation.
  - `select_best_plan()` no longer retains a stale current plan when the current goal has no fresh viable plan.
  - added a new stale-belief craft-restock golden plus deterministic replay companion.
- Deviations from original plan:
  - the ticket did not finish as a `ProduceCommodity` plus two-agent depletion scenario; that assumption was wrong for the live planner because harvest-backed recovery belongs under `RestockCommodity`.
  - the final golden uses a single merchant with a seeded stale Orchard belief and a lawful Bandit Camp fallback.
  - the golden proves the earlier fresh-search fallback boundary after local perception, not a forced `StartFailed`/revalidation failure path.
  - `SourceDepleted` memory semantics are covered by focused unit tests instead of being forced into the golden contract.
- Verification results:
  - all focused tests listed above passed
  - `cargo test -p worldwake-ai --test golden_supply_chain` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test -p worldwake-core` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
