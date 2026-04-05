# S44GENCONSUB-006: Contention-aware action validation

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — contention queue admission actions, grant-gated action start, planner/search queue integration
**Deps**: S44GENCONSUB-004, S44GENCONSUB-005

## Problem

Loot, bury, and heal actions currently resolve contention by implicit tick order — whoever's action starts first wins. FOUNDATIONS P8, P9, P20, and Canonical Scenario E require an explicit queue/grant world process for queue-based contention domains. The live runtime has no lawful scheduler-level "queued, not started" outcome, so this ticket must introduce real queue-admission actions for corpse/care targets and gate the exclusive loot/bury/heal actions on grants.

## Assumption Reassessment (2026-04-04)

1. `register_loot_action()` at `crates/worldwake-systems/src/combat.rs:55-64`. Action handler: `start_loot, tick_loot, commit_loot, abort_loot`. Confirmed.
2. `register_bury_action()` at same file lines 66-77. Confirmed.
3. `register_heal_action()` at same file lines 79-90. Domain `ActionDomain::Care`. Confirmed.
4. `validate_action_def_authoritatively()` at `crates/worldwake-sim/src/action_validation.rs` checks actor constraints and preconditions. It does not own contention admission today. Confirmed.
5. `start_gate.rs` and `Scheduler::start_affordance()` still only support two outcomes: action starts, or start fails and is reported through `ActionStartFailure` / `ActionTraceKind::StartFailed`. There is no existing third "queued, not started" scheduler result. Confirmed.
6. The existing lawful queue-join pattern is the real `queue_for_facility_use` action in `crates/worldwake-systems/src/facility_queue_actions.rs`, which commits queue state through authoritative world mutation rather than through a special scheduler status. Confirmed.
7. Planning/search already understands queue-join as a progress-barrier operation through `PlannerOpKind::QueueForFacilityUse`, and planning snapshots already carry generalized contention state for arbitrary contention-managed entities even though some helper names still say "facility". Confirmed.
8. After S44GENCONSUB-004 and 005, `ContentionQueue`, `ContentionPolicy`, `ContentionStatus`, and maintenance-time waiter pruning are available. Missing piece: lawful queue admission and grant gating for corpse/care exclusive actions.

## Architecture Check

1. Queue-based contention domains must materialize a real queue-join action instead of inventing a hidden scheduler status. This matches FOUNDATIONS P8/P9/P20/P21 more honestly than overloading `StartFailed` to mean "you are now waiting."
2. Loot, bury, and heal then become grant-gated exclusive actions, analogous to harvest/craft on exclusive facilities, with grant release on completion/abort.
3. The existing planner-side queue op can be reused as the queue barrier abstraction for these new actions; no new live authority path is needed.
2. No backward-compatibility shims.

## Verification Layers

1. Queue-for-corpse / queue-for-care action on contention-managed target with room → waiter added + `ContentionIntents` updated → authoritative world state
2. Queue action on full queue → structured rejection → action trace / scheduler start failure
3. Loot/bury/heal without grant on contention-managed target → structured rejection → action trace
4. Loot/bury/heal with matching grant → action starts normally
5. Loot/bury/heal commit or abort clears matching grant + actor intent → authoritative world state
6. AI/search emits the lawful queue step ahead of loot/bury/heal when contention queueing is available
7. Cross-layer: planner/search (AI) selects queue op, queue action mutates contention state (systems/core), start gate respects grants (sim/systems). All state-mediated (P26).

## What to Change

### 1. Add explicit queue-admission actions for corpse/care contention

Add real short-lived queue actions for the queue-based Phase 1 domains:
- corpse contention queue join (for loot / bury on the corpse entity)
- care contention queue join (for heal on the patient entity)

These actions should:
- enqueue the actor into the target `ContentionQueue`
- update the actor's `ContentionIntents`
- commit through normal action world mutation, like `queue_for_facility_use`
- reject with structured failure when the queue is full or the actor is already queued/granted

### 2. Gate loot / bury / heal on matching contention grants

Before loot, bury, or heal can start on a contention-managed target:
- if no `ContentionPolicy` / `ContentionQueue` exists, behavior remains unchanged
- if contention state exists, the actor must hold the matching grant for that target + action
- missing or wrong grant should fail structurally with a clear contention rejection path

### 3. Add shared enqueue / intent-sync helper

