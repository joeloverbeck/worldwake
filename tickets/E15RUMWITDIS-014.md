# E15RUMWITDIS-014: Replace Hardcoded Belief Confidence Policy With Explicit Profile Data

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` belief confidence API and agent profile shape
**Deps**: `archive/tickets/completed/E15RUMWITDIS-010.md`, `specs/E15-rumor-witness-discovery.md`, `docs/FOUNDATIONS.md`

## Problem

`belief_confidence()` now exists, but its policy is encoded as hardcoded numeric constants in code. That is acceptable as a stopgap, but it is not the architecture we want to build on:

1. It bakes epistemic policy into code instead of authored/profiled data.
2. It makes future variation awkward because every change becomes a code edit rather than an explicit profile change.
3. It risks ad hoc copycat formulas when planner or downstream systems start depending on confidence in different contexts.

The cleaner long-term architecture is to keep confidence derived, but derive it from explicit profile data rather than from hidden constants.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-core/src/belief.rs` now contains `belief_confidence(source, staleness_ticks)` and the implementation currently uses hardcoded base values and penalties.
2. The helper currently has no production callers outside its own tests, so changing the API now is materially cheaper than waiting until planner or system code depends on the current shape.
3. `specs/E15-rumor-witness-discovery.md` requires confidence to remain derived from provenance and staleness and explicitly forbids storing it as authoritative state.
4. Active tickets `E15RUMWITDIS-011`, `E15RUMWITDIS-012`, and `E15RUMWITDIS-013` do not own this concern:
   - `E15RUMWITDIS-011` is integration coverage only
   - `E15RUMWITDIS-012` enforces required presence of existing information-sharing profiles
   - `E15RUMWITDIS-013` fixes event-local witness snapshots
5. The repository guidance in `AGENTS.md` and `docs/FOUNDATIONS.md` favors explicit, profile-driven parameters over magic numbers when behavior tuning is intended to be part of the simulation model.

## Architecture Check

1. Confidence should remain a derived read-model, but the policy behind that derivation should be explicit data, not hidden constants. That preserves Principle 3 while removing magic-number policy from code.
2. The cleanest home is an explicit serializable confidence-policy value owned by agent information/perception configuration, not scattered constants in helper functions.
3. Because there are no downstream callers yet, this should be a direct replacement rather than a backwards-compatibility wrapper. Remove the hardcoded-only path instead of supporting both.
4. This ticket should keep scope narrow: define the policy object, thread it through the helper, and add tests. It should not prematurely wire confidence into planner ranking unless another ticket concretely needs that behavior.

## What to Change

### 1. Introduce explicit belief-confidence policy data

Add a serializable deterministic policy type in `worldwake-core`, for example:

```rust
pub struct BeliefConfidencePolicy {
    pub direct_observation_base: Permille,
    pub report_base: Permille,
    pub rumor_base: Permille,
    pub inference_base: Permille,
    pub report_chain_penalty: Permille,
    pub rumor_chain_penalty: Permille,
    pub staleness_penalty_per_tick: Permille,
}
```

The exact field names may vary, but the policy must make the current confidence ladder explicit in data.

Requirements:

1. Integer-only and deterministic.
2. Serializable and stable for save/load.
3. Sufficient to express provenance ordering and staleness decay without hidden fallback constants.

### 2. Attach the policy to the agent-side information profile surface

Choose one clean ownership model and use it consistently:

- Preferred: nest the policy under `PerceptionProfile`
- Acceptable alternative if code structure demands it: a dedicated `BeliefConfidenceProfile` component attached to agents

Recommendation:

Use `PerceptionProfile` as the owning profile surface because confidence interpretation is part of how an agent evaluates observed/reported information over time, and `PerceptionProfile` already owns memory and sensing-related tuning.

If the nested-policy route is chosen, reshape the profile along these lines:

```rust
pub struct PerceptionProfile {
    pub memory_capacity: u32,
    pub memory_retention_ticks: u64,
    pub observation_fidelity: Permille,
    pub confidence_policy: BeliefConfidencePolicy,
}
```

Update defaults accordingly.

### 3. Replace the helper API

Replace the current hardcoded helper with an explicit-policy version, for example:

```rust
pub fn belief_confidence(
    source: &PerceptionSource,
    staleness_ticks: u64,
    policy: &BeliefConfidencePolicy,
) -> Permille
```

If the policy lives under `PerceptionProfile`, callers can pass `&profile.confidence_policy`.

This must fully replace the current hardcoded-only API. Do not keep both versions alive unless implementation reveals a compelling repository-wide need, which is unlikely because there are no production callers yet.

### 4. Lock the policy behavior down with tests

Add tests proving:

1. ordering is preserved for the default policy
2. chain penalties and staleness penalties are driven by policy values, not hidden constants
3. custom policy values can materially change the derived confidence results
4. serialization/default behavior remains deterministic

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify if profile defaults/factory wiring need updates)
- `crates/worldwake-core/src/component_tables.rs` (modify if profile layout changes affect roundtrip tests)
- `crates/worldwake-core/src/delta.rs` (modify if profile layout changes affect deltas)
- `crates/worldwake-core/src/world_txn.rs` (modify if profile layout changes affect component setters/deltas)

## Out of Scope

- Planner ranking changes that consume confidence
- Tell action behavior changes
- Event-local witness snapshot work in `E15RUMWITDIS-013`
- Required-profile invariant cleanup in `E15RUMWITDIS-012`
- Storing confidence as authoritative belief state
- A generalized tuning/authoring UI for profiles

## Acceptance Criteria

### Tests That Must Pass

1. Default-profile confidence ordering still satisfies `DirectObservation > Report(chain_len 1) > Rumor(chain_len 1) > deeper chains`
2. Staleness decay remains monotonic for a fixed source under the default policy
3. Changing policy values changes derived confidence outputs without editing helper code
4. No hidden hardcoded fallback path remains in `belief_confidence()`
5. Existing suite: `cargo test -p worldwake-core`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

### Invariants

1. Confidence remains derived from provenance plus age and is never stored as authoritative state
2. Confidence policy is explicit serialized data, not hidden magic numbers in helper code
3. No backwards-compatibility alias keeps both hardcoded and explicit-policy confidence derivation paths alive

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — add tests proving default policy preserves the intended provenance ordering
2. `crates/worldwake-core/src/belief.rs` — add tests proving custom policy values alter outputs, so the helper is truly policy-driven
3. `crates/worldwake-core/src/belief.rs` — add serialization/default tests for the new policy data
4. `crates/worldwake-core/src/world.rs` and related core tests — update agent/profile roundtrip assertions if the profile shape changes

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
