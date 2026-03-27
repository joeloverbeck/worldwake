# S30-001: Unify exhaustion cache into ExhaustionEntry struct

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AgentDecisionRuntime field layout, planning exhaustion skip/backoff logic
**Deps**: None (standalone refactoring)

## Problem

`AgentDecisionRuntime` tracks goal exhaustion state across two separate maps: `search_exhausted_goals: BTreeMap<GoalKey, Tick>` and `exhaustion_counts: BTreeMap<GoalKey, u8>`. This duplication complicates serialization (S30-002) and blocks S31's plan to add `invalidation: Option<ExhaustionInvalidationCondition>` to each entry. Unifying into a single `ExhaustionEntry` struct is a prerequisite for clean serde derives and future extensibility.

## Assumption Reassessment (2026-03-27)

1. `search_exhausted_goals: BTreeMap<GoalKey, Tick>` confirmed at `decision_runtime.rs:75`. `exhaustion_counts: BTreeMap<GoalKey, u8>` confirmed at `decision_runtime.rs:79`.
2. Both maps are read in `build_candidate_plans()` at `planning.rs:157-158` (skip filter at line 171-175, backoff at line 211-216).
3. Both maps are written in `planning.rs` via `record_exhausted_goals` and `reset_exhausted_goals_if_needed` — grep confirms only `planning.rs` and `decision_runtime.rs` reference these fields.
4. `GoalKey` already derives `Serialize, Deserialize` (`goal.rs:86`). `Tick` already derives `Serialize, Deserialize`. No transitive serde work needed for the key/value types.
5. This is a pure data-model refactoring. No behavioral change, no cross-system interaction, no ordering dependency.

## Architecture Check

1. A single `BTreeMap<GoalKey, ExhaustionEntry>` is cleaner than two parallel maps that must be kept in sync. It eliminates the implicit invariant that both maps share the same key set.
2. No backward-compatibility shims — the old fields are removed outright and all call sites updated.

## Verification Layers

1. Exhaustion skip logic unchanged → focused unit test asserting TTL skip behavior with `ExhaustionEntry`
2. Exponential backoff unchanged → focused unit test asserting budget reduction with `ExhaustionEntry.count`
3. Single-layer ticket (AI runtime data model) — no cross-layer mapping needed.

## What to Change

### 1. Define `ExhaustionEntry` struct in `decision_runtime.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ExhaustionEntry {
    pub exhausted_at: Tick,
    pub count: u8,
}
```

Note: Do NOT add `Serialize, Deserialize` here — that comes in S30-002 when the full `AgentDecisionRuntime` gains serde.

### 2. Replace two fields with one in `AgentDecisionRuntime`

Remove:
- `search_exhausted_goals: BTreeMap<GoalKey, Tick>`
- `exhaustion_counts: BTreeMap<GoalKey, u8>`

Add:
- `exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>`

### 3. Update `planning.rs` call sites

- `build_candidate_plans()`: Change parameters from `skip_exhausted: &BTreeMap<GoalKey, Tick>, exhaustion_counts: &BTreeMap<GoalKey, u8>` to `exhaustion_cache: &BTreeMap<GoalKey, ExhaustionEntry>`. Update TTL skip filter to read `entry.exhausted_at`. Update backoff logic to read `entry.count`.
- `record_exhausted_goals` (or equivalent): Write to unified `exhaustion_cache` map.
- `reset_exhausted_goals_if_needed` (or equivalent): Clear/prune from unified `exhaustion_cache` map.

### 4. Update callers of `build_candidate_plans()` in `agent_tick/planning.rs`

Pass `&runtime.exhaustion_cache` instead of the two separate map references.

## Files to Touch

- `crates/worldwake-ai/src/decision_runtime.rs` (modify — define `ExhaustionEntry`, replace two fields)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — update `build_candidate_plans` signature and all exhaustion read/write sites)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — if callers pass the two maps separately, update to pass unified cache)

## Out of Scope

- Adding `Serialize, Deserialize` to `ExhaustionEntry` or `AgentDecisionRuntime` (S30-002)
- Adding `invalidation` field to `ExhaustionEntry` (S31)
- Changing `EXHAUSTION_SKIP_TTL` value (S30-007)
- Save/load format changes (S30-003)
- Any behavioral change to exhaustion skip or backoff logic

## Acceptance Criteria

### Tests That Must Pass

1. All existing exhaustion-related unit tests in `planning.rs` and `search/tests.rs` pass with the unified struct
2. All golden tests: `cargo test -p worldwake-ai` (zero golden hash changes expected — pure refactoring)
3. Full workspace: `cargo test --workspace`
4. `cargo clippy --workspace` clean

### Invariants

1. Observable behavior is identical — same goals are skipped, same backoff budgets are applied
2. `ExhaustionEntry` keys and values cover exactly the same state as the two prior maps
3. No new ECS components introduced
4. Determinism preserved — `BTreeMap` ordering unchanged

## Test Plan

### New/Modified Tests

1. None expected — existing coverage should transfer. If any test directly constructs `search_exhausted_goals` or `exhaustion_counts` fields, update them to use `exhaustion_cache` with `ExhaustionEntry`.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace && cargo test --workspace`
