# E11TRAECO-008: Implement Trade Action Handler

**Status**: COMPLETED

## Summary
Implement the action registration and commit-time handler for concrete co-located trade. A completed trade action must validate the proposed bundle against authoritative state, transfer existing lots through ownership/possession changes only, append `LotOperation::Traded` provenance to the transferred lots, and emit normal append-only event-log deltas tagged for trade.

This ticket also corrects a missing action-framework seam: trade terms are concrete per-attempt state, not reusable action-definition metadata. The action runtime must therefore carry the concrete trade payload on the started action instance, with the affordance/input request path able to provide a per-instance payload override.

## Dependencies
- Archived `E11TRAECO-001` (`LotOperation::Traded`)
- Archived `E11TRAECO-006` (`TradeActionPayload`)
- Archived `E11TRAECO-007` (`evaluate_trade_bundle`)
- Archived component tickets for trade schema in `worldwake-core`

## Assumption Reassessment (2026-03-11)

1. The original ticket assumed several trade prerequisites were still missing. That is stale:
   - `LotOperation::Traded` already exists in `worldwake-core`.
   - `TradeActionPayload` and `ActionPayload::Trade` already exist in `worldwake-sim`.
   - `evaluate_trade_bundle` already exists in `crates/worldwake-sim/src/trade_valuation.rs`.
   - Trade components already exist in `crates/worldwake-core/src/trade.rs`.
2. The original file plan was outdated. There is no existing `crates/worldwake-systems/src/trade.rs`, and creating one just for the action handler is not the cleanest split. Action registration/handler logic should live in a focused `trade_actions.rs` module. The trade system tick remains separate work for `E11TRAECO-009`.
3. The original ticket treated `TradeActionPayload` as if definition-scoped payload were sufficient. That is the wrong architectural seam for trade:
   - `ActionDef` describes reusable semantics.
   - A trade proposal is concrete per action attempt.
   - Keeping trade terms only on `ActionDef` was tolerable for recipe-backed actions but is a poor long-term fit for negotiation bundles.
   - This ticket must therefore migrate concrete trade payload ownership to `ActionInstance` and add an affordance/input override seam so a request can instantiate a generic trade definition with concrete bundle terms.
4. The original ticket promised a dedicated trade event with structured fields such as failure reason. The current event system does not support bespoke event payload types; it records `StateDelta`s plus stable tags. This ticket should use that model instead of inventing parallel event plumbing.
5. The original ticket claimed duration could come from the initiator's `TradeDispositionProfile` directly through the existing action definition surface. That is not true today because `DurationExpr` has no variant for trade-disposition-backed resolution. This ticket should add the needed duration expression rather than hardcoding a fixed tick count externally.
6. The original conservation/test wording was stale. The authoritative helpers are `verify_live_lot_conservation` and `verify_authoritative_conservation`, not `verify_conservation`.

## Architecture Check

1. Moving concrete payload onto `ActionInstance` is more robust than keeping trade terms on `ActionDef`. Reusable definitions and concrete invocations should not share the same storage slot.
2. The handler should stay inside `worldwake-systems`, while valuation remains in `worldwake-sim`. That preserves the current separation: simulation runtime provides the action framework and belief-facing helper; systems provide world-domain behavior.
3. Trade should continue to mutate shared world state and the append-only event log only:
   - relation changes for owner/possessor transfer
   - item-lot provenance append
   - event tags and state deltas
   No dedicated trade subsystem or hidden market object should be introduced.
4. The clean long-term path is:
   - generic trade action semantics in the registry
   - concrete proposal terms carried on the action instance
   - affordance/input layers deciding which proposal to instantiate through per-instance payload overrides
   That is better than encoding trade proposals into a registry explosion of definition variants.
5. This ticket should not introduce backward-compatibility aliasing between definition payloads and instance payloads. The model changes once; existing handlers and tests are updated to the new seam.

## Scope Correction

This ticket should:

