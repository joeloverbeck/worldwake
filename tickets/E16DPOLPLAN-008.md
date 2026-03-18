# E16DPOLPLAN-008: Golden Scenario 11 + 11b — Simple office claim via DeclareSupport + deterministic replay

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E16DPOLPLAN-007, E16DPOLPLAN-022, E16DPOLPLAN-023, E16DPOLPLAN-024, E16DPOLPLAN-025

## Problem

No golden test covers the simplest political path: a single agent claiming a vacant office through self-support declaration and succession resolution.

## Assumption Reassessment (2026-03-18)

1. `GoalKind::ClaimOffice { office }` exists in candidate_generation — confirmed
2. `PlannerOpKind::DeclareSupport` has working `apply_planner_step` — confirmed (already handled)
3. `succession_system()` resolves vacant offices with Support law after succession_period ticks — confirmed from E16
4. `enterprise_weight` in `UtilityProfile` drives political goal generation — confirmed
5. `SuccessionLaw::Support` and `succession_period` fields on `OfficeData` — confirmed

## Architecture Check

1. Tests real AI loop (AgentTickDriver + AutonomousControllerRuntime) + real system dispatch — no mocking
2. Scenario 11b verifies same seed produces identical world + event log hashes (determinism)
3. **Terminal kind note (post E16DPOLPLAN-024)**: This is an uncontested scenario (single agent, no competitor). The coalition-aware planner now produces `GoalSatisfied` (not `ProgressBarrier`) for solo DeclareSupport when no competitor exists. Any assertions about `PlanTerminalKind` should expect `GoalSatisfied`.

## What to Change

### 1. New test file: `crates/worldwake-ai/tests/golden_offices.rs`

- **Scenario 11**: Vacant office (Support law, period=5) at VillageSquare. Single sated agent with `enterprise_weight=pm(800)`, eligible (no faction rule). Agent generates `ClaimOffice` -> plans `DeclareSupport(self)` -> executes -> after succession period, Politics system installs agent as holder.
- **Scenario 11b**: Same seed, verify identical world + event log hashes.

### 2. Assertions

- Office holder == agent after N ticks
- Event log contains Political + installation tags
- Determinism: hash comparison for 11b

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (new)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Multi-agent competition (Scenario 12)
- Bribe/Threaten scenarios
- Changes to production code
- Changes to golden_harness (done in E16DPOLPLAN-007)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_simple_office_claim_via_declare_support` — agent installed as holder
2. `golden_simple_office_claim_deterministic_replay` — identical hashes
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agent plans from beliefs, never world state (Principle 10)
2. Deterministic: same seed -> same outcome
3. Conservation holds throughout

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_simple_office_claim_via_declare_support`
2. `golden_offices.rs::golden_simple_office_claim_deterministic_replay`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`

## Dependency Chain Note

This ticket depends on the coalition-aware planner changes from E16DPOLPLAN-022 through E16DPOLPLAN-025. Specifically, E16DPOLPLAN-024 changed uncontested DeclareSupport to produce `GoalSatisfied` instead of `ProgressBarrier`. This scenario (single agent, no competitor) exercises that path directly.
