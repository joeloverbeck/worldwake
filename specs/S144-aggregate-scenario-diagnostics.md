# S144: Aggregate Scenario Diagnostics

**Status**: Draft

## Summary

The external assessment in `reports/ai-architecture-improvements.md` (Section 17, "Build the missing aggregate diagnostics now") calls this out as the prerequisite for all planning tuning: "Without aggregate metrics, every planning change will be guesswork." The observer binary already collects rich per-tick / per-agent traces (`AgentDecisionTrace`, `PlanAttemptTrace`, `SearchExpansionSummary`, `RepairAttemptTrace`, `CausalLinkCapHit`) and surfaces 9 anomaly detectors (`STUCK_AGENT`, `GEOGRAPHIC_CONVERGENCE`, `MAINTENANCE_STARVATION`, `RECIPE_MONOCULTURE`, `ACUTE_NEED_SPIKE`, `ACTION_LOOP`, `REDUNDANT_PERCEPTION`, `FAILED_ACTION_SPIRAL`, `SUSTAINED_CRITICAL_NEED`). It does *not* surface scenario-level rolled-up metrics: candidate counts by schema and slot, plan-attempt budget exhaustion rates, frontier-exhaustion ratios, beam-truncation ratios, plan-depth p50/p95/max, terminal-kind distribution, repair attempt/success ratio, invalidation-reason histograms, belief contradiction counts, source-reliability change counts, queue wait-time distributions, or snapshot build/cache cost trajectories.

S144 lands the missing aggregate-consumer layer as a new observer Section 13 (`Scenario Diagnostics`), backed by a deterministic `ScenarioDiagnosticsReport` produced over the full tick range of a run. The aggregator reads only the already-emitted trace data and the existing event log; it adds no engine-side coupling, no new authoritative state, and no new event tags. FND-29 (Debuggability is a product feature) makes this work first-class. The report is a tooling/observer boundary — it never mutates world meaning.

This spec is also the prerequisite that Phase 12's other accepted proposals reference when defending their priority. The rejected PR-9 (Incremental snapshots + multi-queue search) is queued for reassessment specifically against the diagnostics S144 surfaces.

## Phase and Status

Phase 12: AI Architecture Evolution — Draft

## Crates

- `worldwake-ai` — owns a new `scenario_diagnostics` module that defines the `ScenarioDiagnosticsReport` data type and the deterministic aggregator that consumes `Vec<AgentDecisionTrace>`, `Vec<PlanAttemptTrace>`, `Vec<RepairAttemptTrace>`, the event log, and snapshot-cost counters. Pure read-only computation.
- `worldwake-cli` — observer binary (`crates/worldwake-cli/src/bin/observer.rs`) gains a Section 13 renderer for `ScenarioDiagnosticsReport`. New CLI flags `--diagnostics-format=text|json`, `--diagnostics-percentiles`, `--diagnostics-top-n`.
- `worldwake-core` — exposes `PercentileBucket` (`worldwake-core/src/percentile.rs`, new) helper for deterministic percentile computation over integer collections. No new authoritative state.
- `worldwake-sim` — no change.
- `worldwake-systems` — no change.

## Dependencies

- S110 (Decision History Events, archived) — provides the decision-event payload S144 reads for terminal-kind, motive-source, and rejected-goal aggregation.
- S136 (Decision Event Payload Extension, archived) — provides the always-on payload structure the aggregator reads.
- S117 (Convergence/Maintenance Observer Smells, archived) — defines the existing detector pattern S144 extends.
- S120 (Survival Critical-Window Forensics, archived) — provides the precedent for deterministic derived-report surfaces.

## Design Goals

