# S157: Planner Snapshot Admission Provenance

## Summary

Add an explicit **admission source** to every entity (and relevant non-self field) admitted
into `PlanningSnapshot`, plus a snapshot-admission trace, so the planner can prove *why* an
entity is visible and cannot read fields that were admitted for an unrelated reason. Ticket
`S157SNAADMPRO-001` landed the per-entity `SnapshotEntity.admission` source tag and changed
`collect_entities()` to retain admission provenance. `S157SNAADMPRO-002` replaced the strategic
workstation, seller, resource-source, and acquisition-place scans with a source-restricted
physical-field accessor. `S157SNAADMPRO-003` surfaced opportunity-scoped snapshot-admission
traces in the decision trace.

This spec is **defense-in-depth** and a debuggability/provenance improvement (FND-15, FND-29),
not a correctness fix. The remote-truth leak was already closed at the source by **S155**
(belief-view boundary correctness, now COMPLETED — `archive/specs/S155-belief-view-boundary-correctness.md`);
the snapshot is now built from a belief-correct view, so source tagging *hardens and explains*
the boundary rather than repairing it. S155's landing satisfies this spec's only hard
prerequisite.

## Phase

AI Architecture Consolidation (Adjunct Wave — derived from `reports/ai-architecture-consolidation-first-iteration.md`)

## Status

COMPLETED

## Crates

- `worldwake-ai` (`planning_snapshot.rs`, `search/strategic.rs`, decision-trace types)

## Dependencies

- **S155 (Belief-View Boundary Correctness)** — **COMPLETED**, archived at
  `archive/specs/S155-belief-view-boundary-correctness.md`. It fixed the FND-14A remote-truth
  leak in `PerAgentBeliefView::effective_place()`, so the belief surface feeding the snapshot is
  now belief-correct. This spec's only hard prerequisite is therefore satisfied.

## Problem Statement

### Motivation and evidence

`reports/ai-architecture-consolidation-first-iteration.md` (Findings #2 and #13, rated
*High* and *Medium/High*) flagged that snapshot admission lacks source metadata and that
strategic search scans the admitted entity map directly. Current state:

- `SnapshotEntity` (`planning_snapshot.rs`) now carries an `admission: AdmissionSource` field
  alongside the existing `entity/spatial/inventory/combat/social/economic/political/temporal/
  profiles/facility/control` sub-structs.
- `collect_entities()` now returns `BTreeMap<EntityId, AdmissionSource>` and records actor,
  evidence, topology, local same-tick, belief-last-seen, and possession/containment-frontier
  admission sources.
- `build_planning_snapshot()` passes that source into `build_snapshot_entity()` for every admitted id.
- `search/strategic.rs` now uses a source-restricted physical-field accessor to find
  workstations, sellers, resource sources, and acquisition places.

### Why this matters

FND-15 says beliefs should carry provenance (source, claimed/acquired time, confidence) "where
relevance matters." FND-29 requires the engine to answer "why does this agent know/consider
this?" A planner that admits a remote entity as an evidence carrier and later reads its current
location/inventory/occupants — without recording that those fields were never legitimately
admitted — cannot prove it is leak-free, even when (now that S155 has landed) it happens to be.
Source tagging turns an invariant currently held by convention into one the planner can assert
and trace.

### Why this is bounded

S155 removed the live-truth fallback at the belief accessors that feed snapshot construction,
so the snapshot is now built from belief-correct inputs and the amplification risk is already
neutralized at the source. This spec is therefore not a correctness fix but defense-in-depth and
provenance hardening. The report's heavier proposal (a generic `PlannerVisible<T>` wrapper, four
split view traits, and a hard "`worldwake-ai` must never receive `&World`" rule) is a large
surface that risks Option-C-style churn; this spec deliberately captures only a **bounded**
version (an admission enum + trace + guarded iterators), leaving the heavier refactor as an
explicit Non-Goal.

## Design Goals

- Every entity admitted to `PlanningSnapshot` carries an explicit admission source.
- Strategic scans of the entity map are restricted to entities/fields legal for their admission
  source (or use an iterator that enforces the restriction).
- A snapshot-admission trace records, per admitted entity, why it was admitted — answering
  "why is this in the plan's view?"

## Non-Goals

- The generic `PlannerVisible<T>` wrapper and full four-trait view split from the report — out
  of scope; revisit only if the bounded enum proves insufficient.
- The blanket "`worldwake-ai` must never receive `&World`" architectural rule — a separate,
  larger refactor.
- Any belief-view accessor change — owned by S155.

## FOUNDATIONS Alignment

| Principle | How this spec satisfies it |
|-----------|----------------------------|
| FND-14 / FND-14A | Source tags make co-located/self/possessed reads (legal) distinguishable from belief/last-seen/evidence/topology admissions; scans cannot silently read fields admitted for another reason. |
| FND-15 (Provenance) | Admission source (and where relevant acquired tick/confidence) is recorded for planner-visible entities. |
| FND-27 (Derived summaries are caches) | The snapshot remains a derived read-model over belief/world state; source tags annotate, they do not become new truth. |
| FND-29 (Debuggability) | Admission trace answers "why is this entity in the planner's view, and which fields may it read?" |

