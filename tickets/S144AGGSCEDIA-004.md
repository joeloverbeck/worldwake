# S144AGGSCEDIA-004: ScenarioDiagnosticsReport and CandidateSuppressionCategory types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S144AGGSCEDIA-001.md, archive/tickets/S144AGGSCEDIA-002.md

## Problem

S144 needs a deterministic `ScenarioDiagnosticsReport` data type — the aggregate-consumer surface for scenario-level planning metrics — plus the `CandidateSuppressionCategory` enum that keys its suppression histogram. Neither exists; the observer currently surfaces only per-tick traces and 9 anomaly detectors, no rolled-up metrics.

## Assumption Reassessment (2026-05-14)

1. `crates/worldwake-ai/src/scenario_diagnostics/` does not exist (confirmed via `ls`); no `ScenarioDiagnosticsReport` or `CandidateSuppressionCategory` type exists in the workspace (confirmed during S144 reassessment). Net-new module — no existing focused/unit, runtime trace, or golden/E2E coverage.
2. S144 spec D1+D5 (`specs/S144-aggregate-scenario-diagnostics.md`) specify the full type tree. Every key/value type the report uses is verified present and serde-ready: `GoalKind` (`crates/worldwake-core/src/goal.rs:62` — `Copy, Ord, Hash, Serialize, Deserialize`), `Discrepancy` (`crates/worldwake-core/src/discrepancy.rs:9` — `Copy, Ord, Serialize, Deserialize`), `PlanTerminalKind` (`crates/worldwake-ai/src/planner_ops.rs:387` — serde-ready), `Permille` (`crates/worldwake-core/src/numerics.rs:25`), `Tick` (`crates/worldwake-core/src/ids.rs:57`). `PercentileBucket` is provided by `archive/tickets/S144AGGSCEDIA-001.md`; `SlotKind` is made `pub` + serde-ready by ticket 002.
3. Shared abstraction boundary: this ticket defines the data contract `ScenarioDiagnosticsReport` that ticket 005 (aggregator) produces and ticket 006 (observer) renders. The contract under audit is the struct/enum shape and the requirement that the entire tree derives `Serialize, Deserialize` (for D7 JSON output, D9 round-trip, D10 fixture). `Discrepancy` carries three payload-bearing variants (`NeedHorizonExceeded`, `Omission`, `ArtifactNotActionable`); per S144 D1 the `invalidation_reasons` histogram groups payload-bearing variants by discriminant — this ticket only defines the field type `BTreeMap<Discrepancy, u64>`; the discriminant-grouping logic is the aggregator's responsibility (ticket 005).

## Architecture Check

1. A single derived-view module owns the entire report type tree, keeping `ScenarioDiagnosticsReport` deletable and recomputable (FND-27). `CandidateSuppressionCategory` lives in the same module because it is a net-new aggregation key with no meaning outside the report — it is not a migration of any existing type.
2. No backwards-compatibility aliasing/shims — `CandidateSuppressionCategory` is net-new; it unifies the existing scattered suppression sources (`GoalRejectionReason`, the `CandidateTrace` stage buckets) into one histogram key rather than aliasing any of them.

## Verification Layers

1. Whole-tree serde round-trip (`ScenarioDiagnosticsReport` → JSON → identical structure) -> focused unit test in the module.
2. `CandidateSuppressionCategory` is `Ord` + serde-ready (usable as a `BTreeMap` key and serializable) -> focused unit test in the module.
3. Single-layer ticket: this is a pure type-definition module with no decision-trace, action-trace, or event-log surface — additional layer mapping is not applicable.

## What to Change

### 1. New `scenario_diagnostics` module with the report type tree

Create `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` defining `ScenarioDiagnosticsReport` and its six sub-structs (`GoalPressureMetrics`, `PlanningMetrics`, `RevalidationRepairMetrics`, `BeliefMetrics`, `CoordinationMetrics`, `PerformanceMetrics`) exactly as specified in S144 D1. Every struct derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `Permille` for ratios, `PercentileBucket` for distributions, `u64` for counts — no floats.

### 2. `CandidateSuppressionCategory` enum

In the same module, define `CandidateSuppressionCategory` per S144 D5 — the net-new aggregation-key enum unifying the post-generation `GoalRejectionReason` variants, the `CandidateTrace` ranking-stage buckets, and the pre-generation omission families. Derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.

### 3. Module registration

Add `pub mod scenario_diagnostics;` to `crates/worldwake-ai/src/lib.rs` and re-export `ScenarioDiagnosticsReport` and `CandidateSuppressionCategory`.

## Files to Touch

- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — module declaration + re-exports)

## Out of Scope

- The aggregator (`build_scenario_diagnostics`) — ticket 005.
- The discriminant-grouping logic for `Discrepancy` payload variants — ticket 005.
- Observer rendering and CLI flags — ticket 006.
- `aggregator.rs` — created by ticket 005 inside this module.

## Acceptance Criteria

### Tests That Must Pass

1. `ScenarioDiagnosticsReport` with all sub-structs populated round-trips through serde JSON to an equal value.
2. `CandidateSuppressionCategory` is usable as a `BTreeMap` key and round-trips through serde.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The entire `ScenarioDiagnosticsReport` tree derives `Serialize, Deserialize` — no field type breaks serializability.
2. No floats anywhere in the type tree — ratios are `Permille`, distributions are `PercentileBucket`, counts are `u64` (CLAUDE.md Determinism invariant).
3. `ScenarioDiagnosticsReport` carries no authoritative state — it is a derived view (FND-27).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (inline `#[cfg(test)]`) — full-tree serde JSON round-trip; `CandidateSuppressionCategory` BTreeMap-key + serde round-trip.

### Commands

1. `cargo test -p worldwake-ai scenario_diagnostics`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `cargo test -p worldwake-ai` (narrow boundary — this ticket touches only `worldwake-ai`)
