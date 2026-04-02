# S38LRNPREF-006: Route cost penalty in travel estimation

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — travel cost estimation in worldwake-sim (PerAgentBeliefView)
**Deps**: S38LRNPREF-001, S38LRNPREF-003

## Problem

All agents estimate travel costs identically regardless of personal experience. After this ticket, agents with hostile route experience perceive dangerous edges as costlier, making safer alternatives more attractive. This is a tie-breaking influence within priority class, never a suppression mechanism.

## Assumption Reassessment (2026-04-02)

1. `GoalBeliefView::adjacent_places_with_travel_ticks()` at `crates/worldwake-sim/src/per_agent_belief_view.rs:1343` returns `Vec<(EntityId, NonZeroU32)>` — raw topology costs from `edge.travel_time_ticks()`.
2. The planner uses this method for pathfinding and cost estimation. Modifying the returned values transparently affects all downstream planning without planner changes.
3. `NonZeroU32` return type means the penalty must not produce zero (already guaranteed since base_ticks > 0 and penalty is additive).
4. `RouteExperience` and `PreferenceProfile` accessible via `GoalBeliefView` after S38LRNPREF-003.
5. `TravelEdge` in topology provides `id()` returning `TravelEdgeId` and `travel_time_ticks()` returning `u32`.
6. `Permille` arithmetic: `value()` returns `u16`. All computation must use integer arithmetic — no floats. Determinism invariant.
7. The agent calling `adjacent_places_with_travel_ticks` is `self.agent` in `PerAgentBeliefView` — reading its own `RouteExperience` is a self-authoritative read (P14 compliant).

## Architecture Check

1. Modifying `adjacent_places_with_travel_ticks()` in `PerAgentBeliefView` is the minimal-diff approach — the planner already uses this method as its cost oracle. No planner changes needed. Agents without `PreferenceProfile` get default `None` from the trait method and return raw costs unchanged.
2. Alternative: adding a separate `experienced_travel_ticks()` method would require planner modifications. Less clean.
3. No backward-compatibility shims. Agents without `PreferenceProfile` return identical costs to pre-spec behavior.

## Verification Layers

1. No experience → raw topology cost returned → focused unit test
2. Safe-only experience → no penalty applied → focused unit test
3. Hostile experience → proportional penalty applied → focused unit test with known danger ratio
4. No `PreferenceProfile` → raw topology cost returned → focused unit test
5. Integer arithmetic produces correct Permille results → focused unit test with boundary values
6. Single-layer ticket (worldwake-sim belief view); verification via focused tests.

## What to Change

### 1. Modify `adjacent_places_with_travel_ticks` in PerAgentBeliefView

After computing raw `(destination, travel_ticks)` pairs from topology:

1. Check if agent has `PreferenceProfile` and `RouteExperience`. If either is `None`, return raw costs.
2. For each edge: look up `EdgeExperience` by the edge's `TravelEdgeId`.
3. If no experience for this edge: use raw cost.
4. Compute danger ratio in Permille: `hostile_encounters as u32 * 1000 / (safe_trips + hostile_encounters) as u32`.
5. Compute penalty: `effective_ticks = base_ticks * (1000 + route_caution_weight.value() as u32 * danger_ratio / 1000) / 1000`.
6. Clamp to `NonZeroU32` (always >= 1 since base_ticks >= 1 and penalty is additive).
7. Return modified costs.

### 2. Helper function for danger ratio computation

Extract `fn danger_ratio_permille(experience: &EdgeExperience) -> u32` as a pure function for testability. Returns 0 if total trips is 0.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — `adjacent_places_with_travel_ticks` implementation)
- `crates/worldwake-core/src/experience.rs` (modify — add `danger_ratio_permille` helper, or place in worldwake-sim)

## Out of Scope

- Source reliability discount (S38LRNPREF-007)
- Experience recording in action handlers (S38LRNPREF-004, 005)
- Golden tests (S38LRNPREF-008)
- Multi-hop route planning (the planner already handles multi-hop via per-edge costs)

## Acceptance Criteria

### Tests That Must Pass

1. Agent with no `RouteExperience` → raw topology cost
2. Agent with no `PreferenceProfile` → raw topology cost
3. Agent with all-safe experience (5 safe, 0 hostile) → raw topology cost (danger ratio = 0)
4. Agent with 50% hostile experience → cost penalty proportional to `route_caution_weight`
5. Agent with 100% hostile experience → maximum penalty applied
6. Integer arithmetic produces correct results at Permille boundaries
7. `NonZeroU32` result guaranteed (no zero-cost edges)
8. Existing suite: `cargo test --workspace`

### Invariants

1. Agents without `PreferenceProfile` behave identically to pre-spec behavior
2. All arithmetic is integer — no floats (determinism)
3. Route penalty is additive (never reduces cost below base)
4. Experience influences cost estimation, never suppresses route availability

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (new focused tests) — route cost penalty with various experience ratios, no-experience passthrough, no-profile passthrough
2. `crates/worldwake-core/src/experience.rs` (modify) — `danger_ratio_permille` unit tests with boundary values

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-core experience`
3. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

## Outcome

Completed: 2026-04-02

What changed:
- Added shared `danger_ratio_permille` in `crates/worldwake-core/src/experience.rs` and exported it through `worldwake-core`.
- Updated `PerAgentBeliefView::adjacent_places_with_travel_ticks` in `crates/worldwake-sim/src/per_agent_belief_view.rs` to apply an additive hostile-route penalty derived from the actor's own `RouteExperience` and `PreferenceProfile`.
- Left raw topology costs unchanged when the actor has no `RouteExperience`, no `PreferenceProfile`, or only safe travel history for a given edge.
- Added focused belief-view tests covering no-experience passthrough, no-profile passthrough, safe-route passthrough, proportional hostile penalty, and maximum hostile penalty.
- Added focused learned-experience helper tests covering zero, partial, and full hostile danger ratios.

Deviations from original plan:
- No architecture correction was required after reassessment; the ticket remained a clean `worldwake-core` + `worldwake-sim` slice.
- The helper was exported from `worldwake-core` so downstream S38 work can reuse the same ratio logic rather than duplicating it.

Verification results:
- `cargo test -p worldwake-core experience -- --nocapture`
- `cargo test -p worldwake-sim per_agent_belief_view -- --nocapture`
- `cargo test -p worldwake-sim`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
