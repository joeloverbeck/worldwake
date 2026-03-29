# S37: Cooldown-Based Exhaustion

## Summary

Replace budget-halving exhaustion backoff with cooldown-based retry. Currently `effective_max_expansions()` halves the search budget on each consecutive failure (224→112→56→28→16). This makes search *shallower* on repeated failure — the wrong shape for a world where resources are temporarily blocked. Agents should retry at full search depth, just less frequently.

## Source

Derived from ChatGPT architecture review WW-AI-004 (Repair-first replanning, blocker scope, and deterministic barrier continuation), filtered to the exhaustion shape component only. `BlockingFact` enrichment is deferred to a separate spec — the existing 16 variants require a proper audit before extension. Plan repair and search memo reuse are also deferred as optimizations without correctness impact.

## Phase

Phase 3+: AI Architecture Overhaul, Step 13.5 Wave 5

## Crates

- `worldwake-ai` (exhaustion logic, failure handling, decision runtime, planning loop, decision trace)

## Dependencies

- S31 ✅ (goal-aware exhaustion invalidation — provides `ExhaustionEntry` structure and invalidation conditions this spec modifies)
- S33 ✅ (opportunity-scoped goal identity — exhaustion keyed by `OpportunityKey`; this spec changes retry semantics within that key)

## FOUNDATIONS Alignment

- **P18** (Resource-Bounded Practical Reasoning): Agents should reason within bounded time/knowledge, but repeated failure should not *degrade* reasoning quality. Retrying less often is bounded; searching more shallowly is degraded.
- **P2** (No Ungrounded Triggers): Cooldown parameters are profile-driven via `PlanningBudget`, not hardcoded constants. Per-agent tuning avoids magic numbers.
- **P3** (Concrete State Over Abstract Scores): Cooldown parameters are concrete, traceable per-agent properties, not abstract globals.
- **P20** (Agent Diversity Through Concrete Variation): Different agents can have different retry patience via `PlanningBudget` configuration — a cautious agent might wait longer between retries; an aggressive one retries quickly.
- **P27** (Debuggability): Decision traces log cooldown state and retry eligibility per opportunity per tick, making exhaustion behavior inspectable.

## Design Goals

1. **Full-depth retry**: Every retry attempt uses the full `max_node_expansions` budget. Search quality never degrades.
2. **Increasing cooldown**: Consecutive failures increase the delay before the next retry, not the search budget.
3. **Profile-driven**: Cooldown parameters live in `PlanningBudget`, not as hardcoded constants. This enables per-agent diversity (P20) and avoids magic numbers (P2).
4. **Invalidation removes entry**: S31 invalidation conditions (state changes) continue to remove entries entirely from the cache — fresh evidence deserves immediate full-budget search, and empty entries are dead state (P26).
5. **Deterministic**: Cooldown ticks are deterministic tick arithmetic, not wall-clock based.
6. **Efficient planning trigger**: `has_pending_budget_retry()` checks cooldown eligibility so agents don't enter the planning loop only to skip all exhausted goals.

## Current Shape

```rust
// decision_runtime.rs:105-109
pub fn effective_max_expansions(&self, base: u16) -> u16 {
    let shift = self.consecutive_budget_exhaustions.min(4);
    (base >> shift).max(16)
}
```

Pattern: 224→112→56→28→16 (floors at 4 consecutive exhaustions).

`ExhaustionEntry` (post-S31/S33):
```rust
pub struct ExhaustionEntry {
    pub retry_state: ExhaustionRetryState,  // FrontierExhausted | BudgetRetryPending
    pub invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
    pub baseline: ExhaustionBaseline,
    pub consecutive_budget_exhaustions: u8,
}
```

`has_pending_budget_retry()` (planning.rs:435-440):
```rust
fn has_pending_budget_retry(runtime: &AgentDecisionRuntime) -> bool {
    runtime.exhaustion_cache.values()
        .any(ExhaustionEntry::is_budget_retry_pending)
}
```

This unconditionally forces replanning for any `BudgetRetryPending` entry regardless of tick timing.

## Deliverables

### 1. Add cooldown parameters to `PlanningBudget`

Add two new fields to `PlanningBudget` (budget.rs):

