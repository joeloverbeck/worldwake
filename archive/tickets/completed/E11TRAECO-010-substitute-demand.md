# E11TRAECO-010: Implement Deterministic Substitute Trade Selection

**Status**: COMPLETED

## Summary
Implement the substitute-demand seam that fits the current trade architecture: when an actor has a desired commodity and `SubstitutePreferences`, trade code must be able to deterministically identify a locally available, valuation-approved substitute candidate in preference order.

This ticket does not authorize hidden mutation of an in-flight trade action. A trade action payload is concrete per attempt; if the actor wants to pursue a substitute, that must become a new explicit trade proposal rather than a commit-time rewrite of the original payload.

## Dependencies
- Archived `E11TRAECO-005` (`SubstitutePreferences`)
- Archived `E11TRAECO-007` (`evaluate_trade_bundle`)
- Archived `E11TRAECO-008` (trade action handler / payload seam)
- Archived `E11TRAECO-009` (trade tick exists, but only ages `DemandMemory`)

## Assumption Reassessment (2026-03-11)

1. The original ticket targeted `crates/worldwake-systems/src/trade.rs`. That is stale:
   - `trade.rs` only ages `DemandMemory`.
   - concrete trade execution lives in `crates/worldwake-systems/src/trade_actions.rs`.
2. The original ticket assumed substitute demand should run "within trade handler flow" by changing what gets bought after a preferred trade fails. That is the wrong architectural seam:
   - `TradeActionPayload` is explicit attempt state.
   - commit-time handler logic should validate and execute the attempted bundle, not silently replace it with another commodity.
   - hidden fallback inside commit would make action outcomes non-local and harder to reason about in event traces.
3. The current code already has the inputs needed for deterministic substitute selection:
   - `SubstitutePreferences` in `worldwake-core`
   - `CommodityKind::spec().trade_category`
   - local co-location queries in `trade_actions.rs`
   - `evaluate_trade_bundle` for buyer-side acceptance
4. The original demand-memory assumption was also stale. No current code records `WantedToBuyButSellerOutOfStock` or `WantedToBuyButTooExpensive` during trade attempts; `trade_system_tick` only prunes memory. Recording new observations is a separate concern and should not be coupled to substitute selection here.
5. The original wording implied "preferred good unavailable" means the trade handler already knows actor intent beyond the concrete payload. It does not. At this phase boundary, the clean seam is a helper that starts from a desired commodity and current local state, so future affordance/query layers can explicitly request the substitute trade they want.

## Architecture Check

1. The current architecture is better served by explicit substitute selection than by automatic fallback inside `commit_trade`.
2. The clean long-term path is:
   - trade/affordance code determines the desired commodity
   - substitute-selection code proposes the first acceptable local substitute
   - the caller starts a new explicit trade action with that substitute payload
3. This is more robust than auto-retrying inside the action handler because:
   - action history remains truthful
   - event-log causality stays inspectable
   - human and AI control stay symmetric
   - future planners can reason about alternatives before committing time to a negotiation
4. No compatibility wrapper or alias path should be introduced. We should add one deterministic selection path and have future callers use it directly.
5. A likely future refinement is to move trade-candidate generation into affordance/query layers once `E11TRAECO-012` is corrected and implemented. This ticket should provide the deterministic core logic that those layers can reuse.

## Scope Correction

This ticket should:

1. Add deterministic substitute-candidate selection to `crates/worldwake-systems/src/trade_actions.rs`.
2. Reuse `evaluate_trade_bundle` from the buyer's perspective to reject bad substitutes.
3. Respect `SubstitutePreferences` ordering and locality.
4. Return explicit substitute trade terms that a caller can use to start a new trade attempt.
5. Add focused tests covering preference order, locality, no-preferences behavior, and valuation rejection.

This ticket should not:

1. Rewrite an active trade action payload at commit time.
2. Add substitute-demand behavior to `crates/worldwake-systems/src/trade.rs`.
3. Record new `DemandObservation` entries.
4. Generate trade affordances or planner goals directly. That remains integration work.
5. Add hidden market state, default substitute tables, or compatibility shims.

