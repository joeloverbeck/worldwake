# E14PERBEL-008: Unify Authoritative Component Manifest and Remove Duplicated Schema Inventories

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — component schema macro plumbing and schema-invariant tests
**Deps**: E14PERBEL-003 (completed component registration now exposes the duplication hazard)

## Problem

The authoritative component manifest currently has two brittle duplication points:

1. `crates/worldwake-core/src/component_schema.rs` repeats component entries across separate macro arms, including the `select_txn_simple_set_components` expansion path used by `WorldTxn`.
2. Some downstream tests hard-code the full ordered component inventory instead of deriving from the canonical schema.

This is workable, but it is not the ideal architecture. Adding a new authoritative component currently risks partial registration, stale txn setter generation, or stale downstream schema assertions. That is exactly the sort of duplication drift the project should eliminate rather than normalize.

## Assumption Reassessment (2026-03-14)

1. `with_component_schema_entries!` has multiple expansion arms in `crates/worldwake-core/src/component_schema.rs`, and the txn-simple-set path currently requires keeping a second entry list in sync manually — confirmed.
2. `ComponentKind` / `ComponentValue`, `ComponentTables`, `World`, and `WorldTxn` all derive from the authoritative component manifest, but not all derivations currently flow from a single literal source — confirmed.
3. Exactly one active downstream workspace test currently hard-codes the full authoritative `ComponentKind::ALL` inventory: `crates/worldwake-systems/tests/e09_needs_integration.rs`. The same full-order inventory is also restated inside `crates/worldwake-core/src/delta.rs` for the core schema contract — confirmed.
4. None of the remaining active E14 tickets (`E14PERBEL-004` through `E14PERBEL-007`) explicitly own this cleanup. They build on the schema but do not specify manifest deduplication or schema-assertion centralization — confirmed.

## Architecture Check

1. The clean design is a single authoritative component manifest that every derived surface consumes: tables, world API, txn API, deltas, and schema assertions. That removes a whole class of partial-update bugs.
2. This ticket should remove duplication rather than paper over it with more comments or checklist discipline. No alias paths, no shadow manifests, no “remember to update both places” rules.
3. The current architecture already has a good canonical surface for downstream consumers: `ComponentKind::ALL`. The robust direction is to make core own the full declaration-order contract and let downstream tests assert only the subset and ordering guarantees they actually integrate against.
4. This ticket should not introduce a second exported registry abstraction just to avoid touching macro plumbing. The ideal cleanup is still one manifest plus derived projections, not more public schema layers.

## What to Change

### 1. Collapse component-schema duplication to one literal manifest

Refactor `crates/worldwake-core/src/component_schema.rs` so the txn-simple-set selection path is derived from the same authoritative component entry list as the rest of the schema expansion.

Acceptable outcomes include:

- keeping one literal component entry list and deriving both `forward_authoritative_components` and `select_txn_simple_set_components` from it, or
- another equivalent refactor that leaves exactly one literal authoritative manifest to maintain.

The result must preserve current generated behavior for:

- `ComponentTables`
- `World`
- `WorldTxn`
- `ComponentKind`
- `ComponentValue`

### 2. Replace duplicated downstream schema inventories with canonical assertions

Audit active non-archived tests that restate the full authoritative component list. Replace cross-crate expectations with assertions that derive from canonical core data or from targeted subset/order helpers, while still preserving useful invariants such as:

- specific required components exist
- ordering remains stable where ordering is part of the contract for that test
- shared E09/E12/E14 surfaces still expose the components those tests care about

Keep the full declaration-order contract asserted in `worldwake-core`, where the manifest is owned. Do not weaken coverage into “non-empty” or “contains a few values,” but do remove duplicate full inventories from downstream integration tests.

### 3. Add focused regression coverage for schema derivation integrity

Add or strengthen tests in `worldwake-core` that prove:

- txn-simple-set components are derived correctly from the canonical manifest
- newly added simple components automatically appear in the generated txn setter surface
- the authoritative component inventory remains stable and deterministic

## Files to Touch

- `crates/worldwake-core/src/component_schema.rs` (modify — unify manifest source)
- `crates/worldwake-core/src/world_txn.rs` (modify only if tests/helpers need stronger manifest assertions)
- `crates/worldwake-core/src/delta.rs` (modify — keep the canonical full-order contract in core, potentially via a shared helper)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify — remove duplicated full schema inventory while preserving E09/E12/E14 integration coverage)
- Any other non-archived active test file that still hard-codes the full component inventory (modify if found during implementation)

## Out of Scope

- Adding new authoritative components
- Changing component semantics, entity-kind guards, or hashing policy
- Changing the `BeliefView` / perception architecture
- Reworking unrelated macro systems outside authoritative component registration
- Editing archived tickets or specs unless a later implementation finds a factual mismatch

## Acceptance Criteria

### Tests That Must Pass

1. There is only one literal authoritative component manifest to maintain in `component_schema.rs`
2. `WorldTxn` simple setter generation is derived from that same canonical manifest
3. Core retains the authoritative full declaration-order assertion, and no downstream active test file maintains its own stale-prone full copy of the authoritative component list
4. Existing generated APIs for `ComponentTables`, `World`, `WorldTxn`, `ComponentKind`, and `ComponentValue` still compile and behave the same
5. `cargo test -p worldwake-core`
6. `cargo clippy --workspace`
7. `cargo test --workspace`

### Invariants

1. The authoritative component inventory remains deterministic and declaration-ordered
2. Adding a future txn-simple-set component requires changing the manifest in only one place
3. Schema tests still detect missing or reordered components, but they do so from canonical data rather than duplicated hand-maintained lists

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/component_schema.rs` or the nearest existing schema-focused core test location — add regression coverage that txn-simple-set derivation stays aligned with the canonical manifest
   Rationale: catches the exact drift this ticket is meant to eliminate.
2. `crates/worldwake-core/src/delta.rs` and/or other existing core manifest tests — keep the full declaration-order contract in core while deriving any secondary projections from canonical schema data where possible
   Rationale: preserves strong ordering/inventory checks at the ownership boundary without duplicating the manifest across crates.
3. `crates/worldwake-systems/tests/e09_needs_integration.rs` — replace the duplicated full inventory copy with focused canonical subset/order assertions
   Rationale: prevents downstream breakage every time the authoritative schema grows while preserving the integration guarantees the test actually owns.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What changed:
  - Collapsed `with_component_schema_entries!` to a single literal authoritative manifest in `crates/worldwake-core/src/component_schema.rs`.
  - Restored txn-simple-set derivation for `FacilityQueueDispositionProfile`, `ExclusiveFacilityPolicy`, and `FacilityUseQueue`, then removed the handwritten `WorldTxn` setter workaround those omissions had forced.
  - Added core regression coverage proving the txn-simple-set projection matches the canonical component inventory and that every selected setter/clearer method is generated.
  - Reworked `crates/worldwake-systems/tests/e09_needs_integration.rs` to assert the E09/E12/E14 subset and ordering it actually depends on instead of restating the full global component inventory.
- Deviations from original plan:
  - The reassessment found the downstream duplication was narrower than stated: one active cross-crate full-inventory assertion, not multiple.
  - The implementation also exposed existing selector drift for facility-queue-related components. Fixing that drift and removing the compensating handwritten setters was necessary to reach the intended architecture.
- Verification results:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-systems --test e09_needs_integration`
  - `cargo clippy --workspace`
  - `cargo test --workspace`