```rust
pub struct PlanningBudget {
    // ... existing fields ...
    /// Initial cooldown in ticks after first budget exhaustion.
    /// Doubles per consecutive failure up to `max_cooldown_ticks`.
    pub initial_cooldown_ticks: u32,
    /// Maximum cooldown in ticks (cap for exponential doubling).
    pub max_cooldown_ticks: u32,
}

impl Default for PlanningBudget {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            initial_cooldown_ticks: 4,
            max_cooldown_ticks: 64,
        }
    }
}
```

### 2. Replace budget halving with cooldown on `ExhaustionEntry`

Remove `consecutive_budget_exhaustions: u8` and `effective_max_expansions()`. Replace with cooldown-based retry fields:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExhaustionEntry {
    pub retry_state: ExhaustionRetryState,
    pub invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
    pub baseline: ExhaustionBaseline,
    /// Tick at which this opportunity becomes eligible for retry.
    /// None = eligible immediately (first tick after recording, before any
    /// cooldown has been applied — only happens transiently).
    pub next_retry_tick: Option<Tick>,
    /// Number of consecutive budget exhaustions (drives cooldown doubling).
    pub consecutive_failures: u8,
}
```

Remove `effective_max_expansions()`, `is_budget_retry_pending()`, and `suppresses_planning()`. Replace with:

```rust
impl ExhaustionEntry {
    /// Whether this opportunity is eligible for retry at the given tick.
    /// FrontierExhausted entries are never eligible (they require invalidation).
    /// BudgetRetryPending entries are eligible when the cooldown has elapsed.
    #[must_use]
    pub fn is_retry_eligible(&self, current_tick: Tick) -> bool {
        match self.retry_state {
            ExhaustionRetryState::FrontierExhausted => false,
            ExhaustionRetryState::BudgetRetryPending => match self.next_retry_tick {
                None => true,
                Some(tick) => current_tick >= tick,
            },
        }
    }

    /// Whether this entry suppresses planning entirely (frontier exhausted).
    #[must_use]
    pub fn suppresses_planning(&self) -> bool {
        matches!(self.retry_state, ExhaustionRetryState::FrontierExhausted)
    }

    /// Record a budget exhaustion. Sets next retry tick based on cooldown.
    ///
    /// Cooldown progression with defaults (initial=4, max=64):
    ///   1st failure → 4 ticks, 2nd → 8, 3rd → 16, 4th → 32, 5th+ → 64
    ///
    /// Formula: initial_cooldown << (consecutive_failures - 1), capped at max.
    pub fn record_budget_exhaustion(
        &mut self,
        current_tick: Tick,
        budget: &PlanningBudget,
    ) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        let shift = self.consecutive_failures.saturating_sub(1).min(6);
        let cooldown = budget.initial_cooldown_ticks
            .saturating_shl(u32::from(shift))
            .min(budget.max_cooldown_ticks);
        self.next_retry_tick = Some(Tick(current_tick.0 + u64::from(cooldown)));
        self.retry_state = ExhaustionRetryState::BudgetRetryPending;
    }
}
```

Factory methods update:

```rust
impl ExhaustionEntry {
    #[must_use]
    pub fn frontier_exhausted(
        invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
        baseline: ExhaustionBaseline,
    ) -> Self {
        Self {
            retry_state: ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions,
            baseline,
            next_retry_tick: None,
            consecutive_failures: 0,
        }
    }

    /// Create a new BudgetRetryPending entry with first-failure cooldown.
    #[must_use]
    pub fn budget_retry_pending(
        invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
        baseline: ExhaustionBaseline,
        current_tick: Tick,
        budget: &PlanningBudget,
    ) -> Self {
        let mut entry = Self {
            retry_state: ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions,
            baseline,
            next_retry_tick: None,
            consecutive_failures: 0,
        };
        entry.record_budget_exhaustion(current_tick, budget);
        entry
    }
}
```

### 3. Planning budget always full

When `is_retry_eligible(current_tick)` returns true, the planner uses the full `PlanningBudget::max_node_expansions` (currently 224). No halving. No floor.

In `planning.rs`, replace the budget-reduction block:

```rust
// BEFORE (budget halving):
let effective_budget = match exhaustion_cache.get(&opportunity) {
    Some(entry) if entry.is_budget_retry_pending() => {
        let mut reduced = budget.clone();
        reduced.max_node_expansions =
            entry.effective_max_expansions(budget.max_node_expansions);
        reduced
    }
    _ => budget.clone(),
};

