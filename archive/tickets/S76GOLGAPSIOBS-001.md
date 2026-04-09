# S76GOLGAPSIOBS-001: Golden S76-A and S76-B — remote travel and idle cap

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: None

## Problem

The simulation observer report revealed two behavioral pathologies with no golden test coverage: (1) agents failing to travel to remote resources when local supply is exhausted (Finding 2, 5), and (2) agents idling for 1000+ consecutive ticks when multiple needs are locally unsatisfiable (Finding 3 — Guard Theron 1019 idle ticks). Without regression guards, these emergent behavior chains can silently break.

## Assumption Reassessment (2026-04-09)

1. `seed_actor_local_beliefs` and `seed_belief_from_world` exist in `crates/worldwake-ai/tests/golden_harness/mod.rs` and are sufficient to seed only the planner-relevant remote beliefs for these scenarios. Safe correction: use those helpers instead of inventing new harness surface.
2. `golden_multi_hop_travel_plan()` in `crates/worldwake-ai/tests/golden_ai_decisions.rs` already covers a remote food journey, but not the specific observer gap of leaving a locally barren indoor start and proving the 200-tick no-stall bound. `golden_fallback_to_addressable_need_when_top_need_unsatisfiable()` still covers only partial unsatisfiability with local food available.
3. Live ownership should be a new file, `crates/worldwake-ai/tests/golden_simulation_gaps.rs`, because `golden_ai_decisions.rs` is already large and these scenarios form a coherent observer-gap cluster. Safe correction: keep the test-only boundary but move the owning file.
4. The prototype-world topology already provides the honest live affordances needed for both tests: `VillageSquare` is indoor and locally barren for food/water, `OrchardFarm` is the remote food destination, and remote resource beliefs can be seeded without a custom map.
5. The original S76-B prose overstated the live setup as “total local unsatisfiability except relieve_wilderness.” In the actual prototype world, the honest contract is narrower: remote food/water scarcity with lawful local self-care fallback (for example sleep). Safe correction: keep the idle-cap invariant but rewrite the scenario to match live affordances instead of forcing a stale wilderness-only narrative.
6. The original S76-A conservation prose assumed the final authoritative apple total stayed at 10. Live execution proved the deterministic end-state is one harvest plus one eat, so the final conservation checks are `verify_live_lot_conservation(..., 1)` and `verify_authoritative_conservation(..., 9)`. Safe correction: update the ticket to the live arithmetic instead of weakening the test.
7. Scenario IDs `126` and `127` were unused at implementation time, so the new metadata can land without inventory collisions. The golden inventory docs must be regenerated after adding those headers.

## Architecture Check

1. New test file `golden_simulation_gaps.rs` avoids further bloating `golden_ai_decisions.rs` (already 2386 lines). The file name clearly indicates these are observer-identified gap scenarios.
2. No backwards-compatibility shims. Tests only.

## Verification Layers

