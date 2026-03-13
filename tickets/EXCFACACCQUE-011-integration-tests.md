# EXCFACACCQUE-011 — Integration Tests: Multi-Agent Queue Contention

**Spec sections**: Tests (all 12 test cases)
**Crates**: `worldwake-systems` (tests), `worldwake-ai` (tests)

## Summary

Write end-to-end integration tests that verify the complete queue/grant lifecycle under multi-agent contention. These tests exercise the full stack from queue-join through grant promotion, exclusive operation, and re-entry.

## Deliverables

### 1. Multi-agent harvest contention test

Four hungry agents at one orchard. Verify:
- All four agents queue locally instead of colliding on direct harvest starts
- The first two harvest grants in a finite `Quantity(4)` orchard go to two distinct queued agents before any one agent receives a second grant

### 2. Grant requirement enforcement test

Verify starting a harvest without a matching grant is impossible even if the facility is otherwise valid and stocked.

### 3. Queue pruning tests

- Actor death removes queue membership automatically
- Actor departure from the facility's place removes queue membership automatically

### 4. Grant expiry test

Expired grants advance the queue to the next eligible actor.

### 5. Temporary depletion stall test

Queue head with temporarily depleted stock stalls the queue (does not prune) until stock regenerates or agent replans away.

### 6. Permanent impossibility pruning test

Queue head with permanently impossible operation (workstation removed) is pruned with a local failure event.

### 7. Craft station queue test

Craft stations use the same queue/grant path without a separate fairness subsystem.

### 8. Determinism test

Planning snapshot and replay remain deterministic under queue advancement. Run two identical simulations with the same seed and verify identical event log hashes.

### 9. Best-effort safety net test

Best-effort autonomous input handling remains a safety net but is no longer exercised in the normal contested-harvest path.

### 10. Grant abandonment test

Agent with grant that replans to a different goal lets the grant expire and the queue advances to the next agent.

### 11. Non-exclusive interleaving test

Agent in queue can perform non-exclusive actions (eat, drink) without losing queue position.

## Files to Touch

- `crates/worldwake-systems/tests/facility_queue_integration.rs` — **new file**, integration tests for queue system + actions
- `crates/worldwake-ai/tests/facility_queue_ai_integration.rs` — **new file**, integration tests for AI queue routing

## Out of Scope

- Implementation of any queue functionality (EXCFACACCQUE-001–010 — all assumed complete)
- Modifying existing integration tests (except if they need queue setup to continue passing — that's EXCFACACCQUE-004's responsibility)
- Performance benchmarks
- E14 perception-gated queue visibility (future work)

## Acceptance Criteria

### Tests that must pass
All 11 test scenarios listed above must pass. Specifically:

1. `test_four_agents_queue_at_contested_orchard` — agents queue instead of colliding
2. `test_distinct_agents_get_first_two_grants` — no monopolization before all queued agents get a turn
3. `test_harvest_without_grant_fails` — grant is mandatory
4. `test_dead_actor_pruned_from_queue` — death removes entry
5. `test_departed_actor_pruned_from_queue` — departure removes entry
6. `test_expired_grant_advances_queue` — expiry promotes next
7. `test_depleted_stock_stalls_queue` — no pruning on temporary depletion
8. `test_permanent_impossibility_prunes_with_event` — workstation removal prunes + event
9. `test_craft_uses_same_queue_path` — craft stations are not special-cased
10. `test_queue_determinism_under_replay` — identical seeds produce identical hashes
11. `test_best_effort_not_exercised_in_normal_path` — autonomous agents use queue, not direct starts
12. `test_grant_abandonment_advances_queue` — unused grant expires, next agent gets turn
13. `test_non_exclusive_actions_preserve_queue_position` — eat/drink during wait keeps position

- `cargo test --workspace` — no regressions

### Invariants that must remain true
- All tests use the same `SimulationState` setup infrastructure as existing integration tests
- Tests are deterministic (seeded RNG, BTreeMap ordering)
- Tests verify event log contents (causal linking, event tags)
- Tests verify conservation invariants still hold
- No test relies on scheduler ordering — contention is resolved through queue state
