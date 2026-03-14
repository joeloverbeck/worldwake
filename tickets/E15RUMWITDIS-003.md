# E15RUMWITDIS-003: Add TellProfile Component

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new component type in core, component schema registration
**Deps**: None (pure additive)

## Problem

The Tell action requires a per-agent `TellProfile` component controlling information sharing behavior: how many belief subjects an agent offers as Tell affordances, the maximum rumor chain depth it will relay, and a probabilistic acceptance gate for incoming told beliefs. This component enables agent diversity per Principle 20.

## Assumption Reassessment (2026-03-14)

1. Component registration pattern confirmed in `crates/worldwake-core/src/component_schema.rs` — uses macro-generated typed storage, each Agent component has getter/setter/remove methods and EntityKind restriction.
2. `Permille` type confirmed in `crates/worldwake-core/src/numerics.rs` — `Permille(u16)` with `new()` validation and `value()` accessor.
3. 13 components currently registered for `EntityKind::Agent` in component_schema.rs. TellProfile will be the 14th.
4. Component trait impl pattern: `impl Component for TellProfile {}` with the trait defined in `crates/worldwake-core/src/traits.rs`.

## Architecture Check

1. Follows established component registration pattern exactly — no new patterns introduced.
2. Default values (max_tell_candidates: 3, max_relay_chain_len: 3, acceptance_fidelity: Permille(800)) come directly from the spec.
3. No backwards-compatibility shims.

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
            acceptance_fidelity: Permille::new_unchecked(800),
        }
    }
}

impl Component for TellProfile {}
```

### 2. Register TellProfile in component schema

In `crates/worldwake-core/src/component_schema.rs`, add TellProfile registration for `EntityKind::Agent` following the same macro pattern as PerceptionProfile.

### 3. Export TellProfile

In `crates/worldwake-core/src/lib.rs`, add `TellProfile` to public exports.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add TellProfile struct)
- `crates/worldwake-core/src/component_schema.rs` (modify — register TellProfile for Agent)
- `crates/worldwake-core/src/component_tables.rs` (modify — add typed storage via macro)
- `crates/worldwake-core/src/lib.rs` (modify — export TellProfile)

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
6. Existing suite: `cargo test --workspace`
7. `cargo clippy --workspace`

### Invariants

1. TellProfile registered only on EntityKind::Agent
2. Default values match spec exactly
3. All existing component registrations unchanged

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — unit test for TellProfile::default() values
2. `crates/worldwake-core/src/component_schema.rs` or integration tests — set/get/remove TellProfile on Agent; verify rejection on non-Agent

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
