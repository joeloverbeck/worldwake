# S157SNAADMPRO-001: Admission-source enum and per-entity recording

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planner snapshot construction (`planning_snapshot.rs`)
**Deps**: None

## Problem

`PlanningSnapshot` admits entities for several distinct lawful reasons — the actor itself,
same-tick co-located physical observation (FND-14A), belief-store last-seen memory, grounded
evidence carriers, included public-topology places, and the possession/containment frontier —
but `collect_entities()` (`crates/worldwake-ai/src/planning_snapshot.rs:1411`) merges them all
into a single `BTreeSet<EntityId>`, after which the reason each entity was admitted is lost.
`build_planning_snapshot()` then builds a uniform `SnapshotEntity` for every admitted id with no
provenance. The planner therefore cannot prove *why* an entity is visible, nor assert that a
later read of a field only touches entities whose admission source legitimately exposes it. This
ticket adds an explicit admission-source enum recorded per admitted entity, turning an invariant
currently held by convention into one the planner can assert and trace (S157 D1; FND-15, FND-29).

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `collect_entities()` exists at `crates/worldwake-ai/src/planning_snapshot.rs:1411` and returns
   `BTreeSet<EntityId>`, merging `actor`, `evidence_entities`, `included_places`, and a
   possession/containment frontier (verified 2026-05-20). The actor-place branch reads
   `view.entities_at(*place)` (same-tick local observation, FND-14A); non-actor places admit
   entities whose belief `last_known_place == Some(*place)` (last-seen memory). The frontier walk
   adds direct containers/possessors. These branches are the lawful admission carriers the new
   enum must distinguish.
2. `build_planning_snapshot()` (`planning_snapshot.rs:1289`) and the per-entity factory
   `build_snapshot_entity()` (`planning_snapshot.rs:1026`) build a `SnapshotEntity`
   (`planning_snapshot.rs:215`) for every admitted id. `SnapshotEntity` derives
   `Clone, Debug, Eq, PartialEq` (line 214) and has a `Default` impl (line 229). `PlanningSnapshot`
   (`planning_snapshot.rs:438`, `entities: BTreeMap<EntityId, SnapshotEntity>` at line 442) carries
   **no** `Serialize`/`Deserialize` derive — it is a transient derived read-model rebuilt each
   planning pass (S157 H.4), so there is no serde-default or `SAVE_FORMAT_VERSION` concern for the
   new field.
3. Shared boundary under audit: the entity-admission path between `collect_entities()` (which
   currently discards provenance) and `build_planning_snapshot()`/`build_snapshot_entity()` (which
   stores the per-entity read-model). The new source must be threaded from the admitting branch in
   `collect_entities()` through to the `SnapshotEntity` written for that id; the data contract is
   "every admitted id carries exactly one admission source."
4. Existing focused coverage exercising these admission branches (must continue to pass, extended
   to assert the recorded source where natural): `build_snapshot_includes_actor_evidence_and_places_within_horizon`
   (`planning_snapshot.rs:2486`), `snapshot_filter_containment_walk_includes_inventory` (2443),
   `build_snapshot_does_not_pull_in_unreachable_places_without_evidence` (2641),
   `build_snapshot_excludes_remote_unbelieved_facility_within_horizon` (2674),
   `build_snapshot_uses_belief_summary_for_remote_facility_visibility` (2710).
5. The admission-source enum is `worldwake-ai`-local (it lives on `SnapshotEntity`, an ai-crate
   read-model type, not an ECS component) — no `worldwake-core` residence constraint and no
   `component_schema.rs` registration applies. Field types resolve within the ai crate.

## Architecture Check

1. The source is a fieldless enum stored directly on `SnapshotEntity`, co-located with the
   entity's other sub-structs, rather than a parallel `BTreeMap<EntityId, AdmissionSource>` on
   `PlanningSnapshot`. Co-location keeps the read-model self-describing (the source travels with
   the entity it qualifies) and avoids a second map that could drift out of sync with `entities`.
2. The source annotates a derived read-model; it never becomes authoritative world state (FND-27).
   `PlanningSnapshot` remains a cache rebuilt each pass — deleting it and recomputing yields the
   same sources. No backward-compat shim is introduced: `collect_entities()`'s return type changes
   in place (no parallel old/new collection path), and all callers are updated in this ticket.

## Verification Layers

1. Recorded source matches the admitting branch (self / local same-tick / belief last-seen /
   evidence / topology / hypothetical) -> focused unit tests over `build_planning_snapshot()` /
   `collect_entities()` asserting `snapshot.entities[id]`'s source per construction scenario.
2. Every admitted id carries exactly one source (no id in `entities` without a source) -> the
   type system enforces this (the field is non-`Option` on `SnapshotEntity`); a focused test
   confirms the actor, an evidence entity, a co-located entity, and a frontier entity each receive
   their expected variant.
