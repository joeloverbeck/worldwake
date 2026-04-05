# S53COGEXE-002: Migrate all ReasoningProfile consumers to split profiles

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI runtime/search consumers migrated to CognitiveProfile + ExecutionBudget, plus test/golden setup and CLI inspect fallout moved onto the split profiles
**Deps**: S53COGEXE-001

## Problem

CognitiveProfile and ExecutionBudget types exist (from 001) but no consumer reads them. All 13 AI crate files still read ReasoningProfile. This ticket migrates every consumer to the split profiles so ReasoningProfile can be safely removed in ticket 003.

## Assumption Reassessment (2026-04-05)

1. ReasoningProfile still drives the live AI in the expected production files — confirmed by grep:
   - `goal_model.rs`, `failure_handling.rs`, `agent_tick/planning.rs`, `agent_tick/frame.rs`, `search/mod.rs`, `agent_tick/mod.rs`, `agent_tick/active_action.rs`, `search/transition.rs`, `search/heuristic.rs`, `decision_runtime.rs`
2. Additional setup fallout exists beyond those production readers: `agent_tick/tests.rs`, `search/tests.rs`, and `tests/golden_harness/mod.rs` still author behavior through `ReasoningProfile`, so this ticket must migrate those setup paths onto the split carriers to preserve behavioral equivalence after the consumer rewrite.
3. Each consumer reads specific ReasoningProfile fields. After migration, each reads from the appropriate split type:
   - Cognitive fields (`max_candidates_to_plan`, `max_plan_depth`, `switch_margin`, `*_block_ticks`, `*_cooldown_ticks`) → CognitiveProfile
   - Engine fields (`max_node_expansions`, `beam_width`, `snapshot_travel_horizon`, `max_prerequisite_locations`) → ExecutionBudget
4. `worldwake-ai/src/lib.rs` still re-exports `ReasoningProfile`; downstream imports that rely on the AI crate surface must move to `CognitiveProfile` / `ExecutionBudget`.
5. CLI inspect at `crates/worldwake-cli/src/handlers/inspect.rs` currently displays none of the reasoning-profile components. The acceptance criterion is therefore additive: explicit separate `CognitiveProfile` / `ExecutionBudget` output must be added rather than "updating" an old `ReasoningProfile` display.

### Reassessment Note (2026-04-05)

- ticket says: this is a 13-file AI consumer rewrite plus possible CLI display fallout
- live code has: production AI readers, plus test and golden harness setup that still writes only `ReasoningProfile`, and CLI inspect does not display any reasoning profiles yet
- correction applied: expanded the owned migration surface to include test/golden setup migration and explicit CLI inspect output for the split profiles
- why safe: these are direct consequences of moving behavior onto the new authoritative carriers already landed in `S53COGEXE-001`

## Architecture Check

1. Pure consumer migration in production AI code — each live reader changes from `ReasoningProfile` to the appropriate split type. No planner algorithm changes.
2. Setup paths that intentionally express behavioral variation must move to the split carriers in the same ticket. Leaving tests or golden harnesses to author only `ReasoningProfile` would silently change behavior once the migrated AI stops reading it.
3. After this ticket, `ReasoningProfile` remains present only as transitional authoritative state and persistence input for ticket 003. AI consumers and their setup surfaces should no longer depend on it for live behavior.
4. No backward-compatibility shims.

## Verification Layers

1. All live AI readers use `CognitiveProfile` for cognitive fields → compilation success + grep confirms no remaining `ReasoningProfile` field reads in production AI paths
2. All live AI readers use `ExecutionBudget` for engine fields → compilation success
3. Focused AI tests and golden harness setup write split profiles directly where they are proving behavior changes → targeted AI test pass
4. Behavioral equivalence: golden tests still pass with equivalent split values → `cargo test -p worldwake-ai`
5. CLI inspect shows `CognitiveProfile` and `ExecutionBudget` separately
6. Cross-layer: AI reads split profiles from authoritative world state. Verified by focused AI tests plus golden pass.

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
- Update re-exports that still expose `ReasoningProfile` from the AI crate

### 5. Migrate AI tests and golden harness setup

In `crates/worldwake-ai/src/search/tests.rs`, `crates/worldwake-ai/src/agent_tick/tests.rs`, and `crates/worldwake-ai/tests/golden_harness/mod.rs`:
- Replace setup that expresses behavioral variation through `ReasoningProfile`
- Add or use split-profile helpers so custom cognitive vs engine values remain explicit and equivalent after the production migration

### 6. Migrate CLI inspect output

In `crates/worldwake-cli/src/handlers/inspect.rs`:
- Add explicit `CognitiveProfile` and `ExecutionBudget` display sections

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
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- relevant `crates/worldwake-ai/tests/golden_*.rs` files that currently author behavior through `ReasoningProfile` (modify as needed)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify)

## Out of Scope

- Removing ReasoningProfile type/registration — ticket 003
- Save format migration — ticket 003
- Behavioral validation conformance test — ticket 004
- Changing any planner algorithm or parameters

## Acceptance Criteria

### Tests That Must Pass

1. Zero remaining `ReasoningProfile` field reads in live worldwake-ai production code
2. AI unit tests and migrated harness setup pass with split profiles
3. All golden tests pass — behavioral equivalence with same parameter values
4. CLI inspect shows CognitiveProfile and ExecutionBudget separately
5. Existing suite: `cargo test --workspace`

### Invariants

1. Behavioral equivalence: equivalent split-profile values produce identical behavior to the old combined profile
2. All golden tests pass unchanged — no goal selection divergence from the migration alone
3. Test and golden setup no longer rely on `ReasoningProfile` as the live behavior carrier

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — Updated to use split profiles
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — Updated to use split profiles
3. `crates/worldwake-ai/tests/golden_harness/mod.rs` and affected `golden_*.rs` files — updated split-profile setup where custom reasoning behavior is authored
4. `crates/worldwake-cli/src/handlers/inspect.rs` — inspect output coverage for the new profiles

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-cli`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completed**: 2026-04-05
- **What changed**:
  - Migrated the live `worldwake-ai` planning/search/runtime consumers from `ReasoningProfile` to `CognitiveProfile` plus `ExecutionBudget`, including [`search/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/mod.rs), [`search/heuristic.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/heuristic.rs), [`search/transition.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/transition.rs), [`agent_tick/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/mod.rs), [`agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs), [`agent_tick/active_action.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/active_action.rs), [`decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs), [`failure_handling.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs), and [`goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs).
  - Migrated split-profile setup surfaces used to author behavior in tests and goldens, including [`agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs), [`search/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs), [`golden_harness/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs), [`golden_supply_chain.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs), [`golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs), [`golden_reasoning_diversity.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_reasoning_diversity.rs), and [`golden_care.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs).
  - Added explicit split-profile inspect output in [`inspect.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/inspect.rs).
  - Absorbed bounded migration fallout in [`agent_tick/execution.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/execution.rs) and narrow test-wrapper/clippy cleanup needed to keep the staged contract compiling and lint-clean.
- **Deviations from original plan**:
  - Ticket `002` remained a live-consumer migration, not a full `ReasoningProfile` eradication. Temporary test compatibility wrappers, public re-exports, and CLI setup/persistence surfaces still exist and remain owned by `S53COGEXE-003`.
  - [`worldwake-ai/src/lib.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/lib.rs) still re-exports `ReasoningProfile`; that cleanup stays with the removal ticket rather than being silently absorbed here.
- **Verification**:
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-cli`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
