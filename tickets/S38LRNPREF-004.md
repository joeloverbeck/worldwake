# S38LRNPREF-004: Travel experience recording (commit + abort)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — travel action handlers in worldwake-systems
**Deps**: S38LRNPREF-001, S38LRNPREF-002

## Problem

Agents complete or abort travel without recording any experience. After this ticket, `commit_travel` records safe or hostile edge experience, and `abort_travel` records hostile experience when combat caused the abort (P10 — failure is new state).

## Assumption Reassessment (2026-04-02)

1. `commit_travel` at `crates/worldwake-systems/src/travel_actions.rs:202` — signature: `fn commit_travel(_def: &ActionDef, instance: &ActionInstance, _context: &ActionExecutionContext<'_>, _rng: &mut DeterministicRng, txn: &mut WorldTxn<'_>)`.
2. `abort_travel` at `crates/worldwake-systems/src/travel_actions.rs:221` — same signature pattern.
3. `ActionState::Travel { edge_id, origin, destination, departure_tick, arrival_tick }` — provides `edge_id` (TravelEdgeId) and `departure_tick` for combat detection window.
4. `EventLog` at `crates/worldwake-core/src/event_log.rs:8` — indices: `by_tick`, `by_actor`, `by_tag`, `by_place`, `by_cause`.
5. `EventTag::Combat` exists in `crates/worldwake-core/src/event_tag.rs` — confirmed during reassessment.
6. Combat detection requires intersecting `events_by_tag(EventTag::Combat)` with `events_by_actor(agent_id)` and filtering by tick range `[departure_tick, current_tick)`. No existing helper does this intersection — must be implemented.
7. `WorldTxn` provides access to event log and component store for reading/writing `RouteExperience`.
8. `RouteExperience::enforce_limits` from S38LRNPREF-002 must be called after recording.

## Architecture Check

1. Recording in the action handler (commit/abort) is the natural place — the handler already has access to the agent, the action state (edge_id, departure_tick), and the world transaction. No new system needed.
2. Combat detection via event log intersection is the correct approach per the spec — it uses authoritative event history, not global state queries. Consistent with P15 (knowledge acquired locally — the agent experienced the combat).
3. No backward-compatibility shims.

## Verification Layers

1. Safe travel increments `safe_trips` → focused unit test with no combat events in travel window
2. Combat during travel increments `hostile_encounters` → focused unit test with combat event in travel window
3. Combat-aborted travel increments `hostile_encounters` → focused unit test with abort + combat event
4. Non-combat abort does not record → focused unit test with abort + no combat event
5. Eviction called after recording → focused unit test with capacity-limited profile
6. Single-layer ticket (worldwake-systems action handlers); verification via focused tests on authoritative state.

## What to Change

### 1. Combat detection helper

Add a helper function (in `travel_actions.rs` or a shared utility) that checks whether combat events involving a given agent occurred between two ticks:

```rust
fn had_combat_during_travel(
    event_log: &EventLog,
    agent: EntityId,
    start_tick: Tick,
    end_tick: Tick,
) -> bool
```

Intersects `events_by_tag(EventTag::Combat)` with `events_by_actor(agent)`, filters to `[start_tick, end_tick)`.

### 2. Modify `commit_travel`

After existing commit logic:
1. Get agent's `RouteExperience` (or create default if absent).
2. Get `edge_id` and `departure_tick` from `ActionState::Travel`.
3. Call `had_combat_during_travel` for the travel window.
4. Update `EdgeExperience`: increment `safe_trips` or `hostile_encounters`, set `last_travel_tick`.
5. If agent has `PreferenceProfile`, call `enforce_limits`.
6. Write updated `RouteExperience` back to world.

### 3. Modify `abort_travel`

After existing abort logic:
1. Check if abort was due to combat (call `had_combat_during_travel`).
2. If combat: update `RouteExperience` with `hostile_encounters` increment.
3. If not combat: no experience update.
4. If agent has `PreferenceProfile`, call `enforce_limits`.

## Files to Touch

- `crates/worldwake-systems/src/travel_actions.rs` (modify — commit_travel, abort_travel, new combat detection helper)

## Out of Scope

- Harvest/trade recording (S38LRNPREF-005)
- Route cost penalty in ranking (S38LRNPREF-006)
- Golden tests (S38LRNPREF-008)

## Acceptance Criteria

### Tests That Must Pass

1. `commit_travel` with no combat events → `safe_trips` incremented, `hostile_encounters` unchanged
2. `commit_travel` with combat event in travel window → `hostile_encounters` incremented, `safe_trips` unchanged
3. `abort_travel` with combat event → `hostile_encounters` incremented
4. `abort_travel` without combat event → no experience update
5. `last_travel_tick` updated to current tick on both commit and combat-abort
6. Eviction called after recording (capacity limit respected)
7. Agent without `RouteExperience` component gets one created on first travel
8. Existing suite: `cargo test --workspace`

### Invariants

1. Experience records only created from actual action outcomes, never from abstract scoring (P3)
2. Combat detection uses event log intersection, not global state query (P15)
3. Binary eviction enforced after every record update

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/travel_actions.rs` (new focused tests) — safe travel recording, hostile travel recording, abort recording, non-combat abort no-op, eviction after recording

### Commands

1. `cargo test -p worldwake-systems travel`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
