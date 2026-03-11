**Status**: ✅ COMPLETED

# E11TRAECO-012: Reassess Trade Exposure Boundaries

## Summary
Reassess the original "trade affordance integration" plan against the implemented E11 architecture. Correct the ticket assumptions before any code changes. This ticket is now a scope-correction and closure ticket, not a request to add trade-specific logic to the generic affordance layer.

## Assumption Reassessment (2026-03-11)

The original ticket assumptions were no longer accurate:

- `ActionPayload::Trade(TradeActionPayload)` already exists in [action_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs).
- The trade action handler already exists in [trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs) as `register_trade_action()`.
- Trade action duration is already modeled cleanly through `DurationExpr::ActorTradeDisposition` in [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs), so no trade-specific duration wiring is missing.
- The trade system dispatch slot is already wired to `trade_system_tick` in [lib.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs), and trade-system tests already cover that behavior in [trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade.rs).
- `BeliefView` does not currently need trade-component getters for this ticket. The generic affordance query only evaluates generic constraints, targets, and preconditions. Adding `merchandise_profile`, `demand_memory`, or similar methods here would not solve the real integration problem.

## Corrected Scope

This ticket does not add trade-specific affordance synthesis to `worldwake-sim::get_affordances()`.

This ticket also does not:

- auto-register trade inside `ActionDefRegistry::new()`
- add trade-specific variants to `Affordance`
- extend `BeliefView` solely for trade affordance generation
- duplicate valuation or bundle-search logic inside `worldwake-sim`

Instead, the corrected scope is:

- record the architectural reassessment
- confirm that the current E11 implementation already provides the generic trade action machinery
- explicitly reject the stale proposal to make `get_affordances()` fabricate concrete `TradeActionPayload` bundles
- close E11 with the current cleaner boundary: `worldwake-sim` exposes generic parameterized actions, while higher-level AI / CLI code must decide which concrete trade bundle to propose through `payload_override`

## Architecture Check

The originally proposed implementation is less robust than the current architecture.

Reasons:

- Generic affordance enumeration in [affordance_query.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs) is intentionally domain-agnostic. Making it invent trade bundles would push trade search and negotiation proposal logic into the wrong layer.
- A concrete trade action is not just "target this agent." It requires a parameterized bundle in `TradeActionPayload`. Generating that payload requires trade-domain reasoning about desired goods, offered goods, quantities, substitutes, and likely acceptance. That belongs with decision-making or explicit user input, not with generic action binding.
- Auto-registering trade inside `worldwake-sim` would invert crate boundaries by making the sim layer own system-specific registration. The current explicit registration model is cleaner and keeps composition at the boundary that assembles the runtime.
- Adding trade-only `BeliefView` methods here would create API surface without a generic affordance-layer need. E13 can extend belief queries where grounded planning actually needs them.

## Files Reassessed

- [action_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs)
- [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs)
- [affordance_query.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/affordance_query.rs)
- [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs)
- [omniscient_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/omniscient_belief_view.rs)
- [trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs)
- [trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade.rs)
- [lib.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/lib.rs)

## Corrected Acceptance Criteria

- The ticket documents which original assumptions were false.
- The ticket narrows scope to match the implemented architecture.
- No trade-specific bundle synthesis is added to generic affordance enumeration.
- Existing trade action machinery remains the E11 boundary:
  - explicit `register_trade_action()`
  - parameterized `TradeActionPayload`
  - `DurationExpr::ActorTradeDisposition`
  - `trade_system_tick` in system dispatch
- Relevant trade and workspace verification passes.

## Tests

Verification for this ticket is regression-only because the reassessment concluded that no runtime change is the correct implementation.

Targeted verification:

- `cargo test -p worldwake-systems trade`
- `cargo test -p worldwake-sim`

Full verification:

- `cargo test --workspace`
- `cargo clippy --workspace`

## Outcome

Completion date: 2026-03-11

What actually changed:

- Rewrote this ticket to match the implemented E11 architecture.
- Recorded that `ActionPayload::Trade`, `register_trade_action()`, `DurationExpr::ActorTradeDisposition`, and trade system dispatch were already implemented.
- Narrowed scope so this ticket no longer proposes trade-specific affordance synthesis inside `worldwake-sim`.

Deviations from original plan:

- The original plan to make `get_affordances()` construct concrete trade offers was rejected.
- No `BeliefView` trade-component extensions were added for this ticket.
- No runtime code changes were required; the correct result was a ticket-scope correction and closure.

Verification results:

- `cargo test -p worldwake-systems trade` passed.
- `cargo test -p worldwake-sim` passed.
- `cargo test --workspace` passed.
- `cargo clippy --workspace` passed.
