# S157SNAADMPRO-003: Snapshot-admission trace

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` decision/snapshot trace (`decision_trace.rs`)
**Deps**: S157SNAADMPRO-001

## Problem

Once ticket 001 records why each entity was admitted to `PlanningSnapshot`, that provenance is
only useful for debugging if it is surfaced where a developer can inspect it. FND-29 requires the
engine to answer "why is this entity in the planner's view?" Today the decision/snapshot trace
carries no admission provenance. This ticket surfaces the per-entity `AdmissionSource` in the
decision/snapshot trace so the question is answerable from the authoritative trace surface rather
than guessed (S157 D3; FND-29).

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-ai/src/decision_trace.rs` exists and already hosts snapshot-adjacent trace
   types — `SnapshotCacheCounters` (line 111), `SnapshotContinuationTrace` (line 1431),
   `OpportunityCompilerLoad` (line 981) (verified 2026-05-20). The exact host for per-entity
   admission provenance (a new field on an existing snapshot-trace struct vs. a new
   `SnapshotAdmissionTrace` struct keyed by `EntityId`) must be chosen during implementation
   against the current trace-emission flow; confirm where `PlanningSnapshot` is available at trace
   time before wiring.
2. This ticket depends on ticket 001 having added `AdmissionSource` to `SnapshotEntity` and (if
   needed) re-exported it from `lib.rs`. Confirm `AdmissionSource` is reachable from
   `decision_trace.rs` before emitting it; if 001 left it module-private, widen its visibility in
   001's scope or here as a minimal follow-on.
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

## Verification Layers

1. The trace records, per admitted entity, its admission source -> decision-trace focused test:
   build a snapshot with a known admission mix, run the trace emission, and assert the trace
   surface reports the expected `AdmissionSource` per entity.
2. Single-layer ticket (trace surfacing only) — no action-trace, event-log, or world-state surface
   applies because the trace is read-only debug provenance with no authoritative effect. Decision
   trace is the strongest and correct proof surface for this provenance claim (precision rule 6).

## What to Change

### 1. Add admission provenance to the snapshot/decision trace

Add a per-entity admission-source field (or a small `SnapshotAdmissionTrace { entity, source }`
record collection) to the appropriate snapshot-trace struct in `decision_trace.rs`. Derive the
standard trace derives used by sibling trace types in the file (confirm against the chosen host
struct's derives).

### 2. Populate it at snapshot-trace emission time

Where the planner emits the snapshot/decision trace, read `SnapshotEntity.admission` for each id
in `PlanningSnapshot.entities` and record it on the new trace surface, answering "why is this
entity in the planner's view?"

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add admission-provenance trace field/struct)
- `Likely: crates/worldwake-ai/src/planning_snapshot.rs` or the snapshot-trace emission site
  (modify — populate the trace from `SnapshotEntity.admission`; confirm the emission site with
  `grep` for where the snapshot trace is currently written)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export the new trace type if observer/golden
  consumers need it, mirroring sibling snapshot-trace exports)

## Out of Scope

- Recording the admission source (ticket 001).
- Source-restricted strategic scans (ticket 002).
- Adding any new authoritative state or changing planning decisions — this is read-only trace
  provenance.

## Acceptance Criteria

### Tests That Must Pass

1. A focused decision-trace test asserts the trace reports the expected `AdmissionSource` for an
   actor, an evidence entity, and a belief-last-seen entity in a constructed snapshot.
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The trace's admission source equals `SnapshotEntity.admission` for the same id (no independent
   recomputation).
2. The trace surface is read-only debug provenance — it adds no authoritative state and alters no
   planning decision.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` or the snapshot-trace test module — new focused
   test asserting per-entity admission provenance in the emitted trace.

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
