**Status**: PENDING

# S23: Refined Blocked Intents

## Summary

Refine `BlockedIntentMemory` keying from goal-level to compound-keyed failure records so that blocking at Place A no longer suppresses the same goal at Place B. Introduce `BTreeMap<BlockerKey, BlockedIntent>` storage with tiered matching semantics (exact > place+target > place > goal-scoped). Add place-scoped blocker pruning in plan search so agents route around blocked locations. Reform `Unknown` blockers with a dedicated short TTL and diagnostic tracing.

The existing `clear_resolved_blockers` per-tick mechanism is already proactive; this spec does not replace it but ensures compound-keyed blockers integrate with it correctly.

## Phase

Phase 3+: AI Architecture Refinement (post-E13)

## Crate

- `worldwake-core` (BlockedIntentMemory redesign, BlockerKey, BlockerDiagnostic)
- `worldwake-ai` (recording, lookup, failure handling, search pruning, trace integration)

## Dependencies

- S21 (Promote Causal Runtime State) — COMPLETED. `handle_plan_failure()` now takes `jc: &mut Option<JourneyCommitment>`. S23 changes are additive; the `jc` parameter is unchanged.
- S20 (cleaner code surface) is helpful but not blocking.

## FOUNDATIONS Alignment

- **P3** (Concrete state over abstract scores): Blockers carry concrete failure context (where, what target, what action, what method). The `BlockerKey` encodes the specific place, target entity, and action definition that failed — not just the goal category.
- **P7** (Locality of information): A failure observed at Place A is local information. It suppresses only actions at Place A, not the same goal at Place B, which the agent has not yet attempted. Place-scoped blockers are checked during plan search, not at candidate generation.
- **P27** (Debuggability): `Unknown` blockers carry diagnostic context (`BlockerDiagnostic`) with the failed action definition. When `DecisionTraceSink` is active, Unknown blockers emit `UnknownBlockerTrace` events including `PlannerOpKind` so developers can identify the root cause and add a proper `BlockingFact` variant. Place-scoped blocker pruning in search is recorded via `PlaceBlocker` filter reasons in the search expansion trace.

## Motivation

Three problems with the current `BlockedIntentMemory`:

### 1. Over-broad suppression

`BlockedIntentMemory` stores a `Vec<BlockedIntent>` where each entry has a `goal_key: GoalKey`. The `record()` method deduplicates by `goal_key` alone (retains entries with different goal keys, replaces the existing entry for the same goal key). The `is_blocked()` method checks only `goal_key` match.

This means: if an agent fails to harvest at OrchardFarm (resource depleted), `record()` stores a blocker with `goal_key = AcquireCommodity(Fruit, ...)`. On the next tick, `is_blocked()` matches that goal key and suppresses the harvest goal entirely — even though GeneralStore's orchard has fruit available.

The `related_place` field already exists on `BlockedIntent` and is populated by `handle_plan_failure()`, but it is only used by `blocker_resolved()` for resolution checks — it plays no role in keying or matching.

### 2. Unknown opacity

`BlockingFact::Unknown` gets `transient_block_ticks` (default 20 ticks). While 20 ticks is shorter than the structural TTL of 200, it still silently suppresses goals with zero diagnostic information. The fallback to `Unknown` in `derive_blocking_fact()` means any unrecognized failure mode becomes opaque.

### 3. Record replacement loses concurrent blockers

Because `record()` retains only entries with a *different* `goal_key`, recording a new blocker for the same goal at a different place *replaces* the previous one. An agent that fails to harvest at Place A, then fails to harvest at Place B, only remembers the Place B failure. If Place A's resource regenerates first, the agent has no blocker for Place A to clear.

### 4. Action collisions at same location (NEW)

Two different action types can fail at the same place for the same target (e.g., "trade apples for coin" vs "trade apples for cloth"). Without action identity in the key, recording the second replaces the first. The blocker key must include `ActionDefId` to fully disambiguate.

## Current State (Accurate)

