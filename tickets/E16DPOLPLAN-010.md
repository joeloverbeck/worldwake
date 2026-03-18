# E16DPOLPLAN-010: Golden Scenario 13 — Bribe -> support coalition (full-quantity transfer)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None
**Deps**: E16DPOLPLAN-003, E16DPOLPLAN-007

## Problem

No golden test covers the full bribe political loop: planner selects Bribe -> commodity transfer -> target's AI generates support -> succession.

## Assumption Reassessment (2026-03-18)

1. `enumerate_bribe_payloads` offers full commodity stock per payload — confirmed
2. `commit_bribe` transfers commodity and increases loyalty — confirmed
3. Bribe planning arm (E16DPOLPLAN-003) deducts commodity and adds hypothetical support — confirmed dependency
4. After bribe, target's AI should generate `SupportCandidateForOffice` from loyalty increase — confirmed design

## Architecture Check

1. Tests the full cross-system chain: AI goal -> planner Bribe op -> commodity transfer -> conservation -> loyalty -> target AI -> support -> succession
2. Commodity conservation is explicitly verified (full stock transfer, 5->0)

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Vacant office. Agent A eligible, `enterprise_weight=pm(900)`, holds 5 bread. Agent B at jurisdiction, no initial loyalty to A. Both sated.
- **Expected**: A generates `ClaimOffice`. Planner finds `Bribe(B, bread)` -> `DeclareSupport(self)`. A bribes B (all 5 bread transfer). B's loyalty increases. B generates `SupportCandidateForOffice(A)` and declares support. Politics system installs A.
- **Assertions**: A is office holder. A's bread == 0. B has A's former bread. Conservation holds. A has support from self + B.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Threaten scenarios
- Partial commodity transfer (not supported by `enumerate_bribe_payloads`)
- Changes to production code
- Changes to bribe handler

## Acceptance Criteria

### Tests That Must Pass

1. `golden_bribe_support_coalition` — A installed, bread transferred, conservation holds
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Conservation: total bread in world unchanged
2. Full stock transfer: A's bread == 0 after bribe
3. Belief-only planning (Principle 10)

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_bribe_support_coalition`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`
