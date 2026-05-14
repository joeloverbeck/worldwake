# S144AGGSCEDIA-005: Diagnostics aggregator

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S144AGGSCEDIA-003.md, archive/tickets/S144AGGSCEDIA-004.md

## Problem

S144's report type needs the deterministic aggregator that produces it — a single-pass pure function over the existing trace collections and event log. Without `build_scenario_diagnostics`, the report type (ticket 004) has no producer and the observer (ticket 006) has nothing to render.

## Assumption Reassessment (2026-05-14)

1. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` does not exist (confirmed via `ls`); `build_scenario_diagnostics` does not exist. Net-new pure function — no existing focused/unit, runtime trace, or golden/E2E coverage. Its inputs all exist: `AgentDecisionTrace` (`decision_trace.rs:94`), `PlanAttemptTrace` (`decision_trace.rs:1095`, with `PlanSearchOutcome::{BudgetExhausted, FrontierExhausted, Found}` and `expansion_summaries: Vec<SearchExpansionSummary>`), `RepairAttemptTrace` (`decision_trace.rs:106`, with `budget_consumed`/`budget_total`/`chosen_kind`), `EventLog` (`crates/worldwake-core/src/event_log.rs:27`).
2. S144 spec D4 (`specs/S144-aggregate-scenario-diagnostics.md`) specifies the signature `build_scenario_diagnostics(decision_traces, plan_traces, repair_traces, event_log, tick_range) -> ScenarioDiagnosticsReport` as a single-pass pure function with deterministic sort + percentile computation, no I/O. `EventLog` exposes no whole-log iterator — accessors are `events_at_tick`, `events_by_tag`, `events_by_actor`, `events_by_place`, `get` — so the aggregator walks `events_at_tick` across `tick_range` and `events_by_tag(EventTag::QueueGrantPromoted)` etc. for the coordination metrics. The queue-grant `EventTag` variants (`QueueGrantPromoted`, `QueueGrantExpired`, `QueueHeadFailed`, `ContentionResolved`) are confirmed present in `crates/worldwake-core/src/event_tag.rs`.
3. Mixed-layer shared boundary under audit: the aggregator consumes the `AgentDecisionTrace` carrier added by ticket 003 (`snapshot_cache_counters`) and produces the `ScenarioDiagnosticsReport` type defined by ticket 004. The contract under audit is the determinism guarantee — same trace inputs + tick range → byte-identical report.
4. Suppression and invalidation key sources: `candidates_suppressed_by_category` is built from event-log `DecisionEventPayload::GoalSuppressed` payloads (`GoalSuppressedPayload.reason: GoalRejectionReason`, `crates/worldwake-core/src/decision_event_payload.rs:152`) plus the `CandidateTrace` stage buckets on `AgentDecisionTrace.candidates` (`suppressed`, `damped`, `zero_motive`, `omitted_political`/`omitted_bandit`/`omitted_social`/`omitted_violation_detection`). `invalidation_reasons` keys on `Discrepancy` (`crates/worldwake-core/src/discrepancy.rs:9`); its three payload-bearing variants (`NeedHorizonExceeded`, `Omission`, `ArtifactNotActionable`) are grouped by variant discriminant so the histogram counts reasons, not payload permutations — this discriminant-grouping is this ticket's responsibility.

## Architecture Check

1. A single pure function over append-only traces + the append-only event log keeps the report fully derived and recomputable (FND-27, FND-29A) — it adds no engine coupling, reads no global state on behalf of any agent, and never mutates the world.
2. No backwards-compatibility aliasing/shims — net-new function; it reuses the existing trace surfaces and `EventLog` accessors rather than introducing a parallel transport path.

## Verification Layers

1. Determinism (same inputs → byte-identical report) -> focused unit test re-running the aggregator on identical fixtures.
2. Per-metric-category correctness (hand-built small trace fixtures → asserted aggregator output) -> focused unit tests, one per metric category.
3. Discriminant-grouping of payload-bearing `Discrepancy` variants (two `NeedHorizonExceeded` with different payloads → one histogram entry of count 2) -> focused unit test.
4. The aggregator is a pure read-only function — its proof surface is focused unit coverage over constructed trace fixtures; there is no action-trace or event-log-delta surface because the function only reads, so no additional layer mapping applies.

## What to Change

### 1. New `aggregator.rs` with `build_scenario_diagnostics`

Create `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` defining the pure `build_scenario_diagnostics` function per the S144 D4 signature. Single pass over each trace collection; deterministic sort before percentile computation via `PercentileBucket::from_sorted`.

### 2. Goal-pressure, planning, and revalidation/repair metrics

Roll up `candidates_emitted_by_kind`/`_by_slot`, suppression-by-category (from event-log `GoalSuppressed` payloads + the `CandidateTrace` stage buckets), plan-attempt budget/frontier exhaustion, plan-depth percentiles, and terminal-kind distribution from `PlanAttemptTrace`; repair attempt/success/budget from `RepairAttemptTrace`; invalidation reasons from event-log payloads keyed on `Discrepancy` discriminant.

### 3. Belief, coordination, and performance metrics

Belief metrics from decision/event surfaces (flat `source_reliability_changes: u64` — no by-topic breakdown); coordination metrics from `events_by_tag` queue-grant pairs (wait-tick distribution from `ContentionGrant` `granted_at`/`expires_at` latency); performance metrics from the existing `OpportunityCompilerLoad` carrier, `SearchExpansionSummary` counts, and the ticket-003 `snapshot_cache_counters` carrier.

### 4. Module wiring

Add `pub mod aggregator;` to `scenario_diagnostics/mod.rs` and re-export `build_scenario_diagnostics`.

## Files to Touch

- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (new)
- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (modify — submodule declaration + re-export)

## Out of Scope

- Observer rendering and CLI flags — ticket 006.
- Golden / fixture coverage — ticket 007.
- Any new event tag — S144 fold-rejects metrics that would need one (e.g. contract bid/award/failure counts are absent from `CoordinationMetrics` by design).
- Periodic (every-N-ticks) invocation mode — the spec supports it but the first-shipped path is single-shot.

## Acceptance Criteria

### Tests That Must Pass

1. `build_scenario_diagnostics` over a fixed set of trace fixtures produces a byte-identical report across repeated calls.
2. Each metric category (goal pressure, planning, revalidation/repair, belief, coordination, performance) is correctly rolled up from hand-built fixtures.
3. Payload-bearing `Discrepancy` variants are grouped by discriminant in `invalidation_reasons`.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The aggregator is pure — no I/O, no mutation of any input, no global-state reads (FND-26, FND-29A).
2. Output determinism — identical inputs always yield a byte-identical report (AGENTS.md determinism invariant).
3. No new event tags and no engine coupling are introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs` (inline `#[cfg(test)]`) — determinism re-run; one focused test per metric category over small `Vec<AgentDecisionTrace>` / `Vec<PlanAttemptTrace>` / `Vec<RepairAttemptTrace>` fixtures; `Discrepancy` discriminant-grouping test.

### Commands

1. `cargo test -p worldwake-ai scenario_diagnostics`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `cargo test -p worldwake-ai` (narrow boundary — this ticket touches only `worldwake-ai`)
