# S157: Planner Snapshot Admission Provenance

## Summary

Add an explicit **admission source** to every entity (and relevant non-self field) admitted
into `PlanningSnapshot`, plus a snapshot-admission trace, so the planner can prove *why* an
entity is visible and cannot read fields that were admitted for an unrelated reason. Today
`SnapshotEntity`'s sub-structs carry no provenance tag; `collect_entities()` returns a uniform
set, and strategic search scans `state.snapshot().entities.keys()` for workstations, sellers,
resource sources, and acquisition places. That scan is only sound if snapshot admission is
airtight — which the planner currently cannot self-verify.

**This spec is DEFERRED and intentionally OUT of the active implementation order.** It is
defense-in-depth and a debuggability/provenance improvement (FND-15, FND-29), not a correctness
fix. The actual remote-truth leak is closed at the source by **S155** (belief-view boundary
correctness); once the snapshot is built from a belief-correct view, source tagging hardens and
explains the boundary rather than repairing it. Implement only after S155 lands and the team
chooses to invest in snapshot-level provenance.

## Phase

AI Architecture Consolidation (Adjunct Wave — derived from `reports/ai-architecture-consolidation-first-iteration.md`) — **Deferred**

## Status

DRAFT — DEFERRED (not scheduled in active order)

## Crates

- `worldwake-ai` (`planning_snapshot.rs`, `search/strategic.rs`, decision-trace types)

## Dependencies

- **S155 (Belief-View Boundary Correctness)** — must land first; this spec assumes the belief
  surface feeding the snapshot is already belief-correct.

## Problem Statement

### Motivation and evidence

`reports/ai-architecture-consolidation-first-iteration.md` (Findings #2 and #13, rated
*High* and *Medium/High*) flagged that snapshot admission lacks source metadata and that
strategic search scans the admitted entity map directly. Verified:

- `SnapshotEntity` (`planning_snapshot.rs`) is composed of `entity/spatial/inventory/combat/
  social/economic/political/temporal/profiles/facility/control` sub-structs — **none** carries
  an admission source, acquisition tick, or confidence.
- `collect_entities()` merges actor + evidence entities + included places + possession/
  containment frontier into a single `BTreeSet<EntityId>`; after collection the reason an
  entity was admitted is lost.
- `build_planning_snapshot()` then builds a `SnapshotEntity` uniformly for every admitted id.
- `search/strategic.rs` scans `state.snapshot().entities.keys()` to find workstations, sellers,
  resource sources, and acquisition places.

### Why this matters

FND-15 says beliefs should carry provenance (source, claimed/acquired time, confidence) "where
relevance matters." FND-29 requires the engine to answer "why does this agent know/consider
this?" A planner that admits a remote entity as an evidence carrier and later reads its current
location/inventory/occupants — without recording that those fields were never legitimately
admitted — cannot prove it is leak-free, even when (post-S155) it happens to be. Source tagging
turns an invariant currently held by convention into one the planner can assert and trace.

### Why deferred

S155 removes the live-truth fallback at the belief accessors that feed snapshot construction,
so the snapshot is built from belief-correct inputs and the amplification risk is largely
neutralized at the source. The report's heavier proposal (a generic `PlannerVisible<T>` wrapper,
four split view traits, and a hard "`worldwake-ai` must never receive `&World`" rule) is a large
surface that risks Option-C-style churn; this spec captures a **bounded** version (an admission
enum + trace + guarded iterators) to be scheduled deliberately, not bundled into the critical
fix wave.

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

## Deliverables (for when scheduled)

### D1 — Admission-source enum on snapshot entities
Add an admission-source enum (self-authoritative, local same-tick physical, belief-store claim,
last-seen memory, testimony/record, grounded-evidence carrier, public topology, hypothetical
planner effect) recorded per admitted entity in `PlanningSnapshot`. Populate it in
`collect_entities()`/`build_planning_snapshot()` from the path that admitted each entity.

### D2 — Source-restricted strategic scans
Replace raw `state.snapshot().entities.keys()` scans in `search/strategic.rs` with accessors
that restrict by admission source (e.g. `entities_admitted_for(predicate)` /
`visible_entities_by_source(...)`), so workstation/seller/resource/acquisition scans only see
entities whose source legitimately exposes those fields.

### D3 — Snapshot-admission trace
Surface per-entity admission source in the decision/snapshot trace for "why is this entity in
the planner's view?" debugging.

### D4 — Tests
Tests assert the recorded source for local, belief, last-seen, evidence, topology, and
hypothetical admissions, and that a strategic scan does not pick up an entity admitted for an
unrelated reason.

## Test Plan (for when scheduled)

1. Focused: `cargo test -p worldwake-ai planning_snapshot` (admission-source unit tests).
2. Golden: `cargo test -p worldwake-ai --test golden_ai` — no world-outcome regression.
3. `./scripts/verify.sh` before PR.
