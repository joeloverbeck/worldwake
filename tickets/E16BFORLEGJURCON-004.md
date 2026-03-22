# E16BFORLEGJURCON-004: Implement PressForceClaim and YieldForceClaim action handlers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — action handlers in worldwake-systems, action validation
**Deps**: E16BFORLEGJURCON-001, E16BFORLEGJURCON-002, E16BFORLEGJURCON-003

## Problem

The `PressForceClaim` and `YieldForceClaim` actions need commit-time handlers that mutate world state: adding/removing `contests_office` relations, creating hostility against incumbents, and emitting visible political events with `InstitutionalClaim::ForceControl` metadata for perception projection.

## Assumption Reassessment (2026-03-22)

1. `office_actions.rs` in `worldwake-systems` contains handlers for `bribe`, `threaten`, `declare_support`. No force-claim handlers exist.
2. Action validation follows the `action_validation.rs` pattern in `worldwake-sim`. Precondition checks for political actions exist (e.g., `validate_declare_support`).
3. Event emission with `InstitutionalClaim` metadata follows the pattern in `declare_support` handler: emits `EventTag::Political` with `VisibilitySpec::SamePlace` and attaches `InstitutionalClaim` for perception extraction.
4. N/A — not an AI regression ticket.
5. N/A — no ordering dependency.
6. N/A — no heuristic removal.
7. N/A — not a start-failure ticket.
8. Political closure boundary: `PressForceClaim` commit produces `contests_office` relation mutation + hostility mutation + visible event. `YieldForceClaim` commit produces `contests_office` removal + visible event. Both closures are authoritative-layer only.
9. N/A — no ControlSource manipulation.
10. N/A — no golden scenario.
11. No mismatches found.
12. N/A — no cumulative arithmetic.

## Architecture Check

1. Follows the `declare_support` handler pattern exactly. Event emission with `InstitutionalClaim` metadata reuses the existing perception pipeline. Hostility creation on claim-press reuses `txn.add_hostility()`.
2. No backward-compatibility shims. Net-new handlers.

## Verification Layers

1. `PressForceClaim` commit adds `contests_office` → focused test on authoritative world state after commit
2. `PressForceClaim` against incumbent adds `hostile_to` → focused test on relation state
3. `PressForceClaim` emits `EventTag::Political` with `VisibilitySpec::SamePlace` → focused test on event log
4. `PressForceClaim` emits `InstitutionalClaim::ForceControl` metadata → focused test on event metadata
5. `YieldForceClaim` removes `contests_office` → focused test on world state
6. `YieldForceClaim` emits visible political event → focused test on event log
7. Precondition rejection (wrong place, not eligible, already contesting, not alive) → focused validation tests

## What to Change

### 1. Add handlers in `office_actions.rs`

**PressForceClaim handler** (on commit):
- `txn.add_force_claim(actor, office)`
- If office has a recognized holder who is not the actor: `txn.add_hostility(actor, holder)`
- Emit `InstitutionalClaim::ForceControl { office, controller: Some(actor), contested: false, effective_tick }` as event metadata
- Emit visible `Political` event at jurisdiction with `VisibilitySpec::SamePlace`

**YieldForceClaim handler** (on commit):
- `txn.remove_force_claim(actor, office)`
- Emit `InstitutionalClaim::ForceControl` metadata (controller unchanged, claim withdrawn)
- Emit visible `Political` event at jurisdiction with `VisibilitySpec::SamePlace`

### 2. Add precondition validation

**PressForceClaim preconditions**:
- Actor is alive
- Actor is at the office jurisdiction
- Office uses `SuccessionLaw::Force`
- Actor is eligible under office rules
- Actor does not already contest this office

**YieldForceClaim preconditions**:
- `contests_office(actor, office)` exists
- Actor is at the office jurisdiction

### 3. Register handlers

Register both handlers in the action handler registry, following the existing political action pattern.

## Files to Touch

- `crates/worldwake-systems/src/office_actions.rs` (modify — add handlers and action defs)
- `crates/worldwake-sim/src/action_validation.rs` (modify — add precondition validation functions)
- `crates/worldwake-sim/src/action_handler_registry.rs` (modify — register handlers)

## Out of Scope

- Force control system (per-tick state machine) — that's E16BFORLEGJURCON-005
- Removing `resolve_force_succession` — E16BFORLEGJURCON-005
- AI affordance enumeration — E16BFORLEGJURCON-007
- Institutional belief query methods — E16BFORLEGJURCON-006
- Record integration (appending to office register) — E16BFORLEGJURCON-005 (part of installation)
- Tell propagation of force-control beliefs — E16BFORLEGJURCON-006

## Acceptance Criteria

### Tests That Must Pass

1. Pressing a force claim at the jurisdiction succeeds and creates `contests_office` relation
2. Pressing a force claim when not at the jurisdiction fails precondition
3. Pressing a force claim when not eligible fails precondition
4. Pressing a force claim when already contesting fails precondition
5. Pressing a force claim against an incumbent creates `hostile_to(actor, holder)`
6. Force claim event is `EventTag::Political` with `VisibilitySpec::SamePlace`
7. Force claim event carries `InstitutionalClaim::ForceControl` metadata
8. Yielding a claim removes `contests_office` relation
9. Yielding a claim when not contesting fails precondition
10. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Force claims are explicit world actions — never inferred from proximity alone
2. Pressing a claim against an incumbent always creates hostility (persistent aftermath, Principle 9)
3. All force-claim events are locally visible at the jurisdiction (Principle 7)
4. No existing tests break

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/office_actions.rs` test module — handler commit tests for both actions
2. `crates/worldwake-sim/src/action_validation.rs` test module — precondition validation tests

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
