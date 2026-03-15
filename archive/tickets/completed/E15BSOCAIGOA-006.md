# E15BSOCAIGOA-006: Extend golden test harness with explicit belief and profile helpers

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test infrastructure only
**Deps**: E15 (completed)

## Problem

The golden test harness can seed snapshot-based beliefs (`seed_actor_beliefs`, `seed_actor_local_beliefs`, `seed_actor_world_beliefs`), but it cannot yet inject arbitrary `BelievedEntityState` values or conveniently override an agent's social/perception profiles after seeding. The E15b social golden tests need those narrower capabilities to set up stale beliefs, rumor/report provenance, and acceptance/relay policy variants without duplicating agent construction logic.

## Assumption Reassessment (2026-03-15)

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` already has `seed_actor_beliefs()`, `seed_actor_local_beliefs()`, and `seed_actor_world_beliefs()`. They derive beliefs from authoritative world snapshots, which is useful for locality-preserving setup but insufficient for stale, rumor, report, or otherwise hand-crafted belief states.
2. `World::create_agent(...)` already attaches `AgentBeliefStore`, `PerceptionProfile`, and `TellProfile` by default. The current `seed_agent()` helper inherits that behavior through `txn.create_agent(...)`.
3. Because of (2), the original proposal to add `seed_agent_with_tell_profile(...)` duplicates responsibility that already belongs to agent creation. The cleaner test API is to keep one agent-construction path and add explicit profile-override helpers.
4. `AgentBeliefStore` exposes `update_entity(id, state)` and `get_entity(&id)`. Important nuance: `update_entity` is not raw BTreeMap overwrite semantics. It ignores an incoming belief if the existing belief has a newer `observed_tick`; equal or newer ticks replace.
5. `WorldTxn` already exposes `set_component_agent_belief_store`, `set_component_perception_profile`, and `set_component_tell_profile`, so the harness can stay aligned with authoritative component mutation APIs instead of reaching around them.
6. Existing worldwake-ai golden tests currently hand-roll profile overrides in individual files. Centralizing those setup operations in the harness will reduce repetition and keep future social tests more consistent.

## Architecture Check

1. Pure test-helper additions remain the right scope. No production behavior change is needed here.
2. Adding a second agent-construction helper for Tell/Perception profiles is not the best architecture. It would create overlapping constructor variants in the harness and encourage more variants later.
3. The cleaner, more extensible shape is an orthogonal helper surface:
   - `seed_agent(...)` remains the single way to create a standard AI agent in the harness
   - explicit setters override `TellProfile` or `PerceptionProfile` when a scenario needs non-default policy
   - explicit belief seed/query helpers manipulate the belief store directly through `WorldTxn`
4. This keeps the harness DRY, composable, and aligned with the project's "no backward compatibility / no aliasing" rule. We are not preserving two competing patterns for the same responsibility.
5. If social tests later need more agent-policy customization, that should extend the override-helper layer rather than proliferating constructor wrappers.

## What to Change

### 1. Add explicit profile override helpers

```rust
pub fn set_agent_tell_profile(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    tell_profile: TellProfile,
)
```

```rust
pub fn set_agent_perception_profile(
    world: &mut World,
    event_log: &mut EventLog,
    agent: EntityId,
    perception_profile: PerceptionProfile,
)
```

These wrap the existing `WorldTxn` component setters. They replace the original `seed_agent_with_tell_profile(...)` idea because agent creation already attaches both components by default.

### 2. Add seed_belief helper

```rust
pub fn seed_belief(
    world: &mut World,
    agent: EntityId,
    subject: EntityId,
    believed_state: BelievedEntityState,
)
```

Loads the agent's existing `AgentBeliefStore`, applies `update_entity`, and writes the updated store back through `WorldTxn`.

The helper should preserve the authoritative `update_entity` semantics:
- newer stored beliefs are not downgraded by an older `observed_tick`
- equal or newer ticks replace the previous value

### 3. Add agent_belief_about accessor

```rust
pub fn agent_belief_about<'a>(
    world: &'a World,
    agent: EntityId,
    subject: EntityId,
) -> Option<&'a BelievedEntityState>
```

### 4. Add agent_belief_count accessor

```rust
pub fn agent_belief_count(world: &World, agent: EntityId) -> usize
```

Returns `AgentBeliefStore.known_entities.len()`.

## Files to Touch

- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify)

## Out of Scope

- Golden social test file (E15BSOCAIGOA-007 through E15BSOCAIGOA-010)
- Any production code changes
- Changes to existing `seed_agent()` signature
- Harness changes for non-social tests

## Acceptance Criteria

### Tests That Must Pass

1. `set_agent_tell_profile` updates an already-seeded agent's TellProfile through world component queries
2. `set_agent_perception_profile` updates an already-seeded agent's PerceptionProfile through world component queries
3. `seed_belief` inserts a belief retrievable via `agent_belief_about`
4. `seed_belief` replaces a same-subject belief when the new `observed_tick` is equal or newer
5. `seed_belief` does not replace a same-subject belief with an older `observed_tick`
6. `agent_belief_count` returns 0 for an agent with no seeded beliefs and the correct count after multiple `seed_belief` calls
7. Existing suite: `cargo test -p worldwake-ai` — no regressions

### Invariants

1. Test helpers do not bypass World's component storage API
2. All helpers use the same event_log pattern as existing harness functions
3. No production code modified

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` — helper tests for TellProfile override, PerceptionProfile override, explicit belief seeding, count access, and older-belief suppression semantics

### Commands

1. `cargo test -p worldwake-ai golden_harness`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Reassessed the ticket against the live harness, E15b spec, and current core APIs before implementation
  - Corrected the ticket scope to remove the redundant `seed_agent_with_tell_profile(...)` constructor idea
  - Added orthogonal golden-harness helpers for `TellProfile` override, `PerceptionProfile` override, explicit belief seeding, belief lookup, and belief counting
  - Added helper tests that lock in the real `AgentBeliefStore::update_entity(...)` semantics: equal/newer snapshots replace, older snapshots do not
- Deviations from original plan:
  - Did not add a second agent-construction helper because `World::create_agent(...)` already attaches `AgentBeliefStore`, `PerceptionProfile`, and `TellProfile` by default, and duplicating that path would weaken the harness architecture
  - Replaced the stale "BTreeMap overwrite semantics" assumption with the actual observed-tick-gated replacement rule used by production belief storage
- New/modified tests:
  - `crates/worldwake-ai/tests/golden_harness/mod.rs` — `profile_override_helpers_update_agent_components`
  - `crates/worldwake-ai/tests/golden_harness/mod.rs` — `seed_belief_accessors_and_count_reflect_seeded_state`
  - `crates/worldwake-ai/tests/golden_harness/mod.rs` — `seed_belief_replaces_same_subject_when_tick_is_equal_or_newer`
  - `crates/worldwake-ai/tests/golden_harness/mod.rs` — `seed_belief_preserves_newer_existing_belief_against_older_input`
- Verification results:
  - `cargo test -p worldwake-ai golden_harness`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