1. Agent travels to remote location within tick budget -> authoritative world state (agent's `CurrentPlace` reaches `OrchardFarm`)
2. Agent performs eat/drink at remote location -> action trace (`eat` or `drink` commit)
3. Max consecutive idle bounded -> action trace (no idle streak exceeds 100 ticks over 300-tick window)
4. Deterministic replay -> authoritative world state equality across two runs with same seed
5. Golden inventory/docs sync -> generated docs refresh via `scripts/golden_inventory.py`
6. Single-layer ticket (golden E2E tests only). No production code changes, so no additional layer mapping needed.

## What to Change

### 1. Create `golden_simulation_gaps.rs`

New file at `crates/worldwake-ai/tests/golden_simulation_gaps.rs`.

Add standard golden test imports (matching patterns from `golden_ai_decisions.rs`): `GoldenHarness`, `Seed`, `worldwake_core` types, `worldwake_sim` types, `golden_harness` module.

### 2. Implement S76-A scenario runner

Create `run_remote_travel_when_local_supply_exhausted(seed: Seed)` returning an observation struct:

- Use the live prototype world: `VillageSquare` as the barren indoor origin and `OrchardFarm` as the remote resource place.
- Spawn 1 AI agent at `VillageSquare` with explicit `PerceptionProfile` and `CognitiveProfile`; use hunger-heavy utility math so the remote food branch is stably selected.
- Seed local beliefs at the origin plus explicit beliefs about `OrchardFarm` and its apple source.
- Run for 300 ticks.
- Collect: whether the agent reached `OrchardFarm`, whether it committed `eat`/`drink`, the first tick it left `VillageSquare`, and stationary-origin tick count.

### 3. Implement S76-A test and replay companion

```rust
// Scenario 126: Remote Travel To Resource Under Local Scarcity
#[test]
fn golden_remote_travel_when_local_supply_exhausted() { ... }

#[test]
fn golden_remote_travel_when_local_supply_exhausted_replays_deterministically() { ... }
```

Use `Seed([176; 32])`. Assert the agent reaches `OrchardFarm`, commits `eat` or `drink` within 300 ticks, and leaves `VillageSquare` before a 200-tick local stall.

### 4. Implement S76-B scenario runner

Create `run_max_idle_under_remote_resource_scarcity(seed: Seed)` returning an observation struct:

- Use the live prototype world: `VillageSquare` as the start and `OrchardFarm` as the remote resource place.
- Spawn 1 AI agent with moderate hunger/thirst/fatigue/bladder plus explicit `PerceptionProfile` and `CognitiveProfile`.
- Seed beliefs about the remote apple source and a remote water lot at `OrchardFarm`.
- Run for 300 ticks.
- Collect: max consecutive idle tick count, committed actions, and whether the agent ever reached the remote resource place.

### 5. Implement S76-B test and replay companion

```rust
// Scenario 127: Idle Cap Under Remote Resource Scarcity
#[test]
fn golden_max_idle_under_remote_resource_scarcity() { ... }

#[test]
fn golden_max_idle_under_remote_resource_scarcity_replays_deterministically() { ... }
```

Use `Seed([177; 32])`. Assert `max_consecutive_idle < 100` over 300 ticks.

### 6. Conservation check for S76-A

S76-A involves a deterministic remote harvest-and-eat chain. After the simulation run, verify the live end-state arithmetic with `verify_live_lot_conservation(&world, Apple, 1)` and `verify_authoritative_conservation(&world, Apple, 9)`. S76-B has no ticket-owned commodity arithmetic assertions.

## Files to Touch

- `crates/worldwake-ai/tests/golden_simulation_gaps.rs` (new)

## Out of Scope

- Fixing the root cause of impoverished beliefs (S77, already completed)
- Fixing the planner — the planner is architecturally sound; sim failures are belief-driven
- Observer tooling enhancements (S78, already completed)
- Perception belief coverage (S76GOLGAPSIOBS-002)
- Utility profile diversity testing (S76GOLGAPSIOBS-003)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_remote_travel_when_local_supply_exhausted` passes and proves the remote-travel regression guard.
2. `golden_remote_travel_when_local_supply_exhausted_replays_deterministically` passes and proves deterministic replay for S76-A.
3. `golden_max_idle_under_remote_resource_scarcity` passes and proves `max_consecutive_idle < 100` over 300 ticks.
4. `golden_max_idle_under_remote_resource_scarcity_replays_deterministically` passes and proves deterministic replay for S76-B.
5. Golden inventory/docs regenerate cleanly after the new scenario metadata.
6. CI-matching `cargo clippy --workspace --all-targets -- -D warnings` passes.

### Invariants

1. No production code changes — engine behavior is unchanged
2. Deterministic replay: same seed produces identical observation structs
3. Apple conservation holds in S76-A at the deterministic final end-state: live lots = 1, authoritative total = 9

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_simulation_gaps.rs::golden_remote_travel_when_local_supply_exhausted` — regression guard for remote travel under local scarcity
2. `crates/worldwake-ai/tests/golden_simulation_gaps.rs::golden_remote_travel_when_local_supply_exhausted_replays_deterministically` — determinism guard
3. `crates/worldwake-ai/tests/golden_simulation_gaps.rs::golden_max_idle_under_remote_resource_scarcity` — regression guard for bounded idle under remote resource scarcity
4. `crates/worldwake-ai/tests/golden_simulation_gaps.rs::golden_max_idle_under_remote_resource_scarcity_replays_deterministically` — determinism guard

### Commands

1. `cargo test -p worldwake-ai golden_remote_travel_when_local_supply_exhausted`
2. `cargo test -p worldwake-ai golden_max_idle_under_remote_resource_scarcity`
3. `cargo test -p worldwake-ai`
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Added `crates/worldwake-ai/tests/golden_simulation_gaps.rs` with Scenario 126 and Scenario 127 plus deterministic replay companions.
- Kept the ticket test-only: no production code changed.
- Narrowed S76-B to the honest live contract of remote food/water scarcity with lawful local self-care fallback, because the prototype world does not match the original “only relieve_wilderness available” wording.
- Corrected S76-A conservation math to the deterministic final state reached by the landed scenario: one live apple lot and nine authoritative apples total after one remote harvest plus one eat.
- Regenerated the golden inventory/docs after landing the new scenario metadata.

## Verification Result

- Passed `cargo test -p worldwake-ai golden_remote_travel_when_local_supply_exhausted -- --nocapture`
- Passed `cargo test -p worldwake-ai golden_max_idle_under_remote_resource_scarcity -- --nocapture`
- Failed `cargo test -p worldwake-ai` due an existing unrelated failure in `crates/worldwake-ai/tests/golden_ai_decisions.rs::golden_blocked_intent_memory_with_ttl_expiry`
- Confirmed the unrelated broader failure by rerunning `cargo test -p worldwake-ai golden_blocked_intent_memory_with_ttl_expiry -- --nocapture`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
