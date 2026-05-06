# S135PLAPERBUD-001: Foundation types, PerceptionProfile extension, AgentBeliefStore omission log

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — core types, PerceptionProfile schema, AgentBeliefStore diff/save shape, save-format version, scenario CLI deserialization
**Deps**: spec `archive/specs/S135-planner-perception-budget.md`

## Problem

Per S135's design, the planner snapshot's hidden per-place cap (`max_snapshot_entities_per_place=50`) silently truncates accumulated belief entities, violating FND-12 (performance compressing causality) and FND-7 (locality auditability). To fix this, S135 introduces (a) two new fields on `PerceptionProfile` declaring per-agent salience policy and omission-log capacity, (b) typed records of every observation perception drops, and (c) a runtime-only ring-buffered omission log owned by each agent's `AgentBeliefStore`. This ticket lands the foundation: the new types and the single canonical belief-store/diff/save path. No behavior change yet — the cap-removal and perception write happen in ticket 003.

## Assumption Reassessment (2026-05-05)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PerceptionProfile` struct lives at `crates/worldwake-core/src/belief.rs:2554` with 11 fields; derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize` plus `#[serde(deny_unknown_fields)]`; Default impl at line 2586. Construction sites: 40 across crates (validated via `rg '^\s*PerceptionProfile\s*\{$' crates/`); none use spread syntax. Existing focused tests touching these construction sites: `crates/worldwake-systems/src/perception.rs` cfg-test block at line 1407 (`agent_observes_place_without_scene_evidence`:2163, `active_action_does_not_cross_place_boundaries_or_self_observe`:2684, ~25 other inline tests at lines 1688–2683); `crates/worldwake-ai/src/agent_tick/tests.rs` (line 200 mock construction). Existing trace/integration coverage: `crates/worldwake-ai/tests/golden_perception_exposure.rs::golden_observation_budget_prioritizes_agents_and_facilities_over_waste` (line 606).
2. `BeliefStoreDiff` is a struct at `crates/worldwake-core/src/belief.rs:1120` with paired `*_set`/`*_removed` fields per sub-store. `SAVE_FORMAT_VERSION` lives at `crates/worldwake-sim/src/save_load.rs:6` and is currently 66; this ticket bumps it to 67. Existing save/load round-trip coverage: `crates/worldwake-sim/src/save_load.rs` cfg-test block.
3. Shared abstraction boundary under audit: `AgentBeliefStore` as the canonical owner for agent-local belief/perception residue plus the `BeliefStoreDiff` delta-compaction contract. The original draft's separate `ObservationOmissionLog` component would have created a second lawful transport path for the same omission fact, conflicting with FND-12/FND-27. The selected boundary is one canonical field on `AgentBeliefStore` with paired diff fields.
4. `World::create_agent()` already seeds `AgentBeliefStore::new()`. The new `ObservationOmissionLog` default is seeded through that store, not through a separate universal Agent component. The `world_txn.rs` create-agent delta assertion remains on `AgentBeliefStore` and does not gain a new component entry.
5. Scenario authoring of `perception_profile:` exists in 10+ committed scenarios (`scenarios/*.ron`); each authors named fields explicitly. Adding the two new fields with `#[serde(default)]` keeps existing scenarios deserializing unchanged. `scripts/profile_docs.py` regenerates `docs/profiles/all-profiles.md` from the `PerceptionProfile` source; the live script supports `--write` only, so freshness is verified by regenerating to `/tmp` and comparing with `docs/profiles/all-profiles.md`.

## Architecture Check

