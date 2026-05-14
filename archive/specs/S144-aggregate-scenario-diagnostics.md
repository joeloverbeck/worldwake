# S144: Aggregate Scenario Diagnostics

**Status**: COMPLETED

## Summary

The external assessment in `reports/ai-architecture-improvements.md` (Section 17, "Build the missing aggregate diagnostics now") calls this out as the prerequisite for all planning tuning: "Without aggregate metrics, every planning change will be guesswork." The observer binary already collects rich per-tick / per-agent traces (`AgentDecisionTrace`, `PlanAttemptTrace`, `SearchExpansionSummary`, `RepairAttemptTrace`, `CausalLinkCapHit`) and surfaces 9 anomaly detectors (`STUCK_AGENT`, `GEOGRAPHIC_CONVERGENCE`, `MAINTENANCE_STARVATION`, `RECIPE_MONOCULTURE`, `ACUTE_NEED_SPIKE`, `ACTION_LOOP`, `REDUNDANT_PERCEPTION`, `FAILED_ACTION_SPIRAL`, `SUSTAINED_CRITICAL_NEED`). It does *not* surface scenario-level rolled-up metrics: candidate counts by goal kind and portfolio slot, suppression counts by category, plan-attempt budget exhaustion rates, frontier-exhaustion ratios, beam-truncation ratios, plan-depth p50/p95/max, terminal-kind distribution, repair attempt/success ratio, invalidation-reason histograms, belief contradiction counts, source-reliability change counts, queue wait-time distributions, or opportunity-compiler load trajectories.

S144 lands the missing aggregate-consumer layer as a new observer Section 13 (`Scenario Diagnostics`), backed by a deterministic `ScenarioDiagnosticsReport` produced over the full tick range of a run. The aggregator reads only the already-emitted trace data and the existing event log; it adds no engine-side coupling, no new authoritative state, and no new event tags. FND-29 (Debuggability is a product feature) makes this work first-class. The report is a tooling/observer boundary — it never mutates world meaning.

This spec is also the prerequisite that Phase 12's other accepted proposals reference when defending their priority. The rejected PR-9 (Incremental snapshots + multi-queue search) is queued for reassessment specifically against the diagnostics S144 surfaces.

## Phase and Status

Phase 12: AI Architecture Evolution — Completed and archived

## Crates

- `worldwake-ai` — owns a new `scenario_diagnostics` module that defines the `ScenarioDiagnosticsReport` data type, the `CandidateSuppressionCategory` aggregation-key enum, and the deterministic aggregator that consumes `&[AgentDecisionTrace]`, `&[PlanAttemptTrace]`, `&[RepairAttemptTrace]`, and the event log. Pure read-only computation. Also: promotes `SlotKind` (`crates/worldwake-ai/src/agent_tick/portfolio.rs`) to `pub` with serde derives so the report can key on it (D3), and adds deterministic logical hit/miss/invalidation counters to the existing `PlanningSnapshot` caches surfaced through the decision trace (D8). No new authoritative state.
- `worldwake-cli` — observer binary (`crates/worldwake-cli/src/bin/observer.rs`) gains a Section 13 renderer for `ScenarioDiagnosticsReport`. New CLI flags `--diagnostics-format=text|json`, `--diagnostics-percentiles`, `--diagnostics-top-n`, `--no-diagnostics`.
- `worldwake-core` — exposes `PercentileBucket` (`crates/worldwake-core/src/percentile.rs`, new) helper for deterministic percentile computation over integer collections. No new authoritative state.
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.

## Dependencies

- S110 (Decision History Events) — archived at `archive/specs/S110-decision-history-events.md`. Provides the decision-event payload S144 reads for terminal-kind, motive-source, and rejected-goal aggregation.
- S136 (Decision Event Payload Extension) — archived at `archive/specs/S136-decision-event-payload-extension.md`. Provides the always-on payload structure the aggregator reads.
- S117 (Convergence/Maintenance Observer Smells) — archived at `archive/specs/S117-convergence-maintenance-observer-smells.md`. Defines the existing detector pattern S144 extends.
- S120 (Survival Critical-Window Forensics) — archived at `archive/specs/S120-survival-critical-window-forensics.md`. Provides the precedent for deterministic derived-report surfaces.

