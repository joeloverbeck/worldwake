# S174SHESLESUR-004: Sleep handler — RestOccupancy lifecycle + PromotableContentionKind::RestSite

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — sleep action handler writes/releases `RestOccupancy`; sleep action local state records rough-vs-known rest mode; `abort_sleep_episode` threads `SleepFailureCause` through end-sleep path; new variant on crate-private `PromotableContentionKind`; exhaustive-match arm in `contention_target_matches_kind`; rough-sleep recovery floor applied at handler tick path
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
8. Reassessment correction (2026-05-26): the drafted `reservation_requirements` update is not lawful for this slice. `ReservationReq { target_index: 0 }` is unconditional and exclusive, but S174 sleep has two legal shapes: targetless RoughSleep (no rest slot) and KnownRestSite sleep with multi-occupant `RestCapacity`. Generic reservations would reject targetless rough sleep and would over-constrain capacity > 1 sites. This ticket keeps the sleep action registration's reservation requirements empty and makes `RestOccupancy` the authoritative start-time slot gate. S44 queue promotion is still enabled by `PromotableContentionKind::RestSite`, but the queue points at a rest-capable Place rather than a generic reservation held by the sleep action.

## Architecture Check

1. Writing `RestOccupancy` at action start (rather than reserving via planner intent) aligns with FND-8 and FND-21 — intent is not entitlement; occupancy is the concrete claim. A losing actor whose precondition rejected via "rest site full" replans through the standard `agent_tick.rs::handle_plan_failure` path.
2. Reusing S44's contention substrate (`ContentionQueue` via the new `PromotableContentionKind::RestSite` variant) avoids a parallel queue. Per-place `ContentionPolicy` applies uniformly — no per-kind policy routing.
3. The asymmetric WakeReason cause supply path (`abort_sleep_episode` supplies the cause; `WakeCondition::LocalDisturbance` stays bare) preserves the FND-28 single-cause-surface contract approved in spec Q1=(b).

## Verified Layers

1. KnownRestSite sleep start writes `RestOccupancy.occupants` with the actor's `EntityId` -> `known_rest_site_sleep_start_writes_rest_occupancy`
2. KnownRestSite sleep commit removes the actor from `RestOccupancy.occupants` -> `known_rest_site_sleep_commit_releases_rest_occupancy`
3. KnownRestSite sleep abort removes the actor from `RestOccupancy.occupants` and records a structured cause -> `known_rest_site_sleep_abort_releases_rest_occupancy_and_records_cause`
4. RoughSleep sleep start writes no `RestOccupancy` and caps recovery by profile floor -> `rough_sleep_writes_no_rest_occupancy_and_caps_recovery`
5. Capacity-full start fails with `Err(ActionError::PreconditionFailed)` before creating an active instance -> `known_rest_site_sleep_rejects_full_capacity`
6. `PromotableContentionKind::RestSite` classifies sleep actions and matches only Places with `RestCapacity` -> `promotable_contention_kind_classifies_sleep_action_as_rest_site` and `rest_site_contention_kind_matches_only_places_with_rest_capacity`
7. Shared `ActionState::Sleep` serialization remains covered -> `action_state_bincode_roundtrip_covers_every_variant`

## Landed Changes

### 1. Sleep-discriminator carrier

Added `ActionState::Sleep { rough: bool, place: EntityId }` in `crates/worldwake-sim/src/action_state.rs`. `start_sleep_episode` now records this local state after deriving the mode from the bound sleep target: targetless sleep is RoughSleep, while a target equal to the actor's current place with `RestCapacity` is KnownRestSite. Existing exhaustive local-state matches were updated where needed.

### 2. RestOccupancy lifecycle

`start_sleep_episode` writes `RestOccupancy.occupants` only for KnownRestSite sleep. It rejects full rest sites with `ActionError::PreconditionFailed("rest site ... is full")`. `end_sleep_episode` releases occupancy before clearing the `SleepEpisode`, so normal commit, abort, and actor-death abort all pass through the same cleanup seam.

### 3. Structured abort cause

`abort_sleep_episode` maps `DangerNearby` to `SleepFailureCause::HostileProximity`, actor-death aborts to `ActorIncapacitated`, rest-site-full revalidation details to `RestSiteContended`, and leaves truly unclassified aborts as `Generic`.

### 4. Rough-sleep recovery floor

RoughSleep caps the cached `SleepEpisode.recovery_modifier` and the tick path by `MetabolismProfile.rough_sleep_recovery_floor`. KnownRestSite sleep keeps the place's `SleepQualityProfile.recovery_modifier` unchanged.

### 5. Rest-site queue classification

Added `PromotableContentionKind::RestSite`, `(ActionDomain::Needs, "sleep")` classification, and a target matcher that accepts only Places carrying `RestCapacity`.

### 6. Reservation-free sleep action

The sleep action registration keeps `reservation_requirements: Vec::new()`. Rest-site slot admission is enforced by `RestOccupancy` because generic reservations are unconditional and exclusive, while S174 needs targetless RoughSleep and multi-occupant rest sites.

## Landed Files

