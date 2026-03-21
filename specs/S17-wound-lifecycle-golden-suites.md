# S17: Wound Lifecycle Golden E2E Suites

## Summary

S11 (wound lifecycle audit) delivered two cross-system behaviors tested only at the focused unit level:

1. **Deprivation wound worsening** (`worsen_or_create_deprivation_wound` in `needs.rs`): repeated deprivation threshold fires consolidate into one wound instead of creating duplicates. WoundId preserved, severity increases, `inflicted_at` updates.

2. **Recovery-aware AI priority boost** (`promote_for_clotted_wound_recovery` in `ranking.rs`): when an agent has clotted wounds and a recovery-blocking need (hunger, thirst, fatigue) at `High` threshold, the AI promotes that need goal from `High` to `Critical`.

The golden E2E suite (129 tests) does not exercise either code path through the full emergent AI loop. Existing wound goldens cover death cascades (Scenario 8), passive bleed/clot/recovery with a sated agent (Scenario 7g), and utility-weight priority resolution (S07a/b), but none exercise the S11-specific invariants.

This spec adds two cross-system golden suites that exercise these missing contracts through ordinary emergent behavior. Both suites prove that S11 is not a focused-test-only patch but part of the live wound lifecycle that drives real agent decisions.

## Phase

Phase 3: Information & Politics (post-S11, parallel with other Phase 3 work)

## Crates

`worldwake-ai`
- add cross-system golden suites

`docs`
- update golden E2E coverage docs after the suites land

## Dependencies

- E12 (combat & health — wound schema, CombatProfile, wound progression)
- E09 (needs & metabolism — deprivation exposure, threshold firing)
- E13 (decision architecture — GOAP planner, goal ranking, candidate generation)
- S11 (wound lifecycle audit — worsening logic, recovery-aware boost)

## Why This Exists

Current golden coverage proves wound creation, progression, and death, but it still leaves two meaningful end-to-end gaps:

1. **No golden currently proves deprivation wound consolidation.**
   Existing deprivation goldens (Scenarios 8, 8d, 9d) use `wound_capacity: pm(200)` with pre-existing wounds — agents die from ONE additional threshold fire. None survive long enough for a second fire to exercise the `worsen_or_create_deprivation_wound` path. The consolidation invariant (wound count stays at 1 across multiple fires) is tested only in focused unit tests.

2. **No golden currently proves recovery-aware priority promotion.**
   Scenario 7g tests bleed→clot→recovery with a sated agent (recovery gate always open). Scenarios S07a/S07b test wound-vs-hunger priority via `UtilityProfile` weight differences. Neither exercises the `promote_for_clotted_wound_recovery` mechanism where a `High`-threshold need is promoted to `Critical` because of clotted wounds, nor proves that the resulting action opens the recovery gate.

The gap is not that S11 lacks any tests. The gap is that the remaining unproven behaviors are exactly the ones that distinguish S11 from a unit-test-only patch.

## Foundational Alignment

This spec exists to strengthen proof of the following principles in [FOUNDATIONS.md](../docs/FOUNDATIONS.md):

- Principle 3: concrete state over abstract scores — wound identity preserved under worsening
- Principle 4: persistent identity — WoundId survives multiple severity changes
- Principle 5: carriers of consequence — worsened wounds propagate pain pressure downstream
- Principle 9: outcomes leave aftermath — deprivation accumulates concrete damage, not abstract counters
- Principle 10: physical dampeners — recovery gate is a concrete physical condition (satiation, rest)
- Principle 18: resource-bounded practical reasoning — priority promotion drives real action selection
- Principle 24: systems interact through state — needs writes wounds, combat reads wounds for recovery, AI reads wounds for ranking
- Principle 27: debuggability — decision traces and action traces available for diagnosis

## Design Goals

1. Prove that deprivation wound consolidation works through the live needs system dispatch, not just through direct `worsen_or_create_deprivation_wound` calls.
2. Prove that recovery-aware priority promotion affects real action selection and enables downstream wound recovery.
3. Keep the suites maximally emergent: AI drives all actions through the real planner, no test-only shortcuts.
4. Use the strongest assertion surfaces per `docs/golden-e2e-testing.md`: authoritative state for wound invariants, state-delta ordering for action priority, deterministic replay companions.
5. Keep setup lawful and explicit. Remove competing affordances through concrete world state (zeroed metabolism rates, absence of commodities), not by bypassing systems.

