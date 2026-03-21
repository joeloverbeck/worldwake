# S18AIPLTRACE-001: Add planner trace provenance for stale-belief invalidation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision-trace data model, trace population in candidate generation / search / selection
**Deps**: `docs/FOUNDATIONS.md`, `docs/golden-e2e-testing.md`, `tickets/README.md`, `archive/tickets/completed/S18PREAWAEME-003.md`

## Problem

The stale-belief recovery behavior fixed in `S18PREAWAEME-003` is now correct, but the planner traces still do not explain the causal reason a previously viable branch disappeared after local perception. Today the trace exposes selected plans, search-expansion counts, and high-level candidate/ranking surfaces, but not the concrete prerequisite-place or candidate-evidence provenance that caused the branch change.

That is an architectural observability gap, not just a debugging inconvenience. Worldwake is optimizing for explainable emergence; if a belief-driven branch disappears, the trace should show which local evidence invalidated it and which remaining local options stayed viable.

## Assumption Reassessment (2026-03-21)

1. Current decision traces record high-level planning surfaces through `worldwake_ai::decision_trace::{CandidateTrace, SearchExpansionSummary, SelectionTrace, SelectedPlanTrace, SelectedPlanSearchProvenance}` and are populated from `crates/worldwake-ai/src/agent_tick.rs`. Those surfaces expose generated/ranked goals, search expansion counts, selected plan shape, and selected-plan provenance, but not prerequisite-place inclusion/exclusion reasons or candidate-evidence provenance.
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
9. Authoritative arithmetic is not the contract here. The ticket concerns provenance over belief-local planning inputs that are already computed during candidate generation and search.

## Architecture Check

1. The clean architecture is to expose planner-owned provenance as append-only derived diagnostics, not to infer it indirectly from action traces or broad golden outcomes. That keeps source-of-truth state in existing world/belief/planning data while making causal reasoning inspectable.
2. The trace additions must remain derived summaries, never truth. They must not change planner selection semantics or become new decision inputs.
3. No backwards-compatibility aliasing or parallel trace path should be introduced. Extend the canonical decision-trace model in `worldwake-ai` directly.

## Verification Layers

1. prerequisite-place inclusion/exclusion reasons are recorded for traced search expansions -> focused decision-trace unit/runtime coverage
2. candidate evidence provenance records actionable seller/source/corpse inputs and excludes depleted resource sources -> focused candidate-generation/runtime trace coverage
3. first post-perception branch replacement is explained by structured trace data rather than inferred from missing later actions -> golden decision-trace assertions in `golden_supply_chain`
4. authoritative world behavior remains unchanged while traceability improves -> existing focused planner tests plus existing golden continue to pass

## What to Change

### 1. Extend decision-trace data structures with provenance details

Add compact planner-owned structs under `crates/worldwake-ai/src/decision_trace.rs` for:

- prerequisite-place inclusion and exclusion reasons at the search-expansion boundary
- candidate-evidence provenance by source kind and entity/place
- selected-plan-change explanation for stale-belief invalidation when a previously active/current branch disappears after local perception

Keep the payload compact and deterministic. It should describe concrete entities, places, and reason enums, not narrative strings.

### 2. Populate provenance from existing planner-owned computations

Thread the new summaries through the existing planner pipeline without recomputing from authoritative world state:

- candidate-evidence provenance should come from `acquisition_path_evidence_inner()` and related candidate-generation surfaces
- prerequisite-place provenance should come from the same planning/belief inputs already used to build search guidance
- selected-branch invalidation summaries should be derived at selection/revalidation time from already-available previous-goal/current-plan context plus the fresh traced planning result

The implementation must preserve locality: traces may only summarize the agent-visible belief/planning inputs already in play.

### 3. Add focused and golden trace assertions

Add focused tests that prove the new provenance surfaces are populated for depleted-source exclusion and stale-branch replacement, then strengthen `golden_stale_prerequisite_belief_discovery_replan` to assert on the new trace payload instead of reconstructing causality indirectly.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify)

## Out of Scope

- new planner heuristics or behavior changes beyond trace population
- UI formatting or human-readable reporter overhauls beyond what tests need
- authoritative action-trace or event-log schema changes

## Acceptance Criteria

### Tests That Must Pass

1. traced stale-belief fallback surfaces which prerequisite place was excluded and why after local perception
2. traced candidate evidence surfaces actionable source provenance and omits depleted resource sources
3. existing stale-belief recovery golden still passes with stronger decision-trace assertions
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. decision traces remain derived diagnostics and do not alter planning behavior
2. provenance records only concrete, local planner inputs already available to the agent belief/planning pipeline

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — add focused trace-model/runtime tests for prerequisite-place and candidate-evidence provenance payloads
2. `crates/worldwake-ai/src/agent_tick.rs` or `crates/worldwake-ai/src/search.rs` — add focused planner-trace tests proving stale current-branch invalidation is surfaced as structured provenance
3. `crates/worldwake-ai/tests/golden_supply_chain.rs` — strengthen the stale-belief golden to assert the new causal trace data directly

### Commands

1. `cargo test -p worldwake-ai --lib`
2. `cargo test -p worldwake-ai --test golden_supply_chain`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
