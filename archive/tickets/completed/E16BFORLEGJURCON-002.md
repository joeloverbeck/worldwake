# E16BFORLEGJURCON-002: Add force-claim and office-controller relations + WorldTxn helpers

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — relations, world social helpers, WorldTxn mutation helpers
**Deps**: E16BFORLEGJURCON-001, E16 (RelationTables pattern exists)

## Problem

The spec requires two new relation pairs to distinguish explicit force-claim participation from physical office control:
- `contests_office / contested_by` (many:many — claimants to offices)
- `office_controller / offices_controlled` (1:1 — one controller per office)

These relations, plus transactional mutation helpers, are the authoritative data layer that the entire force-control system reads and writes.

## Assumption Reassessment (2026-03-22)

1. `RelationTables` in [crates/worldwake-core/src/relations.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/relations.rs) already stores `office_holder/offices_held`, `member_of/members_of`, `hostile_to/hostility_from`, and `support_declarations`. The four E16b relations (`contests_office`, `contested_by`, `office_controller`, `offices_controlled`) do not exist yet.
2. `WorldTxn` in [crates/worldwake-core/src/world_txn.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs) already exposes the exact mutation patterns this ticket should follow: `declare_support`, `clear_support_declaration`, `assign_office`, `vacate_office`, `add_hostility`, `remove_hostility`, plus shared delta helpers such as `push_presence_relation_delta` and `push_single_target_relation_delta`. The four new force-control helpers do not exist yet.
3. `World` social accessors in [crates/worldwake-core/src/world/social.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/social.rs) already expose the same authoritative-query shape this ticket should extend: `office_holder`, `offices_held_by`, `support_declarations_for_office`, `hostile_targets_of`, `hostile_towards`. The four force-control getters do not exist yet.
4. `RelationKind` / `RelationValue` in [crates/worldwake-core/src/delta.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs) currently do not model the new relation families. This ticket must extend them; `RelationDelta` itself already supports new families once the enum variants exist.
5. The original ticket understated archive/purge impact. In current code, relation-family additions must also update exact archive surfaces: `ArchiveDependencyKind` / `RelationTables::archive_dependencies` / `RelationTables::remove_all` in [crates/worldwake-core/src/relations.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/relations.rs), `ArchivePreparationPolicy`, `ArchiveMutationSnapshot`, and lifecycle cleanup in [crates/worldwake-core/src/world/lifecycle.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/lifecycle.rs), plus archive delta emission in [crates/worldwake-core/src/world_txn.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs).
6. There is no `RelationTables::new()` constructor to update; the live type derives `Default`. Scope should name `Default`/serde roundtrip coverage rather than a nonexistent `new()` function.
7. Current focused coverage already exists in the right layers and should be extended rather than replaced: `world::tests::*social*`, `relations::tests::*remove_all*`, `delta::tests::relation_*`, and `world_txn::tests::*relation*`.
8. No AI/golden/operator reassessment is required for this ticket because it is limited to authoritative relation storage and event-log delta plumbing. The Authoritative-To-AI rule becomes relevant in later tickets that wire these relations into affordances, planning, or start-time validation.
9. No ControlSource path changes, ranking arithmetic, or cumulative arithmetic are in scope here.
10. Mismatch found and corrected: the original `Files to Touch` list omitted [crates/worldwake-core/src/world/lifecycle.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/lifecycle.rs), which is part of the current authoritative archive/purge path for relation families.

## Architecture Check

1. This is still the right architectural move. E16b explicitly separates recognized title (`office_holder`) from physical control (`office_controller`) and explicit challenge participation (`contests_office`). Encoding those as first-class authoritative relations is cleaner than inferring them from location, hostility, or transient system state.
2. `office_controller` should mirror the existing single-target `office_holder` architecture instead of introducing a parallel component field. That preserves a single authoritative source for controller identity and keeps temporal continuity in `OfficeForceState`, exactly as the spec intends.
3. `contests_office` should mirror the existing many:many relation families instead of being encoded as a derived query over proximity or combat posture. That makes contest participation explicit, auditable, archivable, and event-log visible.
4. No aliases or compatibility shims. If later systems need these semantics, they should read the new canonical relations directly.

## Verification Layers

