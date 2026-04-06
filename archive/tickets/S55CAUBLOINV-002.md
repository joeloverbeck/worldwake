# S55CAUBLOINV-002: Populate clearing conditions at blocker construction

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` failure handling blocker construction
**Deps**: S55CAUBLOINV-001

## Problem

After ticket 001, all blockers are constructed with `clearing_condition: TtlOnly` and `baseline_snapshot: None`. This ticket populates real clearing conditions and belief-state baselines at construction time, implementing the spec's mapping table. This is a data-population step — no behavior change occurs because the existing `blocker_resolved` still governs clearing until ticket 003.

## Assumption Reassessment (2026-04-06)

1. `handle_plan_failure` at `crates/worldwake-ai/src/failure_handling.rs` is the only production `BlockedIntent` construction site in scope for this ticket's real clearing-condition mapping work. Additional production constructors now also exist in `worldwake-ai/src/agent_tick/{candidates,frame,observation}.rs` and `worldwake-systems/src/trade_actions.rs`, but those currently emit TTL-only blockers such as `ExclusiveFacilityUnavailable`, `PatienceExhausted`, `AssumptionFailed`, and `NoBuyer` that remain out of scope here unless reassessment proves one needs a non-TTL condition.
2. `derive_blocking_fact` at `failure_handling.rs:93` takes `view: &dyn RuntimeBeliefView` — the view is already available at the construction site. No new parameters needed.
3. The construction site also has access to `agent: EntityId`, `goal_key: &GoalKey`, and `step: &PlannedStep` — sufficient to derive baselines for all mapping table entries.
4. `RuntimeBeliefView` provides the live read surfaces needed for this ticket's strongest available baselines: `commodity_quantity`, `locally_observed_commodity_quantity`, `unique_item_count`, `effective_place`, `adjacent_places_with_travel_ticks`, `entity_kind`, `current_attackers_of`, `visible_hostiles_for`, `facility_queue_position`, `facility_grant`, `reservation_ranges`, and `resource_source`.
5. `BlockerKey` contains `goal_key`, `place`, and `target` — these identify the blocker's scope and provide entity references needed for condition construction (e.g., `place` for `CommodityAvailabilityChanged`, `target` for `EntityReappeared`).
6. Ticket says / spec prose suggests `TargetGone` uniformly maps to `EntityReappeared { entity }` with `EntityBelieved(false)` baseline. Live code has two meanings: for `GoalKind::TreatWounds { .. }`, `GoalKind::ReduceDanger`, and non-pursuit goals, `blocker_resolved` checks entity reappearance; for `GoalKind::RaidTarget { .. }` and `GoalKind::EngageHostile { .. }`, `blocker_resolved` intentionally returns `false` so stale-place pursuit blockers stay TTL-bound. Correction applied: this ticket will map pursuit `TargetGone` blockers to `TtlOnly`/`None`, while keeping `EntityReappeared` baselines for the reappearance-shaped cases, because that preserves the live contract and avoids baking a false world meaning into the stored condition.
7. Single-layer ticket (AI failure handling only). The boundary under audit is `handle_plan_failure` -> `BlockedIntent` construction, not blocker evaluation.
8. No golden tests are expected to change — this ticket only populates new fields that are not yet read by any evaluation logic.

## Architecture Check

1. Deriving clearing conditions at construction time is cleaner than deriving them at evaluation time because: the condition is fixed when the blocker is recorded (the blocking fact determines what would clear it), and the baseline must be captured at block time to detect future changes.
2. For contention blockers, capturing the actor's current queue position when available is cleaner than using only reservation/job presence because `ClearingBaseline::ContentionPosition` already exists in core and best matches the spec's "contention state changed" contract.
3. No backward-compatibility shims. The existing `blocker_resolved` function ignores the new fields entirely — it matches on `blocking_fact` directly. Both coexist safely until ticket 003 replaces `blocker_resolved`.

## Verification Layers

1. Each `BlockingFact` variant maps to the correct `BlockerClearingCondition` → focused unit test per variant
2. Baseline snapshots capture current belief state at construction time → focused unit test verifying snapshot values match view returns
3. `TtlOnly` fallback for unmappable or intentionally TTL-bound variants (`Unknown`, `PatienceExhausted`, `AssumptionFailed`, `NoBuyer`, pursuit-shaped `TargetGone`) → focused unit test
4. Existing `blocker_resolved` behavior unchanged → existing `clear_resolved_blockers_*` tests pass without modification
5. Single-layer ticket — additional layer mapping not applicable

## What to Change

### 1. New helper function in `failure_handling.rs`

Add `derive_clearing_condition` alongside existing `derive_blocking_fact`:

```rust
fn derive_clearing_condition(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocking_fact: BlockingFact,
    blocker_key: &BlockerKey,
) -> (BlockerClearingCondition, Option<ClearingBaseline>) {
    match blocking_fact {
        BlockingFact::SellerOutOfStock => {
            let (place, seller) = match (blocker_key.place, blocker_key.target) {
                (Some(p), Some(s)) => (p, s),
                _ => return (BlockerClearingCondition::TtlOnly, None),
            };
            let commodity = match blocker_key.goal_key.commodity {
                Some(c) => c,
                None => return (BlockerClearingCondition::TtlOnly, None),
            };
            let baseline = view.commodity_quantity(seller, commodity);
            (
                BlockerClearingCondition::CommodityAvailabilityChanged { commodity, place },
                Some(ClearingBaseline::CommodityQuantity { quantity: baseline }),
            )
        }
        BlockingFact::TooExpensive => {
            let baseline = view.commodity_quantity(agent, CommodityKind::Coin);
            (
                BlockerClearingCondition::InventoryChanged { commodity: CommodityKind::Coin },
                Some(ClearingBaseline::InventoryQuantity { quantity: baseline }),
            )
        }
        BlockingFact::MissingInput(commodity) => {
            let baseline = view.commodity_quantity(agent, *commodity);
            (
                BlockerClearingCondition::InventoryChanged { commodity: *commodity },
                Some(ClearingBaseline::InventoryQuantity { quantity: baseline }),
            )
        }
        BlockingFact::MissingTool(kind) => {
            let baseline = view.unique_item_count(agent, *kind);
            (
                BlockerClearingCondition::UniqueItemAcquired { kind: *kind },
                Some(ClearingBaseline::UniqueItemCount(baseline)),
            )
        }
        BlockingFact::NoKnownSeller => {
            // No meaningful baseline — clear when any seller or commodity source appears
            match blocker_key.goal_key.commodity {
                Some(commodity) => match blocker_key.place {
                    Some(place) => (
                        BlockerClearingCondition::CommodityAvailabilityChanged { commodity, place },
                        None,
                    ),
                    None => (BlockerClearingCondition::TtlOnly, None),
                },
                None => (BlockerClearingCondition::TtlOnly, None),
            }
        }
        BlockingFact::NoKnownPath => {
            match blocker_key.place {
                Some(destination) => (
                    BlockerClearingCondition::PathDiscovered { destination },
                    Some(ClearingBaseline::PathKnown(false)),
                ),
                None => (BlockerClearingCondition::TtlOnly, None),
            }
        }
        BlockingFact::TargetGone => match blocker_key.goal_key.kind {
            GoalKind::RaidTarget { .. } | GoalKind::EngageHostile { .. } => {
                (BlockerClearingCondition::TtlOnly, None)
            }
            _ => match blocker_key.target {
                Some(entity) => (
                    BlockerClearingCondition::EntityReappeared { entity },
                    Some(ClearingBaseline::EntityBelieved(false)),
                ),
                None => (BlockerClearingCondition::TtlOnly, None),
            },
        },
        BlockingFact::DangerTooHigh | BlockingFact::CombatTooRisky => {
            match blocker_key.place.or_else(|| view.effective_place(agent)) {
                Some(place) => (
                    BlockerClearingCondition::DangerReduced { place },
                    None, // No Permille danger level available as a scalar baseline
                ),
                None => (BlockerClearingCondition::TtlOnly, None),
            }
        }
        BlockingFact::WorkstationBusy
        | BlockingFact::ExclusiveFacilityUnavailable
        | BlockingFact::ReservationConflict => {
            match blocker_key.target {
                Some(facility) => (
                    BlockerClearingCondition::ContentionChanged { facility },
                    Some(ClearingBaseline::ContentionPosition(
                        view.facility_queue_position(facility, agent),
                    )),
                ),
                None => (BlockerClearingCondition::TtlOnly, None),
            }
        }
        BlockingFact::SourceDepleted => {
            let (commodity, place) = match (blocker_key.goal_key.commodity, blocker_key.place) {
                (Some(c), Some(p)) => (c, p),
                _ => return (BlockerClearingCondition::TtlOnly, None),
            };
            let baseline = blocker_key.target
                .and_then(|source| view.resource_source(source))
                .map_or(Quantity(0), |r| r.available_quantity);
            (
                BlockerClearingCondition::CommodityAvailabilityChanged { commodity, place },
                Some(ClearingBaseline::CommodityQuantity { quantity: baseline }),
            )
        }
        BlockingFact::Unknown
        | BlockingFact::PatienceExhausted
        | BlockingFact::AssumptionFailed
        | BlockingFact::NoBuyer => (BlockerClearingCondition::TtlOnly, None),
    }
}
```

### 2. Update `BlockedIntent` construction in `handle_plan_failure`

At the construction site (currently line ~71), call `derive_clearing_condition` and populate the new fields:

```rust
let (clearing_condition, baseline_snapshot) =
    derive_clearing_condition(context.view, context.agent, blocking_fact, &blocker_key);

