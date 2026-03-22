**Status**: PENDING

# S23: Refined Blocked Intents

## Summary

Refine `BlockedIntentMemory` keying from goal-level to compound-keyed failure records so that blocking at Place A no longer suppresses the same goal at Place B. Add a dedicated short TTL for `Unknown` blockers with diagnostic tracing. The existing `clear_resolved_blockers` per-tick mechanism is already proactive; this spec does not replace it but ensures compound-keyed blockers integrate with it correctly.

## Phase

Phase 3+: AI Architecture Refinement (post-E13)

## Crate

- `worldwake-core` (BlockedIntentMemory redesign)
- `worldwake-ai` (recording, lookup, failure handling)

## Dependencies

None strictly required. S20 (cleaner code surface) is helpful but not blocking.

## FOUNDATIONS Alignment

- **P3** (Concrete state over abstract scores): Blockers should carry concrete failure context (where, what target, what method), not suppress an entire goal category from a single failure at one location.
- **P7** (Locality of information): A failure observed at Place A is local information. It should not suppress the same goal at Place B, which the agent has not yet attempted.
- **P27** (Debuggability): `Unknown` blockers that silently suppress goals for 20 ticks with no diagnostic information are hard to trace. Every blocker should be traceable to a specific failure with context.

## Motivation

Three problems with the current `BlockedIntentMemory`:

### 1. Over-broad suppression

`BlockedIntentMemory` stores a `Vec<BlockedIntent>` where each entry has a `goal_key: GoalKey`. The `record()` method deduplicates by `goal_key` alone (retains entries with different goal keys, replaces the existing entry for the same goal key). The `is_blocked()` method checks only `goal_key` match.

This means: if an agent fails to harvest at OrchardFarm (resource depleted), `record()` stores a blocker with `goal_key = AcquireCommodity(Fruit, ...)`. On the next tick, `is_blocked()` matches that goal key and suppresses the harvest goal entirely -- even though GeneralStore's orchard has fruit available.

The `related_place` field already exists on `BlockedIntent` and is populated by `handle_plan_failure()`, but it is only used by `blocker_resolved()` for resolution checks -- it plays no role in keying or matching.

### 2. Unknown opacity

`BlockingFact::Unknown` gets `transient_block_ticks` (default 20 ticks). While 20 ticks is shorter than the structural TTL of 200, it still silently suppresses goals with zero diagnostic information. The fallback to `Unknown` in `derive_blocking_fact()` (line 140 of failure_handling.rs) means any unrecognized failure mode becomes opaque.

### 3. Record replacement loses concurrent blockers

Because `record()` retains only entries with a *different* `goal_key`, recording a new blocker for the same goal at a different place *replaces* the previous one. An agent that fails to harvest at Place A, then fails to harvest at Place B, only remembers the Place B failure. If Place A's resource regenerates first, the agent has no blocker for Place A to clear.

## Current State (Accurate)

```rust
// Storage: Vec<BlockedIntent> (not BTreeMap)
pub struct BlockedIntentMemory {
    pub intents: Vec<BlockedIntent>,
}

// Dedup: by goal_key only
pub fn record(&mut self, intent: BlockedIntent) {
    self.intents.retain(|existing| existing.goal_key != intent.goal_key);
    self.intents.push(intent);
}

// Lookup: by goal_key only
pub fn is_blocked(&self, key: &GoalKey, current_tick: Tick) -> bool {
    self.intents.iter().any(|intent| {
        intent.goal_key == *key
            && intent.expires_tick > current_tick
            && intent.blocks_goal_generation()
    })
}

// Already does NOT block generation for these two:
// BlockingFact::ExclusiveFacilityUnavailable | BlockingFact::SourceDepleted
pub const fn blocks_goal_generation(&self) -> bool { ... }

// TTL: Unknown gets transient_block_ticks (20), not structural (200)
fn blocking_fact_ttl(fact: BlockingFact, budget: &PlanningBudget) -> u32 { ... }
```

Proactive clearing already exists: `clear_resolved_blockers()` is called per-tick in `agent_tick.rs` (line 788). It runs `expire()` then `blocker_resolved()` which checks concrete state per variant.

## Design

### A. Compound Blocker Key

Introduce a `BlockerKey` struct that includes the failure location and target, so multiple blockers for the same goal at different places can coexist:

```rust
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BlockerKey {
    pub goal_key: GoalKey,
    pub place: Option<EntityId>,
    pub target: Option<EntityId>,
}
```

Change `BlockedIntentMemory`:

- **Storage**: Remains `Vec<BlockedIntent>`, but `BlockedIntent` gains a `blocker_key: BlockerKey` field (replacing the current `goal_key` field).
- **`record()`**: Deduplicates by `BlockerKey` (goal + place + target), not just `GoalKey`. Multiple blockers for the same goal at different places coexist.
- **`is_blocked()`**: Signature changes to `is_blocked(&self, key: &GoalKey, target: Option<EntityId>, place: Option<EntityId>, current_tick: Tick)`:
  - Exact match: a blocker with `(goal, Some(place), Some(target))` only matches queries with that same place and target.
  - Place-scoped: a blocker with `(goal, Some(place), None)` matches any query at that place.
  - Goal-scoped: a blocker with `(goal, None, None)` matches any query for that goal (for truly global failures like `NoKnownPath` with no specific place).
