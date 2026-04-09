# S81GLDGAP-005: Golden test S81-B -- death traceability

**Status**: PENDING
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
5. `HomeostaticNeedId::Hunger` and `Thirst` are the expected death causes. Default metabolism rates at `crates/worldwake-core/src/needs.rs` determine which need escalates fastest. The test should assert either Hunger or Thirst rather than pinning to one.
15. Survivability math: default metabolism escalates needs over time. With starvation/dehydration tolerance ticks determining wound creation frequency, and wound severity accumulating via `worsen_or_create_deprivation_wound`, the wound load must reach `wound_capacity` within 600 ticks. The test must set `CombatProfile.wound_capacity` low enough (or metabolism rates high enough) to ensure death is reachable. If default values make 600 ticks insufficient, adjust metabolism or wound_capacity in the test setup.

## Architecture Check

1. This golden test exercises the full causal chain across 3 systems (needs, wounds, planning) through state only (P26). No new system coupling introduced.
2. No backward-compatibility shims.

## Verification Layers

1. Agent dies within 600 ticks -> authoritative world state (`get_component_dead_at`)
2. Death cause is `NeedDeprivation` -> authoritative component state (`DeadAt.cause`)
3. Death event tagged `EventTag::Death` -> event-log delta (filter events by tag, assert agent is target)
4. No post-death actions -> action trace (no `ActionStarted` events for agent after `DeadAt.tick`)
5. Multi-layer golden E2E ticket: the contract spans needs system (wound creation), wound system (load check), and planning system (halt).

## What to Change

### 1. Add S81-B golden test to golden_simulation_gaps.rs

In `crates/worldwake-ai/tests/golden_simulation_gaps.rs`, add:

- Helper function `run_death_traceability(seed: Seed)`:
  - Create 1 agent at a barren indoor location (no food, water, or resource sources)
  - Do NOT seed beliefs about remote resources
  - Do NOT give recipe knowledge
  - Set default metabolism (needs escalate over time)
  - Set `CombatProfile` with a wound_capacity that makes death reachable within 600 ticks under default deprivation wound escalation
  - Add `PerceptionProfile` (required for agent to participate in perception ticks)
  - Run for 600 ticks (or until agent dies)
  - Assert: `DeadAt` component is set on the agent
  - Assert: `DeadAt.cause` is `DeathCause::NeedDeprivation { need }` where `need` is `Hunger` or `Thirst`
  - Assert: event log contains at least one event tagged `EventTag::Death` with the agent as target
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
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Death is a persistent state transition (P4) -- once `DeadAt` is set, it is never removed
2. Death leaves explicit aftermath (P10) -- `DeadAt.cause` + `EventTag::Death` event
3. Planning halts for dead agents -- no `ActionStarted` after death tick
4. Deterministic replay under same seed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_simulation_gaps.rs` -- 2 new test functions (golden + replay)

### Commands

1. `cargo test -p worldwake-ai -- golden_death_traceability`
2. `cargo test -p worldwake-ai`
