# S83BELCANDPR-003: Belief-gated place filtering in AcquireCommodity candidates

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — candidate generation filtering, diagnostics
**Deps**: S83BELCANDPR-001, S83BELCANDPR-002

## Problem

`acquisition_path_opportunities_inner()` currently passes all topologically reachable places (potentially 1000-7000+) to candidate generation for `AcquireCommodity` goals. Each place generates a candidate that the 224-300 expansion budget cannot process, causing planner budget exhaustion. This ticket adds a belief-gating layer that filters reachable places to only those where the agent believes the target commodity exists, reducing the candidate set to a handful of belief-supported places.

## Assumption Reassessment (2026-04-10)

1. `acquisition_path_opportunities_inner` at `crates/worldwake-ai/src/candidate_generation.rs:4058`. Returns `Vec<(EntityId, Evidence, EvidenceTrace)>`. Uses `reachable_places_within_horizon(view, origin, travel_horizon).into_iter().filter_map(...)` chain at lines 4071-4085.
2. `resource_sources_at(place, commodity)` on `GoalBeliefView` at `belief_view.rs:184` — 2-param signature (no agent), belief view is agent-scoped. `controlled_commodity_quantity_at_place(agent, place, commodity)` returns `Quantity` (newtype `Quantity(pub u32)` at `numerics.rs:75`).
3. `known_place_observations(view, agent)` at `candidate_generation.rs:4147` returns `BTreeMap<EntityId, Tick>` of places the agent has beliefs about. Reused for speculative check instead of adding a new trait method.
4. `CandidateGenerationDiagnostics` at `candidate_generation.rs:159` has 6 fields with `#[derive(Default)]`. New `u32` fields default to 0.
5. `emit_self_consume_candidates` at line 2284 has `&mut diagnostics` access — this is where diagnostic counters will be recorded.
6. `acquisition_path_opportunities` (line 4014) and `direct_acquisition_path_opportunities` (line 4036) both delegate to `_inner`. The belief filter in `_inner` covers both paths.

## Architecture Check

1. Belief-gated filtering is the minimal intervention: one new private function + one filter insertion point. No new types, no new traits, no new actions. The existing `known_place_observations()` helper is reused for the speculative path, avoiding trait surface growth.
2. No backward-compatibility shims. The change replaces exhaustive enumeration with belief-informed enumeration. Agents with `speculative_acquisition: false` (the default) only consider places with positive belief evidence. Agents with no beliefs about remote resources generate zero remote acquisition candidates, falling through to S80's `ExploreLocation`.

## Verification Layers

1. Belief-gated filtering reduces candidate set to belief-supported places -> focused unit test with TestBeliefView
2. Agent with no remote beliefs generates zero remote acquisition candidates -> focused unit test
3. Speculative mode includes known-but-no-evidence places -> focused unit test
4. Diagnostic counters record filtering ratio -> focused unit test
5. Existing golden tests continue passing (no behavioral change for agents that already have beliefs about resource places) -> golden E2E suite
6. Single-system ticket (candidate generation only); cross-system layer mapping not applicable beyond the belief view read boundary verified in ticket 002.

## What to Change

### 1. Add `belief_gated_places` function

In `crates/worldwake-ai/src/candidate_generation.rs`, add a private function:

```rust
fn belief_gated_places(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    reachable: &[EntityId],
    commodity: CommodityKind,
    include_speculative: bool,
) -> Vec<EntityId>
```

Logic:
- Always include the agent's current place (local acquisition)
- Include places where `view.resource_sources_at(place, commodity)` is non-empty
- Include places where `view.controlled_commodity_quantity_at_place(agent, place, commodity).0 > 0` (agent's own remote stockpiles)
- If `include_speculative`: include places in `known_place_observations(view, agent)`

### 2. Integrate into `acquisition_path_opportunities_inner`

Replace the direct `.into_iter().filter_map()` chain with:

```rust
let cognitive = view.cognitive_profile(agent);
let include_speculative = cognitive
    .map(|p| p.speculative_acquisition)
    .unwrap_or(false);
let reachable = reachable_places_within_horizon(view, origin, travel_horizon);
let belief_filtered = belief_gated_places(
    view, agent, &reachable, commodity, include_speculative,
);
belief_filtered
    .into_iter()
    .filter_map(|candidate_place| { ... })
    .collect()
```

### 3. Add diagnostic fields to CandidateGenerationDiagnostics

Add two fields:

```rust
pub places_reachable: u32,
pub places_after_belief_filter: u32,
```

Record these at the call sites in `emit_self_consume_candidates` (or propagate from `_inner` via return value).

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Hierarchical plan decomposition (TravelTo + AcquireLocal subgoals)
- Dynamic expansion budget scaling per-agent
- Modifying `reachable_places_within_horizon()` itself
- Filtering for goal kinds other than `AcquireCommodity`

## Acceptance Criteria

### Tests That Must Pass

1. `belief_gated_places` returns only the agent's current place when no remote beliefs exist
2. `belief_gated_places` includes places with believed resource sources
3. `belief_gated_places` includes places with agent's own controlled commodity
4. `belief_gated_places` includes known-but-no-evidence places only when `include_speculative` is true
5. `acquisition_path_opportunities_inner` produces fewer candidates than reachable places when beliefs are sparse
6. Diagnostic counters correctly reflect pre- and post-filter counts
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Local acquisition candidates (at agent's current place) are always generated regardless of belief state
2. Agents with no remote beliefs generate zero remote acquisition candidates (FND-14, FND-15)
3. The belief view is the sole source of filtering data — no authoritative world state is read (FND-14)
4. `known_place_observations` is reused, not duplicated (DRY)
5. All existing golden tests pass: `cargo test -p worldwake-ai -- golden`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs::belief_gated_places_returns_current_place_with_no_remote_beliefs` — verifies zero remote candidates when agent has no beliefs
2. `crates/worldwake-ai/src/candidate_generation.rs::belief_gated_places_includes_resource_source_places` — verifies believed resource sources pass the filter
3. `crates/worldwake-ai/src/candidate_generation.rs::belief_gated_places_includes_controlled_commodity_places` — verifies agent's own remote stockpiles pass the filter
4. `crates/worldwake-ai/src/candidate_generation.rs::belief_gated_places_speculative_includes_known_places` — verifies speculative mode includes visited places without current evidence
5. `crates/worldwake-ai/src/candidate_generation.rs::belief_gated_diagnostics_records_filtering_ratio` — verifies diagnostic counters

### Commands

1. `cargo test -p worldwake-ai belief_gated`
2. `cargo test -p worldwake-ai -- golden`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
