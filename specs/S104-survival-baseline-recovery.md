# S104: Survival Baseline Recovery

## Summary

Agents cannot satisfy basic needs (eating, drinking, washing, sleeping, relieving) in realistic 1440-tick simulations. The root cause is not architectural — the needs chain (pressure → goal → plan → action → satisfaction) works correctly when agents have the right profiles, recipes, and knowledge. The root cause is that development built features and golden tests in sterile isolation, and no scenario ever proved agents can bootstrap survival from realistic starting conditions. The golden test suite (~366 tests, 29 files) now actively blocks survival fixes because behavioral assertions encode the current broken priority ordering.

This spec defines a three-phase recovery: (1) triage and remove golden tests that block progress, (2) fix profile-gating so all candidate emitters gracefully handle absent profiles and design a minimal survival scenario, (3) rebuild golden test coverage bottom-up from a proven survival baseline.

## Phase

Core infrastructure (prerequisite for all future gameplay specs)

## Status

Draft

## Crates

- `worldwake-ai` (candidate generation profile-gating, golden test triage)
- `worldwake-cli` (survival baseline scenario)
- `worldwake-core` (no changes expected)
- `worldwake-sim` (no changes expected)
- `worldwake-systems` (no changes expected)

## Dependencies

- S103 (belief claim deduplication) — completed, archived
- All Phase 7 specs (S60–S66) — blocked until survival baseline is proven

## Problem Statement

### Evidence

The starvation diagnostic (`reports/needs-starvation-diagnostic.md`) on `cli-evaluation.ron` seed 7777, 1440 ticks:

