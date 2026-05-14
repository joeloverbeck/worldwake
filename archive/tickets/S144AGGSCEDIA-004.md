# S144AGGSCEDIA-004: ScenarioDiagnosticsReport and CandidateSuppressionCategory types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: archive/tickets/S144AGGSCEDIA-001.md, archive/tickets/S144AGGSCEDIA-002.md

## Problem

S144 needed a deterministic `ScenarioDiagnosticsReport` data type — the aggregate-consumer surface for scenario-level planning metrics — plus the `CandidateSuppressionCategory` enum that keys its suppression histogram. Before this ticket, neither existed; the observer surfaced only per-tick traces and 9 anomaly detectors, no rolled-up metrics.

## Assumption Reassessment (2026-05-14)

1. Before implementation, `crates/worldwake-ai/src/scenario_diagnostics/` did not exist (confirmed via `ls`); no `ScenarioDiagnosticsReport` or `CandidateSuppressionCategory` type existed in the workspace (confirmed during S144 reassessment). Net-new module — no existing focused/unit, runtime trace, or golden/E2E coverage.
2. S144 spec D1+D5 (`archive/specs/S144-aggregate-scenario-diagnostics.md`) specify the full type tree. Every key/value type the report uses is verified present and serde-ready: `GoalKind` (`crates/worldwake-core/src/goal.rs:62` — `Copy, Ord, Hash, Serialize, Deserialize`), `Discrepancy` (`crates/worldwake-core/src/discrepancy.rs:9` — `Copy, Ord, Serialize, Deserialize`), `PlanTerminalKind` (`crates/worldwake-ai/src/planner_ops.rs:387` — serde-ready), `Permille` (`crates/worldwake-core/src/numerics.rs:25`), `Tick` (`crates/worldwake-core/src/ids.rs:57`). `PercentileBucket` is provided by `archive/tickets/S144AGGSCEDIA-001.md`; `SlotKind` is made `pub` + serde-ready by ticket 002.
3. Shared abstraction boundary: this ticket defined the data contract `ScenarioDiagnosticsReport` that ticket 005 (aggregator) produces and ticket 006 (observer) renders. The contract under audit was the struct/enum shape and the requirement that the entire tree derives `Serialize, Deserialize` (for D7 JSON output, D9 round-trip, D10 fixture). `Discrepancy` carries three payload-bearing variants (`NeedHorizonExceeded`, `Omission`, `ArtifactNotActionable`); per S144 D1 the `invalidation_reasons` histogram groups payload-bearing variants by discriminant — this ticket only defines the field type `BTreeMap<Discrepancy, u64>`; the discriminant-grouping logic is the aggregator's responsibility (ticket 005).
4. Reassessment correction: derived serde is format-agnostic, but raw `serde_json` object-key serialization is only guaranteed for JSON-string-compatible map keys. Because `GoalKind` and `Discrepancy` include payload-bearing variants, ticket 006/007 own the deterministic JSON representation for full observer/fixture output. This ticket proves whole-tree serde through bincode and a JSON smoke round-trip over JSON-compatible sample keys.

## Architecture Check

1. A single derived-view module owns the entire report type tree, keeping `ScenarioDiagnosticsReport` deletable and recomputable (FND-27). `CandidateSuppressionCategory` lives in the same module because it is a net-new aggregation key with no meaning outside the report — it is not a migration of any existing type.
2. No backwards-compatibility aliasing/shims — `CandidateSuppressionCategory` is net-new; it unifies the existing scattered suppression sources (`GoalRejectionReason`, the `CandidateTrace` stage buckets) into one histogram key rather than aliasing any of them.

## Verified Layers

1. Whole-tree serde round-trip (`ScenarioDiagnosticsReport` -> bincode -> identical structure) -> focused unit test in the module.
2. JSON-compatible report sample round-trip (`ScenarioDiagnosticsReport` -> JSON -> identical structure) -> focused unit test in the module.
3. `CandidateSuppressionCategory` is `Ord` + serde-ready (usable as a `BTreeMap` key and serializable) -> focused unit test in the module.
4. Single-layer ticket: this is a pure type-definition module with no decision-trace, action-trace, or event-log surface — additional layer mapping is not applicable.