## Non-Goals

1. Re-testing wound creation, bleed progression, or death already covered by Scenarios 7g, 8, 8d, 9d.
2. Re-testing utility-weight priority resolution already covered by S07a/S07b.
3. Adding new wound system mechanics or changing the recovery gate.
4. Promoting every focused S11 unit invariant into a golden scenario.
5. Testing deprivation + combat wound coexistence — the pressure system sums severities without special cross-system logic, so no distinct emergent chain exists beyond what Scenarios 29 and 30 exercise separately.

## Scenario Gaps To Close

### Scenario 29: Deprivation Wound Worsening Consolidates Not Duplicates

**File**: `crates/worldwake-ai/tests/golden_emergent.rs`
**Systems exercised**: Needs (metabolism, deprivation exposure, `worsen_or_create_deprivation_wound`), Combat (wound list storage, wound_capacity survival check), deterministic replay
**Principles proven**: 3, 4, 5, 9, 24

**Intent**:
Prove that repeated deprivation threshold fires consolidate into ONE wound with preserved `WoundId`, increasing severity, and updated `inflicted_at` — through the live needs system dispatch, not direct function calls.

**Setup**:
- Single agent at `VILLAGE_SQUARE`
- Hunger above critical threshold: `HomeostaticNeeds::new(pm(920), pm(0), pm(0), pm(0), pm(0))`
- Custom `MetabolismProfile` with `starvation_tolerance_ticks: nz(5)` (fires every 5 critical ticks; default is nz(480), far too slow for a golden test window). All other tolerance ticks set high. `hunger_rate: pm(0)` (hunger already above critical, no need to accumulate further). All other rates `pm(0)` to prevent competing needs.
- `DeprivationExposure` pre-seeded with `hunger_critical_ticks: 4` (1 tick from first fire)
- `CombatProfile` with `wound_capacity: pm(1000)`, `natural_recovery_rate: pm(0)`, `natural_clot_resistance: pm(0)` — agent survives two+ fires, no wound healing
- Empty `WoundList` — no pre-existing wounds
- No food, no workstations, no other agents, no recipes
- `seed_actor_local_beliefs` with `DirectObservation`
- Default `DriveThresholds`

**Emergent behavior proven**:
- First deprivation threshold fire creates one starvation wound (wound count becomes 1)
- Exposure counter resets after fire, accumulates 5 more critical ticks, fires again
- Second fire worsens the existing wound instead of creating a duplicate (wound count stays 1)
- `WoundId` is preserved across both fires
- Severity increases between first and second fire
- `inflicted_at` updates to the tick of the second fire
- Agent remains alive throughout (wound_load never exceeds wound_capacity pm(1000))

**Why this is distinct**:
- Scenario 8 (`golden_death_cascade`): `wound_capacity: pm(200)` + pre-existing pm(150) wound — agent dies from ONE additional fire. Never survives to exercise worsening.
- Scenario 8d (`golden_death_while_traveling`): Same fragile setup, death before destination.
- Scenario 9d (`golden_dead_agent_pruned_from_facility_queue`): Same fragile setup, death while queued.
- This scenario uses `wound_capacity: pm(1000)` with NO pre-existing wound, specifically designed for the agent to survive TWO+ worsening events.

**Assertion surface**:
1. Authoritative wound state: `wounds.len() <= 1` every tick (KEY consolidation invariant)
2. After two fires: same `WoundId`, higher severity, later `inflicted_at`
3. Agent alive throughout (never exceeds wound_capacity)
4. Determinism: replay companion

**Scenario-isolation choice**:
- No food available — agent cannot eat, so no competing `ConsumeOwnedCommodity` goal
- All non-hunger metabolism rates zeroed — no competing needs goals
- No other agents — no social, trade, or combat interactions
- Agent is idle (no plannable goals satisfy hunger without food), so the only system activity is needs ticking and deprivation firing

---

