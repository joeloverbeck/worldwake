# S82WASDISINV-008: Golden E2E test for waste disposal cycle

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: `crates/worldwake-ai/src/feasibility.rs`
**Deps**: S82WASDISINV-005, S82WASDISINV-007

## Problem

No golden test exercised the complete carried-waste disposal cycle as a first-class AI goal: an agent starts capacity-strained by carried waste, emits and selects `FreeCarryCapacity`, executes `drop_item`, and leaves the same lot on the ground at the local place with the strain relieved. Without this test, regressions in the disposal goal pipeline would go undetected.

## Assumption Reassessment (2026-04-10)

1. Golden E2E tests live in `crates/worldwake-ai/tests/`. They use `PerceptionProfile` on agents that need to observe post-production output (per CLAUDE.md: "Golden production tests require PerceptionProfile").
2. The original "produce waste first" scenario shape is not a lawful autonomous golden root on the current branch. `emit_produce_goals()` only emits `ProduceCommodity` for recipe outputs that serve self-consume or merchant restock, so an AI agent has no live reason to autonomously choose a pure waste-producing recipe.
3. The stronger revised "disposal unblocks local bread production" shape was also too broad for the live planner contract. Capacity-constrained pickup/craft plans can already synthesize `put_down` through normal transport planning, so that chain does not isolate `FreeCarryCapacity` as the gating root. The strongest honest golden is therefore a pure disposal-cycle scenario that proves the dedicated disposal goal and its exact-full-threshold boundary case directly.
4. Focused proof exposed a production contradiction in the AI pipeline: `GoalDispatchDeclaration::FreeCarryCapacity` already routed through `FeasibilityStrategy::AlwaysLikely`, but `goal_specific_feasibility()` did not match `GoalKind::FreeCarryCapacity`, so the first real golden hit an `unreachable!()` panic. This ticket therefore required a small production fix in `crates/worldwake-ai/src/feasibility.rs`, not just test coverage.
5. `drop_item` preserves lot identity and clears possession, but an item lot is not guaranteed to have an owner. The stable world-state proof is same lot `EntityId` on the local ground with unchanged ownership state, not necessarily `owner == actor`.
6. Existing golden ownership still fits this ticket better than a new file: `crates/worldwake-ai/tests/golden_production.rs` already owns transport/conservation golden helpers and deterministic replay patterns.

## Architecture Check

1. Standard golden E2E test pattern: set up scenario with agents, run simulation for N ticks, assert the earlier AI boundary with decision traces, the disposal lifecycle with action traces, and the durable consequences with authoritative world state.
2. Production fallout fix required: extend `AlwaysLikely` feasibility matching to include `GoalKind::FreeCarryCapacity` so the live goal-dispatch contract does not panic during ranking/planning.
3. No backward-compatibility shims.

## Verification Layers

1. `FreeCarryCapacity` goal generated and selected while carried waste strains capacity -> decision trace
2. `drop_item` starts and commits -> action trace
3. Same waste lot remains in the world at the agent's place after drop -> authoritative world state
4. Agent's carried waste quantity drops to zero and the disposal goal stops being selected after the drop resolves the strain -> authoritative world state + later decision traces
5. Conservation invariants hold for waste throughout the scenario -> conservation helpers
6. Multi-layer ticket: decision trace + action lifecycle + authoritative outcome + focused feasibility unit coverage

## What to Change

### 1. Golden test scenario

Create a scenario with:
- One agent carrying enough `Waste` to exceed the disposal threshold and nearly fill carry capacity
- No competing urgent needs that would let another goal family dominate the opening decision
- `DisposalProfile` with default threshold (800 permille)
- Local belief seeding or equivalent lawful setup so the disposal and downstream production opportunities are visible without scenario-distorting timing assumptions

### 2. Golden test assertions

In `crates/worldwake-ai/tests/`:

1. Run simulation from an initial state where carried waste already strains capacity
2. Assert `FreeCarryCapacity` appears in the decision trace before the disposal action
3. Assert `drop_item` appears in the action trace
4. Assert the same waste lot is on the ground at the agent's place after commit
5. Assert the agent's carried waste quantity decreases to zero
6. Assert later planning traces no longer select `FreeCarryCapacity` once disposal resolves the strain
7. Assert conservation helpers pass for waste throughout the scenario

### 3. Falsification variant

Add a boundary variant: an agent with `capacity_strain_threshold: Permille(1000)` still disposes when carried waste exactly fills carry capacity.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify)

## Out of Scope

- Waste decay or cleanup systems
- Multi-agent waste observation scenarios
- Performance benchmarks

## Acceptance Criteria

### Tests That Must Pass

1. Capacity-strained carried waste causes `FreeCarryCapacity` to appear in decision traces
2. Agent drops the carried waste via `drop_item`
3. The dropped waste lot exists on the ground at the agent's location after commit
4. The agent's carried waste quantity is reduced after dropping
5. Later traces stop selecting `FreeCarryCapacity` after the disposal commit resolves the strain
6. `verify_conservation` passes throughout the scenario
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Item identity preserved across drop (same waste lot `EntityId`)
2. Ownership state preserved across drop if present; no test assumes ownership where none exists
3. Item counts conserved (no waste created/destroyed outside lawful action effects)
4. Agent plans from beliefs only, never authoritative state (P14)
5. `cargo clippy --workspace --all-targets -- -D warnings` passes

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs` — golden E2E: carried waste strain → `FreeCarryCapacity` → `drop_item` → same lot on ground → goal no longer selected
2. `crates/worldwake-ai/tests/golden_production.rs` — boundary variant: full-capacity threshold (`1000‰`) still triggers disposal when exactly full
3. `crates/worldwake-ai/src/feasibility.rs` — focused unit coverage for `AlwaysLikely` routing and `FreeCarryCapacity`

### Commands

1. `cargo test -p worldwake-ai golden_waste_disposal_cycle`
2. `cargo test -p worldwake-ai golden_waste_disposal_exact_full_threshold_cycle`
3. `cargo test -p worldwake-ai always_likely_strategy_covers_free_carry_capacity -- --nocapture`
4. `cargo test -p worldwake-ai declaration_routing_assigns_expected_feasibility_strategies -- --nocapture`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completion date**: 2026-04-10
- **What changed**: Added disposal-cycle goldens in `crates/worldwake-ai/tests/golden_production.rs` covering the default threshold and the exact-full `1000‰` boundary case, plus deterministic replay companions. The landed proof surface demonstrates `FreeCarryCapacity` generation/selection, `drop_item` commit, same-lot persistence on the local ground, carried-waste removal, and post-commit goal de-selection once strain is relieved.
- **Deviations from original plan**: Reassessment narrowed the scenario twice. First, the proposed "produce waste first" root was not a lawful autonomous branch on the live planner. Second, the broader "disposal unblocks local bread production" chain was not uniquely attributable to `FreeCarryCapacity` because generic transport planning could already free capacity via `put_down`. Focused proof also exposed an in-scope production contradiction: `FreeCarryCapacity` used `FeasibilityStrategy::AlwaysLikely`, but `goal_specific_feasibility()` did not match that goal and panicked until `crates/worldwake-ai/src/feasibility.rs` was corrected.
- **Verification results**:
  - `cargo test -p worldwake-ai golden_waste_disposal_cycle -- --nocapture`
  - `cargo test -p worldwake-ai golden_waste_disposal_exact_full_threshold_cycle -- --nocapture`
  - `cargo test -p worldwake-ai always_likely_strategy_covers_free_carry_capacity -- --nocapture`
  - `cargo test -p worldwake-ai declaration_routing_assigns_expected_feasibility_strategies -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