3. Single-layer ticket (snapshot construction only) — no action-trace or event-log surface is
   involved because the snapshot is transient planner-internal state, not an authoritative
   mutation. Behavior-preservation of downstream planning is proven by the unchanged existing
   golden suite (run as the full-suite command), not by a new event-log assertion.

## What to Change

### 1. Add the admission-source enum

Define an `AdmissionSource` enum in `crates/worldwake-ai/src/planning_snapshot.rs` with one
variant per lawful carrier the snapshot already uses:

- `SelfAuthoritative` — the planning actor itself.
- `LocalSameTickPhysical` — co-located entity read via `view.entities_at(actor_place)` (FND-14A).
- `BeliefLastSeen` — admitted because a belief's `last_known_place` matched an included place.
- `GroundedEvidence` — member of `evidence_entities`.
- `PublicTopology` — an included place (public place graph).
- `PossessionContainmentFrontier` — reached by the container/possessor frontier walk.
- `HypotheticalPlannerEffect` — entity materialized by a hypothetical planner effect, if such
  ids enter the snapshot (confirm during implementation; if no such path feeds
  `build_planning_snapshot`, omit this variant rather than declaring it dead — see Out of Scope).

Derive `Clone, Copy, Debug, Eq, PartialEq` (matches `SnapshotEntity`'s derives; `Copy` is free for
a fieldless enum). Add `Ord, PartialOrd` only if a test or accessor needs ordering.

### 2. Thread the source through `collect_entities()`

Change `collect_entities()` to return the admission source alongside each id (e.g.,
`BTreeMap<EntityId, AdmissionSource>` instead of `BTreeSet<EntityId>`), assigning the variant at
each admitting branch: actor → `SelfAuthoritative`; `evidence_entities` → `GroundedEvidence`;
`included_places` → `PublicTopology`; actor-place `entities_at` reads → `LocalSameTickPhysical`;
non-actor `last_known_place` belief matches → `BeliefLastSeen`; frontier additions →
`PossessionContainmentFrontier`. When an id is admitted by more than one branch, define a
deterministic precedence (recommended: `SelfAuthoritative` > `LocalSameTickPhysical` >
`GroundedEvidence` > `BeliefLastSeen` > `PossessionContainmentFrontier` > `PublicTopology`) and
document it inline; the strongest/most-direct carrier wins so a field-legality check is not
weakened by a coincidental weaker admission.

### 3. Store the source on `SnapshotEntity`

Add an `admission: AdmissionSource` field to `SnapshotEntity`. Populate it in
`build_snapshot_entity()` / `build_planning_snapshot()` from the per-id source produced by
`collect_entities()`. Give the `Default` impl (line 229) a defensible default
(`PublicTopology` is the weakest/most-conservative carrier) so test-only `SnapshotEntity::default()`
construction still compiles; real construction always sets the field explicitly.

## Files to Touch

- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — add `AdmissionSource`, reshape
  `collect_entities()` return, add `SnapshotEntity.admission` field + `Default`, populate in
  `build_snapshot_entity`/`build_planning_snapshot`)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export `AdmissionSource` only if it must be
  visible to ticket 003's trace surface or to tests outside the module; confirm during
  implementation)

## Out of Scope

- Source-restricted strategic scans (ticket 002 consumes the recorded source).
- Surfacing the source in the decision/snapshot trace (ticket 003).
- Declaring an `AdmissionSource` variant for a carrier that does not actually feed
  `build_planning_snapshot` today — do not add dead variants (FND-28). If hypothetical-effect ids
  never reach the snapshot, omit `HypotheticalPlannerEffect`.
- Any belief-view accessor change (owned by S155, landed).

## Acceptance Criteria

### Tests That Must Pass

1. A focused test builds a snapshot with the actor, a grounded-evidence entity, a co-located
   entity at the actor's place, and a remote belief-known entity, and asserts each `entities[id]`
   carries the expected `AdmissionSource` variant.
2. A focused test asserts a frontier-admitted container/possessor receives
   `PossessionContainmentFrontier` (extends or sits beside
   `snapshot_filter_containment_walk_includes_inventory`).
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Every id present in `PlanningSnapshot.entities` carries exactly one `AdmissionSource`
   (enforced by the non-`Option` field type).
2. `PlanningSnapshot` remains non-serialized and transient — no `Serialize`/`Deserialize` derive
   is added, no `SAVE_FORMAT_VERSION` bump occurs.
3. The admission source is derived metadata over belief/world state; it is never written back to
   authoritative state (FND-27).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planning_snapshot.rs` (`#[cfg(test)]` after line 1484) — new focused
   tests asserting recorded source per admission branch (self / local / evidence / belief
   last-seen / topology / frontier).
2. Extend `build_snapshot_includes_actor_evidence_and_places_within_horizon` (line 2486) to assert
   the actor's and evidence entities' sources, since it already constructs that admission mix.

### Commands

1. `cargo test -p worldwake-ai planning_snapshot`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`