1. Add `crates/worldwake-systems/src/trade_actions.rs` with trade action registration and handler logic.
2. Export trade action registration from `crates/worldwake-systems/src/lib.rs`.
3. Move concrete action payload ownership onto `crates/worldwake-sim/src/action_instance.rs`, and update action start/runtime code to use it.
4. Add a per-instance payload override seam on affordances/input requests so generic action definitions can be instantiated with concrete trade terms.
5. Add the minimal `DurationExpr` support needed to resolve negotiation duration from `TradeDispositionProfile`.
6. Add the minimal `WorldTxn`/world mutation support needed to append `LotOperation::Traded` provenance entries cleanly through the event-sourced boundary.
7. Add focused unit/integration tests around successful and failed trade commit behavior.

This ticket should not:

1. Implement trade affordance generation (`E11TRAECO-012`).
2. Implement demand-memory aging or unmet-demand recording (`E11TRAECO-009`).
3. Implement substitute demand (`E11TRAECO-010`).
4. Implement merchant restock queries (`E11TRAECO-011`).
5. Introduce bespoke trade event payload structs, global price state, or compatibility wrappers.

## Files to Touch
- `crates/worldwake-systems/src/trade_actions.rs` — new action registration + handler module
- `crates/worldwake-systems/src/lib.rs` — module export / re-export
- `crates/worldwake-sim/src/action_instance.rs` — carry concrete action payload on the instance
- `crates/worldwake-sim/src/affordance.rs` — carry optional per-instance payload overrides
- `crates/worldwake-sim/src/input_event.rs` — allow action requests to provide payload overrides
- `crates/worldwake-sim/src/tick_step.rs` — thread request payload overrides into resolved affordances
- `crates/worldwake-sim/src/start_gate.rs` — copy affordance override or definition payload into new instances
- `crates/worldwake-sim/src/action_semantics.rs` — add duration resolution from `TradeDispositionProfile`
- `crates/worldwake-core/src/world_txn.rs` — add provenance-append support through the transaction boundary
- Existing action tests that currently assume payload lives only on `ActionDef`

## Out of Scope
- Trade system tick (`E11TRAECO-009`)
- Trade affordance wiring (`E11TRAECO-012`)
- Merchant restock (`E11TRAECO-011`)
- Substitute demand (`E11TRAECO-010`)
- BeliefView expansion beyond what the trade handler needs today

## Implementation Details

### Action Registration
Register a `trade` action definition with:
- `interruptibility: FreelyInterruptible`
- `visibility: VisibilitySpec::SamePlace`
- `causal_event_tags`: include `EventTag::Trade`, `EventTag::Transfer`, and `EventTag::WorldMutation`
- Preconditions:
  - `Precondition::ActorAlive`
  - `Precondition::TargetExists(0)`
  - `Precondition::TargetAtActorPlace(0)`
  - target kind must be `EntityKind::Agent`
- Commit conditions:
  - both parties remain co-located
  - seller still controls the requested goods
  - buyer still controls the offered payment/goods
- Duration:
  - resolved from the initiator's `TradeDispositionProfile.negotiation_round_ticks`

### Concrete Trade Terms
`TradeActionPayload` must be carried by the action instance, not only the definition.

Required runtime behavior:
1. `ActionDef` remains generic for trade.
2. The affordance/input request layer may provide a concrete `payload_override`.
3. When an action starts, clone the payload override if present; otherwise clone the definition payload onto the new `ActionInstance`.
4. Action handlers read the instance payload.
5. Trade commit rejects instances whose payload is absent or not `ActionPayload::Trade`.

### Commit Logic
At commit:

1. Read the instance payload.
2. Determine buyer/seller orientation from the payload:
   - actor offers `offered_*`
   - counterparty offers `requested_*`
3. Revalidate co-location and current control over both lots.
4. Resolve concrete controlled lots at the shared place.
5. For each side, call `evaluate_trade_bundle` from that side's perspective.
6. Abort the deal if either side rejects.
7. For partial-lot transfers, split first through `WorldTxn::split_lot`.
8. Transfer both `owned_by` and `possessed_by` relations for the exchanged lots.
9. Append `LotOperation::Traded` provenance entries to each transferred lot through `WorldTxn`.
10. Commit through normal event-log deltas and tags; do not add custom event payload infrastructure.

### Provenance
The provenance append must go through `WorldTxn`, not by mutating `World` behind the transaction boundary.

