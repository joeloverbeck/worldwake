# E16BFORLEGJURCON-005: Implement office force-control system (replace resolve_force_succession)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — office system in worldwake-systems
**Deps**: E16BFORLEGJURCON-001, E16BFORLEGJURCON-002, E16BFORLEGJURCON-003, E16BFORLEGJURCON-004

## Problem

The current `resolve_force_succession()` in `worldwake-systems/src/offices.rs` is a thin placeholder: if exactly one eligible agent is present after the vacancy period, they are installed. This must be replaced with the full state machine: explicit control tracking, contested state, departure clears control, uncontested hold period before installation. The old function is removed, not kept alongside (Principle 26).

## Assumption Reassessment (2026-03-22)

1. `resolve_force_succession()` still exists in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) and is still a placeholder: after the vacancy timer elapses it installs exactly one eligible present contender and otherwise does nothing. This remains the core production gap.
2. The force-control substrate is already live in `worldwake-core`, not merely pending from earlier tickets. `OfficeForceProfile`, `OfficeForceState`, `contests_office/contested_by`, `office_controller/offices_controlled`, and the `WorldTxn` helpers for claims/controllers already exist in [`crates/worldwake-core/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/offices.rs), [`crates/worldwake-core/src/world/social.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/social.rs), and [`crates/worldwake-core/src/world_txn.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs). The ticket scope is therefore system replacement plus focused tests, not substrate creation.
3. `PressForceClaim` and `YieldForceClaim` actions are also already implemented in [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs). This ticket should consume their authoritative `contests_office` state; it should not duplicate action-layer claim logic.
4. Current force-law timing still routes through the generic vacancy timer in `evaluate_office_succession()` via `OfficeData.succession_period_ticks`. That is now an architectural contradiction because `OfficeForceProfile` is the explicit force-law timing surface. This ticket owns removing that split authority for the authoritative force branch and consuming `uncontested_hold_ticks` as the installation gate. The profile’s grace fields remain explicit policy substrate but are not yet cleanly specified enough to implement here without inventing new contest provenance semantics.
5. Existing focused tests already cover the placeholder behavior in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs): `force_succession_installs_only_uncontested_eligible_present_agent`, `force_succession_blocks_when_multiple_contenders_are_present`, and `force_succession_trace_records_install_and_blocked_cases`. Those tests must be replaced or rewritten to assert the new controller/state-machine behavior rather than preserving the placeholder heuristic.
6. Mismatch corrected: the original ticket implied `InstitutionalClaim::ForceControl`, `InstitutionalBeliefKey::ForceControllerOf`, and controller-transition belief projection were already available. They are not present in [`crates/worldwake-core/src/institutional.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs), [`crates/worldwake-systems/src/perception.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/perception.rs), or the tell/consult record surfaces. This ticket should not silently invent a partial parallel belief path. Force-control institutional projection remains follow-up work aligned with E16c / `E16BFORLEGJURCON-006`.
7. Installation-side office-register writes already happen through the existing `WorldTxn::assign_office()` path, which supersedes the `OfficeRegister` entry for `InstitutionalClaim::OfficeHolder`. This ticket should continue to reuse `install_office_holder()`; it does not need a second register-write path for holder installation.
8. Closure boundary: this ticket completes authoritative force-office control in `worldwake-systems`. It owns per-tick `office_controller` / `OfficeForceState` transitions, dead-claimant cleanup, force-law timing authority migration, and installation through the existing holder-assignment path. It does not own AI affordances/planning, force-control institutional belief propagation, or guard/public-order responses.
9. Mismatch corrected: the current `remove_force_claim` world/txn path requires a live claimant, so stale dead claims cannot be pruned through the existing helper set alone. This ticket may add the minimal authoritative cleanup helper in `worldwake-core` required for system-side dead-claim pruning.
10. N/A — not an AI regression ticket yet. Current `worldwake-ai` still hard-omits force-law political candidates, so this change should not assert planner behavior in scope.
11. N/A — no special action-start failure surface beyond ordinary authoritative mutation checks.
12. Mismatch corrected: `crates/worldwake-ai/tests/golden_offices.rs` currently includes a force-law golden scenario that encodes the placeholder heuristic ("sole living eligible contender installs without explicit claim"). That golden is no longer correct under the target architecture and must be updated alongside this ticket.
13. Force installation should be gated by explicit control continuity (`OfficeForceState`) and `OfficeForceProfile::uncontested_hold_ticks`. `SuccessionLaw::Force` should no longer rely on the generic `OfficeData.succession_period_ticks` vacancy timer once this ticket is complete.

## Architecture Check

1. Replacing the presence-count heuristic with an explicit controller/state machine is materially better architecture because it stores the real causal facts the rest of the simulation needs: who is contesting, who physically controls, and how long uncontested control has persisted. That is durable substrate; the current heuristic is not.
2. `resolve_force_succession()` should be removed entirely, not wrapped. Keeping both paths would preserve a hidden alias architecture where force offices can still bypass `OfficeForceState`, which directly violates Principle 26.
3. Force-law timing must become law-specific. Leaving `SuccessionLaw::Force` dependent on `OfficeData.succession_period_ticks` would keep two competing timing authorities for one concept and make future extensions brittle.
4. This ticket should stay disciplined about layering. The clean move is to finish the authoritative state machine first, then let later institutional-belief work project that state through one canonical path. Adding ad-hoc force-control metadata here without the full institutional substrate would create a second-class architecture we would have to delete later.

## Verification Layers

1. Force controller establishment / removal -> authoritative relation state (`office_controller`) plus `OfficeForceState` component fields
2. Contest transition timing -> authoritative `OfficeForceState.contested_since` / `last_uncontested_tick`
3. Dead claimant pruning -> authoritative `contests_office` / `contested_by` relation state
4. Hold-based installation -> authoritative `office_holder` relation plus `OfficeForceState` reset/cleanup
5. Installation event visibility -> committed event log record from the existing holder-assignment path
6. Installation office-register history -> `RecordKind::OfficeRegister` data written by `WorldTxn::assign_office()`
7. Placeholder removal / trace shape -> focused politics trace assertions in `worldwake-systems` tests

## What to Change

### 1. Replace `resolve_force_succession()` in `offices.rs`

Remove the old function entirely. Add new per-tick force-control logic:

For each force office with `OfficeForceProfile` + `OfficeForceState`:
0. Read force installation timing from `OfficeForceProfile::uncontested_hold_ticks`; do not use `OfficeData.succession_period_ticks` for the force branch.
1. Gather force claimants from `contested_by(office)`, prune dead claimants, and filter present live claimants at jurisdiction.
2. **Departure rule**: if current `office_controller` is not present/alive at jurisdiction, `clear_office_controller(office)` and reset control continuity.
3. Derive situation:
   - **No present claimants**: clear controller, preserve recognized holder if still vacant
   - **One present claimant, office uncontrolled**: establish controller and start continuity
   - **Same sole controller remains**: preserve continuity and advance `last_uncontested_tick`
   - **Different sole claimant after control break**: replace controller and restart continuity
   - **Multiple present claimants**: clear controller and mark contested continuity
4. **Installation gate**: if one controller remains uncontested for the required hold period, install through the existing `install_office_holder()` helper and clear all active force claims for that office.

### 2. Add the minimal dead-claim cleanup substrate if needed

If the current `World` / `WorldTxn` force-claim API still requires a live claimant, add the smallest authoritative cleanup helper needed to clear stale dead claims during system maintenance. Use one canonical cleanup path; do not add a compatibility alias layer.

## Files to Touch

- `crates/worldwake-systems/src/offices.rs` (modify — replace force placeholder with controller/state machine and focused tests)
- `crates/worldwake-core/src/world/social.rs` and/or `crates/worldwake-core/src/world_txn.rs` (only if needed for dead-claim cleanup substrate)
- `crates/worldwake-sim/src/politics_trace.rs` (if trace outcomes need to reflect controller-state transitions)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` and affected golden suites (if stale force-law test scaffolding still encodes placeholder auto-install behavior)

## Out of Scope

- Public order degradation from contested offices — deferred to E19
- Guard responses to coups — deferred to E19
- Patrol escalation around disputed seats — deferred to E19
- Additional installation gates (guard acquiescence, faction support thresholds) — deferred to E19
- Interpreting `vacancy_claim_grace_ticks` / `challenger_presence_grace_ticks` into authoritative contest semantics — deferred to a follow-up ticket with explicit behavioral coverage
- AI integration (affordances, planner ops) — E16BFORLEGJURCON-007/008
- Institutional belief queries — E16BFORLEGJURCON-006
- `InstitutionalClaim::ForceControl` / `InstitutionalBeliefKey::ForceControllerOf` propagation — deferred until the institutional belief surface exists

## Acceptance Criteria

### Tests That Must Pass

1. One uncontested claimant becomes controller but NOT immediately recognized holder
2. Controller continuity breaks when another claimant arrives
3. Controller continuity breaks when controller dies
4. Controller loses control immediately upon leaving jurisdiction
5. Returning to jurisdiction after departure restarts control clock (`control_since` resets)
6. After `uncontested_hold_ticks`, sole controller with no other live claimants is installed as `office_holder`
7. Multiple simultaneous claimants keep office contested and block installation
8. `office_controller` and `office_holder` never diverge into invalid multiplicity (both 1:1)
9. Dead claimants are removed from `contests_office`
10. Installation clears all active force claims for the office
11. Installation emits the existing visible political holder-installation event
12. Installation appends/supersedes the office-holder entry in the office register via the existing `WorldTxn::assign_office()` path
13. The stale golden force-succession scenario is updated to match explicit-claim architecture
14. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. No office has more than one recognized holder (`office_holder` is 1:1)
2. No office has more than one current controller (`office_controller` is 1:1)
3. Controller and recognized holder are distinct concepts stored in separate relations
4. Physical presence at jurisdiction is required to hold control; departure clears control immediately (Principle 8)
5. No hidden "time at place" heuristic substitutes for stored control state
6. The provisional `resolve_force_succession` is fully removed (Principle 26)
7. All values remain deterministic and integer-based
8. `SuccessionLaw::Force` no longer consults `OfficeData.succession_period_ticks`; force timing authority lives on `OfficeForceProfile`
9. No ad-hoc parallel belief path for force control is introduced before the institutional claim surface exists
10. Golden coverage no longer preserves the placeholder "unclaimed sole contender auto-installs" rule

## Tests

### New/Modified Tests

1. `offices::tests::force_control_establishes_controller_before_installation`
Rationale: proves the new architecture distinguishes controller from recognized holder and stores continuity before installation.
2. `offices::tests::force_control_contest_clears_controller_and_sets_contested_state`
Rationale: proves multiple live present claimants produce explicit contested state rather than a silent no-op.
3. `offices::tests::force_control_departure_or_death_clears_controller_and_prunes_dead_claims`
Rationale: proves control depends on physical presence and that dead claimants cannot keep authoritative contest state.
4. `offices::tests::force_control_installation_requires_uncontested_hold_and_clears_claims`
Rationale: proves the hold gate, holder installation, claim cleanup, committed event, and office-register write all happen through the canonical path.
5. `offices::tests::force_control_trace_reflects_controller_state_machine`
Rationale: proves the politics trace reports the new force-control flow instead of the removed placeholder install/block heuristic.
6. `golden_offices::{golden_force_succession_sole_eligible, golden_force_succession_deterministic_replay}`
Rationale: rewrites the stale force-office golden to use explicit force claims instead of placeholder auto-install.
7. `golden_emergent::{golden_combat_death_triggers_force_succession, golden_combat_death_triggers_force_succession_replays_deterministically}`
Rationale: updates the combat-driven political golden to seed the explicit force claim required by the new authoritative architecture.

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-22
- Actual changes:
  - replaced the placeholder force succession heuristic with explicit controller-state resolution in `worldwake-systems`
  - force offices now require explicit claims, track controller continuity in `OfficeForceState`, install holders only after `uncontested_hold_ticks`, and clear claims/controller state on installation
  - added the minimal core-side stale-claim cleanup support needed to prune dead claimants authoritatively
  - extended politics tracing to expose controller-establish / maintain / contested outcomes
  - updated stale golden harness setup and force-law golden scenarios that were still encoding auto-install without explicit claims
- Deviations from original plan:
  - this ticket intentionally did not implement `InstitutionalClaim::ForceControl` or force-controller belief projection because that surface is not live yet
  - `vacancy_claim_grace_ticks` and `challenger_presence_grace_ticks` remain explicit profile substrate but were left for follow-up work rather than inventing underspecified semantics here
- Verification results:
  - `cargo test -p worldwake-systems` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
