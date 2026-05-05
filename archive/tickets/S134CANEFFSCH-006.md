# S134CANEFFSCH-006: Trade, queue, and escort schemas

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — replaces empty-placeholder schemas with real `EffectSchema` literals in trade, staff-market, queue, and escort actions and switches their commit handlers to `apply_effects_with_context(..., Authoritative)` through category-owned authoritative sinks
**Deps**: archive/tickets/S134CANEFFSCH-001.md, archive/tickets/S134CANEFFSCH-002.md

## Problem

S134 deliverable D5 requires migrating the trade-and-coordination family — `trade` and `staff_market` (in `trade_actions.rs`), `queue_for_facility_use` (in `facility_queue_actions.rs`), and `escort_to_safety` (in `escort_actions.rs`) — to declarative `EffectSchema` evaluation. Trade introduces multi-party effect chains (counterparty agreement and bilateral commodity transfer), facility-queue actions exercise the contention queue substrate, and escort introduces multi-step movement-and-delivery semantics. The planner continues to use the old `apply_hypothetical_transition` path until S134CANEFFSCH-010; goldens for these actions must preserve the same behavior.

## Assumption Reassessment (2026-05-05)

1. Trade registrations live at `crates/worldwake-systems/src/trade_actions.rs` via `register_trade_action` and `register_staff_market_action`. Queue registration at `crates/worldwake-systems/src/facility_queue_actions.rs` via `register_queue_for_facility_use_action`. Escort registration at `crates/worldwake-systems/src/escort_actions.rs` via `register_escort_to_safety_action`.
2. At intake, the four action definitions still used `EffectSchema::empty()`.
3. The drafted generic `Transfer` chain for trade was not representable from registry-time schema literals. The requested commodity is derived from the sale lot, the committed offered quantity comes from `ActionState::Trade.agreed_price`, and the commit also records demand observations and source reliability. The truthful S134 boundary follows the category-owned pattern from tickets 003-005: `EffectStep::CompleteTrade` is interpreted by `TradeEffectSink`.
4. `staff_market` is presence-only but its commit records unproductive demand and blocked sell intent from the live `MerchandiseProfile`, home facility, and displayed/possessed stock state. That is a category aftermath, so it lands as `EffectStep::RecordStaffMarketDemand`.
5. `queue_for_facility_use` can use the existing generic `EffectStep::EnqueueContention` with `EffectActionRef::PayloadQueueIntendedAction`; it does not consume a grant at commit. The local `QueueEffectSink` applies the existing `enqueue_for_contention` helper.
6. `escort_to_safety` cannot be represented by a generic `Move` step on the live branch. Its commit settles the final route leg, emits movement evidence, records route experience using event-log-derived hostile travel state, ensures care contention state, and queues the escorting actor for the intended heal action. It lands as `EffectStep::CompleteEscortToSafety` interpreted by `EscortEffectSink`.
7. The planner hypothetical path remains unchanged until S134CANEFFSCH-010. The new category steps intentionally default to `Discrepancy::ImproperPlanningState` in generic/hypothetical sinks until ticket 010 owns mode parity.
8. No persisted state shape changed. `ActionDef.effect_schema` is registry-time data, so `SAVE_FORMAT_VERSION` is unchanged.
9. Existing focused/unit coverage:
   - `trade_actions.rs`, `facility_queue_actions.rs`, `escort_actions.rs` `#[cfg(test)]` blocks
   - Goldens — live trade/queue/escort ownership is `golden_merchant_selling.rs`, `golden_survival_trade.rs`, and `golden_survival_escort.rs`.
   - Conformance tests: `conformance_trade_exact_acquisition` (line 993), `conformance_queue_for_facility` (line 1976) at `planner_conformance.rs`.
10. Bitwise-identical behavior invariant: trade transfers/listing cleanup/demand recording, queue entry order, and escort movement/care-handoff semantics are preserved by routing the existing mutation helpers through category-owned sinks.

## Architecture Check

1. Trade and escort use typed category steps because their commit semantics depend on runtime payload/state and domain aftermath that the generic schema language cannot express without lossy placeholders.
2. Queue admission uses the shared contention substrate through `EffectStep::EnqueueContention`; no grant is consumed by this action.
3. The category-owned sinks preserve FND-12: schema delegation changes the computation route, not the causal meaning or emitted world state.

## Verification Layers

1. Trade and staff-market behavior → focused `worldwake-systems trade` tests plus `golden_merchant_selling` and ignored `golden_survival_trade` cases.
2. Queue behavior → focused `worldwake-systems facility_queue` tests, `conformance_queue_for_facility`, and the queue branch inside ignored `golden_survival_trade`.
3. Escort behavior → focused `worldwake-systems escort` tests plus ignored `golden_survival_escort` cases.
4. Schema-registration invariant → focused registration assertions for trade, staff-market, queue, and escort effect-schema steps.

## What to Change

### 1. Construct `EffectSchema` literal for trade

Landed as `EffectSchema { steps: vec![EffectStep::CompleteTrade], .. }` with a `TradeEffectSink` that preserves negotiation-state validation, dynamic sale-lot commodity resolution, bilateral transfer, demand observation, and successful source-acquisition recording.

### 2. Construct `EffectSchema` literal for staff_market

Landed as `EffectStep::RecordStaffMarketDemand`. The live action is presence-only; displayed listing state is staged/unstaged elsewhere and reconciled by the trade system.

### 3. Construct `EffectSchema` literal for queue_for_facility_use

Landed as the generic `EffectStep::EnqueueContention { actor: Actor, entity: Target(0), intended_action: PayloadQueueIntendedAction }`. The queue action joins the contention queue; grant promotion/consumption happens elsewhere.

### 4. Construct `EffectSchema` literal for escort_to_safety

