# E22INTSOATES-007: T33 — Office Vacancy → Patrol Gap → Crime Opportunity → Recovery

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None
**Deps**: E22INTSOATES-001

## Problem

No existing golden chains vacancy through patrol degradation into crime opportunity and back to recovery after succession. T33 tests the full feedback loop with its physical dampener: succession completion restores patrol, which re-deters crime. This is a stricter instantiation of FOUNDATIONS Section F that proves the dampener works.

## Assumption Reassessment (2026-03-31)

1. `OfficeData`, `OfficeForceProfile`, `OfficeForceState` exist — confirmed.
2. `SuccessionLaw::Force` variant exists — confirmed.
3. `PatrolRoute`, `PatrolProfile` with `patrol_motive_weight` exist — confirmed.
4. `TheftDispositionProfile` with `witness_risk_penalty` exists — confirmed.
5. `GoalKind::StealItem` exists — confirmed.
6. `GoalKind::ClaimOffice`, `GoalKind::SupportCandidateForOffice` exist — confirmed.
7. `GoalKind::Patrol` exists — confirmed.
8. Decision trace records goal switching and `witness_risk_penalty` evaluation — confirmed via `DecisionTraceSink`.
9. T33 vs T21: T21 focuses on merchant predation during vacancy. T33 focuses on theft during vacancy AND recovery after succession. Different causal endpoint.
10. T33 exercises Combat→Social→Travel→Transport→Epistemic (≥ 5 domains).
11. Thief with `witness_risk_penalty: Permille(900)` is highly deterred by guard presence — will only steal when no guards are present. This is the core mechanism: guard absence → theft enabled, guard return → theft suppressed.
12. No adjacent contradictions.
13. Tick budget: ≤ 7200 ticks (5 days). Ruler killed → vacancy → succession → guard return → theft suppression.
14. Succession completes within `uncontested_hold_ticks` plus travel/claim delay (concrete profile values).

## Architecture Check

1. T33 exercises the same systems as T21 but adds crime/theft and verifies the full feedback loop including recovery. The dampener (succession → patrol return → theft deterrence) is physical, not a numeric clamp.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Ruler death → authoritative component state (`DeadAt` on ruler)
2. Office vacancy → authoritative component state (`OfficeData.vacancy_since = Some`)
3. Guard political distraction → decision trace (political goal outranks patrol)
4. Patrol gap → authoritative world state (no guard at Market for extended period)
5. Theft during vacancy → action trace (StealItem action committed between `vacancy_since = Some` and succession completion)
6. No pre-vacancy theft → event-log scan (no theft events before ruler death)
7. Succession completion → authoritative component state (`vacancy_since` returns to `None`)
8. Guard patrol resumption → authoritative world state (guard returns to Market after succession)
9. Theft suppression post-recovery → decision trace on thief (`witness_risk_penalty` re-applied, `StealItem` not generated)
10. Cross-domain ≥ 5 → event-log scan ({Combat, Social, Travel, Transport, Epistemic})
11. Determinism → state hash comparison across 2 seeds

## What to Change

### 1. Add T33 scenario to `crates/worldwake-ai/tests/golden_integration.rs`

- Build 5-place topology: RulersHall, Market, Road, Farm, GuardPost
- Ruler office entity with `OfficeData { succession_law: Force }`, `OfficeForceProfile { uncontested_hold_ticks: NonZeroU32(20) }`, `OfficeForceState { control_since: Some(Tick(0)) }`
- Ruler agent
- 2 guard agents with `PatrolRoute { assigned_places: [Market, Road] }`, `PatrolProfile { patrol_motive_weight: Permille(700) }`
- 1 thief at Road with `TheftDispositionProfile { witness_risk_penalty: Permille(900) }` (highly deterred)
- Merchant at Market with goods
- Kill ruler to trigger vacancy
- Enable decision tracing
- Verify: no theft before ruler death (guards deter)
- Verify: theft occurs during vacancy (guards distracted)
- Verify: thief decision trace shows `witness_risk_penalty` evaluation changing based on guard presence/absence
- Verify: after succession + guard return, theft suppressed
- `fn run_t33_vacancy_crime_recovery(seed: Seed) -> (StateHash, StateHash)`
- Two `#[test]` functions

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Changes to succession, patrol, crime, or theft systems
- Modifying existing political or crime golden tests
- Any engine code changes

## Acceptance Criteria

### Tests That Must Pass

1. `t33_vacancy_crime_recovery_seed_1` — vacancy → theft → succession → patrol return → theft suppression
2. `t33_vacancy_crime_recovery_seed_2` — determinism verification
3. Theft event occurs during vacancy period
4. No theft event occurs before ruler death
5. Thief decision traces show `witness_risk_penalty` evaluation changing based on guard presence/absence
6. Succession completes within reasonable delay
7. Event log crosses ≥ 5 `ActionDomain` values from {Combat, Social, Travel, Transport, Epistemic}
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Theft deterrence is driven by guard physical presence and `witness_risk_penalty` — not abstract "public order"
2. Physical dampener: succession → patrol return → crime suppression (Principle 11)
3. All assertions reference component values, decision traces, and event records — no derived metrics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs::t33_vacancy_crime_recovery_seed_1` — proves full vacancy→crime→recovery feedback loop
2. `crates/worldwake-ai/tests/golden_integration.rs::t33_vacancy_crime_recovery_seed_2` — determinism

### Commands

1. `cargo test -p worldwake-ai --test golden_integration -- t33`
2. `cargo test --workspace`
