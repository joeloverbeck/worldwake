# E16BFORLEGJURCON-009: Force legitimacy golden E2E tests

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Golden coverage, docs inventory, and force-law planner/runtime fixes discovered during verification
**Deps**: `specs/E16b-force-legitimacy-and-jurisdiction-control.md`

## Problem

The explicit E16b force-legitimacy architecture is implemented, but golden E2E coverage still under-represents the end-to-end force-control lifecycle. Lower layers already prove the state machine, action validation, and belief plumbing in isolation; this ticket should add only the missing golden scenarios that validate cross-layer behavior without duplicating focused tests.

## Assumption Reassessment (2026-03-22)

1. The ticket's original dependency framing was stale. The live code already includes the explicit E16b substrate:
   - `OfficeForceProfile` and `OfficeForceState` in `crates/worldwake-core/src/offices.rs`
   - `contests_office` / `contested_by` and `office_controller` / `offices_controlled` in `crates/worldwake-core/src/relations.rs`
   - `PressForceClaim` / `YieldForceClaim` payloads in `crates/worldwake-sim/src/action_payload.rs`
   - force-claim action handlers in `crates/worldwake-systems/src/office_actions.rs`
   - explicit force control/install resolution in `crates/worldwake-systems/src/offices.rs`
   - force-control institutional projection in `crates/worldwake-systems/src/perception.rs`
   - force-law `ClaimOffice` AI surfaces in `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/goal_model.rs`, and `crates/worldwake-ai/src/planner_ops.rs`
2. The ticket's current statement that prior tickets “must all be integrated before these goldens can run” is no longer true. The relevant architecture is already live and testable.
3. Existing golden force coverage is real but narrow:
   - `golden_force_succession_sole_eligible` and `golden_force_succession_deterministic_replay` in `crates/worldwake-ai/tests/golden_offices.rs`
   - `golden_combat_death_triggers_force_succession` and replay companion in `crates/worldwake-ai/tests/golden_emergent.rs`
   These prove installation after an already-seeded force claim and a combat-driven vacancy chain, but they do not yet prove the ordinary AI `PressForceClaim` path, explicit contested resolution via `YieldForceClaim`, or force-control belief locality plus Tell relay.
4. Focused and integration coverage already proves several lower-layer invariants, so those do not need separate golden scenarios unless the cross-layer chain itself is the contract:
   - action/authoritative claim entry and hostility: `office_actions::tests::press_force_claim_commit_adds_claim_and_hostility_against_incumbent`
   - affordance validation: `office_actions::tests::press_force_claim_affordance_*`, `yield_force_claim_affordance_*`
   - control-state machine and departure clearing: `offices::tests::force_control_establishes_controller_before_installation`, `force_control_contest_clears_controller_and_sets_contested_state`, `force_control_departure_or_death_clears_controller_and_prunes_dead_claims`, `force_control_installation_requires_uncontested_hold_and_clears_claims`
   - witness projection and Tell relay: `perception::tests::political_event_projects_force_control_claim_for_witness`, `tell_actions::tests::tell_commit_relays_force_control_claims`
   - force-law AI candidate/planner surfaces: `candidate_generation::tests::political_candidates_emit_claim_for_force_law_offices_and_keep_support_suppressed`, `candidate_generation::tests::political_candidates_emit_claim_for_enemy_held_force_office`, `goal_model::tests::claim_office_force_law_*`, `agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning`
5. The old scenario list overstated missing coverage and mixed architectural layers together. The missing gap is specifically golden/E2E coverage for:
   - AI candidate generation -> plan search -> `press_force_claim` action execution -> controller establishment -> delayed installation
   - contested force control -> explicit `yield_force_claim` resolution -> delayed installation
   - same-place witness belief acquisition and remote ignorance until Tell for `InstitutionalBeliefKey::ForceControllerOf`