1. New types live in `worldwake-core` because `AgentBeliefStore` and `BeliefStoreDiff` are core-owned. `OmissionReason` derives `Copy` so it can later become a `Discrepancy::Omission` payload (ticket 004) without breaking `Discrepancy`'s `Copy` derive.
2. `ObservationOmissionLog` is classified as **runtime-only / scenario-exempt** state owned by `AgentBeliefStore` (analogous to `social_observations`). No `AgentDef` field, no `*Def` wrapper, no `spawn_agent()` set call. The log starts empty through `AgentBeliefStore::new()` and is mutated only by ticket 003's perception write.
3. `SaliencePolicy` opens with a single variant `PriorityWithNeedBoost` matching S105's actual `compute_observation_priority` behavior (composition of priority class with `need_salience_boost` at `crates/worldwake-systems/src/perception.rs:714`). The enum reserves room for future genuinely-different policies (e.g., `OcclusionAware`) without forcing a redesign.
4. No backward-compatibility shim. The two new `PerceptionProfile` fields are required (not `Option`), but `#[serde(default)]` covers RON deserialization and the explicit construction sites are updated atomically in this ticket.

## Verification Layers

1. New types compile and round-trip serde → focused unit test in `belief.rs` cfg-test block.
2. `AgentBeliefStore::new()` seeds `ObservationOmissionLog::default()` → focused unit test asserting the default store contains an empty log.
3. `BeliefStoreDiff` carries omission-log additions/removals through the same compact ring-buffer pattern as `social_observations` → focused unit test.
4. `SAVE_FORMAT_VERSION` bump preserves existing save round-trip → existing save-load round-trip unit test extended to cover version bump and a non-empty `ObservationOmissionLog` payload inside `AgentBeliefStore`.
5. `BeliefStoreDiff` paired field additions deserialize from older diffs (with the new fields absent) via `#[serde(default)]` → focused unit test.

## What to Change

### 1. Add new types to `worldwake-core`