| Agent | Need | Root Cause | Outcome |
|-------|------|-----------|---------|
| Guard Theron | Hunger | No food recipes + duty goals (InvestigateViolation, PostNotice, Patrol) outranked survival at hunger=1000 + no exploration profile | **Dead** at tick 769 |
| Kael | Hunger | Stuck at resource-poor Dusty Trail + no knowledge of Golden Fields (food source) + ProduceCommodity budget-exhausted 5 times | Hunger saturated at 1000 from tick 757 |
| Forager Lina | Dirtiness | No WashBasin at Eldergrove Forest + no perception profile (only 216 observations vs Kael's 444) + exploration targeted known-adjacent places that also lack facilities | Dirtiness at 1000 for 810 ticks |
| Merchant Vara | Thirst | Transient geographic desert at Dusty Trail | **Self-resolved** through travel |

### Why golden tests didn't catch this

Golden tests construct sterile environments:
- Agents placed adjacent to needed resources
- Agents given exactly the recipes and knowledge they need
- Profile requirements hard-wired for the feature under test
- Short runs (50–300 ticks) that don't expose bootstrap failures

This means golden tests verify "does the mechanism work when everything is set up correctly?" but never test "can agents bootstrap survival from realistic starting conditions?" — which is exactly what FND-06 (World Runs Without Observers) and FOUNDATIONS Scenario B (Hungry Agent → Market Trip) demand.

### Why golden tests now block fixes

Every behavioral change to candidate generation (profile-gating, priority ordering, exploration triggers) changes the exact sequence of goals and actions agents take. Approximately 260 of ~366 tests use `StateHash` comparisons or tick-specific action assertions that break with any behavioral change. The test suite has become a constraint that preserves broken behavior.

## Design Goals

1. Remove golden tests that encode the current broken behavior, freeing the codebase for survival fixes
2. Fix the one profile-gating defect where candidate generation panics on absent profiles
3. Design a minimal scenario that proves agents can meet all five homeostatic needs (Hunger, Thirst, Fatigue, Bladder, Dirtiness) for 1440 ticks
4. Rebuild golden test coverage from a survival baseline, layering complexity incrementally
5. Establish a workflow where scenarios prove behavior before golden tests pin it

## Non-Goals

- Changing the GOAP planner architecture or search algorithm
- Modifying the needs/metabolism system mechanics
- Adding new gameplay features or systems
- Changing how profiles are defined in scenarios (RON format)
- Performance optimization (covered by S103)

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| FND-06 (World Runs Without Observers) | Survival baseline must work autonomously for 1440 ticks with no human intervention |
| FND-20 (Resource-Bounded Practical Reasoning) | Survival goals must be practically achievable by agents with limited knowledge and bounded planning budgets |
| FND-22 (Agent Diversity) | Role-specific profiles must be optional — an agent with only survival profiles must function, not panic |
| FND-31 (Validation First-Class) | Golden tests must validate real emergent behavior from realistic starting conditions, not pre-wired sterile setups |
| FND-01 (Maximal Emergence) | Survival must emerge from the interaction of needs pressure, goal generation, planning, and action execution — not from authored success paths in test harnesses |

## Design

### Phase 1: Golden Test Triage

#### Triage Criteria

| Category | Criterion | Action |
|----------|-----------|--------|
| **KEEP** | Tests system mechanics via structural invariants, zero StateHash calls, zero tick-specific action sequence assertions | Retain unchanged |
| **REMOVE** | Uses StateHash comparisons or asserts specific goal/action sequences at specific ticks | Delete file |
| **TRIAGE** | Mixed — some tests are invariant-based, others are sequence-dependent | Review per-test, split if warranted |

#### Per-File Triage

**KEEP (4 files, ~32 tests):**

| File | Tests | Rationale |
|------|-------|-----------|
| `golden_activation_decay.rs` | 6 | Pure perception lifecycle invariants (decay thresholds, salience boosts). Zero hashes. |
| `golden_exploration.rs` | 8 | Exploration mechanic invariants (frontier unlocking, multi-hop, persistence). Zero hashes. |
| `golden_perception_exposure.rs` | 6 | Perception exposure invariants (witnessed-event, place concealment, fatigue). Zero hashes. |
| `golden_travel_physiology.rs` | 12 | Travel exertion invariants (body cost multipliers, need escalation, critical thresholds). Zero hashes. |

**REMOVE (19 files, ~251 tests):**

| File | Tests | Hash Calls | Reason |
|------|-------|-----------|--------|
| `golden_budget_exhaustion_snapshots.rs` | 7 | 0 | Tick-specific action sequence assertions |
| `golden_care.rs` | 18 | 17 | Hash-dominant, care interaction chains sensitive to priority ordering |
| `golden_combat.rs` | 27 | 23 | Hash-dominant, combat decision trees depend on survival/duty ordering |
| `golden_commodity_opportunity.rs` | 3 | 2 | Pure hash assertions, no invariant fallback |
| `golden_determinism.rs` | 12 | 10 | Hash-dominant determinism checks — valuable concept but hashes must be regenerated after fixes |
| `golden_emergent.rs` | 51 | 53 | Largest file, highest hash density, multi-system chains cascade with priority changes |
| `golden_expectation.rs` | 10 | 10 | Hash-dependent with tick-specific search assertions |
| `golden_integration.rs` | 28 | 29 | Hash-dominant cross-system integration tests |
| `golden_long_scenarios.rs` | 4 | 5 | Long-running scenarios with tick-specific assertions on office vacancy chains |
| `golden_patrol.rs` | 8 | 2 | Hash-dependent patrol scenarios sensitive to goal ordering |
| `golden_production.rs` | 34 | 31 | Hash-dominant production chains sensitive to priority ordering |
| `golden_pursuit.rs` | 6 | 7 | Hash-dependent pursuit scenarios |
| `golden_reasoning_diversity.rs` | 6 | 9 | Hash-dependent reasoning traces |
| `golden_resilience.rs` | 2 | 6 | High hash density (6 hashes on 2 tests) with tick assertions |
| `golden_soak.rs` | 1 | 3 | Soak invariants are valuable but hash calls and specific thresholds will break; rebuild after baseline proven |
| `golden_social.rs` | 18 | 36 | Highest hash density (36 calls), social chains shift with priority ordering |
| `golden_supply_chain.rs` | 2 | 13 | Hash-dominant supply chain |
| `golden_t22_bandit_camp_destruction.rs` | 8 | 8 | Hash-dependent multi-system scenario |
| `golden_trade.rs` | 11 | 10 | Hash-dependent trade decisions |

**TRIAGE (6 files, ~83 tests):**

| File | Tests | Hash Calls | Notes |
|------|-------|-----------|-------|
| `golden_ai_decisions.rs` | 19 | 2 | Low hash count, heavy invariants (64). Scenario 1 tests two hungry agents — directly relevant to survival. Review per-test. |
| `golden_experience_preferences.rs` | 6 | 4 | Small file. Route preference learning may not depend on goal ordering. |
| `golden_merchant_selling.rs` | 20 | 10 | Enterprise-weighted, may be insensitive to survival reordering. |
| `golden_offices.rs` | 24 | 19 | High hash count but enterprise-weighted. Individual review needed. |
| `golden_planner_pathology.rs` | 4 | 2 | Low hash count. Pathology-focused tests may be goal-ordering independent. |
| `golden_simulation_gaps.rs` | 10 | 11 | Mixed invariants and hashes. Gap-specific tests need individual review. |

#### Infrastructure Files (unchanged)

- `golden_harness/mod.rs` — shared test framework. No changes.
- `golden_harness/soak_world.rs` — T30 world setup. No changes.
- `golden_harness/timeline.rs` — timeline builder. No changes.

#### Generated Documentation Update

After triage, re-run `python3 scripts/golden_inventory.py --write --check-docs` to regenerate:
- `docs/generated/golden-scenario-index.md`
- `docs/generated/golden-coverage-matrix.md`
- `docs/generated/golden-e2e-inventory.md`
- `docs/generated/golden-scenario-details/*.md`

### Phase 2: Profile-Gating Cleanup + Survival Scenario

#### 2a. Profile-Gating Fixes

Comprehensive audit of `candidate_generation.rs` found one defect:

| Function | Line | Profile | Problem | Fix |
|----------|------|---------|---------|-----|
| `emit_social_candidates` | 1153 | `TellProfile` | **Panics** via `unwrap_or_else(\|\| panic!(...))` | Change to `let Some(profile) = ctx.view.tell_profile(ctx.agent) else { return; }` |

All other emitters (~45 functions) already use graceful skip patterns (`let ... else { return; }` or `if let Some(...)`). No `unwrap_or_default()` issues were found — `utility_profile_for_goal_generation` uses `unwrap_or_default()` but downstream functions check weight values and return early when zero.

The TellProfile fix aligns with FND-22: role-specific profiles must be optional. An agent without social capabilities should simply not generate social goals, not crash.

#### 2b. Survival Baseline Scenario

Create `scenarios/survival-baseline.ron` with:

**Places (4):**
- **Riverside Camp** (place tags: Camp, Latrine): Facilities: Well. Resource sources: Water (via Well). Starting location for Agents A and B. Latrine tag enables indoor bladder relief.
- **Fertile Fields** (place tags: Field, Farm): Facilities: OrchardRow. Resource sources: Apple (via OrchardRow). Travel: 3 ticks from Riverside. Outdoor tags (Field, Farm) enable outdoor bladder relief with dirtiness penalty.
- **Forest Clearing** (place tags: Forest): Facilities: Well. Resource sources: Water (via Well). Travel: 4 ticks from Riverside, 5 ticks from Fields. Outdoor tag (Forest) enables outdoor bladder relief with dirtiness penalty.
- **Hillside Shelter** (place tags: Camp, Latrine): No workstations or resource sources. Travel: 3 ticks from Forest, 6 ticks from Riverside. Latrine tag enables indoor bladder relief.

**Need satisfaction mapping:**

| Need | Satisfaction Mechanism | Required Facility/Tag |
|------|----------------------|----------------------|
| Hunger | Eat action (consume Apple, Grain, or Bread) | Production: OrchardRow + resource source in the minimal authored baseline |
| Thirst | Drink action (consume Water) | Production: Well + resource source |
| Fatigue | Sleep action | None — agents sleep anywhere |
| Bladder | Toilet action (at Latrine) or Relieve Wilderness (at outdoor tag: Forest, Field, Farm, Trail, Road) | PlaceTag::Latrine or OUTDOOR_RELIEF_TAGS |
| Dirtiness | Wash action (consumes 1 Water from inventory) | None — requires Water in possession |

**Agents (3):**
Each agent has ONLY survival-relevant profiles:
- `MetabolismProfile` with varied rates (agent diversity per FND-22)
- `PerceptionProfile` with standard detection ranges
- `ExplorationProfile` with default frontier search settings
- `CognitiveProfile` with standard planning budgets
- `DriveThresholds` with standard low/critical/max values

No agent has: PatrolProfile, JusticeDispositionProfile, TellProfile, ArtifactPostingProfile, MerchandiseProfile, CombatProfile, ViolationDispositionProfile, or any other duty/role profile.

**Recipes:**
All agents know: `"Harvest Apples"` and `"Harvest Water"`. These are the minimum recipes used by the authored baseline that still prove food and water self-sufficiency through exploration.

**Starting knowledge:**
- Agent A starts at Riverside Camp, knows Riverside and Fertile Fields
- Agent B starts at Riverside Camp, knows only Riverside
- Agent C starts at Forest Clearing, knows Forest and Hillside Shelter

This creates varied starting conditions: Agent A has a known food source, Agent B must explore to find food, Agent C has water but must explore for food. All agents have access to bladder relief at their starting locations (Latrine at Riverside/Hillside, Forest outdoor tag at Forest Clearing).

#### 2c. Observer Validation

Run: `observer scenarios/survival-baseline.ron --ticks 1440 --output reports/survival-baseline-validation.md`

**Success criteria:**
- Zero deaths
- No need sustained above critical threshold (Permille 750) for more than 100 consecutive ticks — applies to all five homeostatic needs (Hunger, Thirst, Fatigue, Bladder, Dirtiness)
- All agents execute at least one eat, drink, wash, sleep, and relieve action
- Agent B successfully explores to discover a food source
- Zero deaths, all five needs kept below the sustained-critical threshold, and all agents execute eat/drink/wash/sleep/relieve at least once in the observer report
- The remaining survival-path `ProduceCommodity` budget-exhaustion signatures were removed in `archive/tickets/S104SURBASREC-007.md`, so Layer 0 can now pin the baseline against the clean observer report
- No anomaly flags for idle stretches > 50 ticks or action loops

### Phase 3: Golden Test Rebuild

After Phase 2 proves survival works, rebuild golden test coverage in layers:

**Layer 0: Survival Baseline Golden Tests**
- New file: `golden_survival_baseline.rs`
- Tests: 3-4 agents with only survival profiles, 1440 ticks
- Assertions: invariant-style (all five needs stay managed, no deaths, exploration discovers resources)
- No StateHash assertions — only structural invariants
- This is now the permanent survival regression test, implemented in `archive/tickets/S104SURBASREC-005.md`
- It loads `scenarios/survival-baseline.ron` through a test-only `worldwake-cli` scenario bridge, proving survival, self-care action coverage, Agent B food discovery, and the absence of survival-goal `BudgetExhausted` attempts

**Layer 1: Single-System Addition Tests**
For each non-survival system (trade, combat, social, offices, patrol, etc.):
- Add the system's profiles to survival-baseline agents
- Run 1440 ticks
- Verify survival isn't degraded (same invariants as Layer 0)
- Add system-specific invariants (e.g., "merchant completes at least one trade AND stays alive")

**Layer 2: Cross-System Golden Tests**
Rebuilt versions of the removed multi-system tests, now grounded in survival-capable agents:
- Tests begin from survival-proven configurations
- Add cross-system interactions one at a time
- Each test proves its specific cross-system chain without assuming survival magically works

**Layer 3: Soak and Determinism Rebuild**
- New soak test with survival baseline world
- Determinism test with regenerated hashes
- Long-scenario tests with survival-capable agents

## FND-01 Section H Analysis

### Information-path analysis

No new information paths introduced. The profile-gating fix removes a panic path — it does not change what information agents receive. The survival scenario creates agents with perception profiles that discover resources through the existing perception → belief → planning path (FND-07, FND-14, FND-15).

### Positive-feedback analysis

No new feedback loops introduced. The existing needs → goal → action → satisfaction loop has a natural dampener: successful consumption reduces need pressure, reducing goal priority. The exploration → discovery → acquisition path has a natural dampener: once an agent discovers and reaches a resource, exploration goals stop being generated for that need.

### Concrete dampeners

Existing dampeners unchanged:
- **Need satisfaction**: Eating/drinking/sleeping/relieving reduces need pressure, stopping goal generation (physical process)
- **Exploration exhaustion**: `max_consecutive_explorations` limits unbounded exploration (bounded agent capacity)
- **Acquisition exhaustion**: Failed acquisition attempts are tracked, preventing infinite retry loops (agent memory)

### Stored state vs. derived read-model list

| Item | Classification | Change |
|------|---------------|--------|
| Agent profiles (MetabolismProfile, etc.) | Authoritative stored state | No change — profiles remain optional components |
| `emit_social_candidates` TellProfile check | Transient computation (candidate generation) | Changed from panic to graceful skip |
| Golden test assertions | Test infrastructure (not world state) | Removed/rebuilt |
| `scenarios/survival-baseline.ron` | Authored initial conditions | New |

## Verification

### Phase 1 Verification
1. `cargo clippy --workspace --all-targets -- -D warnings` — clean
2. `cargo test -p worldwake-ai` — passes with reduced test count (KEEP + remaining TRIAGE tests)
3. `cargo test -p worldwake-systems` — system integration tests unaffected
4. Regenerate golden docs: `python3 scripts/golden_inventory.py --write --check-docs`

### Phase 2 Verification
1. `cargo clippy --workspace --all-targets -- -D warnings` — clean
2. `cargo test -p worldwake-ai` — KEEP tests still pass after profile-gating fix
3. Observer run: `observer scenarios/survival-baseline.ron --ticks 1440` meets all success criteria
4. Manual CLI smoke test: load `survival-baseline.ron`, tick 100, verify agents are eating/drinking/sleeping

### Phase 3 Verification
1. `cargo test -p worldwake-ai` — all new golden tests pass
2. Layer 0 survival test passes with 1440 ticks
3. Layer 1 tests demonstrate survival + single-system coexistence
4. Full `cargo test --workspace` clean
5. `cargo clippy --workspace --all-targets -- -D warnings` clean
