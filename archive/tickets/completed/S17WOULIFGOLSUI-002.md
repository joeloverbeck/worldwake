# S17WOULIFGOLSUI-002: Golden Scenario 30 — Recovery-Aware Priority Boost Eats Before Wash

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: S17WOULIFGOLSUI-001 (implementation order per spec), S11 (wound lifecycle audit — `promote_for_clotted_wound_recovery`), E12 (wound schema, recovery gate), E09 (needs — eat action), E13 (decision architecture — ranking, candidate generation, planner)

## Problem

The `promote_for_clotted_wound_recovery` mechanism (where a `High`-threshold need goal is promoted to `Critical` when clotted wounds exist and the need is recovery-relevant) is only tested via focused unit tests. No golden E2E test proves this promotion affects real action selection, nor that the downstream consequence (eating opens the recovery gate → wound severity decreases) works through the live AI loop. Existing wound goldens either use sated agents (Scenario 7g — recovery gate always open) or test utility-weight differences (S07a/b — `pain_weight` vs `hunger_weight`), not threshold-based promotion.

## Assumption Reassessment (2026-03-21)

1. `promote_for_clotted_wound_recovery` exists in [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) and is called from `drive_priority()`. Focused coverage already exists in the ranking unit tests in that file, and adjacent focused candidate coverage for `Wash` exists in [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs). Existing golden coverage proves `Wash` in isolation via `golden_wash_action` in [crates/worldwake-ai/tests/golden_ai_decisions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_ai_decisions.rs) and proves wound-vs-hunger utility divergence via `golden_wound_vs_hunger_pain_first` / `golden_wound_vs_hunger_hunger_first` in [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs), but no current golden combines clotted-wound recovery promotion, wash competition, and downstream natural recovery. Confirmed by `cargo test -p worldwake-ai --test golden_combat -- --list`, [docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md), and [docs/generated/golden-e2e-inventory.md](/home/joeloverbeck/projects/worldwake/docs/generated/golden-e2e-inventory.md).
2. `recovery_conditions_met` exists in [crates/worldwake-systems/src/combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs) and gates recovery on `hunger < thresholds.hunger.high()`, `thirst < thresholds.thirst.high()`, `fatigue < thresholds.fatigue.high()`, and `!engaged_in_combat`. `promote_for_clotted_wound_recovery` explicitly documents that it is intended to stay aligned with that authoritative combat gate.
3. This is a mixed-layer golden ticket. The intended proof chain is candidate generation -> ranking/selection -> authoritative action lifecycle -> authoritative wound-state change. Full action registries are required because both `eat` and `wash` must be available through the live harness.
4. **Ordering layer**: this should be expressed as decision-trace selection plus action lifecycle ordering, not just state-delta ordering. The compared branches are not fully symmetric in the current architecture: both start at `GoalPriorityClass::High`, but their motive scores differ under the proposed setup. With `HomeostaticNeeds::new(pm(760), pm(0), pm(0), pm(0), pm(860))` and `UtilityProfile::default()`, `eat` has motive `500 * 760 = 380_000` while `wash` has motive `500 * 860 = 430_000`. Without promotion, `wash` would outrank `eat` inside the shared `High` class. The live architectural claim is stronger than the original ticket stated: recovery-aware promotion intentionally overrides a higher wash motive by elevating the hunger branch to `Critical`.
5. No heuristic removal. This ticket adds coverage for existing behavior.
6. Not a stale-request or start-failure ticket.
7. Not a political office-claim ticket.
8. No ControlSource manipulation. Agent uses default AI control.
9. **Isolation choice**: thirst, fatigue, and bladder stay at `pm(0)` so no other recovery-relevant need can lawfully receive the same promotion. No other agents, no workstations, and a single place remove unrelated social/trade/travel/combat branches. Bread and water are directly possessed so the competition is between immediate local `eat` and `wash`, not acquisition planning.
10. Ticket corrections required before implementation:
    - replace the nonexistent `golden-e2e-coverage.md` reference with the current generated inventory/docs
    - replace the old "state delta alone is sufficient" proof shape with the repo-preferred mixed-layer proof: decision trace for ranking/selection, action trace for `eat` before `wash`, authoritative world state for recovery
    - correct the "equal motive scores / ONLY differentiator" claim; the current architecture intentionally uses class promotion to beat a stronger `wash` motive
    - use the repo's same-seed replay companion pattern, not a nonexistent `replay_and_verify` helper
11. **Concrete arithmetic / survivability**:
    - default thresholds are hunger `high = pm(750)` and dirtiness `high = pm(850)` in [crates/worldwake-core/src/drives.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/drives.rs)
    - Bread relieves hunger by `pm(260)` per unit, so one `eat` moves hunger from `pm(760)` to `pm(500)`, lawfully opening the hunger leg of the recovery gate
    - Water enables `wash`, but `Wash` is not recovery-relevant
    - the seeded clotted wound starts at `pm(200)` severity with `bleed_rate_per_tick = pm(0)`, `natural_clot_resistance = pm(0)`, and `natural_recovery_rate = pm(18)`, so once eating opens the gate the wound should begin decreasing without any production-code changes

## Architecture Check

