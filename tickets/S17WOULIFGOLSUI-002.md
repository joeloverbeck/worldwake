# S17WOULIFGOLSUI-002: Golden Scenario 30 — Recovery-Aware Priority Boost Eats Before Wash

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S17WOULIFGOLSUI-001 (implementation order per spec), S11 (wound lifecycle audit — `promote_for_clotted_wound_recovery`), E12 (wound schema, recovery gate), E09 (needs — eat action), E13 (decision architecture — ranking, candidate generation, planner)

## Problem

The `promote_for_clotted_wound_recovery` mechanism (where a `High`-threshold need goal is promoted to `Critical` when clotted wounds exist and the need is recovery-relevant) is only tested via focused unit tests. No golden E2E test proves this promotion affects real action selection, nor that the downstream consequence (eating opens the recovery gate → wound severity decreases) works through the live AI loop. Existing wound goldens either use sated agents (Scenario 7g — recovery gate always open) or test utility-weight differences (S07a/b — `pain_weight` vs `hunger_weight`), not threshold-based promotion.

## Assumption Reassessment (2026-03-21)

1. `promote_for_clotted_wound_recovery` exists in `crates/worldwake-ai/src/ranking.rs` and is called during goal ranking. It promotes recovery-relevant goals from `High` to `Critical` when the agent has clotted wounds. Focused tests exist in `ranking.rs` unit tests. No golden test exercises this — confirmed by absence in `golden-e2e-coverage.md`.
2. `recovery_conditions_met` exists in `crates/worldwake-systems/src/combat.rs` and gates wound recovery on hunger < 750, thirst < 700, fatigue < 800, not in combat. Documented in E12 combat spec.
3. This is an AI golden ticket. The intended layer is the full AI loop: candidate generation → ranking (with promotion) → plan search → action selection → execution → state change. Full action registries required (eat and wash both need registered handlers).
4. **Ordering layer**: State-delta observation ordering (following S07a/b pattern). Hunger decrease before dirtiness decrease. The compared branches (eat vs wash) are symmetric in the current architecture EXCEPT for the `promote_for_clotted_wound_recovery` call. Both map to `GoalPriorityClass::High` from need levels; the divergence depends on priority class promotion (eat → Critical, wash stays High). Default `UtilityProfile` ensures equal motive scores — the ONLY differentiator is the recovery-aware promotion.
5. No heuristic removal. This ticket adds coverage for existing behavior.
6. Not a stale-request or start-failure ticket.
7. Not a political office-claim ticket.
8. No ControlSource manipulation. Agent uses default AI control.
9. **Isolation choice**: Default `UtilityProfile` with equal weights (ensures priority class, not motive score, determines action order). Thirst/fatigue/bladder at `pm(0)` (eliminates competing goals that could also be recovery-boosted). No other agents (no social/trade/combat). Single location `VILLAGE_SQUARE` (no travel). Only eat and wash are plannable.
10. No mismatch found.

## Architecture Check

1. Pure test-addition ticket. Uses existing live stack: AI ranking (`promote_for_clotted_wound_recovery`), candidate generation, planner search, eat/wash action execution, combat recovery gate. No production code changes.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. Hunger decrease before dirtiness decrease → state-delta observation ordering (action priority proof)
2. Wound severity decreases after hunger drops below High threshold → authoritative world state (recovery gate proof)
3. Agent alive throughout → authoritative world state
4. Deterministic replay → replay companion
5. This ticket asserts on state-delta ordering (which action fires first) plus downstream authoritative state (wound recovery). No decision-trace assertions needed because the ordering surface is sufficient to prove the promotion effect.

## What to Change

### 1. Add `golden_recovery_aware_boost_eats_before_wash` to `golden_combat.rs`

**Setup**:
- Single agent at `VILLAGE_SQUARE`
- One clotted wound: `Wound { id: WoundId(1), body_part: BodyPart::Torso, cause: WoundCause::Deprivation(DeprivationKind::Starvation), severity: pm(200), inflicted_at: Tick(0), bleed_rate_per_tick: pm(0) }`
- `HomeostaticNeeds::new(pm(760), pm(0), pm(0), pm(0), pm(860))` — hunger at High (above 750), dirtiness at High (above 850), everything else at 0
- Custom `MetabolismProfile`: very low rates (`pm(1)` for hunger and dirtiness, `pm(0)` for everything else) to prevent drift during test window
- `CombatProfile`: `natural_recovery_rate: pm(18)`, `natural_clot_resistance: pm(0)` — recovery proceeds after conditions met, no re-bleeding
- Default `UtilityProfile` — equal weights, so recovery-aware promotion is the ONLY differentiator
- `give_commodity`: Bread quantity 3 (directly possessed) — one eat brings hunger well below High
- `give_commodity`: Water quantity 1 (directly possessed) — enables Wash affordance
- No workstations, no other agents, no recipes
- `seed_actor_local_beliefs` with `DirectObservation`

**Assertions**:
1. State-delta ordering: capture initial hunger and dirtiness; step ticks; first observed decrease is hunger (not dirtiness)
2. After eating: hunger drops below `thresholds.hunger.high()` (pm(750))
3. `recovery_conditions_met` becomes true → wound severity begins decreasing
4. Wound severity after sufficient ticks < initial severity pm(200)
5. Agent eventually washes too (dirtiness decreases)
6. Agent alive throughout

### 2. Add replay companion

Standard deterministic replay companion test using `replay_and_verify`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add Scenario 30 test function)

## Out of Scope

- Any production code changes (no changes to `ranking.rs`, `combat.rs`, `needs.rs`, `needs_actions.rs`, or any `src/` file)
- Any harness structural changes
- Scenario 29 (deprivation worsening) — separate ticket S17WOULIFGOLSUI-001
- Docs updates — separate ticket S17WOULIFGOLSUI-003
- Re-testing wound bleed/clot/recovery with sated agent (Scenario 7g)
- Re-testing utility-weight priority resolution (S07a/b)
- Adding new wound mechanics or changing the recovery gate
- Testing `promote_for_clotted_wound_recovery` with sleep or thirst (only hunger is exercised here)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_combat golden_recovery_aware_boost` — new scenario passes
2. `cargo test -p worldwake-ai --test golden_combat` — full combat suite unchanged
3. `cargo test -p worldwake-ai` — full AI crate suite unchanged
4. `cargo test --workspace` — no regressions
5. `cargo clippy --workspace --all-targets -- -D warnings` — no warnings

### Invariants

1. Agent eats before washing — hunger decrease is the first state-delta observed (recovery-aware promotion from `High` to `Critical` > unpromoted `High`)
2. Default `UtilityProfile` used — equal weights prove priority class, not motive score, determines order
3. Wound recovery activates after eating satisfies recovery gate conditions (hunger < 750, thirst < 700, fatigue < 800)
4. Wound severity strictly decreases after recovery gate opens (Principle 10: physical dampeners)
5. Agent remains alive throughout
6. Deterministic replay produces identical state hash
7. No production code modified — this is a coverage-only ticket

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_recovery_aware_boost_eats_before_wash` — proves recovery-aware promotion drives real action selection and enables downstream wound recovery
2. `crates/worldwake-ai/tests/golden_combat.rs::golden_recovery_aware_boost_eats_before_wash_replay` — deterministic replay companion

### Commands

1. `cargo test -p worldwake-ai --test golden_combat golden_recovery_aware_boost`
2. `cargo test -p worldwake-ai --test golden_combat`
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
