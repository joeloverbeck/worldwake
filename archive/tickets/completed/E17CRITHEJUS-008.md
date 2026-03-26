# E17CRITHEJUS-008: Implement accuse action

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new action definition + handler in systems crate
**Deps**: E17CRITHEJUS-003 (needs `InstitutionalClaim::Accusation`, `RecordKind::CrimeRegister`), E17CRITHEJUS-015 (typed social-evidence detail), E17CRITHEJUS-016 (social-evidence relay through Tell)

## Problem

No mechanism exists for agents to formally accuse another agent of theft. The accusation system requires a new action that appends an `Accusation` entry to a `CrimeRegister` record entity, validated against the accuser's concrete evidence in their belief store.

## Assumption Reassessment (2026-03-26)

Shared abstraction boundary under audit: the accusation case identity `(accused, violation_id)` across `GoalKind::Accuse` in `crates/worldwake-core/src/goal.rs`, `InstitutionalBeliefKey::CrimeCase` and `InstitutionalClaim::Accusation` in `crates/worldwake-core/src/institutional.rs`, and the runtime/planner payload surface in `crates/worldwake-sim/src/action_payload.rs` plus `crates/worldwake-ai/src/goal_model.rs`.

1. `consult_record_actions.rs` and `office_actions.rs` in `crates/worldwake-systems/src/` remain the closest structural precedents. The live record mutation path is `WorldTxn::append_record_entry()` in `crates/worldwake-core/src/world_txn.rs`, not direct ad hoc `RecordData` mutation inside handlers.
2. `CrimeRegister` and `InstitutionalClaim::Accusation` already exist in `crates/worldwake-core/src/institutional.rs`. The ticket no longer depends on E17CRITHEJUS-003 for those types.
3. Typed theft evidence already exists. `SocialObservationDetail::SuspectedTheft { missing_entity, expected_place, suspect }` is live in `crates/worldwake-core/src/belief.rs`, and `investigate_actions.rs` already records both the social observation and `ViolationKind::SuspectedTheft` when the investigating owner confirms the missing item.
4. Relayable social-evidence topics already exist. `TellTopic::SocialObservation` is live, `tell_actions.rs` already relays `SuspectedTheft`, and the old sidecar-on-`TellTopic::EntityBelief` assumption is stale. This ticket should consume the canonical social-observation path rather than preserve the older coupling narrative.
5. `ActionDomain::Social` is still the correct domain. `TargetSpec::EntityAtActorPlace { kind: EntityKind::Agent }` is a better fit than the ticket's earlier generic `SpecificEntity` wording because it matches the current action binding architecture and same-place precondition surface.
6. `GoalKind::Accuse` already exists in `crates/worldwake-core/src/goal.rs`, but the planner still deliberately defers it via `DEFERRED_CRIME_JUSTICE_OPS` in `crates/worldwake-ai/src/goal_model.rs` and the search regression `accuse_and_punish_goals_remain_deferred_without_actions` in `crates/worldwake-ai/src/search/tests.rs`. If we add a real `accuse` action, leaving `GoalKind::Accuse` deferred would preserve a known architectural contradiction. Correct scope is to wire the planner/runtime payload path for `Accuse` now, while keeping accusation candidate generation and punishment actions out of scope.
7. No live candidate generation exists yet for `GoalKind::Accuse`; that remains future work under E17CRITHEJUS-011. This ticket must not silently broaden into new social goal emission.
8. The most robust evidence contract is narrower than the original ticket text. The live code can prove accusation against a concrete `(accused, violation_id)` when the accuser has either:
   - `ViolationKind::SuspectedTheft { suspect: Some(accused), .. }` in `ViolationMemory`, or
   - `SocialObservationDetail::SuspectedTheft { suspect: Some(accused), .. }` in `AgentBeliefStore`.
   The earlier idea of accepting any witnessed crime-tagged event or a vague belief that the stolen item is in the accused's possession is not currently grounded in typed belief surfaces. Expanding to those weaker paths now would force new ambiguous inference rules into the handler.
9. Duplicate-case rejection should use the current authoritative record, not prior consultation as a hard prerequisite. `RecordData::active_entries()` plus `InstitutionalClaim::Verdict` supersession semantics in `crates/worldwake-core/src/institutional.rs` provide the lawful unresolved-case check.
10. Relevant live tests already cover the adjacent dependencies this ticket relies on: `investigate_actions::tests::owner_investigating_missing_owned_entity_records_suspected_theft`, tell-action tests covering `TellTopic::SocialObservation`, and `consult_record_actions` tests projecting `CrimeCase` institutional beliefs. The focused implementation gap is the missing `accuse` action plus planner payload wiring.