## Design Goals

1. **Aggregate-only, never authoritative.** `ScenarioDiagnosticsReport` is a derived view over append-only traces + event log. Deleting it and recomputing produces an identical report.
2. **Deterministic — logical counts only, no wall-clock.** Same seed, same scenario, same final tick → byte-identical report. Every metric the report carries is a deterministic logical quantity (counts, depths, expansion totals, tick-delta distributions). Wall-clock timings (`std::time::Duration`, nanosecond elapsed) are forbidden in the report — they would break determinism, the D9 golden, and the D10 committed fixture, and they violate AGENTS.md's no wall-clock/determinism invariant. The pre-existing `perf_telemetry.rs` early/late wall-clock timing infrastructure is *not* consumed by S144.
3. **No new engine coupling.** No new `SystemFn`, no new event tags, no new ECS components. The aggregator reads existing surfaces only; D8 adds only read-only logical counters to data structures the AI crate already owns.
4. **Useful for tuning before tuning happens.** Specifically: per-goal-kind plan-attempt budget exhaustion ratio, frontier-exhaustion rate, p95 plan depth, repair attempted-vs-succeeded ratio, candidate-suppression counts by category.
5. **Anomaly detectors retained.** The existing 9 detectors keep their format; S144 adds a parallel "metrics" channel of rolled-up histograms and distributions.
6. **Streaming-friendly.** The aggregator can be invoked at the end of a run (single shot) or as a periodic snapshot (every N ticks). The current observer model is single-shot; periodic mode is supported but not the default.

## Non-Goals

- **No live engine metrics dashboard.** S144 is a post-run / on-demand report, not a real-time UI.
- **No new authoritative state.** The aggregator does not write back to the world.
- **No anomaly-detector behavior change.** S117/S118 detectors keep their current emission rules; this spec adds metrics alongside them.
- **No prescriptive thresholds.** S144 reports observed distributions; it does not enforce health contracts. The S119/S121 authored survival-health-contract surface is the contract-enforcement layer.
- **No wall-clock performance profiling.** Wall-clock cost metrics (snapshot build time, candidate-extraction time in nanoseconds) are deliberately excluded — they are non-deterministic and out of scope for a byte-stable report. If wall-clock profiling is wanted later, it belongs in a separate, explicitly-non-deterministic channel that is not part of `ScenarioDiagnosticsReport`.
- **No contract-coordination metrics in the first-shipped report.** Contract bid/award/failure counts are fold-rejected (see D8) because no contract event tags exist in the event log; surfacing them would require new event tags, which S144 does not introduce.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-12 (Performance May Compress Computation, Never Causality) | The aggregator is purely derived; world meaning is unchanged. No wall-clock data enters the report, so the boundary changes nothing about what the world means. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `ScenarioDiagnosticsReport` is explicitly a derived view; the spec calls it out as deletable and recomputable. The D8 logical cache counters are themselves derived read-model state, never authoritative. |
| FND-29 (Debuggability Is a Product Feature) | Directly serves "Why did this scenario fail to scale?" — the questions the diagnostics report answers are the questions the assessment lists as missing. |
| FND-29A (Causal History Is Authoritative, Append-Only, and Queryable) | The aggregator reads append-only traces and the append-only event log; nothing it computes is destructive. |
| FND-31 (Validation and Falsification Are First-Class) | The metrics surface is the substrate for "fitness for purpose under explicit evaluation criteria" — without it, planning-tuning is unscientific. |

## Deliverables

### D1: `ScenarioDiagnosticsReport` data type

