# S174SHESLESUR-009: Scenario C — survival-rest-interrupted-by-danger.ron (HostileProximity wake cause)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — golden scenario plus narrow simulator/AI fixes required to make hostile co-location interrupt active Sleep with a structured HostileProximity cause.
**Deps**: `archive/tickets/S174SHESLESUR-001.md`, `archive/tickets/S174SHESLESUR-002.md`, `archive/tickets/S174SHESLESUR-003.md`, `archive/tickets/S174SHESLESUR-004.md`, `archive/tickets/S174SHESLESUR-005.md`, `archive/tickets/S174SHESLESUR-006.md`

## Outcome

Scenario C landed as `scenarios/survival-rest-interrupted-by-danger.ron` plus the golden module `crates/worldwake-ai/tests/scenarios/survival_rest_interrupted_by_danger.rs`. The scenario proves that a hostile actor arriving at a sleeping agent's rest site interrupts the active Sleep episode with `WakeReason::LocalDisturbance { cause: SleepFailureCause::HostileProximity }`, a matching `ActionTraceDetail::SleepInterrupted`, preserved partial recovery, `RestOccupancy` release, forensic failed-rest recording, and deterministic replay.

The ticket was reassessed from a golden-only task to a narrow production fix. The live code already mapped `InterruptReason::DangerNearby` to `SleepFailureCause::HostileProximity`, but no authoritative bridge interrupted active Sleep when a live hostile became co-located with the sleeper. Candidate generation also continued emitting current-place Sleep candidates after local hostile observation, which contradicted Scenario C's "does not immediately re-attempt Sleep at the hostile-occupied shelter" invariant.

## Assumption Reassessment (2026-05-26)

1. Verified current code state: hostile-proximity sleep interruption is triggered by the existing interrupt/abort substrate when a hostile actor enters the sleeper's place. Per S173's `SelfCareOccupancy` and S174's `RestOccupancy` semantics, the sleep handler's abort path is invoked via `abort_sleep_episode` (`needs_actions.rs:667-682`). `archive/tickets/S174SHESLESUR-004.md` refined the cause mapping so the hostile-proximity branch supplies `SleepFailureCause::HostileProximity` rather than the transitional `Generic`.
2. Spec assumption verified against S174 Scenario C. The scenario uses one place (`shelter` with `RestCapacity(1)`) and an adjacent `outpost` that hosts a hostile agent. The hostile travels toward `shelter` mid-sleep. Assertions: sleep aborts mid-episode; `WakeReason::LocalDisturbance { cause: HostileProximity }` fires; `ActionTraceDetail::SleepInterrupted` populates; `RestOccupancy` releases; partial recovery preserved; agent's next tick emits a different goal.
3. Shared abstraction boundary under audit: the interrupt/abort path's classification of hostile-proximity events. Verify `archive/tickets/S174SHESLESUR-004.md` wires the abort handler to read the abort reason and supply the correct `SleepFailureCause`.
4. Live `GoalKind` under test: starts with `GoalKind::Sleep`; after interruption, the agent replans (likely to `GoalKind::Flee` or `GoalKind::Engage` depending on combat profile). The scenario doesn't strictly require the post-replan branch to be deterministic — only that the agent does NOT immediately re-attempt Sleep at the same hostile-occupied place.
5. Cumulative arithmetic: the sleep accumulates ~10-20 ticks of recovery before the hostile arrives (depending on edge travel_time and seed). The partial-recovery assertion checks `accumulated_recovery > 0 && accumulated_recovery < target_recovery`.
6. Scenario isolation: the intended branch under test is `SleepFailureCause::HostileProximity` wake cause routing + partial recovery preservation. Excluded: starvation/dehydration depletion (agent must be near-sated for non-fatigue needs); other hostiles (only one hostile actor).
7. Hostile actor scenario authoring: this requires a non-trivial agent with `CombatProfile` + a hostile relationship toward `Aster`. Existing scenarios with hostile actors include `survival-combat.ron` and `survival-theft.ron` — use those as templates for the hostile-agent authoring shape.
8. Scope correction: live code did not actually interrupt active Sleep when a hostile actor became co-located. `archive/tickets/S174SHESLESUR-004.md` mapped `InterruptReason::DangerNearby` to `SleepFailureCause::HostileProximity`, but no authoritative local-disturbance bridge produced that interrupt for active Sleep. Scenario C therefore cannot be golden-only.
9. Scope correction: once a hostile is locally visible, the Sleep candidate emitter still emitted a current-place known-rest-site and rough-sleep candidate. Scenario C's "does not immediately re-attempt Sleep at the hostile-occupied shelter" invariant requires local-hostile suppression for current-place Sleep candidates.

## Architecture Result