### Scenario 30: Recovery-Aware Priority Boost Eats Before Wash

**File**: `crates/worldwake-ai/tests/golden_combat.rs`
**Systems exercised**: AI (candidate generation, `promote_for_clotted_wound_recovery` ranking, planning, action selection), Needs (eat action, hunger relief), Combat (wound recovery gate, `recovery_conditions_met`, natural recovery progression), deterministic replay
**Principles proven**: 3, 10, 18, 24, 27

**Intent**:
Prove that an agent with clotted wounds and High-threshold hunger eats before washing (because eat is recovery-boosted to Critical while wash stays High), and that eating satisfies the recovery gate, enabling wound severity to decrease.

**Setup**:
- Single agent at `VILLAGE_SQUARE`
- One clotted wound: `Wound { id: WoundId(1), body_part: BodyPart::Torso, cause: WoundCause::Deprivation(DeprivationKind::Starvation), severity: pm(200), inflicted_at: Tick(0), bleed_rate_per_tick: pm(0) }`
- Hunger at High threshold: `pm(760)` (above `thresholds.hunger.high()` = pm(750), below `critical()` = pm(900)). Maps to `GoalPriorityClass::High`, then boosted to `Critical` by `promote_for_clotted_wound_recovery` because `recovery_relevant: true`.
- Dirtiness at High threshold: `pm(860)` (above `thresholds.dirtiness.high()` = pm(850), below `critical()` = pm(950)). Maps to `GoalPriorityClass::High`, stays `High` because `Wash` has `recovery_relevant: false`.
- Thirst, fatigue, bladder at `pm(0)` — no competing needs goals. Sleep (`recovery_relevant: true`) maps to `Background` at fatigue 0, so no boost interference.
- Custom `MetabolismProfile` with very low rates (`pm(1)` for hunger and dirtiness, `pm(0)` for everything else) — prevents significant needs drift during the test window.
- `CombatProfile` with `natural_recovery_rate: pm(18)`, `natural_clot_resistance: pm(0)` — recovery proceeds after conditions met, no re-bleeding.
- Default `UtilityProfile` — equal weights keep the comparison symmetric at the weight layer, but motive scores still differ because dirtiness pressure exceeds hunger pressure. In the live setup, bread motive is `500 * 760 = 380_000` while wash motive is `500 * 860 = 430_000`, so the scenario proves the stronger contract: recovery-aware class promotion overrides a higher competing wash motive.
- `give_commodity`: Bread (quantity 3) — hunger relief per unit brings hunger well below High threshold after one eat. Directly possessed.
- `give_commodity`: Water (quantity 1) — enables Wash affordance. Directly possessed.
- No workstations, no other agents, no recipes
- `seed_actor_local_beliefs` with `DirectObservation`

**Emergent behavior proven**:
- Initial ranking contains both eat-Bread and `Wash`, with `Wash` carrying the higher motive score but bread promoted from `High` to `Critical`
- Agent selects eat-Bread first because the recovery-aware boost changes class ordering: eat mapped to `High` → promoted to `Critical`; wash mapped to `High` → stayed `High`
- After eating, hunger drops below `thresholds.hunger.high()` (pm(750))
- `recovery_conditions_met()` becomes true (hunger < 750, thirst < 700, fatigue < 800, not in combat)
- Wound severity begins decreasing via `natural_recovery_rate` (pm(18) per tick)
- Agent also eventually washes (dirtiness decreases)

**Why this is distinct**:
- Scenario 7g (`golden_wound_bleed_clotting_natural_recovery`): Uses `HomeostaticNeeds::new_sated()` — recovery gate is always open. Tests passive wound lifecycle, NOT the recovery-blocking priority boost.
- S07a/S07b (`golden_wound_vs_hunger_pain_first` / `_hunger_first`): Uses `no_recovery_combat_profile()` (zero recovery) and different `pain_weight` vs `hunger_weight` in `UtilityProfile`. Tests weight-based priority resolution, NOT threshold-based promotion.
- This scenario uses equal utility weights and tests the `promote_for_clotted_wound_recovery` code path, then proves the downstream consequence (recovery gate opens after eating).

