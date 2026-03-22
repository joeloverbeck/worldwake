**Status**: PENDING

# S16b: Force-Legitimacy Cross-System Emergence Golden E2E Suites

## Summary

Add 3 cross-system emergence golden tests to `golden_emergent.rs` that prove E16b's force-legitimacy mechanisms participate in emergent multi-system chains. Currently, E16b force-control tests live in `golden_offices.rs` (Scenarios 19-21) and exercise force-control in relative isolation, and `golden_emergent.rs` Suite 5 chains combat->death->force succession. These new scenarios prove that travel, hostility, and institutional belief systems interact with force-legitimacy through shared state (Principle 1) to produce outcomes no single system orchestrates.

## Phase

Phase 3: Information & Politics (post-E16b)

## Crate

`worldwake-ai` (golden tests only -- no new system code)

## Dependencies

- E16b (force-legitimacy -- `PressForceClaim`, `YieldForceClaim`, `OfficeForceProfile`, `OfficeForceState`, `office_controller` relations, hostility-on-claim, `InstitutionalClaim::ForceControl`)
- E14 (perception/belief system -- belief boundary, social observation, Tell)
- E16c (institutional beliefs -- `InstitutionalClaim` types, institutional belief storage, belief projection from witnessed political events)
- E12 (combat -- hostility relations infrastructure)
- E07/E08 (action framework, scheduler -- travel actions)
- S13 (political emergence golden suites -- establishes the emergent test patterns in `golden_emergent.rs`)

## Scenarios

### Suite 10: Force Controller Departure Enables Rival Claim

**File**: `golden_emergent.rs`
**Systems exercised**: Travel (controller departure), Force-Control State Machine (departure detection, control clearing), AI (candidate generation for vacant force office, plan search for PressForceClaim), action tracing, politics tracing, deterministic replay
**Principles proven**: P1 (maximal emergence -- travel consequence cascades into political domain), P8 (travel occupancy as a real dampener -- leaving jurisdiction has political cost), P10 (no positive feedback without dampener -- physical presence requirement prevents remote force control)

**Setup**:
- Force-law office ("War Chief") at VillageSquare, succession_period=5, no eligibility rules
- Agent A ("Controller"): human-controlled, at VillageSquare. Has pressed force claim and established as `office_controller` (pre-seeded via `add_force_claim` + enough ticks for control establishment, or directly set `office_controller` relation if harness supports it).
- Agent B ("Rival"): AI-controlled, sated, enterprise_weight=pm(800), at VillageSquare. Has perception profile, institutional belief about the office with A as force controller (`ForceControl { controller: Some(A), contested: false }`). Known office at VillageSquare.
- Controller A is issued a travel action to leave VillageSquare (human input: `RequestAction` for travel to a different place).

**Emergent behavior proven**:
- A begins travel away from VillageSquare.
- Force-control system detects A's departure from jurisdiction, clears `office_controller` relation.
- B's AI observes the now-uncontrolled force-law office (via perception of the control-clearing event or updated affordance enumeration).
- B generates `ClaimOffice` goal, plans `PressForceClaim`, and executes.
- B becomes `office_controller` and, after uncontested hold, installs as `office_holder`.
- No orchestrator connects travel to politics -- the chain emerges from physical-presence state.

**Assertion surface**:
1. Action trace: A's `travel` action starts/commits before B's `press_force_claim` (travel departure is the causal trigger)
2. Authoritative state: `office_controller(office) == None` after A departs (before B claims)
3. Politics trace: `ForceControllerCleared` (or equivalent outcome variant) when A leaves, `ForceControllerEstablished { controller: B }` later
4. Authoritative state: B becomes `office_holder` after uncontested hold
5. Decision trace: B's tick-0 (or pre-departure tick) should NOT have ClaimOffice as selected goal (office is controlled); B's post-departure tick SHOULD generate and select ClaimOffice
6. Negative: no `declare_support` commits
7. Determinism: replay companion

**Intended branch**: Controller departure -> control cleared -> rival AI claims vacant force office.
**Lawful competing affordances**: B could have other goals (needs-based), but enterprise_weight=pm(800) and sated needs ensure ClaimOffice dominates.
**Scenario isolation**: B's needs are sated so no survival-driven goals compete. Only one rival is present to keep the scenario focused on departure->claim, not contested dynamics.

---

### Suite 11: Force Claim Creates Hostility Witnessed and Propagated

**File**: `golden_emergent.rs`
**Systems exercised**: PressForceClaim action handler (hostility creation as side effect), Force-Control State Machine, Perception (witness observes political event), Institutional Belief Projection (ForceControl belief from witnessed event), Travel, Social Tell (belief transfer), Belief Store (remote belief update), action tracing, deterministic replay
**Principles proven**: P1 (maximal emergence -- political violence creates social consequence that propagates physically), P7 (information locality -- force-control knowledge arrives via physical carrier), P9 (outcomes leave aftermath -- force claim creates persistent hostility), P13 (knowledge travels physically -- witness must carry and Tell the information)

