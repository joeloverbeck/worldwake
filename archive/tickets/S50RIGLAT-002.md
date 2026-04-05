# S50RIGLAT-002: Migrate OfficeData.jurisdiction to BTreeSet + separate office seat locality

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — OfficeData shape change, first live jurisdiction right, SAVE_FORMAT_VERSION bump
**Deps**: S50RIGLAT-001

## Problem

`OfficeData.jurisdiction` is a single `EntityId`, but FOUNDATIONS P23 says "a jurisdiction can stop at the town gate" — implying multi-place jurisdiction is the correct model. Live code also uses that same field as the single political record/event locality for office registers, support ledgers, succession traces, and office-local political actions. Those are different meanings. This ticket separates them by migrating jurisdiction coverage to `BTreeSet<EntityId>` and making the office's canonical seat locality explicit.

## Assumption Reassessment (2026-04-05)

1. `OfficeData.jurisdiction: EntityId` at `crates/worldwake-core/src/offices.rs:9`. Verified this session.
2. `offices_with_jurisdiction(place, world)` at `crates/worldwake-systems/src/offices.rs:121` uses `office_data.jurisdiction == place`. Verified.
3. `office_actions.rs` has locality checks and affordance generation keyed to `office_data.jurisdiction`, not just a single helper. Verified via grep.
4. `world_txn.rs` and `offices.rs` use `office_data.jurisdiction` as the unique `OfficeRegister` / `SupportLedger` place and political trace locality, not just a containment filter. Verified this session.
5. `SAVE_FORMAT_VERSION` is currently 19 at `crates/worldwake-sim/src/save_load.rs:6`. Bump to 20.
6. `OfficeData` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `BTreeSet<EntityId>` satisfies all these bounds.
7. AI/planner code currently also treats office jurisdiction as a single travel/locality target (`planning_snapshot.rs`, `goal_model.rs`, `candidate_generation.rs`, `ranking.rs`). Those callers need the new seat/jurisdiction split, not just a type rename.
8. Scenario/test fixtures create offices with single-place jurisdiction and currently rely on that place as the canonical seat too. They must set both `seat` and `jurisdiction: BTreeSet::from([seat])`.

## Architecture Check

1. Direct field migration per P28 — no shim, no compatibility layer. The old single-place representation is replaced everywhere.
2. `BTreeSet<EntityId>` is deterministic (BTree ordering) per project invariants. No `HashSet`.
3. `jurisdiction` and office seat locality are distinct world facts. After this ticket:
   - `jurisdiction: BTreeSet<EntityId>` means the places where office authority applies
   - `seat: EntityId` means the single canonical locality for office records, political traces, and office-local actions
4. The first live `RightKind::JurisdictionalAuthority` result lands here. It is derived from office holding plus `jurisdiction.contains(entity_place)`, not from the office seat.

## Verification Layers

1. `offices_with_jurisdiction()` returns correct offices for multi-place jurisdiction → focused unit test
2. Rights queries surface `JurisdictionalAuthority` when an office holder acts within jurisdiction coverage → focused unit test
3. Office-local political actions and record updates use office `seat`, not the jurisdiction set → focused unit tests
4. Save/load round-trip preserves `seat` plus multi-place jurisdiction → save/load test
5. All existing golden tests pass unchanged when offices still use `jurisdiction = { seat }`

## What to Change

### 1. Split office seat from jurisdiction coverage

In `crates/worldwake-core/src/offices.rs`:
```rust
// Before
pub jurisdiction: EntityId,
// After
pub seat: EntityId,
pub jurisdiction: BTreeSet<EntityId>,
```

Update all `OfficeData` constructors and fixtures so today's single-place offices use
`seat: place` and `jurisdiction: BTreeSet::from([place])`.

### 2. Update jurisdiction containment queries

In `crates/worldwake-systems/src/offices.rs`:
```rust
// Before
(office_data.jurisdiction == place).then_some(office)
// After
office_data.jurisdiction.contains(&place).then_some(office)
```

Also migrate any other authority/jurisdiction checks, including justice validation and the new
authoritative `JurisdictionalAuthority` branch in `effective_rights()`.

### 3. Move office-local actions, records, and traces onto seat locality

Replace single-place office locality callers so they use the canonical seat instead of the
jurisdiction set:
- `world_txn.rs` office-register/support-ledger updates
- `offices.rs` succession txns, force-control record updates, and politics trace locality
- `office_actions.rs` political-social/force-claim locality gates
- AI/planner surfaces that currently travel to or reason about a single office locality

`seat` is the office-local place. `jurisdiction` remains the area where office authority applies.

### 4. Update world_txn.rs office creation and record updates

In `crates/worldwake-core/src/world_txn.rs`, update all `OfficeData` constructions to set
`seat: place` and `jurisdiction: BTreeSet::from([place])`, and move record lookup/update helpers
to `office_data.seat`.

### 5. Update AI/planner single-place office locality

Any AI/planner helper that currently expects `OfficeData.jurisdiction: EntityId` must move to the
correct side of the split:
- travel/local office-action prerequisites use `seat`
- route/public-order/justice authority checks use `jurisdiction`

### 6. Update scenario/test fixtures

Grep for all `OfficeData { ... jurisdiction: ... }` patterns in tests and scenario setup. Set both
`seat` and `jurisdiction: BTreeSet::from([seat])`, or add extra jurisdiction places where the test
explicitly wants wider coverage.

