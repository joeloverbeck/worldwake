# E11TRAECO-012: Wire Trade into Affordance Query and System Dispatch

## Summary
Wire the trade action definition into the `ActionDefRegistry`, add trade affordance generation to `get_affordances()`, and register the trade system tick in the `SystemDispatchTable`. This is the integration ticket that makes trade available to agents.

## Dependencies
- E11TRAECO-006 (TradeActionPayload)
- E11TRAECO-008 (trade action handler)
- E11TRAECO-009 (trade system tick)
- All component tickets (E11TRAECO-002 through 005)

## Files to Touch
- `crates/worldwake-sim/src/action_def_registry.rs` — register trade action def
- `crates/worldwake-sim/src/affordance_query.rs` — add trade affordance generation
- `crates/worldwake-sim/src/affordance.rs` — may need trade-specific affordance variant (if not already generic enough)
- `crates/worldwake-sim/src/belief_view.rs` — add any methods needed for trade queries (e.g., `merchandise_profile`, `substitute_preferences`)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` — implement new BeliefView methods

## Out of Scope
- Trade handler implementation details (E11TRAECO-008)
- Valuation logic (E11TRAECO-007)
- Component definitions (core tickets)
- GOAP/AI integration (E13)
- CLI integration (E21)

## Implementation Details

### Action Definition Registration
Register a trade `ActionDef` with:
- Name: `"trade"`
- Targets: `[TargetSpec::SpecificEntity(counterparty)]`
- Preconditions: `ActorAlive`, `TargetAtActorPlace(0)`
- Duration: derived from initiator's `TradeDispositionProfile.negotiation_round_ticks`
- Interruptibility: `FreelyInterruptible`
- Visibility: `SamePlace`

### Affordance Generation
In `get_affordances()`, for each agent at a place:
- Find co-located agents
- For each co-located agent who possesses goods the actor wants (or vice versa), emit a trade affordance
- Affordance includes the counterparty EntityId

### BeliefView Extensions
If needed, add methods for querying trade components:
- `merchandise_profile(entity) -> Option<&MerchandiseProfile>`
- `trade_disposition_profile(entity) -> Option<&TradeDispositionProfile>`
- `substitute_preferences(entity) -> Option<&SubstitutePreferences>`
- `demand_memory(entity) -> Option<&DemandMemory>`

Implement these on `OmniscientBeliefView`.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-sim` — all existing tests pass
- New test: trade action def is present in registry
- New test: trade affordance appears for co-located agents with tradeable goods
- New test: trade affordance does NOT appear for agents at different places
- New test: OmniscientBeliefView correctly returns trade component data
- `cargo test --workspace` — full workspace passes

### Invariants That Must Remain True
- Principle 7: affordances only consider co-located agents
- SystemId::Trade dispatch slot wired to real trade_system_tick
- No regressions in existing affordance generation
- Deterministic affordance ordering
- `cargo clippy --workspace` clean
