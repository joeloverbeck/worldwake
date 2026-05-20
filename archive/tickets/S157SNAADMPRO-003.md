# S157SNAADMPRO-003: Snapshot-admission trace

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` decision/snapshot trace (`decision_trace.rs`)
**Deps**: `archive/tickets/S157SNAADMPRO-001.md`

## Problem

Once ticket 001 records why each entity was admitted to `PlanningSnapshot`, that provenance is
only useful for debugging if it is surfaced where a developer can inspect it. FND-29 requires the
engine to answer "why is this entity in the planner's view?" Before this ticket, the
decision/snapshot trace carried no admission provenance. This ticket surfaces the per-entity
`AdmissionSource` in the
decision/snapshot trace so the question is answerable from the authoritative trace surface rather
than guessed (S157 D3; FND-29).

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-ai/src/decision_trace.rs` exists and already hosts snapshot-adjacent trace
   types — `SnapshotCacheCounters`, `SnapshotContinuationTrace`, and `OpportunityCompilerLoad`
   (verified 2026-05-20). The implemented host is an opportunity-scoped
   `SnapshotAdmissionTrace { opportunity, entity, source }` collection on `AgentDecisionTrace`,
   because the traced planning loop can build more than one opportunity-specific
   `PlanningSnapshot` in a tick.
2. Ticket 001 added `AdmissionSource` to `SnapshotEntity` as crate-private substrate. This ticket
   widened `AdmissionSource` to a public enum and re-exported it from `lib.rs` because the public
   decision-trace surface now exposes the source value.
3. Shared boundary under audit: the decision-trace emission path reading `SnapshotEntity.admission`
   and writing it into the trace sink. This is a read-only trace surface — it adds no authoritative
   state and changes no planning decision, so no event-log or world-state layer is involved.
4. Coverage gap classification: no existing test asserts admission provenance in the trace (none
   exists yet). This ticket adds focused decision-trace coverage; it is not a golden/E2E concern
   because the trace contents, not a world outcome, are the contract (precision rule 3).

## Architecture Check

1. The admission source is surfaced as trace metadata derived from the `SnapshotEntity` read-model,
   not recomputed independently — single source of truth (ticket 001's recorded field). The trace
   does not promote the source to authoritative state (FND-27, FND-29).
2. No backward-compat path: the trace field/struct is net-new; nothing is aliased or shimmed.

## Verified Layers

1. The trace records, per admitted entity, its admission source -> decision-trace focused test:
   build a snapshot with a known admission mix, run the trace emission, and assert the trace
   surface reports the expected `AdmissionSource` per entity.
2. Single-layer ticket (trace surfacing only) — no action-trace, event-log, or world-state surface
   applies because the trace is read-only debug provenance with no authoritative effect. Decision
   trace is the strongest and correct proof surface for this provenance claim (precision rule 6).

## Landed Changes

### 1. Added admission provenance to the snapshot/decision trace

Added `SnapshotAdmissionTrace { opportunity, entity, source }` and
`AgentDecisionTrace::snapshot_admissions`, plus `DecisionTraceSink` storage/query helpers keyed by
agent/tick.

### 2. Populated it at snapshot-trace emission time

The traced planning loop now reads `SnapshotEntity.admission` for every entity in each
opportunity-specific `PlanningSnapshot` and records that source without recomputing admission.

## Landed Files

- `crates/worldwake-ai/src/decision_trace.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agent_tick/mod.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-ai/src/planning_snapshot.rs`
- `crates/worldwake-ai/src/lib.rs`
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`
- `crates/worldwake-ai/src/survival_forensics.rs`
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`
- `crates/worldwake-ai/tests/golden_harness/timeline.rs`
- `crates/worldwake-cli/src/bin/observer.rs`
- `crates/worldwake-visualizer/src/trace_buffers.rs`

## Out of Scope

- Recording the admission source (ticket 001).
- Source-restricted strategic scans (ticket 002).
- Adding any new authoritative state or changing planning decisions — this is read-only trace
  provenance.

## Acceptance Result

### Tests That Passed

1. A focused decision-trace test asserts the trace reports the expected `AdmissionSource` for an
   actor, an evidence entity, and a belief-last-seen entity in a constructed snapshot.
2. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. The trace's admission source equals `SnapshotEntity.admission` for the same id (no independent
   recomputation).
2. The trace surface is read-only debug provenance — it adds no authoritative state and alters no
   planning decision.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — `sink_records_snapshot_admissions_by_agent_tick`
   proves the decision trace sink exposes admission traces by agent/tick.
2. `crates/worldwake-ai/src/planning_snapshot.rs` —
   `build_snapshot_includes_actor_evidence_and_places_within_horizon` now also proves the emitted
   snapshot-admission entries preserve actor, evidence, and belief-last-seen sources.

### Commands Run

1. `cargo test -p worldwake-ai --lib -- --list | rg 'snapshot_admission|build_snapshot_includes_actor_evidence'`
2. `cargo test -p worldwake-ai --lib decision_trace::tests::sink_records_snapshot_admissions_by_agent_tick -- --exact`
3. `cargo test -p worldwake-ai --lib planning_snapshot::tests::build_snapshot_includes_actor_evidence_and_places_within_horizon -- --exact`
4. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-05-20.

- Added opportunity-scoped snapshot-admission trace records to `AgentDecisionTrace`.
- Populated admission traces from the canonical `SnapshotEntity.admission` field in the traced
  planning loop.
- Added sink/query support and public re-exports for `SnapshotAdmissionTrace` and
  `AdmissionSource`.
- Updated AI/CLI/visualizer test helpers for the added trace field.

## Deviations

- The landed trace includes the searched `OpportunityKey` in addition to entity and source so a
  single agent-tick can distinguish multiple opportunity-specific planning snapshots.
- No hypothetical-admission trace case landed because the live `AdmissionSource` enum has no
  hypothetical variant; ticket 001 recorded that no live hypothetical-effect id path feeds
  `build_planning_snapshot`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib -- --list | rg 'snapshot_admission|build_snapshot_includes_actor_evidence'`.
- Passed `cargo test -p worldwake-ai --lib decision_trace::tests::sink_records_snapshot_admissions_by_agent_tick -- --exact`.
- Passed `cargo test -p worldwake-ai --lib planning_snapshot::tests::build_snapshot_includes_actor_evidence_and_places_within_horizon -- --exact`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo test -p worldwake-visualizer --lib trace_buffers`.
