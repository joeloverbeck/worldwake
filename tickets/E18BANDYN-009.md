# E18BANDYN-009: Golden test T22 — bandit camp destruction chain

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None — test only
**Deps**: E18BANDYN-003, archive/tickets/completed/E18BANDYN-004.md, E18BANDYN-005, E18BANDYN-006, E18BANDYN-007, E18BANDYN-008, E18BANDYN-010

## Problem

The brainstorming spec (T22) requires a golden end-to-end test demonstrating the full bandit camp destruction chain: camp with raiders → external attack → survivors flee → rally-point regrouping → new camp establishment → route belief aging → downstream economic behavior change. This test validates the emergent integration of all E18 deliverables and serves as the Phase 4 acceptance gate for the bandit dynamics epic.

## Assumption Reassessment (2026-03-29)

1. Golden test harness exists in `crates/worldwake-ai/tests/golden_harness/mod.rs`. It provides `GoldenHarness` with methods for world setup, agent creation, stepping ticks, and asserting state.
2. `PerceptionProfile` must be set on agents that need to observe events (CLAUDE.md: "Golden production tests require PerceptionProfile on agents that need to observe post-production output").
3. Decision traces (`h.driver.enable_tracing()`) and action traces (`h.enable_action_tracing()`) are available for debugging. Traces should be enabled during development but not left in committed test code unless the test explicitly asserts on trace data.
4. The test must demonstrate causal depth >= 4 across >= 3 subsystems (spec T22 requirement).
5. Pass threshold: "Within 5 in-world days, route safety and at least one downstream economic behavior must change because of the diaspora."
6. The test must set up: bandit camp with 3+ members, supplies, raid history, merchants with route beliefs, external attackers (guards/adventurers).
7. All E18 components must be integrated, but the policy contract is no longer correctly described as place-backed `BanditCampProfile`. After `E18BANDYN-010`, the golden chain should use active `BanditCamp` state on places plus `BanditFactionPolicy` on the faction entity.
8. This is a multi-subsystem integration test spanning: combat (E12), needs (E09), AI decisions (E13), production/trade (E10/E11), beliefs (E14/E15), and E18's new bandit dynamics.

## Architecture Check

1. A golden E2E test is the correct verification surface because: (a) the spec explicitly requires T22 as an integration test, (b) the behavior being tested is emergent from multiple interacting systems — unit tests cannot verify the full causal chain, (c) the golden harness provides deterministic replay and tracing for debuggability (FND-27).
2. The test must be designed so that each major phase is independently assertable, allowing diagnosis of which subsystem fails if the chain breaks.
3. No backwards-compatibility shims. Net-new test file.

## Verification Layers

1. Camp destruction → authoritative world state: `BanditCamp` component removed after all members die/flee and grace period expires
2. Survivor flight → action trace: Travel actions by surviving bandits
3. Rally-point navigation → decision trace: `RegroupWithFaction` goal selected by survivors with rally belief
4. New camp establishment → authoritative world state: `BanditCamp` component on new place after `EstablishCamp` commits
5. Route belief aging → derived query: `route_threat_estimate` decreases for former patrol routes over time
6. Economic behavior change → authoritative world state or decision trace: merchant changes route or resumes trade on formerly dangerous route
7. Causal depth → event-log analysis: trace chain from initial attack through >= 4 causal steps across >= 3 subsystems

## What to Change

### 1. Create golden test file

In `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs`:

#### Setup Phase
- Create a place graph: village — road — crossroads — forest path — bandit camp place — rally place
- Create bandit faction with `FactionData { purpose: Military }`
- Create 4 bandit agents with `MemberOf`, `CombatProfile`, `UtilityProfile`, `HomeostaticNeeds`, `PerceptionProfile`
- Create active `BanditCamp` on the camp place and the faction-scoped camp policy component on the bandit faction (with rally place set there)
- Create camp supplies container with food
- Create 2 merchant agents with `PerceptionProfile`, trade profiles, route knowledge
- Create 2 attacker agents (guards) with strong `CombatProfile`
- Set up initial beliefs: merchants have beliefs about bandit presence on forest path (from prior raids)

#### Phase 1: Initial State Validation
- Assert: camp exists, bandits are members, supplies present
- Assert: merchants avoid forest path (route threat estimate > 0)

#### Phase 2: Attack on Camp
- Move attackers to camp place
- Step ticks until combat resolves
- Assert: some bandits dead (`DeadAt`), some wounded, attackers may be wounded
- Assert: surviving bandits with high danger pressure generate `ReduceDanger` → flee

#### Phase 3: Survivor Flight and Regrouping
- Step ticks for travel duration
- Assert: survivors with rally-point belief travel toward rally place
- Assert: survivors without rally-point belief do NOT navigate to rally place
- Assert: dead bandits do NOT participate (no goals, no movement)

#### Phase 4: Camp Abandonment
- Step ticks past grace period
- Assert: `BanditCamp` component removed from original camp place
- Assert: `CampAbandoned` event emitted
- Assert: supply container still at original camp place (lootable)

#### Phase 5: New Camp Establishment
- Step ticks until enough survivors reach rally place
- Assert: `EstablishCamp` action starts when `min_regroup_count` met
- Step ticks for establishment duration
- Assert: new `BanditCamp` on rally place

#### Phase 6: Route Belief Aging and Economic Response
- Step ticks (simulating passage of time without new raids on old routes)
- Assert: merchants' beliefs about former patrol routes age/decay
- Assert: at least one merchant changes route decision or resumes trade activity on formerly dangerous route
- This satisfies T22 pass threshold: "route safety and at least one downstream economic behavior must change"

#### Phase 7: Causal Depth Verification
- Trace the event chain and verify causal depth >= 4 across >= 3 subsystems

## Files to Touch

- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (new — golden E2E test)

## Out of Scope

- Modifying any production code — this is a test-only ticket
- Guard response to bandit reports (E19)
- Belief propagation implementation changes — uses existing E14/E15 belief system
- Performance optimization of the golden test
- CLI integration for bandit camp display

## Acceptance Criteria

### Tests That Must Pass

1. Full T22 golden test passes end-to-end
2. Camp destruction leaves persistent aftermath (dead bodies, lootable supplies)
3. Survivors retain injuries, inventory, faction membership after flight
4. No respawn: member count only decreases or stays constant
5. Regrouping requires physical travel (multi-tick Travel actions observed)
6. Bandits without rally belief do NOT navigate to rally place
7. EstablishCamp requires minimum member count at rally place
8. Route beliefs age when no new attacks occur on former patrol routes
9. At least one merchant changes route/trade behavior due to belief changes (T22 pass threshold)
10. Causal depth >= 4 across >= 3 subsystems
11. Within 5 simulated in-world days (configurable tick count)
12. Existing golden tests unaffected: `cargo test -p worldwake-ai`
13. Existing suite: `cargo clippy --workspace`

### Invariants

1. FND-1 (Emergence): all behavior emerges from interacting systems, no scripted sequences in the test assertions
2. FND-4 (Persistent Identity): dead bandits stay dead, supplies conserved, bodies persist
3. FND-7 (Locality): rally-point knowledge via beliefs, not global queries from faction policy
4. FND-17 (Agent Symmetry): bandits use same action framework as attackers and merchants
5. FND-27 (Debuggability): causal chain is reconstructable from event log and traces
6. Conservation: `verify_live_lot_conservation` passes at each assertion checkpoint
7. Determinism: test produces identical results with same seed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` — full E2E golden test for bandit camp destruction chain

### Commands

1. `cargo test -p worldwake-ai golden_t22`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
4. `cargo build --workspace`