```rust
// crates/worldwake-ai/src/scenario_diagnostics/mod.rs (new)
pub struct ScenarioDiagnosticsReport {
    pub tick_range: (Tick, Tick),
    pub goal_pressure: GoalPressureMetrics,
    pub planning: PlanningMetrics,
    pub revalidation_repair: RevalidationRepairMetrics,
    pub belief: BeliefMetrics,
    pub coordination: CoordinationMetrics,
    pub performance: PerformanceMetrics,
}

pub struct GoalPressureMetrics {
    pub candidates_emitted_by_kind: BTreeMap<GoalKind, u64>,
    pub candidates_emitted_by_slot: BTreeMap<SlotKind, u64>,
    pub candidates_suppressed_by_category: BTreeMap<CandidateSuppressionCategory, u64>,
    pub top_k_not_planned: BTreeMap<GoalKind, u64>,
    pub active_intention_continuation_rate: Permille,
}

pub struct PlanningMetrics {
    pub plan_attempts: u64,
    pub plan_attempts_by_kind: BTreeMap<GoalKind, u64>,
    pub budget_exhaustion_count: u64,
    pub budget_exhaustion_rate: Permille,
    pub frontier_exhaustion_count: u64,
    pub frontier_exhaustion_rate: Permille,
    pub beam_truncation_ratio: Permille,
    pub plan_depth: PercentileBucket,           // p50/p95/p99/max
    pub terminal_kind_distribution: BTreeMap<PlanTerminalKind, u64>,
    pub heuristic_helpful_action_hit_rate: Permille,
}

pub struct RevalidationRepairMetrics {
    pub invalidation_reasons: BTreeMap<Discrepancy, u64>,
    pub repair_attempts: u64,
    pub repair_succeeded: u64,
    pub repair_failed: u64,
    pub repair_success_rate: Permille,
    pub repair_budget_consumed: PercentileBucket,
    pub full_replan_count: u64,
}

pub struct BeliefMetrics {
    pub stale_belief_actions: u64,
    pub contradicted_belief_actions: u64,
    pub source_reliability_changes: u64,
    pub false_rumor_propagation_count: u64,
    pub correction_latency: PercentileBucket,
}

pub struct CoordinationMetrics {
    pub queue_wait_ticks: PercentileBucket,
    pub reservation_conflict_count: u64,
    pub abandoned_grant_count: u64,
    pub dead_claimant_cleanup_count: u64,
}

pub struct PerformanceMetrics {
    pub opportunity_compiled_count: PercentileBucket,
    pub opportunity_salience_floored: PercentileBucket,
    pub opportunity_learned_memory_damped: PercentileBucket,
    pub opportunity_cap_truncated: PercentileBucket,
    pub search_expansions: PercentileBucket,
    pub cache_hit_count: u64,
    pub cache_miss_count: u64,
    pub cache_invalidation_count: u64,
}
```

`Permille` for all ratios; `PercentileBucket` for distributions. No floats. The whole tree must derive `Serialize`/`Deserialize` (D7 JSON output, D9 round-trip, D10 fixture), so every key and value type must be serde-ready: `GoalKind` (`crates/worldwake-core/src/goal.rs:62` — derives `Copy, Ord, Hash, Serialize, Deserialize`), `PlanTerminalKind` (`crates/worldwake-ai/src/planner_ops.rs` — serde-ready), `Discrepancy` (`crates/worldwake-core/src/discrepancy.rs:9` — serde-ready), `Permille`, `PercentileBucket`, `SlotKind` (made serde-ready by D3), and `CandidateSuppressionCategory` (D5). The type tree is format-agnostic serde data; observer/fixture JSON must use a deterministic representation that handles payload-bearing map keys instead of assuming every `BTreeMap<GoalKind, _>` or `BTreeMap<Discrepancy, _>` key can be emitted as a raw JSON object key.

Type-reference notes:

- `candidates_emitted_by_kind` / `top_k_not_planned` / `plan_attempts_by_kind` key on `GoalKind` (the actual enum; there is no `GoalKindDiscriminant` type in the codebase).
- `invalidation_reasons` keys on `Discrepancy` (the actual enum; there is no `DiscrepancyKind` type). `Discrepancy` carries three payload-bearing variants (`NeedHorizonExceeded { .. }`, `Omission(..)`, `ArtifactNotActionable { .. }`); the aggregator groups these by variant discriminant so the histogram counts reasons, not payload permutations.
- `candidates_emitted_by_slot` keys on `SlotKind` (`Survival | Commitment | Economic` — portfolio-slot semantics from `crates/worldwake-ai/src/agent_tick/portfolio.rs`). This is the intended per-portfolio-slot bucketing of emitted candidates.
- `BeliefMetrics.source_reliability_changes` is a flat `u64` count. A by-topic breakdown was considered but fold-rejected: there is no `TopicScope` type in the codebase, and a per-topic source-reliability surface would need substrate that does not yet exist. A future spec can extend this to a per-topic map once a topic-scope type lands.