**Setup**:
- Force-law office ("War Chief") at VillageSquare, succession_period=5, no eligibility rules
- Agent A ("Incumbent"): human-controlled, installed as `office_holder` at VillageSquare. Perception profile.
- Agent B ("Challenger"): human-controlled, at VillageSquare. Issues `PressForceClaim` against the office.
- Agent C ("Witness"): AI-controlled, social_weight=pm(600), low enterprise_weight, at VillageSquare. Perception profile. Tell profile. Has explicit entity beliefs about A, B, and the office (for perception to fire).
- Agent D ("Remote Listener"): at a remote place (e.g., BanditCamp). Perception profile. Tell profile for reception. Has no institutional belief about the office initially.

**Emergent behavior proven**:
- Phase 1: B presses force claim against the office where A is incumbent.
  - The `press_force_claim` handler creates `hostile_to(B, A)` relation as a side effect (verified at `office_actions.rs:820`).
  - C, co-located, perceives the political event and acquires `ForceControl { office, controller: Some(B), contested: false, effective_tick }` institutional belief via `force_control_claims_for_event()` (perception.rs:491-563).
- Phase 2: C travels to BanditCamp where D resides.
  - C's AI generates `ShareBelief` goal (social weight drives this).
  - C tells D about the office entity, transferring the `ForceControl` institutional belief.
- Phase 3: D's institutional belief store now contains `ForceControllerOf { office }` with `controller: Some(B)`, learned via Tell.
  - D's belief transitions from `Unknown` to `Certain` (or appropriate confidence level).

**Assertion surface**:
1. Authoritative state (Phase 1): `hostile_to(B, A)` relation exists after `press_force_claim` commits. This is the key cross-system side effect.
2. Action trace: `press_force_claim` committed by B
3. Institutional belief (Phase 1): C's belief store contains `ForceControllerOf { office }` with controller=B after perception fires
4. Action trace: C commits `travel` to remote place, then `tell` to D
5. Institutional belief (Phase 3): D's belief store transitions from `Unknown` -> populated with force-control knowledge
6. Decision trace: C generates `ShareBelief` candidate containing the office entity
7. Negative: D has no force-control belief before C's tell commits
8. Determinism: replay companion

**Intended branch**: Force claim -> hostility created + belief projected -> witness travels -> Tell propagates political knowledge to remote agent.
**Lawful competing affordances**: C could generate other social goals, but social_weight=pm(600) and focused Tell profile ensure office-related Tell dominates. C has no political ambition (low enterprise_weight).
**Scenario isolation**: Only one force claim (no contested state). The focus is on the hostility side effect and belief propagation chain, not on contest resolution. D is passive (no enterprise weight) -- the contract is belief arrival, not downstream political action.

**Why this is distinct from existing coverage**:
- Suite 5 (combat->death->succession): Pre-seeds hostility; doesn't test hostility as an emergent side effect of force claim.
- Scenario 21 (force-control belief locality): Tests uncontested belief propagation but not hostility creation. Only involves witness + listener at the same place before witness travels.
- This suite proves the FULL chain: political action -> social consequence (hostility) + information (belief) -> physical transport -> remote knowledge update.

---

### Suite 12: Contested Force State Propagates Through Belief System

**File**: `golden_emergent.rs`
**Systems exercised**: PressForceClaim (two claimants), Force-Control State Machine (contested state detection), Perception (witness observes contested political event), Institutional Belief Projection (ForceControl belief with `contested: true`), Travel, Social Tell, Belief Store (remote belief with contested flag), action tracing, deterministic replay
**Principles proven**: P1 (maximal emergence -- political instability is concrete information flowing through the world), P3 (concrete state -- contested is a bool derived from explicit claimant roster, not an abstract instability score), P7 (information locality -- contested state reaches remote agents only through physical carriers), P13 (knowledge provenance -- contested belief arrives with source attribution)

**Setup**:
- Force-law office ("War Chief") at VillageSquare, succession_period=5, no eligibility rules
- Agent A ("Claimant Alpha"): human-controlled, at VillageSquare. Issues `PressForceClaim`.
- Agent B ("Claimant Beta"): human-controlled, at VillageSquare. Issues `PressForceClaim`.
- Agent C ("Witness"): AI-controlled, social_weight=pm(600), low enterprise_weight, at VillageSquare. Perception profile (institutional_memory_capacity sufficient). Tell profile.
- Agent D ("Remote Listener"): at a remote place (e.g., OrchardFarm). Perception profile. Tell profile for reception. No initial institutional belief about the office.

