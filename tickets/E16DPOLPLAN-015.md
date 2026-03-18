# E16DPOLPLAN-015: Golden Scenario 18 + 18b — Force succession + deterministic replay

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-007

## Problem

No golden test covers `SuccessionLaw::Force` — where the Politics system installs the sole living eligible agent without support counting.

## Assumption Reassessment (2026-03-18)

1. `SuccessionLaw::Force` exists on `OfficeData` — confirmed
2. `succession_system()` handles Force law: finds eligible agents, installs sole candidate — confirmed
3. `DeadAt` component exists — API: `world.set_component_dead_at(agent, DeadAt(Tick(0)))` — confirmed
4. Dead agents are excluded from eligibility checks — confirmed

## Architecture Check

1. Tests an alternative succession law path (Force vs Support)
2. Uses `DeadAt` to remove one candidate, leaving sole eligible agent
3. Scenario 18b verifies determinism

## What to Change

### 1. Add to `golden_offices.rs`

- **Scenario 18**: Office with `SuccessionLaw::Force` at VillageSquare. Agent A eligible, sated. Agent B has `DeadAt(Tick(0))`. After succession period, A is sole living eligible → installed.
- **Scenario 18b**: Same seed, verify identical world + event log hashes.
- **Assertions**: A is office holder. No `DeclareSupport` events (force law doesn't use support counting).

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Support succession law (Scenario 11)
- Multi-candidate force succession (contested force)
- Changes to succession logic
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `golden_force_succession_sole_eligible` — A installed, no DeclareSupport events
2. `golden_force_succession_deterministic_replay` — identical hashes
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Force law never uses support counting
2. Dead agents excluded from eligibility
3. Deterministic: same seed → same outcome

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_force_succession_sole_eligible`
2. `golden_offices.rs::golden_force_succession_deterministic_replay`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`
