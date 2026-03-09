# E22: Scenario Integration & Soak Tests

## Epic Summary
Implement full scenario integration tests covering all 4 exemplar scenarios, property tests, soak tests, and acceptance criteria validation.

## Phase
Phase 4: Group Adaptation, CLI & Verification (final epic)

## Crate
Workspace-level integration tests (`tests/` directory)

## Dependencies
- E18 (bandit dynamics)
- E19 (guard patrol)
- E20 (companion behaviors)
- E21 (CLI and human control)

## Deliverables

### World Setup
Per spec sections 4.1-4.2:
- 12-20 places as defined in E02 prototype builder
- 25-40 NPCs with roles:
  - 1 ruler
  - 2-3 succession candidates
  - 1 merchant
  - 1-3 carriers
  - 4-8 guards
  - 4-8 bandits
  - Farmers/laborers/locals filling remainder
- Initial goods distribution per spec 4.3
- 1 human-controlled agent slot
- 0-2 companions

### Scenario Integration Tests

#### T20: Apple Stockout Chain
- Setup: merchant has apples, farm can produce, carrier can restock, bandits threaten route
- Action: any agent buys all apples
- Verify:
  - Shop stock reaches zero
  - Merchant cannot sell non-existent apples
  - Restock plan generated
  - At least one consumer changes plan due to shortage
  - Scarcity affects price or substitute demand
  - Any restock via physical transport
  - If route disrupted, shortage persists
  - Causal chain: purchase → economy + logistics + agent behavior (3+ subsystems)
- Pass: within 2 in-world days, event graph crosses 3+ subsystems

#### T21: Ruler Death Succession Chain
- Setup: one ruler office, 2+ claimants, guards with loyalties, public order metric
- Action: any agent kills ruler
- Verify:
  - Office vacant immediately on death
  - Claimants begin support-seeking
  - Guards may change loyalty
  - Patrol/public order changes
  - Successor emerges without human intervention
  - Never more than one ruler at a time
  - No cutscene or scripted fallback
- Pass: within 3 in-world days, event graph crosses combat/death + politics + security/economy

#### T22: Bandit Camp Destruction Chain
- Setup: bandit camp with members, supplies, morale, raid routes
- Action: any group destroys/routs camp
- Verify:
  - Survivors flee, surrender, or die (no despawn)
  - Survivors retain injuries, morale, inventory, loyalties
  - Group may split, merge, or establish new camp
  - Route danger changes based on actual positions
  - Downstream actors adapt to new danger map
  - Renewed raids from real reconstituted group (no respawn)
- Pass: within 5 in-world days, route safety + downstream economic behavior changes

#### T23: Companion Physiology Chain
- Setup: companion with needs, travel plan, toilet may/may not be available
- Action: allow needs to escalate during travel
- Verify:
  - Companion reprioritizes based on need pressure
  - If toilet reachable: uses it
  - If blocked: fallback behavior (ask, seek privacy, wilderness, accident)
  - Material + social consequences produced
  - No silent need reset
- Pass: at least one fallback observed with persistent consequences

#### T24: Player Replacement
- Action: detach human control, attach to different agent
- Verify:
  - World continues without reset
  - New agent exposes only its legal affordances
  - Former agent continues under AI
  - No simulation code requires hero entity

#### T25: Unseen Crime Discovery
- Action: hidden theft
- Verify:
  - No immediate global accusation
  - Suspicion only after discovery pathway
  - Response depends on who learned what and reliability

#### T26: Camera Independence
- Action: change visibility focus during simulation
- Verify:
  - No restock, healing, despawn, respawn, or need reset from visibility changes

#### T27: Controlled Agent Death
- Action: kill human-controlled agent
- Verify:
  - World continues without rewind
  - Agent stays dead
  - Control transfers to observer or another agent
  - No resurrection

### Soak & Regression Tests

#### T30: Seven-Day Autoplay
- 100 seeded simulations, 7 in-world days, no human input
- Verify:
  - Zero invariant violations
  - Zero deadlocks
  - Zero disappearing agents or goods
  - At least one: shortage, political tension, route-safety change across run set

#### T31: Stress with Frequent Interruptions
- Repeatedly: move goods, kill leaders, block facilities, attack carriers
- Verify:
  - Agents replan or fail gracefully
  - No corrupted state, duplicate items, orphan reservations

#### T32: Long Replay Consistency
- Record 3-day run, replay from seed + input log
- Verify: exact event hash match at every checkpoint

### Acceptance Criteria Validation (Spec Section 11)
- [ ] World remains coherent with zero human input
- [ ] Four exemplar scenarios arise from general rules, not bespoke scripts
- [ ] Human-controlled agent reassignable to any other agent
- [ ] Every major outcome traceable through event graph
- [ ] Same seed + input log reproduces exactly
- [ ] At least one event per exemplar produces causal chain depth >= 4 across >= 3 subsystems

## Tests
All T20-T32 from spec section 10.

## Acceptance Criteria
- All scenario tests pass
- Soak test (100 seeds) with zero violations
- Replay consistency verified
- Causal depth >= 4 across >= 3 subsystems for all exemplar scenarios
- No scripts driving exemplar outcomes

## Spec References
- Section 10.2 (scenario integration tests: T20-T27)
- Section 10.3 (soak and regression: T30-T32)
- Section 11 (acceptance criteria for emergence)
- Section 3.10 (measure emergence directly)