## Files to Touch
- `crates/worldwake-systems/src/trade_actions.rs` — substitute-selection helper and tests

## Out of Scope
- `trade_system_tick` demand-memory recording
- trade affordance generation / registry wiring
- planner/GOAP use of substitute candidates
- merchant restock logic
- `BeliefView` trait expansion

## Implementation Details

Add a helper in `trade_actions.rs` that, given:
- buyer
- desired commodity and quantity
- offered payment commodity and quantity
- current place

does the following:

1. Read the buyer's `SubstitutePreferences`. If absent, return no substitute candidate.
2. Read the `TradeCategory` of the desired commodity.
3. Walk `preferences[category]` in order.
4. Skip the original desired commodity to avoid pretending the same commodity is a substitute.
5. For each substitute commodity, scan co-located agents deterministically.
6. For each seller with accessible stock of that substitute, evaluate the candidate bundle from the buyer's perspective using `evaluate_trade_bundle`.
7. Return the first accepted candidate with explicit seller / commodity / quantity terms.

Determinism requirements:
- seller scan order must be stable
- commodity preference order must come directly from the stored `Vec`
- no `HashMap`/`HashSet`

The helper may live as a private function if tests exercise it through the trade-actions module. It should not mutate world state.

## Acceptance Criteria

### Tests That Must Pass
- `cargo test -p worldwake-systems trade_actions`
- New test: first acceptable substitute is chosen in stored preference order
- New test: unavailable earlier substitute is skipped for a later available substitute
- New test: buyer without `SubstitutePreferences` gets no substitute candidate
- New test: non-co-located sellers are ignored
- New test: valuation-rejected substitute is skipped, and later acceptable substitute is chosen if present

### Invariants That Must Remain True
- Principle 7: substitute selection consults only co-located sellers
- no hidden mutation of active trade payloads
- deterministic ordering from `Vec` + sorted entity iteration
- no global market state
- no `f32`/`f64`
- `cargo clippy --workspace --all-targets -- -D warnings` clean

## Test Plan

### New/Modified Tests
1. `crates/worldwake-systems/src/trade_actions.rs` — preference-order selection coverage
2. `crates/worldwake-systems/src/trade_actions.rs` — locality coverage for substitute sellers
3. `crates/worldwake-systems/src/trade_actions.rs` — valuation-rejection coverage for bad substitutes
4. `crates/worldwake-systems/src/trade_actions.rs` — no-preferences / no-candidate coverage

### Commands
1. `cargo test -p worldwake-systems trade_actions`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - Corrected the ticket away from commit-time fallback mutation and toward a deterministic substitute-selection seam that matches the existing trade action architecture.
  - Added `select_substitute_trade_candidate` and `SubstituteTradeCandidate` to `crates/worldwake-systems/src/trade_actions.rs`.
  - The helper now reads `SubstitutePreferences`, uses the desired commodity's `TradeCategory`, scans co-located sellers deterministically, and reuses `evaluate_trade_bundle` to reject bad substitutes.
  - Added focused unit coverage for preference order, unavailable earlier preferences, no-preferences behavior, locality, and valuation-driven skipping.
- Deviations from original plan:
  - The original ticket proposed adding substitute demand to `crates/worldwake-systems/src/trade.rs`. That was incorrect because `trade.rs` only handles demand-memory aging; trade execution lives in `trade_actions.rs`.
  - The original plan implied silently switching commodities inside an already-started trade action. That was rejected as the wrong seam for a robust long-term architecture; substitute pursuit should become a new explicit trade proposal.
  - No demand-memory recording was added here because the current trade runtime still has no clean observation-recording seam for failed attempts.
- Verification results:
  - `cargo test -p worldwake-systems trade_actions` passed.
  - `cargo test -p worldwake-systems` passed.
  - `cargo clippy --workspace --all-targets -- -D warnings` passed.
  - `cargo test --workspace` passed.
