# S74INTCOMM-001: Add `planning_switch_margin` to CognitiveProfile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new field on `CognitiveProfile` component
**Deps**: None

## Problem

The planning path has no commitment inertia for idle agents. Active-action agents benefit from `switch_margin` (they only switch if a challenger exceeds the margin), but agents in the planning phase have no equivalent parameter. This ticket adds the per-agent `planning_switch_margin` field that S74INTCOMM-002 will use to gate replanning decisions.

## Assumption Reassessment (2026-04-08)

1. `CognitiveProfile` struct is at `crates/worldwake-core/src/cognitive_profile.rs` with the existing derive set `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize`, plus a `Default` impl and `Component` impl. The new `Permille` field satisfies all existing trait bounds (`Copy`, `Serialize`, `Deserialize`, `Eq`, `Ord`).
2. `CognitiveProfile` is part of `AgentDef` in `crates/worldwake-cli/src/scenario/types.rs:86` (`pub cognitive_profile: Option<CognitiveProfile>`). Since it's `Option<CognitiveProfile>`, the new field is automatically scenario-definable — no separate `AgentDef` change needed. Agents that don't specify it get the `Default` impl.
3. Struct literal construction sites without spread syntax (must add new field): `delta.rs:547`, `cognitive_profile.rs:82` (round-trip test), `decision_runtime.rs:347`, `failure_handling.rs:1275`, `agent_tick/planning.rs:1120`, `goal_model.rs:2290`, `agent_tick/tests.rs:91`, `search/tests.rs:43`. Sites using `..CognitiveProfile::default()` or `..Default::default()` are safe (auto-pick up the default).
4. Follow-up constructor sweep correction: the additional `CognitiveProfile` search hits in `worldwake-cli/src/handlers/persistence.rs`, `worldwake-ai/tests/conformance_execution_budget.rs`, `worldwake-ai/tests/golden_reasoning_diversity.rs`, `worldwake-ai/tests/golden_supply_chain.rs`, and `worldwake-ai/tests/golden_care.rs` all use `..CognitiveProfile::default()`. Ticket said those golden files needed explicit field edits; live code already inherits the new field through struct update syntax, so no change is required there. Safe because the new field's intended default is the ticketed behavior.

## Architecture Check

1. Adding a parallel field to `switch_margin` (which already serves the active-action interrupt path) is the cleanest approach — it preserves the existing CognitiveProfile-as-cognitive-parameter pattern and requires no new types or components.
2. No backwards-compatibility aliasing/shims introduced. The new field is simply added with a default value.

## Verification Layers

1. All struct literal construction sites compile with the new field -> build success (workspace-wide `cargo build`)
2. `planning_switch_margin` default is `Permille(150)` -> `CognitiveProfile::default()` unit test
3. Existing `switch_margin` behavior is unaffected -> all existing tests pass (single-layer ticket: the field addition has no runtime behavior until S74INTCOMM-002)

## What to Change

### 1. Add field to CognitiveProfile struct

In `crates/worldwake-core/src/cognitive_profile.rs`:
- Add `pub planning_switch_margin: Permille` to the struct definition (after `switch_margin` for logical grouping)
- Update `Default` impl to set `planning_switch_margin: Permille::new_unchecked(150)`

### 2. Update all full struct literal construction sites

Add `planning_switch_margin: Permille::new_unchecked(150)` (or contextually appropriate values) to every site that constructs `CognitiveProfile` without spread syntax:

**worldwake-core:**
- `delta.rs:547` — delta round-trip test: use a distinct value (e.g., 175) to exercise serialization

**worldwake-ai (test helpers):**
- `decision_runtime.rs:347` — `cognitive()` helper
- `failure_handling.rs:1275` — `cognitive()` helper
- `agent_tick/planning.rs:1120` — `cognitive()` helper
- `goal_model.rs:2290` — `cognitive()` helper
- `agent_tick/tests.rs:91` — `cognitive()` helper
- `search/tests.rs:43` — `cognitive()` helper

All test helpers should use the same value as `Default` (150) unless the test specifically exercises planning switch margin behavior (none do yet — that's S74INTCOMM-002).

## Files to Touch

- `crates/worldwake-core/src/cognitive_profile.rs` (modify)
- `crates/worldwake-core/src/delta.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/failure_handling.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- Modifying `try_continue_snapshot_plan` logic (S74INTCOMM-002)
- Adjusting `golden_merchant_selling` margin (S74INTCOMM-003)
- Removing `is_needs_only()` method from `dirty_set.rs` (S74INTCOMM-002)
- Adding any new behavior that reads `planning_switch_margin` at runtime

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core` — CognitiveProfile default, serialization round-trip
2. `cargo test -p worldwake-ai` — all existing tests compile and pass with new field
3. Existing suite: `cargo test --workspace`

### Invariants

1. `CognitiveProfile::default().planning_switch_margin == Permille(150)`
2. All existing golden tests pass with identical outcomes (the field has no runtime effect yet)
3. Workspace builds cleanly: `cargo clippy --workspace --all-targets -- -D warnings`

## Test Plan

### New/Modified Tests

1. None — the existing `cognitive_profile.rs` unit tests and delta round-trip test exercise the new field via struct literal construction. No new test file needed.

### Commands

1. `cargo test -p worldwake-core -- cognitive_profile` — targeted CognitiveProfile tests
2. `cargo clippy --workspace --all-targets -- -D warnings` — lint verification
3. `cargo test --workspace` — full suite

## Outcome

Completed on 2026-04-08.

- Added `planning_switch_margin: Permille` to `CognitiveProfile` with default `Permille(150)`.
- Updated the core round-trip fixtures and all no-spread AI test helper literals so the shared struct shape remains compile-safe across workspace crates.
- Reassessed the originally listed golden test files and removed them from owned edit scope because their `CognitiveProfile` literals already use `..CognitiveProfile::default()` and inherit the new field automatically.
- Later full-suite reruns confirmed the initial `golden_report_found_after_search` failures were not caused by this field addition; they came from a separate `report_found` planner/validation regression that was fixed afterward in the same worktree.

## Verification Result

- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