blocked_memory.record(BlockedIntent {
    blocker_key,
    blocking_fact,
    diagnostic_context,
    observed_tick: context.current_tick,
    expires_tick,
    clearing_condition,
    baseline_snapshot,
});
```

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify — new `derive_clearing_condition` function, updated construction site)

## Out of Scope

- Evaluating clearing conditions (ticket 003)
- Replacing `blocker_resolved` (ticket 003)
- Changing any clearing behavior — this ticket only populates data
- Golden test changes

## Acceptance Criteria

### Tests That Must Pass

1. New: `derive_clearing_condition_seller_out_of_stock` — maps to `CommodityAvailabilityChanged` with seller's quantity as baseline
2. New: `derive_clearing_condition_too_expensive` — maps to `InventoryChanged { Coin }` with agent's coin as baseline
3. New: `derive_clearing_condition_missing_input` — maps to `InventoryChanged { kind }` with agent's quantity as baseline
4. New: `derive_clearing_condition_missing_tool` — maps to `UniqueItemAcquired { kind }` with agent's count as baseline
5. New: `derive_clearing_condition_ttl_only_fallback` — `Unknown`, `PatienceExhausted`, `AssumptionFailed`, `NoBuyer`, and pursuit-shaped `TargetGone` all map to `TtlOnly`
6. New: `derive_clearing_condition_no_known_path` — maps to `PathDiscovered` with `PathKnown(false)` baseline
7. New: `derive_clearing_condition_target_gone_non_pursuit` — maps to `EntityReappeared` with `EntityBelieved(false)` baseline
8. New: `derive_clearing_condition_contention_blockers_capture_queue_baseline_when_available` — maps to `ContentionChanged` with `ContentionPosition`
9. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Every `BlockingFact` variant has a defined mapping — no unreachable arms
2. Baseline snapshots reflect belief state at construction time, not authoritative world state (P14)
3. Missing context (no place, no target, no commodity) falls back to `TtlOnly` — never panics
4. Existing `blocker_resolved` behavior unchanged — including TTL-only pursuit `TargetGone` suppression

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/failure_handling.rs` (tests module) — new `derive_clearing_condition_*` unit tests covering each `BlockingFact` variant and edge cases (missing place/target)

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo build --workspace`

## Outcome

Completed on 2026-04-06.

- Added `derive_clearing_condition` in `crates/worldwake-ai/src/failure_handling.rs` and wired `handle_plan_failure` to store real `clearing_condition` and `baseline_snapshot` data on `BlockedIntent`.
- Mapped all `BlockingFact` variants owned by `handle_plan_failure`, including queue-position baselines for contention blockers and TTL-only preservation for pursuit-shaped `TargetGone`.
- Added focused `derive_clearing_condition_*` unit coverage and extended the existing blocker-recording test to assert the stored clearing metadata.

Verification:
- `cargo test -p worldwake-ai derive_clearing_condition -- --nocapture`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --workspace`

Deviations from original ticket wording:
- The stored-condition mapping for `TargetGone` is split by goal meaning. Pursuit-shaped `TargetGone` remains `TtlOnly`/`None` because live `blocker_resolved` semantics intentionally keep those blockers TTL-bound until ticket 003 replaces evaluation.
