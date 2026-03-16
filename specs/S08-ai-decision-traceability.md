**Status**: DRAFT

# AI Decision Traceability

## Summary

Add a structured, optional decision trace system to `worldwake-ai` that records why each agent made each decision at each tick. Today the AI decision pipeline is fire-and-forget: each stage computes a result, passes it to the next stage, and discards all reasoning context. This makes debugging emergent behavior — especially in golden e2e tests — require hours of ad-hoc `eprintln` instrumentation for each failure.

The trace system records per-agent-per-tick structured decision records covering the full pipeline: candidate generation, ranking, plan search, plan selection, execution outcome, and failure handling. It is zero-cost when disabled and queryable by tests.

## Why This Exists

Principle 27 states: *"Emergence without introspection is indistinguishable from bugs."* The simulation must support questions like "Why did this agent do that?" and "Why did this agent *not* do that?" with answers *"reconstructable from state, beliefs, records, and causal history — not guessed by developers."*

The current AI layer violates this principle. Five concrete failures during golden e2e test development (S02/S02b/S02c suite, March 2026) demonstrated that the pipeline discards essential diagnostic information at every stage:

### Failure 1: Planner Action Def Confusion
**Question**: "Why does ThirstFirst agent eat instead of drink?"
**Root cause**: `search_plan()` picks "eat" (ActionDefId 0) for a water target because both "eat" and "drink" map to `PlannerOpKind::Consume`, and `consume_transition_matches_goal` checks commodity identity but not `ConsumableEffect`. At execution, the "eat" precondition (`TargetHasConsumableEffect::Hunger`) fails for water.
**Missing information**: Which action defs were tried per goal, which transition checks passed/failed, which action def the plan selected.

### Failure 2: Invisible Instant Action
**Question**: "Why is the first observed action 'eat' when sleep should happen first?"
**Root cause**: Sleep completes in 1 tick (`DurationExpr::Fixed(NonZeroU32::MIN)`). By the time the test checks `agent_active_action_name`, sleep has already completed and the agent is on its second action.
**Missing information**: Per-tick record of what goal was selected, what action was enqueued, what completed within the tick.

### Failure 3: Consumer Trade Failure
**Question**: "Why doesn't the consumer trade with the merchant after restock?"
**Root cause**: Could not be conclusively diagnosed despite ~2 hours of investigation. Potentially: consumer's belief store did not contain the merchant at the right time, or `agents_selling_at` returned empty, or `AcquireCommodity { Apple }` was never generated, or the planner failed to find a trade plan.
**Missing information**: Full candidate list (was acquire-apple even generated?), belief store contents at decision time, `agents_selling_at` result, plan search outcome for each candidate.

### Failure 4: Merchant Oscillation
**Question**: "Why does the merchant keep leaving General Store after restocking?"
**Root cause**: The enterprise restock signal re-fires after the merchant returns because demand memory persists or a new enterprise signal is generated.
**Missing information**: What enterprise signals were active, what candidates were ranked, why the restock goal re-emerged, the dirty-flag reason.

### Failure 5: BestEffort Silent Failure
**Question**: "Why did the action not start?"
**Root cause**: `tick_step.rs` processes `BestEffort` inputs — when action start fails, it silently skips. The agent discovers this next tick via "no active action."
**Missing information**: That an action start was attempted and failed, which precondition failed.

### Common Thread

Every failure required the same debugging loop: "add eprintln → recompile (30s) → run test (20-80s) → read output → hypothesize → repeat." Each cycle takes 1-3 minutes, and diagnosing a single failure required 5-15 cycles. The information needed was always the same — what happened inside the decision pipeline — but it was discarded before the test could observe it.

## Why This Is a Spec, Not a Ticket

The trace system touches the internal structure of every pipeline stage in `worldwake-ai`:
- `agent_tick.rs` — main orchestrator, must thread trace context through all phases
- `candidate_generation.rs` — must report generated candidates before ranking filter
- `ranking.rs` — must report priority class and motive score per candidate
- `search.rs` — must report plan search outcomes (found, budget exhausted, unsupported)
- `plan_selection.rs` — must report why the selected plan beat alternatives
- `failure_handling.rs` — must report failure context
- `GoldenHarness` — must expose trace query API for tests

It is too structural for a single ticket. The design must be coherent across all stages.

## Phase

Post-Phase-2 hardening, parallel with S02–S07. No dependency on E14 or later epics. Can be implemented immediately against the current codebase.

**Blocking**: This spec blocks the S02c golden e2e test (Multi-Role Emergent Supply Chain) which could not be debugged to completion without decision traces. S02c should be re-implemented after this spec lands.

## Crates

- `worldwake-ai` (primary — all trace types and collection logic)
- `worldwake-sim` (minor — `BestEffort` action start failure recording in `tick_step.rs`)

