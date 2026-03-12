# GOLDENE2E-007: Combat Between Living Agents

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Possible
**Deps**: None

## Problem

The Combat domain (attack, defend actions) and ReduceDanger goal are completely untested at E2E level for living agents. Scenario 8 only tests death from deprivation and opportunistic looting of a corpse — no agent-vs-agent combat occurs. This test validates the combat system's wound resolution, attack/guard skill interaction, and the AI's decision to engage in combat.

**Coverage gap filled**:
- GoalKind: `ReduceDanger` (completely untested)
- ActionDomain: Combat — attack and defend actions (completely untested for living combatants)
- Cross-system chain: Threat detection → ReduceDanger goal → attack action → wound infliction → defender response → combat resolution → item conservation

## Assumption Reassessment (2026-03-12)

1. `GoalKind::ReduceDanger` exists (confirmed in `crates/worldwake-core/src/goal.rs`).
2. `CombatProfile` component tracks attack_skill, guard_skill, wound_capacity, etc. (confirmed in `crates/worldwake-core/src/combat.rs`).
3. Combat action handlers exist in `crates/worldwake-systems/src/combat.rs` (confirmed).
4. Candidate generation for ReduceDanger goals — needs verification. The pressure module in `crates/worldwake-ai/src/pressure.rs` calculates danger signals, but the exact trigger for ReduceDanger goal generation needs checking.
5. The combat system resolves attacks using `CombatProfile` fields — wound infliction mechanics exist (confirmed from Scenario 8's wound system usage).

## Architecture Check

1. This test exercises an entirely different motivation pathway: danger/threat rather than need satisfaction. It validates that the AI can generate combat goals and execute attack/defend actions between two living agents.
2. Fits in `golden_combat.rs` since it extends the combat domain tests.
3. The setup requires two agents where one poses a threat to the other. The exact mechanism for "threat perception" needs discovery during implementation (this is a prime Engine Discovery Protocol candidate).

## What to Change

### 1. Write golden test: `golden_combat_between_living_agents`

In `golden_combat.rs`:

Setup:
- Aggressive agent (attacker) at Village Square with high combat stats (attack_skill `pm(700)`, guard_skill `pm(300)`).
- Defender agent at Village Square with moderate combat stats.
- Both agents have items (e.g., coins) to verify conservation.
- Configure the attacker to perceive the defender as a threat or target — this may require specific profile settings or pressure signals. **Engine Discovery Protocol likely applies here**.

Expected emergent chain:
1. Attacker's AI generates ReduceDanger or combat-related goal.
2. Attack action initiated against defender.
3. Wounds inflicted on defender.
4. Defender may respond with counter-attack or guard action.
5. Combat resolves — at least one wound exists on at least one participant.
6. Conservation: all item quantities maintained.

### 2. Update coverage report

Update `reports/golden-e2e-coverage-analysis.md`:
- Move P7 from Part 3 to Part 1.
- Update Part 2: Combat ActionDomain now tested, ReduceDanger GoalKind tested.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add test)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — if new helpers needed for combat setup)
- `reports/golden-e2e-coverage-analysis.md` (modify — update coverage matrices)

## Out of Scope

- Death from combat (deprivation death is already tested in Scenario 8)
- Looting after combat death (already tested in Scenario 8)
- Fleeing behavior (ReduceDanger via flight)
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

**Note**: This ticket has high Engine Discovery Protocol likelihood. The mechanism for one agent to perceive another as a threat (triggering ReduceDanger) may not be fully wired. The implementer should investigate `candidate_generation.rs` and `pressure.rs` for danger/threat signals before writing the test setup.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_combat_between_living_agents` — two living agents engage in combat, wounds are inflicted
2. At least one agent sustains a wound during combat (wound list grows)
3. An attack action is executed (visible in event log or action scheduler)
4. Conservation: all commodity quantities maintained throughout
5. Coverage report `reports/golden-e2e-coverage-analysis.md` updated: Combat ActionDomain and ReduceDanger GoalKind marked as tested
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
