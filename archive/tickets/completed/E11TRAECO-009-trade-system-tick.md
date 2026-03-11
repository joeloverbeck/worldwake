# E11TRAECO-009: Implement Trade System Tick Function

**Status**: COMPLETED

## Summary
Replace the trade-system noop at `SystemId::Trade` with a real `trade_system_tick` that performs deterministic `DemandMemory` aging for agents. This ticket also corrects the original scope: unmet-demand recording is not implementable cleanly yet because the current codebase has no authoritative state or event that represents a failed trade-counterparty search.

## Dependencies
- Archived `E11TRAECO-003` (`DemandMemory`)
- Archived `E11TRAECO-004` (`TradeDispositionProfile`)
- Archived `E11TRAECO-008` (`trade_actions.rs`) for current trade module layout

## Assumption Reassessment (2026-03-11)

1. The original ticket assumed there was already a `crates/worldwake-systems/src/trade.rs` module to extend. That is stale. Today:
   - trade action behavior lives in `crates/worldwake-systems/src/trade_actions.rs`
   - the `SystemId::Trade` slot in `crates/worldwake-systems/src/lib.rs` is still wired to a shared noop
2. Creating a dedicated `crates/worldwake-systems/src/trade.rs` module is still the cleanest architecture for system-level trade behavior:
   - `trade_actions.rs` should continue to own action registration and commit-time exchange behavior
   - `trade.rs` should own trade-system ticking and future trade-domain query helpers
   This is a better long-term split than overloading `trade_actions.rs` with system-tick concerns.
3. The original ticket assumed unmet-demand recording could be done inside the system tick by inspecting failed trade searches. That is not true in the current architecture:
   - there is no implemented trade affordance generation yet
   - there is no authoritative event or state that records "wanted to buy/sell but no counterparty existed"
   - the trade action handler only sees concrete started actions, not failed search attempts
4. Adding hidden bookkeeping just so the tick can synthesize unmet-demand observations would be architecturally worse than the current design. It would introduce implicit market-state side channels instead of grounding observations in concrete action/affordance failures.
5. Because of that, this ticket must be narrowed to the part that is implementable and architecturally sound now: `DemandMemory` aging. Recording new unmet-demand observations should be deferred until the trade affordance/integration layer can emit a concrete failure signal through shared state or append-only events.

## Architecture Check

1. A dedicated `trade.rs` system module is more robust than putting the tick into `lib.rs` or folding it into `trade_actions.rs`. It keeps action semantics and system semantics separate while preserving a coherent trade-domain surface.
2. `DemandMemory` aging is aligned with the current architecture:
   - it operates on authoritative stored state
   - it is deterministic
   - it provides the physical dampener required by Principle 8
3. Unmet-demand recording is not yet justified by the current architecture. Without a real failed-attempt signal, the system would have to infer intent from global scans or hidden transient bookkeeping, which would weaken locality and state-mediated design.
4. The clean future path is:
   - `E11TRAECO-009`: age existing demand memory
   - later trade affordance/input integration: create a concrete, inspectable failed-attempt signal
   - then a later trade-system pass can translate that signal into `DemandObservation` entries
   That is stronger than inventing speculative storage now.

## Scope Correction

This ticket should:

1. Add `crates/worldwake-systems/src/trade.rs` with `trade_system_tick`.
2. Export the new trade module from `crates/worldwake-systems/src/lib.rs`.
3. Wire `SystemId::Trade` in the dispatch table to `trade_system_tick`.
4. Implement deterministic `DemandMemory` aging for agents that also have `TradeDispositionProfile`.
5. Emit normal append-only event-log deltas and `EventTag::System` / `EventTag::WorldMutation` tags when aging changes authoritative state.
6. Add focused tests for aging behavior and dispatch-table wiring.

This ticket should not:

1. Record new unmet-demand observations from failed searches.
2. Introduce hidden search-state caches, synthetic market memory, or compatibility shims.
3. Modify trade action registration/commit behavior from `E11TRAECO-008`.
4. Implement substitute demand (`E11TRAECO-010`).
5. Implement merchant restock queries (`E11TRAECO-011`).
6. Implement trade affordance generation / scheduler integration (`E11TRAECO-012`).