```rust
// crates/worldwake-core/src/blocked_intent.rs

// Storage: Vec<BlockedIntent> (not BTreeMap)
pub struct BlockedIntentMemory {
    pub intents: Vec<BlockedIntent>,
}

// Fields: goal_key is the only key; entity/place/action are contextual only
pub struct BlockedIntent {
    pub goal_key: GoalKey,
    pub blocking_fact: BlockingFact,
    pub related_entity: Option<EntityId>,
    pub related_place: Option<EntityId>,
    pub related_action: Option<ActionDefId>,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
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

Proactive clearing already exists: `clear_resolved_blockers()` is called per-tick in `agent_tick.rs`. It runs `expire()` then `blocker_resolved()` which checks concrete state per variant.

`handle_plan_failure()` already takes `jc: &mut Option<JourneyCommitment>` (added by S21).

## Design

### A. Compound Blocker Key

Introduce a `BlockerKey` struct that includes the failure location, target, and action, so multiple blockers for the same goal at different places/actions can coexist:

```rust
// crates/worldwake-core/src/blocked_intent.rs

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct BlockerKey {
    pub goal_key: GoalKey,
    pub place: Option<EntityId>,
    pub target: Option<EntityId>,
    pub action_def: Option<ActionDefId>,
}
```

All constituent types already derive `Ord`: `GoalKey` (goal.rs:65), `EntityId` (ids.rs macro), `ActionDefId` (ids.rs macro).

### B. BlockedIntent Simplified

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockedIntent {
    pub blocker_key: BlockerKey,
    pub blocking_fact: BlockingFact,
    pub diagnostic_context: Option<BlockerDiagnostic>,
    pub observed_tick: Tick,
    pub expires_tick: Tick,
}
```

Fields removed: `goal_key`, `related_entity`, `related_place`, `related_action` — all subsumed by `blocker_key` and `diagnostic_context`.

`blocks_goal_generation()` is unchanged — still returns `false` only for `ExclusiveFacilityUnavailable` and `SourceDepleted`.

### C. BlockerDiagnostic

```rust
// crates/worldwake-core/src/blocked_intent.rs

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerDiagnostic {
    pub action_def: ActionDefId,
}
```

Only `action_def` is stored on the component (in `worldwake-core`). `PlannerOpKind` is an AI-layer concept that lives only in the trace event (in `worldwake-ai`), not in stored state. This keeps the core crate dependency-free.

### D. BTreeMap Storage

```rust
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockedIntentMemory {
    pub intents: BTreeMap<BlockerKey, BlockedIntent>,
}
```

`BTreeMap` provides deterministic iteration (project invariant: `BTreeMap`/`BTreeSet` only in authoritative state) and O(log n) lookup by exact key.

### E. Updated Methods

**`record()`** — direct insert by compound key:

```rust
pub fn record(&mut self, intent: BlockedIntent) {
    self.intents.insert(intent.blocker_key, intent);
}
```

Multiple blockers for the same goal at different places/targets/actions coexist naturally.

**`is_blocked()`** — tiered matching for candidate generation:

```rust
pub fn is_blocked(
    &self,
    goal_key: &GoalKey,
    place: Option<EntityId>,
    target: Option<EntityId>,
    action_def: Option<ActionDefId>,
    current_tick: Tick,
) -> bool {
    self.intents.values().any(|intent| {
        intent.blocker_key.goal_key == *goal_key
            && intent.expires_tick > current_tick
            && intent.blocks_goal_generation()
            && matches_scope(&intent.blocker_key, place, target, action_def)
    })
}
```

**`is_blocked_for_search()`** — tiered matching without `blocks_goal_generation()` gate, for search pruning where ALL blockers are relevant (including `SourceDepleted` and `ExclusiveFacilityUnavailable`):

```rust
pub fn is_blocked_for_search(
    &self,
    goal_key: &GoalKey,
    place: Option<EntityId>,
    target: Option<EntityId>,
    action_def: Option<ActionDefId>,
    current_tick: Tick,
) -> bool {
    self.intents.values().any(|intent| {
        intent.blocker_key.goal_key == *goal_key
            && intent.expires_tick > current_tick
            && matches_scope(&intent.blocker_key, place, target, action_def)
    })
}
```

**`matches_scope()`** — tiered matching helper:

```rust
fn matches_scope(
    blocker: &BlockerKey,
    query_place: Option<EntityId>,
    query_target: Option<EntityId>,
    query_action: Option<ActionDefId>,
) -> bool {
    // Goal-scoped blocker (place=None, target=None, action=None) matches everything
    if blocker.place.is_none() && blocker.target.is_none() && blocker.action_def.is_none() {
        return true;
    }
    // Place must match if blocker has one
    if let Some(blocker_place) = blocker.place {
        if query_place != Some(blocker_place) {
            return false;
        }
    }
    // Target must match if blocker has one
    if let Some(blocker_target) = blocker.target {
        if query_target != Some(blocker_target) {
            return false;
        }
    }
    // Action must match if blocker has one
    if let Some(blocker_action) = blocker.action_def {
        if query_action != Some(blocker_action) {
            return false;
        }
    }
    true
}
```

