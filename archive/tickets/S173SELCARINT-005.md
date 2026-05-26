# S173SELCARINT-005: Atomic-action interruption traces (eat, drink, wilderness, sleep enrichment)

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — new `abort_emit_self_care_interrupted` handler replacing `abort_noop` for eat/drink/relieve_wilderness; `SelfCareUseKind::Sleep`; `tick_step` abort-trace mapping for all six self-care families
**Deps**: `archive/tickets/S173SELCARINT-002.md` (uses `ActionTraceDetail::SelfCareInterrupted`), `archive/specs/S173-self-care-interruption-occupancy.md` (D2 eat/drink/wilderness/sleep rows)

## Problem

Before this ticket, Eat, Drink, and Wilderness-Relief actions registered `abort_noop` (`crates/worldwake-systems/src/needs_actions.rs`) and `tick_step` only mapped Wash/Toilet abort traces to `ActionTraceDetail::SelfCareInterrupted`. Sleep retained its durable `SleepEpisode` cleanup, but its abort trace also had no self-care discriminator. As a result, downstream observers and goldens could not distinguish all self-care abort families through one typed trace payload. This ticket gives Scenario A in ticket 007 a uniform discrimination surface across Eat, Drink, Sleep, LatrineRelief, WildernessRelief, and Wash while preserving the existing `EventTag::ActionAborted` causal record.

## Assumption Reassessment (2026-05-26)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Existing tests in `crates/worldwake-systems/src/needs_actions.rs` (#[cfg(test)] from L1058) for the four actions touched here:
   - Eat: `eat_consumes_one_unit_and_applies_consumable_effects:1373`, `aborted_eat_does_not_consume_item:1448` (verifies abort no-op state behavior — must still hold), `eat_accepts_actor_owned_ground_lot:2125`, `eat_accepts_possessed_lot:2146`, `uncontrolled_ground_item_does_not_produce_eat_affordance:2091`
   - Drink: `drink_consumes_one_unit_and_applies_consumable_effects:1410`, `drink_accepts_actor_owned_ground_lot:2167`, `drink_accepts_possessed_lot:2188`
   - Sleep: `sleep_episode_reduces_fatigue_at_default_place:1504`, `sleep_wake_reason_uses_first_matching_condition:1623`, `sleep_wake_reason_maps_projected_need_and_scheduled_commitment:1648`
   - Relieve-wilderness: `relieve_wilderness_accepts_outdoor_places:2337`, `relieve_wilderness_rejects_indoor_places:2393`, `relieve_wilderness_commit_effects:2474`, `relieve_wilderness_visibility_is_same_place:2542`, `relieve_wilderness_has_wilderness_relief_event_tag:2549`, `relieve_wilderness_place_dirtiness_saturates:2563`, `relieve_wilderness_commit_emits_scene_evidence:2605`
   - All must continue to pass; new abort-trace-detail behavior is additive (existing tests do not assert on `ActionTraceEvent.detail`).
2. `aborted_eat_does_not_consume_item` is the strongest existing atomic-abort state-preservation proof. The new `abort_emit_self_care_interrupted` handler must remain a state no-op, and the full `needs_actions` module test run verifies that contract.
3. `ActionExecutionContext` does not expose `ActionTraceSink`, and ticket 004 already landed Wash/Toilet abort detail through `crates/worldwake-sim/src/tick_step.rs::abort_trace_detail_for_instance`. The live trace-emission boundary is therefore `tick_step`, not `abort_sleep_episode` or a handler-local trace write.
4. Before this ticket, `SelfCareUseKind` carried `Wash`, `LatrineRelief`, `Eat`, `Drink`, and `WildernessRelief`; this ticket adds `Sleep` so all six self-care abort families share one discriminator. `Sleep` is used only as trace detail, not as `SelfCareOccupancy` state.
5. Shared abstraction boundary: the action abort handler controls state cleanup; the `tick_step` action-trace emission helper controls typed abort-trace detail. The landed design keeps those responsibilities separate and avoids adding a parallel causal event.

## Architecture Check

