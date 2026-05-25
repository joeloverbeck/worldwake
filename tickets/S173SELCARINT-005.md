# S173SELCARINT-005: Atomic-action interruption traces (eat, drink, wilderness, sleep enrichment)

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new `abort_emit_self_care_interrupted` handler replacing `abort_noop` for eat/drink/relieve_wilderness; `abort_sleep_episode` extended to populate trace detail
**Deps**: S173SELCARINT-002 (uses `ActionTraceDetail::SelfCareInterrupted`), `specs/S173-self-care-interruption-occupancy.md` (D2 eat/drink/wilderness/sleep rows)

## Problem

Eat, Drink, and Wilderness-Relief actions abort with `abort_noop` today (`crates/worldwake-systems/src/needs_actions.rs:27, 33, 57`) — interruption fires the generic `EventTag::ActionAborted` engine record but leaves no typed trace payload. Sleep aborts via `abort_sleep_episode` (L552-567) which does meaningful state cleanup (`end_sleep_episode` with `WakeReason::LocalDisturbance`) but does not populate `ActionTraceDetail::SelfCareInterrupted`. As a result, downstream observers and goldens cannot distinguish "wash was interrupted" from "eat was interrupted" without parsing the action def name from the trace. This ticket replaces `abort_noop` for the three atomic actions with `abort_emit_self_care_interrupted` (state-no-op, trace-payload only) and extends `abort_sleep_episode` to populate the same trace detail, giving Scenario A in ticket 007 a uniform discrimination surface across all five self-care families.

## Assumption Reassessment (2026-05-25)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing tests in `crates/worldwake-systems/src/needs_actions.rs` (#[cfg(test)] from L1058) for the four actions touched here:
   - Eat: `eat_consumes_one_unit_and_applies_consumable_effects:1373`, `aborted_eat_does_not_consume_item:1448` (verifies abort no-op state behavior — must still hold), `eat_accepts_actor_owned_ground_lot:2125`, `eat_accepts_possessed_lot:2146`, `uncontrolled_ground_item_does_not_produce_eat_affordance:2091`
   - Drink: `drink_consumes_one_unit_and_applies_consumable_effects:1410`, `drink_accepts_actor_owned_ground_lot:2167`, `drink_accepts_possessed_lot:2188`
   - Sleep: `sleep_episode_reduces_fatigue_at_default_place:1504`, `sleep_wake_reason_uses_first_matching_condition:1623`, `sleep_wake_reason_maps_projected_need_and_scheduled_commitment:1648`
   - Relieve-wilderness: `relieve_wilderness_accepts_outdoor_places:2337`, `relieve_wilderness_rejects_indoor_places:2393`, `relieve_wilderness_commit_effects:2474`, `relieve_wilderness_visibility_is_same_place:2542`, `relieve_wilderness_has_wilderness_relief_event_tag:2549`, `relieve_wilderness_place_dirtiness_saturates:2563`, `relieve_wilderness_commit_emits_scene_evidence:2605`
   - All must continue to pass; new abort-trace-detail behavior is additive (existing tests do not assert on `ActionTraceEvent.detail`).
2. `aborted_eat_does_not_consume_item:1448` is the most likely test to need attention — it asserts that an aborted eat does not consume the item. The new `abort_emit_self_care_interrupted` handler must preserve this contract (no state mutation, only trace-detail population). Verify the test still passes with the new handler.
3. `abort_sleep_episode` (L552-567) currently calls `end_sleep_episode` (L569-599) and returns. The extension adds a single step: after `end_sleep_episode` completes (preserving the existing `WakeReason::LocalDisturbance` contract), populate `ActionTraceEvent.detail = Some(ActionTraceDetail::SelfCareInterrupted { kind: SelfCareUseKind::WildernessRelief, basin: None })` — wait, this is wrong; sleep uses `SelfCareUseKind` and the variant for sleep is... not in the minimal enum. Per ticket 001, `SelfCareUseKind` carries 5 variants: `Wash, LatrineRelief, Eat, Drink, WildernessRelief`. Sleep is not a `SelfCareUseKind` variant — sleep already has `EventTag::SleepEpisodeEnded` as its specialized causal record.
   - **Resolution**: the spec D2 sleep row says "Existing `EventTag::SleepEpisodeEnded`; new trace detail enriches the abort/interrupt path". Re-read: the new trace detail does NOT require a `SelfCareUseKind::Sleep` variant. Either (a) extend `SelfCareUseKind` with a `Sleep` variant in this ticket (or as a back-edit to ticket 001), or (b) sleep abort populates a different trace surface than `SelfCareInterrupted`. Option (a) is cleaner per the spec's D2 note option (i) (single discriminator across all five families). Update ticket 001's enum to 6 variants: `Wash, LatrineRelief, Eat, Drink, WildernessRelief, Sleep` — surface this as an assumption-reassessment correction at /implement-ticket time for ticket 001. Track in this ticket's Files to Touch as a coordinated edit.
