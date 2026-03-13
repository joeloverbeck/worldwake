# GOLDE2E-012: Dead Agent Pruned from Facility Queue

**Status**: PENDING
**Priority**: LOW
**Effort**: Medium
**Engine Changes**: None expected — `prune_invalid_waiters()` exists in facility_queue_system
**Deps**: None (facility queue and death infrastructure exist)

## Problem

The facility queue system has unit tests for pruning dead agents, but this has never been proven end-to-end through the real AI + combat/deprivation loop. An agent dying while waiting in a facility queue should be pruned by `prune_invalid_waiters()`, and the next agent in line should be promoted.

## Report Reference

Backlog item **P-NEW-9** in `reports/golden-e2e-coverage-analysis.md` (Tier 3, composite score 2).

## Assumption Reassessment (2026-03-13)

1. `prune_invalid_waiters()` exists in `facility_queue_system()` and removes dead/departed/deallocated actors.
2. Death from deprivation is proven in scenario 8.
3. Facility queue contention is proven in scenario 9.
4. This test combines both: death + queue interaction.

## Architecture Check

1. Uses existing infrastructure — no new architecture.
2. The pruning is authoritative (system-level), so the AI does not need special handling.

## Engine-First Mandate

If implementing this e2e suite reveals that the interaction between death resolution and facility queue pruning is incomplete or creates race conditions — do NOT patch around it. Instead, design and implement a comprehensive architectural solution that handles death-during-queue cleanly. Document any engine changes in the ticket outcome.

## What to Change

### 1. New golden test in `golden_combat.rs`

**Setup**: Three agents at an exclusive facility. Agent A holds the grant. Agent B (fragile, high deprivation) and Agent C are waiting in queue. Agent B dies from deprivation while waiting.

**Assertions**:
- Agent B enters the queue.
- Agent B dies from deprivation wounds.
- `prune_invalid_waiters()` removes Agent B from the queue.
- Agent C advances in queue position and eventually receives a grant.
- Conservation holds.

## Files to Touch

- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, if helpers needed)

## Out of Scope

- Agent departing facility while in queue (unit-tested separately)
- Multiple simultaneous deaths in queue

## Acceptance Criteria

### Tests That Must Pass

1. `golden_dead_agent_pruned_from_facility_queue` — dead agent removed, next agent promoted
2. Existing suite: `cargo test -p worldwake-ai golden_`
3. Full workspace: `cargo test --workspace`

### Invariants

1. All behavior is emergent — death is from real deprivation, not manual kill
2. Queue order is deterministic
3. Conservation holds

## Post-Implementation

After implementing this suite, update `reports/golden-e2e-coverage-analysis.md`:
- Add the new scenario to Part 1 (Proven Emergent Scenarios)
- Remove P-NEW-9 from the Part 3 backlog
- Update Part 4 summary statistics

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_combat.rs::golden_dead_agent_pruned_from_facility_queue` — proves death + queue interaction

### Commands

1. `cargo test -p worldwake-ai golden_dead_agent_pruned`
2. `cargo test --workspace && cargo clippy --workspace`
