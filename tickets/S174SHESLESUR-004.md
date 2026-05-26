# S174SHESLESUR-004: Sleep handler — RestOccupancy lifecycle + PromotableContentionKind::RestSite

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — sleep action handler writes/releases `RestOccupancy`; `abort_sleep_episode` threads `SleepFailureCause` through end-sleep path; new variant on crate-private `PromotableContentionKind`; exhaustive-match arm in `contention_target_matches_kind`; rough-sleep recovery floor applied at handler tick path
**Deps**: `archive/tickets/S174SHESLESUR-001.md` (RestOccupancy component, SleepFailureCause enum), `archive/tickets/S174SHESLESUR-002.md` (MetabolismProfile.rough_sleep_recovery_floor)

## Problem

S174 D2 requires the sleep action handler to write `RestOccupancy.occupants` at action start for KnownRestSite candidates and release on commit / abort / actor death / place departure. Currently `start_sleep_episode` (`needs_actions.rs:481-532`) creates a `SleepEpisode` but writes no occupancy state, so multi-agent contention on shelter capacity is unenforced. Additionally, S44 contention substrate must classify rest sites for queue promotion via a new `PromotableContentionKind::RestSite` variant.

## Assumption Reassessment (2026-05-26)

1. Verified current code: `start_sleep_episode` at `crates/worldwake-systems/src/needs_actions.rs:481-487` returns `Result<Option<ActionState>, ActionError>` per the standard action-handler contract; precondition failures flow through `Err(ActionError)`. `abort_sleep_episode` at lines 667-682 currently calls `end_sleep_episode(actor, current_tick, Some(WakeReason::LocalDisturbance), txn)` — line 679 is the construction site updated by ticket 001 to supply `SleepFailureCause::Generic`. `end_sleep_episode` at lines 684-714 emits the `SleepEpisodeEnded` event. `PromotableContentionKind` enum at `crates/worldwake-systems/src/facility_queue.rs:29` is crate-private with 5 variants; `promotable_contention_kind` at line 465 maps `(ActionDomain, action_name)` to variants; `contention_target_matches_kind` at lines 485-516 is the exhaustive match site requiring a new arm.
2. Spec assumption verified against S174 D2. The reservation_requirements vec on the sleep action registration is currently `Vec::new()` (`needs_actions.rs:77-87`, default of the `register_def` helper).
3. Shared abstraction boundary under audit: action lifecycle (start writes occupancy, commit/abort/death releases) + S44 contention substrate (promotion classification). Both must agree on what "sleep is contended on a Place's rest slot" means; the per-Place `RestCapacity` value is the authoritative slot count.
4. The placeholder-replace pattern applies: ticket 001 introduced `WakeReason::LocalDisturbance { cause: SleepFailureCause::Generic }` as the transitional default at `needs_actions.rs:679`. This ticket refines that path — when `abort_sleep_episode` is invoked due to a known abort reason (incapacitation, surface-invalidated by place-departure, etc.), the handler supplies the matching `SleepFailureCause` variant; only truly unclassifiable aborts fall through to `Generic`. The HostileProximity cause (used in scenario C / ticket 009) flows from the local-disturbance trigger, not from the abort path — that wiring lands in ticket 006 (forensics).
5. KnownRestSite vs RoughSleep discriminator: the handler needs to know which path the candidate emitter chose. Per S174 spec Open Question #1 (resolved per recommended default in spec text), rough sleep is currently allowed at places with `RestCapacity`; the handler distinguishes by reading whether the emitter wrote a "rough sleep" marker on the action's ActionState. Implementation choice — extend `ActionState` to carry a `RoughSleep { ... }` variant for sleep actions, or thread a flag through the action payload. The cleaner path is a new `ActionState::Sleep { rough: bool, place: EntityId }` variant; this is decided at ticket-implementation time and documented in What to Change step 1.
6. Existing inline tests exercising sleep handler behavior: `crates/worldwake-ai/tests/scenarios/sleep_episode.rs:170, 229, 284, 309, 361` (5 golden tests). The `interrupted_sleep_records_partial_recovery` golden at line 284 will need updates if it depends on the `WakeReason::LocalDisturbance` shape (ticket 001 already addresses this). The `sleep_episode_at_default_place_runs_to_intended_max` golden at line 170 uses places without `RestCapacity` — those continue working as rough-sleep paths (RestOccupancy never written). The `site_preference_adopts_higher_quality_sleep_place` golden at line 309 may need a `RestCapacity` annotation on the target place if the test intends to exercise the KnownRestSite path; otherwise it falls through to rough-sleep.
7. Mismatch + correction: the placeholder at `needs_actions.rs:679` (set by ticket 001 to `SleepFailureCause::Generic`) is refined here. List `actor_incapacitated`, `surface_invalidated_by_departure`, and `rest_site_contended` cases when their triggering conditions are known; preserve `Generic` only for truly unclassified aborts.

## Architecture Check

