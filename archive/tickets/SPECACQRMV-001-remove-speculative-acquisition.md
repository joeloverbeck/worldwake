# SPECACQRMV-001: Remove speculative_acquisition From Architecture

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — CognitiveProfile, candidate generation, scenario profiles
**Deps**: None

## Problem

`speculative_acquisition` is a boolean on `CognitiveProfile` (introduced in S83) that, when `true`, causes the planner to generate `AcquireCommodity` candidates at every known place regardless of whether the agent has any belief about the target commodity there. This was intended to support "optimistic" agents who try places they've merely heard about. In practice it:

1. **Violates Principle 14 (World State Is Not Belief State)** — agents plan from the absence of beliefs, treating "I don't know if this place has food" as "maybe it does." Planning should be grounded in accessible belief state, not in its absence.
2. **Creates oscillation loops** — ProgressBarrier plans for evidence-free acquisitions outrank ExploreLocation in goal selection, trapping agents in cycles between known barren locations instead of exploring new frontiers.
3. **Blocks multi-hop resource discovery** — demonstrated in the survival-scattered scenario, where agents with `speculative_acquisition: true` could not discover food 2 hops away because speculative AcquireCommodity(Bread) plans crowded out exploration. Setting it to `false` immediately fixed the problem.
4. **Generates candidates for non-existent commodities** — the agent speculatively plans to acquire Bread at places that have never had Bread and never will, because speculation applies to ALL commodity kinds at ALL known places.

The intended use case (agent remembers a place, wonders if it has food) is already covered by the belief system's staleness and confidence decay — stale beliefs are plannable with lower confidence. The "no known sources" case is covered by S80 (Exploration Drive). The feature fills a gap that doesn't exist.

## Assumption Reassessment (2026-04-16)

1. Before implementation, `speculative_acquisition` was defined in `crates/worldwake-core/src/cognitive_profile.rs` as `pub speculative_acquisition: bool` with default `false`. On the live branch, `survival-baseline.ron` and `cli-evaluation.ron` overrode it to `true`, `survival-scattered.ron` overrode it to `false`, and `default.ron` did not set it. Confirmed 2026-04-16.
2. The feature is consumed in exactly one production place: `crates/worldwake-ai/src/candidate_generation.rs`, where it controls the `include_speculative` flag to `belief_gated_places`. Separate test/doc/scenario references also exist and are current-ticket fallout. Confirmed 2026-04-16.
3. `belief_gated_places` (candidate_generation.rs:4184) uses `include_speculative` to add "known places" (places the agent has observed) as candidate acquisition locations even without commodity-specific evidence. When `false`, only places with direct acquisition support (seller, source, recipe, corpse, loose lot) pass the filter. Confirmed 2026-04-16.
4. **Golden E2E coverage of the feature**: Zero. No golden test exercises `speculative_acquisition: true` behavior or validates it produces beneficial outcomes. The only test is a focused unit test (`belief_gated_places_speculative_includes_known_places`) that verifies the filtering mechanics. Confirmed 2026-04-16.
5. **Survival baseline with `false`**: Needs verification during implementation. The baseline currently sets `true`; changing to `false` should still pass because agents discover food through exploration (Fertile Fields is 2-3 ticks from starting positions). If any baseline test fails, that failure is itself evidence of over-reliance on a broken feature.
6. Shared-profile plus candidate-generation ticket. The owned production boundary is `CognitiveProfile` removal in `worldwake-core` and speculative acquisition-place widening removal in `worldwake-ai::candidate_generation`, with lawful scenario/test/doc fallout in CLI, sim test fixtures, scenario RON, and profile docs. No authoritative world-state or action-validation changes.

## Architecture Check

1. Removal is cleaner than repair. The feature generates evidence-free candidates that are architecturally indistinguishable from noise. Any attempt to "fix" speculation (e.g., limiting it to recently visited places, or penalizing speculative candidates in ranking) adds complexity to compensate for a feature whose use case is already served by belief staleness and exploration. Removing it eliminates the complexity.
2. No backward-compatibility shims needed. The `speculative_acquisition` field can be removed from `CognitiveProfile` and `AgentDef`. Scenario files that set it simply drop the field. RON deserialization with `IMPLICIT_SOME` handles missing fields.
3. Aligns with the repo's no-backward-compatibility rule and `docs/FOUNDATIONS.md` Principle 14: remove the evidence-free planner path instead of preserving it behind compatibility shims or softer ranking tweaks.

## Verification Layers

1. **Baseline survival passes with `false`** → `cargo test -p worldwake-ai --test golden_survival_baseline`
2. **Scattered survival still passes** → `cargo test -p worldwake-ai --test golden_survival_scattered`
3. **CLI evaluation scenario passes** → focused CLI integration load/spawn proof for `scenarios/cli-evaluation.ron`
4. **Focused unit test updated or removed** → `belief_gated_places_speculative_includes_known_places` is deleted (it tests removed functionality)
5. **No regression** → `cargo test --workspace`
6. **CI-matching lint** → `cargo clippy --workspace --all-targets -- -D warnings`