### 7. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`:
```rust
pub const SAVE_FORMAT_VERSION: u32 = 20;
```

## Files to Touch

- `crates/worldwake-core/src/offices.rs` (modify — field type change)
- `crates/worldwake-core/src/world_txn.rs` (modify — office construction + seat-based record updates)
- `crates/worldwake-core/src/world/ownership.rs` (modify — JurisdictionalAuthority in effective_rights)
- `crates/worldwake-core/src/delta.rs` (modify — if OfficeData appears in delta test fixtures)
- `crates/worldwake-core/src/component_tables.rs` (modify — if OfficeData test fixtures exist)
- `crates/worldwake-systems/src/offices.rs` (modify — jurisdiction queries + seat-based traces/records)
- `crates/worldwake-systems/src/office_actions.rs` (modify — seat vs jurisdiction split)
- `crates/worldwake-systems/src/justice_actions.rs` (modify — jurisdiction containment)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — seat/jurisdiction helpers)
- `crates/worldwake-ai/src/goal_model.rs` (modify — office-local travel prerequisites use seat)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — evidence/travel locality split)
- `crates/worldwake-ai/src/ranking.rs` (modify — jurisdiction route coverage checks)
- `crates/worldwake-sim/src/politics_trace.rs` (modify — trace locality naming/shape if needed)
- `crates/worldwake-sim/src/save_load.rs` (modify — SAVE_FORMAT_VERSION bump)
- Test/scenario files with OfficeData fixtures (modify — wrap in BTreeSet)

## Out of Scope

- Adding belief-facing rights queries (ticket 003)
- Justice candidate generation changes beyond the first live authoritative jurisdiction right (ticket 004)
- Changing `can_exercise_control()` signature
- Multi-place jurisdiction scenario content beyond focused rights/locality proofs

## Acceptance Criteria

### Tests That Must Pass

1. `offices_with_jurisdiction()` returns office when place is in jurisdiction set
2. `offices_with_jurisdiction()` returns empty when place is NOT in jurisdiction set
3. `effective_rights()` surfaces `JurisdictionalAuthority` only when actor holds an office whose jurisdiction contains the target place
4. Office-local political actions still require the actor at the office `seat`, not merely anywhere in the jurisdiction set
5. Save/load round-trip preserves `seat` and `BTreeSet<EntityId>` jurisdiction
6. All existing golden tests: `cargo test -p worldwake-ai`
7. Existing suite: `cargo test --workspace`

### Invariants

1. `OfficeData.jurisdiction` is a `BTreeSet<EntityId>` — deterministic ordering, no HashSet
2. `OfficeData.seat` is the single canonical office-local place for records, traces, and office-local actions
3. Jurisdiction coverage checks use `.contains()`, not `==`
4. `SAVE_FORMAT_VERSION` is 20 after this ticket

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` (test module) — test `offices_with_jurisdiction` with multi-place jurisdiction
2. `crates/worldwake-core/src/world.rs` or `world/ownership.rs` — test `JurisdictionalAuthority` in `effective_rights()` with multi-place office
3. Focused office-system / planner tests covering seat-based locality where the jurisdiction set is larger than the seat

### Commands

1. `cargo test -p worldwake-core -- jurisdiction`
2. `cargo test -p worldwake-systems -- jurisdiction`
3. `cargo test -p worldwake-ai -- office`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

## Outcome

**Completed**: 2026-04-05

This ticket landed the corrected split between office authority coverage and office-local record/action locality.

- `OfficeData` now stores `seat: EntityId` plus `jurisdiction: BTreeSet<EntityId>` in [crates/worldwake-core/src/offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/offices.rs)
- authoritative office-register/support-ledger updates now key off `seat` in [crates/worldwake-core/src/world_txn.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world_txn.rs)
- the first live `RightKind::JurisdictionalAuthority` result now lands through `effective_rights()` in [crates/worldwake-core/src/world/ownership.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/ownership.rs), while `can_exercise_control()` remains on the narrower control path
- office-system and justice authority checks now use jurisdiction containment, while office-local political actions and traces use `seat` in [crates/worldwake-systems/src/offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs), [crates/worldwake-systems/src/office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs), and [crates/worldwake-systems/src/justice_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs)
- planner/candidate/ranking surfaces that previously treated jurisdiction as one place now use `seat` for office-local travel semantics and the jurisdiction set for area coverage in [crates/worldwake-ai/src/planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs), [crates/worldwake-ai/src/goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), and [crates/worldwake-ai/src/ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
- political trace locality now records `seat` in [crates/worldwake-sim/src/politics_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs)
- `SAVE_FORMAT_VERSION` bumped to `20` in [crates/worldwake-sim/src/save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs)

Focused proof added:
- multi-place `offices_with_jurisdiction()` membership
- `JurisdictionalAuthority` in `effective_rights()`
- office-local political actions still requiring `seat` even when jurisdiction is wider

Verification:
- `cargo test -p worldwake-core -- effective_rights`
- `cargo test -p worldwake-core has_right_consistency`
- `cargo test -p worldwake-systems offices_with_jurisdiction_matches_any_place_in_jurisdiction_set`
- `cargo test -p worldwake-systems declare_support_requires_office_seat_even_with_wider_jurisdiction`
- `cargo test -p worldwake-sim`
- `cargo test -p worldwake-ai -- office`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
