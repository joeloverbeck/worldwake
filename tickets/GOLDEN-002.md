# GOLDEN-002: Belief Isolation — Unseen Theft Forces Replan (Scenario 10)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only ticket; all perception/belief system code exists (E14)
**Deps**: None (E14PERBEL series completed; perception pipeline, belief store, passive observation all landed)

## Problem

No golden test currently proves belief isolation — the core E14 behavioral contract that agents cannot react to events they did not witness. This is the highest-value missing golden scenario (score 7) because it validates the entire perception → belief → planning chain that distinguishes this architecture from omniscient AI.

The scenario proves: Agent A plans based on stale belief about a distant resource. Agent B (already at the resource) consumes it unseen by A. When A arrives, passive observation updates A's beliefs, forcing a replan. The key assertion is that A does *not* react before arrival.

## Assumption Reassessment (2026-03-14)

1. `AgentBeliefStore` exists in `worldwake-core/src/belief.rs:12` with `enforce_capacity()` and retention window logic.
2. `PerceptionProfile` exists in `worldwake-core/src/belief.rs:161` with `memory_capacity`, `memory_retention_ticks`, `observation_fidelity`.
3. `PerAgentBeliefView` is used in `worldwake-ai/src/agent_tick.rs` and `search.rs` — agents plan from beliefs, not world state.
4. Passive local observation (`observe_passive_local_entities`) exists in `worldwake-systems/src/perception.rs` — fires when an agent is at a place and updates beliefs about co-located entities.
5. `golden_perception.rs` does not yet exist — this will be a new test file.
6. The `golden_harness` module provides `seed_agent`, `give_commodity`, `GoldenHarness`, `step_once`, place constants (`VILLAGE_SQUARE`, `ORCHARD_FARM`), and `pm()` helper.
7. Belief seeding requires setting `AgentBeliefStore` component on the agent via `WorldTxn`. The harness may need a small helper for this (see Architecture Check).

## Architecture Check

1. New test file `golden_perception.rs` follows the established pattern: `mod golden_harness; use golden_harness::*;` at top.
2. Belief seeding may require a new harness helper (`seed_belief_about_resource`) if the existing `give_commodity` / `place_ground_commodity` helpers don't cover belief-only (non-authoritative) state. This helper would call `txn.set_component_agent_belief_store()` to plant a specific believed entity state. If the harness already handles this via perception pipeline (agents auto-observe co-located resources), then the setup can use spatial separation + initial observation tick instead.
3. No backwards-compatibility shims. The scenario is additive.

## What to Change

### 1. Create `golden_perception.rs` test file

New file with `mod golden_harness;` and standard imports from `worldwake_core` and `worldwake_systems`.

### 2. Add `run_belief_isolation_scenario()` scenario runner

Setup:
- Agent A (Alice) at `VILLAGE_SQUARE`, critically hungry (pm(900)), fast metabolism (pm(15)/tick).
- Agent B (Bob) at `ORCHARD_FARM`, critically hungry (pm(950)), fast metabolism (pm(15)/tick).
- `ORCHARD_FARM` has apple resource source with small quantity (`Quantity(3)`).
- Alice seeded with belief that apples exist at `ORCHARD_FARM` (via `AgentBeliefStore` component or via an initial observation tick at `ORCHARD_FARM` before relocating her to `VILLAGE_SQUARE`).
- Bob seeded with belief about apples at `ORCHARD_FARM` (he's already there — passive observation covers this).
- Both agents have `PerceptionProfile::default()` (fidelity pm(875), retention 48 ticks).

Emergent behavior to prove (within 100-tick window):
1. Bob harvests and eats apples at `ORCHARD_FARM` (he's local; Alice cannot witness).
2. Alice plans `AcquireCommodity(SelfConsume)` targeting `ORCHARD_FARM` based on stale belief.
3. Alice travels toward `ORCHARD_FARM`.
4. While Alice is in transit, her belief about `ORCHARD_FARM` apples does NOT change (isolation).
5. Alice arrives at `ORCHARD_FARM`; passive observation fires and updates her beliefs.
6. Alice replans (blocked intent or different goal — she cannot harvest what Bob already consumed).

### 3. Add assertions

- **Belief isolation**: Track Alice's believed apple quantity at `ORCHARD_FARM`. It must NOT decrease before Alice arrives at `ORCHARD_FARM`. It must update (decrease or become zero) only after Alice is co-located.
- **Bob consumes first**: Bob's hunger decreases before Alice arrives (proving Bob ate the apples).
- **Alice replans**: After arrival, Alice either generates a different goal or records a blocked intent for the depleted source.
- **Conservation**: `total_live_lot_quantity(Apple)` is consistent every tick.
- **Deterministic replay**: Two runs with same seed produce identical hashes.

### 4. Add test functions

- `golden_belief_isolation_unseen_theft_forces_replan` — main scenario test.
- `golden_belief_isolation_unseen_theft_replays_deterministically` — deterministic replay companion.

### 5. Update `reports/golden-e2e-coverage-analysis.md`

- Move Scenario 10 from "Part 3: Missing Scenarios" to "Part 1: Proven Emergent Scenarios" with full writeup.
- Update Part 2 cross-system interaction coverage: mark "Stale belief → travel to depleted source → passive re-observation → replan" as **Yes**.
- Update Part 4 summary statistics: proven tests count, cross-system chains count.
- Remove Scenario 10 from "Pending Backlog Summary" and "Recommended Implementation Order".

## Files to Touch

- `crates/worldwake-ai/tests/golden_perception.rs` (new — ~100-120 lines)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — possibly add belief-seeding helper if needed, ~10-15 lines)
- `reports/golden-e2e-coverage-analysis.md` (modify — move scenario, update stats)

## Out of Scope

- No changes to `worldwake-core`, `worldwake-sim`, `worldwake-systems`, or `worldwake-ai/src/` production code.
- No changes to `PerceptionProfile` defaults or `AgentBeliefStore` logic.
- No changes to other golden test files (`golden_care.rs`, `golden_combat.rs`, etc.).
- Do not implement Scenario 11 (memory decay) in this ticket — that is GOLDEN-003.
- Do not add multi-hop belief propagation (gossip/report) scenarios — out of scope for this golden test.
- Do not modify the perception pipeline or observation system.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_belief_isolation_unseen_theft_forces_replan` — Alice's belief about apples at OrchardFarm does not update until she physically arrives; she replans after passive observation reveals depletion.
2. `golden_belief_isolation_unseen_theft_replays_deterministically` — two runs with same seed produce identical world and event-log hashes.
3. Existing suite: `cargo test --workspace` passes (no regressions).

### Invariants

1. **Belief isolation**: Alice's `AgentBeliefStore` for the apple source at `ORCHARD_FARM` does not change while Alice is NOT at `ORCHARD_FARM`.
2. **No omniscient leakage**: Alice does not react to Bob's consumption before arriving at the same place.
3. **Conservation**: Apple lot totals are consistent every tick.
4. **No manual action queueing**: All behavior is emergent through `AgentTickDriver` + `AutonomousControllerRuntime`.
5. **Deterministic replay**: Identical seeds produce identical hashes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_perception.rs::golden_belief_isolation_unseen_theft_forces_replan` — proves E14 belief isolation contract.
2. `crates/worldwake-ai/tests/golden_perception.rs::golden_belief_isolation_unseen_theft_replays_deterministically` — proves determinism.

### Commands

1. `cargo test -p worldwake-ai --test golden_perception`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
