# S46GOLGAP-001: Implement golden_patrol_driven_crime_discovery (Scenario 57)

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — focused planner/root-candidate fix plus golden coverage
**Deps**: `specs/S46-golden-gaps-E19.md`, existing patrol/crime/perception code in `worldwake-ai`, `worldwake-systems`, and `worldwake-core`

## Problem

No golden test demonstrates the cross-system chain where patrol physically brings a guard to a place, local perception detects a violated expectation there, and that mismatch enters the live `InvestigateViolation` pipeline. Reassessment against the current runtime shows this is not only a missing golden: the planner currently synthesizes an impossible remote `investigate` root candidate for an `ActorPlace` action, so patrol-driven investigation can be generated yet still fail to execute lawfully. This ticket now covers the minimal production fix plus the missing golden coverage.

## Assumption Reassessment (2026-03-30)

1. **Shared abstraction boundary under audit**: the live boundary is `Patrol`-driven arrival into the local belief/perception mismatch pipeline:
   `GoalKind::Patrol { place }` -> local observation at `effective_place` -> `ViolationMemory` `EntityMissing` record -> `GoalKind::InvestigateViolation { violation_id, place }`.
   This is the actual cross-system contract to prove in this ticket.
2. **Patrol state is present and already exercised**: `PatrolRoute` and `PatrolProfile` live in [`crates/worldwake-core/src/patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs). Existing `golden_patrol.rs` scenarios 52–56 already prove route cycling, interruption, motive scaling, route adaptation, and locality.
3. **Violation generation depends on an investigation profile**: `emit_expectation_violation_candidates()` in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) returns early unless the agent has a `ViolationDispositionProfile`. The original ticket omitted this required component. Scenario 57 must seed it.
4. **Live mismatch shape is `EntityMissing`, not theft by default**: the candidate-generation path emits `InvestigateViolation` for `ViolationKind::EntityMissing` and `SupplyDepleted`; it does not synthesize `SuspectedTheft` goals. That means the scenario should assert patrol-driven `EntityMissing` discovery first, not assume typed theft evidence appears automatically.
5. **`SuspectedTheft` escalation is owner-only in the current architecture**: `commit_investigate()` in [`crates/worldwake-systems/src/investigate_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/investigate_actions.rs) records `WitnessedAbsence` for any investigator, but only records `SocialObservationDetail::SuspectedTheft` and `ViolationKind::SuspectedTheft` when `belief.believed_owner_of(subject) == Some(actor)`. A guard investigating an unowned or third-party lot should not produce suspected-theft evidence under the live rules.
6. **The spec/ticket narrative diverged here**: the active spec still describes a patrol-driven theft discovery chain ending in `SuspectedTheft`, but the live code distinguishes absence discovery from owner-aware theft inference. Forcing this ticket to prove non-owner theft escalation would either encode a false assumption or require an architectural change, so the ticket scope is corrected to the lawful current behavior.
7. **Harness and file-local helpers available today**: `seed_agent`, `set_agent_perception_profile`, `seed_belief_from_world`, `new_txn`, and `commit_txn` exist in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs). `golden_patrol.rs` already contains the local patrol helpers; this ticket can add a file-local `set_violation_profile` helper rather than exporting new harness surface.
8. **Scenario isolation remains correct, but the theft should be a manual relocation to a non-local place**: manually moving the lot to the guard's current location would lawfully refresh the guard's belief before arrival and defeat the mismatch. The scenario should relocate the lot to another place the guard does not currently observe.
9. **Newly exposed production contradiction**: the failing first implementation showed `GroundedGoal::synthesized_root_candidate_targets()` in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) synthesizing a direct `PlannerOpKind::Investigate` root candidate for `GoalKind::InvestigateViolation` with `TargetSpec::ActorPlace`. That bypasses the real affordance boundary and lets the planner keep an impossible remote investigate branch alive. Decision traces showed this as an `UnknownBlockerTrace` on `Investigate` while the actor was away from the violation place. This is a required consequence of the intended change, not a separate bug to defer.
10. **No existing golden covers this exact chain**: `golden_patrol.rs` has no patrol-plus-investigation scenario; `golden_emergent.rs` covers same-place and owner-driven investigation chains without patrol; `golden_social.rs` does not cover patrol-driven discovery. The gap is still real, but it is specifically a missing patrol-driven `EntityMissing` golden, and the planner contradiction must be fixed for that golden to be truthful.

## Architecture Check

