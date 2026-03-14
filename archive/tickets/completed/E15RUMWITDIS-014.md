# E15RUMWITDIS-014: Replace Hardcoded Belief Confidence Policy With Explicit Profile Data

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` belief-confidence policy data, `PerceptionProfile` shape, and cross-crate profile construction/tests
**Deps**: `archive/tickets/completed/E15RUMWITDIS-010.md`, `specs/E15-rumor-witness-discovery.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/FOUNDATIONS.md`

## Problem

`belief_confidence()` now exists, but its policy is encoded as hardcoded numeric constants in `crates/worldwake-core/src/belief.rs`. That was acceptable as a narrow E15 follow-up when there were no consumers, but it is not the architecture we want to build on:

1. It bakes epistemic policy into code instead of authored/profiled data.
2. It makes future variation awkward because every change becomes a code edit rather than an explicit profile change.
3. It risks ad hoc copycat formulas when planner or downstream systems start depending on confidence in different contexts.

The cleaner long-term architecture is to keep confidence derived, but derive it from explicit serialized profile data rather than from hidden constants.

## Assumption Reassessment (2026-03-14)

1. `crates/worldwake-core/src/belief.rs` contains `belief_confidence(source, staleness_ticks)` and the implementation currently uses hardcoded base values and penalties.
2. The helper currently has no production callers outside `belief.rs` tests, so changing the API now is materially cheaper than waiting until planner or system code depends on the current shape.
3. `specs/E15-rumor-witness-discovery.md` requires confidence to remain derived from provenance and staleness and explicitly forbids storing it as authoritative state.
4. `PerceptionProfile` is already real shared agent state, not an isolated core detail. It is instantiated and serialized across `worldwake-core`, `worldwake-systems`, and `worldwake-ai`, including golden tests and component roundtrip/delta coverage. This ticket therefore changes a cross-crate profile contract, not just a helper signature.
5. The tickets this one originally described as active are no longer active:
   - `archive/tickets/completed/E15RUMWITDIS-011.md` is integration coverage
   - `archive/tickets/E15RUMWITDIS-012.md` is required Tell information-component enforcement
   - `archive/tickets/completed/E15RUMWITDIS-013.md` is event-local witness snapshot work
6. The repository guidance in `AGENTS.md` and `docs/FOUNDATIONS.md` favors explicit, profile-driven parameters over magic numbers when behavior tuning is intended to be part of the simulation model.

## Architecture Check

1. Confidence should remain a derived read-model, but the policy behind that derivation should be explicit data, not hidden constants. That preserves Principle 3 while removing magic-number policy from code.
2. The cleanest ownership model is to nest an explicit serializable confidence-policy value inside `PerceptionProfile`, not to introduce a parallel component. Confidence interpretation belongs with the same per-agent information profile that already owns observation fidelity and memory behavior.
3. Because there are still no production callers, this should be a direct replacement rather than a backwards-compatibility wrapper. Remove the hardcoded-only path instead of supporting both.
4. The migration cost is not zero: `PerceptionProfile` shape changes will propagate through profile constructors, defaults, serialization roundtrips, deltas, and golden tests. That cost is justified now because it is still profile-surface churn, not entrenched gameplay behavior.
5. This ticket should stay narrow despite that migration surface: define the policy object, thread it through the helper and profile defaults, and add tests. It should not prematurely wire confidence into planner ranking unless another ticket concretely needs that behavior.

## Scope Correction

This ticket is broader than `E15RUMWITDIS-010`, but narrower than a planner or cognition redesign.

In scope:

1. Add explicit serialized confidence-policy data in `worldwake-core`
2. Attach that policy to `PerceptionProfile`
3. Replace hardcoded `belief_confidence()` policy with explicit policy-driven derivation
4. Update affected profile construction and serialization/delta coverage across crates
5. Strengthen tests to prove the policy is data-driven, deterministic, and default-ordered

Out of scope:

1. Planner ranking changes that consume confidence
2. Tell behavior changes
3. Discovery/mismatch semantics
4. New authoring tooling or external data loading
5. Any compatibility shim that preserves both hardcoded and policy-driven confidence paths

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

Choose one clean ownership model and use it consistently.

Required approach:

- Nest the policy under `PerceptionProfile`

Rationale:

`PerceptionProfile` is already the authoritative per-agent information profile surface. Adding a second component just for confidence policy would widen the invariant surface, duplicate profile ownership, and recreate the kind of split configuration that `E15RUMWITDIS-012` was already pushing away from. Keep one coherent information profile.

If the nested-policy route is chosen, reshape the profile along these lines:

```rust
pub struct PerceptionProfile {
    pub memory_capacity: u32,
    pub memory_retention_ticks: u64,
    pub observation_fidelity: Permille,
    pub confidence_policy: BeliefConfidencePolicy,
}
```

Update defaults accordingly. The default policy should preserve the current ordering and approximately preserve the current default behavior so this remains an architectural cleanup rather than an unreviewed gameplay rebalance.

### 3. Replace the helper API

Replace the current hardcoded helper with an explicit-policy version, for example:

```rust
pub fn belief_confidence(
    source: &PerceptionSource,
    staleness_ticks: u64,
    policy: &BeliefConfidencePolicy,
) -> Permille
```

Callers should pass `&profile.confidence_policy`.

This must fully replace the current hardcoded-only API. Do not keep both versions alive unless implementation reveals a compelling repository-wide need, which is unlikely because there are no production callers yet.

### 4. Lock the policy behavior down with tests

Add tests proving:

1. ordering is preserved for the default policy
2. chain penalties and staleness penalties are driven by policy values, not hidden constants
3. custom policy values can materially change the derived confidence results
4. `PerceptionProfile` and `BeliefConfidencePolicy` serialization/default behavior remains deterministic
5. cross-crate profile roundtrip/delta tests still reflect the new authoritative profile shape

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/world_txn.rs` (modify if profile layout changes affect component setter/delta tests)
- `crates/worldwake-systems/src/perception.rs` (modify tests/helpers if profile constructors change)
- `crates/worldwake-systems/src/tell_actions.rs` (modify tests/helpers if profile constructors change)
- `crates/worldwake-systems/tests/e15_information_integration.rs` (modify test profile builders)
- `crates/worldwake-ai/tests/` and `crates/worldwake-ai/src/agent_tick.rs` (modify seeded profile construction where literals are used)

