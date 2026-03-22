**Status**: PENDING

# S20: AI Pipeline Structural Cleanup

## Summary
Split `agent_tick.rs` (~6124 lines) and `search.rs` (~5880 lines) in `worldwake-ai` into modular sub-components with typed stage boundaries. This is pure refactoring — all golden tests must pass unchanged with zero behavioral modification.

## Phase
Phase 3+: AI Architecture Overhaul (Step 13.5, Wave 0)

## Crate
`worldwake-ai`

## Dependencies
None. This is groundwork for S21–S28.

## FOUNDATIONS Alignment
- **P27** (Debuggability): Clearer module boundaries improve causal inspection of the decision pipeline.
- No behavioral changes — all Principle compliance is inherited from existing code.

## Motivation
Both files contain well-structured pipeline code, but their size makes navigation, review, and modification difficult. Every subsequent epic in the AI Architecture Overhaul (S21–S28) touches these files. Splitting them first provides a cleaner surface for all later work.

## Scope

### agent_tick.rs Split
Extract the `process_agent` pipeline into explicit stages, each as a sub-module under `agent_tick/`:

| Module | Contents | Approximate Lines |
|--------|----------|-------------------|
| `agent_tick/observation.rs` | `refresh_runtime_for_read_phase()`, `observation_snapshot_changed()`, snapshot comparison helpers, `InFlightReconciliation` | ~400 |
| `agent_tick/candidates.rs` | `ReadPhaseResult`, candidate generation orchestration, facility queue expiry | ~200 |
| `agent_tick/active_action.rs` | `handle_active_action_phase()`, interrupt evaluation integration | ~300 |
| `agent_tick/planning.rs` | `plan_and_validate_next_step()`, `plan_and_validate_next_step_traced()`, plan selection orchestration | ~400 |
| `agent_tick/execution.rs` | `enqueue_valid_step_or_handle_failure()`, input enqueueing, step start logic | ~300 |
| `agent_tick/journey.rs` | `update_journey_fields_for_adopted_plan()`, `handle_recoverable_travel_step_blockage()`, journey lifecycle helpers | ~400 |
| `agent_tick/mod.rs` | `AgentTickDriver`, `AgentTickContext`, `AutonomousController` impl, top-level `produce_inputs()` dispatching to stages | ~500 |

All types remain `pub(crate)` or narrower. No public API changes.

### search.rs Split
Extract search internals into sub-modules under `search/`:

| Module | Contents | Approximate Lines |
|--------|----------|-------------------|
| `search/frontier.rs` | `FrontierEntry`, `BinaryHeap` management, beam-width truncation, `compare_search_nodes()` | ~300 |
| `search/heuristic.rs` | `compute_heuristic()`, A* heuristic helpers, Dijkstra integration | ~200 |
| `search/transition.rs` | `build_successor()`, `build_successor_detailed()`, hypothetical transition delegation to `planner_ops` | ~400 |
| `search/candidates.rs` | `search_candidates()`, `relevant_action_defs()`, binding rejection, travel pruning, facility exclusivity filtering | ~500 |
| `search/mod.rs` | `search_plan()` entry point, `SearchNode`, `PlanSearchResult`, orchestration across sub-modules | ~400 |

### Test Organization
- Tests in `agent_tick.rs` move to `agent_tick/tests.rs` (or individual `tests` submodules per stage)
- Tests in `search.rs` move to `search/tests.rs` (or per sub-module)
- All test function names, assertions, and behavior remain identical

## Constraints
1. **Zero behavioral change**: Every golden test in `crates/worldwake-ai/tests/golden_*.rs` must pass without modification.
2. **No public API changes**: All type visibility remains unchanged. External consumers (golden tests, CLI) see the same imports.
3. **No new dependencies**: This is intra-crate reorganization only.
4. **Incremental**: agent_tick.rs and search.rs can be split independently. Each split is a separate commit.

## Tickets

### S20-001: Split agent_tick.rs into staged sub-modules
- Convert `agent_tick.rs` into `agent_tick/mod.rs` + sub-modules as specified above
- Move all functions to their designated modules
- Re-export necessary types from `mod.rs`
- Verify: `cargo test -p worldwake-ai` passes unchanged

### S20-002: Split search.rs into policy sub-modules
- Convert `search.rs` into `search/mod.rs` + sub-modules as specified above
- Move all functions to their designated modules
- Re-export necessary types from `mod.rs`
- Verify: `cargo test -p worldwake-ai` passes unchanged

### S20-003: Organize test modules
- Move inline tests from both files into dedicated test sub-modules
- Preserve all test names and assertions
- Verify: `cargo test -p worldwake-ai` — exact same test count, all pass

### S20-004: Workspace verification
- `cargo test --workspace` — all tests pass
- `cargo clippy --workspace` — no new warnings
- Golden test count unchanged (verify via `cargo test -p worldwake-ai --test golden_* -- --list | wc -l`)

## Verification
1. `cargo test --workspace` — all pass
2. `cargo clippy --workspace` — no new warnings
3. `cargo test -p worldwake-ai --test golden_* -- --list | wc -l` — count matches pre-split count
4. `git diff --stat` confirms only file moves/renames within `crates/worldwake-ai/src/`, no changes to other crates