1. The cleaner architecture is still to test the current evidence boundary honestly: patrol creates lawful co-location, perception discovers a violated expectation, investigation records absence. This preserves the existing distinction between observed absence and ownership-backed theft inference.
2. The newly exposed planner contradiction is not a ticket-only artifact. Synthesizing a direct remote `investigate` root candidate for an `ActorPlace` action is weaker architecture than relying on the real affordance boundary. It lets plan search admit impossible branches and defer failure to runtime revalidation.
3. The focused production fix is to remove that invalid synthesis path for `InvestigateViolation` and let investigate planning flow through lawful affordances when co-located, with travel as the prerequisite path when remote. That is cleaner, more robust, and more extensible than tuning motive weights or adding patrol-specific exceptions.
4. Changing the engine so a non-owner patrol guard upgrades any missing public lot into `SuspectedTheft` would still weaken the evidence model by conflating "I observed an absence" with "I can infer theft." That remains out of scope.

## Verification Layers

1. Opening patrol intent -> decision trace via `planning_trace_at(...).selection.selected_goal()`
2. Patrol-driven movement to the crime scene -> action trace (`patrol` / `travel` lifecycle events) plus authoritative `effective_place`
3. Local expectation mismatch on arrival -> authoritative `ViolationMemory` record with `ViolationKind::EntityMissing`
4. Investigation candidate generation -> decision trace (`candidates.generated` / selected goal)
5. Investigation execution -> action trace (`investigate` committed with `ActionTraceDetail::Investigate`)
6. Investigation aftermath -> authoritative `AgentBeliefStore` contains `SocialObservationDetail::WitnessedAbsence`
7. Non-owner evidence boundary -> authoritative `ViolationMemory` and `AgentBeliefStore` do not contain `SuspectedTheft`
8. Planner legality boundary -> focused unit coverage on `GroundedGoal::synthesized_root_candidate_targets()` and/or equivalent planner search surface proves `InvestigateViolation` no longer synthesizes a direct remote `ActorPlace` root candidate
9. Conservation -> authoritative lot conservation / explicit relocation only, no item creation or destruction

## What to Change

### 1. Add Scenario 57 test function in `golden_patrol.rs`

Write `golden_patrol_driven_crime_discovery` test function with:

- **Topology**: use prototype places already present in the golden harness: `VillageSquare`, `GeneralStore`, and one non-local destination for the relocated lot such as `CommonHouse`.
- **Guard agent**: place at `VillageSquare` with a single-waypoint patrol route `PatrolRoute { assigned_places: [GeneralStore], current_index: 0 }`, a moderate `PatrolProfile`, `PerceptionProfile`, and `ViolationDispositionProfile`. This isolates patrol-as-arrival from route-cycling behavior already covered elsewhere. `set_patrol_state` seeds the belief store and violation memory; a new file-local helper can seed the violation-disposition component.
- **Missing subject**: create a bread lot at `GeneralStore` via world transaction and seed the guard's belief from world at tick 0.
- **Manual relocation**: after the guard opens with the `Patrol` goal but before arrival at `GeneralStore`, relocate the bread lot to a different place the guard is not currently observing. Do not move it to `VillageSquare`, because that would lawfully refresh the guard's belief before patrol arrival.
- **Enable tracing**: `h.driver.enable_tracing()` and `h.enable_action_tracing()`.
- **Step loop**: run enough ticks for the guard to finish the opening dwell, travel to `GeneralStore`, discover the local mismatch, generate `InvestigateViolation`, and commit `investigate`.

**Assertions**:
- Opening tick: guard selects `GoalKind::Patrol { place: GeneralStore }` (decision trace).
- Patrol lifecycle: trace shows `travel` toward `GeneralStore`, then `patrol` once the guard arrives there, and authoritative location becomes `GeneralStore` before investigation.
- After arrival: guard's `ViolationMemory` contains an unresolved `ViolationKind::EntityMissing { entity: bread_lot, expected_place: GeneralStore }`.
- Decision trace: a later planning tick generates or selects `GoalKind::InvestigateViolation { violation_id, place: GeneralStore }`.
- Action trace: `investigate` commits for that same `violation_id`.
- Final state: guard's `AgentBeliefStore` contains `SocialObservationDetail::WitnessedAbsence` for the lot at `GeneralStore`; the original `EntityMissing` record is resolved; no `SuspectedTheft` belief or violation record is added for this non-owner investigation.

### 2. Fix remote investigate root synthesis

- Update `GroundedGoal::synthesized_root_candidate_targets()` in `crates/worldwake-ai/src/goal_model.rs` so `GoalKind::InvestigateViolation` does not synthesize a direct `PlannerOpKind::Investigate` root candidate for an `ActorPlace` action.
- The planner should rely on real affordances for `investigate` when co-located and on `travel` as the lawful prerequisite path when remote.
- Add focused unit coverage in `goal_model.rs` for this root-synthesis contract.

**Scenario comment block** following established format:
```
// Scenario 57: Patrol-Driven Crime Discovery Chain
// Systems: AI, Travel, Patrol, Perception, Investigation
// GoalKinds: Patrol, InvestigateViolation
// ActionDomains: Travel, Generic
// Places: VillageSquare, GeneralStore
// Principles: 1, 7, 14, 17
```

