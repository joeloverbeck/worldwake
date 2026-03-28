# S37: Cooldown-Based Exhaustion

## Summary

Replace budget-halving exhaustion backoff with cooldown-based retry. Currently `effective_max_expansions()` halves the search budget on each consecutive failure (256→128→64→32→16). This makes search *shallower* on repeated failure — the wrong shape for a world where resources are temporarily blocked. Agents should retry at full search depth, just less frequently. Additionally, extend `BlockingFact` with more specific failure classifications where the simulation can determine a concrete cause.

## Source

Derived from ChatGPT architecture review WW-AI-004 (Repair-first replanning, blocker scope, and deterministic barrier continuation), filtered to the exhaustion shape and failure classification components. Plan repair and search memo reuse are deferred — they are optimizations that can come later without correctness impact.

## Phase

Phase 3+: AI Architecture Overhaul, Step 13.5 Wave 5

## Crates

- `worldwake-ai` (exhaustion logic, failure handling, decision runtime)

## Dependencies

- S31 ✅ (goal-aware exhaustion invalidation — provides `ExhaustionEntry` structure and invalidation conditions this spec modifies)
- S33 (opportunity-scoped goal identity — exhaustion keyed by `OpportunityKey`; this spec changes retry semantics within that key)

## FOUNDATIONS Alignment

- **P18** (Resource-Bounded Practical Reasoning): Agents should reason within bounded time/knowledge, but repeated failure should not *degrade* reasoning quality. Retrying less often is bounded; searching more shallowly is degraded.
- **P9** (Outcomes Are Granular): Failure should carry specific information about *why* the plan failed, not generic "Unknown" — this enables better replanning decisions.
- **P27** (Debuggability): Specific failure facts make decision traces actionable. "Search budget exhausted after 16 expansions" is less useful than "Source depleted at orchard."

## Design Goals

1. **Full-depth retry**: Every retry attempt uses the full `max_node_expansions` budget. Search quality never degrades.
2. **Increasing cooldown**: Consecutive failures increase the delay before the next retry, not the search budget.
3. **Invalidation reset**: S31 invalidation conditions (state changes) still reset the cooldown to zero — fresh evidence deserves immediate full-budget search.
4. **Specific failure facts**: Where the simulation can determine a concrete cause, failure classification should be precise rather than `Unknown`.
5. **Deterministic**: Cooldown ticks are deterministic (tick arithmetic), not wall-clock based.

## Current Shape

```rust
// decision_runtime.rs:105-108
pub fn effective_max_expansions(&self, base: u16) -> u16 {
    let shift = self.consecutive_budget_exhaustions.min(4);
    (base >> shift).max(16)
}
```

Pattern: 256→128→64→32→16 (floors at 4 consecutive exhaustions).

`ExhaustionEntry` (post-S31):
```rust
pub struct ExhaustionEntry {
    pub retry_state: ExhaustionRetryState,  // FrontierExhausted | BudgetRetryPending
    pub invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
    pub baseline: ExhaustionBaseline,
    pub consecutive_budget_exhaustions: u8,
}
```

## Deliverables

### 1. Replace budget halving with cooldown

Remove `effective_max_expansions()`. Replace `consecutive_budget_exhaustions: u8` with cooldown-based retry fields:

```rust
pub struct ExhaustionEntry {
    pub retry_state: ExhaustionRetryState,
    pub invalidation_conditions: Vec<ExhaustionInvalidationCondition>,
    pub baseline: ExhaustionBaseline,
    /// Tick at which this opportunity becomes eligible for retry.
    /// None = eligible immediately (first exhaustion or after invalidation reset).
    pub next_retry_tick: Option<Tick>,
    /// Current cooldown duration in ticks. Doubles per consecutive failure.
    pub cooldown_ticks: u32,
    /// Number of consecutive failures (for cooldown doubling).
    pub consecutive_failures: u8,
}
```

Cooldown progression: 4→8→16→32→64 ticks, capped at 64.

```rust
impl ExhaustionEntry {
    pub const INITIAL_COOLDOWN: u32 = 4;
    pub const MAX_COOLDOWN: u32 = 64;

    /// Whether this opportunity is eligible for retry at the given tick.
    pub fn is_retry_eligible(&self, current_tick: Tick) -> bool {
        match self.next_retry_tick {
            None => true,
            Some(tick) => current_tick >= tick,
        }
    }

    /// Record a new budget exhaustion. Sets next retry tick based on cooldown.
    pub fn record_budget_exhaustion(&mut self, current_tick: Tick) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.cooldown_ticks = (Self::INITIAL_COOLDOWN << self.consecutive_failures.min(4))
            .min(Self::MAX_COOLDOWN);
        self.next_retry_tick = Some(Tick(current_tick.0 + self.cooldown_ticks as u64));
        self.retry_state = ExhaustionRetryState::BudgetRetryPending;
    }

    /// Reset cooldown after invalidation (state change detected).
    pub fn reset_cooldown(&mut self) {
        self.consecutive_failures = 0;
        self.cooldown_ticks = 0;
        self.next_retry_tick = None;
    }
}
```

### 2. Planning budget always full

