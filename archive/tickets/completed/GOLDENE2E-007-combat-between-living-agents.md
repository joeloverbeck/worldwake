# GOLDENE2E-007: Combat Between Living Agents

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Possible
**Deps**: None

## Problem

The Combat domain (attack, defend actions) is completely untested at E2E level for living agents. Scenario 8 only tests death from deprivation and opportunistic looting of a corpse; no agent-vs-agent combat occurs. This ticket should validate living combat through the real AI loop without overloading defensive danger mitigation into an offensive-aggression abstraction.

**Coverage gap filled**:
- ActionDomain: Combat — attack and defend actions (completely untested for living combatants)
- Cross-system chain: hostility relation → offensive combat goal generation → attack action → wound infliction → defender danger response → combat resolution → item conservation

## Assumption Reassessment (2026-03-12)

1. `GoalKind::ReduceDanger` exists (confirmed in `crates/worldwake-core/src/goal.rs`).
2. `CombatProfile` component tracks attack_skill, guard_skill, wound_capacity, etc. (confirmed in `crates/worldwake-core/src/combat.rs`).
3. Combat action handlers exist in `crates/worldwake-systems/src/combat.rs` (confirmed).
4. `ReduceDanger` is currently a defensive goal, not a clean offensive-combat goal. Candidate generation emits it from danger pressure, and planning can legally satisfy it via `travel`, `defend`, or `attack` depending on context. Using it as the aggressor's motivation would conflate "mitigate incoming danger" with "initiate hostile violence."
5. Visible hostility already exists as concrete world state in the belief view (`visible_hostiles_for` via hostility relations and runtime attackers). That provides a grounded input for a dedicated offensive goal without introducing magic-number aggression heuristics.
6. The combat system resolves attacks using `CombatProfile` fields, and direct scheduler integration tests already prove wound infliction outside the AI loop (`crates/worldwake-systems/tests/e12_combat_integration.rs`).

## Architecture Check

1. The original ticket proposal treated `ReduceDanger` as both a defensive and offensive goal. That is not the clean architecture. Defensive danger mitigation and proactive hostile engagement are distinct motivations and should not share a single goal identity.
2. The more robust change is to add a dedicated hostility-driven combat goal keyed to a concrete hostile target. This preserves `ReduceDanger` as a defensive/self-protective concept and makes offensive combat extensible.
3. Fits in `golden_combat.rs` since it extends combat-domain golden coverage. The ticket no longer claims to close `ReduceDanger` coverage.

## What to Change

### 1. Introduce a dedicated offensive hostility goal

- Add a new targeted goal kind for hostile engagement against a living agent.
- Generate it from concrete local hostility relations, not from abstract aggression scores.
- Reuse the existing `attack` combat action for execution; do not add compatibility aliases or special-case combat-only planner paths.
- Keep `ReduceDanger` defensive. Do not expand it to cover proactive aggression.

### 2. Write golden test: `golden_combat_between_living_agents`

In `golden_combat.rs`:

Setup:
- Aggressive agent (attacker) and defender at Village Square.
- A concrete hostility relation exists between them at setup time.
- Attacker has stronger combat stats than defender so the hostile-engagement path produces an observable wound outcome deterministically.
- Both agents have items (e.g., coins) to verify conservation.

Expected emergent chain:
1. Attacker's AI generates the hostile-engagement goal against the defender.
2. Attack action initiated against defender.
3. Wounds inflicted on defender.
4. Defender responds through the existing defensive combat path (`ReduceDanger`, `defend`, or counter-attack as selected by the real planner).
5. Combat resolves — at least one wound exists on at least one participant.
6. Conservation: all item quantities maintained.

### 3. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move the living-combat scenario from backlog to proven coverage.
- Mark Combat ActionDomain as tested.
- Leave `ReduceDanger` coverage unchanged unless this ticket ends up proving it directly as a consequence of the real scenario.