### D2: `PercentileBucket` in `worldwake-core`

```rust
// crates/worldwake-core/src/percentile.rs (new)
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PercentileBucket {
    pub n: u64,
    pub min: u64,
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
    pub max: u64,
    pub mean: u64,
}

impl PercentileBucket {
    pub fn from_sorted(values: &[u64]) -> Self { /* deterministic */ }
}
```

Integer-only percentile computation over sorted slices; `mean` is `sum / n` integer division. Used by both `ScenarioDiagnosticsReport` and future tooling. No `f64`.

### D3: `SlotKind` visibility promotion + serde derives

`SlotKind` already exists at `crates/worldwake-ai/src/agent_tick/portfolio.rs` (variants `Survival`, `Commitment`, `Economic`). S144AGGSCEDIA-002 landed the D3 enabling change: it is now public and derives serde traits. Because `ScenarioDiagnosticsReport` (D1) is a `pub` type consumed by the `worldwake-cli` observer and must JSON-serialize (D7/D9/D10), `SlotKind`:

- is public and re-exported from `crates/worldwake-ai/src/lib.rs`,
- derives `Serialize, Deserialize`.

No variant or semantic change. This is a pure visibility/derive widening so the aggregation key is nameable and serializable outside the `agent_tick` module.

### D4: Aggregator

```rust
// crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs (new)
pub fn build_scenario_diagnostics(
    decision_traces: &[AgentDecisionTrace],
    plan_traces: &[PlanAttemptTrace],
    repair_traces: &[RepairAttemptTrace],
    event_log: &EventLog,
    tick_range: (Tick, Tick),
) -> ScenarioDiagnosticsReport;
```

Pure function. Single pass through each trace collection; deterministic sort + percentile computation; no I/O. `EventLog` exposes no whole-log iterator, so the aggregator walks the event log via `events_at_tick(tick)` across `tick_range` for decision payloads and filters targeted `events_by_tag(EventTag::QueueGrantPromoted)` / `QueueGrantExpired` / `QueueHeadFailed` / `ContentionResolved` lookups by `tick_range` for coordination metrics. Performance-metric inputs are read from the existing `OpportunityCompilerLoad` carrier on `AgentDecisionTrace` (`crates/worldwake-ai/src/decision_trace.rs:99`, struct at `:889`) and `SearchExpansionSummary` on `PlanAttemptTrace` — no new carrier type is introduced. Test coverage proves identical reports across reruns.

### D5: `CandidateSuppressionCategory` enum

```rust
// crates/worldwake-ai/src/scenario_diagnostics/mod.rs (new — net-new aggregation key)
pub enum CandidateSuppressionCategory {
    // Post-generation suppression — sourced from event-log `GoalSuppressed`
    // decision events, whose payload carries `GoalRejectionReason`
    // (crates/worldwake-core/src/decision_event_payload.rs).
    RejectedLowerMotive,
    RejectedFeasibilityProbeFailed,
    RejectedSuppressedByBlocker,
    RejectedSuppressedByDiscrepancy,
    RejectedSuppressedByStressPolicy,
    RejectedSuppressedByContentionPreempt,
    RejectedArbitrationLost,
    RejectedSwitchMarginInsufficient,
    // Ranking-stage filtering — sourced from `CandidateTrace` on
    // `AgentDecisionTrace` (crates/worldwake-ai/src/decision_trace.rs:289,371).
    ZeroMotive,           // CandidateTrace.zero_motive
    SoftDamped,           // CandidateTrace.damped
    FullyBlockedDesire,   // CandidateTrace.fully_blocked_desires
    SituationallySuppressed, // CandidateTrace.suppressed (no reason carried)
    // Pre-generation omission — sourced from `CandidateTrace.omitted_*`.
    OmittedPolitical,
    OmittedBandit,
    OmittedSocial,
    OmittedViolationDetection,
}
```