## Architecture Check

1. A dedicated `justice_actions.rs` module is cleaner than growing `office_actions.rs`. Crime/justice records are institution-adjacent, but they are not office succession logic. Keeping them separate preserves domain boundaries and leaves room for `fine`/`exile` without turning `office_actions.rs` into a mixed-domain grab bag.
2. The action should use the canonical crime-case identity already present in core types: payload carries `violation_id`, target carries `accused`, record keying uses `(accused, violation_id)`. Reusing that identity across systems/planner code is cleaner than inventing a second lookup path based on missing-item tuples or implicit suspect inference.
3. No backward-compatibility aliasing. Evidence validation reads only from the actor's subjective belief surfaces (`ViolationMemory`, `AgentBeliefStore` social observations, believed alive status), never from world truth. Wrong accusations remain architecturally possible, which is the correct P12/P14 behavior.
4. Wiring `GoalKind::Accuse` through planner semantics is more robust than leaving it as a dead goal enum variant once a real action exists. This ticket still stops short of new accusation candidate generation, so scope remains targeted.

## Verification Layers

1. `accuse` commit appends `InstitutionalClaim::Accusation` into `CrimeRegister` -> authoritative `RecordData` state
2. Start gate rejects unresolved duplicate `(accused, violation_id)` cases -> action start-failure surface
3. Start gate rejects missing or mismatched subjective theft evidence -> action start-failure surface
4. Planner/runtime payload path resolves `GoalKind::Accuse` into `ActionPayload::Accuse { violation_id }` and relevant action defs -> focused AI search/unit coverage
5. Wrong accusation remains possible with subjective but wrong suspect evidence -> focused systems test proving belief-driven validation, not truth-driven validation

## What to Change

### 1. New `justice_actions.rs` module in worldwake-systems

- Add `register_accuse_action()` following the established registration pattern.
- Action definition: name `"accuse"`, domain `ActionDomain::Social`, target `TargetSpec::EntityAtActorPlace { kind: EntityKind::Agent }`, `VisibilitySpec::SamePlace`, tags `[EventTag::Social, EventTag::Crime, EventTag::WorldMutation]`, duration `Fixed(1)`.
- Add typed runtime payload `ActionPayload::Accuse(AccuseActionPayload { violation_id })` in `crates/worldwake-sim/src/action_payload.rs`.

### 2. Start handler

Validate authoritatively:
- Actor and accused are co-located
- A `CrimeRegister` record entity exists at the actor's current place
- Actor has subjective theft evidence for the exact `(accused, violation_id)` case via either:
  - unresolved `ViolationKind::SuspectedTheft { suspect: Some(accused), .. }`, or
  - `SocialObservationDetail::SuspectedTheft { suspect: Some(accused), .. }`
- Accused is believed alive
- No unresolved `Accusation` or `Verdict`-open duplicate exists for the same `(accused, violation_id)` in the chosen `CrimeRegister`

### 3. Commit handler

- Append `InstitutionalClaim::Accusation { accuser, accused, violation_id, effective_tick }` to the CrimeRegister
- Emit event with `EventTag::Crime`, `VisibilitySpec::SamePlace`

### 4. Planner/runtime wiring

- Add `PlannerOpKind::Accuse` and classify the new action in `crates/worldwake-ai/src/planner_ops.rs`.
- Replace the deferred-op hole for `GoalKind::Accuse` in `crates/worldwake-ai/src/goal_model.rs` so search can treat `accuse` as a real relevant operator.
- Build `ActionPayload::Accuse { violation_id }` from `GoalKind::Accuse`.
- Leave `GoalKind::PunishAccused` deferred.

### 5. Register and export

Wire into `register_all_actions()` in `action_registry.rs`. Export from `lib.rs`.

## Files to Touch

