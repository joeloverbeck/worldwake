# S43COMTYPDIF-002: Extend GoalKind::ShareBelief with communication_class + update candidate generation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — GoalKind enum variant extended, candidate generation updated
**Deps**: S43COMTYPDIF-001

## Problem

The AI pipeline has no way to carry communication class through goal ranking, suppression, and plan synthesis. The classification must be computed at candidate generation time (where belief context is available) and travel with the goal. Without this, tickets 003 (suppression/ranking) and 005 (golden tests) cannot distinguish alarm vs gossip ShareBelief goals.

## Assumption Reassessment (2026-04-03)

1. `GoalKind::ShareBelief { listener: EntityId, topic: TellTopic }` at `goal.rs:63` — confirmed. Derives: `Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`.
2. `CommunicationClass` (from ticket 001) must also derive `Copy` to preserve `GoalKind`'s `Copy` bound — ticket 001 specifies this.
3. `GoalKey::from(GoalKind)` at `goal.rs:139` extracts `entity: Some(listener)` for ShareBelief — adding `communication_class` does not change the key extraction logic since class is deterministically derived from topic.
4. `emit_social_candidates()` at `candidate_generation.rs:709` is a private function. It already has access to `ctx.view` which provides belief store access via `ctx.view.known_entity_beliefs(ctx.agent)` — sufficient to call `classify_communication`.
5. ShareBelief match arms exist in: `goal.rs` (GoalKey extraction, tests), `goal_policy.rs` (suppression), `ranking.rs` (motive), `feasibility.rs`, `exhaustion.rs`, `decision_trace.rs`, `goal_dispatch_decl.rs`, `goal_dispatch_key.rs`, `goal_model.rs`, `candidate_generation.rs`, `agent_tick/tests.rs`, plus golden test files (`golden_social.rs`, `golden_emergent.rs`, `golden_integration.rs`, `golden_offices.rs`, `golden_care.rs`, `golden_resilience.rs`, `golden_t22_bandit_camp_destruction.rs`, `planner_conformance.rs`).
6. All non-candidate-generation match arms can use `communication_class: _` wildcard or `..` to avoid coupling to the new field where the class is irrelevant.
7. Existing golden tests construct `GoalKind::ShareBelief` in scenario setup — these need the new field. Since classification is deterministic, use `classify_communication()` or `CommunicationClass::Gossip` as the default for existing tests where the class is not under test.

## Architecture Check

1. Placing the class on the `GoalKind` variant (not on `GroundedGoal`) keeps the data local to ShareBelief — no `Option` pollution on a shared struct. The class is deterministically derived from topic + belief state, so it doesn't change GoalKey equality semantics.
2. No backwards-compatibility shims. All existing match arms are updated to acknowledge the new field.

## Verification Layers

1. GoalKind::ShareBelief construction compiles with new field -> compilation of `cargo build --workspace`
2. Candidate generation attaches correct class -> focused unit test in `candidate_generation.rs` or runtime decision-trace assertion
3. All existing golden tests still pass with updated constructors -> `cargo test -p worldwake-ai`
4. GoalKey extraction unchanged -> existing `goal_key_extracts_listener_and_subject_for_share_belief` test in `goal.rs`

## What to Change

### 1. Extend GoalKind::ShareBelief variant

In `crates/worldwake-core/src/goal.rs`:

```rust
ShareBelief {
    listener: EntityId,
    topic: TellTopic,
    communication_class: CommunicationClass,
}
```

Add `CommunicationClass` to the imports from `crate`.

### 2. Update GoalKey extraction

In `goal.rs`, the `From<GoalKind>` impl for `GoalKey` — the ShareBelief arm extracts `listener` and topic. Add `communication_class: _` to the destructuring pattern (class is not part of the key).

### 3. Update all match arms across worldwake-ai

For each file with ShareBelief match arms, update the destructuring. Use `..` or `communication_class: _` for arms that don't use the class. Key files:

- `goal_policy.rs` — will be modified in ticket 003, for now use `..`
- `ranking.rs` — will be modified in ticket 003, for now use `..`
- `feasibility.rs`, `exhaustion.rs`, `decision_trace.rs`, `goal_dispatch_decl.rs`, `goal_dispatch_key.rs`, `goal_model.rs` — use `..`
- `agent_tick/tests.rs` — update constructors

### 4. Update emit_social_candidates()

In `candidate_generation.rs:795`, where `GoalKind::ShareBelief` goals are constructed for each selected topic, compute the class:

```rust
let communication_class = classify_communication(&topic, speaker_beliefs);
```

The speaker's belief store is already available via `ctx.view.known_entity_beliefs(ctx.agent)` (for entity beliefs) and the full `AgentBeliefStore` via `ctx.view`. Pass it to the classification function.

### 5. Update golden test constructors

In all golden test files that construct `GoalKind::ShareBelief`, add the `communication_class` field. For existing tests where the class is not under test, use `CommunicationClass::Gossip` (the safe default) or compute via `classify_communication()` if belief context is available.

## Files to Touch

- `crates/worldwake-core/src/goal.rs` (modify) — extend variant, update GoalKey extraction, update tests
- `crates/worldwake-ai/src/candidate_generation.rs` (modify) — compute and attach class
- `crates/worldwake-ai/src/goal_policy.rs` (modify) — update match arm destructuring
- `crates/worldwake-ai/src/ranking.rs` (modify) — update match arm destructuring
- `crates/worldwake-ai/src/feasibility.rs` (modify) — update match arm
- `crates/worldwake-ai/src/exhaustion.rs` (modify) — update match arm
- `crates/worldwake-ai/src/decision_trace.rs` (modify) — update match arm
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify) — update match arm
- `crates/worldwake-ai/src/goal_dispatch_key.rs` (modify) — update match arm
- `crates/worldwake-ai/src/goal_model.rs` (modify) — update match arms and constructors
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify) — update constructors
- `crates/worldwake-ai/tests/golden_social.rs` (modify) — update constructors
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify) — update constructors
- `crates/worldwake-ai/tests/golden_integration.rs` (modify) — update constructors
- `crates/worldwake-ai/tests/golden_offices.rs` (modify) — update constructors
- `crates/worldwake-ai/tests/golden_care.rs` (modify) — update constructors
- `crates/worldwake-ai/tests/golden_resilience.rs` (modify) — update constructors
- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify) — update constructors
- `crates/worldwake-ai/tests/planner_conformance.rs` (modify) — update constructors

## Out of Scope

- Class-aware suppression logic in goal_policy.rs (ticket 003)
- Class-aware ranking multiplier in ranking.rs (ticket 003)
- Tell handler changes (ticket 004)
- New golden test scenarios (ticket 005)

## Acceptance Criteria

### Tests That Must Pass

1. `goal_key_extracts_listener_and_subject_for_share_belief` in `goal.rs` still passes
2. All existing golden tests pass with updated constructors
3. `emit_social_candidates` attaches the correct class for at least one Alarm-class and one Gossip-class topic (new or extended test)
4. Existing suite: `cargo test --workspace`

### Invariants

1. `GoalKind` remains `Copy + Serialize + Deserialize` — `CommunicationClass` is `Copy`
2. GoalKey equality is unchanged — `communication_class` is not part of key extraction
3. All existing behavior is preserved — class field is added but not yet consumed by suppression/ranking

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/goal.rs` — update existing ShareBelief constructor tests to include `communication_class`
2. `crates/worldwake-ai/src/candidate_generation.rs` — extend existing social candidate test (or add new) to verify class is attached correctly
3. All golden test files listed above — update constructors (no new test logic)

### Commands

1. `cargo test -p worldwake-core -- share_belief`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`
