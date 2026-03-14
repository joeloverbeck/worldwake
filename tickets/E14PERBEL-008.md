# E14PERBEL-008: Unify Authoritative Component Manifest and Remove Duplicated Schema Inventories

**Status**: PENDING
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
3. At least one downstream workspace test (`crates/worldwake-systems/tests/e09_needs_integration.rs`) hard-codes the full `ComponentKind::ALL` inventory and had to be updated when E14 belief components were added — confirmed.
4. None of the remaining active E14 tickets (`E14PERBEL-004` through `E14PERBEL-007`) explicitly own this cleanup. They build on the schema but do not specify manifest deduplication or schema-assertion centralization — confirmed.

## Architecture Check

1. The clean design is a single authoritative component manifest that every derived surface consumes: tables, world API, txn API, deltas, and schema assertions. That removes a whole class of partial-update bugs.
2. This ticket should remove duplication rather than paper over it with more comments or checklist discipline. No alias paths, no shadow manifests, no “remember to update both places” rules.
3. Schema-invariant tests should verify the authoritative manifest without retyping the whole ordered list in multiple crates. Tests should fail when the schema is wrong, not because a second hand-maintained copy drifted.

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

Audit active non-archived tests that restate the full authoritative component list. Replace those expectations with assertions that derive from the canonical manifest or a centralized helper, while still preserving useful invariants such as:

- specific required components exist
- ordering remains stable where ordering is part of the contract
- shared E09/E12/E14 surfaces still expose the components those tests care about

Do not weaken test coverage into “non-empty” or “contains a few values.” Keep the checks strong, but remove duplicate inventories.

### 3. Add focused regression coverage for schema derivation integrity

Add or strengthen tests in `worldwake-core` that prove:

- txn-simple-set components are derived correctly from the canonical manifest
- newly added simple components automatically appear in the generated txn setter surface
- the authoritative component inventory remains stable and deterministic

## Files to Touch

- `crates/worldwake-core/src/component_schema.rs` (modify — unify manifest source)
- `crates/worldwake-core/src/world_txn.rs` (modify only if tests/helpers need stronger manifest assertions)
- `crates/worldwake-core/src/delta.rs` (modify only if tests are reworked to use a shared helper)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify — remove duplicated full schema inventory if still present)
- Any other non-archived test file that hard-codes the full component inventory (modify — centralize/derive)

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
3. No active test file maintains its own stale-prone full copy of the authoritative component list when a canonical assertion can be used instead
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
2. `crates/worldwake-core/src/delta.rs` and/or other existing manifest tests — update assertions to rely on canonical schema data where possible
   Rationale: preserves strong ordering/inventory checks without duplicating the manifest.
3. `crates/worldwake-systems/tests/e09_needs_integration.rs` and any similar active tests — replace duplicated inventory copies with focused canonical assertions
   Rationale: prevents downstream breakage every time the authoritative schema grows.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
