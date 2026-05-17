# S151TESRELROU-008: Travel cost integration with RoutePreference

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — extends `perceived_direct_travel_cost_from_memory` signature to accept `RoutePreference` + `RoutePreferenceProfile`, plus caller threading
**Deps**: archive/tickets/S151TESRELROU-001.md, S151TESRELROU-002, S151TESRELROU-004

## Problem

S151's D8 integrates `RoutePreference` into the planner's travel-cost computation as an additive bias atop the existing route-threat penalty. Routes with high safe-traversal counts cost less; routes with high dangerous-traversal counts cost more. The integration extends `perceived_direct_travel_cost_from_memory` at `crates/worldwake-ai/src/route_threat.rs:187-212` rather than adding a parallel cost function.

## Assumption Reassessment (2026-05-17)

1. `perceived_direct_travel_cost_from_memory` at `crates/worldwake-ai/src/route_threat.rs:187-212` currently takes `(current_tick, confidence_policy, entity_beliefs, social_observations, edge_from, edge_to, base_ticks)` and returns `u32` (base_ticks + threat penalty). The function is `pub(crate)`. Extending the signature with two new parameters (`Option<&RoutePreference>` and `Option<&RoutePreferenceProfile>`) is mechanical at the function-definition level but requires updating all callers in the planner.
2. **Existing tests exercising this function**: `perceived_direct_travel_cost_scales_with_route_threat` at `route_threat.rs:447` and `perceived_direct_travel_cost_scales_with_threat_warning_notice` at `route_threat.rs:475`. Both must be updated to pass the new parameters (likely `None`/`None` to preserve their existing behavior), and a new sibling test must cover the `RoutePreference`-driven cost adjustment.
3. `RouteSegment` at `crates/worldwake-core/src/blocker_scope.rs:67-81` has a `::new(from, to)` constructor that canonicalizes endpoint order — the function constructs the lookup key from `(edge_from, edge_to)` via this constructor, making the preference lookup direction-independent.
4. Caller sites: identify via `grep -rn perceived_direct_travel_cost_from_memory crates/` during implementation. The function is `pub(crate)`, so callers are within `worldwake-ai`. Each caller has access to the agent's `AgentDecisionRuntime` (for `RoutePreference`) and a `GoalBeliefView` (for `RoutePreferenceProfile` via ticket 004's accessor).
5. Per the Authoritative-to-AI Impact Rule (CLAUDE.md): this ticket modifies a cost function consumed by planner search (affects #3 plan search ranking). No precondition or validation changes; no affordance enumeration changes; no candidate-emission changes; no plan-failure handler changes. Golden tests in ticket 011 cover the end-to-end behavior.

## Architecture Check

1. Per FND-3: route preference is concrete state with traversal counts and event provenance, not an abstract danger score. The derived `preference: Permille` is read at query time, not stored as authoritative truth.
2. Per FND-26: state-mediated; `perceived_direct_travel_cost_from_memory` reads `RoutePreference` (from `AgentDecisionRuntime`) and `RoutePreferenceProfile` (via `GoalBeliefView`), produces a cost number. No cross-system command path.
3. Per FND-28: `Option<&RoutePreference>` and `Option<&RoutePreferenceProfile>` parameters preserve a `None`-call path for tests and bootstrap code that doesn't yet have populated preferences — but the production planner callers always pass `Some(...)`. No deprecated parallel function.
4. Existing route-threat penalty continues to dominate near-term hazards (computed first); preference is additive on top — soft bias, not entitlement.
5. Direction-independent lookup via `RouteSegment::new(edge_from, edge_to)` matches `BlockerScope::RouteSegment` (S150) so a route is "preferred" or "avoided" symmetrically regardless of travel direction.

## Verification Layers

1. Cost-function correctness → focused unit test in `route_threat.rs#[cfg(test)]` — agent with positive preference for `(A, B)` produces lower cost than baseline; agent with negative preference produces higher cost.
2. Threat dominance → focused unit test asserting that a strong threat signal still produces the dominant penalty even when route preference is positive.
3. None-call backward compatibility → existing tests at `route_threat.rs:447, 475` pass `None`/`None` and produce identical output to current behavior.
4. Direction symmetry → preference lookup for `(A, B)` and `(B, A)` returns the same `RoutePreferenceEntry` because `RouteSegment::new` canonicalizes.

## What to Change

### 1. Extend `perceived_direct_travel_cost_from_memory` (`crates/worldwake-ai/src/route_threat.rs:187-212`)

```rust
pub(crate) fn perceived_direct_travel_cost_from_memory(
    current_tick: Tick,
    confidence_policy: BeliefConfidencePolicy,
    entity_beliefs: &BTreeMap<EntityId, BelievedEntityState>,
    social_observations: &[SocialObservation],
    edge_from: EntityId,
    edge_to: EntityId,
    base_ticks: u32,
    route_preference: Option<&RoutePreference>,
    route_preference_profile: Option<&RoutePreferenceProfile>,
) -> u32 {
    let threat = route_threat_estimate_from_memory(/* existing args */);
    let after_threat = if threat.value() == 0 {
        base_ticks
    } else {
        base_ticks.saturating_add(base_ticks.saturating_mul(u32::from(threat.value())).div_ceil(1000))
    };

    // S151 route preference modifier (additive on top of threat penalty)
    let (Some(pref), Some(profile)) = (route_preference, route_preference_profile) else {
        return after_threat;
    };
    let segment = RouteSegment::new(edge_from, edge_to);
    let Some(entry) = pref.get(&segment) else {
        return after_threat;
    };
    let preference = entry.preference(profile, current_tick);  // Permille; 500 = neutral
    apply_preference_modifier(after_threat, preference)
}

fn apply_preference_modifier(cost: u32, preference: Permille) -> u32 {
    // preference > 500 → cost reduction; preference < 500 → cost increase
    // Magnitude proportional to |preference - 500|. Exact formula determined during implementation;
    // clamp the result to [base_ticks_minimum_floor, base_ticks_saturation_ceiling].
    todo!()
}
```

### 2. Update all callers

Enumerate via `grep -rn perceived_direct_travel_cost_from_memory crates/worldwake-ai/src/`. For each caller:

- If the caller is within the planner (has access to the agent's `AgentDecisionRuntime` and a `GoalBeliefView`), pass `Some(&runtime.route_preference)` and `Some(belief_view.route_preference_profile(agent))`.
- If the caller is a test or bootstrap that doesn't have these, pass `None`/`None` (signature is backward-compatible for the no-preference path).

### 3. Update existing tests

- `route_threat.rs:447 perceived_direct_travel_cost_scales_with_route_threat`: append `None, None` to the call.
- `route_threat.rs:475 perceived_direct_travel_cost_scales_with_threat_warning_notice`: append `None, None`.

### 4. Add new test

- `perceived_direct_travel_cost_scales_with_route_preference`: agent with positive preference produces lower cost than baseline; negative preference produces higher cost; preference-driven adjustment composes with threat penalty.

## Files to Touch

- `crates/worldwake-ai/src/route_threat.rs` (modify — signature extension, modifier helper, test updates + new test)
- Caller sites within `crates/worldwake-ai/src/` (enumerate via grep at implementation time — likely `crates/worldwake-ai/src/search/strategic.rs`, `crates/worldwake-ai/src/search/tactical.rs`, or a planner-cost utility)

## Out of Scope

- Ranking damping + emission suppression for testimony — ticket 007
- Observation hook populating `RoutePreference` — ticket 006
- Diagnostics aggregator — ticket 009
- `SAVE_FORMAT_VERSION` bump — ticket 010

## Acceptance Criteria

### Tests That Must Pass

1. `perceived_direct_travel_cost_scales_with_route_preference` (new) — preference > 500 reduces cost; preference < 500 increases cost; preference == 500 (neutral) leaves cost unchanged from threat-only baseline.
2. `perceived_direct_travel_cost_scales_with_route_threat` (existing, line 447) — passes with `None`/`None` arguments preserving original behavior.
3. `perceived_direct_travel_cost_scales_with_threat_warning_notice` (existing, line 475) — passes with `None`/`None` arguments.
4. Direction symmetry: preference lookup for `(A, B)` and `(B, A)` returns the same entry.
5. Existing suite: `cargo test --workspace`.

### Invariants

1. Threat-driven penalty continues to dominate near-term hazards — preference is additive bias, not multiplicative override.
2. Cost remains `u32` and is bounded below by `base_ticks_minimum_floor` (no negative or underflow values).
3. `RouteSegment::new(edge_from, edge_to)` is the canonical lookup key — direction independence preserved.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/route_threat.rs#[cfg(test)]` — new `perceived_direct_travel_cost_scales_with_route_preference` test.
2. `crates/worldwake-ai/src/route_threat.rs:447, 475` — update existing tests to pass `None`/`None`.

### Commands

1. `cargo test -p worldwake-ai route_threat`
2. `cargo test -p worldwake-ai` (broad — caller-site compile + behavior preservation)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh`
