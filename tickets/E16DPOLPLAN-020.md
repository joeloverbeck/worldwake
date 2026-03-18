# E16DPOLPLAN-020: Golden Scenario 19 — Incumbent defense

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None
**Deps**: E16DPOLPLAN-003, E16DPOLPLAN-007

## Problem

No golden test covers office retention dynamics: an incumbent with fewer supporters being replaced by a challenger who builds a coalition.

## Assumption Reassessment (2026-03-18)

1. `office_holder` relation can be pre-seeded — confirmed
2. Succession system re-evaluates support declarations periodically — confirmed
3. A challenger with more support replaces incumbent — confirmed (support counting)
4. Bribe planning (E16DPOLPLAN-003) enables coalition building — confirmed dependency

## Architecture Check

1. Three-agent test: A (incumbent), B (challenger), C (bribe target)
2. Tests the full challenger loop: ClaimOffice → Bribe → coalition exceeds incumbent → replacement

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Agent A already holds office (installed as holder via direct relation setup). Agent B eligible, `enterprise_weight=pm(900)`, holds bread for bribing. Agent C at jurisdiction as potential bribe target. All sated.
- **Expected**: A has 1 support (self). B claims via `ClaimOffice` → bribes C → declares for self. B accumulates support from self + C = 2. A has only self-support = 1. B's coalition exceeds A's → Politics system installs B.
- **Assertions**: B is new office holder. Support count comparison determines outcome.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Incumbent actively defending (bribing/threatening to retain supporters)
- Tiebreaking rules
- Changes to succession system
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `golden_incumbent_defense` — B replaces A as holder after building coalition
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Support counting determines outcome
2. Incumbent has no special advantage beyond existing support
3. Conservation holds (bribe commodity transfer)

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_incumbent_defense`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`