Matching hierarchy:
- `(goal, None, None, None)` → matches any query for that goal (global blockers like `NoKnownPath`)
- `(goal, Some(place), None, None)` → matches any query at that place
- `(goal, Some(place), Some(target), None)` → matches queries at that place+target
- `(goal, Some(place), Some(target), Some(action))` → exact match only

**`expire()`** — unchanged semantics, BTreeMap retain:

```rust
pub fn expire(&mut self, current_tick: Tick) {
    self.intents.retain(|_, intent| intent.expires_tick > current_tick);
}
```

**`clear_for()` and `clear_all_for_goal()`**:

```rust
pub fn clear_for(&mut self, key: &BlockerKey) {
    self.intents.remove(key);
}

pub fn clear_all_for_goal(&mut self, goal_key: &GoalKey) {
    self.intents.retain(|k, _| k.goal_key != *goal_key);
}
```

### F. Candidate Generation Integration

`emit_candidate()` and `emit_candidate_with_trace()` in `candidate_generation.rs` currently call `blocked.is_blocked(&key, current_tick)` with only the `GoalKey`.

After the change, these functions call `blocked.is_blocked(&key, None, None, None, current_tick)`:

- This passes `None` for place, target, and action — a global-only check.
- Global blockers (`NoKnownPath`, `DangerTooHigh`, `CombatTooRisky`, `Unknown`) have `place: None` in their `BlockerKey`, so they match `(goal, None, None, None)` queries and continue to suppress at candidate generation.
- Place-specific blockers (`SourceDepleted`, `WorkstationBusy`, etc.) have `place: Some(...)` in their `BlockerKey`, so they do NOT match `(goal, None, None, None)` queries — the goal is still generated as a candidate.
- This is consistent with the existing `blocks_goal_generation()` carve-out for `SourceDepleted` and `ExclusiveFacilityUnavailable`, but now the carve-out is structural (driven by the key's place field) rather than per-variant.

### G. Plan Search Pruning

This is where place-specific blockers take effect. The goal candidate is generated, but the plan search prunes specific locations that are blocked.

**`search_plan()` signature change:**

```rust
// crates/worldwake-ai/src/search/mod.rs

pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
    budget: &PlanningBudget,
    recipes: &RecipeRegistry,
    blocked: &BlockedIntentMemory,  // NEW
    mut binding_rejections: Option<&mut Vec<BindingRejection>>,
    mut expansion_summaries: Option<&mut Vec<SearchExpansionSummary>>,
) -> PlanSearchResult
```

This propagates to `search_candidates()` which gains the same `blocked` parameter.

**Callers to update** (4 call sites):
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-ai/src/search/tests.rs`
- `crates/worldwake-ai/src/goal_model.rs`

**Pruning logic in `search_candidates()`:**

After the binding check passes for each candidate and before pushing to the filtered list, check place-scoped blockers:

```rust
fn is_candidate_blocked(
    candidate: &SearchCandidate,
    goal: &GroundedGoal,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    blocked: &BlockedIntentMemory,
    current_tick: Tick,
) -> bool {
    let place = candidate_action_place(candidate, node, semantics_table);
    let target = candidate.authoritative_targets.first().copied();
    blocked.is_blocked_for_search(
        &goal.key,
        place,
        target,
        Some(candidate.def_id),
        current_tick,
    )
}
```

**Place extraction:**

```rust
fn candidate_action_place(
    candidate: &SearchCandidate,
    node: &SearchNode<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> Option<EntityId> {
    let semantics = semantics_table.get(&candidate.def_id)?;
    match semantics.op_kind {
        // Travel target IS the destination place
        PlannerOpKind::Travel => candidate.authoritative_targets.first().copied(),
        // All other actions happen at the actor's current simulated place
        _ => node.state.effective_place_ref(
            PlanningEntityRef::Authoritative(node.state.snapshot().actor()),
        ),
    }
}
```

**Trace integration** — add `PlaceBlocker` variant to `RootCandidateOutcome` / filter reasons in `decision_trace.rs`:

```rust
// crates/worldwake-ai/src/decision_trace.rs
// (Add to existing filter/outcome enum)

PlaceBlocker {
    place: Option<EntityId>,
    blocking_fact: BlockingFact,
},
```

When a candidate is pruned by a place-scoped blocker, the trace records the place and blocking fact so `dump_agent()` can explain "why didn't the agent try harvesting at OrchardFarm? → SourceDepleted blocker at OrchardFarm."

**Behavioral result:** "harvest fruit" goal is still generated as a candidate. Plan search generates candidates for all known places with fruit. OrchardFarm candidates are pruned (blocked, depleted). GeneralStore candidates pass. The agent plans to harvest at GeneralStore instead.

### H. Failure Recording Narrowing

`handle_plan_failure()` in `failure_handling.rs` already extracts `related_entity` and `related_place` from the failed step. The change: construct `BlockerKey` from these fields plus the action definition:

```rust
let blocker_key = BlockerKey {
    goal_key: context.goal_key,
    place: related_place(context.view, context.agent, &context.goal_key, context.failed_step),
    target: related_entity(context.failed_step),
    action_def: Some(context.failed_step.def_id),
};
```

The `related_place()` helper already returns the agent's effective place or the step's target place depending on the op kind. The `related_entity()` helper already extracts the first target. These move into `BlockerKey` fields instead of standalone `BlockedIntent` fields.

### I. blocker_resolved Integration

`blocker_resolved()` already uses `intent.related_entity` and `intent.related_place` for resolution checks. After compound keying, these fields move into the `BlockerKey`. The resolution logic accesses them from `intent.blocker_key.target` and `intent.blocker_key.place` instead:

```rust
fn blocker_resolved(view: &dyn RuntimeBeliefView, agent: EntityId, intent: &BlockedIntent) -> bool {
    match intent.blocking_fact {
        BlockingFact::NoKnownPath => {
            let Some(target_place) = intent.blocker_key.place else { return false; };
            // ... rest unchanged, uses target_place
        }
        BlockingFact::SellerOutOfStock => {
            let Some(seller) = intent.blocker_key.target else { return false; };
            // ... rest unchanged, uses seller
        }
        // ... etc — all variants access blocker_key.target / blocker_key.place
    }
}
```

`clear_resolved_blockers()` uses `BTreeMap::retain`:

```rust
pub fn clear_resolved_blockers(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocked_memory: &mut BlockedIntentMemory,
    current_tick: Tick,
) {
    blocked_memory.expire(current_tick);
    blocked_memory.intents.retain(|_, intent| !blocker_resolved(view, agent, intent));
}
```

No behavioral change to the resolution logic itself — it already checks concrete state per variant.

### J. Unknown Blocker Reform

**Reduced TTL**: Add a new `PlanningBudget` field:

```rust
// crates/worldwake-ai/src/budget.rs
pub unknown_block_ticks: u32,  // default: 5
```

`blocking_fact_ttl()` uses this for `Unknown` instead of `transient_block_ticks`:

```rust
BlockingFact::Unknown => budget.unknown_block_ticks,
```

**Diagnostic context on stored state**: When `derive_blocking_fact()` returns `Unknown`, populate `diagnostic_context`:

```rust
let diagnostic = if matches!(blocking_fact, BlockingFact::Unknown) {
    Some(BlockerDiagnostic {
        action_def: context.failed_step.def_id,
    })
} else {
    None
};
```

**Trace event for Unknown blockers**: Add to `decision_trace.rs`:

```rust
#[derive(Clone, Debug)]
pub struct UnknownBlockerTrace {
    pub goal_key: GoalKey,
    pub failed_action_def: ActionDefId,
    pub op_kind: PlannerOpKind,
    pub targets: Vec<PlanningEntityRef>,
    pub place: Option<EntityId>,
}
```

Add to `PlanningPipelineTrace`:

```rust
pub unknown_blockers: Vec<UnknownBlockerTrace>,
```

**Emission point**: In `agent_tick` (which has access to `DecisionTraceSink`), after calling `handle_plan_failure()`, if the blocking fact was `Unknown` and tracing is active, push an `UnknownBlockerTrace` to the current tick's `PlanningPipelineTrace.unknown_blockers`.

**`dump_agent()` integration**: When printing planning traces, also emit unknown blocker details:

```
  Unknown blockers recorded:
    goal=AcquireCommodity(Bread) action=adef42 op=Trade place=Some(eid5)
```

## Tickets

### S23-001: Introduce BlockerKey, BlockerDiagnostic, and refactor BlockedIntentMemory to BTreeMap

**File**: `crates/worldwake-core/src/blocked_intent.rs`

- Add `BlockerKey` struct with `goal_key: GoalKey`, `place: Option<EntityId>`, `target: Option<EntityId>`, `action_def: Option<ActionDefId>`. Derive `Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize`.
- Add `BlockerDiagnostic` struct with `action_def: ActionDefId`. Derive `Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize`.
- Replace `Vec<BlockedIntent>` with `BTreeMap<BlockerKey, BlockedIntent>` on `BlockedIntentMemory`.
- Replace `goal_key`, `related_entity`, `related_place`, `related_action` on `BlockedIntent` with `blocker_key: BlockerKey` and `diagnostic_context: Option<BlockerDiagnostic>`.
- `record()`: `self.intents.insert(intent.blocker_key, intent)`
- `is_blocked()`: new signature `(goal_key, place, target, action_def, current_tick)` with tiered `matches_scope()`.
- Add `is_blocked_for_search()`: same as `is_blocked()` but without `blocks_goal_generation()` check.
- `expire()`: `self.intents.retain(|_, i| i.expires_tick > current_tick)`
- `clear_for(&BlockerKey)`: `self.intents.remove(key)`
- Add `clear_all_for_goal(&GoalKey)`: `self.intents.retain(|k, _| k.goal_key != *goal_key)`
- Update all existing unit tests in `blocked_intent.rs`
- Verify: `cargo test -p worldwake-core`

### S23-002: Update failure_handling.rs for compound blocker recording

**File**: `crates/worldwake-ai/src/failure_handling.rs`

- `handle_plan_failure()`: construct `BlockerKey` from `goal_key` + `related_place()` + `related_entity()` + `Some(failed_step.def_id)`. Populate `diagnostic_context` when blocking fact is `Unknown`.
- `blocker_resolved()`: read target/place from `intent.blocker_key.target` / `intent.blocker_key.place` instead of `intent.related_entity` / `intent.related_place`.
- `clear_resolved_blockers()`: update `.retain()` for `BTreeMap` signature `retain(|_, intent| ...)`.
- Update all failure_handling unit tests.
- Verify: `cargo test -p worldwake-ai`

### S23-003: Update candidate generation for compound blocker lookup

**File**: `crates/worldwake-ai/src/candidate_generation.rs`

- `emit_candidate()`: change `blocked.is_blocked(&key, current_tick)` to `blocked.is_blocked(&key, None, None, None, current_tick)` — global-only check.
- `emit_candidate_with_trace()`: same change.
- No behavioral change for global blockers (`NoKnownPath`, `DangerTooHigh`, `CombatTooRisky`, `Unknown`).
- Place-specific blockers no longer suppress at candidate generation (they have `place: Some(...)` so they do not match `(goal, None, None, None)` queries).
- Verify: `cargo test -p worldwake-ai` — all golden tests pass.

### S23-004: Add blocker check to plan search for place-specific blockers

**Files**:
- `crates/worldwake-ai/src/search/mod.rs` — add `blocked: &BlockedIntentMemory` parameter to `search_plan()`
- `crates/worldwake-ai/src/search/candidates.rs` — add `blocked` parameter to `search_candidates()`, implement `is_candidate_blocked()` and `candidate_action_place()`
- `crates/worldwake-ai/src/decision_trace.rs` — add `PlaceBlocker { place, blocking_fact }` variant to the root candidate outcome/filter enum
- **Callers** (pass `blocked` through):
  - `crates/worldwake-ai/src/agent_tick/planning.rs`
  - `crates/worldwake-ai/src/agent_tick/tests.rs`
  - `crates/worldwake-ai/src/search/tests.rs`
  - `crates/worldwake-ai/src/goal_model.rs`

Implementation:
- In `search_candidates()`, after binding check passes and before pushing to filtered list, call `is_candidate_blocked()` which uses `blocked.is_blocked_for_search()` (no `blocks_goal_generation` filter — at search level, ALL blockers prune).
- `candidate_action_place()` resolves place: Travel → target place from targets; all others → actor's simulated place from `PlanningState::effective_place_ref`.
- When pruned, record `PlaceBlocker` filter reason in trace.
- Verify: `cargo test -p worldwake-ai`

### S23-005: Reform Unknown blocker TTL and diagnostics

**Files**:
- `crates/worldwake-ai/src/budget.rs` — add `unknown_block_ticks: u32` with default 5
- `crates/worldwake-ai/src/failure_handling.rs` — update `blocking_fact_ttl()` for `Unknown` to use `budget.unknown_block_ticks`
- `crates/worldwake-ai/src/decision_trace.rs` — add `UnknownBlockerTrace` struct, add `unknown_blockers: Vec<UnknownBlockerTrace>` field to `PlanningPipelineTrace`, integrate into `dump_agent()` and `summary()`
- `crates/worldwake-ai/src/agent_tick/mod.rs` — emit `UnknownBlockerTrace` after `handle_plan_failure()` when blocking fact is `Unknown` and tracing is active

Implementation:
- `PlanningBudget` gains `unknown_block_ticks: u32` (default 5, separate from `transient_block_ticks` which remains 20).
- `blocking_fact_ttl()` maps `BlockingFact::Unknown => budget.unknown_block_ticks`.
- Update `planning_budget_default_matches_ticket_values` test.
- Verify: `cargo test -p worldwake-core` and `cargo test -p worldwake-ai`

### S23-006: Workspace verification

- `cargo test --workspace` — all pass
- `cargo clippy --workspace` — no new warnings
- Golden tests where blockers are exercised still pass
- Confirm: harvesting failure at Place A no longer blocks harvesting at Place B
- Unknown blockers expire in 5 ticks, not 20
- Decision traces for Unknown blockers include diagnostic context

## FND-01 Section H Analysis

### Information-path analysis

Blocked intents are private agent state. No other agent reads another agent's blockers. Blockers are recorded by the agent's own failure handling pipeline (`handle_plan_failure` in agent_tick) and cleared by the per-tick `clear_resolved_blockers` call in the same pipeline. The clearing function queries the agent's own belief view (`RuntimeBeliefView`) to check whether the blocking condition persists.

Information path for blocker creation: action execution fails → `PlanFailureContext` captures failed step details → `derive_blocking_fact` classifies the failure → `handle_plan_failure` records `BlockedIntent` with `BlockerKey` derived from the failed step's target, place, and action definition.

Information path for blocker clearing: per-tick `clear_resolved_blockers` → `blocker_resolved` queries belief view for concrete state (resource availability, entity presence, path existence) → removes blocker if condition no longer holds.

No information crosses agent boundaries. No system writes to another agent's blocker memory. The `ActionDefId` in `BlockerKey` is derived from the failed step's `def_id` — it does not introduce cross-agent information flow.

### Positive-feedback analysis

No amplifying loops. Blockers are purely dampening: they suppress goal candidates or plan search steps, reducing action attempts. The clearing mechanism (both TTL and proactive) ensures dampening does not persist indefinitely.

One potential concern: if blocker clearing is too aggressive (clears blocker, agent retries, fails again, records blocker, clears again), this creates a retry oscillation. However, this is bounded by the tick cost of attempting the action (travel time + action duration), not by the blocker system itself.

### Concrete dampeners

- **TTL expiry**: Every blocker has a finite `expires_tick`. Unknown: 5 ticks (new). Transient: 20 ticks. Structural: 200 ticks.
- **Proactive clearing**: `blocker_resolved()` checks concrete world state per variant (resource quantity > 0, entity alive, path exists). This is a physical-world process, not a numeric clamp.
- **Retry cost**: Even without blockers, attempting an action costs travel time and action ticks, naturally limiting retry frequency.

### Stored state vs. derived read-model list

**Stored (authoritative)**:
- `BlockedIntentMemory` component: `BTreeMap<BlockerKey, BlockedIntent>`
- Each `BlockerKey` stores: `goal_key`, `place`, `target`, `action_def`
- Each `BlockedIntent` stores: `blocker_key`, `blocking_fact`, `diagnostic_context`, `observed_tick`, `expires_tick`

**Derived (transient)**:
- `is_blocked()` / `is_blocked_for_search()` query result: derived from stored blockers + current tick for expiry check + tiered matching semantics
- `blocker_resolved()` result: derived from stored blocker + current belief view state
- `blocking_fact_ttl()`: pure function of `BlockingFact` variant + `PlanningBudget` parameters
- `matches_scope()`: pure function of blocker key fields + query parameters
- `candidate_action_place()`: derived from candidate targets + actor's simulated position in planning state
- `UnknownBlockerTrace`: transient trace event, not persisted — emitted only when `DecisionTraceSink` is active

## Verification

1. `cargo test --workspace` — all pass
2. `cargo clippy --workspace` — no new warnings
3. Golden tests where blockers are exercised (any scenario with plan failures, StartFailed outcomes, or multi-location resource competition) still pass
4. Harvesting failure at Place A no longer blocks harvesting at Place B (verifiable in golden tests with two resource sources)
5. Unknown blockers expire in 5 ticks, not 20
6. Decision traces for Unknown blockers include diagnostic context (action_def)
7. Search trace shows `PlaceBlocker` filter reasons when candidates are pruned