6. `vacancy_claim_grace_ticks` and `challenger_presence_grace_ticks` are still present in `OfficeForceProfile` but are not used by `resolve_force_succession()` in `crates/worldwake-systems/src/offices.rs`. This ticket must not invent goldens for those unused fields.
7. The current force-law timing contract in production is driven by `OfficeForceProfile.uncontested_hold_ticks`; the live `seed_office()` test helper maps force-law `succession_period_ticks` into that profile for convenience in goldens. Ticket language must describe the authoritative timing source accurately and avoid implying that support-law `succession_period_ticks` is still the force-resolution authority.
8. No production contradiction was found during reassessment. The current architecture is cleaner than the old shortcut and already aligns with the E16b spec: physical control is explicit state, controller identity is stored in a relation instead of duplicated in a component, and remote knowledge still travels through belief propagation rather than omniscient queries. This ticket should strengthen goldens, not reopen production design.

## Architecture Check

1. The live explicit force-control architecture is more robust than the old provisional shortcut because it separates claimants, current controller, and recognized office holder into concrete world state instead of collapsing them into a hidden timer.
2. The clean next step is more E2E proof, not more production indirection. Lower-layer tests already lock down the force-control state machine; adding redundant production abstraction here would not improve extensibility.
3. Golden scope should stay narrow and architectural:
   - prove the ordinary AI-driven `ClaimOffice -> PressForceClaim` path exists end-to-end
   - prove contested force control resolves through explicit public claims and `YieldForceClaim`, not hidden tie-breaking
   - prove `ForceControllerOf` knowledge remains local until it is physically relayed
4. Departure-reset and hostility-after-claim remain important invariants, but the current architecture already proves them precisely in focused tests. Adding separate goldens for those same single-boundary facts would add suite cost without increasing architectural confidence proportionally.
5. No backward-compatibility shims or alias paths should be introduced. If the old seeded-claim golden is rewritten, it should be rewritten to the live AI/action path rather than preserved as a parallel oracle.

## Verification Layers

1. Force-law candidate generation for an eligible visible office -> decision trace and existing focused `candidate_generation` coverage
2. Planner selects the force-law office-claim path -> decision trace (`GoalKind::ClaimOffice`, selected plan / selected op surface)
3. `PressForceClaim` and `YieldForceClaim` actually execute -> action trace
4. Controller establishment, contested clearing, and final installation -> authoritative world state plus politics trace
5. Same-place force-control belief acquisition by witnesses -> institutional belief store state (optionally institutional knowledge trace if useful)
6. Remote ignorance before Tell and acquisition after Tell -> belief store state plus action trace for committed `tell`
7. Determinism -> replay companion world hash + event-log hash equality

## What to Change

### 1. Replace the old seeded-claim force golden with a real AI-driven force-claim golden

Add or rewrite the current force-law office golden so that it proves the ordinary path:

- agent starts eligible, alive, local, and informed about a vacant `SuccessionLaw::Force` office
- decision trace shows `GoalKind::ClaimOffice { office }` is generated
- action trace shows a committed `press_force_claim`
- no `declare_support` commit occurs
- politics trace / authoritative state show controller establishment first and installation only after the uncontested hold delay

### 2. Add a contested-resolution golden

Add a dedicated golden where:

- two claimants publicly press force claims for the same office
- office becomes contested and no installation occurs while both claims remain active
- one claimant explicitly yields through `yield_force_claim`
- the remaining claimant becomes sole controller and installs only after the hold delay

This should use the real action path, but it does not need autonomous AI on both actors if human-requested actions produce a cleaner and more stable E2E proof of the contested lifecycle.

### 3. Add a force-control locality + Tell golden

Add a golden where:

- a claimant publicly establishes force control in the presence of a same-place witness
- the witness acquires `ForceControllerOf { office }`
- a remote listener does not acquire that belief from co-existence or scheduler progression alone
- a committed `tell` relays the force-control belief to the remote listener

The contract is locality of institutional knowledge for force control, not office-holder installation.

### 4. Add deterministic replay companions

