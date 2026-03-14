# E14PERBEL-006: Migrate agent_tick.rs to PerAgentBeliefView and Delete OmniscientBeliefView

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI crate belief view migration, deletion of omniscient view
**Deps**: E14PERBEL-004 (PerAgentBeliefView must be fully implemented), E14PERBEL-005 (perception system must populate belief stores)

## Problem

The AI crate currently constructs `OmniscientBeliefView` to give agents perfect world knowledge. After E14PERBEL-004 provides `PerAgentBeliefView` and E14PERBEL-005 populates belief stores, all AI call sites must switch to the new view. Once migrated, `OmniscientBeliefView` and `OmniscientBeliefRuntime` are deleted entirely — not wrapped, not retained (spec requirement).

## Assumption Reassessment (2026-03-14)

1. `runtime_belief_view()` helper in `agent_tick.rs` creates `OmniscientBeliefView::with_runtime()` — confirmed, this is the primary factory.
2. ~16 call sites in `agent_tick.rs` use the belief view — confirmed from exploration.
3. `OmniscientBeliefView` is defined in `crates/worldwake-sim/src/omniscient_belief_view.rs` (~1515 lines) — confirmed.
4. `OmniscientBeliefRuntime` is in the same file — confirmed.
5. Affordance queries already use `&dyn BeliefView` — concrete type change is transparent to them.
6. No other crate besides `worldwake-ai` constructs `OmniscientBeliefView` for agent reasoning — confirmed (systems use World directly, which is correct).
7. `TickInputContext` in `tick_input_producer.rs` may also reference `OmniscientBeliefView` — needs verification during implementation.

## Architecture Check

1. Migration is mechanical: replace `OmniscientBeliefView::with_runtime(world, runtime)` with `PerAgentBeliefView::new(agent, world, belief_store, runtime)` at each call site.
2. Deletion is clean: after migration, `omniscient_belief_view.rs` is entirely removed, `lib.rs` updated.
3. Compile-time enforcement: after deletion, no code in `worldwake-ai` can accidentally use `OmniscientBeliefView` — it won't exist.

## What to Change

### 1. Update `runtime_belief_view()` in `agent_tick.rs`

Change the factory function to construct `PerAgentBeliefView` instead of `OmniscientBeliefView`:

```rust
fn runtime_belief_view<'a>(
    agent: EntityId,
    world: &'a World,
    belief_store: &'a AgentBeliefStore,
    scheduler: &'a Scheduler,
    action_defs: &'a ActionDefRegistry,
) -> PerAgentBeliefView<'a> {
    PerAgentBeliefView::with_runtime(
        agent,
        world,
        belief_store,
        PerAgentBeliefRuntime::new(scheduler.active_actions(), action_defs),
    )
}
```

### 2. Update all ~16 call sites in `agent_tick.rs`

Each call to `runtime_belief_view()` needs the agent's `EntityId` and `&AgentBeliefStore` passed. The belief store is retrieved from `World` component tables.

### 3. Update `TickInputContext` and `autonomous_controller.rs` if needed

Check if `TickInputContext` or the autonomous controller trait passes or constructs `OmniscientBeliefView`. Update to use `PerAgentBeliefView`.

### 4. Ensure agent creation paths attach belief components

All agent creation paths must attach `AgentBeliefStore` and `PerceptionProfile` so the AI can read them. This includes:
- Test utilities in `worldwake-core/src/test_utils.rs`
- CLI agent creation in `worldwake-cli`
- Any `build_prototype_world()` helper

### 5. Delete `omniscient_belief_view.rs`

Remove the entire file `crates/worldwake-sim/src/omniscient_belief_view.rs`.

### 6. Update `crates/worldwake-sim/src/lib.rs`

Remove `pub mod omniscient_belief_view;` and all re-exports of `OmniscientBeliefView` and `OmniscientBeliefRuntime`.

### 7. Remove all remaining references

Grep across workspace for `OmniscientBeliefView` and `OmniscientBeliefRuntime` and remove/update any remaining imports or references. This may include:
- `crates/worldwake-ai/src/lib.rs` or other AI modules that import it
- Test files that construct `OmniscientBeliefView` for test harnesses

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — migrate ~16 call sites)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (delete)
- `crates/worldwake-sim/src/lib.rs` (modify — remove module + re-exports)
- `crates/worldwake-sim/src/tick_input_producer.rs` (modify — if it references OmniscientBeliefView)
- `crates/worldwake-ai/src/agent_tick.rs` (modify — update factory function)
- `crates/worldwake-core/src/test_utils.rs` (modify — add belief components to test agents)
- `crates/worldwake-cli/src/*.rs` (modify — add belief components to CLI agent creation)
- Any test files importing `OmniscientBeliefView` (modify — switch to PerAgentBeliefView)

## Out of Scope

- Changing `BeliefView` trait (no changes needed)
- Modifying action handlers or system functions (they use World directly, correctly)
- Implementing perception logic (done in E14PERBEL-005)
- Adding confidence derivation (E15 scope)
- Modifying affordance query infrastructure (already uses `&dyn BeliefView`)
- Changing planning or search algorithms

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — all existing tests pass with `PerAgentBeliefView`
2. AI golden tests (if any) pass with belief-based view
3. Agent decision runtime correctly plans from beliefs, not world state
4. Grep for `OmniscientBeliefView` across workspace returns zero matches (excluding archived specs/docs)
5. Grep for `OmniscientBeliefRuntime` across workspace returns zero matches
6. `cargo clippy --workspace` — no warnings
7. Agent with empty belief store (newly created) can still query self-state and topology
8. Agent with populated belief store plans based on believed entity states

### Invariants

1. After this ticket, no code path in any crate constructs `OmniscientBeliefView` — it doesn't exist
2. All agent AI reasoning goes through `PerAgentBeliefView` → `BeliefView` trait
3. Action validation and system execution continue using `World` directly (correct — they validate against authoritative state)
4. Affordance queries continue working transparently via `&dyn BeliefView`
5. World/belief separation is enforced at compile time (no import path to omniscient view)
6. Phase 3 gate criterion: "OmniscientBeliefView fully replaced — no code path uses it"

## Test Plan

### New/Modified Tests

1. Update any test that constructed `OmniscientBeliefView` to use `PerAgentBeliefView` instead
2. Add test verifying agent with no beliefs about other entities produces valid (sparse) plans
3. Add test verifying agent with stale beliefs plans based on stale data (not auto-refreshed)

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace`
4. `cargo test --workspace`