4. The trace-sink emission API from inside an abort handler — same boundary as ticket 004. Verify how `ActionExecutionContext` exposes the action-trace sink at implementation time.
5. Shared abstraction boundary: action abort handlers and the action-trace sink. The four atomic actions emit the same typed trace detail with different `kind` discriminators; the abort state effect remains `Ok(())` (eat/drink/wilderness) or unchanged (sleep retains `end_sleep_episode`).

## Architecture Check

1. Two new handlers (one new + one extended) collapse the five-family abort-trace surface into a uniform shape. Per FND-29 debuggability, "Why didn't this agent X?" is answerable from `EventTag::ActionAborted` filtered by action name PLUS the typed trace detail. No parallel surface.
2. Per FND-28, no shim retained: the previous `abort_noop` registrations for eat/drink/wilderness are replaced, not aliased. The `abort_noop` symbol itself remains in scope (still registered for non-self-care actions per spec Non-Goals); only the registrations change.
3. Sleep retains its existing `SleepEpisode` durable contract — this ticket only layers the trace detail on top.

## Verification Layers

1. Atomic abort state preservation → focused unit test on each of eat/drink/wilderness: assert post-abort state is unchanged (no item consumed for eat; no bladder cleared for wilderness; etc.). This is the existing-test invariant.
2. Trace-detail population → focused unit test: instantiate an action of each kind, fire abort, assert `ActionTraceEvent.detail` carries `Some(ActionTraceDetail::SelfCareInterrupted { kind: <correct kind>, basin: None })` (for eat/drink/wilderness) or `kind: Sleep, basin: None` (for sleep — adapt per ticket 001 enum update).
3. Sleep dual surface preservation → focused unit test: assert `EventTag::SleepEpisodeEnded` still fires AND `ActionTraceDetail::SelfCareInterrupted` is populated. Both surfaces fire on the same abort tick.
4. Cross-system trace mapping → per `docs/precision-rules.md` Rule 5: action-trace `detail` field carries the typed payload; event-log carries `ActionAborted` (and `SleepEpisodeEnded` for sleep). Two layers, distinct proof surfaces.

## What to Change

### 1. Coordinated update to ticket 001's `SelfCareUseKind` enum

When this ticket is implemented before ticket 001 ships, raise a back-edit request to ticket 001's `SelfCareUseKind` enum to add a sixth variant: `Sleep`. If ticket 001 has already shipped, this ticket adds the variant as a small enum extension (single-file change in `crates/worldwake-core/src/self_care_occupancy.rs`, plus updates to the `SelfCareUseKind` derive surface if anything depends on variant count).

**Placeholder note**: ticket 001's enum is shipped with 5 variants; this ticket's implementation may add the 6th (`Sleep`) when sleep-trace integration lands. The 6-variant final shape is the contract; the 5-variant intermediate is the staged version. Replaced by ticket 005's implementation if the back-edit is not folded into ticket 001 at land time.

### 2. Implement `abort_emit_self_care_interrupted` handler

Add a new handler in `needs_actions.rs` following the `abort_noop` signature pattern (L354-364) but with trace-detail population:

```rust
fn abort_emit_self_care_interrupted(
    def: &ActionDef,
    _instance: &ActionInstance,
    context: &worldwake_sim::ActionExecutionContext<'_>,
    _reason: &AbortReason,
    _event_log: &worldwake_core::EventLog,
    _rng: &mut DeterministicRng,
    _txn: &mut WorldTxn<'_>,
) -> Result<(), ActionError> {
    // Derive SelfCareUseKind from the action def name:
    //   "eat" → SelfCareUseKind::Eat
    //   "drink" → SelfCareUseKind::Drink
    //   "relieve_wilderness" → SelfCareUseKind::WildernessRelief
    // Populate ActionTraceEvent.detail = Some(ActionTraceDetail::SelfCareInterrupted { kind, basin: None }).
    // Return Ok(()).
    Ok(())
}
```

The kind-derivation can be a small inline match or a helper function. Avoid threading the kind through `ActionDef` metadata — the def name is already unique and stable.

### 3. Wire `abort_emit_self_care_interrupted` into eat, drink, relieve_wilderness registrations

