# S174SHESLESUR-008: Scenario B — survival-sleep-contention.ron (multi-slot contention + S44 queue promotion)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — golden scenario plus narrow scenario-contract and queue-readiness fixes required to exercise rest-site queue promotion.
**Deps**: `archive/tickets/S174SHESLESUR-001.md`, `archive/tickets/S174SHESLESUR-002.md`, `archive/tickets/S174SHESLESUR-003.md`, `archive/tickets/S174SHESLESUR-004.md`, `archive/tickets/S174SHESLESUR-005.md`, `archive/tickets/S174SHESLESUR-006.md`

## Outcome

Scenario B landed as `scenarios/survival-sleep-contention.ron` plus the golden module `crates/worldwake-ai/tests/scenarios/survival_sleep_contention.rs`. The scenario uses one roofed capacity-2 barracks and three tired agents. The golden proof records three KnownRestSite emissions, two immediate targeted Sleep starts, one failed full-site Sleep attempt, S44 rest-site queue admission, promotion after an occupant releases a slot, promoted targeted Sleep start, completion for all three agents, bounded elevated-fatigue frames, and deterministic replay.

The ticket was reassessed from a test-only task to a narrow production change because the scenario contract could not author `ContentionPolicy` on a place, place targets were not fully visible to affordance and validation paths, and S44 queue readiness treated rest-site contention as exclusive instead of capacity-gated.

## Reassessment Result

1. S44 already had `ContentionQueue`, `ContentionPolicy`, grant/expiry semantics, `EventTag::ContentionResolved`, `EventTag::QueueGrantPromoted`, and `PromotableContentionKind::RestSite`.
2. Scenario authoring accepted `RestCapacity` on `PlaceDef` but lacked place-level `ContentionPolicy` and queue seeding. This ticket added that contract rather than faking queue state in the golden test.
3. `queue_for_facility_use` was facility-targeted. Rest-site queue admission needed to accept a rest-capable `Place` target for intended `Sleep` actions while preserving the existing workstation/facility validation branch for non-sleep uses.
4. The generic S44 readiness helper treated every promotable action as exclusive. Rest-site queues now use `RestCapacity` and `RestOccupancy` as the multi-slot gate.
5. Full rest sites stay visible as planning opportunities so queue-managed sites can choose the queue branch, but direct targeted Sleep materialization is suppressed while a rest site is full. Unmanaged full rest sites still allow targetless rough-sleep fallback.

## Architecture Result

1. The implementation reuses S44's contention queue substrate for rest sites. No parallel rest queue was introduced.
2. Place-backed queue state remains state-mediated through `ContentionPolicy`, `ContentionQueue`, `RestCapacity`, `RestOccupancy`, and event-log promotion records.
3. AI planning remains belief-backed: the planning snapshot carries rest-site capacity and occupant count from the per-agent belief view instead of reading authoritative world state directly.

## Landed Changes

1. Added place-level `contention_policy` scenario authoring, queue seeding on places, coverage extraction, and component-schema validity for `ContentionPolicy` and `ContentionQueue` on `EntityKind::Place`.
2. Expanded place-target handling in authoritative validation, affordance enumeration, and per-agent local visibility so an actor's current place can be a lawful place target.
3. Expanded `queue_for_facility_use` to admit sleep queue payloads targeting rest-capable places and left the existing workstation/exclusive-operation validation path intact for other intended actions.
4. Added rest-site capacity and occupant-count fields to planning snapshots and `FacilityBeliefView`.
5. Updated sleep candidate/search behavior so full rest sites remain candidate-visible for queue planning, direct targeted Sleep is not planned against a full site, queue-managed full rest sites emit queue candidates, and rough sleep is suppressed while the actor is queued or granted at the local rest site.
6. Treated `QueueForFacilityUse` as a Sleep progress barrier and preserved rest-site queue intent after the queue action completes so the waiting actor can resume after promotion.
7. Updated S44 rest-site promotion readiness so active sleepers do not make a multi-slot rest site globally exclusive, full rest sites do not promote waiters until a slot opens, and place-backed queues use the place itself for locality and queue events.
8. Added focused regression tests for place contention policy authoring, place-target validation and affordance enumeration, Sleep progress-barrier handling, rest-site active-action exclusivity, full rest-site promotion after release, candidate generation, and search behavior.

