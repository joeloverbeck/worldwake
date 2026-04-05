# S53COGEXE-005: Reclassify behavior-changing ExecutionBudget fields as cognitive

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — split-profile field reclassification across core types, AI consumers, scenario/save surfaces, and conformance proofs
**Deps**: S53COGEXE-004

## Problem

`S53COGEXE-004` proved that two fields still carried in `ExecutionBudget` are behavior-changing under unchanged `CognitiveProfile`: `max_node_expansions` and `snapshot_travel_horizon`. That violates the S53 split contract and Principle 12, because those fields are not merely compressing planner computation; they change which goal or plan the agent selects. They must be reclassified onto `CognitiveProfile`.

## Assumption Reassessment (2026-04-05)

1. `crates/worldwake-core/src/execution_budget.rs` still defines `ExecutionBudget { max_node_expansions, beam_width, snapshot_travel_horizon, max_prerequisite_locations }`.
2. `crates/worldwake-core/src/cognitive_profile.rs` still omits `max_node_expansions` and `snapshot_travel_horizon`, even though both now have focused evidence of behavior change.
3. `crates/worldwake-ai/tests/conformance_execution_budget.rs` now proves:
   - `max_node_expansions` changes plan selection on the S97-style bounded multi-step scenario
   - `snapshot_travel_horizon = 2` suppresses one-hop remote-acquire goal selection from the same planning boundary
   - `beam_width` and `max_prerequisite_locations` remain representative-scenario safe at the owned proof surface
4. The exact shared contract under audit is the split-profile classification boundary between `worldwake-core` profile state and the `worldwake-ai` planner/search consumers that read those fields.
5. After `S53COGEXE-003`, live saves persist only `CognitiveProfile` plus `ExecutionBudget`, so reclassifying fields again is an authoritative persisted-shape change and requires a new `SAVE_FORMAT_VERSION` bump plus migration.
6. Scenario and CLI setup surfaces still expose `cognitive_profile` and `execution_budget` separately. Reclassification must move the two fields across those user-facing definitions too; otherwise scenario-authored behavior becomes misleading.
7. This is not a planner algorithm ticket. The owned change is field ownership, persistence/schema fallout, consumer updates, and proof-surface updates.
8. Mismatch + correction: S53 spec prose still classifies `max_node_expansions` and `snapshot_travel_horizon` as engine knobs. This ticket exists because the live conformance proof falsified that classification.

## Architecture Check

1. Reclassifying the two behavior-changing fields is cleaner than weakening the conformance contract. The repo now has direct evidence that these values shape agent identity, so the type split must reflect that instead of preserving a false engine/cognitive boundary.
2. `ExecutionBudget` should keep only fields that survived the conformance boundary at the strongest honest owned layer (`beam_width`, `max_prerequisite_locations`), while `CognitiveProfile` absorbs the behavior-defining fields. No shim layer should preserve the old field locations beyond the explicit save migration.

## Verification Layers

1. `CognitiveProfile` / `ExecutionBudget` field ownership matches the proved behavior boundary -> core type tests + grep over consumer reads
2. AI consumers read the reclassified fields from `CognitiveProfile` instead of `ExecutionBudget` -> focused AI compile/test coverage + conformance tests
3. Save migration preserves the moved field values across the authoritative persisted boundary -> focused save/load migration test
4. Scenario/CLI surfaces author the fields in their new home -> focused CLI scenario/persistence tests
5. Reclassified conformance remains truthful -> updated `conformance_execution_budget.rs` proving only the remaining engine fields are safe and the moved fields are no longer treated as engine knobs

## What to Change

### 1. Move fields between the split profile types

In `crates/worldwake-core/src/cognitive_profile.rs` and `crates/worldwake-core/src/execution_budget.rs`:
- add `max_node_expansions` and `snapshot_travel_horizon` to `CognitiveProfile`
- remove those fields from `ExecutionBudget`
- update defaults, roundtrip tests, and registration-focused tests accordingly

### 2. Migrate AI consumers to the corrected field owners

In `crates/worldwake-ai/src/`:
- move reads of `max_node_expansions` and `snapshot_travel_horizon` from `ExecutionBudget` to `CognitiveProfile`
- keep `beam_width` and `max_prerequisite_locations` on `ExecutionBudget`
- update any focused test fixtures or helper constructors that still build the old field split

### 3. Update scenario, CLI, and save boundaries

In `crates/worldwake-cli/src/scenario/`, `crates/worldwake-cli/src/handlers/`, and `crates/worldwake-sim/src/save_load.rs`:
- move the two fields to the `cognitive_profile` scenario shape
- remove them from the `execution_budget` scenario shape
- bump `SAVE_FORMAT_VERSION`
- add migration logic for the prior split-only save version so persisted agents retain the same values after reclassification

### 4. Update the conformance and regression proofs

In `crates/worldwake-ai/tests/conformance_execution_budget.rs` and any directly affected focused tests:
- keep the explicit evidence that justified the reclassification
- update the conformance framing so `ExecutionBudget` now validates only the remaining engine fields

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify)
- `crates/worldwake-core/src/execution_budget.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify if re-export ordering or tests need it)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/heuristic.rs` (modify)
- `crates/worldwake-ai/src/search/transition.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify as needed)
- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (modify)
- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify)
- `crates/worldwake-cli/src/handlers/persistence.rs` (modify if fixture coverage needs it)
- `crates/worldwake-sim/src/save_load.rs` (modify)
- `scenarios/cli-evaluation.ron` (modify if it still authors these fields)

## Out of Scope

- Planner algorithm changes
- Tuning default values for the remaining engine fields
- Re-running or refactoring the entire golden suite into a universal reusable scenario API

## Acceptance Criteria

### Tests That Must Pass

1. `CognitiveProfile` owns `max_node_expansions` and `snapshot_travel_horizon`; `ExecutionBudget` no longer does
2. `ExecutionBudget` conformance coverage now only validates the remaining engine fields
3. Save migration preserves reclassified field values across the version bump
4. Existing suite: `cargo test --workspace`

### Invariants

1. Behavior-changing profile fields do not remain in `ExecutionBudget`
2. No backwards-compatibility alias path leaves the moved fields readable from both profile types after migration
3. Scenario-authored and saved agent behavior stays equivalent after the reclassification

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/cognitive_profile.rs` — updated defaults/roundtrip coverage for the expanded cognitive carrier
2. `crates/worldwake-core/src/execution_budget.rs` — updated defaults/roundtrip coverage for the reduced engine carrier
3. `crates/worldwake-ai/tests/conformance_execution_budget.rs` — updated to validate only the remaining engine fields while preserving the evidence for the moved fields
4. `crates/worldwake-sim/src/save_load.rs` — migration coverage for the post-004 field move

### Commands

1. `cargo test -p worldwake-ai --test conformance_execution_budget -- --nocapture`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
