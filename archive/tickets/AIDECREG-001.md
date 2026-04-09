# AIDECREG-001: Reassess and fix `golden_blocked_intent_memory_with_ttl_expiry`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` belief claim eviction policy plus `worldwake-ai` golden setup/proof tightening
**Deps**: None

## Problem

The broader `cargo test -p worldwake-ai` suite currently fails in `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_blocked_intent_memory_with_ttl_expiry` with `Agent should eventually harvest apples after resource regeneration`. This is not owned by S76, but it blocks honest same-crate full-suite verification and indicates either a real regression in the blocked-intent/resource-regeneration path or a stale golden contract.

## Assumption Reassessment (2026-04-09)

1. The failing scenario lives in `crates/worldwake-ai/tests/golden_ai_decisions.rs` around the “Goal Invalidation / blocked intent TTL” coverage and currently expects a depleted orchard source with `regeneration_ticks_per_unit: Some(nz(5))` to regenerate and produce apple lots within 200 ticks.
2. The failure reproduces in isolation with `cargo test -p worldwake-ai golden_blocked_intent_memory_with_ttl_expiry -- --nocapture`, so this is not an artifact of broad-suite contention.
3. Archived ticket `archive/tickets/E16DPOLPLAN-019.md` previously listed this test as passing, so the current failure represents either later runtime drift or a stale scenario assumption.
4. The live boundary under audit is mixed-layer: resource-source regeneration in authoritative state, candidate/planner behavior around depleted sources, and the golden’s proof surface about eventual harvest after regeneration.
5. The ticket must reassess first whether the intended invariant is still “TTL expiry enables eventual harvest after source regeneration” or whether the current architecture lawfully requires a different proof surface or setup math.
6. Live tracing and lower-layer inspection showed the authoritative source does regenerate and the agent’s believed snapshot also refreshes to the replenished `ResourceSource`, but the believed workstation can lose `last_known_place` under `AgentBeliefStore::enforce_entity_claim_capacity()`. Once that location claim is pruned, `entities_at(ORCHARD_FARM)` no longer returns the source facility, so `AcquireCommodity(SelfConsume)` sees zero local harvest candidates even though the regenerated source is lawfully known.
7. Safe correction: this is a production belief-memory bug, not a stale TTL-only golden. The earliest owned contradiction is in `crates/worldwake-core/src/belief.rs` claim eviction tiering for facility locations. The golden should stay focused on blocker recording plus eventual harvest after regeneration, with explicit local-belief seeding so the scenario isolates the regeneration/blocker path rather than co-location discovery timing.
8. Broader verification after the fix no longer fails in `golden_blocked_intent_memory_with_ttl_expiry`, but `cargo test -p worldwake-ai` now exposes a different failing golden, `crates/worldwake-ai/tests/golden_emergent.rs::golden_witnessed_theft_accusation_chain`. That blocker is outside this ticket’s owned surface and needs its own follow-up ticket.

## Architecture Check

1. A bounded reassessment-and-fix ticket is cleaner than treating the failing test as incidental verification noise.
2. The ticket should fix the earliest concrete contradiction: here that is facility-location claim eviction in the belief layer, then tighten the golden so it proves blocker recording plus post-regeneration harvest against the live information path.

## Verification Layers

1. Resource regeneration occurs authoritatively -> authoritative world state / focused lower-layer proof
2. Facility location survives belief claim pruning for resource-source facilities -> focused `worldwake-core` belief test
3. AI revisits the opportunity after blocker recording and source regeneration -> `golden_blocked_intent_memory_with_ttl_expiry`
4. Broader same-crate rerun after the fix -> `cargo test -p worldwake-ai`, with any newly exposed unrelated blocker isolated and handed off explicitly

## What to Change

### 1. Reassess the failing golden against live code

- Name the exact authoritative regeneration symbols and blocked-intent invalidation symbols under audit.
- Determine whether the current failure is stale setup, stale assertion surface, or a production regression.

### 2. Land the smallest honest fix

- Fix the belief-layer claim eviction policy so resource-source facilities retain location under claim-capacity pruning.
- Tighten the golden setup/proof so it explicitly seeds local beliefs, observes the blocked-intent phase, and still proves eventual harvest after regeneration.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)

## Out of Scope

- S76 simulation-gap scenarios
- Broad golden-suite cleanup unrelated to this failing path
- Documentation-only alignment work for S76

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core enforce_entity_claim_capacity_preserves_facility_location_for_resource_sources -- --nocapture`
2. `cargo test -p worldwake-ai golden_blocked_intent_memory_with_ttl_expiry -- --nocapture`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test -p worldwake-ai` is rerun as a broader blocker sweep; if it exposes a different failing test outside this ticket’s owned surface, that failure must be isolated and handed off explicitly rather than silently folded into this ticket

### Invariants

1. The final fix preserves the honest contract for resource regeneration and blocked-intent expiry rather than papering over the failure
2. Resource-source facilities must retain location under belief claim-capacity pruning so place-scoped opportunity discovery remains lawful
3. If the golden changes, its scenario prose and assertion surface match the live causal boundary

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs::enforce_entity_claim_capacity_preserves_facility_location_for_resource_sources` — focused regression guard for the facility-location eviction bug
2. `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_blocked_intent_memory_with_ttl_expiry` — repaired regeneration/blocker golden with explicit local-belief setup and blocker observation

### Commands

1. `cargo test -p worldwake-core enforce_entity_claim_capacity_preserves_facility_location_for_resource_sources -- --nocapture`
2. `cargo test -p worldwake-ai golden_blocked_intent_memory_with_ttl_expiry -- --nocapture`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Fixed the live regression in `crates/worldwake-core/src/belief.rs` by treating facility `Location` claims as infrastructure-tier during entity-claim pruning, which preserves `last_known_place` for resource-source facilities.
- Added a focused core regression test proving resource-source facilities retain location when claim capacity is tight.
- Tightened `golden_blocked_intent_memory_with_ttl_expiry` in `crates/worldwake-ai/tests/golden_ai_decisions.rs` so it seeds local beliefs explicitly, observes the blocked-intent phase, and still proves eventual harvest after the orchard regenerates.
- Reassessed the broader `worldwake-ai` sweep honestly: the original blocker is fixed, but a different unrelated failing golden now surfaces.

## Verification Result

- Passed `cargo test -p worldwake-core enforce_entity_claim_capacity_preserves_facility_location_for_resource_sources -- --nocapture`
- Passed `cargo test -p worldwake-ai golden_blocked_intent_memory_with_ttl_expiry -- --nocapture`
- Failed broader `cargo test -p worldwake-ai -q` on an unrelated newly exposed blocker: `crates/worldwake-ai/tests/golden_emergent.rs::golden_witnessed_theft_accusation_chain`
- Confirmed the unrelated broader blocker in isolation with `cargo test -p worldwake-ai golden_witnessed_theft_accusation_chain -- --nocapture`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
