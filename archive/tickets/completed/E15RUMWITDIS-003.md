# E15RUMWITDIS-003: Add TellProfile Component

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component type in core, component schema registration
**Deps**: None (pure additive)

## Problem

The Tell action requires a per-agent `TellProfile` component controlling information sharing behavior: how many belief subjects an agent offers as Tell affordances, the maximum rumor chain depth it will relay, and a probabilistic acceptance gate for incoming told beliefs. This component enables agent diversity per Principle 20.

## Assumption Reassessment (2026-03-14)

1. Component registration pattern confirmed in `crates/worldwake-core/src/component_schema.rs` — uses macro-generated typed storage, each Agent component has getter/setter/remove methods and EntityKind restriction.
2. `Permille` type confirmed in `crates/worldwake-core/src/numerics.rs` — `Permille(u16)` with `new()` validation, `new_unchecked()` for compile-time constants, and `value()` accessor. Existing non-const `Default` impls use `Permille::new(...).unwrap()` rather than `new_unchecked()`.
3. The schema already has 22 Agent-only component registrations, not 13. `TellProfile` will become the 23rd Agent-only component and the 32nd authoritative component overall.
4. The schema manifest fans out into `ComponentTables`, `ComponentKind`/`ComponentValue`, `World`, and `WorldTxn` via macros. Adding a schema entry changes generated APIs and several exact-manifest tests even if those files need no manual edits.
5. `EventTag::Social`, `EventTag::Discovery`, `SocialObservationKind::WitnessedTelling`, and `ActionDomain::Social` are already implemented. This ticket should stay narrowly focused on the per-agent Tell profile.
6. `World::create_agent()` currently auto-attaches `AgentBeliefStore` and `PerceptionProfile`, and the core test suite asserts that default profile attachment and the exact creation delta shape. If `TellProfile` is a required agent behavior profile, the clean architecture is to attach it there as well instead of leaving it optional.
7. Component trait impl pattern remains `impl Component for TellProfile {}` with the trait defined in `crates/worldwake-core/src/traits.rs`.

## Architecture Check

1. Adding `TellProfile` is beneficial only if it becomes a first-class agent profile parallel to `PerceptionProfile`: authoritative, typed, default-backed, and always present on newly created agents. That keeps Tell logic simple and avoids optional-profile branches or ad hoc fallback behavior later.
2. Placing `TellProfile` in `crates/worldwake-core/src/belief.rs` alongside `PerceptionProfile` matches the existing architecture: both are per-agent information-handling profiles rather than action-system state.
3. Default values (max_tell_candidates: 3, max_relay_chain_len: 3, acceptance_fidelity: `Permille(800)`) still come directly from the E15 spec, but the implementation should use the repo’s normal `Permille::new(...).unwrap()` default pattern.
4. No backwards-compatibility shims or alias components.

## What to Change

### 1. Define `TellProfile` struct

In `crates/worldwake-core/src/belief.rs` (alongside PerceptionProfile), add:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TellProfile {
    pub max_tell_candidates: u8,
    pub max_relay_chain_len: u8,
    pub acceptance_fidelity: Permille,
}

impl Default for TellProfile {
    fn default() -> Self {
        Self {
            max_tell_candidates: 3,
            max_relay_chain_len: 3,
            acceptance_fidelity: Permille::new(800).unwrap(),
        }
    }
}

impl Component for TellProfile {}
```

### 2. Register TellProfile in component schema

In `crates/worldwake-core/src/component_schema.rs`, add TellProfile registration for `EntityKind::Agent` following the same macro pattern as PerceptionProfile.

### 3. Attach TellProfile to newly created agents

Update `World::create_agent()` so every new agent receives `TellProfile::default()` together with `AgentBeliefStore` and `PerceptionProfile`.

Rationale: Tell behavior is core agent information behavior, not an optional bolt-on. Making it present-by-default gives the cleanest downstream API and avoids missing-component branches in action handlers, affordance enumeration, and AI.

### 4. Export TellProfile

In `crates/worldwake-core/src/lib.rs`, add `TellProfile` to public exports.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add TellProfile struct)
- `crates/worldwake-core/src/component_schema.rs` (modify — register TellProfile for Agent)
- `crates/worldwake-core/src/world.rs` (modify — attach TellProfile in `create_agent()`)
- `crates/worldwake-core/src/lib.rs` (modify — export TellProfile)

## Files Likely Affected Indirectly By The Schema Entry

- `crates/worldwake-core/src/component_tables.rs` tests
- `crates/worldwake-core/src/delta.rs` tests
- `crates/worldwake-core/src/world_txn.rs` tests

These files should only need targeted expectation/test updates unless the implementation reveals a missing abstraction.

## Out of Scope

- Tell action definition, payload, or handler
- Affordance enumeration logic using TellProfile
- MismatchKind or discovery events
- Any AI/planner changes
- Modifying existing components

## Acceptance Criteria

### Tests That Must Pass

1. `TellProfile::default()` returns max_tell_candidates=3, max_relay_chain_len=3, acceptance_fidelity=Permille(800)
2. TellProfile can be set and retrieved on an Agent entity via component schema getters/setters
3. TellProfile is rejected for non-Agent entity kinds
4. TellProfile serializes and deserializes correctly (roundtrip)
5. WorldTxn can set and commit TellProfile changes
6. `World::create_agent()` attaches `TellProfile::default()` to every new agent
7. Agent-creation and manifest-projection tests are updated to reflect the new authoritative component
8. Existing suite: `cargo test --workspace`
9. `cargo clippy --workspace`

### Invariants

1. TellProfile registered only on EntityKind::Agent
2. Default values match spec exactly
3. All existing component registrations unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — unit test for `TellProfile::default()` values and serde/component trait bounds
2. `crates/worldwake-core/src/world.rs` — component roundtrip on Agent, rejection on non-Agent, and `create_agent()` default attachment
3. `crates/worldwake-core/src/world_txn.rs` — set/commit and clear expectations for `TellProfile`, plus updated `create_agent()` delta expectations
4. `crates/worldwake-core/src/component_tables.rs` and `crates/worldwake-core/src/delta.rs` — exact-manifest regression updates for the new authoritative component

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Added `TellProfile` to `crates/worldwake-core/src/belief.rs` with the E15 default values and public export from `worldwake-core`.
  - Registered `TellProfile` as an Agent-only authoritative component through the shared component schema so it flows through component tables, delta typing, world APIs, and `WorldTxn` setters/clearers.
  - Updated `World::create_agent()` so newly created agents receive `TellProfile::default()` automatically, matching the existing pattern for belief/perception defaults.
  - Added and updated regression tests across belief, world, component table, delta, and transaction coverage.
- Deviations from original plan:
  - The ticket was corrected before implementation because its assumptions about the current codebase were stale.
  - The implementation intentionally went beyond simple registration/export by attaching `TellProfile` during agent creation. This is the cleaner long-term architecture because Tell behavior is a core agent information profile, not an optional add-on.
  - The implementation used `Permille::new(800).unwrap()` in `Default` for consistency with existing non-const defaults rather than `Permille::new_unchecked(800)`.
- Verification results:
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
