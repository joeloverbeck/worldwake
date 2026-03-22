# E16BFORLEGJURCON-002: Add force-claim and office-controller relations + WorldTxn helpers

**Status**: PENDING
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

1. `RelationTables` in `relations.rs` already stores `office_holder/offices_held`, `member_of/members_of`, `hostile_to/hostility_from`, `supports_for_office/support_declarations_for`. The four new fields (`contests_office`, `contested_by`, `office_controller`, `offices_controlled`) do not exist.
2. WorldTxn helpers follow patterns like `declare_support()`, `add_hostility()` in `world_txn.rs`. The four new helpers (`add_force_claim`, `remove_force_claim`, `set_office_controller`, `clear_office_controller`) do not exist.
3. `World`'s `social.rs` module has getters like `supporters_of()`, `office_holder()`. The four new getters (`force_claimants_for_office`, `offices_contested_by`, `office_controller`, `offices_controlled_by`) do not exist.
4. N/A — not an AI regression ticket.
5. N/A — no ordering dependency.
6. N/A — no heuristic removal.
7. N/A — not a start-failure ticket.
8. N/A — not a political closure ticket yet.
9. N/A — no ControlSource manipulation.
10. N/A — no golden scenario.
11. No mismatches found.
12. N/A — no cumulative arithmetic.

## Architecture Check

1. Follows the established `RelationTables` + `WorldTxn` + `World` getter pattern exactly. `office_controller` is 1:1 (like `office_holder`), `contests_office` is many:many (like `hostile_to`). This matches the spec's single authoritative source for physical control.
2. No backward-compatibility shims. Net-new relation fields and helpers.

## Verification Layers

1. `add_force_claim` creates `contests_office` entry → focused unit test on WorldTxn + World getter
2. `remove_force_claim` removes entry → focused unit test
3. `set_office_controller` sets 1:1 relation → focused unit test verifying old controller is replaced
4. `clear_office_controller` clears relation → focused unit test
5. `RelationDelta` recorded for each mutation → focused unit test on delta output
6. Single-layer ticket (data + transaction helpers). Verification is focused/unit.

## What to Change

### 1. Add relation fields to `RelationTables`

```rust
pub contests_office: BTreeMap<EntityId, BTreeSet<EntityId>>,   // claimant -> offices
pub contested_by: BTreeMap<EntityId, BTreeSet<EntityId>>,      // office -> claimants
pub office_controller: BTreeMap<EntityId, EntityId>,            // office -> controller (1:1)
pub offices_controlled: BTreeMap<EntityId, BTreeSet<EntityId>>, // controller -> offices
```

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

### 4. Include in serialization/archive/purge

Ensure the new relation fields participate in `RelationTables` serialization, `ArchiveDependency` checks, and any purge/cleanup paths.

## Files to Touch

- `crates/worldwake-core/src/relations.rs` (modify — add 4 fields, initialize in `new()`)
- `crates/worldwake-core/src/world/social.rs` (modify — add 4 getter methods)
- `crates/worldwake-core/src/world_txn.rs` (modify — add 4 mutation helpers with `RelationDelta`)
- `crates/worldwake-core/src/delta.rs` (modify — add `RelationDelta` variants if needed)

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
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `office_controller` is 1:1 — at most one controller per office
2. `contests_office / contested_by` are symmetric — if A contests office, office's contested_by contains A
3. All mutations go through WorldTxn and produce RelationDelta records (no direct mutation)
4. No existing tests break

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/relations.rs` or `world_txn.rs` test module — focused tests for all 4 helpers with delta verification

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