## Section H — Causal Hooks Declaration

### H.1 Information-path analysis
No new information path; this annotates the *existing* admission path with its source so the
path is inspectable. Sources enumerate the lawful carriers already in use: self-authoritative,
local same-tick physical (FND-14A), belief-store claim, last-seen memory, testimony/record,
grounded-evidence carrier, public topology, and hypothetical planner effect.

### H.2 Positive-feedback analysis
None.

### H.3 Concrete dampeners
N/A (per H.2).

### H.4 Stored state vs. derived read-model
The `PlanningSnapshot` is and remains a transient derived read-model rebuilt each planning pass;
admission source is metadata on that derived model, never authoritative world state.

### H.5 Planner-formalism analysis
No formalism change. Strategic search keeps scanning admitted entities, but through
source-restricted accessors so it cannot consume a field illegitimately.

### Agent Profile Scenario Contract
N/A — no ECS component, no `Permille`, no profile parameter.

## Deliverables

### D1 — Admission-source enum on snapshot entities (landed by `S157SNAADMPRO-001`)
Add an admission-source enum (self-authoritative, local same-tick physical, belief-store claim,
last-seen memory, testimony/record, grounded-evidence carrier, public topology, hypothetical
planner effect) recorded per admitted entity in `PlanningSnapshot`. Populate it in
`collect_entities()`/`build_planning_snapshot()` from the path that admitted each entity.

The landed source is a fieldless enum stored on `SnapshotEntity` and derives
`Clone, Copy, Debug, Eq, PartialEq`. The implemented live variants are
`SelfAuthoritative`, `LocalSameTickPhysical`, `GroundedEvidence`, `BeliefLastSeen`,
`PossessionContainmentFrontier`, and `PublicTopology`. No `HypotheticalPlannerEffect` variant
landed because no live hypothetical-effect id path feeds `build_planning_snapshot`.
`PlanningSnapshot` remains a transient derived read-model and is **not** serialized
(no `Serialize`/`Deserialize`), so there is no serde-default or save/replay-compatibility concern
for the field.

### D2 — Source-restricted strategic scans (landed by `S157SNAADMPRO-002`)
Replaced raw `state.snapshot().entities.keys()` scans in `search/strategic.rs` with the
`entities_admitted_for_physical_fields()` accessor, so workstation/seller/resource/acquisition
scans only see entities whose source legitimately exposes those fields. The landed policy admits
`SelfAuthoritative`, `LocalSameTickPhysical`, `GroundedEvidence`, and `BeliefLastSeen` for these
physical/economic facility fields, while excluding topology-only and possession-frontier
admissions.

### D3 — Snapshot-admission trace (landed by `S157SNAADMPRO-003`)
Surfaced per-entity admission source in the decision/snapshot trace for "why is this entity in
the planner's view?" debugging. The landed trace is
`SnapshotAdmissionTrace { opportunity, entity, source }` on `AgentDecisionTrace`, with
`DecisionTraceSink` query support keyed by agent/tick. The `opportunity` field is included because
one traced planning tick can build several opportunity-specific snapshots.

### D4 — Tests (landed across `S157SNAADMPRO-001` through `S157SNAADMPRO-003`)
Tests assert recorded sources for self, local, evidence, topology, possession-frontier, and
belief-last-seen admissions, prove strategic scans do not pick up an entity admitted for an
unrelated reason, and prove the decision trace sink exposes snapshot-admission entries. No
hypothetical-admission case exists because no live hypothetical-effect id path feeds
`build_planning_snapshot`.

## Test Plan

1. Focused: `cargo test -p worldwake-ai planning_snapshot` (admission-source unit tests).
2. Crate-level: `cargo test -p worldwake-ai` — includes the golden integration target's
   non-ignored tests and proves no AI trace/test-helper fallout.
3. `./scripts/verify.sh` before PR.

## Outcome

Completed on 2026-05-20.

- `S157SNAADMPRO-001` added the per-entity `AdmissionSource` substrate to
  `PlanningSnapshot`.
- `S157SNAADMPRO-002` routed strategic workstation, seller, resource-source, and acquisition
  scans through the source-restricted physical-field accessor.
- `S157SNAADMPRO-003` surfaced opportunity-scoped snapshot-admission traces in
  `AgentDecisionTrace` and `DecisionTraceSink`.
- The landed enum intentionally excludes a hypothetical-planner-effect variant because no live
  hypothetical-effect id path feeds `build_planning_snapshot`.

Verification included focused admission-source and decision-trace tests plus
`cargo test -p worldwake-ai` during the ticket-family implementation. The final pre-push gate is
owned by the harness after spec archival.
