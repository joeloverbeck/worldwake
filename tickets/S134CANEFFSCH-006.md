# S134CANEFFSCH-006: Trade, queue, and escort schemas

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — replaces empty-placeholder schemas with real `EffectSchema` literals in trade, queue, and escort actions and switches their commit handler bodies to `apply_effects(..., Authoritative)`
**Deps**: archive/tickets/S134CANEFFSCH-001.md, S134CANEFFSCH-002

## Problem

S134 deliverable D5 requires migrating the trade-and-coordination family — `trade` and `staff_market` (in `trade_actions.rs`), `queue_for_facility_use` (in `facility_queue_actions.rs`), and `escort_to_safety` (in `escort_actions.rs`) — to declarative `EffectSchema` evaluation. Trade introduces multi-party effect chains (counterparty agreement and bilateral commodity transfer), facility-queue actions exercise the contention-grant substrate (`ContentionGrantHeld` precondition + `ConsumeContentionGrant` step), and escort introduces multi-step movement-and-delivery semantics. The planner continues to use the old `apply_hypothetical_transition` path; goldens for these actions must produce bitwise-identical event logs.

## Assumption Reassessment (2026-05-04)

1. Trade registrations live at `crates/worldwake-systems/src/trade_actions.rs` via `register_trade_action` and `register_staff_market_action`. Queue registration at `crates/worldwake-systems/src/facility_queue_actions.rs` via `register_queue_for_facility_use_action`. Escort registration at `crates/worldwake-systems/src/escort_actions.rs` via `register_escort_to_safety_action`.
2. After ticket 001, each `ActionDef` literal in these three files has `effect_schema: EffectSchema::empty()`.
3. Trade is bilateral: actor receives commodity X, counterparty receives commodity Y. The schema's `EffectStep::Transfer` chain encodes both directions in one schema. Counterparty willingness is currently a handler-internal check; in the schema language it becomes an `EffectPrecondition` (likely `BeliefHeld { agent: counterparty, claim: WillingToTrade(...) }` or a new variant — confirm during reassessment).
4. Queue actions exercise the contention-grant substrate: `ContentionGrantHeld` precondition checks queue-grant ownership; the step list includes queue-membership mutation (join/leave) and `ConsumeContentionGrant` if the action consumes a grant.
5. Escort to safety is a multi-step ferry: movement update for both actor and protectee, with co-location check throughout. Likely uses standard `EffectStep` variants once the schema language is fleshed out — confirm during reassessment whether escort needs a domain-specific step variant.
6. Shared abstraction boundary under audit: trade's bilateral transfer is the most complex schema in this ticket — both directions must commit atomically (`PartialOnFailure`'s rollback semantics from ticket 002 apply if either side's `Transfer` precondition fails).
7. Existing focused/unit coverage:
   - `trade_actions.rs`, `facility_queue_actions.rs`, `escort_actions.rs` `#[cfg(test)]` blocks
   - Goldens — `golden_merchant_*.rs`, `golden_trade_*.rs`, `golden_facility_queue_*.rs`, `golden_escort_*.rs`. Enumerate during reassessment.
   - Conformance tests: `conformance_trade_exact_acquisition` (line 993), `conformance_queue_for_facility` (line 1976) at `planner_conformance.rs`.
8. Bitwise-identical event-log invariant: every trade emission (offer, agreement, transfer, settlement), queue join/leave/grant event, and escort movement event must have identical payload values pre- and post-ticket.

## Architecture Check

1. Bilateral trade as a single declarative schema makes the atomic-commit semantics explicit (both transfers happen or neither does), replacing handler-internal coordination with the schema's `PartialOnFailure` rollback. This makes the contract auditable from the schema literal alone (FND-29).
2. The contention-grant substrate (`facility_queue_actions.rs` queues) is exercised through `EffectPrecondition::ContentionGrantHeld` and `EffectStep::ConsumeContentionGrant`, integrating queue actions into the same evaluation pipeline as combat and harvest (which also use grants for corpse-use and resource-source contention) — uniform contention substrate.
3. Escort-to-safety's movement semantics use the same place-graph mutation primitives existing handlers do; the schema's step language must include movement (likely an `EffectStep::Move { entity, destination }` variant — add in this ticket if not yet present).

## Verification Layers

1. Bitwise-identical event-log invariant → event-log delta on trade-touching, queue-touching, and escort-touching goldens.
2. Bilateral-trade atomicity invariant → action trace: when one side's preconditions fail mid-execution, neither transfer is committed; the existing handler's atomic-commit behavior is preserved by `PartialOnFailure` rollback.
3. Contention-grant invariant → focused runtime test: `conformance_queue_for_facility` continues to pass; queue grants are consumed in the same order pre- and post-ticket.
4. Canonical state hash invariant → soak: identical hashes on the three soak scenarios.

## What to Change