1. Writing `RestOccupancy` at action start (rather than reserving via planner intent) aligns with FND-8 and FND-21 — intent is not entitlement; occupancy is the concrete claim. A losing actor whose precondition rejected via "rest site full" replans through the standard `agent_tick.rs::handle_plan_failure` path.
2. Reusing S44's contention substrate (`ContentionQueue` via the new `PromotableContentionKind::RestSite` variant) avoids a parallel queue. Per-place `ContentionPolicy` applies uniformly — no per-kind policy routing.
3. The asymmetric WakeReason cause supply path (`abort_sleep_episode` supplies the cause; `WakeCondition::LocalDisturbance` stays bare) preserves the FND-28 single-cause-surface contract approved in spec Q1=(b).

## Verification Layers

1. KnownRestSite sleep start writes `RestOccupancy.occupants` with the actor's EntityId -> integration test exercising `start_sleep_episode` + ECS component query
2. KnownRestSite sleep commit removes the actor from `RestOccupancy.occupants` -> integration test exercising `end_sleep_episode` (commit branch)
3. KnownRestSite sleep abort removes the actor from `RestOccupancy.occupants` -> integration test exercising `abort_sleep_episode`
4. RoughSleep sleep start writes NO `RestOccupancy` -> integration test, negative branch
5. Capacity-full precondition fails sleep start with `Err(ActionError)` -> action-trace assertion (the ActionAborted event with precondition rejection)
6. `PromotableContentionKind::RestSite` arm matches when target is a Place with `RestCapacity` -> focused unit test on `contention_target_matches_kind`
7. Authoritative mutation ordering: `RestOccupancy` writes are tick-aligned at action start (matches S173's `SelfCareOccupancy` pattern) -> event-log delta assertion

## What to Change

### 1. Extend `ActionState` (or equivalent) with a sleep-discriminator carrier

Sleep actions currently use `ActionPayload::None` (`needs_actions.rs:109` for the `sleep` action registration). To distinguish KnownRestSite from RoughSleep at the handler, add a new `ActionState::Sleep { rough: bool, place: EntityId }` variant in `crates/worldwake-sim/src/action_state.rs` (or wherever `ActionState` lives — locate via `grep -rn "pub enum ActionState" crates/`). The emitter (ticket 005) writes this variant via the planner; the handler reads `rough` to decide whether to write `RestOccupancy`. Update existing exhaustive matches on `ActionState` to add the new variant (typically `ActionState::Travel { ... }` is the precedent).

Likely alternative if `ActionState` lives elsewhere: thread the flag through a sleep-specific ActionPayload variant. The exact mechanism is decided at ticket-implementation time per the codebase's current pattern; the requirement is that the handler can read "rough vs known" from authoritative action state.

### 2. Update `start_sleep_episode` to write `RestOccupancy` for KnownRestSite candidates

In `crates/worldwake-systems/src/needs_actions.rs:481-532`, extend the handler:

- Read the `rough` flag from the action's state (per step 1).
- If `rough == false` AND the target place has `RestCapacity`: check `RestOccupancy.occupants.len() < capacity`. If full, return `Err(ActionError)` with a precondition-rejection reason. Otherwise insert the actor into `RestOccupancy.occupants` (creating the component if absent, initializing to empty).
- If `rough == true` OR the target place has no `RestCapacity`: skip the `RestOccupancy` write entirely. Rough sleep does not consume a rest slot.

### 3. Update `end_sleep_episode` and `abort_sleep_episode` to release `RestOccupancy`

In `needs_actions.rs:684-714` (`end_sleep_episode`) and the existing place-departure / actor-incapacitation cleanup paths, remove the actor from `RestOccupancy.occupants` if present. The release is idempotent — calling it for a rough-sleep actor (which never inserted) is a no-op. If the resulting `RestOccupancy.occupants` is empty, the component may be removed or left as an empty set (both are equivalent under the start-time precondition `len() < capacity`).

In `abort_sleep_episode` at lines 667-682, refine the `SleepFailureCause` value supplied to `end_sleep_episode`. The existing ticket-001 placeholder `SleepFailureCause::Generic` becomes a fallback; when the abort reason is known (`AbortReason::Incapacitated` → `SleepFailureCause::ActorIncapacitated`, `AbortReason::SurfaceInvalidated` → `SleepFailureCause::SurfaceInvalidated`, etc.), supply the specific cause. Verify the `AbortReason` enum's current variants and map each meaningfully.

### 4. Apply `rough_sleep_recovery_floor` at the handler tick path

Locate the sleep-tick recovery accumulation path (likely a sleep_tick function adjacent to `start_sleep_episode`). For sleep actions where `rough == true`, cap the per-tick `SleepRecoveryModifier` at `metabolism_profile.rough_sleep_recovery_floor`. For KnownRestSite sleep, use the place's `SleepQualityProfile.recovery_modifier` unchanged.

### 5. Add `PromotableContentionKind::RestSite` variant

In `crates/worldwake-systems/src/facility_queue.rs:29`, add a new variant:

```rust
enum PromotableContentionKind {
    FacilityExclusive(WorkstationTag),
    Corpse,
    Care,
    SelfCareWash,
    SelfCareLatrine,
    RestSite,             // new
}
```

In `promotable_contention_kind` at line 465, add a match arm:

```rust
(ActionDomain::Needs, "sleep") => Some(PromotableContentionKind::RestSite),
```

In `contention_target_matches_kind` at lines 485-516, add a new arm for `RestSite` that matches when the target is a `Place` carrying `RestCapacity`. The exhaustive match must compile after this addition — verify by running the test suite.

### 6. Update sleep action registration

In `needs_actions.rs:77-87`, the sleep action's `reservation_requirements` is currently `Vec::new()` via `register_def`'s default. Update the registration to carry a non-empty reservation_requirements entry indicating that the target rest-site facility must be reservable (no current `RestOccupancy` full state) for the action to start. Exact reservation_requirements shape depends on the existing pattern for `wash` and `toilet` (which were extended in S173); mirror that precedent.

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify — `start_sleep_episode`, `end_sleep_episode`, `abort_sleep_episode`, sleep action registration, sleep-tick recovery cap)
- `crates/worldwake-systems/src/facility_queue.rs` (modify — `PromotableContentionKind::RestSite` variant, `promotable_contention_kind` arm, `contention_target_matches_kind` arm)
- Likely: `crates/worldwake-sim/src/action_state.rs` (modify — `ActionState::Sleep` variant or equivalent); locate via `grep -rn "pub enum ActionState" crates/`
- Likely: existing exhaustive-match sites on `ActionState` — locate via `grep -rn "match.*ActionState" crates/` and update each arm
- Likely: existing inline tests in `needs_actions.rs` (verify via `grep -n "#\[cfg(test)\]" crates/worldwake-systems/src/needs_actions.rs` — boundary at line 1179; check for sleep-handler tests above the boundary)
- Likely: existing tests in `facility_queue.rs` (verify via inline-test boundary; ensure `contention_target_matches_kind` tests cover the new `RestSite` arm)
- `crates/worldwake-ai/tests/scenarios/sleep_episode.rs:170, 229, 284, 309, 361` (verify each golden — update assertions where `RestCapacity` annotation is needed to exercise the KnownRestSite path)