- `crates/worldwake-systems/src/justice_actions.rs` (new)
- `crates/worldwake-systems/src/action_registry.rs` (modify)
- `crates/worldwake-systems/src/lib.rs` (modify)
- `crates/worldwake-sim/src/action_payload.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- Fine and Exile actions (E17CRITHEJUS-009)
- Steal action (E17CRITHEJUS-006)
- AI candidate generation for accusation (E17CRITHEJUS-011)
- CrimeRegister entity creation in world setup beyond focused tests
- Contest or appeal of accusations (future spec)
- Validating evidence accuracy against world truth (accusations use belief store — P12)
- New inference paths from generic witnessed crime events or indirect possession suspicion to an accusation case

## Acceptance Criteria

### Tests That Must Pass

1. Accusation creates `InstitutionalClaim::Accusation` entry in CrimeRegister
2. Accusation entry has correct `accuser`, `accused`, `violation_id`, `effective_tick`
3. Duplicate accusation (same accused + same violation) rejected at start
4. Accusation without any evidence in accuser's belief store rejected at start
5. Accusation with typed theft evidence matching accused succeeds
6. Accusation event emitted with `EventTag::Crime` and `VisibilitySpec::SamePlace`
7. `GoalKind::Accuse` no longer remains deferred once the action exists; focused planner tests cover relevant-op and payload resolution
8. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. Evidence validation reads from `AgentBeliefStore`, never from world truth (P12)
2. Evidence validation may also read the actor's authoritative `ViolationMemory` because that memory is still the actor's subjective state, not global truth
3. Wrong accusations are possible — the handler does not verify evidence accuracy (P14)
4. CrimeRegister is append-only — accusation entries are never mutated
5. Accusation is a public act (`VisibilitySpec::SamePlace`) — co-located agents can witness it
6. `PunishAccused` stays deferred until verdict actions exist; this ticket only removes the architectural stub for `Accuse`

## Tests

### New/Modified Tests And Rationale

1. `crates/worldwake-systems/src/justice_actions.rs` — successful accusation appends the correct claim to the colocated `CrimeRegister`
   Rationale: proves the authoritative record mutation and event-facing core outcome.
2. `crates/worldwake-systems/src/justice_actions.rs` — duplicate unresolved accusation for the same `(accused, violation_id)` start-fails
   Rationale: proves the shared crime-case identity is enforced through current record state, not duplicated.
3. `crates/worldwake-systems/src/justice_actions.rs` — accusation without matching subjective theft evidence start-fails
   Rationale: proves the handler reads belief surfaces rather than world truth or generic suspicion.
4. `crates/worldwake-systems/src/justice_actions.rs` — accusation can succeed from wrong-but-subjective suspect evidence
   Rationale: proves P12/P14 behavior and guards against future truth-check regressions.
5. `crates/worldwake-ai/src/goal_model.rs` — `GoalKind::Accuse` builds `ActionPayload::Accuse { violation_id }`
   Rationale: proves planner/runtime payload wiring for the new action.
6. `crates/worldwake-ai/src/search/tests.rs` — `GoalKind::Accuse` exposes the `accuse` action as a relevant search operator while `PunishAccused` remains deferred
   Rationale: proves the implementation removes only the intended architectural hole.

### Commands

1. `cargo test -p worldwake-systems justice_actions::tests`
2. `cargo test -p worldwake-ai goal_model::tests::accuse_goal_builds_accuse_payload_override`
3. `cargo test -p worldwake-ai search::tests::accuse_goal_exposes_accuse_action_while_punish_remains_deferred`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace`
7. `cargo build --workspace`

## Outcome

- Completed: 2026-03-26
- What actually changed:
  - Added a real `accuse` action in `crates/worldwake-systems/src/justice_actions.rs`, including typed payloads, subjective-evidence validation, duplicate-case rejection, and append-only `CrimeRegister` mutation.
  - Added `ActionPayload::Accuse` in `crates/worldwake-sim/src/action_payload.rs` and exported the payload through `crates/worldwake-sim/src/lib.rs`.
  - Wired `GoalKind::Accuse` through planner/runtime surfaces (`PlannerOpKind::Accuse`, goal payload synthesis, relevant-op mapping, progress-barrier handling) while leaving `PunishAccused` deferred.
  - Strengthened planner architecture beyond the original ticket by adding narrow goal-synthesized search fallbacks for exact-goal operators whose payloads are already derivable from goal identity (`trade`, `press_force_claim`, `investigate`, `tell`). This removed a broader architectural contradiction where grounded goals could exist but search still depended on affordance payload enumeration to surface the leaf action.
  - Preserved investigation duration data in `PlanningSnapshot` by carrying `ViolationDispositionProfile`, so investigate plans remain duration-estimable in runtime planning, not just in focused unit views.
- Deviations from original plan:
  - Scope grew slightly beyond the raw accuse action because reassessment showed the live architecture already had multiple grounded-goal/operator mismatches. Leaving those in place would have made the new `Accuse` path another special-case patch rather than a durable planner fix.
  - No accusation candidate generation was added; `GoalKind::Accuse` is now executable once present, but goal emission remains deferred to follow-up ticket E17CRITHEJUS-011.
- Verification results:
  - `cargo test -p worldwake-systems justice_actions::tests` ✅
  - `cargo test -p worldwake-systems` ✅
  - `cargo test -p worldwake-ai goal_model::tests::accuse_goal_builds_accuse_payload_override` ✅
  - `cargo test -p worldwake-ai search::tests::accuse_goal_exposes_accuse_action_while_punish_remains_deferred` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo build --workspace` ✅
