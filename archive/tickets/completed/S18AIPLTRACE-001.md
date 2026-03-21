# S18AIPLTRACE-001: Add planner trace provenance for stale-belief invalidation

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision-trace data model plus planner-owned diagnostic collection in candidate generation / search / selection
**Deps**: `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, `tickets/README.md`, `archive/tickets/completed/S18PREAWAEME-003.md`

## Problem

The stale-belief recovery behavior fixed in `S18PREAWAEME-003` is now correct, but the planner traces still do not explain the causal reason a previously viable branch disappeared after local perception. Today the trace exposes selected plans, search-expansion counts, and high-level candidate/ranking surfaces, but not the concrete prerequisite-place or candidate-evidence provenance that caused the branch change.

That is an architectural observability gap, not just a debugging inconvenience. Worldwake is optimizing for explainable emergence; if a belief-driven branch disappears, the trace should show which local evidence invalidated it and which remaining local options stayed viable.

## Assumption Reassessment (2026-03-21)

1. Current decision traces record high-level planning surfaces through `worldwake_ai::decision_trace::{CandidateTrace, SearchExpansionSummary, SelectionTrace, SelectedPlanTrace, SelectedPlanSearchProvenance}` and are populated from [`agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs). Those surfaces expose generated/ranked goals, search expansion counts, selected plan shape, ranked-goal provenance for danger/drive, travel-pruning summaries, and selected-plan provenance, but not prerequisite-place inclusion/exclusion reasons or typed candidate-evidence provenance.
2. The stale-source pruning decisions that mattered in `S18PREAWAEME-003` currently happen in production code, not only in tests: `crates/worldwake-ai/src/goal_model.rs::places_with_resource_source()` now filters depleted `ResourceSource`s by `available_quantity`, and `crates/worldwake-ai/src/candidate_generation.rs::acquisition_path_evidence_inner()` now excludes depleted source places/entities from actionable evidence.
3. Existing coverage is split across focused tests and one golden:
   - focused/unit: `blocked_intent::tests::source_depleted_does_not_block_goal_generation` in [blocked_intent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/blocked_intent.rs)
   - focused/unit: `goal_model::tests::prerequisite_places_produce_commodity_exclude_depleted_resource_sources` in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs)
   - focused/unit: `candidate_generation::tests::depleted_resource_sources_are_excluded_from_produce_goal_evidence` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
   - focused/unit: `plan_selection::tests::stale_current_plan_is_not_retained_when_current_goal_has_no_plan` in [plan_selection.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs)
   - golden/E2E: `golden_stale_prerequisite_belief_discovery_replan` and replay companion in [golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)