Add `SaliencePolicy`, `ObservationOmission`, `ObservationOmissionLog`, `OmissionReason` enums/structs to `crates/worldwake-core/src/belief.rs` (or a new `crates/worldwake-core/src/observation_omission.rs` re-exported via `lib.rs` if file size warrants). Derives:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub enum SaliencePolicy {
    #[default]
    PriorityWithNeedBoost,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OmissionReason {
    OverBudget { budget: u8, candidates_seen: u16 },
    SalienceBelowFloor { policy: SaliencePolicy },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationOmission {
    pub omitted_entity: EntityId,
    pub reason: OmissionReason,
    pub observed_tick: Tick,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservationOmissionLog {
    pub entries: VecDeque<ObservationOmission>,
}
```

Add a `default_omission_log_capacity()` function returning `16` for use in `#[serde(default = "...")]`.

### 2. Extend `PerceptionProfile`

In `crates/worldwake-core/src/belief.rs:2554`, add the two new fields with serde defaults:

```rust
pub struct PerceptionProfile {
    // ... existing 11 fields ...
    #[serde(default)]
    pub salience_policy: SaliencePolicy,
    #[serde(default = "default_omission_log_capacity")]
    pub omission_log_capacity: u8,
}
```

Update `Default for PerceptionProfile` at line 2586 to include both new fields. Then update all explicit construction sites with default salience policy and default omission-log capacity values (or contextually-appropriate non-default values for tests that exercise specific bounds).

### 3. Add `ObservationOmissionLog` to `AgentBeliefStore`

Add `#[serde(default)] pub observation_omission_log: ObservationOmissionLog` to `AgentBeliefStore`, adjacent to other agent-local belief/perception residue. This is the single canonical storage path for omission facts. `World::create_agent()` continues seeding `AgentBeliefStore::new()`; no separate component registration or create-agent delta entry is added.

### 4. Extend `BeliefStoreDiff` for save/replay parity

In `crates/worldwake-core/src/belief.rs:1120`, add paired fields matching the existing `social_observations_added` / `social_observations_removed_count` pattern for ring-buffered sub-stores:

```rust
pub struct BeliefStoreDiff {
    // ... existing 14 paired fields ...
    #[serde(default)]
    pub omission_log_added: Vec<ObservationOmission>,
    #[serde(default)]
    pub omission_log_removed_count: u16,
}
```

Update the diff-build/diff-apply functions in `crates/worldwake-core/src/belief.rs` and `crates/worldwake-core/src/delta.rs`.

### 5. Bump `SAVE_FORMAT_VERSION`

In `crates/worldwake-sim/src/save_load.rs:6`, change `pub const SAVE_FORMAT_VERSION: u32 = 66;` to `67`. Update the `load_*` dispatch and any version-specific test fixtures.

### 6. Regenerate profile docs

Run `python3 scripts/profile_docs.py --write` and commit the regenerated `docs/profiles/all-profiles.md`.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify) — add new types, extend `AgentBeliefStore`, extend `PerceptionProfile`, extend `BeliefStoreDiff`
- `crates/worldwake-core/src/component_tables.rs` (modify) — sample `AgentBeliefStore` and 1 `PerceptionProfile` construction site at line 391
- `crates/worldwake-core/src/world.rs` (modify) — 1 `PerceptionProfile` construction at line 810
- `crates/worldwake-core/src/delta.rs` (modify) — sample `AgentBeliefStore` / diff field handling
- `crates/worldwake-core/src/lib.rs` (modify) — re-export new types
- `crates/worldwake-sim/src/save_load.rs` (modify) — version bump
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify) — 2 `PerceptionProfile` construction sites at lines 3562, 4152
- `crates/worldwake-sim/src/institutional_knowledge_trace.rs` (modify) — 1 site at line 387
- `crates/worldwake-systems/src/patrol.rs` (modify) — 1 site at line 315
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — 3 sites at lines 966, 1076, 7051
- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify) — 1 site at line 145
- `crates/worldwake-ai/tests/golden_offices.rs` (modify) — 3 sites at lines 64, 178, 1919
- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (modify) — 1 site at line 83
- `crates/worldwake-ai/tests/golden_source_reliability.rs` (modify) — 1 site at line 22
- `crates/worldwake-ai/tests/golden_activation_decay.rs` (modify) — 1 site at line 35
- `crates/worldwake-ai/tests/golden_harness/soak_world.rs` (modify) — 1 site at line 128
- `crates/worldwake-ai/tests/golden_travel_physiology.rs` (modify) — 2 sites at lines 1361, 1569
- `crates/worldwake-ai/tests/golden_simulation_gaps.rs` (modify) — 1 site at line 21
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify) — 4 sites at lines 193, 478, 1257, 1491
- `crates/worldwake-ai/tests/golden_perception_exposure.rs` (modify) — 2 sites at lines 200, 380
- `crates/worldwake-ai/tests/golden_source_composite.rs` (modify) — 1 site at line 88
- `crates/worldwake-ai/tests/golden_exploration.rs` (modify) — 1 site at line 220
- `crates/worldwake-ai/tests/golden_experience_preferences.rs` (modify) — 1 site at line 95
- `docs/profiles/all-profiles.md` (modify, regenerated)

## Out of Scope

