# E16BFORLEGJURCON-003: Add PressForceClaim and YieldForceClaim action payloads

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — action payloads in worldwake-sim
**Deps**: E16BFORLEGJURCON-001, E16BFORLEGJURCON-002

## Problem

The spec requires two new action types (`PressForceClaim`, `YieldForceClaim`) for agents to explicitly enter and exit force contests. The payload variants and action definitions must exist before handlers or AI wiring can be built.

## Assumption Reassessment (2026-03-22)

1. `ActionPayload` enum in `action_payload.rs` contains variants like `DeclareSupport`, `Bribe`, `Threaten`, `Tell`, etc. `PressForceClaim` and `YieldForceClaim` do not exist.
2. Action definitions follow the `ActionDef` pattern in `action_def.rs` with `ActionDomain`, preconditions, and duration. `ActionDomain::Social` exists and is appropriate.
3. Not a planner or golden ticket — pure action-type registration.
4. N/A — not an AI regression.
5. N/A — no ordering dependency.
6. N/A — no heuristic removal.
7. N/A — not a start-failure ticket.
8. N/A — not a political closure ticket.
9. N/A — no ControlSource manipulation.
10. N/A — no golden scenario.
11. No mismatches found.
12. N/A — no cumulative arithmetic.

## Architecture Check

1. Follows the existing `DeclareSupport` action payload pattern exactly: struct payload + enum variant + accessor method + `ActionDef` registration.
2. No backward-compatibility shims. Net-new payload variants.

## Verification Layers

1. `ActionPayload::PressForceClaim` round-trip → focused unit test
2. `ActionPayload::YieldForceClaim` round-trip → focused unit test
3. Action def registration → verified by action_def_registry lookup in test
4. Single-layer ticket (payload types + def registration).

## What to Change

### 1. Define payload structs in `action_payload.rs`

```rust
pub struct PressForceClaimActionPayload {
    pub office: EntityId,
}

pub struct YieldForceClaimActionPayload {
    pub office: EntityId,
}
```

### 2. Add enum variants to `ActionPayload`

Add `PressForceClaim(PressForceClaimActionPayload)` and `YieldForceClaim(YieldForceClaimActionPayload)` variants, with `as_press_force_claim()` and `as_yield_force_claim()` accessor methods following the existing pattern.

### 3. Register action definitions

Add `ActionDef` entries for both actions:
- Domain: `ActionDomain::Social`
- Duration: 1 tick (`NonInterruptible`)
- Preconditions as specified in spec (actor alive, at jurisdiction, eligible, etc.)

Register in `ActionDefRegistry` (in `action_def_registry.rs` or wherever the existing political actions are registered).

## Files to Touch

- `crates/worldwake-sim/src/action_payload.rs` (modify — add structs, variants, accessors)
- `crates/worldwake-sim/src/action_def.rs` or `action_def_registry.rs` (modify — register new action defs)
- `crates/worldwake-sim/src/action_domain.rs` (verify `Social` domain exists — likely no change)

## Out of Scope

- Action handlers (commit effects) — that's E16BFORLEGJURCON-004
- Force control system — E16BFORLEGJURCON-005
- AI affordance enumeration — E16BFORLEGJURCON-007
- Institutional belief variants — E16BFORLEGJURCON-006
- Precondition validation logic (authoritative checks) — E16BFORLEGJURCON-004

## Acceptance Criteria

### Tests That Must Pass

1. `ActionPayload::PressForceClaim` can be constructed and accessed via `as_press_force_claim()`
2. `ActionPayload::YieldForceClaim` can be constructed and accessed via `as_yield_force_claim()`
3. Both action defs are discoverable in `ActionDefRegistry`
4. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Both actions use `ActionDomain::Social` (matching existing political actions)
2. Both actions are 1-tick, non-interruptible
3. Payload structs contain only `office: EntityId` — no redundant controller identity field
4. No existing tests break

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_payload.rs` test module — round-trip and accessor tests for both payloads
2. `crates/worldwake-sim/src/action_def_registry.rs` test module — registry lookup tests

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