## Landed Changes

### 1. New `scenario_diagnostics` module with the report type tree

Added `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` defining `ScenarioDiagnosticsReport` and its six sub-structs (`GoalPressureMetrics`, `PlanningMetrics`, `RevalidationRepairMetrics`, `BeliefMetrics`, `CoordinationMetrics`, `PerformanceMetrics`) as specified in S144 D1. Every struct derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Ratios use `Permille`, distributions use `PercentileBucket`, and counts use `u64` — no floats.

### 2. `CandidateSuppressionCategory` enum

In the same module, added `CandidateSuppressionCategory` per S144 D5 — the net-new aggregation-key enum unifying the post-generation `GoalRejectionReason` variants, the `CandidateTrace` ranking-stage buckets, and the pre-generation omission families. It derives `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`.

### 3. Module registration

Added `pub mod scenario_diagnostics;` to `crates/worldwake-ai/src/lib.rs` and re-exported `ScenarioDiagnosticsReport` and `CandidateSuppressionCategory`.

### 4. Test dependency

Added `serde_json` as a `worldwake-ai` dev-dependency for focused JSON smoke coverage. `Cargo.lock` already contained the transitive crate; the lockfile now records `worldwake-ai`'s direct dev-dependency.

## Landed Files

- `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (added)
- `crates/worldwake-ai/src/lib.rs` (modified — module declaration + re-exports)
- `crates/worldwake-ai/Cargo.toml` (modified — dev-dependency for focused JSON proof)
- `Cargo.lock` (modified — direct `worldwake-ai` dev-dependency edge)

## Out of Scope

- The aggregator (`build_scenario_diagnostics`) — ticket 005.
- The discriminant-grouping logic for `Discrepancy` payload variants — ticket 005.
- Observer rendering and CLI flags — ticket 006.
- `aggregator.rs` — created by ticket 005 inside this module.

## Acceptance Criteria

### Tests Passed

1. `ScenarioDiagnosticsReport` with all sub-structs populated round-trips through bincode serde to an equal value.
2. `ScenarioDiagnosticsReport` with JSON-compatible sample keys round-trips through serde JSON to an equal value.
3. `CandidateSuppressionCategory` is usable as a `BTreeMap` key and round-trips through serde JSON.
4. Existing suite passed: `cargo test -p worldwake-ai`

### Invariants

1. The entire `ScenarioDiagnosticsReport` tree derives `Serialize, Deserialize` — no field type breaks format-agnostic serde serializability.
2. No floats anywhere in the type tree — ratios are `Permille`, distributions are `PercentileBucket`, counts are `u64` (AGENTS.md determinism invariant).
3. `ScenarioDiagnosticsReport` carries no authoritative state — it is a derived view (FND-27).

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/src/scenario_diagnostics/mod.rs` (inline `#[cfg(test)]`) — full-tree bincode serde round-trip, JSON-compatible report sample round-trip, and `CandidateSuppressionCategory` BTreeMap-key + serde JSON round-trip.

### Commands Passed

1. `cargo test -p worldwake-ai --lib scenario_diagnostics`
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
3. `cargo test -p worldwake-ai` (narrow boundary — this ticket touches only `worldwake-ai`)

## Outcome

Completed on 2026-05-14.

- Added the `worldwake_ai::scenario_diagnostics` report type tree and the `CandidateSuppressionCategory` histogram key.
- Re-exported `ScenarioDiagnosticsReport` and `CandidateSuppressionCategory` from `worldwake-ai`.
- Added focused module tests covering whole-tree serde, JSON-compatible sample output, and ordered/serializable suppression-category map keys.
- Kept the aggregator, discriminant grouping, observer renderer, CLI flags, golden fixture, and full JSON representation for payload-bearing map keys in their existing sibling tickets.

## Deviations

- Rebound the drafted "full-tree JSON round-trip" proof to two honest seams: bincode proves the full type tree's serde contract, while `serde_json` proves the report shape for JSON-compatible sample keys. Payload-bearing `GoalKind` and `Discrepancy` keys need a deterministic observer/fixture JSON representation owned by tickets 006/007, so the active S144 spec was truth-synced.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib scenario_diagnostics`.
- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