// AFTER (full budget, cooldown-gated):
// No budget reduction. The cooldown gate in candidate filtering already
// ensures we only reach here when the opportunity is retry-eligible.
// Use the full budget.
let effective_budget = budget.clone();
```

### 4. Cooldown-aware planning trigger

Update `has_pending_budget_retry()` to accept `current_tick` and check cooldown eligibility:

```rust
fn has_pending_budget_retry(runtime: &AgentDecisionRuntime, current_tick: Tick) -> bool {
    runtime.exhaustion_cache.values()
        .any(|entry| entry.is_retry_eligible(current_tick))
}
```

Update both call sites (planning.rs:509, planning.rs:664) to pass `current_tick`.

This prevents entering the planning loop when all exhausted goals are still in cooldown.

### 5. Candidate filtering with cooldown

In `build_candidate_plans()`, update the exhaustion filter to skip non-eligible entries:

```rust
// BEFORE:
let admitted_candidates: Vec<_> = ranked_candidates
    .iter()
    .filter(|c| {
        let key = OpportunityKey { ... };
        !exhaustion_cache
            .get(&key)
            .is_some_and(ExhaustionEntry::suppresses_planning)
    })
    .collect();

// AFTER:
let admitted_candidates: Vec<_> = ranked_candidates
    .iter()
    .filter(|c| {
        let key = OpportunityKey { ... };
        match exhaustion_cache.get(&key) {
            Some(entry) if entry.suppresses_planning() => false,
            Some(entry) if !entry.is_retry_eligible(current_tick) => false,
            _ => true,
        }
    })
    .collect();
```

### 6. Exhaustion recording with cooldown

In `record_exhausted_goals()`, update the budget exhaustion path to use cooldown:

```rust
// BEFORE:
crate::PlanSearchResult::BudgetExhausted { .. } => {
    let mut e = ExhaustionEntry::budget_retry_pending(
        invalidation_conditions, baseline,
    );
    e.consecutive_budget_exhaustions = prev_count.saturating_add(1);
    e
}

// AFTER:
crate::PlanSearchResult::BudgetExhausted { .. } => {
    match runtime.exhaustion_cache.get(&plan.opportunity) {
        Some(existing) if existing.retry_state == ExhaustionRetryState::BudgetRetryPending => {
            let mut e = existing.clone();
            e.invalidation_conditions = invalidation_conditions;
            e.baseline = baseline;
            e.record_budget_exhaustion(tick, budget);
            e
        }
        _ => ExhaustionEntry::budget_retry_pending(
            invalidation_conditions, baseline, tick, budget,
        ),
    }
}
```

Note: `record_exhausted_goals()` needs access to `budget: &PlanningBudget` and `tick: Tick` (the `_tick` parameter is already passed but unused — rename and use it).

### 7. Invalidation interaction

`invalidate_exhausted_goals()` continues to **remove** entries where any invalidation condition fires. No changes needed — removal already gives the correct behavior (immediate full-budget retry on next planning pass).

`FrontierExhausted` entries continue to suppress planning entirely until invalidation fires. Cooldown does not apply to frontier-exhausted entries — they need world state changes, not time.

### 8. Decision trace extension

Add exhaustion cooldown state to `PlanningPipelineTrace`:

```rust
// In decision_trace.rs:

/// Snapshot of one opportunity's exhaustion state at trace time.
#[derive(Clone, Debug)]
pub struct ExhaustionTraceEntry {
    pub opportunity: OpportunityKey,
    pub retry_state: ExhaustionRetryState,
    pub consecutive_failures: u8,
    pub next_retry_tick: Option<Tick>,
    pub retry_eligible: bool,
}

pub struct PlanningPipelineTrace {
    // ... existing fields ...
    /// Exhaustion cache state at trace construction time (P27).
    pub exhaustion_snapshot: Vec<ExhaustionTraceEntry>,
}
```

Populate `exhaustion_snapshot` from `runtime.exhaustion_cache` during trace construction.

### 9. Save/load

`ExhaustionEntry` fields change (new `next_retry_tick` and `consecutive_failures`, removed `consecutive_budget_exhaustions`). Bump `SAVE_FORMAT_VERSION` in `save_load.rs`.

Migration strategy: Old entries use `#[serde(default)]` on new fields:
- `next_retry_tick: Option<Tick>` defaults to `None` (eligible immediately)
- `consecutive_failures: u8` defaults to `0` (fresh start)

