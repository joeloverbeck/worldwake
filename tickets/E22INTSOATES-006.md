# E22INTSOATES-006: T21 — Ruler Death → Office Vacancy → Patrol Gap → Route Predation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None
**Deps**: E22INTSOATES-001

## Problem

Political goldens test succession mechanics in isolation. T21 chains vacancy through patrol degradation into economic consequences — the FOUNDATIONS Section F canonical regression scenario. This proves that office vacancy, guard goal switching, patrol gap, and bandit predation emerge from general rules without scenario-specific triggers.

## Assumption Reassessment (2026-03-31)

1. `OfficeData` with `vacancy_since` field exists — confirmed in core.
2. `OfficeForceProfile` with `uncontested_hold_ticks` exists — confirmed.
3. `OfficeForceState` with `control_since` exists — confirmed.
4. `SuccessionLaw::Force` variant exists — confirmed.
5. `PatrolRoute` with `assigned_places` exists — confirmed in `crates/worldwake-core/src/patrol.rs`.
6. `PatrolProfile` with `vigilance`, `patrol_motive_weight`, `route_adaptation_sensitivity` exists — confirmed.
7. `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice` exist — confirmed.
8. `GoalKind::Patrol` exists — confirmed.
9. `BanditCamp`, `PursuitProfile`, `CombatProfile` exist — confirmed.
10. `MerchandiseProfile` exists — confirmed.
11. Decision trace can show goal switching between patrol and political goals — confirmed via `DecisionTraceSink`.
12. Existing political goldens (`golden_offices.rs`, `golden_emergent.rs` Suites 5–12) test succession in isolation. T21 adds: vacancy → guard political distraction → patrol gap → merchant predation → supply disruption.
13. T21 exercises Combat→Social→Travel→Needs domains (≥ 4).
14. No adjacent contradictions.
15. Tick budget: ≤ 7200 ticks (5 days). Ruler killed at tick 0, succession within 2880 ticks, downstream effects observable.

## Architecture Check

1. T21 exercises existing succession, patrol, combat, and trade systems through their standard interfaces. The chain emerges because guard agents have `UtilityProfile` weights that allow political goals to outrank patrol goals during vacancy. No forced goal switching.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Ruler death → authoritative component state (`DeadAt` on ruler)
2. Office vacancy → authoritative component state (`OfficeData.vacancy_since` transitions from `None` to `Some(Tick(N))`)
3. Guard political goals → decision trace (≥ 1 guard generates `ClaimOffice` or `SupportCandidateForOffice` that competes with `Patrol`)
4. Patrol gap → authoritative world state (no guard at GateRoad for ≥ 100 consecutive ticks, measured by scanning placement relations)
5. Merchant predation → action trace + event-log delta (combat at GateRoad without guard presence)
6. Supply disruption → authoritative world state (cargo loss or merchant injury)
7. Succession completion → authoritative component state (`vacancy_since` returns to `None` within 2880 ticks)
8. Cross-domain ≥ 4 → event-log scan ({Combat, Social, Travel, Needs})
9. No abstract assertions → zero references to "public order", "morale", or derived metrics
10. Determinism → state hash comparison across 2 seeds

## What to Change

### 1. Add T21 scenario to `crates/worldwake-ai/tests/golden_integration.rs`

- Build 6-place topology: RulersHall, Market, GateRoad, BanditForest, GuardPost, Farm
- Ruler office entity with `OfficeData { succession_law: Force, vacancy_since: None }`, `OfficeForceProfile`
- Ruler agent with fragile `CombatProfile` (low wound capacity)
- 2 claimant agents with faction membership and `UtilityProfile` with non-zero `enterprise_weight`
- 3 guard agents with `PatrolRoute { assigned_places: [GateRoad, Market, GuardPost] }`, `PatrolProfile`
- 2 bandits at BanditForest with `BanditCamp`, `PursuitProfile`, `CombatProfile`
- Merchant at Market with `MerchandiseProfile` and goods
- Inject lethal combat event killing ruler at tick 0
- Run up to 7200 ticks
- Enable decision tracing on driver
- Verify full causal chain per spec
- `fn run_t21_ruler_death_patrol_gap(seed: Seed) -> (StateHash, StateHash)`
- Two `#[test]` functions

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Changes to succession, patrol, combat, or trade systems
- Modifying existing political golden tests
- Any engine code changes

## Acceptance Criteria

### Tests That Must Pass

1. `t21_ruler_death_patrol_gap_seed_1` — ruler death → vacancy → guard distraction → patrol gap → predation
2. `t21_ruler_death_patrol_gap_seed_2` — determinism verification
3. `OfficeData.vacancy_since` changes exactly once from `None` to `Some`
4. New office holder emerges within 2880 ticks
5. ≥ 1 guard generates political goal competing with patrol (verified via decision trace)
6. No guard at GateRoad for ≥ 100 consecutive ticks
7. Merchant encounters bandit without guard presence at GateRoad
8. Event log crosses ≥ 4 distinct `ActionDomain` values from {Combat, Social, Travel, Needs}
9. No assertion references "public order", "morale", or any derived metric
10. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Office vacancy is caused only by ruler death — not forced by test logic
2. Guard goal switching is driven by `UtilityProfile` weights — not scripted
3. All assertions reference component values, event records, or entity positions — no abstract scores

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs::t21_ruler_death_patrol_gap_seed_1` — proves FOUNDATIONS Section F regression scenario
2. `crates/worldwake-ai/tests/golden_integration.rs::t21_ruler_death_patrol_gap_seed_2` — determinism

### Commands

1. `cargo test -p worldwake-ai --test golden_integration -- t21`
2. `cargo test --workspace`