`worldwake-core` is not touched. The trace system is an AI-layer concern, not a world-state concern.

## Design Principles

1. **Zero cost when disabled** — trace collection is behind a runtime flag on `AgentTickDriver`; when disabled, no allocations occur on the hot path
2. **Structured, not strings** — typed Rust enums and structs, not format strings; queryable by test assertions
3. **Per-agent per-tick** — one `AgentDecisionTrace` record per agent per tick
4. **Append-only accumulation** — traces accumulate in a Vec, queryable after simulation completes
5. **Pipeline-complete** — covers every stage from candidate generation through execution outcome
6. **Separate from the event log** — the event log is the authoritative causal record of world-state changes; decision traces record epistemic reasoning, not causality; mixing them would bloat the event log and change deterministic hashes

## Trace Data Model

### Top-Level Record

```rust
/// One complete decision record for one agent at one tick.
#[derive(Clone, Debug)]
pub struct AgentDecisionTrace {
    pub agent: EntityId,
    pub tick: Tick,
    pub outcome: DecisionOutcome,
}
```

### Decision Outcome

```rust
#[derive(Clone, Debug)]
pub enum DecisionOutcome {
    /// Agent is dead — no decision pipeline ran.
    Dead,

    /// Agent has an active action — interrupt evaluation ran.
    ActiveAction {
        action_def_id: ActionDefId,
        action_name: String,
        interrupt: InterruptTrace,
    },

    /// Agent had no active action — full planning pipeline ran.
    Planning(PlanningPipelineTrace),
}
```

### Planning Pipeline Trace

```rust
#[derive(Clone, Debug)]
pub struct PlanningPipelineTrace {
    pub dirty_reasons: Vec<DirtyReason>,
    pub candidates: CandidateTrace,
    pub planning: PlanSearchTrace,
    pub selection: SelectionTrace,
    pub execution: ExecutionTrace,
}
```

### Stage 1: Candidate Generation + Ranking

```rust
#[derive(Clone, Debug)]
pub struct CandidateTrace {
    /// All grounded goal keys generated (before suppression/zero-motive filter).
    pub generated: Vec<GoalKey>,
    /// Ranked goals after all filters (sorted by ranking order).
    pub ranked: Vec<RankedGoalSummary>,
    /// Goals that were suppressed and why.
    pub suppressed: Vec<GoalKey>,
    /// Goals filtered by zero motive score.
    pub zero_motive: Vec<GoalKey>,
}

#[derive(Clone, Debug)]
pub struct RankedGoalSummary {
    pub goal: GoalKey,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
}
```

### Stage 2: Plan Search

```rust
#[derive(Clone, Debug)]
pub struct PlanSearchTrace {
    /// One entry per candidate that was planned (top N by budget).
    pub attempts: Vec<PlanAttemptTrace>,
}

#[derive(Clone, Debug)]
pub struct PlanAttemptTrace {
    pub goal: GoalKey,
    pub outcome: PlanSearchOutcome,
}

#[derive(Clone, Debug)]
pub enum PlanSearchOutcome {
    /// Plan found.
    Found {
        steps: Vec<PlannedStepSummary>,
        terminal_kind: PlanTerminalKind,
    },
    /// Node expansion budget exhausted.
    BudgetExhausted { expansions_used: u16 },
    /// Goal kind is unsupported by planner.
    Unsupported,
    /// Frontier exhausted without finding a plan.
    FrontierExhausted { expansions_used: u16 },
}

#[derive(Clone, Debug)]
pub struct PlannedStepSummary {
    pub action_def_id: ActionDefId,
    pub action_name: String,
    pub op_kind: PlannerOpKind,
    pub targets: Vec<EntityId>,
    pub estimated_ticks: u32,
}
```

### Stage 3: Plan Selection

```rust
#[derive(Clone, Debug)]
pub struct SelectionTrace {
    /// The goal/plan that was selected (None if no plans available).
    pub selected: Option<GoalKey>,
    /// Whether a goal switch occurred from the previous tick's goal.
    pub goal_switch: Option<GoalSwitchSummary>,
    /// The previous goal (if any) for context.
    pub previous_goal: Option<GoalKey>,
}

#[derive(Clone, Debug)]
pub struct GoalSwitchSummary {
    pub from: GoalKey,
    pub to: GoalKey,
    pub kind: GoalSwitchKind,
}
```

### Stage 4: Execution Outcome

```rust
#[derive(Clone, Debug)]
pub struct ExecutionTrace {
    /// The step that was submitted for execution.
    pub enqueued_step: Option<PlannedStepSummary>,
    /// Whether revalidation of the step passed.
    pub revalidation_passed: Option<bool>,
    /// If the step could not be enqueued, why.
    pub failure: Option<ExecutionFailureReason>,
}

#[derive(Clone, Debug)]
pub enum ExecutionFailureReason {
    RevalidationFailed,
    TargetResolutionFailed,
    RecoverableTravelBlockage,
    PlanFailureHandled { blocked_goal: Option<GoalKey> },
}
```

