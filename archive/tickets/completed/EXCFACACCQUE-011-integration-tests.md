# EXCFACACCQUE-011 — Integration Tests: Multi-Agent Queue Contention

**Status**: COMPLETED

**Spec sections**: Tests (all 12 test cases)
**Crates**: `worldwake-ai` (golden/integration tests), `worldwake-systems` (only if a newly exposed gap requires a local regression)

## Summary

The queue/grant architecture from `EXCFACACCQUE-001` through `EXCFACACCQUE-010` is already implemented in the current codebase. This ticket is no longer a greenfield "write the whole queue test suite" task.

The real remaining work is to add the missing end-to-end contested-facility regressions that prove the AI-driven normal path uses queue/grant state under load, and to tighten any gaps that the new tests expose.

This ticket must not duplicate localized coverage that already exists in source-file tests. It should add the smallest set of higher-level regressions needed to lock the architecture in place.

## Current Codebase Reality

The following assumptions from the original ticket were stale and are now corrected:

- `EXCFACACCQUE-001` through `EXCFACACCQUE-010` are not pending. The queue/grant stack already exists across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-ai`.
- Queue components already exist in [`crates/worldwake-core/src/facility_queue.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/facility_queue.rs).
- `queue_for_facility_use` is already implemented and validated in [`crates/worldwake-systems/src/facility_queue_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/facility_queue_actions.rs).
- `facility_queue_system` already exists with pruning, expiry, and promotion behavior in [`crates/worldwake-systems/src/facility_queue.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/facility_queue.rs).
- Harvest/craft start gating already requires and consumes matching grants in [`crates/worldwake-systems/src/production_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs).
- Planning/search/runtime queue semantics already exist in [`crates/worldwake-ai/src/search.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs) and [`crates/worldwake-ai/src/agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs).

Existing coverage already proves many behaviors this ticket originally treated as new work:

- Queue action validation and commit behavior in [`crates/worldwake-systems/src/facility_queue_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/facility_queue_actions.rs)
- Queue system pruning, expiry, stall, promotion, and idempotence in [`crates/worldwake-systems/src/facility_queue.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/facility_queue.rs)
- Harvest/craft grant gating in [`crates/worldwake-systems/src/production_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs)
- Search-layer queue routing in [`crates/worldwake-ai/src/search.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs)
- Runtime grant-arrival, invalidation, patience, and queued non-exclusive behavior in [`crates/worldwake-ai/src/agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs)

## Revised Scope

Add only the missing end-to-end regressions that require the full scheduler + systems + AI stack to run together:

- a contested exclusive orchard scenario where multiple hungry agents use the queue/grant path as the normal arbitration mechanism
- explicit evidence that first grants rotate across distinct actors instead of letting one actor immediately monopolize the source
- deterministic replay/hash coverage for the exclusive contested scenario itself, not just the older non-exclusive production race

If these tests expose a real implementation defect or an under-covered invariant, fix the implementation and add the narrow supporting regression in the owning crate.

## Deliverables

### 1. Exclusive contested-orchard end-to-end test

Run four hungry AI agents at one orchard that is explicitly configured as an exclusive facility. Verify:

- queue/grant state, not incidental request collision, is the normal arbitration path
- queued membership becomes concrete facility state during contention
- the first two consumed grants in a finite `Quantity(4)` orchard belong to two distinct actors before any actor receives a second turn

### 2. Exclusive contested-orchard determinism test

Run the same exclusive contested-orchard scenario twice with the same seed and verify identical world/event-log hashes.

### 3. Narrow follow-up regressions only if exposed by the new end-to-end test

Do not proactively rewrite the localized queue suites. Add extra supporting tests there only if the new AI-level scenario reveals a missing invariant that is best locked at the lower layer.

## Files to Touch

- Prefer extending [`crates/worldwake-ai/tests/golden_production.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs) and/or its shared harness in [`crates/worldwake-ai/tests/golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs).
- Touch `worldwake-systems` tests only if the new end-to-end scenario exposes a lower-level defect that needs a localized regression.

## Out of Scope

- Rewriting or relocating the existing queue-specific unit/source tests just to satisfy this ticket
- Re-testing already-covered queue behaviors without adding new architectural signal
- Performance benchmarks
- E14 perception-gated queue visibility (future work)

## Acceptance Criteria

### Tests that must pass

The following must pass after completion:

1. A new AI-level exclusive contested-orchard test proving queue/grant state is exercised during normal multi-agent contention.
2. A new assertion that the first two grants/turns belong to distinct actors in the finite-stock orchard scenario.
3. A determinism replay/hash test for that exclusive contested scenario.

- `cargo test --workspace` — no regressions

### Invariants that must remain true
- New tests reuse existing golden/setup infrastructure where possible
- Tests are deterministic (seeded RNG, BTreeMap ordering)
- Tests verify event log contents (causal linking, event tags)
- Tests verify conservation invariants still hold
- No new test should pass merely because incidental scheduler order happened to avoid contention
- End-to-end tests must prove exclusive contention is resolved through queue/grant state

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - corrected the ticket to reflect that the queue/grant architecture and most localized regressions were already implemented
  - added exclusive-facility golden coverage in `worldwake-ai` for contested orchard queue/grant behavior and deterministic replay
  - added an explicit exclusive-workstation test helper so end-to-end tests can opt into the queue path instead of silently using the older non-exclusive fixture
- Deviations from original plan:
  - did not create new dedicated `worldwake-systems/tests/` or `worldwake-ai/tests/facility_queue_*` files because the missing signal fit cleanly into the existing golden production harness
  - did not duplicate already-covered queue action, queue system, search, runtime, or grant-gating tests
- Verification results:
  - `cargo test -p worldwake-ai golden_exclusive_queue_contention -- --nocapture`
  - `cargo test -p worldwake-ai golden_resource_exhaustion_race -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