In `needs_actions.rs:23-58`, replace `abort_noop` with `abort_emit_self_care_interrupted` on the eat (L27), drink (L33), and relieve_wilderness (L57) `ActionHandler::new(...)` calls. Wash and toilet (L51, L45) remain `abort_release_self_care_occupancy` from ticket 004.

### 4. Extend `abort_sleep_episode` to populate the new trace detail

In `needs_actions.rs:552-567`, after the existing `end_sleep_episode(...)` call completes, populate `ActionTraceEvent.detail = Some(ActionTraceDetail::SelfCareInterrupted { kind: SelfCareUseKind::Sleep, basin: None })`. The `WakeReason::LocalDisturbance` contract and `EventTag::SleepEpisodeEnded` firing are preserved unchanged — the trace detail is additive.

### 5. Update `register_needs_actions_adds_all_six_defs_and_handlers` test

The registration test at L1299 asserts handler identity. After this ticket lands, the expected handler set is:
- eat → `abort_emit_self_care_interrupted`
- drink → `abort_emit_self_care_interrupted`
- sleep → `abort_sleep_episode` (unchanged identity; extended behavior)
- toilet → `abort_release_self_care_occupancy` (from ticket 004)
- wash → `abort_release_self_care_occupancy` (from ticket 004)
- relieve_wilderness → `abort_emit_self_care_interrupted`

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify — new handler + 3 registration changes + sleep abort extension + registration test update)
- `crates/worldwake-core/src/self_care_occupancy.rs` (modify — add `Sleep` variant to `SelfCareUseKind` if not folded into ticket 001's land)

## Out of Scope

- Occupancy state writes/removes — eat/drink/wilderness never write occupancy; sleep uses `SleepEpisode` not `SelfCareOccupancy`. Owned by ticket 004 for wash/toilet only.
- New `EventTag` variant — explicitly rejected; `EventTag::ActionAborted` and `EventTag::SleepEpisodeEnded` reused.
- Candidate-emitter filtering — owned by ticket 006.
- Scenario goldens — owned by ticket 007 (Scenario A exercises this ticket's behavior most directly).
- Per-kind discriminator alternative (sibling `SelfCareTraceKind` enum) — explicitly rejected per spec D2 note option (i).

## Acceptance Criteria

### Tests That Must Pass

1. New unit test: `aborted_eat_populates_self_care_interrupted_trace_detail` — existing state contract holds (no item consumed) AND `ActionTraceEvent.detail = Some(ActionTraceDetail::SelfCareInterrupted { kind: Eat, basin: None })`.
2. Symmetric tests for drink (`kind: Drink`), relieve_wilderness (`kind: WildernessRelief`), and sleep (`kind: Sleep`).
3. New unit test: `aborted_sleep_preserves_existing_wake_reason_and_emits_self_care_interrupted` — `EventTag::SleepEpisodeEnded` fires with `WakeReason::LocalDisturbance` AND `ActionTraceEvent.detail` carries the new typed payload.
4. Existing test `aborted_eat_does_not_consume_item:1448` still passes (the new handler is state-no-op).
5. Existing tests `sleep_episode_reduces_fatigue_at_default_place`, `sleep_wake_reason_uses_first_matching_condition`, `sleep_wake_reason_maps_projected_need_and_scheduled_commitment` — all sleep-related behavior preserved.
6. Existing tests for relieve_wilderness commit/affordance behavior — all preserved.
7. `register_needs_actions_adds_all_six_defs_and_handlers` updated per the new handler identity map.

### Invariants

1. Atomic-action abort state remains a no-op (no item consumed for eat/drink; no bladder cleared for wilderness pre-commit; no occupancy written).
2. Sleep abort preserves `SleepEpisode` lifecycle: `accumulated_recovery` rolls into `HomeostaticNeeds::fatigue` via existing `end_sleep_episode` machinery; `EventTag::SleepEpisodeEnded` fires unchanged.
3. All five atomic abort surfaces (`Eat`, `Drink`, `Sleep`, `WildernessRelief`, plus the occupancy-bearing `Wash` and `LatrineRelief` from ticket 004) populate `ActionTraceDetail::SelfCareInterrupted` with the correct `kind`. The trace sink is the single typed discrimination surface (FND-29).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` inline tests (existing `#[cfg(test)]` from L1058) — 4 new tests (one per non-occupancy family) plus the registration-test update.

### Commands

1. `cargo test -p worldwake-systems needs_actions`
2. `cargo build --workspace -- -D warnings`
3. `./scripts/verify.sh` before commit.
