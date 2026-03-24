# S23-001: Introduce BlockerKey, BlockerDiagnostic, and refactor BlockedIntentMemory to BTreeMap

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — BlockedIntentMemory storage and API change (worldwake-core)
**Deps**: S21 (completed), S20 (completed)

## Problem

`BlockedIntentMemory` uses a `Vec<BlockedIntent>` keyed only by `GoalKey`. This means recording a blocker for "harvest fruit" at Place A replaces any existing blocker for "harvest fruit" at Place B, and `is_blocked()` cannot distinguish place-scoped from global blockers. The data model must change before any consumer can take advantage of compound keying.

## Assumption Reassessment (2026-03-24)

1. `BlockedIntentMemory` is currently `Vec<BlockedIntent>` with `record()` deduplicating by `goal_key` alone — confirmed in `crates/worldwake-core/src/blocked_intent.rs:7-8,20-24`.
2. `BlockedIntent` fields: `goal_key`, `blocking_fact`, `related_entity`, `related_place`, `related_action`, `observed_tick`, `expires_tick` — confirmed at lines 39-47.
3. `GoalKey` derives `Ord` (via `goal.rs`), `EntityId` derives `Ord` (via `ids.rs` macro), `ActionDefId` derives `Ord` (via `ids.rs` macro) — all constituents needed for `BTreeMap<BlockerKey, _>` key.
4. `sample_blocked_intent()` in `test_utils.rs:105-114` constructs `BlockedIntent` with all old fields — must be updated.
5. `sample_blocked_intent_memory()` in `test_utils.rs:118-122` wraps a `Vec` — must be updated to `BTreeMap`.
6. `is_blocked()` is called from `candidate_generation.rs` (S23-003 scope) and nowhere else in core — confirmed no other core callers.
7. `clear_for(&GoalKey)` has zero call sites in worldwake-ai — confirmed by grep. It is only used in core tests. The spec replaces it with `clear_for(&BlockerKey)` and adds `clear_all_for_goal(&GoalKey)`.
8. `blocks_goal_generation()` is unchanged by this ticket — same two-variant carve-out.
9. Single-layer ticket: changes are purely structural (data model). No planner/golden/runtime behavioral change.

## Architecture Check

1. `BTreeMap<BlockerKey, BlockedIntent>` gives deterministic iteration (project invariant: BTreeMap only in authoritative state) and O(log n) exact-key lookup. The compound key naturally supports coexistence of multiple blockers for the same goal at different places/targets/actions.
2. No backward-compatibility shims — the old `Vec` storage and old field names are removed entirely.

## Verification Layers

1. `BlockerKey` derives correct bounds → compile-time verification (BTreeMap key requires Ord)
2. `record()` inserts by compound key → focused unit test (two blockers for same goal at different places coexist)
3. `is_blocked()` tiered matching → focused unit tests (global blocker matches all, place-scoped only matches that place)
4. `is_blocked_for_search()` skips `blocks_goal_generation()` gate → focused unit test
5. Serialization round-trip → existing `blocked_intent_memory_roundtrips_through_bincode` test (updated)

## What to Change

### 1. Add `BlockerKey` struct

```rust
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BlockerKey {
    pub goal_key: GoalKey,
    pub place: Option<EntityId>,
    pub target: Option<EntityId>,
    pub action_def: Option<ActionDefId>,
}
```