### Engine Changes Made

- Added a dedicated hostility-driven combat goal for proactive living-agent combat instead of overloading `ReduceDanger`.
- Tightened `ReduceDanger` candidate generation so it only emits at high-or-above danger, matching its goal-satisfaction semantics and preventing zero-step no-op plans under medium visible hostility.
- Fixed combat goal payload binding so entity-targeted combat plans cannot silently bind to the wrong affordance target.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify — add hostile-engagement goal identity)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — emit hostility-driven goal)
- `crates/worldwake-ai/src/goal_model.rs` (modify — map goal to combat attack planning)
- `crates/worldwake-ai/src/ranking.rs` (modify — rank hostility-driven combat goal)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — planner semantics for the new goal)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add test)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — if new helpers needed for combat setup)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Reworking the entire combat motivation stack beyond the minimal dedicated hostility goal
- Death from combat (deprivation death is already tested in Scenario 8)
- Looting after combat death (already tested in Scenario 8)
- Full fleeing-behavior coverage for `ReduceDanger`
- Weapon-enhanced combat (using Sword/Bow items)
- Multi-agent melee

## Engine Discovery Protocol

This ticket is a golden e2e test that exercises emergent behavior through the real AI loop.
If implementation reveals that the engine cannot produce the expected emergent behavior,
the following protocol applies:

1. **Diagnose**: Identify the specific engine limitation (missing candidate generation path, planner op gap, action handler deficiency, belief view gap, etc.).
2. **Do not downgrade the test**: The test scenario defines the desired emergent behavior. Do not weaken assertions or remove expected behaviors to work around engine gaps.
3. **Fix forward**: Implement the minimal, architecturally sound engine change that enables the emergent behavior. Document the change in a new "Engine Changes Made" subsection under "What to Change". Each fix must:
   - Follow existing patterns in the affected module
   - Include focused unit tests for the engine change itself
   - Not introduce compatibility shims or special-case logic
4. **Scope guard**: If the required engine change exceeds this ticket's effort rating by more than one level (e.g., a Small ticket needs a Large engine change), stop and apply the 1-3-1 rule: describe the problem, present 3 options, recommend one, and wait for user confirmation before proceeding.
5. **Document**: Record all engine discoveries and fixes in the ticket's Outcome section upon completion, regardless of whether fixes were needed.

**Note**: The original ticket premise was corrected. The discovery result is that hostility perception exists, but a clean offensive goal does not. This ticket now owns that missing goal because it is the minimal architecture that makes living-agent combat emergent without corrupting `ReduceDanger`.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_combat_between_living_agents` — two living agents engage in combat, wounds are inflicted
2. At least one agent sustains a wound during combat (wound list grows)
3. An attack action is executed (visible in event log or action scheduler)
4. Conservation: all commodity quantities maintained throughout
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Combat ActionDomain marked as tested and the living-combat scenario is moved into proven coverage
6. Existing suite: `cargo test -p worldwake-ai --test golden_combat`
7. Full workspace: `cargo test --workspace` and `cargo clippy --workspace`

### Invariants

1. All behavior is emergent — no manual action queueing
2. Conservation: item lots never increase
3. Determinism: same seed produces same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_combat_between_living_agents` — proves living agent combat

### Commands

1. `cargo test -p worldwake-ai --test golden_combat golden_combat_between_living_agents`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

Originally planned:
- Prove living-agent combat by driving the aggressor through `ReduceDanger`.
- Mark both Combat and `ReduceDanger` as covered.

Actually changed:
- Introduced a dedicated proactive hostility goal for living combat because using `ReduceDanger` for aggression was the wrong architecture.
- Added two golden combat tests (scenario + deterministic replay) plus focused unit coverage for hostile-goal emission and the corrected `ReduceDanger` boundary.
- Updated the coverage report and initiative index to reflect that Combat is now covered, while `ReduceDanger` remains a separate defensive gap.
