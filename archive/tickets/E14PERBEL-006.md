# E14PERBEL-006: Replace OmniscientBeliefView with PerAgentBeliefView and Delete OmniscientBeliefView

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — workspace-wide belief view migration for actor-specific affordance/planning reads, deletion of omniscient view
**Deps**: E14PERBEL-004 (PerAgentBeliefView must be fully implemented), E14PERBEL-005 (perception system must populate belief stores)

## Problem

`OmniscientBeliefView` is still live across non-archived code even after `PerAgentBeliefView` and perception stores landed. It remains the default read-model in:

- AI planning / agent tick code
- sim-side affordance resolution and some tests
- CLI affordance display tests and handlers
- systems tests and a few actor-specific production helpers such as trade evaluation

This violates the E14 spec and Phase 3 gate: `OmniscientBeliefView` must be fully removed, not merely bypassed in `agent_tick.rs`. After migration, all actor-specific planning/affordance reads must go through `PerAgentBeliefView`, and `OmniscientBeliefView` / `OmniscientBeliefRuntime` must be deleted entirely.

## Assumption Reassessment (2026-03-14)

1. `runtime_belief_view()` in [crates/worldwake-ai/src/agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs) still constructs `OmniscientBeliefView::with_runtime()` and remains the main AI runtime factory.
2. `agent_tick.rs` is only part of the remaining footprint. Non-archived code still references `OmniscientBeliefView` in `worldwake-ai`, `worldwake-sim`, `worldwake-cli`, `worldwake-systems`, and several tests.
3. `TickInputContext` in [crates/worldwake-sim/src/tick_input_producer.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_input_producer.rs) does **not** mention either belief-view type. The prior assumption was false.
4. Core agent creation already attaches `AgentBeliefStore` and `PerceptionProfile` in [crates/worldwake-core/src/world.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs#L145), so this ticket does not need to add those components to the canonical creation path.
5. `OmniscientBeliefView` and `OmniscientBeliefRuntime` still live in [crates/worldwake-sim/src/omniscient_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/omniscient_belief_view.rs).
6. Affordance queries already use `&dyn BeliefView`, so the call-site migration is primarily about constructing the correct concrete view for the acting agent.
7. `archive/tickets/completed/E14PERBEL-004.md` was correct: the current `BeliefView` trait still mixes subjective planning reads with authoritative executor-style helpers. This ticket removes the omniscient adapter anyway because the spec requires that, but it does **not** finish the larger boundary split. That follow-up remains `E14PERBEL-009`.

## Architecture Check

1. The right end state is still deletion, not aliasing. Keeping an omniscient `BeliefView` implementation alive would continue to normalize the wrong abstraction and violate Principle 26.
2. Migration is broader than originally described. It must update every non-archived actor-specific read path that still constructs `OmniscientBeliefView`, not just AI planner code.
3. `PerAgentBeliefView` is an honest interim adapter, not the ideal final architecture. It is acceptable here because it already makes the subjective/authoritative split explicit at the implementation boundary instead of pretending omniscience is valid.
4. This ticket must not “solve” the mixed-boundary problem by adding a renamed omniscient wrapper or a compatibility shim. The clean long-term architecture is to split planner-facing subjective knowledge from authoritative executor helpers in `E14PERBEL-009`.

## What to Change

### 1. Update AI runtime view construction in `agent_tick.rs`

Change the factory function to construct `PerAgentBeliefView` instead of `OmniscientBeliefView`:

```rust
fn runtime_belief_view<'a>(
    agent: EntityId,
    world: &'a World,
    scheduler: &'a Scheduler,
    action_defs: &'a ActionDefRegistry,
) -> PerAgentBeliefView<'a> {
    let belief_store = world
        .get_component_agent_belief_store(agent)
        .expect("agents must have AgentBeliefStore before agent tick");
    PerAgentBeliefView::with_runtime(
        agent,
        world,
        belief_store,
        PerAgentBeliefRuntime::new(scheduler.active_actions(), action_defs),
    )
}
```

### 2. Update all `agent_tick.rs` call sites

Replace all `OmniscientBeliefView` and `runtime_belief_view()` usage in `agent_tick.rs` with `PerAgentBeliefView`, including read-phase helpers and tests in the same file.

### 3. Update remaining non-archived production call sites

Migrate all remaining non-archived production usages that are actor-specific and currently construct `OmniscientBeliefView`, including at minimum:

- `crates/worldwake-ai/src/lib.rs`
- `crates/worldwake-ai/src/search.rs`
- `crates/worldwake-sim/src/tick_step.rs`
- `crates/worldwake-cli/src/handlers/actions.rs`
- `crates/worldwake-systems/src/trade_actions.rs`

If another live production file still imports `OmniscientBeliefView`, it is in scope for this ticket as well.

### 4. Update live tests and debug/CLI affordance call sites

Update non-archived tests and CLI/debug handlers that still construct `OmniscientBeliefView` / `OmniscientBeliefRuntime` to use `PerAgentBeliefView` / `PerAgentBeliefRuntime`.

### 5. Delete `omniscient_belief_view.rs`

Remove the entire file `crates/worldwake-sim/src/omniscient_belief_view.rs`.

### 6. Update `crates/worldwake-sim/src/lib.rs`

Remove `pub mod omniscient_belief_view;` and all re-exports of `OmniscientBeliefView` and `OmniscientBeliefRuntime`.

### 7. Remove all remaining references

Grep across workspace for `OmniscientBeliefView` and `OmniscientBeliefRuntime` and remove/update any remaining imports or references. This may include:
- `crates/worldwake-ai/src/lib.rs`
- `crates/worldwake-cli/tests/integration.rs`
- `crates/worldwake-ai/tests/golden_combat.rs`
- `crates/worldwake-sim/src/affordance_query.rs`
- systems test modules that currently use the omniscient adapter only to query affordances for a specific acting agent

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify — migrate runtime and test call sites)
- `crates/worldwake-ai/src/search.rs` (modify — migrate tests)
- `crates/worldwake-ai/src/lib.rs` (modify — remove omniscient export expectations)
- `crates/worldwake-sim/src/omniscient_belief_view.rs` (delete)
- `crates/worldwake-sim/src/lib.rs` (modify — remove module + re-exports)
- `crates/worldwake-sim/src/tick_step.rs` (modify — actor-specific affordance resolution)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — tests)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify — CLI affordance display)
- `crates/worldwake-cli/src/handlers/control.rs` (modify — tests)
- `crates/worldwake-cli/tests/integration.rs` (modify — tests)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify — tests)
- `crates/worldwake-systems/src/trade_actions.rs` (modify — participant-specific read-models)
- systems test files that construct `OmniscientBeliefView` for affordance checks