- `crates/worldwake-sim/src/action_state.rs`
- `crates/worldwake-systems/src/needs_actions.rs`
- `crates/worldwake-systems/src/facility_queue.rs`
- `crates/worldwake-systems/src/travel_actions.rs`
- `crates/worldwake-systems/tests/e09_needs_integration.rs`
- `crates/worldwake-ai/tests/scenarios/sleep_episode.rs`
- `crates/worldwake-cli/src/scenario/mod.rs` (clippy-only cleanup in existing S174 rest-capacity scenario test)

## Out of Scope

- No emitter changes (ticket 005 owns the two-path candidate emission)
- No belief-view accessors (archived `archive/tickets/S174SHESLESUR-003.md`)
- No `FailedRestOpportunity` records (ticket 006)
- No `ActionTraceDetail::SleepInterrupted` population (ticket 006)
- No CLI player-POV gating (ticket 010)
- No new scenario files (tickets 007-011)
- No `WakeCondition::LocalDisturbance` restructuring — stays bare per Q1=(b) and ticket 001's design

## Acceptance Result

### Satisfied Tests

1. `known_rest_site_sleep_start_writes_rest_occupancy`
2. `known_rest_site_sleep_commit_releases_rest_occupancy`
3. `known_rest_site_sleep_abort_releases_rest_occupancy_and_records_cause`
4. `rough_sleep_writes_no_rest_occupancy_and_caps_recovery`
5. `known_rest_site_sleep_rejects_full_capacity`
6. `promotable_contention_kind_classifies_sleep_action_as_rest_site`
7. `rest_site_contention_kind_matches_only_places_with_rest_capacity`
8. `sleep_episode_reduces_fatigue_at_default_place` and `scheduler_driven_care_actions_apply_effects_and_preserve_conservation` now assert targetless rough sleep recovery instead of pre-S174 full-quality sleep.
9. `place_quality_modulates_per_tick_recovery` now targets rest-capable places so it continues proving KnownRestSite quality modifiers.

### Satisfied Invariants

1. `RestOccupancy.occupants` contains an actor iff that actor is currently in a KnownRestSite sleep episode at the parent place
2. `RestOccupancy` is never written for RoughSleep actions
3. `SAVE_FORMAT_VERSION` is not bumped in this ticket (rides ticket 001's bump)
4. Abort cleanup removes `RestOccupancy` membership idempotently — calling it on a rough-sleeping actor (which never inserted) is a no-op

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` — added RestOccupancy lifecycle, full-site rejection, abort-cause, and rough-sleep cap tests.
2. `crates/worldwake-systems/src/facility_queue.rs` — added RestSite classification and target-matching tests.
3. `crates/worldwake-ai/tests/scenarios/sleep_episode.rs` — updated the place-quality golden to use targeted KnownRestSite sleep.
4. `crates/worldwake-systems/tests/e09_needs_integration.rs` — updated scheduler-driven targetless sleep expectation to rough-sleep recovery.
5. `crates/worldwake-sim/src/action_state.rs` — extended bincode roundtrip coverage for `ActionState::Sleep`.

### Verification Commands

1. Passed `cargo test -p worldwake-systems needs_actions`
2. Passed `cargo test -p worldwake-systems facility_queue`
3. Passed `cargo test -p worldwake-sim action_state`
4. Passed `cargo test -p worldwake-ai sleep_episode`
5. Passed `cargo test -p worldwake-systems`
6. Passed `cargo test --workspace`
7. Passed `cargo clippy --workspace --all-targets -- -D warnings`
8. Waived `./scripts/verify.sh` for this ticket iteration because the implement-spec-tickets harness final branch phase owns the full pre-push wrapper; this iteration directly ran its behavioral workspace gate and the CI-shaped all-target clippy gate.

## Outcome

Completed on 2026-05-26.

- Landed the sleep-handler rest-site lifecycle: targeted KnownRestSite sleep occupies and releases `RestOccupancy`; targetless RoughSleep never occupies a rest slot.
- Added `ActionState::Sleep { rough, place }` as the authoritative in-flight sleep mode carrier.
- Applied `rough_sleep_recovery_floor` to rough sleep and updated existing targetless-sleep expectations accordingly.
- Added RestSite queue classification for S44 contention promotion.
- Updated the sleep quality golden to target rest-capable places so it continues proving KnownRestSite quality rather than rough-sleep capping.
- Corrected the drafted reservation requirement: generic `ReservationReq` stayed out of the sleep action because it would be exclusive and unconditional, conflicting with S174's targetless rough sleep and multi-occupant rest capacity.
- Fixed one existing S174 rest-capacity CLI test clippy lint (`manual_let_else`) exposed by the CI-shaped all-target clippy run.

## Deviations

- The drafted combined command `cargo test -p worldwake-systems needs_actions facility_queue` is invalid Cargo syntax; verification used separate `needs_actions` and `facility_queue` runs plus the full `worldwake-systems` and workspace suites.
- The ticket's drafted reservation-registration step was rejected during reassessment for FND-8/FND-21 alignment. `RestOccupancy` is the concrete multi-slot claim; planner intent and generic reservations do not claim rest slots.

## Verification Result

- Passed `cargo test -p worldwake-systems needs_actions`
- Passed `cargo test -p worldwake-systems facility_queue`
- Passed `cargo test -p worldwake-sim action_state`
- Passed `cargo test -p worldwake-ai sleep_episode`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Waived `./scripts/verify.sh` because the final implement-spec-tickets push phase owns the full pre-PR wrapper; this iteration covered the affected behavioral gates plus CI-shaped all-target clippy.
