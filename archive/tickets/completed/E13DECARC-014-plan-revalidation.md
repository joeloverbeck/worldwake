# E13DECARC-014: Plan revalidation by affordance identity

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Sim/AI/shared action-framework extension
**Deps**: E13DECARC-005, E13DECARC-009

## Problem

Before executing the next plan step, the agent must verify the step is still valid by checking that the same executable affordance binding still exists in the current beliefs. This is the canonical revalidation rule. E13 must not duplicate precondition logic from the action framework, and it must not invent an AI-only notion of executable identity that diverges from the sim.

## Assumption Reassessment (2026-03-11)

1. `get_affordances(view, actor, registry, handlers)` returns `Vec<Affordance>` — confirmed.
2. `Affordance` has `def_id`, `actor`, `bound_targets`, `payload_override` — confirmed.
3. `PlannedStep` already exists in `crates/worldwake-ai/src/planner_ops.rs` with `def_id`, ordered `targets`, `payload_override`, `op_kind`, `estimated_ticks`, and `is_materialization_barrier`.
4. `ActionPayload` already derives `Eq`, `PartialEq`, `Ord`, and `PartialOrd` in `crates/worldwake-sim/src/action_payload.rs`; no trait work is needed here.
5. `ActionHandler` can now own affordance payload enumeration through an explicit `affordance_payloads` hook, allowing each system to surface payload-distinct executable variants without hardcoding domain logic in `worldwake-sim`.
6. `crates/worldwake-sim/src/tick_step.rs` now resolves executable availability against the shared exact request identity rule by querying `get_affordances(..., handlers)`.
7. There was no existing `crates/worldwake-ai/src/plan_revalidation.rs` stub. This ticket added the module.

## Architecture Check

1. Revalidation is still by affordance identity comparison, not by re-implementing preconditions.
2. The canonical executable-availability identity is now the exact effective request identity surfaced by the sim: actor, action definition, ordered targets, and effective payload after default normalization.
3. E13DECARC-014 should centralize that identity rule in a shared helper and make both AI revalidation and `tick_step` reuse it.
4. Payload-distinct affordances should be enumerated by the action framework through handlers, not reconstructed in the AI layer and not hardcoded in sim query code by action name.
5. This is a cleaner long-term architecture because execution, legality, search, and revalidation all consume the same sim-owned executable variants.
6. If no matching affordance identity exists, the step is invalid and triggers failure handling during later integration work (E13DECARC-013 / E13DECARC-016).

## What to Change

### 1. Add a shared affordance/request identity helper in `worldwake-sim`

Implement a small shared matcher used anywhere the codebase needs to answer:
"Does the current belief view expose the same executable affordance binding for this request?"

This helper must compare:
- same actor
- same `def_id`
- same ordered targets
- same effective payload after applying definition defaults

`tick_step` should stop open-coding this comparison and should use the shared helper.

### 2. Implement revalidation in `worldwake-ai/src/plan_revalidation.rs`

```rust
pub fn revalidate_next_step(
    view: &dyn BeliefView,
    actor: EntityId,
    step: &PlannedStep,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> bool
```

Steps:
1. Call `get_affordances(view, actor, registry, handlers)`
2. Search for an affordance matching the shared executable identity rule:
   - same actor
   - same `def_id`
   - same ordered `targets`
   - same effective payload
3. Return `true` if found, `false` otherwise

### 3. Surface payload-distinct executable variants through action handlers

`worldwake-sim` should not hardcode dynamic payload enumeration for specific actions. Instead, `ActionHandler` should expose an affordance-payload enumeration hook, and systems should register their own concrete payload variants there.

This keeps payload materialization:
- close to the system that owns the action semantics
- reusable by search, revalidation, and execution
- extensible for future payload-bearing actions without editing sim query code

### 4. Export the revalidation API from `worldwake-ai`

This keeps E13DECARC-016 integration straightforward without making `agent_tick` own the matching details.

## Files to Touch