### Interrupt Trace (for active-action path)

```rust
#[derive(Clone, Debug)]
pub struct InterruptTrace {
    pub decision: InterruptDecision,
    /// The highest-ranked challenger goal, if any.
    pub top_challenger: Option<RankedGoalSummary>,
}
```

### Dirty Reasons

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirtyReason {
    NoPlan,
    PlanFinished,
    ReplanSignal,
    QueueTransition,
    BlockerCleanup,
    SnapshotChanged,
    QueuePatienceExhausted,
}
```

## Collection Architecture

### Trace Sink on AgentTickDriver

```rust
pub struct AgentTickDriver {
    budget: PlanningBudget,
    runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>,
    semantics_cache: Option<...>,
    /// Optional trace collector. When Some, decision traces are recorded.
    trace_sink: Option<DecisionTraceSink>,
}

pub struct DecisionTraceSink {
    traces: Vec<AgentDecisionTrace>,
}

impl DecisionTraceSink {
    pub fn new() -> Self { ... }
    pub fn record(&mut self, trace: AgentDecisionTrace) { ... }
    pub fn traces(&self) -> &[AgentDecisionTrace] { ... }
    pub fn traces_for(&self, agent: EntityId) -> Vec<&AgentDecisionTrace> { ... }
    pub fn trace_at(&self, agent: EntityId, tick: Tick) -> Option<&AgentDecisionTrace> { ... }
    pub fn clear(&mut self) { ... }
}
```

### Threading Through the Pipeline

`process_agent()` receives an `Option<&mut DecisionTraceSink>`. Each sub-function returns its trace fragment alongside its computational result. At the end of `process_agent()`, if the sink is `Some`, the fragments are assembled into one `AgentDecisionTrace` and recorded.

This means:
- `refresh_runtime_for_read_phase` returns `(Vec<RankedGoal>, Option<CandidateTrace>)`
- `plan_candidates` returns `(Vec<(GoalKey, Option<PlannedPlan>)>, Option<PlanSearchTrace>)`
- `plan_and_validate_next_step` returns `(Option<PlannedStep>, Option<bool>, Option<SelectionTrace>)`
- `enqueue_valid_step_or_handle_failure` populates an `Option<ExecutionTrace>`

The `Option` pattern ensures zero cost when tracing is disabled — functions check once at the top and skip all trace allocation when `None`.

### BestEffort Action Start Failure (worldwake-sim)

`tick_step.rs` currently silently skips `BestEffort` action starts that fail. This spec adds a lightweight hook: when an action start fails for a `BestEffort` input, the failure reason is recorded on the `Scheduler` as a `Vec<ActionStartFailure>` that is drained each tick. The AI layer can then incorporate this into the next tick's trace.

```rust
#[derive(Clone, Debug)]
pub struct ActionStartFailure {
    pub tick: Tick,
    pub actor: EntityId,
    pub def_id: ActionDefId,
    pub reason: String,
}
```

This is the only sim-layer change. It is small and backward-compatible.

## Test Query API

### GoldenHarness Integration

```rust
impl GoldenHarness {
    /// Enable decision tracing. Must be called before stepping.
    pub fn enable_tracing(&mut self) {
        self.driver.enable_tracing();
    }

    /// Access the trace sink for queries.
    pub fn traces(&self) -> Option<&DecisionTraceSink> {
        self.driver.trace_sink()
    }
}
```

### Diagnostic Assertions for Golden Tests

The trace API enables assertions that directly answer "why" questions:

```rust
// "Was AcquireCommodity { Apple } ever generated for the consumer?"
let trace = harness.traces().unwrap().trace_at(consumer, Tick(160)).unwrap();
if let DecisionOutcome::Planning(ref planning) = trace.outcome {
    assert!(
        planning.candidates.generated.iter().any(|g| matches!(
            g.kind, GoalKind::AcquireCommodity { commodity: CommodityKind::Apple, .. }
        )),
        "Consumer should generate AcquireCommodity for Apple; generated={:?}",
        planning.candidates.generated
    );
}

// "What plan did the planner find for ConsumeOwnedCommodity { Water }?"
let water_attempt = planning.planning.attempts.iter()
    .find(|a| matches!(a.goal.kind, GoalKind::ConsumeOwnedCommodity { commodity: CommodityKind::Water }));
assert!(matches!(water_attempt.unwrap().outcome, PlanSearchOutcome::Found { .. }));

// "What was the agent's first selected goal?"
let first_planning = harness.traces().unwrap()
    .traces_for(agent)
    .iter()
    .find_map(|t| match &t.outcome {
        DecisionOutcome::Planning(p) => p.selection.selected.as_ref(),
        _ => None,
    });