Refactor or add a shared helper for queue admission that:
- enqueues into `ContentionQueue`
- synchronizes `ContentionIntents`
- preserves the existing facility queue behavior
- can be reused by the new corpse/care queue actions

### 4. Release grant and clear actor intent on action completion / abort

In loot / bury / heal commit and abort handlers:
- clear the matching grant if the actor still holds it
- remove the target entity from the actor's `ContentionIntents`
- leave unrelated queue membership untouched

### 5. Extend planner / search queue integration

Extend the existing queue-planning path so loot / bury / heal goals can select the new queue-join actions before the exclusive action step when contention queueing is available.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify — add queue actions, grant gating, grant/intent release)
- `crates/worldwake-systems/src/facility_queue_actions.rs` (modify — share enqueue / intent-sync helper if appropriate)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — synthesize corpse/care queue candidates)
- `crates/worldwake-ai/src/goal_model.rs` (modify — queue barrier semantics for new queue actions if needed)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — map new queue action defs to the existing queue op abstraction)
- `crates/worldwake-sim/src/action_payload.rs` (modify — add generic or domain-specific queue payload if needed)

## Out of Scope

- Attaching ContentionQueue to entities (S44GENCONSUB-007)
- Perception of contention (S44GENCONSUB-008)
- Golden tests (S44GENCONSUB-009)

## Acceptance Criteria

### Tests That Must Pass

1. Queue join on corpse target with room → waiter added and `ContentionIntents` updated
2. Queue join on care target with room → waiter added and `ContentionIntents` updated
3. Queue join on full queue → structured rejection
4. Loot / bury / heal without matching grant on contention-managed target → structured rejection
5. Loot / bury / heal with matching grant → action starts
6. Grant + `ContentionIntents` cleared on loot / bury / heal commit / abort
7. Search / planner can emit the lawful queue step for corpse / care contention goals
8. Existing suite: `cargo test --workspace`

### Invariants

1. Actions on entities without ContentionQueue are unaffected
2. Grant holder identity matches actor identity check (no spoofing)
3. Queue domains use an explicit queue/grant world process rather than hidden retry timing
4. `ContentionIntents` stays in sync with `ContentionQueue` for queue admission and exclusive completion

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` (tests) — queue admission, grant gating, grant release, intent cleanup
2. `crates/worldwake-ai/src/search/tests.rs` (tests) — planner/search emits corpse/care queue step when appropriate

### Commands

1. `cargo test -p worldwake-systems combat`
2. `cargo test -p worldwake-ai -- search`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed: 2026-04-04

- Added explicit queue-admission actions for contention-managed corpse and care targets in `crates/worldwake-systems/src/combat.rs`:
  - `queue_for_corpse_use`
  - `queue_for_care_target`
- Added shared generalized queue-admission helpers in `crates/worldwake-systems/src/facility_queue_actions.rs` so facility and corpse/care queue joins use the same authoritative enqueue and `ContentionIntents` sync path.
- Made `loot`, `bury`, and `heal` grant-gated on contention-managed targets, and cleared matching grants plus matching `ContentionIntents` on commit/abort.
- Extended AI queue integration in:
  - `crates/worldwake-ai/src/search/candidates.rs`
  - `crates/worldwake-ai/src/planner_ops.rs`
  - `crates/worldwake-ai/src/goal_model.rs`
  - `crates/worldwake-ai/src/failure_handling.rs`
  so corpse/care queue affordances expand to lawful queue steps and direct exclusive actions are suppressed without grants.

Deviations from original plan:

- The ticket was corrected before implementation from a narrow start-validation ticket into a broader queue-admission/runtime-contract ticket. FOUNDATIONS P8/P9/P20/P21 required lawful waiting state for queue-based contention domains instead of a rejection-only `StartFailed` path.
- `crates/worldwake-sim/src/action_payload.rs` did not need changes. Existing payload machinery was sufficient.
- The strongest honest AI proof surface was narrower than a full queue-first end-to-end plan assertion. Focused AI tests now prove queue-affordance expansion plus direct-action suppression without grants, while systems tests own the authoritative queue/grant mutation and cleanup proofs.

Verification:

- Passed `cargo test -p worldwake-systems combat`
- Passed `cargo test -p worldwake-ai -- search`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test -p worldwake-systems harvest_reservation_blocks_second_actor_and_abort_preserves_source`
- Passed `cargo test --workspace`