- **`clear_for()`**: Takes `BlockerKey` instead of `GoalKey`. A convenience `clear_all_for_goal()` method retains the old behavior for callers that need it.

### B. Candidate Generation Integration

`emit_candidate()` and `emit_candidate_with_trace()` in `candidate_generation.rs` currently call `blocked.is_blocked(&key, current_tick)` with only the `GoalKey`.

After the change, these functions must pass relevant target and place context:

- **Place-specific goals** (harvest, craft at a workstation): The `GroundedGoal` already has `evidence_places`. However, blocker checking happens *before* the grounded goal is fully assembled. The check should pass `None` for place/target at the `emit_candidate` level (since the goal is not yet place-bound), and instead the blocker check should happen later during plan search when the specific place is known.

This is a design choice with trade-offs:

1. **Option A: Keep blocking at candidate generation, match loosely.** `is_blocked(goal_key, None, None)` at candidate generation still suppresses goals that have a global (place=None) blocker. Place-specific blockers are checked during plan search when the step targets a specific place. Simpler change, fewer callers to update.

2. **Option B: Move all blocker checking to plan search.** Remove `is_blocked` calls from candidate generation entirely. Blockers filter out plan steps in the search, not entire goals. More precise but requires `search_plan` to query blockers per step.

3. **Recommendation: Option A.** Keep the candidate-generation filter for global blockers (NoKnownPath, DangerTooHigh, CombatTooRisky) which are not place-specific. Place-scoped blockers (SourceDepleted, WorkstationBusy, etc.) should be checked during plan search when the agent evaluates a specific step at a specific place. This is consistent with the existing `blocks_goal_generation()` carve-out for `SourceDepleted` and `ExclusiveFacilityUnavailable`.

### C. Failure Recording Narrowing

`handle_plan_failure()` already extracts `related_entity` and `related_place` from the failed step. The change: populate `BlockerKey { goal_key, place: related_place, target: related_entity }` instead of keying by `goal_key` alone.

The `related_place()` helper in `failure_handling.rs` already returns the agent's effective place or the step's target place depending on the op kind. This becomes the `place` in `BlockerKey`.

### D. Unknown Blocker Reform

