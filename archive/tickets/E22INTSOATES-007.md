# E22INTSOATES-007: T33 — Office Vacancy → Patrol Gap → Crime Opportunity → Recovery

**Status**: ✅ COMPLETED
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
  - RulersHall ↔ Market: 8 ticks (remote — ensures claimant installs before guards contest)
  - Market ↔ Road: 2 ticks; Road ↔ Farm: 3 ticks; RulersHall ↔ GuardPost: 8 ticks; GuardPost ↔ Market: 2 ticks
- Ruler office entity with `OfficeData { succession_law: Force, succession_period_ticks: 5 }`, `OfficeForceProfile { uncontested_hold_ticks: NonZeroU32(5) }`
- Ruler agent at RulersHall (killed at tick 0)
- 1 claimant agent at RulersHall — faction member with `enterprise_weight: pm(900)`, drives succession
- 2 guard agents with `PatrolRoute { assigned_places: [Market, Road] }`, `PatrolProfile { patrol_motive_weight: pm(550) }`, zero metabolism (guards must survive full tick budget), faction members with explicit vacancy belief seeding
- 1 thief at Road with `TheftDispositionProfile { theft_motive_weight: pm(800), witness_risk_penalty: Permille(900) }` (fully deterred by 1+ agents, steals when alone)
- Merchant at Market (human-controlled), stealable goods on ground at Road (separate from merchant to avoid bystander deterrence)
- Kill ruler to trigger vacancy; seed guards with vacancy beliefs (remote guards can't perceive vacancy via co-location)
- Enable decision + action tracing
- Verify: no theft before ruler death (guard at Road deters)
- Verify: theft occurs during vacancy (guard leaves Road for political goals)
- Verify: guard decision trace shows ClaimOffice as interrupt challenger outranking Patrol (via `ActiveAction::interrupt.top_challenger`, not only `Planning` outcomes)
- Verify: after succession + guard return to patrol point, theft suppressed (thief decision trace shows no StealItem candidates)
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

## Outcome

**Completion date**: 2026-03-31

**What changed**:
- Added T33 scenario to `crates/worldwake-ai/tests/golden_integration.rs` (~400 lines): `run_t33_vacancy_crime_recovery()`, `t33_vacancy_crime_recovery_seed_1`, `t33_vacancy_crime_recovery_seed_2`
- Added 3 doc notes to `docs/golden-e2e-testing.md`: human-bystander theft deterrence, Force Succession Calibration section, ActiveAction interrupt trace guidance

**Deviations from original plan**:
- `uncontested_hold_ticks: 5` (ticket said 20) — required to ensure claimant installs before guards contest
- Added dedicated claimant agent at RulersHall (not in original ticket) — guards alone couldn't drive succession reliably
- Guard metabolism zeroed; Needs domain coverage moved to claimant agent — guards died from deprivation over long tick budgets
- Stealable goods placed at Road instead of Market — merchant at Market counted as witness, permanently deterring thief
- RulersHall↔Market travel time 8 ticks (not ~2) — topology separation ensures claimant installs before guards arrive
- Guard vacancy beliefs seeded explicitly — remote guards can't perceive vacancy via co-location perception (Principle 7)
- Political distraction verified via `ActiveAction::interrupt.top_challenger` — guards never re-entered planning pipeline during fast succession

**Verification results**:
- `cargo test -p worldwake-ai --test golden_integration -- t33`: 2 passed
- `cargo test -p worldwake-ai --test golden_integration`: 29 passed, 0 failed
- `cargo test -p worldwake-ai`: all passed
- `cargo build --workspace`: clean