```

### Dump for Interactive Debugging

```rust
impl DecisionTraceSink {
    /// Print a human-readable summary for one agent across all ticks.
    pub fn dump_agent(&self, agent: EntityId, action_defs: &ActionDefRegistry) {
        for trace in self.traces_for(agent) {
            eprintln!("[tick {}] {}", trace.tick.0, trace.outcome.summary());
        }
    }
}
```

## Implementation Plan

### Ticket 1: Trace Data Model
- Define all trace structs and enums in a new `worldwake-ai/src/decision_trace.rs` module
- `DecisionTraceSink` with collection and query methods
- Unit tests for sink operations

### Ticket 2: Collection in `process_agent`
- Thread `Option<&mut DecisionTraceSink>` through `process_agent` and its sub-functions
- `refresh_runtime_for_read_phase` populates `CandidateTrace` (generated list, ranked list, suppressed, zero-motive)
- `plan_candidates` populates `PlanSearchTrace` (per-goal search outcome)
- `plan_and_validate_next_step` populates `SelectionTrace`
- `enqueue_valid_step_or_handle_failure` populates `ExecutionTrace`
- `handle_active_action_phase` populates `InterruptTrace`
- Assembly into `AgentDecisionTrace` at end of `process_agent`

### Ticket 3: BestEffort Failure Recording (worldwake-sim)
- Add `ActionStartFailure` and drain-per-tick collection on `Scheduler`
- Record failure in `tick_step.rs` BestEffort path
- Expose drain API for AI layer consumption

### Ticket 4: GoldenHarness Integration
- `enable_tracing()` / `traces()` on `GoldenHarness`
- Convenience query helpers
- `dump_agent()` for interactive debugging

### Ticket 5: S02c Golden E2E — Multi-Role Emergent Supply Chain
Re-implement the S02c golden test with original intended complexity:
- **Producer** at Orchard Farm: low needs pressure, OrchardRow workstation + ResourceSource(Apple, qty=10), knows harvest recipe, MerchandiseProfile(Apple, home=OrchardFarm), TradeDispositionProfile, PerceptionProfile
- **Merchant** at General Store: enterprise-focused (enterprise_weight=pm(900)), has coins(5), MerchandiseProfile(Apple, home=GeneralStore), enterprise TradeDispositionProfile, DemandMemory with apple demand at GeneralStore, beliefs about orchard workstation + producer, PerceptionProfile
- **Consumer** at General Store: hungry (pm(800)), has coins(5), TradeDispositionProfile, beliefs about merchant
- Run up to 300 ticks
- Assert full chain: merchant leaves → merchant acquires apples → merchant returns → consumer acquires apples → consumer hunger decreases → conservation holds → no deaths
- Companion deterministic replay test
- **Use decision traces to diagnose any failures during development**, proving the trace system works

## What This Does Not Cover

- **Event log expansion**: The event log is authoritative causal state. Decision traces are epistemic, not causal. Mixing them would bloat the log and change deterministic hashes.
- **Perception trace**: Why an agent observed or failed to observe an entity. This is a `worldwake-systems` concern and could be a future extension, but is out of scope here.
- **CLI/UI integration**: The trace is test-only infrastructure for now. Future CLI commands could expose it, but that is separate work.
- **Serialization**: Traces are ephemeral in-memory data. They are not persisted across save/load. If replay debugging needs traces, the replay can re-run with tracing enabled.

## Principles Alignment

| Principle | How This Spec Serves It |
|-----------|------------------------|
| 6. World Runs Without Observers | Traces let us verify *why* the world ran the way it did, after the fact |
| 18. Resource-Bounded Practical Reasoning | Traces expose the bounded reasoning process: budget limits, candidate caps, beam width |
| 27. Debuggability Is a Product Feature | This spec is a direct implementation of Principle 27 for the AI layer |

## FND-01 Section H Analysis

### Information-path analysis
Decision traces flow from the AI pipeline (worldwake-ai) to the test harness. No simulation-time information path is created — traces are a read-side diagnostic, not a write-side causal input. Agents do not read traces; only test code and future CLI tools do.

### Positive-feedback analysis
No positive feedback loops. Traces are append-only output that does not feed back into the decision pipeline.

### Concrete dampeners
N/A — no feedback loops to dampen.

### Stored state vs. derived read-model list
- **Stored state**: `DecisionTraceSink.traces: Vec<AgentDecisionTrace>` — accumulated trace records
- **Stored state**: `Scheduler.action_start_failures: Vec<ActionStartFailure>` — drained per tick
- **Derived**: All query results (`traces_for`, `trace_at`, `dump_agent`) are computed on the fly from stored traces

No derived value is stored as authoritative state.
