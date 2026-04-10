# S81GLDGAP-005: Golden test S81-B -- death traceability

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S81GLDGAP-003

## Problem

No golden test verifies the death-from-unmet-needs path. Existing golden tests (`golden_supply_chain.rs`) assert agents stay alive. There is no test that asserts agents die correctly when they cannot sustain themselves, that the death cause is traceable, that a death event is emitted, or that post-death planning halts. This gap means regressions in the mortality path or death event emission could go undetected.

## Assumption Reassessment (2026-04-09)

1. After S81GLDGAP-003, the needs system now sets `DeadAt { tick, cause: DeathCause::NeedDeprivation { need } }` when deprivation wounds exceed `CombatProfile.wound_capacity`. Focused unit tests in that ticket verify the mechanism. This golden test exercises the full multi-system chain: needs escalation -> wound creation -> wound load check -> death -> planning halt.
2. `golden_simulation_gaps.rs` exists at `crates/worldwake-ai/tests/golden_simulation_gaps.rs`. S81-B test will be added to this file alongside S81-A.
3. The planning system's dead-agent skip is at `crates/worldwake-ai/src/agent_tick/mod.rs:248`: checks `get_component_dead_at(agent).is_some()` and `runtime.dead_cleanup_done`. This is the mechanism that halts post-death planning.
4. `EventTag::Death` exists after S81GLDGAP-001. The needs mortality path in S81GLDGAP-003 emits events with this tag.
5. `HomeostaticNeedId::Hunger` and `Thirst` are the only lawful need-death causes, but the scenario can and should make one deterministic. The needs system selects the higher of `needs.hunger` vs `needs.thirst` at death time (`crates/worldwake-systems/src/needs.rs:182-188`), so a starvation-only setup can assert `DeathCause::NeedDeprivation { need: Hunger }` directly instead of allowing either branch.
6. Live harness correction: `seed_agent()` still grants `KnownRecipes::with([RecipeId(0)])` in `crates/worldwake-ai/tests/golden_harness/mod.rs:609-626`, so a true "no recipe knowledge" isolation setup must use `seed_agent_with_recipes(..., KnownRecipes::default())` or explicitly clear `KnownRecipes` after spawn.
7. Live profile correction: the golden harness does not seed `PerceptionProfile` or `CognitiveProfile` on agents by default. `golden_simulation_gaps.rs` already uses `configure_remote_resource_agent()` to install both; S81-B must do the same or equivalent before stepping the AI driver.
8. Scenario numbering correction: S81-A already landed as Scenario 130 in `crates/worldwake-ai/tests/golden_simulation_gaps.rs`, so S81-B must use the next free repo-global id rather than the original spec prose.
9. Golden-doc handoff correction: adding S81-B scenario metadata changes the generated golden inventory/docs. Per `docs/golden-e2e-testing.md`, verification must include `python3 scripts/golden_inventory.py --write --check-docs`.
10. Post-death planning proof correction: the strongest live AI-layer proof of "planning halts" is `DecisionOutcome::Dead` in the decision trace, not just the absence of later action-start events. `golden_integration.rs:1230-1240` already uses this pattern for post-death AI verification.
15. Survivability math correction: default metabolism does not make deprivation death reachable within this golden budget. `MetabolismProfile::default()` in `crates/worldwake-core/src/needs.rs:127-144` uses `starvation_tolerance_ticks = 480` and `dehydration_tolerance_ticks = 240`, so the scenario must shorten the relevant tolerance. A deterministic starvation setup is straightforward: start hunger above the critical threshold (for example `pm(950)`), keep thirst below hunger, set `starvation_tolerance_ticks = 2`, and keep `CombatProfile.wound_capacity = pm(1000)`. Then the first lawful starvation wound fires on tick 2 at severity about `pm(954)`, the second fire on tick 4 worsens it beyond `pm(1000)`, and death occurs within a few ticks without weakening production behavior.

## Architecture Check

1. This golden test exercises the full causal chain across 3 systems (needs, wounds, planning) through state only (P26). No new system coupling introduced.
2. No backward-compatibility shims.

## Verification Layers

1. Agent dies within 600 ticks -> authoritative world state (`get_component_dead_at`)
2. Death cause is `NeedDeprivation` -> authoritative component state (`DeadAt.cause`)
3. Death event tagged `EventTag::Death` -> event-log delta (filter events by tag, assert agent is target)
4. Post-death planning halts -> decision trace (`DecisionOutcome::Dead` after `DeadAt.tick`)
5. No post-death execution resumes -> action trace (no later `ActionTraceKind::Started` events after `DeadAt.tick`)
6. Multi-layer golden E2E ticket: the contract spans needs system (wound creation), wound system (load check), and planning system (halt).