- `crates/worldwake-ai/src/plan_revalidation.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — export revalidation API)
- `crates/worldwake-ai/src/search.rs` (modify — query affordances with handlers)
- `crates/worldwake-sim/src/action_handler.rs` (modify — handler affordance payload hook)
- `crates/worldwake-sim/src/affordance.rs` (modify — shared identity helper + tests)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — use handler-provided payload variants)
- `crates/worldwake-sim/src/tick_step.rs` (modify — reuse shared helper)
- `crates/worldwake-systems/src/combat.rs` (modify — attack/loot payload enumeration)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — trade bundle enumeration)

## Out of Scope

- Precondition logic duplication — explicitly forbidden
- Failure handling after invalid revalidation — E13DECARC-013
- Agent tick integration — E13DECARC-016
- Reactive interrupts — E13DECARC-015
- Future payload-bearing action families beyond the current registered handlers

## Acceptance Criteria

### Tests That Must Pass

1. Step with matching affordance identity in current beliefs returns `true`
2. Step with same `def_id` but different targets returns `false`
3. Step with same actor/targets but different effective payload returns `false`
4. Step with no matching `def_id` returns `false`
5. Shared affordance/request identity helper is reused by both AI revalidation and `tick_step`
6. Revalidation uses `get_affordances(..., handlers)` — no custom precondition code (verified by code review / grep)
7. Dynamic payload-bearing actions surface executable variants through registered handlers rather than sim-side action-name branching
8. Existing suite: `cargo test --workspace`

### Invariants

1. No precondition logic duplicated from action framework
2. Executable affordance identity is exact normalized request identity: actor, `def_id`, ordered targets, and effective payload
3. Revalidation is a pure read operation
4. Affordance matching is exact for ordered targets
5. `payload_override` is preserved for execution and revalidated only through sim-enumerated executable variants

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_revalidation.rs` — revalidation tests with mock BeliefView, defs, and handlers
2. `crates/worldwake-sim/src/affordance.rs` — shared affordance/request identity tests
3. `crates/worldwake-sim/src/affordance_query.rs` — handler-driven affordance payload enumeration tests
4. `crates/worldwake-sim/src/tick_step.rs` — regression test that request resolution still follows the shared identity rule
5. `crates/worldwake-systems/src/trade_actions.rs` — concrete trade bundle affordance enumeration test

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Outcome amended: 2026-03-11
- Completion date: 2026-03-11
- Actual changes:
  - Added `worldwake_ai::revalidate_next_step()` in `crates/worldwake-ai/src/plan_revalidation.rs`
  - Upgraded sim affordance identity from target binding only to exact effective execution identity, including normalized payload matching through `Affordance::matches_request_identity()` in `crates/worldwake-sim/src/affordance.rs`
  - Extended the action framework with `ActionHandler::affordance_payloads`, so systems can enumerate payload-distinct executable affordances without sim-side action-name branching
  - Updated `get_affordances()` in `crates/worldwake-sim/src/affordance_query.rs` to combine static payloads with handler-provided dynamic payload variants
  - Updated `crates/worldwake-ai/src/search.rs` to consume affordance-provided payload identity as the canonical payload source
  - Updated `crates/worldwake-sim/src/tick_step.rs` to resolve requests against the same effective identity rule
  - Exported the revalidation API from `crates/worldwake-ai/src/lib.rs`
  - Moved dynamic payload enumeration ownership into the responsible systems: combat enumerates attack/loot payloads and trade enumerates concrete mutually acceptable trade bundles
  - Updated combat integration tests to request explicit loot payloads, matching the new exact affordance identity
  - Added focused tests for AI revalidation, payload-aware affordance identity, handler-driven payload expansion, trade bundle enumeration, and `tick_step` request resolution
- Deviations from original plan:
  - Did not add any `ActionPayload` trait impls because they already existed
  - Expanded beyond the originally archived scope by implementing payload-aware executable identity instead of stopping at `(actor, def_id, ordered targets)`
  - Normalized payload matching against the action definition default payload so explicit and implicit default-payload requests remain the same executable identity
  - Replaced the interim sim-owned hardcoded dynamic payload expansion with handler-owned enumeration in the action framework
  - Trade affordances now surface concrete mutually acceptable `1 coin -> 1 unit` bundles for current Phase 2 planner-visible trade behavior through the trade handler rather than a payload-free seller marker
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo test -p worldwake-sim` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