The removed `consecutive_budget_exhaustions` field already has `#[serde(default)]`, so old saves that include it won't fail deserialization — `serde` ignores unknown fields with `#[serde(deny_unknown_fields)]` absent (which it is for `ExhaustionEntry`).

## Component Registration

No new ECS components. Changes are to existing AI runtime state (`ExhaustionEntry`, `PlanningBudget`).

## FND-01 Section H Analysis

### Information-path analysis
No new information paths. Exhaustion is internal runtime state, not communicated between agents. Cooldown duration is determined by `PlanningBudget` parameters (per-agent profile), not by inter-agent information.

### Positive-feedback analysis
No new positive-feedback loops. The cooldown mechanism is purely dampening — consecutive failures increase delay, not amplify action.

### Concrete dampeners
- `max_cooldown_ticks` (default 64) caps retry suppression, preventing infinite cooldown growth.
- Invalidation (S31 condition changes) removes entries entirely, resetting to zero-cooldown on next failure.
- Both dampeners are profile-driven and inspectable, not hidden numeric clamps.

### Stored state vs. derived read-model list
- **Stored**: `ExhaustionEntry` fields — `next_retry_tick`, `consecutive_failures`, `retry_state`, `invalidation_conditions`, `baseline` (runtime cache per opportunity, persisted via save/load).
- **Derived**: `is_retry_eligible(current_tick)` (computed from `next_retry_tick` vs `current_tick`). `ExhaustionTraceEntry.retry_eligible` (derived snapshot for traces).

## Tests

### Focused tests
- [ ] First budget exhaustion sets cooldown to `initial_cooldown_ticks` (default 4 ticks)
- [ ] Second consecutive exhaustion doubles cooldown to 8 ticks
- [ ] Third consecutive exhaustion sets cooldown to 16 ticks
- [ ] Cooldown caps at `max_cooldown_ticks` (default 64) after sufficient consecutive failures
- [ ] Full `max_node_expansions` budget used on every retry (no halving)
- [ ] `is_retry_eligible()` returns false when `current_tick < next_retry_tick`
- [ ] `is_retry_eligible()` returns true when `current_tick >= next_retry_tick`
- [ ] `is_retry_eligible()` returns false for `FrontierExhausted` regardless of tick
- [ ] Invalidation removes entry entirely (next exhaustion starts fresh)
- [ ] `has_pending_budget_retry()` returns false when all entries are in cooldown
- [ ] `has_pending_budget_retry()` returns true when at least one entry is eligible
- [ ] Custom `PlanningBudget` cooldown values respected (e.g., initial=10, max=100)
- [ ] `FrontierExhausted` entries not affected by cooldown (still suppressed until invalidation)
- [ ] Save/load round-trip preserves cooldown state (`next_retry_tick`, `consecutive_failures`)
- [ ] Legacy save migration: old entries get `next_retry_tick = None`, `consecutive_failures = 0`
- [ ] Decision trace `exhaustion_snapshot` populated with current cooldown state

### Golden tests
- [ ] Agent exhausts search at source, waits cooldown ticks, retries at full depth, finds changed world state (source regenerated)
- [ ] Deterministic replay companion

## Acceptance Criteria

1. Search budget never halved — every retry uses full `max_node_expansions`.
2. Retry frequency decreases via cooldown (default: 4→8→16→32→64 tick delays).
3. Cooldown parameters are profile-driven via `PlanningBudget`, not hardcoded constants.
4. Invalidation (S31 conditions) removes exhaustion entries entirely for fresh retry.
5. `has_pending_budget_retry()` is cooldown-aware — agents don't enter the planning loop when all exhausted goals are in cooldown.
6. Decision traces log cooldown state and retry eligibility per opportunity.
7. `FrontierExhausted` behavior unchanged (suppressed until invalidation).
8. All existing golden tests pass (behavioral equivalence at tick granularity may shift due to cooldown timing vs budget halving, but outcomes should converge).
