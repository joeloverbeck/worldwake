# S50RIGLAT-003: Belief-facing rights queries on GoalBeliefView

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — sim belief-view trait method addition with default impl
**Deps**: S50RIGLAT-002

## Problem

The AI planner has no way to ask "what rights does this agent believe they hold over this entity?" Justice candidate generation needs this to distinguish lawful enforcement (guard with jurisdictional authority) from unlawful force (theft). This ticket adds `believed_rights()` to `GoalBeliefView` with a concrete implementation on `PerAgentBeliefView`.

## Assumption Reassessment (2026-04-05)

1. `GoalBeliefView` trait at `crates/worldwake-sim/src/belief_view.rs:32`. Already has `believed_owner_of()`, `can_control()`, `factions_of()`, `direct_possessions()`. Verified this session.
2. `PerAgentBeliefView` implements `GoalBeliefView` at `crates/worldwake-sim/src/per_agent_belief_view.rs`. `believed_owner_of()` reads from authoritative state filtered by agent access. Verified.
3. `RightKind` and `EffectiveRight` types from ticket 001 are already available, and the first live `JurisdictionalAuthority` result now lands in ticket 002. `believed_rights()` must sit on top of that corrected authoritative baseline. Verified this session.
4. `can_control(actor, entity) -> bool` already exists on the trait, but after ticket 002 it is no longer equivalent to “has any right.” `JurisdictionalAuthority` is a live right that does not imply blanket control. `believed_rights()` must mirror the authoritative typed-rights surface instead of restating the old boolean contract.
5. This remains a single-layer ticket (sim trait + impl). No AI, planning, or system changes are required in this slice because later tickets consume the richer belief query.

## Architecture Check

1. Follows the `believed_owner_of()` pattern: read authoritative state, filter by agent knowledge/access to the target entity. This is the established pattern for belief-facing queries in the project.
2. Default impl returns empty vec — safe for any `GoalBeliefView` impl that hasn't been updated. No backward-compat issue because adding a default method to a trait is non-breaking.
3. No shims. The new method is a genuine capability addition, not a wrapper around `can_control()`.

## Verification Layers

1. `believed_rights()` returns correct typed rights for known entities → focused unit test
2. `believed_rights()` returns empty for unknown entities → focused unit test
3. `believed_rights()` can surface `JurisdictionalAuthority` even when `can_control()` is false → focused unit test
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
- Preserves the typed-rights distinction from ticket 002, including jurisdiction without blanket control

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
3. `test_believed_rights_surfaces_jurisdiction_without_control` — typed-rights result can be nonempty while `can_control()` remains false
4. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `believed_rights()` never reads agent beliefs for rights — it uses authoritative state filtered by knowledge access to the target entity (same pattern as `believed_owner_of`)
2. `believed_rights()` may be nonempty even when `can_control()` is false, because `JurisdictionalAuthority` is a live non-control right after ticket 002

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (test module) — 3 focused unit tests for belief-facing rights queries

### Commands

1. `cargo test -p worldwake-sim -- believed_rights`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completed: 2026-04-05
- Added `believed_rights(actor, entity) -> Vec<EffectiveRight>` to the sim belief-view surface in `crates/worldwake-sim/src/belief_view.rs`, including the runtime-to-goal forwarding path.
- Implemented `PerAgentBeliefView::believed_rights()` in `crates/worldwake-sim/src/per_agent_belief_view.rs` by reusing authoritative `effective_rights()` behind the existing target-knowledge gate used for belief-facing ownership reads.
- Added focused sim tests for known-entity rights, unknown-entity hiding, and the post-`S50RIGLAT-002` case where `JurisdictionalAuthority` is visible in `believed_rights()` while `can_control()` remains false.
- Deviation from original plan: corrected the ticket before coding because its old invariant still treated typed rights as boolean-equivalent to `can_control()`. After `S50RIGLAT-002`, that was no longer true once `JurisdictionalAuthority` became a live non-control right.
- Verification:
  - `cargo test -p worldwake-sim believed_rights -- --nocapture`
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