1. One new state-no-op handler plus the existing `tick_step` abort detail mapper collapse the six-family abort-trace surface into a uniform shape. Per FND-29 debuggability, "Why didn't this agent X?" is answerable from `EventTag::ActionAborted` filtered by action name plus the typed trace detail. No parallel causal surface.
2. Per FND-28, no shim retained: the previous `abort_noop` registrations for eat/drink/wilderness are replaced, not aliased. The `abort_noop` symbol itself remains in scope (still registered for non-self-care actions per spec Non-Goals); only the registrations change.
3. Sleep retains its existing `SleepEpisode` durable contract — this ticket only layers trace detail on top.

## Verified Layers

1. Atomic abort state preservation → focused unit test on each of eat/drink/wilderness: assert post-abort state is unchanged (no item consumed for eat; no bladder cleared for wilderness; etc.). This is the existing-test invariant.
2. Trace-detail population → focused `tick_step` unit test: instantiate an action of each self-care kind and assert `abort_trace_detail_for_instance` returns `ActionTraceDetail::SelfCareInterrupted { kind, basin }`.
3. Sleep durable surface preservation → existing `needs_actions` sleep tests prove `SleepEpisode` behavior still passes; the focused `tick_step` test proves the added `Sleep` trace discriminator.
4. Cross-system trace mapping → per `docs/precision-rules.md` Rule 5: action-trace `detail` field carries the typed payload; event-log carries `ActionAborted` (and `SleepEpisodeEnded` for sleep). Two layers, distinct proof surfaces.

## Landed Changes

### 1. Coordinated update to `SelfCareUseKind`

Added `SelfCareUseKind::Sleep` in `crates/worldwake-core/src/self_care_occupancy.rs`. The enum now carries all trace discriminators while `SelfCareOccupancy` still only uses the occupancy-bearing variants.

### 2. Implement `abort_emit_self_care_interrupted` handler

Added `abort_emit_self_care_interrupted` in `needs_actions.rs`. It intentionally has no state effect; typed trace detail is derived by `tick_step` at emission time from the action definition name and active instance targets.

### 3. Wire `abort_emit_self_care_interrupted` into eat, drink, relieve_wilderness registrations

Replaced `abort_noop` with `abort_emit_self_care_interrupted` on eat, drink, and relieve_wilderness registrations. Wash and toilet remain `abort_release_self_care_occupancy` from ticket 004.

### 4. Extend `tick_step` abort trace detail

Extended `tick_step.rs::abort_trace_detail_for_instance` so eat, drink, sleep, wash, toilet, and relieve_wilderness abort traces all produce `ActionTraceDetail::SelfCareInterrupted` with the correct `kind` and `basin`. The `WakeReason::LocalDisturbance` contract and `EventTag::SleepEpisodeEnded` firing remain unchanged.

### 5. Update `register_needs_actions_adds_all_six_defs_and_handlers` test

The registration test at L1299 asserts handler identity. After this ticket lands, the expected handler set is:
- eat → `abort_emit_self_care_interrupted`
- drink → `abort_emit_self_care_interrupted`
- sleep → `abort_sleep_episode` (unchanged identity; extended behavior)
- toilet → `abort_release_self_care_occupancy` (from ticket 004)
- wash → `abort_release_self_care_occupancy` (from ticket 004)
- relieve_wilderness → `abort_emit_self_care_interrupted`

## Landed Files

- `crates/worldwake-core/src/self_care_occupancy.rs` — added `SelfCareUseKind::Sleep`.
- `crates/worldwake-sim/src/tick_step.rs` — mapped all self-care abort families to typed trace detail and broadened the focused unit test.
- `crates/worldwake-systems/src/needs_actions.rs` — replaced eat/drink/wilderness abort registrations, added the state-no-op handler, updated registration proof, and folded a same-file clippy cleanup.
- `archive/specs/S173-self-care-interruption-occupancy.md` — truth-synced the trace emission boundary.

## Out of Scope

