# E19GUAPAT-003: Implement patrol action definition, payload, and handler

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action definition, handler, payload variant
**Deps**: E19GUAPAT-001 (PatrolRoute, PatrolProfile components), E19GUAPAT-002 (GoalKind::Patrol, EventTag::Patrol)

## Problem

The patrol action (dwell phase at a waypoint) needs to be registered in the action framework so guards can execute it. The action advances `PatrolRoute.current_index` on commit and emits a `Patrolled` event tag.

## Assumption Reassessment (2026-03-30)

1. Action registration follows the pattern in `crates/worldwake-systems/src/action_registry.rs` (lines 21–47): a `register_X_action(defs, handlers)` function called from `register_all_actions()`.
2. Action handler pattern (e.g., `investigate_actions.rs`): `start_X()`, `tick_X()`, `commit_X()`, `abort_X()` functions, plus `register_X_action()` and `X_action_def()`.
3. `ActionPayload` in `crates/worldwake-sim/src/action_payload.rs` currently has 29 variants. Patrol needs a payload to carry the waypoint entity ID.
4. `ActionDomain::Generic` is specified by the spec (line 72). Already exists — no new domain needed.
5. `VisibilitySpec::SamePlace` is the spec's required visibility (line 98).
6. `ActionDef` struct (from `action_def.rs`) requires: name, domain, constraints, preconditions, duration, body_cost, interruptibility, visibility, event_tags, payload, handler.
7. Duration: dwell ticks derived from `PatrolProfile.vigilance`. Spec says `dwell_ticks = base_dwell + (vigilance.value() * dwell_scale / 1000)`. Both `base_dwell` and `dwell_scale` should come from `PatrolProfile` or be defined as profile fields.
8. Commit behavior: advance `current_index` to `(current_index + 1) % assigned_places.len()` via `WorldTxn`.
9. Abort behavior: do NOT advance `current_index` (guard resumes from same waypoint).
10. `Interruptibility` — action must be interruptible (spec line 89–91).
11. No adjacent contradictions found.

## Architecture Check

1. Following the existing action module pattern (`investigate_actions.rs`, `justice_actions.rs`) keeps the codebase uniform. A new `patrol_actions.rs` file in `worldwake-systems` is cleaner than adding to an existing file because patrol is a distinct action domain.
2. Patrol payload is minimal (just the waypoint EntityId) — follows the lightweight payload pattern of `InvestigateActionPayload`.
3. No backwards-compatibility shims.

## Verification Layers

1. Action registration → compilation + registry lookup test
2. Patrol commit advances current_index → focused unit test checking index before/after commit
3. Patrol abort preserves current_index → focused unit test checking index unchanged after abort
4. Patrol emits EventTag::Patrol → action trace or event log assertion
5. Patrol duration scales with vigilance → focused unit test with different PatrolProfile values
6. Precondition enforcement (actor needs PatrolRoute + PatrolProfile) → focused unit test

## What to Change

### 1. New file: `crates/worldwake-systems/src/patrol_actions.rs`

Implement:
- `PatrolActionPayload { waypoint: EntityId }` (or add to action_payload.rs)
- `register_patrol_action(defs, handlers) -> ActionDefId`
- `patrol_action_def() -> ActionDef`
- `start_patrol()` — validate actor is at waypoint, has PatrolRoute/PatrolProfile
- `tick_patrol()` — dwell countdown (standard duration-based)
- `commit_patrol()` — advance `current_index`, emit Patrolled event
- `abort_patrol()` — no index change, no event

### 2. Add payload variant in `crates/worldwake-sim/src/action_payload.rs`

```rust
Patrol(PatrolActionPayload),
```

### 3. Register in `crates/worldwake-systems/src/action_registry.rs`

Add `register_patrol_action(defs, handlers);` call in `register_all_actions()`.

### 4. Declare module in `crates/worldwake-systems/src/lib.rs`

Add `pub mod patrol_actions;` and any necessary re-exports.

### 5. WorldTxn integration for `current_index` mutation

The commit handler must mutate `PatrolRoute.current_index` through `WorldTxn`. Follow existing patterns for component mutation in commit handlers (e.g., how `needs_actions` modifies `HomeostaticNeeds`).

## Files to Touch

- `crates/worldwake-systems/src/patrol_actions.rs` (new)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register patrol action)
- `crates/worldwake-systems/src/lib.rs` (modify — declare module)
- `crates/worldwake-sim/src/action_payload.rs` (modify — add Patrol variant)

## Out of Scope

- Patrol candidate generation / AI goal selection (E19GUAPAT-004)
- Route adaptation logic (E19GUAPAT-006)
- Guard presence factor in public_order() (E19GUAPAT-005)
- Golden E2E tests (E19GUAPAT-007)
- Travel-to-waypoint action (already exists as generic Travel action)
- Patrol system tick function (patrol uses the standard action framework, not a per-tick system)

## Acceptance Criteria

### Tests That Must Pass

1. Patrol action registered and retrievable from ActionDefRegistry by name `"patrol"`
2. Patrol commit advances `current_index` from 0 to 1 on a 3-waypoint route
3. Patrol commit wraps `current_index` from 2 to 0 on a 3-waypoint route (modular wrap)
4. Patrol abort leaves `current_index` unchanged
5. Patrol action rejected when actor lacks PatrolRoute component
6. Patrol action rejected when actor lacks PatrolProfile component
7. Patrol dwell duration is longer with higher vigilance than lower vigilance
8. Patrol commit emits event with EventTag::Patrol
9. Existing suite: `cargo test -p worldwake-systems`
10. `cargo clippy --workspace`

### Invariants

1. `current_index` mutation goes through `WorldTxn` (transactional mutation invariant)
2. Patrol action uses `ActionDomain::Generic` (not a new domain)
3. Action is interruptible (spec requirement)
4. No `HashMap` or `f32`/`f64` introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/patrol_actions.rs` — focused unit tests for commit/abort/precondition/duration behavior
2. May also add tests in a `tests/` integration test file if action framework requires simulation harness

### Commands

1. `cargo test -p worldwake-systems -- patrol`
2. `cargo clippy --workspace && cargo test --workspace`
