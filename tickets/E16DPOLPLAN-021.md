# E16DPOLPLAN-021: Golden Scenario 20 — Information locality for political facts

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: None
**Deps**: E16DPOLPLAN-007

## Problem

No golden test proves Principle 7 (information locality) for political facts: an agent at a distant location should not generate political goals for an office it hasn't learned about.

## Assumption Reassessment (2026-03-18)

1. `emit_political_candidates` in `candidate_generation.rs:195` iterates `known_entity_beliefs` — confirmed
2. Agents without beliefs about an office should not generate `ClaimOffice` — confirmed
3. Tell/rumor/report events can deliver office vacancy information to distant agents — confirmed from E15
4. `known_entity_beliefs` is populated by perception and social transmission — confirmed from E14/E15

## Architecture Check

1. Tests the critical belief-gating path for political goal generation
2. Two-phase test: before belief → no political goals; after belief update → political goals emerge
3. Verifies Principle 7 compliance at the candidate generation level

## What to Change

### 1. Add to `golden_offices.rs`

- **Setup**: Office becomes vacant at VillageSquare. Agent at BanditCamp (distant). Agent has no prior beliefs about the office.
- **Phase 1**: Run ticks — verify agent generates NO political goals (no ClaimOffice, no SupportCandidateForOffice). Check event log for absence of political events.
- **Phase 2**: Deliver office vacancy information to agent (via Tell event or direct belief seeding). Run more ticks — verify agent now generates `ClaimOffice` and travels to jurisdiction.
- **Assertions**: No political goal events before belief update. Political goal events appear after belief update. Agent travels to jurisdiction only after learning about vacancy.

> **Golden E2E documentation**: Review and update `docs/golden-e2e-coverage.md` and `docs/golden-e2e-scenarios.md` as necessary to reflect the new scenario(s) added by this ticket.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- Belief propagation mechanism implementation (already done in E14/E15)
- Tell action testing (already covered in golden_social.rs)
- Changes to perception system
- Changes to production code

## Acceptance Criteria

### Tests That Must Pass

1. `golden_information_locality_for_political_facts` — no political events before belief, political events after
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Principle 7: agents cannot act on information they haven't received
2. `emit_political_candidates` reads from `known_entity_beliefs`, not world state
3. Political goal generation is belief-gated, not world-state-gated

## Test Plan

### New/Modified Tests

1. `golden_offices.rs::golden_information_locality_for_political_facts`

### Commands

1. `cargo test -p worldwake-ai golden_offices`
2. `cargo test --workspace`