This is a **net-new** aggregation-key enum living in the `scenario_diagnostics` module — it is not a migration of any existing type. (There is no `SuppressionReason` enum and no `candidate_trace.rs` file in the codebase; candidate suppression today is already typed, scattered across `GoalRejectionReason`, `CandidateDampingReason`, and four `*OmissionReason` enums plus the bare `CandidateTrace.suppressed` list.) `CandidateSuppressionCategory` unifies those distinguishable sources into one histogram key so the report does not have to expose a fragmented multi-enum surface. The variant set above is grounded strictly in what the aggregator's inputs can actually distinguish; finer per-spec pruning reasons (e.g., commodity-relevance vs. belief-gating) are *not* separable from the trace data and are therefore not represented. The enum derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize` so it can key a `BTreeMap` and serialize.

### D6: Observer Section 13 renderer

```rust
// crates/worldwake-cli/src/bin/observer.rs
fn render_scenario_diagnostics_section(
    report: &ScenarioDiagnosticsReport,
    options: &DiagnosticsRenderOptions,
    out: &mut impl Write,
) -> io::Result<()>;
```

Section 13 is the next available section number after Section 12, Contention. Default text format renders tables. JSON format emits the report through a deterministic JSON representation owned by the observer renderer; it must not rely on raw JSON object keys for payload-bearing enum maps. The renderer follows the existing `render_*_section` naming pattern from observer.rs. The landed renderer uses a local `DiagnosticsRenderOptions` carrier so one call carries format, percentile-column selection, and top-N caps together.

### D7: CLI flags

The observer binary parses CLI args via `clap` derive (`ObserverCli` struct). New flags follow the existing `#[arg(...)]` attribute pattern:

- `--diagnostics-format text|json` (default: `text`)
- `--diagnostics-percentiles 50,95,99` (override the default percentile set)
- `--diagnostics-top-n 10` (cap rendered map entries; histograms get a top-N + "...others")
- `--no-diagnostics` (opt out; default is *on* for runs where the observer would render)

### D8: Performance and coordination counter instrumentation

The aggregator consumes existing traces unchanged where possible. Two categories of metric require a small, read-only, AI-crate-local instrumentation widening — all deterministic logical counts, never wall-clock:

- **Opportunity-compiler load** — already carried: `AgentDecisionTrace.opportunity_compiler_load: Option<OpportunityCompilerLoad>` exposes `compiled_count`, `salience_floored`, `learned_memory_damped`, `cap_truncated` (all `u32`). The aggregator reads these directly; no instrumentation needed.
- **Search expansions** — already carried: `SearchExpansionSummary` on `PlanAttemptTrace` carries per-expansion counts. The aggregator rolls these up; no instrumentation needed.
- **Snapshot cache counters** — `PlanningSnapshot` (`crates/worldwake-ai/src/planning_snapshot.rs`) holds precomputed `DistanceMatrix` travel/cost caches. D8 adds read-only `u64` logical counters to the matrix-backed accessors and surfaces them through `AgentDecisionTrace.snapshot_cache_counters`. A matrix lookup returning `Some(_)` counts as a hit, `None` counts as a miss, and `cache_invalidation_count` remains `0` because these matrices are rebuilt with the snapshot rather than invalidated incrementally. This is a derived read-model addition (FND-27), not authoritative state, and changes no planning behavior.
- **Queue wait ticks** — read from event-log events tagged `QueueGrantPromoted` / `QueueGrantExpired` / `QueueHeadFailed` / `ContentionResolved` (the queue-grant `EventTag` variants in `crates/worldwake-core/src/event_tag.rs`). `ResourceExtractionQueues` is an ECS `Component`, not an event emitter — the *system* emits these tags. The aggregator computes granted-claimant wait distributions from `ContentionEventPayload` claimant `arrived_tick` to payload `at_tick`; expiration and head-failure tags feed abandoned-grant and dead-claimant cleanup counts.

Fold-rejected metrics (require new event tags S144 will not introduce):

- **Contract bid/award/failure counts** — the event log has no `Contract`/`Bid`/`Award` `EventTag` variants. These are noted as future work and are intentionally absent from `CoordinationMetrics` (D1). A future spec that introduces contract event tags can add them.

The rule stands: if a metric requires emitting a new event tag, S144 fold-rejects that metric and notes it as future work. The first-shipped report is what is reachable without new event tags.

### D9: Golden coverage

