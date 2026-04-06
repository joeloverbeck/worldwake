# S55CAUBLOINV-003: Condition-based evaluation replacing blocker_resolved

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` blocker clearing evaluation logic
**Deps**: S55CAUBLOINV-002

## Problem

After ticket 002, blockers carry explicit clearing conditions and baselines, but evaluation still uses the old code-driven `blocker_resolved` match. This ticket replaces `blocker_resolved` with `is_blocker_cleared` that reads stored conditions and compares against current belief state. The old function is removed per P28 (No Backward Compatibility). This is the behavioral change: blockers now clear from stored condition data, while preserving the few fact-specific threshold semantics that golden coverage already proved lawful.

## Assumption Reassessment (2026-04-06)

1. `blocker_resolved` at `crates/worldwake-ai/src/failure_handling.rs:595` — 83-line function matching on all 17 `BlockingFact` variants. Uses `RuntimeBeliefView` for belief queries.
2. `clear_resolved_blockers` at `failure_handling.rs:81` already calls `blocked_memory.expire(current_tick)` then `blocked_memory.sweep_cleared(|intent| blocker_resolved(view, agent, intent))`. Called from `agent_tick/observation.rs:96`. Ticket wording that still mentions direct `.intents.retain()` is stale.
3. `RuntimeBeliefView` at `crates/worldwake-sim/src/belief_view.rs:353` provides the live read surfaces needed for condition-based evaluation: `commodity_quantity`, `locally_observed_commodity_quantity`, `unique_item_count`, `effective_place`, `adjacent_places_with_travel_ticks`, `entity_kind`, `is_alive`, `current_attackers_of`, `visible_hostiles_for`, `listed_sale_lots_at`, `seller_for_sale_lot`, `facility_queue_position`, `facility_grant`, `reservation_ranges`, `resource_source`, and `has_production_job`.
4. After ticket 002, production blockers constructed through `handle_plan_failure` should carry real `clearing_condition` and `baseline_snapshot` values for the mapped `BlockingFact` variants it owns. Other production constructors from ticket 001 may still lawfully remain `TtlOnly`/`None` when their blocker families are defined as TTL-only or remain out of this ticket's scope. Test-constructed blockers from ticket 001 still use `TtlOnly`/`None` defaults unless updated by the specific tests this ticket owns.
5. The `clear_resolved_blockers` test at `failure_handling.rs:1743` tests the existing behavior — it must be updated to use the new evaluation.
6. The existing `blocker_resolved` has nuanced per-variant logic (e.g., `TargetGone` for `RaidTarget`/`EngageHostile` returns `false` to suppress repeated pursuit). The new `is_blocker_cleared` must preserve these semantics through the stored conditions; ticket 002 already maps pursuit-shaped `TargetGone` blockers to `TtlOnly`/`None`, so ticket 003 must preserve that TTL-only behavior rather than clear them through `EntityReappeared`.
7. Cross-system boundary: the evaluation reads `RuntimeBeliefView` (beliefs, not authoritative state) — consistent with P14.
8. Ticket says / draft pseudocode suggests a blanket `(_, None) => false` fallback. Live blocker construction from ticket 002 uses `CommodityAvailabilityChanged { commodity, place }` with `baseline_snapshot: None` for `NoKnownSeller`, where the pre-003 behavior still clears when a seller/listing appears. Correction applied: ticket 003 must preserve that behavior with a variant-specific branch for baseline-less `CommodityAvailabilityChanged`, not collapse all `None` baselines to TTL-only.
9. `sweep_cleared` (added in ticket 001) is already wired into `clear_resolved_blockers`; this ticket only needs to swap the predicate from `blocker_resolved` to `is_blocker_cleared`.

## Architecture Check

1. Data-driven evaluation is cleaner than code-driven because: the clearing condition is declared at construction time (single source of truth), baselines detect *any* belief change rather than only absolute thresholds, and stored conditions are inspectable for debugging (P29). The `blocker_resolved` function duplicates the mapping knowledge that `derive_clearing_condition` already encodes.
2. No backward-compatibility shims. `blocker_resolved` is removed entirely. `clear_resolved_blockers` keeps its existing `sweep_cleared` call but swaps the predicate to `is_blocker_cleared`; no compatibility wrapper remains.

## Verification Layers

1. Each `BlockerClearingCondition` variant evaluates correctly against changed beliefs → focused unit test per variant
2. `TtlOnly` condition never clears (only TTL expiry) → focused unit test
3. Variant-specific baseline-less conditions still preserve prior lawful behavior (for example `NoKnownSeller` clears when a seller/listing appears), while unsupported missing baselines fall back safely → focused unit tests
4. Baseline comparison detects change from snapshot → focused unit test
5. `clear_resolved_blockers` removes condition-cleared + TTL-expired entries, retains active → focused unit test (evolution of existing test)
6. Golden tests pass with new evaluation → `cargo test -p worldwake-ai` (all golden_*.rs tests)
7. Single-layer ticket — the change is entirely within AI evaluation logic

## What to Change

### 1. New `is_blocker_cleared` function in `failure_handling.rs`

Replace `blocker_resolved` with a matcher over `(&blocker.clearing_condition, &blocker.baseline_snapshot)` that keeps the stored-condition boundary from ticket 002, but preserves already-proved behavior where a pure baseline comparison would regress live scenarios:

- `CommodityAvailabilityChanged + Some(CommodityQuantity)`:
  - `SellerOutOfStock` clears when the remembered seller is believed to exist and now has `> 0` of the commodity.
  - `SourceDepleted` clears when the remembered resource source is believed to have `available_quantity > 0`.
  - other facts clear when the locally observed quantity at the remembered place differs from the baseline.
- `CommodityAvailabilityChanged + None` remains the `NoKnownSeller` path: clear when a non-self seller listing now exists at the remembered place.
- `InventoryChanged + Some(InventoryQuantity)`:
  - `TooExpensive` and `MissingInput(_)` preserve the prior “agent now has `> 0` of the commodity” behavior.
  - other facts clear when the held quantity differs from the baseline.
- `UniqueItemAcquired + Some(UniqueItemCount)`:
  - `MissingTool(_)` preserves the prior “agent now has a tool” behavior.
  - other facts clear when the count differs from the baseline.
- `PathDiscovered + Some(PathKnown(false))` clears when the destination is now adjacent from the agent’s believed current place.
- `EntityReappeared + Some(EntityBelieved(false))` clears when the target is believed again; for `TreatWounds` and `ReduceDanger`, it must also still be alive.
- `DangerReduced` clears when both `current_attackers_of(agent)` and `visible_hostiles_for(agent)` are empty.
- `ContentionChanged` stays fact-specific:
  - `WorkstationBusy` clears when the facility no longer has a production job.
  - `ReservationConflict` clears when the facility has no reservation ranges.
  - `ExclusiveFacilityUnavailable` clears when queue position improves from the stored baseline or the facility grant is now held by the agent.
- All unsupported condition/baseline combinations fall back to `false`, which keeps TTL-only behavior safe without a compatibility shim.

### 2. Update `clear_resolved_blockers`

Keep the existing `sweep_cleared` call, but swap the predicate:

```rust
pub fn clear_resolved_blockers(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocked_memory: &mut BlockedIntentMemory,
    current_tick: Tick,
) {
    blocked_memory.expire(current_tick);
    blocked_memory.sweep_cleared(|intent| is_blocker_cleared(view, agent, intent));
}
```

### 3. Remove `blocker_resolved` function

Delete the entire `blocker_resolved` function (currently ~83 lines at line 595). No other code references it — it is private to `failure_handling.rs`.

### 4. Update tests

- Update `clear_resolved_blockers_removes_restored_and_expired_entries` test to construct blockers with appropriate clearing conditions and baselines (not TtlOnly defaults).
- Add new focused tests for `is_blocker_cleared` covering each `BlockerClearingCondition` variant.
- Add a focused test proving `NoKnownSeller` still clears through the baseline-less `CommodityAvailabilityChanged` branch when a seller/listing appears.
- Verify that pursuit-shaped `TargetGone` blockers (previously special-cased in `blocker_resolved` to return `false`) remain TTL-only under the new evaluator, while non-pursuit `TargetGone` blockers clear through `EntityReappeared` as stored by ticket 002.

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify — new `is_blocker_cleared`, updated `clear_resolved_blockers`, removed `blocker_resolved`, updated tests)

## Out of Scope

- Adding new `BlockingFact` variants
- Changing TTL durations or `CognitiveProfile` fields
- Active blocker investigation (agent planning to verify conditions) — deferred per spec Non-Goals
- Modifying `blocks_goal_generation` or `is_blocked_for_search` signatures

## Acceptance Criteria

### Tests That Must Pass

1. New: `is_blocker_cleared_commodity_availability_changed` — clears through seller/source restoration or observed quantity change, depending on the stored fact
2. New: `is_blocker_cleared_inventory_changed` — clears through restored inventory using the fact-appropriate rule
3. New: `is_blocker_cleared_unique_item_acquired` — clears through restored unique-item access using the fact-appropriate rule
4. New: `is_blocker_cleared_path_discovered` — clears when destination becomes adjacent
5. New: `is_blocker_cleared_entity_reappeared` — clears when entity exists in beliefs
6. New: `is_blocker_cleared_danger_reduced` — clears when no attackers and no visible hostiles
7. New: `is_blocker_cleared_contention_changed` — clears when the relevant contention state lawfully improves for the stored fact
8. New: `is_blocker_cleared_ttl_only_never_clears` — TtlOnly always returns false
9. New: `is_blocker_cleared_no_known_seller_listing_appears` — baseline-less `CommodityAvailabilityChanged` still clears when a seller/listing appears
10. New: `is_blocker_cleared_missing_baseline_falls_back` — unsupported `None` baseline still returns false
11. Updated: `clear_resolved_blockers_removes_restored_and_expired_entries` — uses real clearing conditions
12. New: `is_blocker_cleared_pursuit_target_gone_ttl_only` — pursuit-shaped `TargetGone` remains uncleared until TTL expiry
13. Existing golden suite: `cargo test -p worldwake-ai`

### Invariants

1. `blocker_resolved` is fully removed — no dead code (P28)
2. `clear_resolved_blockers` uses `sweep_cleared` — no direct `.intents` access from AI code
3. Clearing is belief-mediated — `is_blocker_cleared` reads `RuntimeBeliefView`, never authoritative world state (P14)
4. Unsupported missing baseline defaults to safe non-clearing behavior — no panic, no false clearing
5. All existing golden tests pass — behavioral equivalence for already-tested scenarios, including TTL-only pursuit suppression

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/failure_handling.rs` (tests module) — 11 new `is_blocker_cleared_*` tests, 1 updated `clear_resolved_blockers_*` test
2. No new test files — all tests in existing module

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-ai -- golden` (specifically verify golden E2E tests)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo build --workspace`

