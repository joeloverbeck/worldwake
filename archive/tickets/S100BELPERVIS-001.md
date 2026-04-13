# S100BELPERVIS-001: Add `infrastructure_retention_ticks` to `PerceptionProfile`

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new field on `PerceptionProfile` component
**Deps**: S77 (archived, completed)

## Problem

Agents forget infrastructure beliefs (places, facilities, resource sources) at the same rate as transient observations. This ticket adds the `infrastructure_retention_ticks` field to `PerceptionProfile` so subsequent tickets can implement tiered retention. Without the field, the retention logic has no per-agent parameter to branch on.

## Assumption Reassessment (2026-04-13)

1. `PerceptionProfile` struct at `crates/worldwake-core/src/belief.rs:2179` has 8 fields, none named `infrastructure_retention_ticks`. Custom `impl Default` at line 2192 enumerates all fields explicitly. Confirmed via grep.
2. `PerceptionProfile` derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize` — the new `u64` field satisfies all these bounds. No `#[serde(default)]` on individual fields, so all construction sites and RON files must include the new field.
3. Live constructor fallout is broader than the original draft counted: there are 121 explicit `PerceptionProfile { ... }` literals across the workspace, all exhaustive and therefore all requiring the new field. Production and support surfaces include `belief.rs` (Default impl + local test helper), `delta.rs`, `component_tables.rs`, `world.rs` test helpers, and same-domain test/support builders across `worldwake-ai`, `worldwake-sim`, `worldwake-systems`, and `worldwake-cli`.
4. Scenario RON ownership is split: explicit authored scenario files such as `scenarios/cli-evaluation.ron` remain owned by ticket 003, but ticket 001 still owns same-crate `worldwake-cli` constructor and deserialization-fixture fallout in `src/scenario/mod.rs` and `src/scenario/types.rs`, because those tests embed `PerceptionProfile` literals and RON snippets directly.

## Architecture Check

1. Adding a `u64` field to an existing profile is the minimal change — no new types, no new components, no new registrations. The field type matches the existing `memory_retention_ticks: u64` field, maintaining consistency.
2. No backward-compatibility shims. All construction sites are updated to include the new field. Authored scenario-file compatibility remains a separate follow-up owned by ticket 003; this ticket only lands the shared field and the compile/test fallout required to keep the workspace honest.

## Verification Layers

1. `PerceptionProfile` struct includes new field → focused unit test plus compile-time verification via exhaustive field enumeration and same-crate serde fixtures
2. Default value is 480 → focused unit test asserting `PerceptionProfile::default().infrastructure_retention_ticks == 480`
3. Shared-type additive ticket. The owned behavior is still single-layer (`PerceptionProfile` shape only), but verification must sweep compile/test fallout across all crates that manually construct or deserialize the struct.

## What to Change

### 1. Add field to `PerceptionProfile` struct

In `crates/worldwake-core/src/belief.rs` at the struct definition (line 2179), add `pub infrastructure_retention_ticks: u64` after `memory_retention_ticks`.

### 2. Update `Default` impl

In `crates/worldwake-core/src/belief.rs` at the Default impl (line 2192), add `infrastructure_retention_ticks: 480` (10x the default `memory_retention_ticks` of 48).

### 3. Update test helper in belief.rs

The `sample_perception_profile()` helper at line 2299 explicitly constructs a `PerceptionProfile`. Add `infrastructure_retention_ticks: 480` (or a test-appropriate value like `memory_retention_ticks * 10`).

### 4. Update production construction sites

- `crates/worldwake-core/src/delta.rs:534` — add field to `ComponentValue::PerceptionProfile` construction
- `crates/worldwake-core/src/component_tables.rs:370` — add field to component table construction
- `crates/worldwake-core/src/world.rs:758` — add field to `sample_perception_profile` test helper

### 5. Update constructor fallout across workspace tests and support code

All exhaustive `PerceptionProfile { ... }` literals must include `infrastructure_retention_ticks`. Use the default value (480) unless a test specifically needs a different retention window. Key files:
- `crates/worldwake-ai/tests/golden_*.rs` (multiple files)
- `crates/worldwake-ai/tests/conformance_execution_budget.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-systems/src/perception.rs` (test section)
- `crates/worldwake-systems/tests/e15_information_integration.rs`
- `crates/worldwake-systems/src/justice_actions.rs`, `office_actions.rs`, `tell_actions.rs`, `consult_record_actions.rs`, `patrol.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`, `institutional_knowledge_trace.rs`, `action_semantics.rs`
- `crates/worldwake-ai/tests/golden_harness/mod.rs`, `golden_harness/soak_world.rs`
- `crates/worldwake-cli/src/scenario/mod.rs`