## Landed Files

- `scenarios/survival-sleep-contention.ron`
- `crates/worldwake-ai/tests/scenarios/survival_sleep_contention.rs`
- `crates/worldwake-ai/tests/scenarios/mod.rs`
- `crates/worldwake-ai/src/agent_tick/active_action.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/planning_snapshot.rs`
- `crates/worldwake-ai/src/planning_state.rs`
- `crates/worldwake-ai/src/search/candidates.rs`
- `crates/worldwake-cli/src/scenario/mod.rs`
- `crates/worldwake-cli/src/scenario/types.rs`
- `crates/worldwake-cli/src/bin/scenario_coverage.rs`
- `crates/worldwake-core/src/component_schema.rs`
- `crates/worldwake-sim/src/action_validation.rs`
- `crates/worldwake-sim/src/affordance_query.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`
- `crates/worldwake-systems/src/facility_queue.rs`
- `crates/worldwake-systems/src/facility_queue_actions.rs`

## Out of Scope

- Single-slot rest-site contention (Scenario A / `archive/tickets/S174SHESLESUR-007.md`)
- Hostile-proximity interruption (Scenario C / ticket 009)
- CLI player-POV (Scenario D / ticket 010)
- Failed-rest cascade for S175 (Scenario E / ticket 011)

## Acceptance Criteria

1. `survival_sleep_contention::scenario_b_multi_slot_contention` passed all golden assertions.
2. `survival_sleep_contention::scenario_b_multi_slot_contention_replays_deterministically` passed.
3. Existing suite: `cargo test --workspace` passed.

### Invariants

1. `RestOccupancy.occupants.len()` never exceeds 2 at `barracks` (the `RestCapacity` cap)
2. Queue grant promotion fires exactly once per occupant release
3. The third agent's Sleep episode completes — no agent permanently stuck at elevated fatigue under nominal scenario conditions

## Verification Result

1. Passed `cargo test -p worldwake-systems active_rest_site_sleep_does_not_make_capacity_slot_exclusive`
2. Passed `cargo test -p worldwake-cli spawn_place_contention_policy_seeds_queue_state`
3. Passed `cargo test -p worldwake-systems facility_queue_actions`
4. Passed `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_sleep_contention`
5. Passed `cargo test -p worldwake-sim target_at_actor_place_accepts_actor_place_target`
6. Passed `cargo test -p worldwake-sim enumerate_targets_any_of_includes_actor_place_when_place_is_allowed`
7. Passed `cargo test -p worldwake-systems full_rest_site_waiter_promotes_after_capacity_opens`
8. Passed `cargo test -p worldwake-ai queue_for_facility_use_is_progress_barrier_for_exclusive_goal_families`
9. Passed `cargo test -p worldwake-ai candidate_generation::tests`
10. Passed `cargo test -p worldwake-ai search::tests`
11. Passed `cargo test -p worldwake-sim affordance_query`
12. Passed `cargo test -p worldwake-systems facility_queue::tests`
13. Passed `cargo test -p worldwake-ai --test golden_ai -- scenarios::survival_safe_rest`
14. Passed `cargo test -p worldwake-ai search::tests::place_anchored_sleep_materializes_rest_site_target`
15. Passed `cargo test -p worldwake-ai`
16. Passed `cargo test -p worldwake-cli latrine_fullness_only_set_on_latrine_tagged_places`
17. Passed `cargo test --workspace`
18. Passed `cargo clippy --workspace`
19. Passed `cargo clippy --workspace --all-targets -- -D warnings`
20. Passed `cargo fmt --all -- --check`
21. Passed `git diff --check`
22. Waived the verify wrapper because its required sub-gates were run directly above.
