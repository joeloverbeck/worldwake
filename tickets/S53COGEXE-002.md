# S53COGEXE-002: Migrate all ReasoningProfile consumers to split profiles

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — 13 AI crate files updated to read CognitiveProfile + ExecutionBudget instead of ReasoningProfile
**Deps**: S53COGEXE-001

## Problem

CognitiveProfile and ExecutionBudget types exist (from 001) but no consumer reads them. All 13 AI crate files still read ReasoningProfile. This ticket migrates every consumer to the split profiles so ReasoningProfile can be safely removed in ticket 003.

## Assumption Reassessment (2026-04-05)

1. ReasoningProfile consumed in 13 worldwake-ai files — confirmed by grep:
   - `goal_model.rs`, `failure_handling.rs`, `agent_tick/planning.rs`, `agent_tick/frame.rs`, `search/mod.rs`, `agent_tick/mod.rs`, `agent_tick/active_action.rs`, `search/transition.rs`, `search/tests.rs`, `search/heuristic.rs`, `lib.rs`, `decision_runtime.rs`, `agent_tick/tests.rs`
2. Each consumer reads specific ReasoningProfile fields. After migration, each reads from the appropriate split type:
   - Cognitive fields (`max_candidates_to_plan`, `max_plan_depth`, `switch_margin`, `*_block_ticks`, `*_cooldown_ticks`) → CognitiveProfile
   - Engine fields (`max_node_expansions`, `beam_width`, `snapshot_travel_horizon`, `max_prerequisite_locations`) → ExecutionBudget
3. CLI display at `crates/worldwake-cli/src/display.rs` may reference ReasoningProfile for `format_goal_kind` or inspect output — needs updating.
4. CLI handlers (`inspect.rs`, etc.) may display ReasoningProfile — needs updating.

## Architecture Check

1. Pure consumer migration — each file changes its import and field access from `ReasoningProfile` to the appropriate split type. No algorithmic changes. The same values are read, just from different components.
2. After this ticket, ReasoningProfile is a dead type with zero readers — ticket 003 can safely remove it.
3. No backward-compatibility shims.

## Verification Layers

1. All AI files read CognitiveProfile for cognitive fields → compilation success + grep confirms no remaining ReasoningProfile field reads
2. All AI files read ExecutionBudget for engine fields → compilation success
3. Behavioral equivalence: all golden tests pass with split profiles carrying same values → `cargo test -p worldwake-ai`
4. CLI inspect/display handles both new profiles → CLI smoke test
5. Cross-layer: AI (worldwake-ai) reads split profiles from world state (worldwake-core). Verified by golden test pass.

## What to Change

### 1. Migrate search module

In `crates/worldwake-ai/src/search/mod.rs`, `search/transition.rs`, `search/heuristic.rs`, `search/tests.rs`:
- Replace `ReasoningProfile` imports with `CognitiveProfile` and `ExecutionBudget`
- `max_node_expansions`, `beam_width`, `snapshot_travel_horizon`, `max_prerequisite_locations` → read from ExecutionBudget
- `max_plan_depth` → read from CognitiveProfile
- Update function signatures that take `&ReasoningProfile` to take both `&CognitiveProfile` and `&ExecutionBudget`

### 2. Migrate agent_tick module

In `crates/worldwake-ai/src/agent_tick/mod.rs`, `agent_tick/planning.rs`, `agent_tick/frame.rs`, `agent_tick/active_action.rs`, `agent_tick/tests.rs`:
- Replace imports
- `max_candidates_to_plan`, `switch_margin`, `*_block_ticks`, `*_cooldown_ticks` → read from CognitiveProfile
- Engine fields → read from ExecutionBudget

### 3. Migrate goal_model, failure_handling, decision_runtime

In `crates/worldwake-ai/src/goal_model.rs`, `failure_handling.rs`, `decision_runtime.rs`:
- Replace imports and field accesses per classification table

### 4. Migrate lib.rs

In `crates/worldwake-ai/src/lib.rs`:
- Update re-exports and trait bounds that reference ReasoningProfile

### 5. Migrate CLI display and handlers

In `crates/worldwake-cli/src/display.rs` and relevant handlers:
- Update inspect output to display CognitiveProfile and ExecutionBudget separately
- Decision trace labels should mark parameters as cognitive or engine for debuggability (P29)

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/transition.rs` (modify)
- `crates/worldwake-ai/src/search/heuristic.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify)
- `crates/worldwake-cli/src/display.rs` (modify)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify — if it displays ReasoningProfile)

## Out of Scope

- Removing ReasoningProfile type/registration — ticket 003
- Save format migration — ticket 003
- Behavioral validation conformance test — ticket 004
- Changing any planner algorithm or parameters

## Acceptance Criteria

### Tests That Must Pass

1. Zero remaining reads of ReasoningProfile fields in worldwake-ai (grep confirms)
2. All AI unit tests pass with split profiles
3. All golden tests pass — behavioral equivalence with same parameter values
4. CLI inspect shows CognitiveProfile and ExecutionBudget separately
5. Existing suite: `cargo test --workspace`

### Invariants

1. Behavioral equivalence: split profiles with default values produce identical behavior to ReasoningProfile::default()
2. All golden tests pass unchanged — no goal selection divergence from the migration alone
3. Decision traces label cognitive vs engine parameters (P29)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — Updated to use split profiles
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — Updated to use split profiles

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-cli`
3. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
