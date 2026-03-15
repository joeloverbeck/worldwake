# E15BSOCAIGOA-006: Extend golden test harness with belief seeding and Tell helpers

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — test infrastructure only
**Deps**: E15 (completed)

## Problem

The golden test harness has no helpers for seeding specific beliefs into an agent's belief store, querying belief state, or setting up TellProfile components. All 13 golden social tests (T1–T13) need these helpers.

## Assumption Reassessment (2026-03-15)

1. `GoldenHarness` in `crates/worldwake-ai/tests/golden_harness/mod.rs` has `seed_actor_beliefs()`, `seed_actor_local_beliefs()`, `seed_actor_world_beliefs()` — these seed position/inventory beliefs but not arbitrary belief content.
2. `AgentBeliefStore` has `update_entity(id, BelievedEntityState)` and `get_entity(id) -> Option<&BelievedEntityState>`.
3. `TellProfile` is a component — can be attached via `world.set_component()`.
4. `seed_agent()` does NOT accept TellProfile or PerceptionProfile — these must be set separately.
5. Existing helpers use `seed_actor_beliefs()` pattern — new helpers should follow the same naming convention.

## Architecture Check

1. Pure test helper additions — no production code changes.
2. Follows existing harness patterns: functions take `&mut World`, `&mut EventLog`, and entity ids.
3. Helpers are composable — `seed_agent_with_tell_profile` wraps `seed_agent` + component attachment.

## What to Change

### 1. Add seed_agent_with_tell_profile helper

```rust
pub fn seed_agent_with_tell_profile(
    world: &mut World,
    event_log: &mut EventLog,
    name: &str,
    place: EntityId,
    needs: HomeostaticNeeds,
    metabolism: MetabolismProfile,
    utility: UtilityProfile,
    tell_profile: TellProfile,
    perception_profile: PerceptionProfile,
) -> EntityId
```

Or alternatively, extend `seed_agent()` to optionally accept TellProfile. Follow whichever pattern is cleaner given current harness structure.

### 2. Add seed_belief helper

```rust
pub fn seed_belief(
    world: &mut World,
    agent: EntityId,
    subject: EntityId,
    believed_state: BelievedEntityState,
)
```

Inserts a specific belief into the agent's `AgentBeliefStore.known_entities`.

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
- Changes to existing seed_agent() signature (unless extending is clearly cleaner)
- Harness changes for non-social tests

## Acceptance Criteria

### Tests That Must Pass

1. `seed_agent_with_tell_profile` creates agent with TellProfile component accessible via world query
2. `seed_belief` inserts belief retrievable via `agent_belief_about`
3. `seed_belief` for same subject overwrites previous belief (BTreeMap semantics)
4. `agent_belief_count` returns 0 for agent with no seeded beliefs
5. `agent_belief_count` returns correct count after multiple `seed_belief` calls
6. Existing suite: `cargo test -p worldwake-ai` — no regressions

### Invariants

1. Test helpers do not bypass World's component storage API
2. All helpers use the same event_log pattern as existing harness functions
3. No production code modified

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_harness/mod.rs` — smoke tests for new helpers (can be inline or in a small test module)

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
