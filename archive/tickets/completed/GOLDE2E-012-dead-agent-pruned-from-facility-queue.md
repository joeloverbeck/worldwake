# GOLDE2E-012: Dead Agent Pruned from Facility Queue

**Status**: ✅ COMPLETED
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: None expected — this should be golden coverage over the existing queue/death architecture
**Deps**: None (facility queue, exclusive workstation queueing, and deprivation death infrastructure already exist)

## Problem

The facility queue system has focused unit tests for pruning dead agents, but this specific interaction has not been proven end-to-end through the real AI + deprivation + exclusive-facility queue loop. An agent dying while waiting in a facility queue should be pruned by `prune_invalid_waiters()`, and the next living agent in line should remain the canonical next head for promotion.

## Report Reference

Backlog item **P-NEW-9** in `reports/golden-e2e-coverage-analysis.md` (Tier 3, composite score 2).

## Assumption Reassessment (2026-03-13)

1. `prune_invalid_waiters()` exists in `crates/worldwake-systems/src/facility_queue.rs` and already removes dead, departed, and deallocated queued actors at system tick time.
2. Deprivation-driven death is already proven end-to-end in `crates/worldwake-ai/tests/golden_combat.rs` scenario 8 (`golden_death_cascade_and_opportunistic_loot`) and scenario 8b (`golden_death_while_traveling`).
3. Exclusive facility queue contention, promotion, patience timeout, and grant expiry are already proven end-to-end in `crates/worldwake-ai/tests/golden_production.rs` scenarios 9, 9b, and 9c.
4. The stale assumption in the original ticket/report is file placement: this scenario belongs with the existing facility-queue golden scenarios in `golden_production.rs`, not in `golden_combat.rs`.
5. No engine gap is currently evident from the reassessment; the expected value is architectural coverage of the existing queue/death boundary, not a redesign.

## Architecture Check

1. Uses existing infrastructure and preserves the current state-mediated architecture: deprivation resolves through needs/combat state, and queue cleanup resolves through `facility_queue_system()`.
2. The pruning is authoritative and system-owned, so the AI should not need a special-case death/queue alias path.
3. The clean architecture remains the current one: death invalidates actor state, queue maintenance observes authoritative actor validity, and promotion proceeds from the queue state on subsequent ticks.

## Engine-First Mandate

If implementing this e2e suite reveals that the interaction between death resolution and facility queue pruning is incomplete or creates race conditions — do NOT patch around it. Instead, design and implement a comprehensive architectural solution that handles death-during-queue cleanly. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_production.rs`

**Setup**: Three agents at an exclusive facility. Agent A holds the initial grant. Agent B (fragile, queued, high deprivation) and Agent C (healthy, queued behind B) wait for the same exclusive orchard.

**Assertions**:
- Agent B enters the authoritative queue ahead of Agent C.
- Agent B dies from deprivation wounds while still waiting rather than while granted or actively harvesting.
- `prune_invalid_waiters()` removes Agent B from the queue without any test-only or AI-side cleanup.
- Agent C becomes the living queue head and later receives a real `QueueGrantPromoted` event.
- Apple conservation holds throughout the scenario.
- The scenario stays in the production/facility-queue golden suite unless the implementation reveals a real cross-domain architectural reason to relocate it.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify only if a small shared helper materially reduces duplication)

## Out of Scope

- Agent departing facility while in queue (already unit-tested separately)
- Multiple simultaneous deaths in queue
- General corpse/loot behavior after queue death unless needed to prove queue correctness

## Acceptance Criteria

### Tests That Must Pass

1. `golden_dead_agent_pruned_from_facility_queue` — dead queued actor is pruned and the next living queued actor receives promotion
2. Relevant golden subset covering adjacent behavior passes first
3. `cargo test --workspace`
4. `cargo clippy --workspace`

### Invariants

1. All behavior is emergent from real deprivation and real queue state; no manual kill or direct queue mutation after setup
2. Queue order remains deterministic after the dead waiter is removed
3. Conservation holds

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P-NEW-9 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs::golden_dead_agent_pruned_from_facility_queue` — proves deprivation death while queued, authoritative pruning, and next-waiter promotion

### Commands

1. `cargo test -p worldwake-ai golden_dead_agent_pruned`
2. `cargo test -p worldwake-ai golden_exclusive_queue_contention`
3. `cargo test -p worldwake-ai golden_death_`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-13
- **What actually changed**: Added the new golden scenario in `crates/worldwake-ai/tests/golden_production.rs` together with a deterministic replay companion. Updated `reports/golden-e2e-coverage-analysis.md` to move this coverage into the proven scenario set and remove `P-NEW-9` from the backlog.
- **Deviations from original plan**: The reassessment confirmed the original file target was stale. The scenario belongs in `golden_production.rs` with the existing exclusive-facility queue coverage, not in `golden_combat.rs`. No engine changes and no harness changes were needed.
- **Verification results**:
  - `cargo test -p worldwake-ai golden_dead_agent_pruned_from_facility_queue`
  - `cargo test -p worldwake-ai golden_`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