## What to Change

### 1. Remove `speculative_acquisition` from `CognitiveProfile`

Delete the field from `crates/worldwake-core/src/cognitive_profile.rs`. Update `Default` impl. Remove from `ComponentDelta` serialization if present.

### 2. Remove speculation path from `belief_gated_places`

In `crates/worldwake-ai/src/candidate_generation.rs`:
- Remove `include_speculative` from `BeliefGateOptions`
- Remove the `known_places` branch in `belief_gated_places`
- Remove the `FilteredAcquisitionPlace::speculative` field
- Remove the `.or_else(|| filtered_place.speculative.then(...))` fallback in the opportunity construction
- Delete the focused unit test `belief_gated_places_speculative_includes_known_places`

### 3. Remove from scenario RON types and all scenarios

- Remove `speculative_acquisition` from direct `Option<CognitiveProfile>` scenario deserialization coverage in `crates/worldwake-cli/src/scenario/types.rs`
- Remove the field from all authored `.ron` scenario files and any schema-drift comments that mention it
- Remove from test fixtures in `search/tests.rs`, `agent_tick/tests.rs`, `agent_tick/planning.rs`, `failure_handling.rs`, `decision_runtime.rs`, `goal_model.rs`, and scenario/per-belief-view tests that construct explicit `CognitiveProfile` values

### 4. Remove doc and roadmap references

- Remove the live profile-doc entry and any active roadmap/reference text that still describes `speculative_acquisition` as a current architectural feature

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify — remove field)
- `crates/worldwake-core/src/delta.rs` (modify — remove from delta if present)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — remove speculation path + test)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — remove from direct `CognitiveProfile` deserialization coverage)
- `scenarios/survival-baseline.ron` (modify — remove field)
- `scenarios/survival-scattered.ron` (modify — remove field)
- `scenarios/cli-evaluation.ron` (modify — remove field)
- `crates/worldwake-ai/src/search/tests.rs` (modify — remove from fixtures)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — remove from fixtures)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — remove from fixtures)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — remove from fixtures)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify — remove from fixtures)
- `crates/worldwake-ai/src/goal_model.rs` (modify — remove from fixtures)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — test fixture fallout only)
- `docs/profiles/all-profiles.md` (modify — remove from docs)
- `specs/IMPLEMENTATION-ORDER.md` (modify — note that the S83 speculative profile support was later removed by this ticket)

## Out of Scope

- Redesigning the exploration system for multi-hop discovery (already works with `speculative_acquisition: false`)
- Travel branching cap (separate archived ticket `archive/tickets/GOAPTRVLSCAL-001-travel-branching-cap-and-locality-ordering.md`)
- Wash budget exhaustion before basin discovery (same archived ticket)

## Acceptance Criteria

### Tests That Must Pass

1. All golden survival baseline tests pass without `speculative_acquisition`
2. All golden survival scattered tests pass
3. Existing suite: `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `CognitiveProfile` no longer contains `speculative_acquisition`
2. `belief_gated_places` only returns places with positive evidence of the target commodity
3. No scenario file references `speculative_acquisition`
4. Agents rely on exploration (S80) for resource discovery, not evidence-free speculation

## Test Plan

### New/Modified Tests

1. Delete `belief_gated_places_speculative_includes_known_places` — tests removed functionality
2. All test fixtures that set `speculative_acquisition: true` — remove the field (defaults suffice)

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_baseline`
2. `cargo test -p worldwake-ai --test golden_survival_scattered`
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Removed `speculative_acquisition` from `CognitiveProfile`, its defaults/roundtrip coverage, and all explicit fixture literals that still mentioned the field.
- Removed speculative acquisition-place widening from `worldwake-ai::candidate_generation`: `BeliefGateOptions` no longer carries `include_speculative`, `FilteredAcquisitionPlace` is place-only again, and `belief_gated_places_speculative_includes_known_places` was deleted.
- Removed authored scenario/profile references from `survival-baseline.ron`, `survival-scattered.ron`, `cli-evaluation.ron`, `docs/profiles/all-profiles.md`, and updated `specs/IMPLEMENTATION-ORDER.md` to record that the temporary S83 profile support was later removed.

## Deviations

- `scenarios/default.ron` did not reference `speculative_acquisition` on the live branch, so the original `Files to Touch` list overstated that scope and no edit was needed there.
- The honest CLI evaluation proof surface was the existing integration test `cargo test -p worldwake-cli --test integration test_cli_evaluation_scenario_loads_with_infrastructure_retention_profiles -- --exact`, not a separate headless observer invocation.
- `cargo fmt --all` initially reflowed a few unrelated files; those formatter-only changes were removed before closeout so the landed diff stayed on the ticket boundary.

## Verification Result

- Passed `cargo fmt --all`
- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-ai --test golden_survival_baseline`
- Passed `cargo test -p worldwake-ai --test golden_survival_scattered`
- Passed `cargo test -p worldwake-cli --test integration test_cli_evaluation_scenario_loads_with_infrastructure_retention_profiles -- --exact`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