### 1. Construct `EffectSchema` literal for trade

Trade is bilateral. Sketch:

```rust
EffectSchema {
    preconditions: vec![
        EffectPrecondition::CoLocated { actor, target: counterparty },
        // counterparty willingness — likely BeliefHeld or a new TradeAgreed variant
        EffectPrecondition::QuantityAvailable { source: actor, commodity: actor_offering, min: actor_quantity },
        EffectPrecondition::QuantityAvailable { source: counterparty, commodity: counterparty_offering, min: counterparty_quantity },
    ],
    steps: vec![
        EffectStep::Transfer { source: actor, dest: counterparty, commodity: actor_offering, quantity: actor_quantity },
        EffectStep::Transfer { source: counterparty, dest: actor, commodity: counterparty_offering, quantity: counterparty_quantity },
        EffectStep::EmitEvent { tag: EventTag::Trade },
    ],
}
```

Both `Transfer` steps must commit atomically — leverage `PartialOnFailure`'s rollback semantics or add an explicit `Atomic { steps }` step variant if `PartialOnFailure`'s primary/fallback shape is not the right fit (decide during reassessment).

### 2. Construct `EffectSchema` literal for staff_market

`staff_market` assigns an agent to a market role. Schema: precondition on role-availability and authority; step on role-assignment component mutation; event emission.

### 3. Construct `EffectSchema` literal for queue_for_facility_use

Schema: `CoLocated` with facility precondition, queue-membership-mutation step, `EmitEvent { tag: EventTag::QueueJoin }`. The actual grant-consumption (when the queue advances) happens elsewhere — confirm during reassessment whether `queue_for_facility_use` itself consumes a grant or just joins.

### 4. Construct `EffectSchema` literal for escort_to_safety

Schema: preconditions on co-location with protectee at start, on knowledge of safe-place, and on no-active-threat. Steps include movement primitives for both actor and protectee, ending in arrival event emission. Likely needs an `EffectStep::Move { entity, destination }` variant if not yet present in `effect_schema.rs`.

### 5. Replace handler bodies with `apply_effects` delegation

Each `commit_*` handler in trade/queue/escort shrinks to the standard delegation. Remove imperative bodies.

## Files to Touch

- `crates/worldwake-systems/src/trade_actions.rs` (modify — 2 schemas, 2 commit handler body replacements)
- `crates/worldwake-systems/src/facility_queue_actions.rs` (modify — 1 schema, 1 commit handler body replacement)
- `crates/worldwake-systems/src/escort_actions.rs` (modify — 1 schema, 1 commit handler body replacement)
- `crates/worldwake-sim/src/effect_schema.rs` (modify if `EffectStep` needs new variants — `Move`, `Atomic`, or trade-specific willingness precondition)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` and `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (modify if new sink methods are added)

## Out of Scope

- Migrating non-trade/queue/escort actions (tickets 003, 004, 005, 007–009).
- Switching the planner search to `apply_effects(..., Hypothetical)` (ticket 010).
- Deleting `apply_hypothetical_transition`, `PlannerTransitionKind`, `apply_planner_step` (ticket 010).
- Changing trade-counterparty agreement semantics, queue contention mechanics, or escort safety semantics — preserved per spec Non-Goals.

## Acceptance Criteria

### Tests That Must Pass

1. All trade-touching, queue-touching, and escort-touching goldens produce bitwise-identical event logs (enumerate during reassessment).
2. Conformance tests `conformance_trade_exact_acquisition` and `conformance_queue_for_facility` continue to pass.
3. `cargo test -p worldwake-systems trade facility_queue escort` — existing inline tests pass with the schema-driven path.
4. `cargo test -p worldwake-ai golden_survival` — soak goldens produce identical canonical state hashes.
5. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Trade's bilateral transfer is atomic: when one side's precondition fails, neither commodity moves (verified by an adversarial focused test if not already covered).
2. `conformance_trade_exact_acquisition`'s payload-quantity expectation is preserved (S127 partial-quantity semantics extend correctly to trade).
3. Bitwise-identical canonical state hash on the three soak scenarios.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/trade_actions.rs` `#[cfg(test)]` block — add focused test for atomic-rollback when counterparty's `QuantityAvailable` precondition fails.
2. `crates/worldwake-systems/src/facility_queue_actions.rs` `#[cfg(test)]` block — modify existing tests to exercise schema-driven path.
3. `crates/worldwake-systems/src/escort_actions.rs` `#[cfg(test)]` block — modify existing tests; add precondition-failure focused test if not present.
4. Existing goldens — no source change.

### Commands

1. `cargo test -p worldwake-systems trade facility_queue escort`
2. `cargo test -p worldwake-ai conformance_trade conformance_queue_for_facility`
3. `cargo test -p worldwake-ai golden_survival`
4. `./scripts/verify.sh`