1. **Aggregate-only, never authoritative.** `ScenarioDiagnosticsReport` is a derived view over append-only traces + event log. Deleting it and recomputing produces an identical report.
2. **Deterministic.** Same seed, same scenario, same final tick → identical report.
3. **No new engine coupling.** No new `SystemFn`, no new event tags, no new ECS components. The aggregator reads existing surfaces only.
4. **Useful for tuning before tuning happens.** Specifically: per-goal-kind plan-attempt budget exhaustion ratio, frontier-exhaustion rate, p95 plan depth, repair attempted-vs-succeeded ratio, candidate-trace `CommodityIrrelevant`/belief-gated suppression counts.
5. **Anomaly detectors retained.** The existing 9 detectors keep their format; S144 adds a parallel "metrics" channel of rolled-up histograms and distributions.
6. **Streaming-friendly.** The aggregator can be invoked at the end of a run (single shot) or as a periodic snapshot (every N ticks). The current observer model is single-shot; periodic mode is supported but not the default.

## Non-Goals

- **No live engine metrics dashboard.** S144 is a post-run / on-demand report, not a real-time UI.
- **No new authoritative state.** The aggregator does not write back to the world.
- **No anomaly-detector behavior change.** S117/S118 detectors keep their current emission rules; this spec adds metrics alongside them.
- **No prescriptive thresholds.** S144 reports observed distributions; it does not enforce health contracts. The S119/S121 authored survival-health-contract surface is the contract-enforcement layer.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-12 (Performance May Compress Computation, Never Causality) | The aggregator is purely derived; world meaning is unchanged. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `ScenarioDiagnosticsReport` is explicitly a derived view; the spec calls it out as deletable and recomputable. |
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
    pub candidates_emitted_by_kind: BTreeMap<GoalKindDiscriminant, u64>,
    pub candidates_emitted_by_slot: BTreeMap<SlotKind, u64>,
    pub candidates_suppressed_by_reason: BTreeMap<SuppressionReason, u64>,
    pub top_k_not_planned: BTreeMap<GoalKindDiscriminant, u64>,
    pub active_intention_continuation_rate: Permille,
}

pub struct PlanningMetrics {
    pub plan_attempts: u64,
    pub plan_attempts_by_kind: BTreeMap<GoalKindDiscriminant, u64>,
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
    pub invalidation_reasons: BTreeMap<DiscrepancyKind, u64>,
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
    pub source_reliability_changes_by_topic: BTreeMap<TopicScope, u64>,
    pub false_rumor_propagation_count: u64,
    pub correction_latency: PercentileBucket,
}

pub struct CoordinationMetrics {
    pub queue_wait_ticks: PercentileBucket,
    pub reservation_conflict_count: u64,
    pub abandoned_grant_count: u64,
    pub dead_claimant_cleanup_count: u64,
    pub contract_bid_count: u64,
    pub contract_award_count: u64,
    pub contract_failure_count: u64,
}