1. The HostileProximity cause maps to a specific abort-handler branch (`archive/tickets/S174SHESLESUR-004.md`'s refinement of `abort_sleep_episode`'s `SleepFailureCause` supply). Per FND-28, the cause taxonomy is a single structured surface — not an ad-hoc string description threaded through the abort path.
2. Partial recovery preservation is a S128 contract — `SleepEpisode.accumulated_recovery` survives the abort and persists into `HomeostaticNeeds.fatigue`. This scenario exercises that contract under the new structured-cause abort path.
3. Asserting on the action trace AND event log (`EventTag::SleepEpisodeEnded` payload) AND `CriticalWindowFrame.failed_rest_opportunities` ensures the proof is layer-strong rather than narrative-only.

## Landed Changes

1. Added the `survival-rest-interrupted-by-danger` scenario with a roofed soft Shelter rest site, adjacent Outpost, hostile Aster/Marauder relationship, Aster's fatigue pressure, and stable authored profiles.
2. Added golden coverage that requests Marauder travel and attack through ordinary external action requests, then asserts the hostile arrival interrupts Aster's active targeted Sleep. The scenario does not depend on AI remote pursuit; the hostile movement/attack trigger is explicit so the wake-cause contract remains isolated.
3. Added an authoritative tick-step bridge that interrupts active Sleep with `InterruptReason::DangerNearby` when a live hostile target is co-located with the sleeper.
4. Suppressed current-place known-rest-site and rough-sleep candidates while a local hostile is visible, while leaving remote rest-site opportunities eligible.
5. Preserved `InterruptReason::DangerNearby` for AI active-action interrupts when a danger-ranked challenger is the reason an active goal is preempted.

## Layer Proofs

1. Sleep starts at `Shelter` by tick 2; `RestOccupancy` admits Aster -> action trace + authoritative world state
2. Marauder starts travel at tick 0, arrives after Sleep starts, and starts attack by the interruption tick -> action trace + authoritative world state
3. Active Sleep aborts with `ActionTraceDetail::SleepInterrupted { place: Shelter, cause: HostileProximity, accumulated_recovery, was_rough_sleep: false }` -> action trace assertion
4. `SleepEpisodeEndedPayload.end_reason` is `WakeReason::LocalDisturbance { cause: HostileProximity }` and carries the same accumulated recovery -> event-log assertion
5. Partial recovery reduces final fatigue from the authored starting value and remains below full recovery -> event-log payload assertion
6. Shelter `RestOccupancy` no longer contains Aster after abort -> authoritative world state assertion
7. `FailedRestKind::Interrupted { cause: HostileProximity }` is recorded for the active fatigue critical window -> survival forensic assertion
8. Aster's post-interrupt planning trace does not emit another Sleep candidate targeting the hostile-occupied Shelter and selects a non-Sleep goal -> decision trace assertion
9. Deterministic replay compares identical observation structs including the event-log hash -> golden replay assertion

## Landed Files

- `scenarios/survival-rest-interrupted-by-danger.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_rest_interrupted_by_danger.rs` (new)
- `crates/worldwake-ai/tests/scenarios/mod.rs`
- `crates/worldwake-sim/src/tick_step.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/agent_tick/active_action.rs`

## Out of Scope

- Other wake-cause variants (`RestSiteContended`, `SurfaceInvalidated`, `ActorIncapacitated`) — scope creep; these are exercised by other variants or future scenarios
- Predator ecology (S61 territory)
- Combat scenario design (existing `survival-combat.ron` covers that; this scenario uses the existing combat substrate as a trigger, not as its primary contract)

## Acceptance Criteria

1. Scenario test `survival_rest_interrupted_by_danger::scenario_c_hostile_proximity_wake` passed all 9 verification-layer assertions.
2. Deterministic replay test `survival_rest_interrupted_by_danger::scenario_c_hostile_proximity_wake_replays_deterministically` passed.
3. Existing suite: `cargo test --workspace` passed.

### Invariants

1. `WakeReason::LocalDisturbance { cause: HostileProximity }` is the only cause variant emitted in this scenario
2. Partial recovery preserved — `accumulated_recovery > 0 && accumulated_recovery < target_recovery` at abort tick
3. The agent does not immediately re-attempt Sleep at the hostile-occupied shelter — replan must reflect updated belief state

## Verification Result

1. Passed `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_rest_interrupted_by_danger`
2. Passed `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_safe_rest scenarios::survival_sleep_contention scenarios::survival_rest_interrupted_by_danger`
3. Passed `cargo test -p worldwake-ai candidate_generation::tests`
4. Passed `cargo test -p worldwake-sim tick_step`
5. Passed `cargo test -p worldwake-ai`
6. Passed `cargo test --workspace`
7. Passed `cargo clippy --workspace`
8. Passed `cargo clippy --workspace --all-targets -- -D warnings`
9. Passed `cargo fmt --all -- --check`
10. Passed `git diff --check`
11. Waived the verify wrapper because its required sub-gates were run directly above.