## Outcome

Completed on 2026-04-06.

Replaced `blocker_resolved` with `is_blocker_cleared` in [crates/worldwake-ai/src/failure_handling.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs), and kept `clear_resolved_blockers` on the existing `BlockedIntentMemory::sweep_cleared` path. The new evaluator now reads stored `clearing_condition` and `baseline_snapshot` data from ticket 002, while preserving fact-specific semantics that were already relied on by live trade and contention behavior.

The focused test surface was expanded with 11 `is_blocker_cleared_*` tests plus an updated `clear_resolved_blockers_removes_restored_and_expired_entries` test. During implementation, two golden regressions forced a factual correction to the original ticket draft: `TooExpensive` / `MissingInput(_)` and several contention blocker families could not be reduced to a pure generic “baseline changed” rule without changing lawful behavior. The final evaluator therefore stays condition-driven at the stored-data boundary, but interprets some conditions through the blocking fact to preserve the already-proved trade, production, and pursuit semantics.

## Verification Result

Passed on 2026-04-06:

1. `cargo test -p worldwake-ai is_blocker_cleared -- --nocapture`
2. `cargo test -p worldwake-ai clear_resolved_blockers_removes_restored_and_expired_entries -- --nocapture`
3. `cargo test -p worldwake-ai contested_harvest_start_failure_recovers_via_remote_fallback -- --nocapture`
4. `cargo test -p worldwake-ai golden_trade_rejection_reroutes_to_reliable_seller -- --nocapture`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo build --workspace`