**Assertion surface**:
1. Decision trace: initial ranking contains both goals, `Wash` has the higher motive score, bread is promoted to `Critical`, and bread is selected
2. Action trace: `eat` commits before any `wash` commit
3. Authoritative world state: hunger drops below High threshold, then wound severity decreases
4. Agent alive throughout
5. Determinism: replay companion

**Scenario-isolation choice**:
- Default `UtilityProfile` with equal weights — keeps weights equal, but motive still differs because dirtiness pressure is higher; the intended proof is that priority class promotion beats that higher motive
- Thirst/fatigue/bladder at 0 — eliminates competing goals that could also be recovery-boosted
- No other agents — no social, trade, or combat interactions
- Single location (VILLAGE_SQUARE) — no travel

## Preferred Placement

Scenario 29 in `golden_emergent.rs` — it is a cross-system chain (needs → wounds → identity preservation) matching the emergence file's purpose.

Scenario 30 in `golden_combat.rs` — it directly tests wound recovery mechanics and the recovery gate, matching the combat file's wound lifecycle coverage.

## Component Registration

No production component or record changes are allowed in this spec.

Test-only harness additions are permitted if they are generic helpers for:
- wound list seeding
- deprivation exposure seeding
- state-delta observation

## SystemFn Integration

No production SystemFn changes are expected.

The implementation should use the existing live stack:
- needs_system dispatch (deprivation exposure, threshold firing, worsen_or_create)
- combat system dispatch (wound progression, recovery gate)
- candidate generation
- ranking (promote_for_clotted_wound_recovery)
- planner search
- eat/wash action execution

If implementation pressure suggests changing live system behavior to make these tests pass, stop and reassess. The purpose of this spec is coverage for already-intended architecture, not a test-driven behavior rewrite.

## Cross-System Interactions (Principle 24)

### Scenario 29 chain:
1. Needs system reads `HomeostaticNeeds` + `DeprivationExposure` + `MetabolismProfile`
2. Hunger stays above critical → exposure counter increments
3. Counter reaches `starvation_tolerance_ticks` → `worsen_or_create_deprivation_wound` writes wound
4. `WoundList` component updated via `WorldTxn` → event committed to log
5. Exposure counter resets → accumulates again → fires again → same wound worsened

### Scenario 30 chain:
1. AI ranking reads `WoundList` → derives `has_clotted_wounds: true`
2. AI ranking reads `HomeostaticNeeds` → classifies hunger as `High`
3. `promote_for_clotted_wound_recovery` promotes hunger goal from `High` to `Critical`
4. Agent selects eat-Bread over wash (Critical > High)
5. Eat action commits → hunger relief applied → hunger drops below High threshold
6. Combat system reads `HomeostaticNeeds` → `recovery_conditions_met()` returns true
7. Wound severity decreases by `natural_recovery_rate` each tick

No test should inject wounds, modify needs, or bypass the recovery gate after the scenario begins.

## FND-01 Section H

### H.1 Information-Path Analysis

| Information | Source | Path |
|-------------|--------|------|
| Deprivation exposure count (Scenario 29) | `DeprivationExposure` component | needs_system reads → increments → fires threshold |
| Wound worsening (Scenario 29) | `worsen_or_create_deprivation_wound` | needs_system → find_deprivation_wound_mut → severity update → WorldTxn commit |
| Clotted wound state (Scenario 30) | `WoundList` component | AI ranking reads via belief view → `has_clotted_wounds` |
| Recovery gate satisfaction (Scenario 30) | `HomeostaticNeeds` component | eat action reduces hunger → combat system reads needs → `recovery_conditions_met` |

### H.2 Positive-Feedback Analysis

**Loop 1: worsening severity → increased pain pressure → stronger goal motivation**
- Deprivation worsening increases wound severity, which increases pain pressure, which could drive stronger goal ranking for pain-relief goals. However, in Scenario 29 there is no food and no plannable pain-relief action, so the loop does not amplify.

**Loop 2: eating → recovery → reduced pressure → less urgency to eat**
- In Scenario 30, eating satisfies hunger AND enables recovery. This is a stabilizing (negative) feedback loop, not an amplifying one.

No positive-feedback loops require dampening in either scenario.

### H.3 Concrete Dampeners