1. Authoritative bidirectional relation integrity -> focused `World`/`RelationTables` tests proving getters read the new forward and reverse tables correctly and `remove_all` tears them down symmetrically.
2. Transaction boundary semantics -> focused `WorldTxn` tests proving add/remove/replace/clear operations emit the correct `RelationDelta` sequence and commit the staged world.
3. Serialization and typed semantic coverage -> focused `delta.rs` and `relations.rs` roundtrip tests proving the new relation families participate in bincode/serde the same way existing families do.
4. Archive preparation and archive teardown correctness -> focused lifecycle / archive tests proving new dependencies block archive when present and are removed with the correct relation deltas when archive proceeds.

## What to Change

### 1. Add relation fields to `RelationTables`

```rust
pub contests_office: BTreeMap<EntityId, BTreeSet<EntityId>>,    // claimant -> offices
pub contested_by: BTreeMap<EntityId, BTreeSet<EntityId>>,       // office -> claimants
pub office_controller: BTreeMap<EntityId, EntityId>,            // office -> controller (1:1)
pub offices_controlled: BTreeMap<EntityId, BTreeSet<EntityId>>, // controller -> offices
```

Also update:
- archive dependency enumeration for offices with controllers / controllers holding offices
- `remove_all` teardown for both new relation families
- relation-table tests and serde roundtrip coverage

### 2. Add World (social.rs) getters

- `force_claimants_for_office(office) -> Vec<EntityId>`
- `offices_contested_by(agent) -> Vec<EntityId>`
- `office_controller(office) -> Option<EntityId>`
- `offices_controlled_by(agent) -> Vec<EntityId>`

### 3. Add WorldTxn mutation helpers

- `add_force_claim(actor, office)` — inserts into both maps, records `RelationDelta`
- `remove_force_claim(actor, office)` — removes from both maps, records `RelationDelta`
- `set_office_controller(office, controller)` — sets 1:1 (clearing previous if any), records `RelationDelta`
- `clear_office_controller(office)` — clears 1:1, records `RelationDelta`

Use the existing transaction helper patterns:
- presence-style relation delta for `contests_office`
- single-target removed-then-added semantics for `office_controller`

### 4. Include in serialization/archive/purge

Ensure the new relation fields participate in:
- `RelationTables` serde / bincode roundtrip behavior
- `ArchiveDependencyKind` and `RelationTables::archive_dependencies`
- `ArchivePreparationPolicy` and lifecycle cleanup/resolution paths
- `ArchiveMutationSnapshot` capture and `WorldTxn` archive delta emission
- `World::archive_entity` / `purge_entity` relation teardown through existing lifecycle helpers

## Files to Touch

- `crates/worldwake-core/src/relations.rs` (modify — add 4 fields, archive dependencies, remove_all, tests)
- `crates/worldwake-core/src/world/social.rs` (modify — add 4 getter methods)
- `crates/worldwake-core/src/world_txn.rs` (modify — add 4 mutation helpers with `RelationDelta`)
- `crates/worldwake-core/src/delta.rs` (modify — add `RelationKind`/`RelationValue` variants and tests)
- `crates/worldwake-core/src/world/lifecycle.rs` (modify — archive snapshot/dependency cleanup support for the new relations)

## Out of Scope

- OfficeForceProfile / OfficeForceState components — that's E16BFORLEGJURCON-001
- Action definitions/handlers — E16BFORLEGJURCON-003/004
- Force control system logic — E16BFORLEGJURCON-005
- AI integration — E16BFORLEGJURCON-007/008
- Institutional belief variants — E16BFORLEGJURCON-006

## Acceptance Criteria

### Tests That Must Pass

1. `add_force_claim(A, office)` → `force_claimants_for_office(office)` returns `[A]` and `offices_contested_by(A)` returns `[office]`
2. `remove_force_claim(A, office)` → both directions return empty
3. `set_office_controller(office, A)` → `office_controller(office)` returns `Some(A)` and `offices_controlled_by(A)` returns `[office]`
4. `set_office_controller(office, B)` after A → A's `offices_controlled` no longer contains office, B's does
5. `clear_office_controller(office)` → returns `None`
6. Each mutation produces appropriate `RelationDelta` entries
7. Archiving an office/controller/claimant with live force-control relations follows the same dependency-and-cleanup rules as existing social relations
8. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `office_controller` is 1:1 — at most one controller per office
2. `contests_office / contested_by` are symmetric — if A contests office, office's contested_by contains A
3. All mutations go through WorldTxn and produce RelationDelta records (no direct mutation)
4. Archive and purge paths cannot silently strand new relation rows
5. No existing tests break