## Out of Scope

- Planner ranking changes that consume confidence
- Tell action behavior changes
- Event-local witness snapshot work from `E15RUMWITDIS-013`
- Required-profile invariant cleanup from `E15RUMWITDIS-012`
- Storing confidence as authoritative belief state
- A generalized tuning/authoring UI for profiles

## Acceptance Criteria

### Tests That Must Pass

1. Default-profile confidence ordering still satisfies `DirectObservation > Report(chain_len 1) > Rumor(chain_len 1) > deeper chains`
2. Staleness decay remains monotonic for a fixed source under the default policy
3. Changing policy values changes derived confidence outputs without editing helper code
4. No hidden hardcoded fallback path remains in `belief_confidence()`
5. `PerceptionProfile` default construction, roundtrip, and delta coverage pass with the new nested policy
6. Existing suite: `cargo test -p worldwake-core`
7. `cargo clippy --workspace`
8. `cargo test --workspace`

### Invariants

1. Confidence remains derived from provenance plus age and is never stored as authoritative state
2. Confidence policy is explicit serialized data, not hidden magic numbers in helper code
3. No backwards-compatibility alias keeps both hardcoded and explicit-policy confidence derivation paths alive
4. Confidence policy ownership stays unified under `PerceptionProfile`, not split into a second required agent information component

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — add tests proving default policy preserves the intended provenance ordering
2. `crates/worldwake-core/src/belief.rs` — add tests proving custom policy values alter outputs, so the helper is truly policy-driven
3. `crates/worldwake-core/src/belief.rs` — add serialization/default tests for the new policy data and nested `PerceptionProfile`
4. `crates/worldwake-core/src/component_tables.rs`, `crates/worldwake-core/src/delta.rs`, and `crates/worldwake-core/src/world.rs` — update profile roundtrip/delta/default assertions for the new profile shape
5. Cross-crate tests that build literal `PerceptionProfile` values — update them to keep the seeded profile contract explicit rather than silently inheriting defaults

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Added explicit `BeliefConfidencePolicy` data in `crates/worldwake-core/src/belief.rs` and nested it under `PerceptionProfile`
  - Replaced the hardcoded-only `belief_confidence()` path with a policy-driven helper signature
  - Updated `worldwake-core` roundtrip/default coverage and the cross-crate `PerceptionProfile` literals that construct agent information profiles in systems and AI tests
  - Added focused `worldwake-core` tests proving default ordering, deterministic serialization, monotonic staleness decay, zero-floor saturation, and custom-policy behavior
- Deviations from original plan:
  - Corrected the ticket first because its dependency and scope assumptions were stale: `E15RUMWITDIS-011` and `E15RUMWITDIS-013` were already archived completed, and `E15RUMWITDIS-012` was archived rather than active
  - `world_txn.rs` did not need production changes; the migration surface was profile construction and test/roundtrip coverage, not transaction behavior
  - Kept the ownership model unified under `PerceptionProfile` rather than introducing a second required agent information component
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