- `wound_capacity` caps total wound accumulation (physical limit)
- `starvation_tolerance_ticks` gates deprivation firing frequency (physical time delay)
- `recovery_conditions_met` gates wound healing on physical need satisfaction (not abstract cooldowns)
- Eat action duration (physical time cost)
- `natural_recovery_rate` limits healing speed per tick (physical metabolic rate)

### H.4 Stored State vs Derived

**Stored**
- `WoundList` (wounds with id, severity, inflicted_at, cause)
- `DeprivationExposure` (per-need critical tick counters)
- `HomeostaticNeeds` (current need levels)
- `CombatProfile` (wound_capacity, recovery_rate)
- `MetabolismProfile` (tolerance ticks, metabolism rates)

**Derived**
- `has_clotted_wounds` — computed from WoundList each tick in ranking
- `GoalPriorityClass` — derived from need level vs threshold band
- Recovery-boosted priority — derived from base priority + clotted wound state
- `recovery_conditions_met` — derived from needs vs thresholds + combat state
- Wound count invariant — observed assertion, not stored

## Acceptance Criteria

1. We have a golden that proves wound count stays at 1 across two deprivation fires (consolidation, not duplication), with preserved WoundId and increasing severity.
2. We have a golden that proves an agent with clotted wounds eats before washing when both goals map to `High` priority, because eat is recovery-boosted to `Critical`.
3. Scenario 30 proves that eating enables wound recovery (severity decreases after hunger drops below High threshold).
4. Both suites use the real AI loop, needs system, and combat system — no test-only shortcuts.
5. Both suites have deterministic replay companions.
6. `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` are updated to reflect both new scenarios.

## Tickets

### S17-001: Deprivation Wound Worsening Consolidates Not Duplicates

**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None

**Deliverable**:
- `golden_deprivation_wound_worsening_consolidates_not_duplicates` in `golden_emergent.rs`
- Deterministic replay companion

**Verification**:
1. `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening`
2. `cargo test -p worldwake-ai --test golden_emergent`
3. `cargo test -p worldwake-ai`

---

### S17-002: Recovery-Aware Priority Boost Eats Before Wash

**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None

**Deliverable**:
- `golden_recovery_aware_boost_eats_before_wash` in `golden_combat.rs`
- Deterministic replay companion

**Verification**:
1. `cargo test -p worldwake-ai --test golden_combat golden_recovery_aware_boost`
2. `cargo test -p worldwake-ai --test golden_combat`
3. `cargo test -p worldwake-ai`

---

### S17-003: Golden E2E Docs Catch-Up

**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None

**Deliverable**:
- Update `docs/golden-e2e-coverage.md` with Scenarios 29 and 30 in the coverage matrix
- Update `docs/golden-e2e-scenarios.md` with detailed scenario descriptions

**Verification**:
1. `python3 scripts/golden_inventory.py --write --check-docs`

## Critical Files

| File | Role |
|------|------|
| `specs/S17-wound-lifecycle-golden-suites.md` | this spec |
| `crates/worldwake-ai/tests/golden_emergent.rs` | Scenario 29 target |
| `crates/worldwake-ai/tests/golden_combat.rs` | Scenario 30 target |
| `crates/worldwake-ai/tests/golden_harness/mod.rs` | existing helpers to reuse |
| `crates/worldwake-systems/src/needs.rs` | `worsen_or_create_deprivation_wound` (Scenario 29 code path) |
| `crates/worldwake-ai/src/ranking.rs` | `promote_for_clotted_wound_recovery` (Scenario 30 code path) |
| `crates/worldwake-systems/src/combat.rs` | `recovery_conditions_met` (Scenario 30 recovery gate) |
| `docs/golden-e2e-coverage.md` | coverage matrix update |
| `docs/golden-e2e-scenarios.md` | scenario catalog update |

## Verification

1. `cargo test -p worldwake-ai --test golden_emergent golden_deprivation_wound_worsening`
2. `cargo test -p worldwake-ai --test golden_combat golden_recovery_aware_boost`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `python3 scripts/golden_inventory.py --write --check-docs`

## Implementation Order

S17-001 → S17-002 → S17-003
