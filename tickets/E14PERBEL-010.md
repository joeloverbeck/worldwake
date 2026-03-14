# E14PERBEL-010: Restore Combat And Corpse Goal Behavior Under Subjective Beliefs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai` combat/corpse candidate generation, ranking, and planning behavior
**Deps**: E14PERBEL-006, E14PERBEL-007, `specs/E14-perception-beliefs.md`, `specs/S02-goal-decision-policy-unification.md`

## Problem

After the E14 migration off `OmniscientBeliefView`, combat-adjacent AI scenarios no longer reliably produce the intended subjective-behavior outcomes:

- agents stop selecting corpse goals that should be visible from believed corpse state
- living combat scenarios no longer produce stable attack/mitigation behavior
- danger mitigation, burial, and opportunistic looting no longer line up with the existing golden acceptance scenarios

This is not a justification to restore omniscient planning. It is a signal that the current combat/corpse goal stack still contains assumptions that were only accidentally satisfied by the removed omniscient path.

## Assumption Reassessment (2026-03-14)

1. `golden_combat.rs` failures after E14 migration are not all stale-test issues. At least one production bug existed: corpse-loot candidate generation depended on non-self `direct_possessions(corpse)`, which `PerAgentBeliefView` intentionally hides for other entities.
2. Combat and corpse goals are still part of Phase 2 AI behavior and remain valid acceptance scenarios under the foundations. Agents should be able to react to perceived hostiles, corpses, grave plots, and known corpse inventory through lawful belief reads.
3. The correct fix is not to reintroduce omniscient discovery or a compatibility wrapper. The correct fix is to make combat/corpse goal generation and selection operate on subjective-but-sufficient belief data.
4. Some remaining failures may reflect changed combat profile dynamics rather than pure belief bugs. Those cases must be resolved by checking which expectation is architecturally correct, not by blindly preserving old output.

## Architecture Check

1. This ticket must preserve Principles 12-15: combat and corpse goals must arise from perceived hostility, perceived corpses, believed inventory, and local evidence, never from omniscient world reads hidden behind planning APIs.
2. Fixes must live in the production AI stack, not in golden-test-only shims. Test harnesses may seed lawful belief state, but they must not become the only place where the behavior exists.
3. No backwards-compatibility aliasing or renamed omniscient adapters may be introduced.

## What to Change

### 1. Make corpse-goal generation depend on subjective evidence

Audit `LootCorpse` and `BuryCorpse` candidate generation, ranking, and planning assumptions so that:

- corpse detection comes from lawful corpse visibility
- loot eligibility can derive from believed corpse inventory, not only authoritative possession structure
- burial eligibility can derive from perceived corpse and local grave-plot evidence

### 2. Reconcile combat-goal behavior with subjective danger and mitigation

Audit living-combat goal generation and selection so that:

- visible hostility and current attackers produce consistent `EngageHostile` / `ReduceDanger` behavior
- mitigation choices remain concrete (`defend`, relocate, etc.)
- the resulting behavior is explainable from beliefs, profiles, and current action context

### 3. Rebaseline any genuinely stale golden expectations only after production audit

If a `golden_combat` assertion is shown to conflict with the correct post-E14 architecture, update the test with a documented rationale. Do not change goldens merely to make them pass.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify if needed)
- `crates/worldwake-ai/src/goal_model.rs` (modify if needed)
- `crates/worldwake-ai/src/agent_tick.rs` (modify if needed)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify only if assumptions are proven wrong)

## Out of Scope

- Splitting the mixed `BeliefView` trait boundary itself (`E14PERBEL-009`)
- Rumor/report propagation (`E15`)
- Reintroducing omniscient planning access in any form

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_combat`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`

### Invariants

1. Combat and corpse goals are explainable from subjective belief state plus allowed self/runtime/public structure queries.
2. Corpse and combat behavior do not rely on omniscient entity discovery.
3. No compatibility alias or fallback path to deleted omniscient belief code is introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs` — validate corpse looting, burial, mitigation, and living combat behavior under subjective beliefs.
2. `crates/worldwake-ai/src/candidate_generation.rs` — unit coverage for corpse-goal emission from believed corpse inventory and local burial evidence.
3. `crates/worldwake-ai/src/ranking.rs` — unit coverage for corpse-goal ranking/selection semantics if behavior changes.

### Commands

1. `cargo test -p worldwake-ai --test golden_combat`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