## Out of Scope

- Redesigning the mixed subjective/authoritative `BeliefView` boundary itself (tracked by `E14PERBEL-009`)
- Reworking every authoritative system helper that currently happens to consume `&dyn BeliefView`; this ticket should migrate the omniscient adapter away, not redesign those APIs wholesale
- Implementing perception logic (done in E14PERBEL-005)
- Adding confidence derivation (E15 scope)
- Changing planning or search algorithms
- Adding a renamed omniscient wrapper or compatibility alias

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --workspace` — all existing tests pass with `PerAgentBeliefView`
2. AI and CLI golden/integration tests that previously used `OmniscientBeliefView` pass with `PerAgentBeliefView`
3. Agent decision runtime correctly plans from beliefs, not world state
4. Grep for `OmniscientBeliefView` across workspace returns zero matches (excluding archived specs/docs)
5. Grep for `OmniscientBeliefRuntime` across workspace returns zero matches
6. `cargo clippy --workspace` — no warnings
7. Agent with empty belief store (newly created) can still query self-state and topology
8. Agent with populated belief store plans or enumerates affordances based on believed entity states

### Invariants

1. After this ticket, no non-archived code path in any crate constructs `OmniscientBeliefView` — it doesn't exist
2. All agent AI reasoning goes through `PerAgentBeliefView` → `BeliefView` trait
3. Authoritative world mutation and action execution still use `World` / `WorldTxn` directly; this ticket only changes belief/read-model construction
4. Affordance queries continue working through `&dyn BeliefView` without an omniscient implementation
5. World/belief separation is enforced at compile time by deleting the omniscient adapter, not by adding a replacement alias
6. Phase 3 gate criterion: "OmniscientBeliefView fully replaced — no code path uses it"
7. Any remaining authoritative fallbacks exposed through `PerAgentBeliefView` stay explicit and temporary; this ticket must not normalize them as the final architecture

## Test Plan

### New/Modified Tests

1. Update tests that constructed `OmniscientBeliefView` / `OmniscientBeliefRuntime` to use `PerAgentBeliefView` / `PerAgentBeliefRuntime`
2. Add or strengthen at least one test proving actor-specific affordance or planning reads still work for an agent with an empty belief store (self/topology queries still available)
3. Add or strengthen at least one test proving a populated belief store drives non-self reads rather than silently refreshing from authoritative world state

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim`
3. `cargo test -p worldwake-cli`
4. `cargo test -p worldwake-systems`
5. `cargo clippy --workspace`
6. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Deleted `OmniscientBeliefView` / `OmniscientBeliefRuntime` from non-archived code and removed their exports from `worldwake-sim`.
  - Migrated AI, sim, CLI, systems, and live tests to `PerAgentBeliefView` / `PerAgentBeliefRuntime`.
  - Added convenience constructors to `PerAgentBeliefView` so actor-specific call sites can build the correct subjective runtime view directly from `World`.
  - Fixed a real production precondition bug uncovered by the migration: `TargetExists` now means existence rather than "alive", so `TargetExists` and `TargetDead` are no longer contradictory.
  - Fixed subjective corpse-goal behavior in production AI by reading believed corpse inventory instead of hidden possession structure, and by making loot/bury planner goals satisfiable under planning-state simulation.
  - Fixed subjective combat intent modeling by separating explicit hostility targets from merely visible hostiles.
  - Fixed heal-goal planning so satisfaction no longer depends on private target thresholds that subjective views should not read.
  - Strengthened CLI scenario agent setup so spawned agents always carry the physiology/self-care components needed by the exposed actions.
  - Added test-harness belief refresh helpers for AI goldens so they can exercise planner logic without reviving omniscient production reads.
- Deviations from original plan:
  - The ticket started as a straight adapter replacement, but the migration exposed several real production defects and planning assumptions that had to be corrected to preserve the intended subjective-belief architecture.
  - The AI golden harness now seeds scenario knowledge explicitly in tests. That is an intentional test-only stopgap, not the final production architecture; follow-up tickets `E14PERBEL-010` and `E14PERBEL-011` track the remaining architectural work around combat/perception behavior and passive local observation.
- Verification results:
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-cli`
  - `cargo test -p worldwake-systems`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
