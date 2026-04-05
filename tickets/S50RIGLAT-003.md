# S50RIGLAT-003: Belief-facing rights queries on GoalBeliefView

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — trait method addition with default impl
**Deps**: S50RIGLAT-001

## Problem

The AI planner has no way to ask "what rights does this agent believe they hold over this entity?" Justice candidate generation needs this to distinguish lawful enforcement (guard with jurisdictional authority) from unlawful force (theft). This ticket adds `believed_rights()` to `GoalBeliefView` with a concrete implementation on `PerAgentBeliefView`.

## Assumption Reassessment (2026-04-05)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:32`. Already has `believed_owner_of()`, `can_control()`, `factions_of()`, `direct_possessions()`. Verified this session.
2. `PerAgentBeliefView` implements `GoalBeliefView` at `crates/worldwake-sim/src/per_agent_belief_view.rs`. `believed_owner_of()` reads from authoritative state filtered by agent access. Verified.
3. `RightKind` and `EffectiveRight` types from ticket 001 must be available in `worldwake-sim` (which depends on `worldwake-core`). Dependency path confirmed.
4. `can_control(actor, entity) -> bool` already exists on the trait (line 147). The new `believed_rights()` is the richer replacement that returns typed rights.
5. This is a single-layer ticket (sim trait + impl). No AI, planning, or system changes.

## Architecture Check

1. Follows the `believed_owner_of()` pattern exactly: read authoritative state, filter by agent knowledge/access. This is the established pattern for belief-facing queries in the project.
2. Default impl returns empty vec — safe for any `GoalBeliefView` impl that hasn't been updated. No backward-compat issue because adding a default method to a trait is non-breaking.
3. No shims. The new method is a genuine capability addition, not a wrapper around `can_control()`.

## Verification Layers

1. `believed_rights()` returns correct rights for known entities → focused unit test
2. `believed_rights()` returns empty for unknown entities → focused unit test
3. `believed_rights()` is consistent with `can_control()` → unit test asserting `!believed_rights().is_empty()` iff `can_control()` is true
4. Single-layer ticket — no cross-system verification needed

## What to Change

### 1. Add believed_rights() to GoalBeliefView trait

In `crates/worldwake-sim/src/belief_view.rs`:
```rust
fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
    Vec::new() // default: no rights known
}
```

### 2. Implement on PerAgentBeliefView

In `crates/worldwake-sim/src/per_agent_belief_view.rs`:
```rust
fn believed_rights(&self, actor: EntityId, entity: EntityId) -> Vec<EffectiveRight> {
    // Only return rights if agent knows about the entity
    let accessible = self.knows_entity(entity)
        || self.world.owner_of(entity) == Some(self.agent);
    if !accessible {
        return Vec::new();
    }
    // Delegate to authoritative effective_rights()
    self.world.effective_rights(actor, entity)
}
```

### 3. Add focused unit tests

Test that `believed_rights()`:
- Returns rights for entities the agent knows about
- Returns empty vec for entities the agent doesn't know about
- Is consistent with `can_control()` for known entities

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify — add trait method with default)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — add impl)

## Out of Scope

- Changing `can_control()` on the trait (it remains as the boolean convenience)
- Justice candidate generation (ticket 004)
- Golden E2E tests (ticket 004)
- Any changes to `worldwake-ai` or `worldwake-systems`

## Acceptance Criteria

### Tests That Must Pass

1. `test_believed_rights_known_entity` — returns rights for known entity
2. `test_believed_rights_unknown_entity` — returns empty for unknown entity
3. `test_believed_rights_consistent_with_can_control` — agreement with boolean check
4. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `believed_rights(a, e).is_empty() == !can_control(a, e)` for all known entities
2. `believed_rights()` never reads agent beliefs for rights — it uses authoritative state filtered by knowledge access (same pattern as `believed_owner_of`)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (test module) — 3 focused unit tests for belief-facing rights queries

### Commands

1. `cargo test -p worldwake-sim -- believed_rights`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