## Tests

### New/Modified Tests

1. `world::tests::force_claim_queries_stay_bidirectional_and_idempotent` — new focused coverage for `add_force_claim`/`remove_force_claim`-equivalent world semantics and public getters.
2. `world::tests::office_controller_queries_replace_prior_controller_and_clear_idempotently` — new focused coverage for the 1:1 controller relation and reverse index updates.
3. `world::tests::social_query_helpers_hide_archived_force_control_entities_even_if_rows_are_stale` — extend or add coverage proving public getters suppress archived claimants/controllers/offices.
4. `relations::tests::remove_all_cleans_force_claim_and_controller_rows` — new low-level teardown coverage for direct relation-table cleanup.
5. `world_txn::tests::force_claim_wrappers_record_add_and_remove_deltas` — new transaction coverage for explicit claim add/remove delta emission.
6. `world_txn::tests::office_controller_wrappers_record_replace_and_clear_deltas` — new transaction coverage for 1:1 controller replacement semantics.
7. `world_txn::tests::archive_entity_records_force_control_relation_teardown` — new archive-delta coverage for the new relation families.
8. `delta::tests::relation_kind_variants_match_semantic_relation_families` and `delta::tests::relation_value_reports_matching_relation_kind` — extend existing enum coverage for the new relation families.
9. `relations::tests::relation_tables_bincode_roundtrip` — extend existing roundtrip coverage so the new relation fields are serialized.

### Rationale

1. Query-surface tests prove the authoritative API shape that later systems will consume.
2. Controller replacement tests enforce the single-authoritative-controller invariant, which is the main architectural reason to add this relation.
3. Archived-entity filtering guards against stale-row leakage through public APIs.
4. Low-level teardown tests prevent relation drift in purge/archive cleanup.
5. Transaction tests prove append-only event-log semantics rather than just in-memory mutation.
6. Replacement/clear delta tests prevent silent controller swaps that would be invisible to the causal record.
7. Archive-delta tests ensure the new family participates in the same authoritative lifecycle as existing relations.
8. Enum tests keep semantic relation typing exhaustive and prevent later omissions.
9. Roundtrip tests catch missed serde fields early.

## Test Plan

### Commands

1. `cargo test -p worldwake-core force_claim_queries_stay_bidirectional_and_idempotent`
2. `cargo test -p worldwake-core office_controller_queries_replace_prior_controller_and_clear_idempotently`
3. `cargo test -p worldwake-core force_claim_wrappers_record_add_and_remove_deltas`
4. `cargo test -p worldwake-core office_controller_wrappers_record_replace_and_clear_deltas`
5. `cargo test -p worldwake-core`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-22
- What actually changed:
  - Added canonical `contests_office / contested_by` and `office_controller / offices_controlled` relation storage in `RelationTables`.
  - Added authoritative `World` getters and low-level mutation helpers for force claims and office controllers.
  - Added `WorldTxn` wrappers and typed `RelationKind` / `RelationValue` coverage for both new relation families.
  - Integrated the new relations into lifecycle/archive preparation, relation cleanup, verification snapshots, and serde/roundtrip coverage.
  - Added focused tests for bidirectional queries, controller replacement semantics, transaction delta emission, archive-preparation blockers, archive rejection while blockers remain, stale-row filtering, and low-level relation cleanup.
- Deviations from original plan:
  - The clean architecture path is to treat outbound force claims and outbound office control exactly like other live authoritative dependents such as `offices_held`: they block archive until `prepare_entity_for_archive` clears them. That is better than silently letting archive tear them down.
  - Because of that, archive verification is primarily through dependency detection and preparation cleanup, not through a normal successful `archive_entity` delta batch for still-live claim/control blockers.
  - `world/lifecycle.rs` was required in scope even though the original ticket omitted it.
- Verification results:
  - `cargo test -p worldwake-core` passed.
  - `cargo clippy --workspace` passed.
  - `cargo test --workspace` passed.