- **Reduce TTL**: Add a new `PlanningBudget` field `unknown_block_ticks: u32` with a default of 5 ticks. `blocking_fact_ttl` uses this for `Unknown` instead of `transient_block_ticks`.
- **Diagnostic context**: When `derive_blocking_fact()` falls through to `Unknown`, record the failed step's `op_kind` and `action_def_id` in the `BlockedIntent` via a new `diagnostic_context: Option<BlockerDiagnostic>` field:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerDiagnostic {
    pub op_kind: PlannerOpKind,
    pub action_def: Option<ActionDefId>,
}
```

- When `DecisionTraceSink` is active, emit a trace event recording the Unknown blocker with its diagnostic context so developers can identify the root cause and add a proper `BlockingFact` variant.

### E. blocker_resolved Integration

`blocker_resolved()` already uses `intent.related_entity` and `intent.related_place` for resolution checks. After compound keying, these fields move into the `BlockerKey`. The resolution logic accesses them from `intent.blocker_key.target` and `intent.blocker_key.place` instead.

No behavioral change to the resolution logic itself -- it already checks concrete state per variant.

## Tickets

### S23-001: Introduce BlockerKey and refactor BlockedIntentMemory

- Add `BlockerKey` struct to `blocked_intent.rs`
- Replace `goal_key: GoalKey` on `BlockedIntent` with `blocker_key: BlockerKey`
- Move `related_entity` and `related_place` from `BlockedIntent` fields into `BlockerKey.target` and `BlockerKey.place`
- Update `record()` to dedup by `BlockerKey`
- Update `is_blocked()` signature: `is_blocked(&self, key: &GoalKey, target: Option<EntityId>, place: Option<EntityId>, current_tick: Tick)`
- Implement matching semantics: exact > place-scoped > goal-scoped
- Update `clear_for()` to take `&BlockerKey`; add `clear_all_for_goal(&GoalKey)`
- Update `blocker_resolved()` to read target/place from `blocker_key`
- Update all existing unit tests in `blocked_intent.rs`
- Verify: `cargo test -p worldwake-core`

### S23-002: Update failure_handling.rs for compound blocker recording

- `handle_plan_failure()`: construct `BlockerKey` from `goal_key` + `related_place()` + `related_entity()`
- Remove `related_entity` and `related_place` fields from `BlockedIntent` (now in `BlockerKey`)
- Keep `related_action: Option<ActionDefId>` on `BlockedIntent` as-is (not part of key)
- Update `clear_resolved_blockers()` to access entity/place from `blocker_key`
- Update all failure_handling unit tests
- Verify: `cargo test -p worldwake-ai`

### S23-003: Update candidate generation for compound blocker lookup

- `emit_candidate()` and `emit_candidate_with_trace()`: pass `None, None` for target/place (global check only)
- Global blockers (NoKnownPath, DangerTooHigh, CombatTooRisky, Unknown) continue to suppress at candidate generation
- Place-specific blockers no longer suppress at candidate generation (they have `place: Some(...)` so they do not match `(goal, None, None)` queries)
- This is consistent with the existing `blocks_goal_generation()` carve-out for SourceDepleted and ExclusiveFacilityUnavailable
- Verify: `cargo test -p worldwake-ai` -- all golden tests pass

### S23-004: Add blocker check to plan search for place-specific blockers

- In `search_plan()`, when evaluating a step at a specific place, check `is_blocked(goal_key, step_target, step_place, current_tick)` against place-scoped blockers
- Steps blocked by a place-specific blocker are pruned from the search (not expanded)
- This means: "harvest fruit" goal is still generated as a candidate, but the plan search skips OrchardFarm (blocked, depleted) and finds GeneralStore instead
- Pass `BlockedIntentMemory` reference into the search context
- Verify: `cargo test -p worldwake-ai`

### S23-005: Reform Unknown blocker TTL and diagnostics

- Add `unknown_block_ticks: u32` field to `PlanningBudget` with default 5
- Update `blocking_fact_ttl()` to use `budget.unknown_block_ticks` for `Unknown`
- Add `BlockerDiagnostic` struct to `blocked_intent.rs`
- Add `diagnostic_context: Option<BlockerDiagnostic>` field to `BlockedIntent`
- `handle_plan_failure()`: populate diagnostic when blocking fact is `Unknown`
- When `DecisionTraceSink` is active, emit diagnostic trace event for Unknown blockers
- Update `planning_budget_default_matches_ticket_values` test
- Verify: `cargo test -p worldwake-core` and `cargo test -p worldwake-ai`

### S23-006: Workspace verification

- `cargo test --workspace` -- all pass
- `cargo clippy --workspace` -- no new warnings
- Golden tests where blockers are exercised still pass
- Confirm: harvesting failure at Place A no longer blocks harvesting at Place B

## FND-01 Section H Analysis

### Information-path analysis

Blocked intents are private agent state. No other agent reads another agent's blockers. Blockers are recorded by the agent's own failure handling pipeline (`handle_plan_failure` in agent_tick) and cleared by the per-tick `clear_resolved_blockers` call in the same pipeline. The clearing function queries the agent's own belief view (`RuntimeBeliefView`) to check whether the blocking condition persists.

Information path for blocker creation: action execution fails -> `PlanFailureContext` captures failed step details -> `derive_blocking_fact` classifies the failure -> `handle_plan_failure` records `BlockedIntent` with `BlockerKey` derived from the failed step's target and place.

Information path for blocker clearing: per-tick `clear_resolved_blockers` -> `blocker_resolved` queries belief view for concrete state (resource availability, entity presence, path existence) -> removes blocker if condition no longer holds.

No information crosses agent boundaries. No system writes to another agent's blocker memory.

### Positive-feedback analysis

No amplifying loops. Blockers are purely dampening: they suppress goal candidates or plan steps, reducing action attempts. The clearing mechanism (both TTL and proactive) ensures dampening does not persist indefinitely.

One potential concern: if blocker clearing is too aggressive (clears blocker, agent retries, fails again, records blocker, clears again), this creates a retry oscillation. However, this is bounded by the tick cost of attempting the action (travel time + action duration), not by the blocker system itself.

### Concrete dampeners

- **TTL expiry**: Every blocker has a finite `expires_tick`. Unknown: 5 ticks. Transient: 20 ticks. Structural: 200 ticks.
- **Proactive clearing**: `blocker_resolved()` checks concrete world state per variant (resource quantity > 0, entity alive, path exists). This is a physical-world process, not a numeric clamp.
- **Retry cost**: Even without blockers, attempting an action costs travel time and action ticks, naturally limiting retry frequency.

### Stored state vs. derived read-model list

**Stored (authoritative)**:
- `BlockedIntentMemory` component: `Vec<BlockedIntent>` with compound `BlockerKey`
- Each `BlockedIntent` stores: `blocker_key`, `blocking_fact`, `related_action`, `diagnostic_context`, `observed_tick`, `expires_tick`

**Derived (transient)**:
- `is_blocked()` query result: derived from stored blockers + current tick for expiry check + matching semantics
- `blocker_resolved()` result: derived from stored blocker + current belief view state
- `blocking_fact_ttl()`: pure function of `BlockingFact` variant + `PlanningBudget` parameters

## Verification

1. `cargo test --workspace` -- all pass
2. `cargo clippy --workspace` -- no new warnings
3. Golden tests where blockers are exercised (any scenario with plan failures, StartFailed outcomes, or multi-location resource competition) still pass
4. Harvesting failure at Place A no longer blocks harvesting at Place B (verifiable in golden tests with two resource sources)
5. Unknown blockers expire in 5 ticks, not 20
6. Decision traces for Unknown blockers include diagnostic context (op_kind, action_def)