Each new scenario should have a replay companion unless it is folded into an already-existing replay helper.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs`
- `docs/generated/golden-e2e-inventory.md` if test inventory changes
- `docs/generated/golden-scenario-map.md` if scenario inventory changes
- `docs/golden-e2e-scenarios.md` only if the inventory regeneration workflow reports a docs mismatch that is not already generated

## Out of Scope

- Production redesign of force legitimacy, force control, or succession timing
- New semantics for `vacancy_claim_grace_ticks` or `challenger_presence_grace_ticks`
- Separate departure-reset or hostility-only goldens for invariants already fully covered by focused tests
- Guard response, public order, or coup suppression mechanics deferred to later political/public-order work

## Acceptance Criteria

### Tests That Must Pass

1. Golden: AI-driven force claim commits `press_force_claim` and later installs through the force-control lifecycle
2. Golden: contested force control blocks installation until an explicit `yield_force_claim` resolves the contest
3. Golden: force-control belief stays local until a committed `tell` relays it
4. Replay companions for each new scenario
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Force-law goldens must not rely on `declare_support`
2. Controller establishment and final office holding must remain separate authoritative boundaries
3. Force-control timing assertions must follow `OfficeForceProfile.uncontested_hold_ticks`
4. Force-control knowledge must obey Principle 7 and Principle 13 locality rules
5. Old seeded-claim golden expectations must be updated or removed if they no longer prove the live architectural path

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_offices.rs` `golden_force_claim_ai_installation` or equivalent renamed replacement
Rationale: proves the ordinary AI `ClaimOffice -> PressForceClaim` path instead of seeding the force claim relation out of band.
2. `crates/worldwake-ai/tests/golden_offices.rs` contested force-control golden plus replay companion
Rationale: proves the public-claim contested state machine and explicit `yield_force_claim` resolution at E2E level.
3. `crates/worldwake-ai/tests/golden_offices.rs` force-control locality/Tell golden plus replay companion
Rationale: proves same-place witness acquisition and remote ignorance until Tell for `ForceControllerOf`.

### Commands

1. `cargo test -p worldwake-ai --test golden_offices -- --list`
2. `cargo test -p worldwake-ai --test golden_offices`
3. `cargo test -p worldwake-ai`
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

Completion date: 2026-03-22

What actually changed:
- Reassessed the ticket against live E16b code and narrowed the intended gap to three missing goldens: AI-driven `PressForceClaim`, contested `YieldForceClaim` resolution, and force-control locality plus Tell relay.
- Replaced the stale seeded-force golden expectations with new scenarios in `crates/worldwake-ai/tests/golden_offices.rs`:
  - `golden_force_claim_ai_installation`
  - `golden_force_claim_ai_installation_replays_deterministically`
  - `golden_contested_force_claim_resolves_after_yield`
  - `golden_contested_force_claim_resolves_after_yield_replays_deterministically`
  - `golden_force_control_locality_requires_tell`
  - `golden_force_control_locality_requires_tell_replays_deterministically`
- Added focused force-law regression coverage in `crates/worldwake-ai/src/goal_model.rs` for payload override behavior.
- Added focused affordance coverage in `crates/worldwake-systems/src/office_actions.rs` proving force-claim affordances key off office jurisdiction rather than office co-location belief.
- Updated `docs/golden-e2e-scenarios.md` and regenerated the golden inventory docs so the scenario catalog matches the implemented force-law goldens.

Deviations from original plan:
- The ticket did not remain test-only. While wiring the new goldens, two real production boundary bugs were exposed and fixed:
  - `PlanningState` / `PlanningSnapshot` were dropping full `OfficeData`, so force-law planning lost office semantics inside the planner snapshot.
  - `planner_ops::build_semantics_table()` did not classify `press_force_claim` / `yield_force_claim` because those action defs use payload overrides with `ActionPayload::None` at registration time.
- `goal_model` also needed a law-aware guard so force-law `ClaimOffice` no longer synthesizes `declare_support`, and now explicitly synthesizes `press_force_claim`.
- The strengthened existing `agent_tick` force-law trace assertion was kept because it passes once the planner semantics bug is fixed.

Verification results:
- `cargo test -p worldwake-ai --test golden_offices`
- `cargo test -p worldwake-ai`
- `cargo test -p worldwake-systems office_actions::tests::press_force_claim_affordance_uses_office_jurisdiction_not_believed_office_place -- --exact`
- `python3 scripts/golden_inventory.py --write --check-docs`
- `cargo test --workspace`
- `cargo clippy --workspace`
