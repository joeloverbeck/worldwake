# E22INTSOATES-001: Integration test file scaffold + T24 (Player Agent Replacement)

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E22 spec (`specs/E22-integration-soak-tests.md`)

## Problem

No integration test file exists for E22's cross-system scenario verification. T24 (Player Agent Replacement) is the simplest scenario — it verifies `ControlSource` swap mid-simulation with world continuity — and serves as the initial proof that the harness supports integration-length tests.

## Assumption Reassessment (2026-03-31)

1. `GoldenHarness` exists in `crates/worldwake-ai/tests/golden_harness/mod.rs` — confirmed. Imports `ActionTraceSink`, `save_to_bytes`, `load_from_bytes`, `step_tick`, `ControllerState`, `DeterministicRng`, etc.
2. `ControlSource` enum (`Human | Ai | None`) defined in `crates/worldwake-core/src/control.rs` — confirmed.
3. `WorldTxn` supports component mutations including `ControlSource` — confirmed via `crates/worldwake-core/src/world_txn.rs`.
4. `get_affordances()` exists in `worldwake-sim` — confirmed via `affordance_query.rs`.
5. `hash_world()` and `hash_event_log()` exist in `crates/worldwake-core/src/canonical.rs` — confirmed.
6. `h.driver.enable_tracing()` enables `DecisionTraceSink` on `AgentTickDriver` — confirmed.
7. `h.enable_action_tracing()` enables `ActionTraceSink` on harness — confirmed.
8. `golden_integration.rs` does not yet exist — confirmed; this ticket creates it.
9. `ControllerState` tracks which entity is human-controlled — confirmed in `crates/worldwake-sim/src/controller_state.rs`.
10. Existing golden test pattern: `fn run_<scenario>(seed: Seed) -> (StateHash, StateHash)` with two `#[test]` functions calling with different seeds — confirmed across all golden files.
11. No `golden_integration` test binary exists yet — this is a new test file that follows the `mod golden_harness;` pattern.
12. T24 isolates ControlSource swap from other competing behaviors by using a minimal world with a mid-travel agent and an agent with an active plan. No political, crime, or bandit systems are exercised.
13. No adjacent contradictions found.
14. No mismatches.
15. T24 tick budget is ≤ 100 ticks. Agent A mid-travel + Agent B with active goal. After swap at tick N, Agent A must generate AI candidates within 5 ticks.

## Architecture Check

1. Creating a new test file `golden_integration.rs` follows the established pattern of one golden file per domain/epic. Shared helpers stay in `golden_harness/mod.rs`; scenario-specific topology builders live in the test file itself.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. ControlSource component change → authoritative world state (read component before/after swap)
2. Agent A AI activation → decision trace (non-empty candidate list within 5 ticks)
3. Agent B affordance legality → `get_affordances()` result against current position/inventory
4. World continuity → `Scheduler.current_tick()` monotonic increase
5. State preservation → component equality checks (inventory, wounds, needs, placement)
6. Determinism → state hash comparison across 2 seeds

## What to Change

### 1. Create `crates/worldwake-ai/tests/golden_integration.rs`

New test file with:
- `mod golden_harness;` import
- Shared topology builder for T24 (minimal 2-place world)
- `fn run_t24_player_replacement(seed: Seed) -> (StateHash, StateHash)`:
  - Build world with Agent A (`ControlSource::Human`, carrying Apple, mid-travel) and Agent B (`ControlSource::Ai`, at different place, with active goal)
  - Run for N ticks to establish mid-simulation state
  - At tick N: swap via `WorldTxn` — Agent A to `ControlSource::Ai`, Agent B to `ControlSource::Human`, update `ControllerState`
  - Verify: only `ControlSource` components changed (snapshot world hash minus control components)
  - Run for 5+ more ticks
  - Verify: Agent A generates AI goals (decision trace shows non-empty candidates)
  - Verify: Agent B's affordances are legal for B's position/inventory/beliefs
  - Verify: Agent A's inventory, wounds, needs, placement all preserved
  - Verify: `Scheduler.current_tick()` strictly increases
  - Return `(hash_world, hash_event_log)` at final tick
- Two `#[test]` functions: `t24_player_replacement_seed_1()`, `t24_player_replacement_seed_2()`
- State hash comparison for determinism

### 2. Add shared integration test helpers (if needed)

Any topology builders or helper functions needed by multiple E22 scenarios should be added as module-level functions in `golden_integration.rs`. If patterns emerge that are reusable beyond E22, they can be promoted to `golden_harness/mod.rs` in later tickets.

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (new)

## Out of Scope

- Other E22 scenarios (T20, T21, T22, T27, T28, T29, T30, T31, T32, T33)
- Changes to `golden_harness/mod.rs` (unless a genuinely missing utility is discovered during implementation)
- Any engine or system code changes

## Acceptance Criteria

### Tests That Must Pass

1. `t24_player_replacement_seed_1` — Agent A transitions to AI control, generates candidates within 5 ticks
2. `t24_player_replacement_seed_2` — same scenario with different seed, determinism hash comparison
3. Agent A's inventory, wounds, `HomeostaticNeeds`, and placement preserved identically after swap
4. Agent B's affordance set returns only actions legal for B's current position, inventory, and beliefs
5. No `InputKind::RequestAction` processed for Agent B after swap to Human (no AI inputs)
6. `Scheduler.current_tick()` continues monotonically
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `ControlSource` swap does not modify any component other than `ControlSource` on the two swapped agents
2. World simulation continues without reset after swap — tick counter monotonically increases
3. Agent symmetry (Principle 19): same legal action set regardless of control source

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs::t24_player_replacement_seed_1` — proves ControlSource swap with world continuity
2. `crates/worldwake-ai/tests/golden_integration.rs::t24_player_replacement_seed_2` — determinism verification

### Commands

1. `cargo test -p worldwake-ai --test golden_integration -- t24`
2. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-31
- **What changed**: Created `crates/worldwake-ai/tests/golden_integration.rs` with T24 scenario (2 tests). Custom 2-place topology (Alpha ↔ Beta, 3-tick travel). Agent A starts Human with Apple and submitted travel action, Agent B starts Ai with hunger. Swap via `WorldTxn` + `ControllerState` at mid-travel tick. Verifies: state preservation (inventory, wounds, needs, placement), AI activation (decision trace non-empty candidates), affordance legality for swapped Human agent, monotonic tick advancement, determinism via hash comparison. Also added tick-alignment documentation to CLAUDE.md for both decision and action trace sections.
- **Deviations**: None. All ticket deliverables implemented as specified.
- **Verification**: `cargo test -p worldwake-ai --test golden_integration -- t24` (2/2 pass), `cargo test -p worldwake-ai` (all pass, 0 failures), `cargo clippy --test golden_integration -p worldwake-ai` (clean).