Each transferred lot gets a new `ProvenanceEntry` with:
- current tick
- `operation: LotOperation::Traded`
- `event_id: None` at staging time, consistent with current transaction patterns
- `related_lot`: optional counter-lot when useful
- `amount`: transferred quantity

### Conservation
Trade transfers existing lots only. No trade path may create or destroy quantity. Partial transfers must use `split_lot`, and the final state must satisfy the live-lot conservation helpers.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-sim`
- `cargo test -p worldwake-systems`
- New test: action instances carry cloned payload copied from the action definition at start
- New test: trade action registration resolves duration from `TradeDispositionProfile`
- New test: successful trade transfers goods and coin between co-located agents
- New test: successful trade appends `LotOperation::Traded` provenance to transferred lots
- New test: partial-lot trade splits first and preserves total quantity
- New test: trade fails when agents are no longer co-located at commit
- New test: trade fails when either side no longer controls the required lot
- New test: trade fails when either side rejects the bundle via `evaluate_trade_bundle`
- New test: successful trade emits a normal committed event tagged with `Trade`
- New test: `verify_live_lot_conservation` still passes after trade

### Invariants That Must Remain True
- Conservation through ownership/possession transfer only
- No negative quantities
- No hidden market state or price table
- Negotiation consumes time via `TradeDispositionProfile`
- Locality: only co-located counterparties can trade
- Append-only event log; no bespoke mutable side channel
- No backwards-compatibility alias path for old payload ownership
- `cargo clippy --workspace` clean

## Test Plan

### New/Modified Tests
1. `crates/worldwake-sim/src/action_instance.rs` — instance serialization/trait tests updated for payload ownership
2. `crates/worldwake-sim/src/start_gate.rs` — starting an action copies payload into the concrete instance
3. `crates/worldwake-systems/src/trade_actions.rs` — registration, commit success, commit rejection, provenance, conservation
4. Existing action-handler tests that currently assume payload is definition-only

### Commands
1. `cargo test -p worldwake-sim`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Added `crates/worldwake-systems/src/trade_actions.rs` with trade action registration, commit-time validation, bilateral valuation checks, deterministic lot selection, partial-lot splitting, ownership/possession transfer, and `LotOperation::Traded` provenance updates.
  - Exported `register_trade_action` from `crates/worldwake-systems/src/lib.rs`.
  - Moved concrete action payload ownership onto `ActionInstance` in `crates/worldwake-sim/src/action_instance.rs`, and added an explicit per-instance payload override seam across `Affordance`, `InputKind::RequestAction`, `tick_step`, and `start_action`.
  - Simplified trade registration so `register_trade_action` now defines generic trade semantics and concrete bundle terms enter through the affordance/request payload override path instead of definition registration.
  - Added a handler-driven abort path in `tick_action` via `ActionError::AbortRequested`, so mutually rejected or invalidated trades abort cleanly instead of surfacing as runtime errors.
  - Added `DurationExpr::ActorTradeDisposition` so negotiation duration resolves from `TradeDispositionProfile.negotiation_round_ticks`.
  - Added `WorldTxn::append_lot_provenance` and an explicit `WorldTxn::set_component_trade_disposition_profile` mutation path so trade provenance and setup remain inside the event-sourced transaction boundary.
  - Updated existing harvest/craft action helpers and tests to read payload from the action instance instead of the definition.
- Deviations from original plan:
  - The ticket was corrected before implementation because the original assumptions about missing trade prerequisites, dedicated trade event payloads, and definition-scoped trade payloads were stale or architecturally wrong.
  - The implementation did not add a dedicated `trade.rs` module. The cleaner split was to keep action behavior in `trade_actions.rs`; the trade system tick remains for `E11TRAECO-009`.
  - The event log continues to use normal `StateDelta` records plus `EventTag::Trade`/`Transfer`, rather than bespoke trade event payload structs.
  - The runtime needed two additional seams beyond the original ticket text: handler-requested aborts at commit time, and a request/affordance payload override path so trade no longer depends on definition-scoped proposal terms.
- Verification results:
  - `cargo test -p worldwake-sim` passed.
  - `cargo test -p worldwake-systems` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