4. The target layer for this ticket is AI / belief-view / planning traceability, not authoritative start failure or event-log ordering. The core contract is planning reasoning visibility, so decision-trace assertions are the primary verification surface; the golden remains a downstream confirmation that the new trace data matches the live stale-belief fallback.
5. No heuristic removal is proposed. The production filtering added in `S18PREAWAEME-003` stays intact. This ticket only makes those existing planner decisions causally legible in traces.
6. This is not a stale-request or start-failure ticket. The first relevant divergence in the motivating scenario happens earlier, at planner search/selection after local perception updates the belief state.
7. The motivating golden isolates one intended branch: stale Orchard belief invalidated locally, then lawful Bandit Camp fallback survives. Competing lawful branches were intentionally removed there, and this ticket should preserve that isolation rather than broadening planner behavior.
8. Mismatch + correction: the missing piece is not more planner behavior changes. The missing piece is structured provenance that explains why a branch disappeared and which replacement branch remained viable.
9. Mismatch + correction: candidate generation does not currently retain typed source-kind provenance internally. `GroundedGoal` stores only aggregated `evidence_entities` / `evidence_places` in [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and `acquisition_path_evidence_inner()` builds an untyped local `Evidence` set in [`candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs). If trace provenance is added, it must be collected explicitly as derived diagnostics at generation time rather than claimed as already available planner data.
10. Mismatch + correction: search currently records only prerequisite-place counts, not the concrete prerequisite-place members. `combined_relevant_places()` in [`search.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs) merges goal-relevant and prerequisite places and preserves only `prerequisite_places_count`, so any trace of inclusion/exclusion reasons requires a new planner-owned summary at that boundary.
11. Authoritative arithmetic is not the contract here. The ticket concerns provenance over belief-local planning inputs that are already computed during candidate generation and search, but some of that provenance must be newly summarized because the live planner currently discards it.

## Architecture Check

1. The clean architecture is to expose planner-owned provenance as append-only derived diagnostics, not to infer it indirectly from action traces or broad golden outcomes. That keeps source-of-truth state in existing world/belief/planning data while making causal reasoning inspectable.
2. The trace additions must remain derived summaries, never truth. They must not change planner selection semantics or become new decision inputs.
3. The cleanest seam is to capture provenance at the moment candidate evidence and prerequisite-place guidance are assembled, then publish that through the canonical decision trace. Recomputing reasons later from selected plans or golden outcomes would duplicate planner logic and risk drift.
4. Do not widen `GroundedGoal` into a diagnostic bag just to serve tracing. Goal grounding should keep its minimal authoritative planning payload; trace-only structure belongs in decision-trace diagnostics.
5. No backwards-compatibility aliasing or parallel trace path should be introduced. Extend the canonical decision-trace model in `worldwake-ai` directly.

## Verification Layers

1. candidate evidence provenance records actionable seller/source/source-place/corpse/loose-lot inputs and excludes depleted resource sources -> focused candidate-generation/runtime trace coverage
2. prerequisite-place guidance records which places were added as prerequisites and which candidate source places were excluded as depleted -> focused search/runtime trace coverage
3. first post-perception branch replacement is explained by structured trace data rather than inferred from missing later actions -> golden decision-trace assertions in `golden_supply_chain`
4. authoritative world behavior remains unchanged while traceability improves -> existing focused planner tests plus existing golden continue to pass

## What to Change

### 1. Extend decision-trace data structures with compact provenance details

Add compact planner-owned structs under `crates/worldwake-ai/src/decision_trace.rs` for:

- candidate-evidence provenance by source kind and entity/place
- prerequisite-place guidance provenance at the search-expansion boundary, including explicit excluded-depleted-source cases for the live stale-belief scenario
- selected-plan replacement explanation when a previously active/current branch disappears after local perception and a fresh fallback search survives

Keep the payload compact and deterministic. It should describe concrete entities, places, and reason enums, not narrative strings.

### 2. Collect provenance at the existing planner boundaries that already make these decisions

Collect and thread the new summaries through the existing planner pipeline without consulting authoritative world state outside the agent belief/planning view:

- candidate-evidence provenance should be emitted where `acquisition_path_evidence_inner()` and related helpers already inspect sellers, loose lots, resource sources, corpses, and recipe paths
- prerequisite-place provenance should be emitted where search combines `goal_relevant_places()` with `prerequisite_places()` and where depleted-source filtering would otherwise silently remove a stale branch
- selected-branch replacement summaries should be derived at selection/revalidation time from the already-available previous-goal/current-plan context plus the fresh traced planning result

The implementation must preserve locality: traces may only summarize the agent-visible belief/planning inputs already in play. Avoid a second "trace recomputation" pass that reconstructs reasons after the fact.

### 3. Add focused and golden trace assertions

Add focused tests that prove the new provenance surfaces are populated for depleted-source exclusion and stale-branch replacement, then strengthen `golden_stale_prerequisite_belief_discovery_replan` to assert on the new trace payload instead of reconstructing causality indirectly.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify if needed to expose prerequisite-source diagnostics without duplicating planner logic)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify)

## Out of Scope