## Files to Touch

- `crates/worldwake-ai/tests/golden_patrol.rs` (modify — add test function and any file-local helpers)
- `crates/worldwake-ai/src/goal_model.rs` (modify — focused root-synthesis fix and unit coverage)
- `docs/generated/golden-e2e-inventory.md` (regenerate)
- `docs/generated/golden-scenario-map.md` (regenerate)
- `docs/generated/golden-coverage-matrix.md` (regenerate if changed by generator)

## Out of Scope

- Replay companion test from the original spec. Existing `golden_patrol.rs` does not pair every scenario with a replay twin, and this ticket is about proving the causal boundary, not duplicating the scenario for determinism-only coverage.
- Adding new golden harness helpers to `golden_harness/mod.rs` (use existing helpers; if a new helper is truly needed, it should be file-local in `golden_patrol.rs`).
- Testing thief AI or the `steal` action handler — the missing lot is simulated by a manual lawful relocation.
- Modifying existing scenarios S52–S56.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery` — new test passes.
2. `cargo test -p worldwake-ai goal_model` or a narrower real test name covering the new root-synthesis unit(s) — focused planner coverage passes.
3. `cargo test -p worldwake-ai --test golden_patrol` — all existing patrol golden tests (S52–S56) still pass.
4. `python3 scripts/golden_inventory.py --write --check-docs` — scenario metadata and generated docs stay in sync.
5. `cargo clippy -p worldwake-ai --test golden_patrol -- -D warnings` or `cargo clippy -p worldwake-ai` — no new warnings.

### Invariants

1. Guard discovers crime only through physical arrival at the crime scene via patrol travel — no remote awareness (FND-7).
2. Guard's investigation triggers from stale belief mismatch against observed state, not from authoritative world truth (FND-14, FND-17).
3. `InvestigateViolation` planning must not admit a direct impossible remote `investigate` root step for an `ActorPlace` action.
4. Non-owner patrol investigation records absence, not typed theft evidence, under the live architecture.
5. Conservation: bread lot is relocated, not destroyed — no conservation violation.
6. Existing patrol scenarios S52–S56 continue to pass unchanged.

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_patrol.rs::golden_patrol_driven_crime_discovery`
Rationale: proves the missing cross-system patrol -> local mismatch -> investigation chain end to end without encoding a false non-owner theft inference.
2. `crates/worldwake-ai/src/goal_model.rs::<new focused investigate root-synthesis test(s)>`
Rationale: locks the production fix at the strongest planner-owned boundary so the golden is not the first place this impossible remote investigate path regresses.

### Commands

1. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery`
2. `cargo test -p worldwake-ai grounded_goal_synthesizes_investigate_root_targets_only_when_colocated`
3. `cargo test -p worldwake-ai --test golden_patrol`
4. `cargo test -p worldwake-ai`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `cargo clippy -p worldwake-ai`

## Outcome

- Completed: 2026-03-30
- What actually changed:
  - Added `golden_patrol_driven_crime_discovery` in [`crates/worldwake-ai/tests/golden_patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs) to prove the lawful patrol -> local `EntityMissing` -> `InvestigateViolation` -> `WitnessedAbsence` chain for a non-owner guard.
  - Added a focused file-local violation-profile helper in [`crates/worldwake-ai/tests/golden_patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs) because the live candidate-generation path requires `ViolationDispositionProfile`.
  - Fixed planner root synthesis in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) and [`crates/worldwake-ai/src/search/candidates.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs) so `InvestigateViolation` only synthesizes an `Investigate` root candidate when the actor is already co-located with the violation place.
  - Added focused planner-owned regression coverage in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) for the colocated-only root-synthesis contract.
  - Regenerated the golden inventory docs in [`docs/generated/golden-e2e-inventory.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-e2e-inventory.md), [`docs/generated/golden-scenario-map.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-scenario-map.md), and [`docs/generated/golden-coverage-matrix.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-coverage-matrix.md).
- Deviations from original plan:
  - The original spec/ticket narrative assumed the patrol guard should end with `SuspectedTheft`. Reassessment against live code showed that inference is owner-only, so the implemented and archived invariant is the cleaner current architecture: non-owner patrol investigation records `WitnessedAbsence`, not theft.
  - The final scenario uses a single-waypoint patrol route to isolate patrol-driven arrival instead of exercising broader route-cycling behavior already covered by S52-S56.
  - A production planner fix became required in scope because the new golden exposed an invalid remote `Investigate` root-synthesis path that contradicted the affordance boundary.
- Verification results:
  - `cargo test -p worldwake-ai grounded_goal_synthesizes_investigate_root_targets_only_when_colocated`
  - `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery`
  - `cargo test -p worldwake-ai --test golden_patrol`
  - `cargo test -p worldwake-ai`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo clippy -p worldwake-ai`
