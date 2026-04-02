# S38LRNPREF-004: Travel experience recording (commit + abort)

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — travel action handlers plus sim action callback boundary
**Deps**: S38LRNPREF-001, S38LRNPREF-002

## Problem

Agents complete or abort travel without recording any experience. After this ticket, `commit_travel` records safe or hostile edge experience, and `abort_travel` records hostile experience when combat caused the abort (P10 — failure is new state). Live reassessment shows this also requires widening the action callback boundary so commit/abort handlers can lawfully read the authoritative event log for the travel window.

## Assumption Reassessment (2026-04-02)

1. `commit_travel` at `crates/worldwake-systems/src/travel_actions.rs` currently receives `ActionExecutionContext` but not the authoritative `EventLog`.
2. `abort_travel` currently receives neither `ActionExecutionContext` nor `EventLog`, so it cannot inspect the current tick or prior causal record.
3. `ActionState::Travel { edge_id, origin, destination, departure_tick, arrival_tick }` — provides `edge_id` (TravelEdgeId) and `departure_tick` for combat detection window.
4. `EventLog` at `crates/worldwake-core/src/event_log.rs` exposes indexed authoritative history (`events_by_actor`, `events_by_tag`, `get`).
5. `EventTag::Combat` exists in `crates/worldwake-core/src/event_tag.rs` — confirmed during reassessment.
6. `EventRecord` exposes `actor_id()` and `target_ids()`, so “combat during travel” can be derived authoritatively by scanning combat-tagged records in the travel window and matching the traveler as either actor or target.
7. `WorldTxn` does not provide read access to prior events. The authoritative event log currently lives on `ActionExecutionAuthority`, outside the handler callback signatures.
8. `RouteExperience::enforce_limits` from S38LRNPREF-002 must be called after recording.

## Architecture Check

1. Recording in the action handler (commit/abort) is still the natural place — the handler owns the action outcome, the travel local state, and the component write.
2. To keep the event log as the single authoritative source of “combat happened during this travel leg,” this ticket must widen the sim action callback boundary rather than inventing a second travel-local flag path. This aligns with P3 and P12.
3. Commit and abort handlers should receive read-only `EventLog` access, and abort handlers should also receive `ActionExecutionContext`, so they can inspect the authoritative travel window without mutating the causal record directly.
4. No backward-compatibility shims.

## Verification Layers

1. Sim callback boundary exposes read-only authoritative event-log access to commit/abort handlers → focused unit tests in `worldwake-sim`
2. Safe travel increments `safe_trips` → focused unit test with no combat events in travel window
3. Combat during travel increments `hostile_encounters` → focused unit test with combat event in travel window
4. Combat-aborted travel increments `hostile_encounters` → focused unit test with interrupt + combat event
5. Non-combat abort does not record → focused unit test with abort + no combat event
6. Eviction called after recording → focused unit test with capacity-limited profile
7. Mixed-layer ticket (`worldwake-sim` callback boundary + `worldwake-systems` travel handlers); verify both layers.

## What to Change

### 1. Widen action callback boundary

Update the sim action callback surface so commit and abort handlers can read the authoritative event log:

- extend the action commit callback signature with a read-only `&EventLog`
- extend the action abort callback signature with both `&ActionExecutionContext` and read-only `&EventLog`
- thread those values through the live call sites in `tick_action.rs` / `interrupt_abort.rs` / related tests

### 2. Combat detection helper

Add a helper function (in `travel_actions.rs` or a shared utility) that checks whether combat events involving a given agent occurred between two ticks:

```rust
fn had_combat_during_travel(
    event_log: &EventLog,
    agent: EntityId,
    start_tick: Tick,
    end_tick: Tick,
) -> bool
```

Scans combat-tagged records in `[start_tick, end_tick)` and treats the traveler as involved when it appears either as `actor_id()` or in `target_ids()`.

### 3. Modify `commit_travel`

After existing commit logic:
1. Get agent's `RouteExperience` (or create default if absent).
2. Get `edge_id` and `departure_tick` from `ActionState::Travel`.
3. Call `had_combat_during_travel` for the travel window.
4. Update `EdgeExperience`: increment `safe_trips` or `hostile_encounters`, set `last_travel_tick`.
5. If agent has `PreferenceProfile`, call `enforce_limits`.
6. Write updated `RouteExperience` back to world.

### 4. Modify `abort_travel`

After existing abort logic:
1. Check if abort was due to combat (call `had_combat_during_travel`).
2. If combat: update `RouteExperience` with `hostile_encounters` increment.
3. If not combat: no experience update.
4. If agent has `PreferenceProfile`, call `enforce_limits`.

## Files to Touch

- `crates/worldwake-sim/src/action_handler.rs` (modify — commit/abort callback signatures)
- `crates/worldwake-sim/src/action_termination.rs` (modify — abort callback threading)
- `crates/worldwake-sim/src/tick_action.rs` (modify — commit/abort callback threading)
- `crates/worldwake-sim/src/interrupt_abort.rs` (modify — abort callback threading)
- `crates/worldwake-systems/src/travel_actions.rs` (modify — commit_travel, abort_travel, new combat detection helper, focused tests)

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
8. Sim callback tests prove commit/abort handlers can read the authoritative event log through the live runtime boundary
8. Existing suite: `cargo test --workspace`

### Invariants

1. Experience records only created from actual action outcomes, never from abstract scoring (P3)
2. Combat detection uses the authoritative event log, not a duplicated travel-local flag path (P12, P15)
3. Binary eviction enforced after every record update

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_action.rs` or `interrupt_abort.rs` (focused tests) — commit/abort callback surface carries read-only event-log access
2. `crates/worldwake-systems/src/travel_actions.rs` (new focused tests) — safe travel recording, hostile travel recording, abort recording, non-combat abort no-op, eviction after recording

### Commands

1. `cargo test -p worldwake-sim tick_action -- --nocapture`
2. `cargo test -p worldwake-sim interrupt_abort -- --nocapture`
3. `cargo test -p worldwake-systems travel -- --nocapture`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

- Completed: 2026-04-02
- What changed:
  - widened the sim action callback boundary so commit handlers receive read-only `&EventLog` and abort handlers receive both `&ActionExecutionContext` and read-only `&EventLog`
  - threaded that boundary through the live sim termination/tick/interrupt call paths and updated affected direct handler test harnesses in dependent crates
  - implemented authoritative combat-window detection in `travel_actions.rs` and recorded `RouteExperience` on safe travel commit, hostile travel commit, and hostile travel abort
  - enforced `PreferenceProfile` route-memory limits immediately after each recorded travel experience update
- Deviations from original plan:
  - the original ticket understated the required scope as a `worldwake-systems`-only change; during reassessment it was corrected to a mixed-layer ticket because `travel_actions` could not lawfully read the event log through the old callback boundary
  - combat detection was refined from actor-only event intersection to authoritative traveler involvement via either `actor_id()` or `target_ids()`
  - dependent AI and systems test harnesses that directly register or invoke handlers also needed factual signature updates after the callback widening
- Verification results:
  - `cargo test -p worldwake-sim tick_action -- --nocapture`
  - `cargo test -p worldwake-sim interrupt_abort -- --nocapture`
  - `cargo test -p worldwake-systems travel -- --nocapture`
  - `cargo test -p worldwake-sim`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