- Removing `CognitiveProfile.max_snapshot_entities_per_place` and the planner-snapshot per-place cap → ticket 003.
- Wiring perception writes that populate `AgentBeliefStore.observation_omission_log` → ticket 003.
- `Discrepancy::Omission` variant → ticket 004.
- `GoalBeliefView` accessor for `AgentBeliefStore.observation_omission_log` → ticket 002.
- `RootCandidateTrace.omitted_anchor` field → ticket 005.
- Observer rendering of omissions → ticket 006.
- Golden tests for omission scenarios → ticket 007.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core --lib` passes (new types' round-trip, `PerceptionProfile` field additions, `AgentBeliefStore` default, `BeliefStoreDiff` paired fields).
2. `cargo test -p worldwake-sim --lib` passes (save-format version bump and round-trip, `BeliefStoreDiff` diff-build/apply with new fields).
3. `cargo test -p worldwake-ai` passes (40 `PerceptionProfile` construction sites updated coherently).
4. `cargo build --workspace` succeeds.
5. `python3 scripts/profile_docs.py --write` regenerates `docs/profiles/all-profiles.md`; a no-write generation compared with `cmp -s` confirms it is current.

### Invariants

1. Every Agent has `AgentBeliefStore.observation_omission_log == ObservationOmissionLog::default()` immediately after `create_agent()`.
2. Save/load round-trip preserves `AgentBeliefStore.observation_omission_log` contents (empty after agent creation; populated only by ticket 003's perception write).
3. `OmissionReason` derives `Copy` (consumed by ticket 004's `Discrepancy::Omission` variant).
4. Existing scenarios deserialize without modification — no `salience_policy:` or `omission_log_capacity:` authoring required.
5. `SAVE_FORMAT_VERSION = 67` is the only version that round-trips post-ticket; older versions are rejected by the live `load_*` dispatch per the repo's no-backward-compatibility rule.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` cfg-test block — new test for `OmissionReason` and `ObservationOmission` round-trip serialization (`Copy`, `Eq`, serde) and `SaliencePolicy::default()` returning `PriorityWithNeedBoost`.
2. `crates/worldwake-core/src/belief.rs` cfg-test block — new/modified test confirming `AgentBeliefStore::new()` seeds `ObservationOmissionLog::default()` and `BeliefStoreDiff` preserves non-empty omission logs.
3. `crates/worldwake-sim/src/save_load.rs` cfg-test block — extend the save/load round-trip test to cover `SAVE_FORMAT_VERSION = 67` and a non-empty `ObservationOmissionLog` payload inside `AgentBeliefStore`.
4. Existing tests in `perception.rs` (line 1407+) and `agent_tick/tests.rs` continue passing — verifies the 40 construction-site updates didn't change behavior.

### Commands

1. `cargo test -p worldwake-core --lib`
2. `cargo test -p worldwake-sim --lib save_load`
3. `cargo test -p worldwake-ai`
4. `python3 scripts/profile_docs.py --write`; `python3 scripts/profile_docs.py > /tmp/worldwake-profile-docs-current.md`; `cmp -s /tmp/worldwake-profile-docs-current.md docs/profiles/all-profiles.md`
5. `./scripts/verify.sh`

Merge note: Ticket 001 bumps `SAVE_FORMAT_VERSION` 66→67 for the omission-log substrate. Later live reassessment in ticket 003 corrected the handoff: removing `CognitiveProfile.max_snapshot_entities_per_place` changes the serialized current `CognitiveProfile` component shape, so ticket 003 bumps `SAVE_FORMAT_VERSION` 67→68.

## Outcome

Completed on 2026-05-05.

- Landed option 2 from the FOUNDATIONS reassessment: `ObservationOmissionLog` is nested under `AgentBeliefStore`, not registered as a separate component.
- Added `SaliencePolicy`, `OmissionReason`, `ObservationOmission`, `ObservationOmissionLog`, `PerceptionProfile.salience_policy`, and `PerceptionProfile.omission_log_capacity`.
- Extended `BeliefStoreDiff` with compact omission-log add/remove fields, bumped `SAVE_FORMAT_VERSION` 66->67, and proved a non-empty omission log survives save/load through `AgentBeliefStore`.
- Regenerated `docs/profiles/all-profiles.md` and updated S135/sibling ticket wording where the draft still described a separate omission component.

## Deviations

- The drafted standalone `ObservationOmissionLog` component was rejected during reassessment because it would duplicate the same omission fact across a component path and `BeliefStoreDiff`. The implemented seam uses `AgentBeliefStore.observation_omission_log` as the single canonical path.
- The drafted `python3 scripts/profile_docs.py --check` command is not available on the live branch. The truthful proof is `--write` plus a generated-output comparison.

## Verification Result

- Passed `cargo test -p worldwake-core --lib --no-run`.
- Passed `cargo test --workspace --no-run`.
- Passed `cargo test -p worldwake-core --lib`.
- Passed `cargo test -p worldwake-sim --lib save_load`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo build --workspace`.
- Passed `python3 scripts/profile_docs.py --write` with existing documentation-gap warnings, then `python3 scripts/profile_docs.py > /tmp/worldwake-profile-docs-current.md` and `cmp -s /tmp/worldwake-profile-docs-current.md docs/profiles/all-profiles.md`.
- Passed `./scripts/verify.sh`, whose live gates are `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
