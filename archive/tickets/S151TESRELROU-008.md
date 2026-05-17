# S151TESRELROU-008: Travel cost integration with RoutePreference

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner travel-cost snapshots now incorporate learned `RoutePreference` state alongside route-threat memory
**Deps**: archive/tickets/S151TESRELROU-001.md, archive/tickets/S151TESRELROU-002.md, archive/tickets/S151TESRELROU-004.md, archive/tickets/S151TESRELROU-006.md

## Problem

S151's D8 integrates `RoutePreference` into the planner's travel-cost computation as an additive bias atop the existing route-threat penalty. Routes with high safe-traversal counts cost less; routes with high dangerous-traversal counts cost more. The integration extends `perceived_direct_travel_cost_from_memory` in `crates/worldwake-ai/src/route_threat.rs` rather than adding a parallel cost function.

## Assumption Reassessment (2026-05-17)

1. The live route-cost helper only had one production caller, through `PlanningSnapshot` construction, rather than separate search/strategic call sites. The implementation therefore threaded route preferences into snapshot construction and candidate-planning entry points instead of changing unrelated search modules directly.
2. Learned route preference lives on `AgentDecisionRuntime`, while `RoutePreferenceProfile` is read from the runtime belief view. The production candidate-planning paths pass `Some(&runtime.route_preference)` and the actor's live profile into the snapshot. Neutral test/bootstrap snapshot builders still pass `None`/`None`.
3. `RouteSegment::new(edge_from, edge_to)` remains the canonical lookup key, so preference lookup is direction-independent.
4. Per the Authoritative-to-AI Impact Rule in `AGENTS.md`, this ticket changes planner search cost ranking only. It does not change affordance enumeration, candidate emission, authoritative preconditions, action start validation, or plan-failure handling.

## Architecture Check

1. Per FND-3: route preference is concrete learned state with traversal counts and event provenance. The derived `preference: Permille` is read at query time, not stored as authoritative truth.
2. Per FND-26: the path is state-mediated. Planner cost computation reads `RoutePreference` plus `RoutePreferenceProfile` and produces a cost number; it does not command another system.
3. Per FND-28: the `None` path is retained for tests and bootstrap snapshots, while runtime-owned planner callers now pass the learned route preference state.
4. Existing route-threat penalty is computed first. Preference then applies a bounded additive adjustment derived from `base_ticks * abs(preference - 500) / 1000`, so preference is a soft bias rather than a hazard override.
5. Snapshot direct-cost consumers and matrix-cost consumers now share the same preference-aware calculation.

## Outcome

Implemented `RoutePreference`-aware travel costs in `crates/worldwake-ai/src/route_threat.rs`:

- `perceived_direct_travel_cost_from_memory` now accepts `Option<&RoutePreference>` and `Option<&RoutePreferenceProfile>`.
- Neutral, missing-entry, or missing-profile calls preserve the existing threat-only result.
- Preference above 500 reduces perceived cost; preference below 500 increases perceived cost; the adjustment is bounded by saturating arithmetic and a minimum cost of 1.
- Lookup uses `RouteSegment::new(edge_from, edge_to)`, preserving direction symmetry.

Integrated the new cost path through planner snapshots:

- `crates/worldwake-ai/src/planning_snapshot.rs` added route-preference-aware snapshot construction and stores the snapshot's preference inputs so `direct_perceived_travel_cost`, heuristic tracing, and matrix costs agree.
- `crates/worldwake-ai/src/agent_tick/planning.rs` threads `runtime.route_preference` and the actor `RoutePreferenceProfile` into production candidate-plan search snapshots.
- `crates/worldwake-ai/src/agent_tick/active_action.rs` uses the same preference-aware candidate-plan builder for interrupt planning.
- Existing neutral snapshot builders and test-only candidate-plan helpers remain available with `None` preference inputs.

Added focused tests:

- `route_preference_biases_perceived_direct_travel_cost_around_neutral`
- `route_preference_cost_bias_uses_canonical_direction`
- `snapshot_perceived_travel_cost_applies_route_preference`

## Verification Result

- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai route_preference_biases_perceived_direct_travel_cost_around_neutral`
- Passed `cargo test -p worldwake-ai route_threat`
- Passed `cargo test -p worldwake-ai snapshot_perceived_travel_cost_applies_route_preference`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
- Passed `cargo fmt --all -- --check`
- Passed `cargo clippy --workspace`

`./scripts/verify.sh` was covered by direct execution of its current documented gate set: fmt check, workspace tests, workspace clippy, and all-target workspace clippy with warnings denied.

## Acceptance Criteria

1. Preference greater than 500 reduces perceived travel cost relative to neutral baseline.
2. Preference less than 500 increases perceived travel cost relative to neutral baseline.
3. Neutral or absent preference leaves existing threat-only behavior unchanged.
4. Preference lookup is direction-symmetric via canonical `RouteSegment`.
5. Existing route-threat tests pass with `None`/`None` preference arguments.
6. The workspace test suite passes.

## Invariants

1. Threat-driven penalty continues to represent near-term hazards; preference is a bounded additive bias, not a multiplicative override.
2. Cost remains `u32` and cannot underflow below 1 on the preference-reduction path.
3. `RouteSegment::new(edge_from, edge_to)` is the canonical lookup key.
4. The implementation remains belief/planner-side and does not change authoritative action validation.

## Out of Scope

- Ranking damping and emission suppression for testimony — ticket 007.
- Observation hooks populating `RoutePreference` — ticket 006.
- Diagnostics aggregator — ticket 009.
- `SAVE_FORMAT_VERSION` bump — ticket 010.