### 2. Add `BlockerDiagnostic` struct

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerDiagnostic {
    pub action_def: ActionDefId,
}
```

### 3. Refactor `BlockedIntent`

Replace `goal_key`, `related_entity`, `related_place`, `related_action` with `blocker_key: BlockerKey`. Add `diagnostic_context: Option<BlockerDiagnostic>`. Keep `blocking_fact`, `observed_tick`, `expires_tick`.

`blocks_goal_generation()` accesses `self.blocking_fact` — unchanged.

### 4. Refactor `BlockedIntentMemory`

- Storage: `BTreeMap<BlockerKey, BlockedIntent>`
- `record()`: `self.intents.insert(intent.blocker_key, intent)`
- `is_blocked(goal_key, place, target, action_def, current_tick)`: iterates values with `matches_scope()` + `blocks_goal_generation()` gate
- `is_blocked_for_search(goal_key, place, target, action_def, current_tick)`: same but WITHOUT `blocks_goal_generation()` gate
- `expire()`: `self.intents.retain(|_, i| i.expires_tick > current_tick)`
- `clear_for(&BlockerKey)`: `self.intents.remove(key)`
- `clear_all_for_goal(&GoalKey)`: `self.intents.retain(|k, _| k.goal_key != *goal_key)`

### 5. Add `matches_scope()` helper

Free function implementing tiered matching: goal-only blockers (place=None, target=None, action=None) match everything; place-scoped blockers require place match; target-scoped require target match; action-scoped require action match.

### 6. Update `test_utils.rs`

- `sample_blocked_intent()` → uses `BlockerKey` instead of individual fields
- `sample_blocked_intent_memory()` → wraps `BTreeMap` instead of `Vec`

### 7. Update all unit tests in `blocked_intent.rs`

Rewrite to use `BlockerKey`-based construction and new `is_blocked()` signature.

## Files to Touch

- `crates/worldwake-core/src/blocked_intent.rs` (modify — primary change)
- `crates/worldwake-core/src/test_utils.rs` (modify — fixture updates)

## Out of Scope

- **No changes to `failure_handling.rs`** — that is S23-002
- **No changes to `candidate_generation.rs`** — that is S23-003
- **No changes to `search/`** — that is S23-004
- **No changes to `budget.rs` or `decision_trace.rs`** — that is S23-005
- **No changes to `agent_tick/`** — consumers adapt in S23-002/003/004/005
- **No new golden tests** — that is S23-006
- **Do not change `BlockingFact` enum variants**
- **Do not change `blocks_goal_generation()` logic**

## Acceptance Criteria

### Tests That Must Pass

1. `blocked_intent_types_satisfy_required_bounds` — `BlockerKey` and `BlockerDiagnostic` satisfy bounds
2. `blocked_intent_memory_defaults_empty` — BTreeMap default is empty
3. `is_blocked_matches_only_live_entries_for_goal_key` — updated for new signature
4. `source_depleted_does_not_block_goal_generation` — unchanged behavior via `blocks_goal_generation()`
5. `record_replaces_existing_entry_for_same_compound_key` — same key overwrites
6. NEW: `record_preserves_different_place_for_same_goal` — two blockers for same goal at different places coexist
7. `expire_removes_entries_at_or_before_current_tick` — updated for BTreeMap
8. `clear_for_removes_matching_blocker_key` — updated for compound key
9. NEW: `clear_all_for_goal_removes_all_entries_for_goal` — clears across all places
10. `blocked_intent_memory_roundtrips_through_bincode` — BTreeMap serialization
11. `exclusive_facility_blockers_do_not_block_goal_generation` — unchanged behavior
12. NEW: `is_blocked_for_search_ignores_blocks_goal_generation_gate` — SourceDepleted blocks at search level
13. NEW: `global_blocker_matches_any_place_query` — (goal, None, None, None) blocker matches (goal, Some(place), ...) query
14. NEW: `place_scoped_blocker_does_not_match_different_place` — (goal, Some(A)) blocker does NOT match (goal, Some(B)) query
15. NEW: `place_scoped_blocker_does_not_match_global_query` — (goal, Some(A)) blocker does NOT match (goal, None, None, None) query
16. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `BTreeMap` only in authoritative state — no `HashMap`/`HashSet` introduced
2. `BlockedIntent` remains `Serialize + Deserialize` — save/load compatibility
3. `blocks_goal_generation()` carve-out for `SourceDepleted` and `ExclusiveFacilityUnavailable` is preserved exactly
4. Deterministic iteration order (BTreeMap guarantees this)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/blocked_intent.rs::tests` — all existing tests rewritten for new API; 5+ new tests for tiered matching, coexistence, search-level blocking
2. `crates/worldwake-core/src/test_utils.rs` — fixture updates (no new tests, but downstream tests depend on these)

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy -p worldwake-core`

**Note**: `cargo test -p worldwake-ai` and `cargo test --workspace` will NOT compile after this ticket until S23-002 and S23-003 are applied. This ticket is expected to be landed together with S23-002 and S23-003 in the same branch.