### 6. Update same-crate deserialization fixtures

Any test fixture RON or inline authored input in the same crate that deserializes directly into `PerceptionProfile` must include `infrastructure_retention_ticks`. Current owned fallout:
- `crates/worldwake-cli/src/scenario/types.rs`

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — struct, Default, test helper)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-core/src/component_tables.rs` (modify)
- `crates/worldwake-core/src/world.rs` (modify — test helper)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — test helper literal)
- `crates/worldwake-cli/src/scenario/types.rs` (modify — deserialization fixture)
- `crates/worldwake-ai/tests/golden_integration.rs` (modify)
- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (modify)
- `crates/worldwake-ai/tests/golden_production.rs` (modify)
- `crates/worldwake-ai/tests/golden_reasoning_diversity.rs` (modify)
- `crates/worldwake-ai/tests/golden_offices.rs` (modify)
- `crates/worldwake-ai/tests/golden_budget_exhaustion_snapshots.rs` (modify)
- `crates/worldwake-ai/tests/golden_simulation_gaps.rs` (modify)
- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify)
- `crates/worldwake-ai/tests/golden_social.rs` (modify)
- `crates/worldwake-ai/tests/golden_exploration.rs` (modify)
- `crates/worldwake-ai/tests/golden_combat.rs` (modify)
- `crates/worldwake-ai/tests/golden_determinism.rs` (modify)
- `crates/worldwake-ai/tests/golden_trade.rs` (modify)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (modify)
- `crates/worldwake-ai/tests/golden_expectation.rs` (modify)
- `crates/worldwake-ai/tests/golden_experience_preferences.rs` (modify)
- `crates/worldwake-ai/tests/golden_patrol.rs` (modify)
- `crates/worldwake-ai/tests/golden_pursuit.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)
- `crates/worldwake-ai/tests/conformance_execution_budget.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify — test section)
- `crates/worldwake-systems/tests/e15_information_integration.rs` (modify)
- `crates/worldwake-systems/src/justice_actions.rs` (modify — test section)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — test section)
- `crates/worldwake-systems/src/patrol.rs` (modify — test section)

## Outcome

Completed on 2026-04-13.

- Added `infrastructure_retention_ticks: u64` to `PerceptionProfile` in [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs) and set the default to `480`.
- Updated exhaustive `PerceptionProfile` literals and same-crate serde fixtures across `worldwake-core`, `worldwake-cli`, `worldwake-systems`, and `worldwake-ai` to include the new field, generally preserving the existing 10x retention ratio already described by the parent spec.
- Added focused proof by extending the core default-profile test to assert the new default field.

## Deviations

- Reassessment corrected the original fallout count from `50` to `121` explicit `PerceptionProfile` literals on the live branch.
- Authored scenario RON files under `scenarios/` were intentionally left to ticket `S100BELPERVIS-003`; only same-crate test/deserialization fixtures were updated here.
- Several originally listed candidate files did not need edits after the compile sweep, so the final touched-file list is narrower than the initial draft.

## Verification Result

- Passed `cargo test -p worldwake-core perception_profile_default_includes_infrastructure_retention_ticks`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`

## Out of Scope

- Modifying retention logic in `enforce_capacity` or `enforce_entity_claim_capacity` (ticket 002)
- Updating authored scenario RON files under `scenarios/` (ticket 003)
- Changing capacity eviction logic (S77, already complete)
- Adding new belief types or memory systems
- Modifying perception or observation scope

## Acceptance Criteria

### Tests That Must Pass

1. `PerceptionProfile::default().infrastructure_retention_ticks == 480`
2. All exhaustive `PerceptionProfile` construction and same-crate deserialization fixtures compile or parse with the new field added
3. All existing tests pass with the new field added (no behavioral change in this ticket)
4. Existing suite: `cargo test --workspace`

### Invariants

1. `PerceptionProfile` remains `Copy + Clone + Serialize + Deserialize` — the `u64` field satisfies all bounds
2. All explicit `PerceptionProfile` construction and deserialization sites compile or parse with the new field — the compiler and serde fixtures enforce exhaustive shape alignment
3. No behavioral change to retention logic — this ticket only adds the field, not the branching logic

## Test Plan

### New/Modified Tests

1. Add one focused assertion in `belief.rs` for the new default value.
2. Update existing constructor/deserialization fixtures mechanically. Behavioral retention tests are in ticket 002.

### Commands

1. `cargo test -p worldwake-core perception_profile_default_includes_infrastructure_retention_ticks` — focused default proof
2. `cargo test -p worldwake-core` — verify core crate compiles and all belief tests pass
3. `cargo test --workspace` — verify all crates compile with the new field
4. `cargo clippy --workspace --all-targets -- -D warnings` — lint clean