- Occupancy state writes/removes — eat/drink/wilderness never write occupancy; sleep uses `SleepEpisode` not `SelfCareOccupancy`. Owned by ticket 004 for wash/toilet only.
- New `EventTag` variant — explicitly rejected; `EventTag::ActionAborted` and `EventTag::SleepEpisodeEnded` reused.
- Candidate-emitter filtering — owned by ticket 006.
- Scenario goldens — owned by ticket 007 (Scenario A exercises this ticket's behavior most directly).
- Per-kind discriminator alternative (sibling `SelfCareTraceKind` enum) — explicitly rejected per spec D2 note option (i).

## Acceptance Result

### Verified Behavior

1. `tick_step::tests::abort_trace_detail_for_self_care_actions_uses_action_family_and_target` verifies Eat, Drink, Sleep, Wash, LatrineRelief, and WildernessRelief trace discriminators.
2. `needs_actions` module tests verify atomic state preservation, SleepEpisode preservation, relieve_wilderness behavior, Wash/Toilet occupancy release, and registration identity.
3. `self_care_occupancy` focused core tests verify the widened enum still satisfies component and bincode contracts.
4. `cargo clippy --workspace --all-targets -- -D warnings` passed after a same-file registration-match cleanup.

### Invariants

1. Atomic-action abort state remains a no-op (no item consumed for eat/drink; no bladder cleared for wilderness pre-commit; no occupancy written).
2. Sleep abort preserves `SleepEpisode` lifecycle: `accumulated_recovery` rolls into `HomeostaticNeeds::fatigue` via existing `end_sleep_episode` machinery; `EventTag::SleepEpisodeEnded` fires unchanged.
3. All six self-care abort surfaces (`Eat`, `Drink`, `Sleep`, `WildernessRelief`, `Wash`, and `LatrineRelief`) populate `ActionTraceDetail::SelfCareInterrupted` with the correct `kind`. The trace sink is the single typed discrimination surface (FND-29).

## Test Plan Result

### Modified Tests

1. `crates/worldwake-sim/src/tick_step.rs` — broadened the existing self-care abort-detail unit test to cover all six self-care families.
2. `crates/worldwake-systems/src/needs_actions.rs` — updated the registration test to assert eat/drink/wilderness use the named atomic abort hook.

### Commands Run

1. Passed `cargo test -p worldwake-sim --lib tick_step::tests::abort_trace_detail_for_self_care_actions_uses_action_family_and_target -- --exact`.
2. Passed `cargo test -p worldwake-core --lib self_care_occupancy -- --nocapture`.
3. Passed `cargo test -p worldwake-systems --lib needs_actions -- --nocapture`.
4. Passed `cargo clippy --workspace --all-targets -- -D warnings`.
5. Waived `./scripts/verify.sh` at this ticket iteration because the `implement-spec-tickets` final branch phase owns the full pre-push gate; the CI-shaped all-target clippy gate and focused tests passed here.

## Outcome

Completed on 2026-05-26.

- Added `SelfCareUseKind::Sleep` and trace-side abort-detail mapping for Eat, Drink, Sleep, Wash, LatrineRelief, and WildernessRelief.
- Replaced the eat/drink/relieve_wilderness `abort_noop` registrations with the named `abort_emit_self_care_interrupted` state-no-op handler.
- Preserved the existing SleepEpisode abort lifecycle and the existing authoritative `EventTag::ActionAborted` record; no new event tag or occupancy state was introduced.

## Deviations

- The drafted handler-local trace write was corrected to the live `tick_step.rs::abort_trace_detail_for_instance` boundary because `ActionExecutionContext` does not carry the action-trace sink.

## Verification Result

- Passed `cargo test -p worldwake-sim --lib tick_step::tests::abort_trace_detail_for_self_care_actions_uses_action_family_and_target -- --exact`.
- Passed `cargo test -p worldwake-core --lib self_care_occupancy -- --nocapture`.
- Passed `cargo test -p worldwake-systems --lib needs_actions -- --nocapture`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Waived `./scripts/verify.sh` for this iteration; the full harness finalization still owns the pre-push wrapper gate.