Landed as `EffectStep::CompleteEscortToSafety` with an `EscortEffectSink` that preserves final-leg settlement, movement evidence, route experience, care contention setup, and heal queue handoff.

### 5. Replace handler bodies with `apply_effects` delegation

Each `commit_*` handler now delegates to `apply_effects_with_context(..., EffectMode::Authoritative)` through the relevant local sink. The existing domain helpers remain the lawful mutation boundary inside the sink.

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (modify — 2 schemas, 2 commit handler body replacements)
- `crates/worldwake-systems/src/facility_queue_actions.rs` (modify — 1 schema, 1 commit handler body replacement)
- `crates/worldwake-systems/src/escort_actions.rs` (modify — 1 schema, 1 commit handler body replacement)
- `crates/worldwake-sim/src/effect_schema.rs` (modify — `CompleteTrade`, `RecordStaffMarketDemand`, `CompleteEscortToSafety`)
- `archive/specs/S134-canonical-effect-schema.md` (modify — active spec truth-sync at implementation time)

## Out of Scope

- Migrating non-trade/queue/escort actions (tickets 003, 004, 005, 007–009).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Deleting `apply_hypothetical_transition`, `PlannerTransitionKind`, `apply_planner_step` (ticket 010).
- Changing trade-counterparty agreement semantics, queue contention mechanics, or escort safety semantics — preserved per spec Non-Goals.

## Acceptance Criteria

### Tests That Must Pass

1. Trade-touching, queue-touching, and escort-touching goldens pass at the live owning seams.
2. Conformance tests `conformance_trade_exact_acquisition` and `conformance_queue_for_facility` continue to pass.
3. Existing inline tests pass with the schema-driven path via separate valid Cargo filters: `trade`, `facility_queue`, and `escort`.
4. `cargo test -p worldwake-ai golden_survival` passes as smoke; exact ignored trade/escort scenario cases also pass.
5. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Trade's bilateral transfer remains committed by the existing `execute_trade_transfers` helper after negotiation/state validation.
2. `conformance_trade_exact_acquisition`'s payload-quantity expectation is preserved.
3. Queue and escort commit semantics are preserved by their existing focused tests and ignored long-run goldens.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade_actions.rs` — existing commit tests exercise schema delegation; registration tests assert `CompleteTrade` and `RecordStaffMarketDemand`.
2. `crates/worldwake-systems/src/facility_queue_actions.rs` — existing commit tests exercise schema delegation; registration test asserts `EnqueueContention`.
3. `crates/worldwake-systems/src/escort_actions.rs` — existing commit tests exercise schema delegation; registration test asserts `CompleteEscortToSafety`.
4. Existing goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems trade`
2. `cargo test -p worldwake-systems facility_queue`
3. `cargo test -p worldwake-systems escort`
4. `cargo test -p worldwake-ai --test planner_conformance conformance_trade_exact_acquisition`
5. `cargo test -p worldwake-ai --test planner_conformance conformance_queue_for_facility`
6. `cargo test -p worldwake-ai golden_survival`
7. `cargo test -p worldwake-ai --test golden_survival_trade -- --ignored`
8. `cargo test -p worldwake-ai --test golden_survival_escort -- --ignored`
9. `cargo clippy --workspace --all-targets -- -D warnings`
10. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-05.

- Added `EffectStep::CompleteTrade`, `EffectStep::RecordStaffMarketDemand`, and `EffectStep::CompleteEscortToSafety` as category-owned schema steps with default sink rejection outside their owning sinks.
- Replaced empty schemas in `trade`, `staff_market`, `queue_for_facility_use`, and `escort_to_safety` with live effect schemas.
- Replaced trade, staff-market, queue, and escort commit bodies with `apply_effects_with_context(..., EffectMode::Authoritative)` delegation through local authoritative sinks.
- Preserved existing trade transfer/listing/demand/source-memory behavior, queue admission ordering, and escort movement/evidence/route/care-handoff behavior.
- Updated the active S134 spec to record why these actions use category-owned steps instead of the drafted generic transfer/move sketches.

## Deviations From Draft

1. Generic bilateral `Transfer` steps were not used for trade because the committed commodity and quantity are runtime-derived from sale-lot state and negotiation state, and the commit has demand/source-memory aftermath.
2. No generic `Move` step was added for escort. The live commit needs route/event-log/evidence/care-queue context beyond a pure location mutation.
3. `queue_for_facility_use` does not consume a contention grant. It only enqueues the actor for the payload's intended action.
4. Planner hypothetical parity remains deferred to S134CANEFFSCH-010.
5. No save-format bump was required because `ActionDef.effect_schema` is registry-time data.

## Verification Result

Passed:

1. `cargo test -p worldwake-systems --lib --no-run`
2. `cargo test -p worldwake-systems trade`
3. `cargo test -p worldwake-systems facility_queue`
4. `cargo test -p worldwake-systems escort`
5. `cargo test -p worldwake-systems`
6. `cargo test -p worldwake-ai --test planner_conformance conformance_trade_exact_acquisition`
7. `cargo test -p worldwake-ai --test planner_conformance conformance_queue_for_facility`
8. `cargo test -p worldwake-ai golden_survival`
9. `cargo test -p worldwake-ai --test golden_survival_trade`
10. `cargo test -p worldwake-ai --test golden_survival_trade -- --ignored`
11. `cargo test -p worldwake-ai --test golden_survival_escort -- --ignored`
12. `cargo test -p worldwake-ai --test golden_merchant_selling`
13. `cargo clippy --workspace --all-targets -- -D warnings`
14. `./scripts/verify.sh`

`./scripts/verify.sh` completed the live repo gate: `cargo fmt --all -- --check`, `cargo test --workspace`, active-goal removal check, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `scenario-coverage --check`.