**Concrete setup math for contested state**:
- Both A and B press force claims in consecutive ticks (or same tick if scheduling permits).
- The force-control system sees 2+ active claimants -> sets `OfficeForceState.contested_since` to the tick when the second claim arrives.
- No `office_controller` is established while the office is contested (multiple claimants, no sole controller).
- The political event emitted when contested state activates carries `contested: true` in its `ForceControl` institutional claim metadata.
- `force_control_claims_for_event()` at `perception.rs:551` reads `contested = projection.contested.unwrap_or(false)` -- this must resolve to `true` for the contested event.

**Emergent behavior proven**:
- Phase 1: A presses force claim -> becomes sole controller (uncontested). B then presses force claim -> office transitions to contested state.
  - `office_controller` is cleared (no sole controller during contest).
  - `OfficeForceState.contested_since` is set.
  - Political event emitted with `ForceControl { contested: true }`.
- Phase 2: C perceives the contested political event. C's institutional belief for `ForceControllerOf { office }` now has `contested: true`.
- Phase 3: C travels to remote place. C tells D about the office.
  - D's institutional belief store receives `ForceControl` with `contested: true`.
  - D's belief reflects the contested state -- not just "someone controls it" but "it is actively disputed."

**Assertion surface**:
1. Authoritative state (Phase 1): `office_controller(office) == None` while contested (no sole controller)
2. Authoritative state: `OfficeForceState` for the office has `contested_since.is_some()` after both claims
3. Politics trace: `ForceContested` outcome (or equivalent variant) when second claim arrives
4. Institutional belief (Phase 2): C's belief for office has `contested == true` (read via `InstitutionalBeliefRead`)
5. Action trace: C commits `travel` then `tell` to D
6. Institutional belief (Phase 3): D's belief for office has `contested == true` after tell
7. Negative: D has `Unknown` force-control belief before C's tell commits
8. Negative: `office_holder` is NOT set during the contested phase (no installation while contested)
9. Determinism: replay companion

**Intended branch**: Two force claims -> contested state -> witness learns contested belief -> travels -> tells remote agent -> remote agent learns contested state.
**Lawful competing affordances**: C could tell about other beliefs, but the office entity with a recently changed institutional state should rank high for ShareBelief. C has no political ambition.
**Scenario isolation**: Two human-controlled claimants keep the contest deterministic. Neither yields during the test -- the focus is on belief propagation of the contested flag, not contest resolution.

**Why this is distinct from existing coverage**:
- Scenario 20 (contested force claim resolution): Tests two claimants and yield mechanics, but never tests belief propagation of contested state.
- Scenario 21 (force-control belief locality): Only tests `contested: false` (single claimant, uncontested).
- Suite 11 (above): Tests hostility + uncontested belief propagation.
- This suite proves the contested flag specifically: `contested: true` is concrete information that propagates through the world, not an invisible system-internal flag.

## Implementation Notes

### Harness Helpers to Reuse
- `seed_agent()`, `seed_agent_with_recipes()` -- agent creation (`golden_harness/mod.rs`)
- `seed_office()` -- force-law office creation (`golden_harness/mod.rs`)
- `set_agent_perception_profile()`, `default_perception_profile()` -- perception setup
- `set_agent_tell_profile()`, `focused_accepting_tell_profile()` -- social Tell setup
- `seed_known_office_at_place()`, `seed_office_holder_belief()`, `seed_force_controller_belief()` -- institutional belief seeding
- `seed_actor_local_beliefs()`, `seed_actor_beliefs()` -- entity belief seeding
- `enterprise_weighted_utility()`, `social_weighted_utility()` -- utility profile factories
- `add_hostility()` -- hostility relation seeding (NOT needed for Suite 11 -- the point is hostility emerges from action)
- `new_txn()`, `commit_txn()` -- transaction helpers for pre-seeding `add_force_claim` etc.
- `lethal_combat_attacker_profile()`, `fragile_office_holder_profile()` -- combat profiles (from Suite 5 pattern)
- `give_commodity()` -- commodity seeding
- `GoldenHarness::enable_action_tracing()`, `enable_politics_tracing()`, `driver.enable_tracing()` -- trace enablement

### New Helpers Potentially Needed
- A helper to pre-seed `office_controller` relation directly (or simulate control establishment by running enough ticks after a force claim) for Suite 10. Check if `seed_force_controller()` or similar exists; if not, use `txn.set_office_controller()` directly.
- Verify that `ForceControllerCleared` (or similar) exists as a `OfficeSuccessionOutcome` variant for politics trace assertion in Suite 10. If not, the assertion should use authoritative state (`office_controller == None`) instead.

### Assertion Pattern Alignment
All suites follow `docs/golden-e2e-testing.md`:
- Prefer authoritative state for durable outcomes (office_holder, office_controller, hostility relations, institutional beliefs)
- Use action traces for lifecycle ordering (travel before claim, claim before tell)
- Use decision traces for AI reasoning (ClaimOffice generation, ShareBelief candidate selection)
- Use politics traces for force-control state machine transitions
- Deterministic replay companions for all suites
