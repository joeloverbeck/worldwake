# S157SNAADMPRO-001: Admission-source enum and per-entity recording

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planner snapshot construction (`planning_snapshot.rs`)
**Deps**: None

## Problem

`PlanningSnapshot` admits entities for several distinct lawful reasons — the actor itself,
same-tick co-located physical observation (FND-14A), belief-store last-seen memory, grounded
evidence carriers, included public-topology places, and the possession/containment frontier —
but before this ticket `collect_entities()` (`crates/worldwake-ai/src/planning_snapshot.rs`)
merged them all into a single `BTreeSet<EntityId>`, after which the reason each entity was
admitted was lost. `build_planning_snapshot()` then built a uniform `SnapshotEntity` for every
admitted id with no provenance. The planner therefore could not prove *why* an entity was visible,
nor assert that a
later read of a field only touches entities whose admission source legitimately exposes it. This
ticket adds an explicit admission-source enum recorded per admitted entity, turning an invariant
currently held by convention into one the planner can assert and trace (S157 D1; FND-15, FND-29).

## Assumption Reassessment (2026-05-20)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Before implementation, `collect_entities()` existed at `crates/worldwake-ai/src/planning_snapshot.rs`
   and returned `BTreeSet<EntityId>`, merging `actor`, `evidence_entities`, `included_places`, and a
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
   discarded provenance before this ticket) and `build_planning_snapshot()`/`build_snapshot_entity()` (which
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

## Verified Layers

1. Recorded source matches the admitting branch (self / local same-tick / belief last-seen /
   evidence / topology / frontier) -> focused unit tests over `build_planning_snapshot()` /
   `collect_entities()` asserting `snapshot.entities[id]`'s source per construction scenario.
2. Every admitted id carries exactly one source (no id in `entities` without a source) -> the
   type system enforces this (the field is non-`Option` on `SnapshotEntity`); a focused test
   confirms the actor, an evidence entity, a co-located entity, and a frontier entity each receive
   their expected variant.
3. Single-layer ticket (snapshot construction only) — no action-trace or event-log surface is
   involved because the snapshot is transient planner-internal state, not an authoritative
   mutation. Behavior-preservation of downstream planning was covered by the unchanged existing
   golden suite through the full-suite command, not by an added event-log assertion.

## Landed Changes

### 1. Added the admission-source enum

Defined an `AdmissionSource` enum in `crates/worldwake-ai/src/planning_snapshot.rs` with one
variant per lawful carrier the snapshot uses:

- `SelfAuthoritative` — the planning actor itself.
- `LocalSameTickPhysical` — co-located entity read via `view.entities_at(actor_place)` (FND-14A).
- `BeliefLastSeen` — admitted because a belief's `last_known_place` matched an included place.
- `GroundedEvidence` — member of `evidence_entities`.
- `PublicTopology` — an included place (public place graph).
- `PossessionContainmentFrontier` — reached by the container/possessor frontier walk.
No `HypotheticalPlannerEffect` variant was added because no live hypothetical-effect id path feeds
`build_planning_snapshot` in this ticket's owned surface.

The enum derives `Clone, Copy, Debug, Eq, PartialEq`; no ordering derive was needed.

### 2. Threaded the source through `collect_entities()`

Changed `collect_entities()` to return the admission source alongside each id as
`BTreeMap<EntityId, AdmissionSource>` instead of `BTreeSet<EntityId>`, assigning the variant at
each admitting branch: actor → `SelfAuthoritative`; `evidence_entities` → `GroundedEvidence`;
`included_places` → `PublicTopology`; actor-place `entities_at` reads → `LocalSameTickPhysical`;
non-actor `last_known_place` belief matches → `BeliefLastSeen`; frontier additions →
`PossessionContainmentFrontier`. When an id is admitted by more than one branch, deterministic
precedence keeps the strongest/most-direct carrier:
`SelfAuthoritative` > `LocalSameTickPhysical` > `GroundedEvidence` > `BeliefLastSeen` >
`PossessionContainmentFrontier` > `PublicTopology`.

### 3. Stored the source on `SnapshotEntity`

Added an `admission: AdmissionSource` field to `SnapshotEntity`. `build_planning_snapshot()` passes
the per-id source from `collect_entities()` into `build_snapshot_entity()`. The `Default` impl uses
`PublicTopology`, the weakest/most-conservative carrier, so test-only `SnapshotEntity::default()`
construction still compiles; real construction sets the field explicitly.

## Landed Files

- `crates/worldwake-ai/src/planning_snapshot.rs` — added `AdmissionSource`, reshaped
  `collect_entities()` to return `BTreeMap<EntityId, AdmissionSource>`, added
  `SnapshotEntity.admission`, and populated the field during snapshot construction.
- `crates/worldwake-ai/src/lib.rs` — no change; ticket 001 did not require an external re-export.

## Out of Scope

- Source-restricted strategic scans (ticket 002 consumes the recorded source).
- Surfacing the source in the decision/snapshot trace (ticket 003).
- Declaring an `AdmissionSource` variant for a carrier that does not actually feed
  `build_planning_snapshot` today — do not add dead variants (FND-28). If hypothetical-effect ids
  never reach the snapshot, omit `HypotheticalPlannerEffect`.
- Any belief-view accessor change (owned by S155, landed).

## Acceptance Result

### Tests Passed

1. A focused test builds a snapshot with the actor, a grounded-evidence entity, a co-located
   entity at the actor's place, and a remote belief-known entity, and asserts each covered
   `entities[id]` carries the expected `AdmissionSource` variant.
2. A focused test asserts a frontier-admitted container/possessor receives
   `PossessionContainmentFrontier` by extending `snapshot_filter_containment_walk_includes_inventory`.
3. Existing suite passed via `cargo test -p worldwake-ai`.

### Invariants

1. Every id present in `PlanningSnapshot.entities` carries exactly one `AdmissionSource`
   (enforced by the non-`Option` field type).
2. `PlanningSnapshot` remains non-serialized and transient — no `Serialize`/`Deserialize` derive
   is added, no `SAVE_FORMAT_VERSION` bump occurs.
3. The admission source is derived metadata over belief/world state; it is never written back to
   authoritative state (FND-27).

## Outcome

Completed on 2026-05-20.

- Added planner-snapshot admission provenance as a transient read-model annotation.
- Reused existing snapshot unit-test scenarios to assert self, local same-tick physical,
  grounded-evidence, belief-last-seen, public-topology, and possession/containment-frontier
  admission sources.
- Omitted the drafted hypothetical-effect variant because no live hypothetical-effect id path feeds
  `build_planning_snapshot`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib planning_snapshot`
- Passed `cargo test -p worldwake-ai`
- Passed `scripts/verify.sh`