- new planner heuristics or behavior changes beyond trace population
- UI formatting or human-readable reporter overhauls beyond what tests need
- authoritative action-trace or event-log schema changes

## Acceptance Criteria

### Tests That Must Pass

1. traced stale-belief fallback surfaces which stale prerequisite branch was invalidated after local perception and which fallback prerequisite/source branch survived
2. traced candidate evidence surfaces actionable source provenance and omits depleted resource sources
3. search-expansion trace exposes concrete prerequisite guidance members rather than only counts for the exercised stale-belief scenario
4. existing stale-belief recovery golden still passes with stronger decision-trace assertions
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. decision traces remain derived diagnostics and do not alter planning behavior
2. provenance records only concrete, local planner inputs already available to the agent belief/planning pipeline

## Test Plan

### New/Modified Tests

1. `candidate_generation::tests::candidate_evidence_trace_records_resource_source_contributors_and_exclusions` in `crates/worldwake-ai/src/candidate_generation.rs`
Rationale: proves typed candidate-evidence provenance records live resource-source contributors, recipe workstations, and depleted-source exclusions instead of only aggregated evidence sets.

2. `search::tests::search_expansion_summary_counts_prerequisite_places_for_remote_treat_wounds` in `crates/worldwake-ai/src/search.rs`
Rationale: proves traced search expansions now carry concrete prerequisite guidance members, not just counts, while preserving the existing remote-care planning contract.

3. `agent_tick::tests::summarize_plan_replacement_records_same_goal_branch_replan` in `crates/worldwake-ai/src/agent_tick.rs`
Rationale: locks the selection-layer summary that distinguishes a same-goal fresh branch replacement from simple plan continuation or goal switching.

4. `golden_stale_prerequisite_belief_discovery_replan` and replay companion in `crates/worldwake-ai/tests/golden_supply_chain.rs`
Rationale: proves the live stale-belief scenario now exposes typed candidate evidence, prerequisite guidance, and same-goal branch replacement at the exact replan boundary.

### Commands

1. `cargo test -p worldwake-ai --lib`
2. `cargo test -p worldwake-ai --test golden_supply_chain golden_stale_prerequisite_belief_discovery_replan`
3. `cargo test -p worldwake-ai --test golden_supply_chain golden_stale_prerequisite_belief_discovery_replan_replays_deterministically`
4. `cargo test -p worldwake-ai --test golden_supply_chain`
5. `cargo test -p worldwake-ai`
6. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-21
- What actually changed:
  - extended the canonical decision-trace model with typed candidate-evidence provenance, prerequisite-guidance provenance, and selected-plan replacement summaries
  - captured candidate-evidence diagnostics at the existing acquisition/recipe generation seams without widening `GroundedGoal`
  - captured prerequisite guidance at the existing search boundary via a goal-model helper instead of recomputing it from golden outcomes
  - strengthened the stale-belief golden and added focused tests for each new trace surface
- Deviations from original plan:
  - the implementation did not add a generic narrative-style invalidation layer; it added compact enum-backed diagnostics at the exact planner seams that already make the underlying decisions
  - `goal_model.rs` was used only to expose derived prerequisite-guidance diagnostics and keep search from duplicating recipe-input/source-filter logic
  - one additional focused test was added in `agent_tick.rs` to lock the same-goal branch-replacement summary explicitly
- Verification results:
  - `cargo test -p worldwake-ai candidate_evidence_trace_records_resource_source_contributors_and_exclusions` passed
  - `cargo test -p worldwake-ai search_expansion_summary_counts_prerequisite_places_for_remote_treat_wounds` passed
  - `cargo test -p worldwake-ai summarize_plan_replacement_records_same_goal_branch_replan` passed
  - `cargo test -p worldwake-ai --test golden_supply_chain golden_stale_prerequisite_belief_discovery_replan` passed
  - `cargo test -p worldwake-ai --test golden_supply_chain` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings` passed