1. Pure test-addition ticket. The current architecture is the cleaner one: `ranking.rs` contains the AI-side urgency adjustment, `combat.rs` remains the sole authority on whether wounds may recover, and the two layers communicate through concrete needs/wound state rather than direct system calls. Adding a golden that proves this cross-layer contract is more robust than moving more logic into ranking, adding special-case wash suppression, or weakening motive scoring. No production code changes are warranted by the current architecture.
2. No backwards-compatibility aliasing/shims introduced.

## Verification Layers

1. `ConsumeOwnedCommodity(Bread)` and `Wash` both exist on the initial tick, but `eat` is selected with `Critical` while `wash` remains `High` -> decision trace
2. `eat` commits before any `wash` commit -> action trace
3. Hunger falls below `thresholds.hunger.high()` after the eat commit -> authoritative world state
4. Wound severity decreases after the recovery gate opens -> authoritative world state
5. Agent eventually commits `wash` too -> action trace and/or authoritative dirtiness state
6. Deterministic replay -> replay companion

## What to Change

### 1. Add `golden_recovery_aware_boost_eats_before_wash` to `golden_combat.rs`

**Setup**:
- Single agent at `VILLAGE_SQUARE`
- One clotted wound: `Wound { id: WoundId(1), body_part: BodyPart::Torso, cause: WoundCause::Deprivation(DeprivationKind::Starvation), severity: pm(200), inflicted_at: Tick(0), bleed_rate_per_tick: pm(0) }`
- `HomeostaticNeeds::new(pm(760), pm(0), pm(0), pm(0), pm(860))` — hunger at High (above 750), dirtiness at High (above 850), everything else at 0
- Custom `MetabolismProfile`: very low rates (`pm(1)` for hunger and dirtiness, `pm(0)` for everything else) to prevent drift during test window
- `CombatProfile`: `natural_recovery_rate: pm(18)`, `natural_clot_resistance: pm(0)` — recovery proceeds after conditions met, no re-bleeding
- Default `UtilityProfile` — equal weights keep the comparison honest, but the motive scores still differ because dirtiness pressure exceeds hunger pressure; the test should prove that the recovery-aware promotion overrides that higher wash motive
- `give_commodity`: Bread quantity 3 (directly possessed) — one eat brings hunger well below High
- `give_commodity`: Water quantity 1 (directly possessed) — enables Wash affordance
- No workstations, no other agents, no recipes
- `seed_actor_local_beliefs` with `DirectObservation`
- Enable decision tracing and action tracing

**Assertions**:
1. Initial planning trace includes both `ConsumeOwnedCommodity { commodity: Bread }` and `Wash`
2. Initial planning trace ranks the Bread consume goal as `Critical` and selects it, while `Wash` remains `High`
3. Action trace shows `eat` commits before any `wash` commit
4. After the eat commit, hunger drops below `thresholds.hunger.high()` (pm(750))
5. Wound severity later decreases below the initial `pm(200)` once recovery is open
6. Agent eventually commits `wash` too (or, equivalently, dirtiness decreases after the eat-first branch has already executed)
7. Agent alive throughout

### 2. Add replay companion

Use the repo's standard same-seed replay companion pattern.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify — add Scenario 30 test function)
- `docs/generated/golden-e2e-inventory.md` (modify — mechanical inventory refresh from `scripts/golden_inventory.py --write --check-docs`)

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

1. Initial selection is `eat`, not `wash`, because recovery-aware promotion elevates the hunger branch from `High` to `Critical`
2. The setup intentionally leaves `wash` with the higher motive score inside the unpromoted class, so the test proves the architectural role of class promotion rather than a motive-score tie
3. Wound recovery activates only after eating satisfies the combat recovery gate (hunger < 750, thirst < 700, fatigue < 800, not in combat)
4. Wound severity strictly decreases after the gate opens (Principle 10: physical dampeners)
5. Agent remains alive throughout
6. Deterministic replay produces identical state hash
7. No production code modified — this is a coverage-only ticket

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_recovery_aware_boost_eats_before_wash` — proves recovery-aware promotion drives real action selection and enables downstream wound recovery
2. `crates/worldwake-ai/tests/golden_combat.rs::golden_recovery_aware_boost_eats_before_wash_replays_deterministically` — deterministic replay companion

### Commands

1. `cargo test -p worldwake-ai --test golden_combat golden_recovery_aware_boost`
2. `cargo test -p worldwake-ai --test golden_combat`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

- Completion date: 2026-03-21
- What changed:
  - Added `golden_recovery_aware_boost_eats_before_wash`
  - Added `golden_recovery_aware_boost_eats_before_wash_replays_deterministically`
  - Refreshed `docs/generated/golden-e2e-inventory.md`
  - Corrected the ticket assumptions to match the current ranking, trace, and documentation surfaces
- Deviations from original plan:
  - The ticket originally treated the scenario as a state-delta-only proof with equal motive scores. The implemented version proves the stronger live contract: `wash` starts with the higher motive score, but recovery-aware class promotion still makes `eat` win.
  - The replay companion uses the repo's current same-seed rerun pattern rather than `replay_and_verify`.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_combat golden_recovery_aware_boost_eats_before_wash -- --exact --nocapture` passed
  - `cargo test -p worldwake-ai --test golden_combat` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `python3 scripts/golden_inventory.py --write --check-docs` passed