## Files to Touch
- `crates/worldwake-systems/src/trade.rs` — add `trade_system_tick` and aging helpers
- `crates/worldwake-systems/src/lib.rs` — export the trade module and wire the trade dispatch slot
- Existing worldwake-systems tests that currently assert the trade slot is still noop

## Out of Scope
- Unmet-demand observation recording
- Trade action handler changes
- Trade affordance generation
- Merchant restock
- Substitute demand
- Valuation logic
- Component/schema work already completed in `worldwake-core`

## Implementation Details

### Trade System Tick

The function signature must match `SystemFn`:

```rust
pub fn trade_system_tick(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>
```

The system should:

1. Inspect agents with `DemandMemory`.
2. For each such agent, read that agent's `TradeDispositionProfile`.
3. If the profile is missing, skip the agent without mutation.
4. Prune observations where `current_tick.0 - observation.tick.0 > demand_memory_retention_ticks as u64`.
5. Leave observations at the exact retention boundary intact.
6. Commit through the normal transaction/event-log boundary only if at least one agent's `DemandMemory` changed.

### Why Only Aging

`DemandMemory` aging is already backed by concrete state:
- authoritative stored observations
- per-agent retention policy in `TradeDispositionProfile`
- current simulation tick

Unmet-demand recording is not. There is currently no authoritative representation of:
- an attempted buy search that found no seller
- an attempted sell search that found no buyer

Until that signal exists in state or events, this ticket must not invent a hidden side channel to fabricate it.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-systems` — all existing tests pass
- New test: aging prunes observations older than per-agent retention threshold
- New test: aging respects per-agent `demand_memory_retention_ticks` for different agents
- New test: observations exactly at or within the retention window are preserved
- New test: agent without `TradeDispositionProfile` has no aging applied
- New test: agent without `DemandMemory` is skipped without panic
- New test: trade dispatch table routes `SystemId::Trade` to the real aging system rather than noop
- New test: `DemandMemory` aging is deterministic for the same input world/tick

### Invariants That Must Remain True
- Principle 8: demand memory ages out; no observation lives forever
- Principle 11: per-agent retention windows remain agent-specific
- No hidden market state or failed-search cache is introduced
- Function signature matches `SystemFn`
- No `HashMap`, no floats, no wall-clock time
- `cargo clippy --workspace` clean

## Outcome

- Outcome amended: 2026-03-11
- Completion date: 2026-03-11
- What actually changed:
  - Added `crates/worldwake-systems/src/trade.rs` with `trade_system_tick` and deterministic `DemandMemory` aging.
  - Wired `SystemId::Trade` in `crates/worldwake-systems/src/lib.rs` to the real trade system instead of the shared noop.
  - Added focused `worldwake-systems` tests for retention pruning, per-agent retention diversity, boundary preservation, missing-component skips, dispatch wiring, and determinism.
  - Restored schema-driven `WorldTxn` setter generation for the full trade-domain component set (`DemandMemory`, `TradeDispositionProfile`, `MerchandiseProfile`, `SubstitutePreferences`) by fixing the duplicated `select_txn_simple_set_components` list in `crates/worldwake-core/src/component_schema.rs`.
  - Removed the ad hoc manual trade-specific setters from `crates/worldwake-core/src/world_txn.rs` once the schema-driven path covered them again.
  - Added focused `worldwake-core` `WorldTxn` regression tests for the trade-domain setter surface so future component-schema drift is caught directly.
  - Updated the old dispatch-table tests in `needs.rs` and `production.rs` so they verify the new trade-slot behavior instead of asserting the outdated noop architecture.
- Deviations from original plan:
  - The ticket was corrected before implementation because the original assumptions about module layout and unmet-demand recording were stale.
  - The implementation intentionally did not record new `DemandObservation`s for failed buy/sell searches. That signal does not yet exist as authoritative shared state, and inventing hidden bookkeeping would weaken the architecture.
  - The dedicated `trade.rs` module was added as the clean long-term home for trade system logic, while `trade_actions.rs` continues to own action-handler behavior.
  - The follow-up architecture refinement went beyond the initial minimal fix by correcting the duplicated schema metadata that had allowed `WorldTxn`'s generated setter surface to drift out of sync with the authoritative component list.
- Verification results:
  - `cargo test -p worldwake-core world_txn -- --nocapture` passed.
  - `cargo test -p worldwake-systems` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