## What to Change

### 1. Add S81-B golden test to golden_simulation_gaps.rs

In `crates/worldwake-ai/tests/golden_simulation_gaps.rs`, add:

- Helper function `run_death_traceability(seed: Seed)`:
  - Create 1 agent at a barren indoor location (no food, water, or resource sources)
  - Do NOT seed beliefs about remote resources
  - Do NOT give recipe knowledge (do not use the default `seed_agent()` helper unchanged)
  - Add both `PerceptionProfile` and `CognitiveProfile`
  - Set a deterministic starvation-focused metabolism/combat setup whose live arithmetic reaches death inside the scenario budget
  - Run for 600 ticks (or until agent dies)
  - Assert: `DeadAt` component is set on the agent
  - Assert: `DeadAt.cause` is `DeathCause::NeedDeprivation { need: HomeostaticNeedId::Hunger }`
  - Assert: event log contains at least one event tagged `EventTag::Death` with the agent as target
  - Assert: decision traces after `DeadAt.tick` show `DecisionOutcome::Dead`
  - Assert: no `ActionStarted` events for the agent after `DeadAt.tick`

- Test function `golden_death_traceability()` with `#[test]`
- Deterministic replay test `golden_death_traceability_replays_deterministically()`

### 2. Scenario isolation

The scenario intentionally removes all survival options (no food, no water, no beliefs, no recipes) to isolate the death-from-deprivation branch. Lawful competing affordances excluded: `AcquireCommodity` (no sources), `ConsumeOwnedCommodity` (no inventory). Only `Sleep` and `Relieve` remain as available actions, which cannot prevent death.

## Files to Touch

- `crates/worldwake-ai/tests/golden_simulation_gaps.rs` (modify -- add S81-B test)

## Out of Scope

- Fixing the mortality mechanism itself (S81GLDGAP-003)
- Multi-agent convergence testing (S81GLDGAP-004)
- Harvest-to-consume chain testing (S81GLDGAP-006)
- Combat death traceability (existing combat tests cover `DeathCause::CombatWounds` and `EventTag::Death` after S81GLDGAP-003)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_death_traceability` -- agent dies with traceable cause, death event emitted, no post-death actions
2. `golden_death_traceability_replays_deterministically` -- same seed produces same outcome
3. Generated golden docs updated cleanly for the new scenario metadata
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Death is a persistent state transition (P4) -- once `DeadAt` is set, it is never removed
2. Death leaves explicit aftermath (P10) -- `DeadAt.cause` + `EventTag::Death` event
3. Planning halts for dead agents -- no `ActionStarted` after death tick
4. Deterministic replay under same seed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_simulation_gaps.rs` -- 2 new test functions (golden + replay)
2. Generated docs under `docs/generated/` refreshed by the canonical golden inventory script

### Commands

1. `cargo test -p worldwake-ai --test golden_simulation_gaps golden_death_traceability`
2. `cargo test -p worldwake-ai --test golden_simulation_gaps golden_death_traceability_replays_deterministically`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-04-10.

- Added Scenario 131 in `crates/worldwake-ai/tests/golden_simulation_gaps.rs` covering deprivation death traceability with a single isolated agent, explicit death-event verification, deterministic `DeathCause::NeedDeprivation { need: Hunger }`, post-death `DecisionOutcome::Dead` proof, and no post-death action-start assertion.
- Added the matching deterministic replay test for the same scenario and seed.
- Regenerated the golden inventory/docs so the new death-traceability scenario metadata is reflected in the generated coverage artifacts under `docs/generated/`.

## Deviations

- Reassessment showed the default `seed_agent()` path grants recipe knowledge and omits both `PerceptionProfile` and `CognitiveProfile`, so the scenario used `seed_agent_with_recipes(..., KnownRecipes::default())` plus explicit profile setup instead of the simpler helper path.
- Reassessment also showed default metabolism would not reach deprivation death within the ticket's stated budget. The scenario therefore uses a deterministic starvation-focused metabolism override (`starvation_tolerance_ticks = 2`) while keeping the default combat wound capacity, rather than weakening the mortality implementation or relying on a vague “dies eventually” assertion.
- The post-death planning halt proof was strengthened from a missing-action-only check to the live AI-layer contract used elsewhere in the suite: at least one post-death decision trace must record `DecisionOutcome::Dead`.

## Verification Result

- Passed focused tests:
  - `cargo test -p worldwake-ai --test golden_simulation_gaps golden_death_traceability`
  - `cargo test -p worldwake-ai --test golden_simulation_gaps golden_death_traceability_replays_deterministically`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai`