When `is_retry_eligible()` returns true, the planner uses the full `PlanningBudget::max_node_expansions` (currently 224). No halving. No floor.

In `agent_tick/planning.rs` (or equivalent): Before searching, check `is_retry_eligible(current_tick)`. If not eligible, skip this opportunity. If eligible, search with full budget.

### 3. Invalidation interaction

S31's `invalidate_exhausted_goals()` continues to check per-opportunity invalidation conditions. When invalidation fires, call `reset_cooldown()` on the affected entry. This makes the opportunity immediately eligible for full-budget retry.

`FrontierExhausted` state (no valid plan exists) continues to suppress planning entirely until invalidation fires — cooldown does not apply to frontier-exhausted entries (they need world state changes, not time).

### 4. Extended `BlockingFact` variants

Add specific failure classifications to `BlockingFact` (worldwake-core):

```rust
pub enum BlockingFact {
    // Existing variants...
    TargetGone,
    NoKnownPath,
    NoInput,
    PatienceExhausted,
    AssumptionFailed,
    // NEW specific variants:
    /// Source entity is depleted (no commodity remaining).
    SourceDepleted { source: EntityId, commodity: CommodityKind },
    /// Counterparty not at expected location.
    CounterpartyAbsent { target: EntityId },
    /// Reservation or queue position was lost.
    ReservationLost { resource: EntityId },
    /// No jurisdiction to perform the intended political action.
    JurisdictionMissing { office: EntityId },
}
```

In `failure_handling.rs`, replace `BlockingFact::Unknown` with specific variants where the failure context provides enough information:
- `StartFailed` with `TargetGone` reason + source entity → `SourceDepleted`
- `StartFailed` with trade target not at place → `CounterpartyAbsent`
- Facility grant expired → `ReservationLost`
- Office action failed due to no jurisdiction → `JurisdictionMissing`

`Unknown` remains as the fallback for genuinely unclassifiable failures.

### 5. Decision trace extension

Log cooldown state in `AgentDecisionTrace`:

```rust
pub struct ExhaustionTraceEntry {
    pub opportunity: OpportunityKey,  // or GoalKey pre-S33
    pub retry_state: ExhaustionRetryState,
    pub consecutive_failures: u8,
    pub cooldown_ticks: u32,
    pub next_retry_tick: Option<Tick>,
    pub retry_eligible: bool,
}
```

### 6. Save/load

`ExhaustionEntry` fields change (new fields, removed `consecutive_budget_exhaustions`). `SAVE_FORMAT_VERSION` bumps. Migration: Old entries get `next_retry_tick = None`, `cooldown_ticks = 0`, `consecutive_failures = 0` (fresh start on load).

## Component Registration

No new ECS components. Changes are to existing runtime state (`ExhaustionEntry`).

## FND-01 Section H Analysis

### Information-path analysis
No new information paths. Exhaustion is internal runtime state, not communicated between agents.

### Positive-feedback analysis
No new loops. Cooldown is a dampening mechanism, not amplifying.

### Concrete dampeners
Cooldown cap at 64 ticks prevents infinite retry suppression. Invalidation (state change) resets cooldown immediately.

### Stored state vs. derived read-model list
- **Stored**: `ExhaustionEntry` fields (runtime cache per opportunity, persisted via save/load).
- **Derived**: `is_retry_eligible()` (computed from `next_retry_tick` vs `current_tick`).

## Tests

### Focused tests
- [ ] First budget exhaustion sets cooldown to 4 ticks
- [ ] Second consecutive exhaustion doubles cooldown to 8 ticks
- [ ] Cooldown caps at 64 ticks after 4+ consecutive failures
- [ ] Full `max_node_expansions` budget used on every retry (no halving)
- [ ] Retry skipped when `current_tick < next_retry_tick`
- [ ] Retry allowed when `current_tick >= next_retry_tick`
- [ ] Invalidation resets cooldown to 0 and clears `next_retry_tick`
- [ ] `FrontierExhausted` entries not affected by cooldown (still suppressed until invalidation)
- [ ] `BlockingFact::SourceDepleted` generated when harvest fails on empty source
- [ ] `BlockingFact::CounterpartyAbsent` generated when trade target not at place
- [ ] `BlockingFact::Unknown` still used for unclassifiable failures
- [ ] Save/load round-trip preserves cooldown state
- [ ] Decision trace includes cooldown and retry eligibility

### Golden tests
- [ ] Agent exhausts search at source, waits cooldown ticks, retries at full depth, finds changed world state (source regenerated)
- [ ] Deterministic replay companion

## Acceptance Criteria

1. Search budget never halved — every retry uses full `max_node_expansions`.
2. Retry frequency decreases via cooldown (4→8→16→32→64 tick delays).
3. Invalidation (S31 conditions) resets cooldown immediately for fresh evidence.
4. At least 4 `BlockingFact` variants replace `Unknown` where concrete cause is determinable.
5. Decision traces log cooldown state and retry eligibility.
6. `FrontierExhausted` behavior unchanged (suppressed until invalidation).
7. All existing golden tests pass (behavioral equivalence at tick granularity may shift slightly due to cooldown timing vs budget halving, but outcomes should converge).
