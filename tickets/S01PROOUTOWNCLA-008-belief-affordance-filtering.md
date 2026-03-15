# S01PROOUTOWNCLA-008: Ownership-aware belief-based affordance filtering for pick_up

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — affordance query, belief-based precondition evaluation
**Deps**: S01PROOUTOWNCLA-006 (believed_owner_of), S01PROOUTOWNCLA-003 (extended can_exercise_control)

## Problem

The AI planner generates `pick_up` affordances for all visible unpossessed lots. With ownership, agents must not plan to pick up lots they believe are owned by others and cannot lawfully control. The affordance filter must use `believed_owner_of()` and belief-based control checks to exclude illegal pickups from the planner's action space.

## Assumption Reassessment (2026-03-15)

1. `get_affordances()` at `affordance_query.rs:8-54` queries available actions — confirmed
2. `evaluate_precondition()` evaluates action preconditions against belief view — confirmed
3. `RuntimeBeliefView::can_control()` exists at `belief_view.rs:167` — confirmed
4. `believed_owner_of()` will be added by S01PROOUTOWNCLA-006 — confirmed
5. `pick_up` preconditions are defined in `transport_actions.rs:48-56` — confirmed

## Architecture Check

1. The belief-based filter mirrors the authoritative check: if believed owner exists, check belief-based control
2. If agent has no belief about ownership (hasn't perceived the lot or its ownership), the lot should still appear as a potential affordance — the authoritative check will catch invalid attempts
3. This follows Principle 10 (belief-only planning): planner never reads world state directly

## What to Change

### 1. Update belief-based precondition evaluation for `pick_up`

In `affordance_query.rs`, when evaluating `pick_up` affordances for a target lot:

```rust
// After checking TargetUnpossessed belief:
if let Some(believed_owner) = belief_view.believed_owner_of(target) {
    // Agent believes this lot has an owner — check if they can control it
    if !belief_view.can_control(actor, target) {
        // Agent believes they cannot lawfully pick this up — filter it out
        continue; // or return false from precondition check
    }
}
// No believed owner or agent can control → affordance is available
```

### 2. Extend belief-based `can_control()` for institutional delegation

The belief-based `can_control()` in `PerAgentBeliefView` must mirror the extended `can_exercise_control()` from S01PROOUTOWNCLA-003. It should check:
1. Direct ownership belief
2. Faction membership belief (agent believes they're in the owning faction)
3. Office holding belief (agent believes they hold the owning office)

This may require adding `believed_factions_of()` and `believed_offices_held_by()` if they don't already exist, or using existing belief store relation queries.

### 3. Update `pick_up` action definition preconditions (if needed)

If the affordance system uses `ActionDef` preconditions for filtering, add a new precondition variant (e.g., `TargetUnownedOrActorControls(target_index)`) and implement its belief-based evaluation.

Alternatively, if the filtering is done inline in `get_affordances()`, handle it there.

## Files to Touch

- `crates/worldwake-sim/src/affordance_query.rs` (modify — add ownership filter)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — extend `can_control()` for institutional delegation if needed)
- `crates/worldwake-sim/src/belief_view.rs` (modify — if new trait methods needed for institutional belief queries)
- `crates/worldwake-systems/src/transport_actions.rs` (modify — if ActionDef precondition list needs update)

## Out of Scope

- Authoritative validation changes (S01PROOUTOWNCLA-007 — already handled)
- Theft planning (E17 — adds explicit unauthorized acquisition as a separate affordance)
- Trade affordance changes (trade already has its own negotiation semantics)
- Planner search changes in worldwake-ai (the planner naturally sees filtered affordances)

## Acceptance Criteria

### Tests That Must Pass

1. Affordance filtering excludes pickup of lots the agent believes are owned by others they cannot control
2. Affordance filtering includes pickup of lots the agent believes are unowned
3. Affordance filtering includes pickup of lots the agent believes are owned by themselves
4. Affordance filtering includes pickup of faction-owned lots when agent believes they're a faction member
5. Affordance filtering includes pickup of office-owned lots when agent believes they hold the office
6. Lots with no ownership belief are included (agent hasn't perceived ownership — let authoritative check handle it)
7. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Belief-only planning — no world state reads in affordance filtering
2. Planner action space correctly reflects lawful actions only
3. Institutional delegation in belief matches authoritative delegation logic
4. Agents without ownership beliefs can still attempt pickup (authoritative validation is the safety net)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/affordance_query.rs` test module — ownership-aware affordance filtering tests
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` test module — institutional delegation in belief-based control

### Commands

1. `cargo test -p worldwake-sim affordance`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