## Out of Scope

- No emitter changes (ticket 005 owns the two-path candidate emission)
- No belief-view accessors (archived `archive/tickets/S174SHESLESUR-003.md`)
- No `FailedRestOpportunity` records (ticket 006)
- No `ActionTraceDetail::SleepInterrupted` population (ticket 006)
- No CLI player-POV gating (ticket 010)
- No new scenario files (tickets 007-011)
- No `WakeCondition::LocalDisturbance` restructuring — stays bare per Q1=(b) and ticket 001's design

## Acceptance Criteria

### Tests That Must Pass

1. New integration test: KnownRestSite sleep at a `RestCapacity(NonZeroU32::new(1))` place writes the actor into `RestOccupancy.occupants`
2. New integration test: KnownRestSite sleep commit removes the actor from `RestOccupancy.occupants`
3. New integration test: KnownRestSite sleep abort (via `abort_sleep_episode`) removes the actor from `RestOccupancy.occupants` and supplies the correct `SleepFailureCause`
4. New integration test: RoughSleep at the same place writes NO `RestOccupancy`
5. New integration test: KnownRestSite sleep start at a capacity-full place returns `Err(ActionError)`
6. New focused unit test: `promotable_contention_kind((ActionDomain::Needs, "sleep"))` returns `Some(PromotableContentionKind::RestSite)`
7. New focused unit test: `contention_target_matches_kind` matches a Place-with-`RestCapacity` target for `RestSite`
8. New integration test: rough-sleep recovery per tick is capped at `MetabolismProfile.rough_sleep_recovery_floor` regardless of `SleepQualityProfile.recovery_modifier`
9. Existing suite: `cargo test -p worldwake-systems needs_actions facility_queue` passes
10. Existing suite: `cargo test -p worldwake-ai sleep_episode` passes (with updated assertions per Files to Touch)

### Invariants

1. `RestOccupancy.occupants` contains an actor iff that actor is currently in a KnownRestSite sleep episode at the parent place
2. `RestOccupancy` is never written for RoughSleep actions
3. `SAVE_FORMAT_VERSION` is not bumped in this ticket (rides ticket 001's bump)
4. Abort cleanup removes `RestOccupancy` membership idempotently — calling it on a rough-sleeping actor (which never inserted) is a no-op

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` (extend inline `#[cfg(test)]`) — RestOccupancy lifecycle for sleep
2. `crates/worldwake-systems/src/facility_queue.rs` (extend inline `#[cfg(test)]`) — `RestSite` variant + `contention_target_matches_kind` arm
3. `crates/worldwake-ai/tests/scenarios/sleep_episode.rs` (modify existing 5 goldens as needed for RestCapacity-aware assertions)

### Commands

1. `cargo test -p worldwake-systems needs_actions facility_queue` (handler + contention coverage)
2. `cargo test -p worldwake-ai sleep_episode` (golden updates)
3. `cargo test --workspace` (full regression)
4. `./scripts/verify.sh` (final pre-PR gate)