pub struct PerformanceMetrics {
    pub snapshot_build_cost: PercentileBucket,
    pub candidate_extraction_cost: PercentileBucket,
    pub affordance_enumeration_cost: PercentileBucket,
    pub search_expansions: PercentileBucket,
    pub cache_hit_count: u64,
    pub cache_miss_count: u64,
    pub cache_invalidation_count: u64,
}
```

`Permille` for all ratios; `PercentileBucket` for distributions. No floats.

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

### D3: Aggregator

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

Pure function. Single pass through each trace collection; deterministic sort + percentile computation; no I/O. Test coverage proves identical reports across reruns.

### D4: `SuppressionReason` enum

```rust
// crates/worldwake-ai/src/candidate_trace.rs (extended)
pub enum SuppressionReason {
    CommodityIrrelevant,         // S94
    BeliefGated,                 // S83
    SaturatedObligation,         // S96
    CooldownActive,              // S107 / S130
    FrontierDamping,             // S130
    NoReachablePlace,            // existing
    ProfileGated,                // S111
    LegallyForbidden,            // S125
    BlockerMatched,              // S109
}
```

Extracts the existing candidate-trace counter sources into a typed enum so the aggregator does not encode strings.

### D5: Observer Section 13 renderer

```rust
// crates/worldwake-cli/src/bin/observer.rs
fn render_scenario_diagnostics_section(
    report: &ScenarioDiagnosticsReport,
    format: DiagnosticsFormat,
    out: &mut impl Write,
) -> io::Result<()>;
```

Default text format renders tables. JSON format emits the full report verbatim. The renderer follows the existing `render_section_*` pattern from observer.rs.

### D6: CLI flags

- `--diagnostics-format text|json` (default: `text`)
- `--diagnostics-percentiles 50,95,99` (override the default percentile set)
- `--diagnostics-top-n 10` (cap rendered map entries; histograms get a top-N + "...others")
- `--no-diagnostics` (opt out; default is *on* for runs where the observer would render)

### D7: Live wiring of currently-missing counters

The aggregator consumes existing traces unchanged where possible. Where a counter currently lives but is not surfaced, the aggregator may need a small AI-crate widening:

- `PerfTelemetry` (`crates/worldwake-ai/src/perf_telemetry.rs`) — already collects `snapshot_build_cost`, `candidate_extraction_cost`. Surface them via a `PerfTelemetrySnapshot` carrier on `AgentDecisionTrace`. No new authoritative state.
- `PlanningSnapshot` cache counters — extend the existing `Rc<RefCell<...>>` caches with read-only hit/miss counters surfaced through `PerfTelemetry`. No correctness change.
- Queue wait ticks — read from existing `ContentionGrant` events and `ResourceExtractionQueues` grant-issuance events; the aggregator computes wait distributions from event-pair latency.

If a metric requires emitting a new event tag, S144 *fold-rejects* that metric and notes it as future work. The first-shipped report is what's reachable without new event tags.

### D8: Golden coverage

`golden_scenario_diagnostics.rs` (new) covers:
- Determinism: same scenario, same seed → identical report.
- Schema coverage: every `ScenarioDiagnosticsReport` field is populated for a known scenario (`survival-baseline.ron`).
- Top-N coverage: `--diagnostics-top-n 3` produces 3 entries plus "...others" summary.
- JSON format: parses back to identical structure.

### D9: Survival-baseline diagnostics regression

A committed `tests/fixtures/expected-scenario-diagnostics.json` for `survival-baseline.ron` at the project's standard seed. Any change that shifts diagnostics requires the fixture to be regenerated and reviewed.

## FND-01 Section H Analysis

### Information-Path Analysis

S144 introduces no new information flows in the simulated world. All inputs are existing trace collections and the existing event log. The report is read-only output to developer tooling.

### Positive-Feedback Analysis

Not applicable. The diagnostics aggregator is a pure observer.

### Concrete Dampeners

Not applicable.

### Stored State vs. Derived Read-Model List

**Stored state**: None new.

**Derived read-model**: `ScenarioDiagnosticsReport` and all its sub-types are derived from append-only traces + event log. Recompute from sources yields identical report.

## SystemFn Integration

Not applicable. S144 introduces no new `SystemFn`.

## Component Registration

Not applicable. S144 introduces no new ECS component.

## Cross-System Interactions

- Reads existing trace collections produced by `worldwake-ai`'s decision pipeline.
- Reads existing event log via the `EventLog` accessor surface.
- Renders through `worldwake-cli` observer binary; no other crate consumes it.

No system-to-system mutation. Pure derived view per FND-26 / FND-27.

## Profile-Driven Parameters

Not applicable. S144's only authored parameters are CLI flags (`--diagnostics-top-n`, etc.). No per-agent profile additions.

## Test Plan

- Determinism golden (same scenario + seed → identical report bytes).
- Schema-coverage golden for `survival-baseline.ron`.
- JSON round-trip test.
- Aggregator unit tests for each metric category (build small `Vec<AgentDecisionTrace>` fixtures, assert aggregator output).
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
