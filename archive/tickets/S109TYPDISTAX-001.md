# S109TYPDISTAX-001: Rename BlockedIntentMemory to BlockerMemory

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — type rename across `worldwake-core`, `worldwake-ai`, `worldwake-cli` (no semantic change)
**Deps**: None

## Problem

S109 splits the overloaded `BlockedIntentMemory` into four purpose-specific memories. Before any semantic migration can begin, the existing `BlockedIntentMemory` must be renamed to `BlockerMemory` so the surviving world-state-blocker store has a name that matches its narrowed post-split meaning. This ticket performs that rename as a pure mechanical refactor — no variants added, no variants removed, no semantic change — so subsequent S109 tickets can build on a stable surface.

## Assumption Reassessment (2026-04-19)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `BlockedIntentMemory` is defined at `crates/worldwake-core/src/blocked_intent.rs:23` and registered in `crates/worldwake-core/src/component_schema.rs` at lines 633–656 under the `with_component_schema_entries!` macro. Re-exported from `crates/worldwake-core/src/lib.rs`. Existing focused unit tests live inside `blocked_intent.rs`'s own `#[cfg(test)]` block (lines 213–721 cover `BlockerKey`, `BlockedIntentMemory`, `BlockerClearingCondition`, `ClearingBaseline`, `BlockedIntent`, `BlockingFact`, `BlockerDiagnostic` bounds + behavior — `blocked_intent_types_satisfy_required_bounds`, `is_blocked_matches_only_live_entries_for_goal_key`, `source_depleted_does_not_block_goal_generation`, `expire_removes_entries_at_or_before_current_tick`, etc.).
2. S109 spec (`specs/S109-typed-discrepancy-taxonomy.md` D3) states that `BlockerMemory` must preserve the full `BlockedIntentMemory` API surface including `BlockerClearingCondition` (8 variants), `ClearingBaseline` (6 variants), `BlockerDiagnostic`, `blocks_goal_generation`, `is_blocked`, `is_blocked_for_search`, `find_blocked_for_search`, `record`, `expire`, `sweep_cleared`, `clear_for`, `clear_all_for_goal`. The rename changes the type name only; all supporting types and methods keep their names.
3. Shared abstraction boundary: the `BlockedIntentMemory` ECS component name and its component_schema-generated accessors (`insert_component_blocked_intent_memory`, `get_component_blocked_intent_memory`, `blocked_intent_memories`, `entities_with_blocked_intent_memory`, etc.). The boundary under audit is purely the component's name and the names of its schema-generated methods — the data contract is unchanged.
4. The drafted file list was narrower than the live rename surface. Additional direct consumers currently using the old names are `crates/worldwake-cli/src/handlers/inspect.rs`, `crates/worldwake-ai/src/{decision_trace.rs,goal_explanation.rs,goal_model.rs,search/mod.rs}`, `crates/worldwake-ai/tests/{golden_ai_decisions.rs,golden_harness/mod.rs}`, `crates/worldwake-systems/src/trade_actions.rs`, and the existing `ComponentKind`/`ComponentValue` coverage tests in `crates/worldwake-core/src/{delta.rs,component_tables.rs,world.rs,world_txn.rs}`. This remains current-ticket scope because all of these are downstream compile consumers of the shared renamed core types and schema-generated accessors.
13. No adjacent contradictions introduced by this rename. The `BlockedIntent` struct (singular) also renames to `Blocker` for symmetry, since `BlockerMemory` holds `Blocker` values rather than `BlockedIntent` values.

## Architecture Check

1. The rename is a prerequisite for S109's semantic split. Doing it as a standalone ticket keeps the subsequent migration ticket (T004) focused on emission routing rather than mixing rename churn with semantic change. Reviewing a mechanical rename in isolation is still cheaper than reviewing a conflated rename-plus-migration diff, even though the live compile surface is broader than the original 15-file draft.
2. No backwards-compatibility aliases introduced. Old `BlockedIntentMemory` symbol is removed, not wrapped. FND-28 compliant.

## Verification Layers

1. Workspace compiles after rename → `cargo build --workspace` passes. Single-layer check: pure-rename ticket has no cross-layer invariants; the compiler is the proof surface for type-name consistency.
2. No residual references to old names → `grep -rn "BlockedIntentMemory\|BlockedIntent\b\|blocked_intent_memor" crates/ scenarios/` returns 0 matches after the rename. Single-layer.
3. All existing `blocked_intent.rs` `#[cfg(test)]` tests pass with the renamed types → `cargo test -p worldwake-core blocker_memory` (after rename). Single-layer check — test names are updated together with type names.

## What to Change

### 1. Rename core types and file

In `crates/worldwake-core/`:

- Move `src/blocked_intent.rs` → `src/blocker_memory.rs`.
- Inside the new file: rename `BlockedIntentMemory` → `BlockerMemory` and `BlockedIntent` → `Blocker`. All other types (`BlockerKey`, `BlockerDiagnostic`, `BlockerClearingCondition`, `ClearingBaseline`, `BlockingFact`) keep their names. Update the `BlockedIntentMemory::record`/`expire`/`sweep_cleared`/`clear_for`/`clear_all_for_goal`/`is_blocked`/`is_blocked_for_search`/`find_blocked_for_search` impl block headers and the `impl BlockedIntent { ... blocks_goal_generation ... }` block headers accordingly.
- Update `pub mod blocked_intent;` in `src/lib.rs` to `pub mod blocker_memory;` and update the re-export list to name `BlockerMemory`/`Blocker` instead of `BlockedIntentMemory`/`BlockedIntent`. Re-exports of `BlockerKey`, `BlockingFact`, etc., are unchanged.

### 2. Update component_schema registration

In `crates/worldwake-core/src/component_schema.rs` lines 632–656, rename the `with_component_schema_entries!` block:

- `blocked_intent_memories` → `blocker_memories`
- `BlockedIntentMemory` → `BlockerMemory` (all positions inside that block)
- `insert_blocked_intent_memory` → `insert_blocker_memory`
- `get_blocked_intent_memory` / `get_blocked_intent_memory_mut` → `get_blocker_memory` / `get_blocker_memory_mut`
- `remove_blocked_intent_memory` → `remove_blocker_memory`
- `has_blocked_intent_memory` → `has_blocker_memory`
- `iter_blocked_intent_memories` → `iter_blocker_memories`
- `insert_component_blocked_intent_memory` → `insert_component_blocker_memory`
- `get_component_blocked_intent_memory` / `get_component_blocked_intent_memory_mut` → `get_component_blocker_memory` / `get_component_blocker_memory_mut`
- `remove_component_blocked_intent_memory` → `remove_component_blocker_memory`
- `has_component_blocked_intent_memory` → `has_component_blocker_memory`
- `entities_with_blocked_intent_memory` → `entities_with_blocker_memory`
- `query_blocked_intent_memory` → `query_blocker_memory`
- `count_with_blocked_intent_memory` → `count_with_blocker_memory`
- String literal `"BlockedIntentMemory"` → `"BlockerMemory"`
- `set_component_blocked_intent_memory` / `clear_component_blocked_intent_memory` → `set_component_blocker_memory` / `clear_component_blocker_memory`

### 3. Update macro expansion site imports

Per `tickets/README.md` check #13, `with_component_schema_entries!` generates code using bare type names at each expansion site. Update the imports and `ComponentKind` arms at:

- `crates/worldwake-core/src/world.rs` — replace `BlockedIntentMemory` with `BlockerMemory` in the import list and any direct test references.
- `crates/worldwake-core/src/delta.rs` — replace `BlockedIntentMemory` with `BlockerMemory` in the import list and enum arms.
- `crates/worldwake-core/src/component_tables.rs` — replace `BlockedIntentMemory` with `BlockerMemory` in the import list and enum arms.
- `crates/worldwake-core/src/world_txn.rs` line 1993 — update the test import of `BlockedIntentMemory` to `BlockerMemory`.
- `crates/worldwake-core/src/test_utils.rs` — rename any `sample_blocked_intent`/`sample_blocked_intent_memory` helpers to `sample_blocker`/`sample_blocker_memory`.

### 4. Rename consumers in `worldwake-ai`

Rename in the 12 consumer files (direct access sites):

- `crates/worldwake-ai/src/failure_handling.rs`
- `crates/worldwake-ai/src/agent_tick/mod.rs`, `agent_tick/frame.rs`, `agent_tick/active_action.rs`, `agent_tick/candidates.rs`, `agent_tick/execution.rs`, `agent_tick/observation.rs`, `agent_tick/planning.rs`, `agent_tick/tests.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/search/candidates.rs`
- `crates/worldwake-ai/src/planning_snapshot.rs`
- `crates/worldwake-ai/src/feasibility.rs`

In each file: replace `BlockedIntentMemory` with `BlockerMemory`, `BlockedIntent` with `Blocker`, `blocked_memory`/`blocked_intent_memory` local-variable aliases with `blocker_memory`, and `get_component_blocked_intent_memory`/`insert_component_blocked_intent_memory` calls with the renamed accessors from Section 2.

### 4b. Update remaining downstream consumers

Rename the same symbols in the additional compile consumers discovered during reassessment:

- `crates/worldwake-ai/src/decision_trace.rs`
- `crates/worldwake-ai/src/goal_explanation.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/search/mod.rs`
- `crates/worldwake-ai/tests/golden_ai_decisions.rs`
- `crates/worldwake-ai/tests/golden_harness/mod.rs`
- `crates/worldwake-systems/src/trade_actions.rs`
- `crates/worldwake-cli/src/handlers/inspect.rs`

These files stay purely mechanical in this ticket: import names, parameter types, helper names, schema accessor calls, and user-facing inspect labels move from `BlockedIntent*` to `Blocker*` with no semantic change.

### 5. Update test-site references

In the existing `#[cfg(test)]` blocks at:

- `crates/worldwake-ai/src/candidate_generation.rs` (boundary line 5200; sites at 7997–8011, 15412, 16170)
- `crates/worldwake-ai/src/failure_handling.rs` (boundary line 1014; sites throughout the test module)
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-ai/src/search/tests.rs`
- the new `blocker_memory.rs` inline tests

Rename all `BlockedIntentMemory`/`BlockedIntent` references. Do not change what the tests assert — only the type/variable names.

## Files to Touch

- `crates/worldwake-core/src/blocked_intent.rs` (move → `blocker_memory.rs`, rename types inside)
- `crates/worldwake-core/src/blocker_memory.rs` (new, from rename)
- `crates/worldwake-core/src/component_schema.rs` (modify — macro names)
- `crates/worldwake-core/src/world.rs` (modify — import + test refs)
- `crates/worldwake-core/src/world_txn.rs` (modify — import in test module)
- `crates/worldwake-core/src/delta.rs` (modify — import + enum arms)
- `crates/worldwake-core/src/component_tables.rs` (modify — import + enum arms)
- `crates/worldwake-core/src/lib.rs` (modify — module + re-exports)
- `crates/worldwake-core/src/test_utils.rs` (modify — helper names)
- `crates/worldwake-core/src/violation.rs` (modify — doc reference rename)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/search/candidates.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify)
- `crates/worldwake-ai/src/feasibility.rs` (modify)
- `crates/worldwake-ai/src/goal_explanation.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-systems/src/trade_actions.rs` (modify)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify)

## Out of Scope

- No semantic change to `BlockingFact` (no variants added or removed — `Unknown`, `AssumptionFailed`, `NoBuyer`, `PatienceExhausted`, `SourceDepleted`, etc. all remain as they are).
- No changes to clearing-condition semantics or `is_blocker_cleared` logic.
- No new memory types (DiscrepancyMemory, RepairMemory, LearnedOpportunityMemory land in T002).
- No changes to `CognitiveProfile` TTL fields.
- No scenario RON changes (scenarios don't reference `BlockedIntentMemory` directly; only `CognitiveProfile::unknown_block_ticks` — unchanged by this ticket).
- No `SAVE_FORMAT_VERSION` bump (handled by T006 as part of the variant-removal cleanup).

## Acceptance Criteria

### Tests That Must Pass

1. All existing `blocked_intent.rs` unit tests pass under the new `blocker_memory.rs` module with renamed types: `cargo test -p worldwake-core blocker_memory`.
2. All existing `worldwake-ai` tests pass unchanged in assertion semantics: `cargo test -p worldwake-ai`.
3. Existing full suite: `cargo test --workspace`.

### Invariants

1. Workspace builds clean: `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
2. Zero residual references to the old names: `grep -rn "BlockedIntentMemory\|blocked_intent_memor\|BlockedIntent\b" crates/ scenarios/` returns empty.
3. `component_schema!`-generated accessors are reachable under new names from all consumer crates (verified by compilation).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/blocker_memory.rs` (moved from `blocked_intent.rs`) — existing unit tests renamed to match new type names (mechanical: `BlockedIntentMemory::default()` → `BlockerMemory::default()`, etc.). No new assertions.

### Commands

1. `cargo test -p worldwake-core blocker_memory`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

## Outcome

Completed on 2026-04-19.

- Renamed the authoritative blocker component module from `blocked_intent.rs` to `blocker_memory.rs`, renamed `BlockedIntentMemory` to `BlockerMemory`, and renamed `BlockedIntent` to `Blocker` without changing blocker semantics.
- Renamed the `component_schema!` registration block and all generated accessor call sites to the new `*_blocker_memory` names across `worldwake-core`, `worldwake-ai`, `worldwake-cli`, and the compile-fallout consumer in `worldwake-systems/src/trade_actions.rs`.
- Updated helper names, inspect output, test names, and remaining documentation references so the old type names no longer appear in live code under `crates/` or `scenarios/`.

## Deviations

- Reassessment and compile fallout showed the ticket's original file list was incomplete. The landed rename also touched `crates/worldwake-ai/src/{decision_trace.rs,goal_explanation.rs,goal_model.rs,search/mod.rs}`, `crates/worldwake-ai/tests/{golden_ai_decisions.rs,golden_harness/mod.rs}`, `crates/worldwake-systems/src/trade_actions.rs`, `crates/worldwake-cli/src/handlers/inspect.rs`, and `crates/worldwake-core/src/violation.rs`.
- Verification included an early `cargo test --workspace --no-run` compile-fallout sweep before the focused and broadened ticket commands so the shared rename surface could be enumerated honestly.

## Verification Result

- Passed `rg -n "BlockedIntentMemory|\bBlockedIntent\b|blocked_intent_memor|pub mod blocked_intent|pub use blocked_intent::|blocked_intent::" crates/worldwake-core crates/worldwake-ai crates/worldwake-cli scenarios`
- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core blocker_memory`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