`crates/worldwake-ai/tests/golden_scenario_diagnostics.rs` (new) covers:
- Determinism: same scenario, same seed → identical report.
- Schema coverage: every `ScenarioDiagnosticsReport` field is present and deterministic for a known scenario (`survival-baseline.ron`), including deterministic zero values for fields that the live scenario does not exercise.
- Top-N coverage: the raw diagnostics report contains more than three candidate groups, so the observer top-N renderer has overflow data available. Direct observer text rendering remains covered by the observer-focused tests.
- JSON format: the observer JSON representation parses back to an identical structure.

### D10: Survival-baseline diagnostics regression fixture

A committed `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` for `survival-baseline.ron` at the project's standard seed. Any change that shifts diagnostics requires the fixture to be regenerated and reviewed. Because the report carries only deterministic logical counts (Design Goal 2), this fixture is byte-stable across reruns.

## FND-01 Section H Analysis

### Information-Path Analysis

S144 introduces no new information flows in the simulated world. All inputs are existing trace collections and the existing event log. The report is read-only output to developer tooling.

### Positive-Feedback Analysis

Not applicable. The diagnostics aggregator is a pure observer.

### Concrete Dampeners

Not applicable.

### Stored State vs. Derived Read-Model List

**Stored state**: None new.

**Derived read-model**: `ScenarioDiagnosticsReport` and all its sub-types are derived from append-only traces + event log. Recompute from sources yields an identical report. The D8 logical cache counters on `PlanningSnapshot` are likewise derived read-model state (run-local counts of cache events), never authoritative — they are reset/recomputed with each run and influence no planning decision.

## SystemFn Integration

Not applicable. S144 introduces no new `SystemFn`.

## Component Registration

Not applicable. S144 introduces no new ECS component.

## Cross-System Interactions

- Reads existing trace collections produced by `worldwake-ai`'s decision pipeline.
- Reads existing event log via the `EventLog` accessor surface (`events_at_tick`, `events_by_tag`, etc.).
- Renders through `worldwake-cli` observer binary; no other crate consumes it.

No system-to-system mutation. Pure derived view per FND-26 / FND-27.

## Profile-Driven Parameters

Not applicable. S144's only authored parameters are CLI flags (`--diagnostics-top-n`, etc.). No per-agent profile additions.

## Test Plan

- Determinism golden (same scenario + seed → byte-identical report).
- Schema-coverage golden for `survival-baseline.ron`.
- JSON round-trip test.
- Aggregator unit tests for each metric category (build small `&[AgentDecisionTrace]` fixtures, assert aggregator output).
- `cargo clippy --workspace --all-targets -- -D warnings` clean.

## Outcome

Completed: 2026-05-14.

S144 landed the aggregate scenario diagnostics surface as a read-only derived observer/tooling layer. The implementation added deterministic integer percentile support, public/serde-ready portfolio slot keys, logical planning-snapshot cache counters, the `ScenarioDiagnosticsReport` type tree, the pure `build_scenario_diagnostics` aggregator, observer Section 13 with text/JSON output and diagnostics flags, and a committed `survival-baseline.ron` diagnostics regression fixture.

The completed ticket chain is archived as:
- `archive/tickets/S144AGGSCEDIA-001.md`
- `archive/tickets/S144AGGSCEDIA-002.md`
- `archive/tickets/S144AGGSCEDIA-003.md`
- `archive/tickets/S144AGGSCEDIA-004.md`
- `archive/tickets/S144AGGSCEDIA-005.md`
- `archive/tickets/S144AGGSCEDIA-006.md`
- `archive/tickets/S144AGGSCEDIA-007.md`

Deviations from the original plan: the first shipped report stayed strictly within existing traces and event tags. Contract bid/award/failure counts remain fold-rejected because the event log has no contract event tags. The golden proof shape was narrowed to the live deterministic JSON representation owned by the observer/fixture path instead of assuming payload-bearing enum maps could be emitted as raw JSON object keys.

Verification recorded by the archived ticket chain:
- Focused core, AI, and observer tests for `PercentileBucket`, `SlotKind` serde/public surface, cache counters, report type serde, aggregation metrics, observer Section 13, diagnostics CLI flags, and the deterministic survival-baseline fixture.
- `crates/worldwake-ai/tests/golden_scenario_diagnostics.rs` covers deterministic report generation and the committed fixture.
- Final broad gates in the S144 chain included `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.
