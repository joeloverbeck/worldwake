# S55CAUBLOINV-003: Condition-based evaluation replacing blocker_resolved

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` blocker clearing evaluation logic
**Deps**: S55CAUBLOINV-002

## Problem

After ticket 002, blockers carry explicit clearing conditions and baselines, but evaluation still uses the old code-driven `blocker_resolved` match. This ticket replaces `blocker_resolved` with `is_blocker_cleared` that reads stored conditions and compares against current belief state. The old function is removed per P28 (No Backward Compatibility). This is the behavioral change — blockers now clear based on stored conditions + baseline comparison rather than inline match + absolute thresholds.

## Assumption Reassessment (2026-04-06)

1. `blocker_resolved` at `crates/worldwake-ai/src/failure_handling.rs:595` — 83-line function matching on all 17 `BlockingFact` variants. Uses `RuntimeBeliefView` for belief queries.
2. `clear_resolved_blockers` at `failure_handling.rs:81` — calls `blocked_memory.expire(current_tick)` then `blocked_memory.intents.retain(|_, intent| !blocker_resolved(view, agent, intent))`. Called from `agent_tick/observation.rs:96`.
3. `RuntimeBeliefView` at `crates/worldwake-sim/src/belief_view.rs:353` — provides `commodity_quantity`, `locally_observed_commodity_quantity`, `unique_item_count`, `effective_place`, `adjacent_places_with_travel_ticks`, `entity_kind`, `is_alive`, `current_attackers_of`, `visible_hostiles_for`, `reservation_ranges`, `resource_source`, `has_production_job`.
4. After ticket 002, production blockers constructed through `handle_plan_failure` should carry real `clearing_condition` and `baseline_snapshot` values for the mapped `BlockingFact` variants it owns. Other production constructors from ticket 001 may still lawfully remain `TtlOnly`/`None` when their blocker families are defined as TTL-only or remain out of this ticket's scope. Test-constructed blockers from ticket 001 still use `TtlOnly`/`None` defaults unless updated by the specific tests this ticket owns.
5. The `clear_resolved_blockers` test at `failure_handling.rs:1743` tests the existing behavior — it must be updated to use the new evaluation.
6. The existing `blocker_resolved` has nuanced per-variant logic (e.g., `TargetGone` for `RaidTarget`/`EngageHostile` returns `false` to suppress repeated pursuit). The new `is_blocker_cleared` must preserve these semantics through the stored conditions (ticket 002 maps these to `TtlOnly` baselines where appropriate).
7. Cross-system boundary: the evaluation reads `RuntimeBeliefView` (beliefs, not authoritative state) — consistent with P14.
8. `sweep_cleared` (added in ticket 001) can replace the direct `.intents.retain()` call in `clear_resolved_blockers`.

## Architecture Check

1. Data-driven evaluation is cleaner than code-driven because: the clearing condition is declared at construction time (single source of truth), baselines detect *any* belief change rather than only absolute thresholds, and stored conditions are inspectable for debugging (P29). The `blocker_resolved` function duplicates the mapping knowledge that `derive_clearing_condition` already encodes.
2. No backward-compatibility shims. `blocker_resolved` is removed entirely. `clear_resolved_blockers` is updated to use `sweep_cleared` + `is_blocker_cleared`. The old `.intents.retain(|_, intent| !blocker_resolved(...))` pattern is replaced, not wrapped.

## Verification Layers

1. Each `BlockerClearingCondition` variant evaluates correctly against changed beliefs → focused unit test per variant
2. `TtlOnly` condition never clears (only TTL expiry) → focused unit test
3. Missing baseline (`None`) falls back to TTL-only → focused unit test
4. Baseline comparison detects change from snapshot → focused unit test
5. `clear_resolved_blockers` removes condition-cleared + TTL-expired entries, retains active → focused unit test (evolution of existing test)
6. Golden tests pass with new evaluation → `cargo test -p worldwake-ai` (all golden_*.rs tests)
7. Single-layer ticket — the change is entirely within AI evaluation logic

## What to Change

### 1. New `is_blocker_cleared` function in `failure_handling.rs`

Replace `blocker_resolved` with:

```rust
fn is_blocker_cleared(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocker: &BlockedIntent,
) -> bool {
    match (&blocker.clearing_condition, &blocker.baseline_snapshot) {
        (BlockerClearingCondition::CommodityAvailabilityChanged { commodity, place },
         Some(ClearingBaseline::CommodityQuantity { quantity: baseline })) => {
            view.locally_observed_commodity_quantity(agent, *place, *commodity) != *baseline
        }
        (BlockerClearingCondition::InventoryChanged { commodity },
         Some(ClearingBaseline::InventoryQuantity { quantity: baseline })) => {
            view.commodity_quantity(agent, *commodity) != *baseline
        }
        (BlockerClearingCondition::UniqueItemAcquired { kind },
         Some(ClearingBaseline::UniqueItemCount(baseline))) => {
            view.unique_item_count(agent, *kind) != *baseline
        }
        (BlockerClearingCondition::PathDiscovered { destination },
         Some(ClearingBaseline::PathKnown(false))) => {
            view.effective_place(agent).is_some_and(|current_place| {
                view.adjacent_places_with_travel_ticks(current_place)
                    .into_iter()
                    .any(|(adj, _)| adj == *destination)
            })
        }
        (BlockerClearingCondition::EntityReappeared { entity },
         Some(ClearingBaseline::EntityBelieved(false))) => {
            view.entity_kind(*entity).is_some()
        }
        (BlockerClearingCondition::DangerReduced { .. }, _) => {
            view.current_attackers_of(agent).is_empty()
                && view.visible_hostiles_for(agent).is_empty()
        }
        (BlockerClearingCondition::ContentionChanged { facility }, _) => {
            // Workstation freed, reservation expired, or facility available
            !view.has_production_job(*facility)
                || view.reservation_ranges(*facility).is_empty()
        }
        (BlockerClearingCondition::TtlOnly, _) => false,
        // Missing or mismatched baseline — TTL fallback
        (_, None) => false,
        _ => false,
    }
}
```

### 2. Update `clear_resolved_blockers`

Replace the direct `.intents.retain()` call with `sweep_cleared`:

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
- Verify that the `TargetGone` for pursuit goals (previously special-cased in `blocker_resolved` to return `false`) is handled correctly — ticket 002 maps these to `EntityReappeared` with baseline, so the standard evaluation applies. If the pursuit-specific suppression was intentional beyond what TtlOnly provides, add a note.

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify — new `is_blocker_cleared`, updated `clear_resolved_blockers`, removed `blocker_resolved`, updated tests)

## Out of Scope

- Adding new `BlockingFact` variants
- Changing TTL durations or `CognitiveProfile` fields
- Active blocker investigation (agent planning to verify conditions) — deferred per spec Non-Goals
- Modifying `blocks_goal_generation` or `is_blocked_for_search` signatures

## Acceptance Criteria

### Tests That Must Pass

1. New: `is_blocker_cleared_commodity_availability_changed` — clears when observed quantity differs from baseline
2. New: `is_blocker_cleared_inventory_changed` — clears when agent's commodity quantity differs from baseline
3. New: `is_blocker_cleared_unique_item_acquired` — clears when agent's unique item count differs from baseline
4. New: `is_blocker_cleared_path_discovered` — clears when destination becomes adjacent
5. New: `is_blocker_cleared_entity_reappeared` — clears when entity exists in beliefs
6. New: `is_blocker_cleared_danger_reduced` — clears when no attackers and no visible hostiles
7. New: `is_blocker_cleared_contention_changed` — clears when facility freed
8. New: `is_blocker_cleared_ttl_only_never_clears` — TtlOnly always returns false
9. New: `is_blocker_cleared_missing_baseline_falls_back` — None baseline always returns false
10. Updated: `clear_resolved_blockers_removes_restored_and_expired_entries` — uses real clearing conditions
11. Existing golden suite: `cargo test -p worldwake-ai`

### Invariants

1. `blocker_resolved` is fully removed — no dead code (P28)
2. `clear_resolved_blockers` uses `sweep_cleared` — no direct `.intents` access from AI code
3. Clearing is belief-mediated — `is_blocker_cleared` reads `RuntimeBeliefView`, never authoritative world state (P14)
4. Missing baseline defaults to TTL-only — no panic, no false clearing
5. All existing golden tests pass — behavioral equivalence for already-tested scenarios

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/failure_handling.rs` (tests module) — 9 new `is_blocker_cleared_*` tests, 1 updated `clear_resolved_blockers_*` test
2. No new test files — all tests in existing module

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-ai -- golden` (specifically verify golden E2E tests)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo build --workspace`
