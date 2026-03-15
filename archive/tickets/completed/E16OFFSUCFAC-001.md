# E16OFFSUCFAC-001: Add OfficeData, FactionData Component Types and Supporting Enums

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new authoritative component types and enums in worldwake-core
**Deps**: None (builds on existing `EntityKind::Office` and `EntityKind::Faction`)

## Problem

E16 introduces offices and factions as first-class entities with authoritative metadata. The `EntityKind::Office` and `EntityKind::Faction` variants already exist, but there are no corresponding core data types to hold office metadata (title, jurisdiction, succession law, eligibility, vacancy tracking) or faction metadata (name, purpose). These types must exist before the follow-on ticket can register them in typed ECS storage.

## Assumption Reassessment (2026-03-15)

1. `EntityKind::Office` and `EntityKind::Faction` already exist in `crates/worldwake-core/src/entity.rs` — confirmed, no need to add them.
2. `Tick` type exists in `crates/worldwake-core/src/ids.rs` — confirmed, needed for `vacancy_since`.
3. `EntityId` exists in `crates/worldwake-core/src/ids.rs` — confirmed, needed for jurisdiction and faction membership references.
4. No `OfficeData` or `FactionData` structs exist yet — confirmed by codebase search.
5. No `SuccessionLaw`, `EligibilityRule`, or `FactionPurpose` enums exist yet — confirmed.
6. In this codebase, authoritative components conventionally implement the `Component` marker trait even before schema registration — confirmed from existing core component modules.
7. Full component usability requires `component_schema.rs` registration, which is currently handled by `E16OFFSUCFAC-002`; adding the types alone does not make them attachable to entities yet.

## Architecture Check

1. Placing new authoritative component types in `worldwake-core` follows the established pattern: all authoritative component types live in core.
2. The new structs should implement `Component` now so they match the existing ECS contract even before storage registration.
3. Storage/schema registration remains a separate concern in this ticket set, but this ticket must be explicit that it only establishes the types and trait bounds.
4. No backward-compatibility shims needed — these are entirely new types.

## What to Change

### 1. Create `crates/worldwake-core/src/offices.rs`

Add new module with:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OfficeData {
    pub title: String,
    pub jurisdiction: EntityId,
    pub succession_law: SuccessionLaw,
    pub eligibility_rules: Vec<EligibilityRule>,
    pub succession_period_ticks: u64,
    pub vacancy_since: Option<Tick>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SuccessionLaw {
    Support,
    Force,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EligibilityRule {
    FactionMember(EntityId),
}
```

Also implement `Component` for `OfficeData`.

### 2. Create `crates/worldwake-core/src/factions.rs`

Add new module with:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FactionData {
    pub name: String,
    pub purpose: FactionPurpose,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FactionPurpose {
    Political,
    Military,
    Trade,
    Religious,
}
```

Also implement `Component` for `FactionData`.

### 3. Wire into `crates/worldwake-core/src/lib.rs`

Add `pub mod offices;` and `pub mod factions;` declarations and re-export the public types.

## Files to Touch

- `crates/worldwake-core/src/offices.rs` (new)
- `crates/worldwake-core/src/factions.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module declarations and re-exports)

## Out of Scope

- Component table storage registration in `component_schema.rs` / generated ECS surfaces (E16OFFSUCFAC-002)
- Relation storage for `support_declarations` (E16OFFSUCFAC-003)
- Any action definitions or handlers
- Any AI integration
- Any system functions
- The `courage` field on `UtilityProfile` (E16OFFSUCFAC-004)

## Acceptance Criteria

### Tests That Must Pass

1. `OfficeData` constructs with all fields and roundtrips through bincode serialization.
2. `FactionData` constructs with all fields and roundtrips through bincode serialization.
3. `SuccessionLaw::Support` and `SuccessionLaw::Force` variants are distinct and serializable.
4. `EligibilityRule::FactionMember(entity_id)` constructs and roundtrips.
5. `FactionPurpose` all four variants construct and roundtrip.
6. `OfficeData` and `FactionData` satisfy the `Component` trait bounds used by authoritative ECS data.
7. `cargo test -p worldwake-core`
8. `cargo clippy --workspace --all-targets -- -D warnings`
9. `cargo test --workspace`

### Invariants

1. Existing ECS registration surfaces are not changed in this ticket; they remain the responsibility of `E16OFFSUCFAC-002`.
2. All new types derive `Serialize, Deserialize` for save/load compatibility.
3. The authoritative component structs implement `Component`.
4. No `f32`/`f64` are introduced.
5. All collections use deterministic ordering (`Vec` for `eligibility_rules` is ordered by insertion).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/offices.rs` — component-bound and serde roundtrip tests for `OfficeData`, plus enum coverage for `SuccessionLaw` and `EligibilityRule`.
2. `crates/worldwake-core/src/factions.rs` — component-bound and serde roundtrip tests for `FactionData` and `FactionPurpose`.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completed: 2026-03-15
- What actually changed:
  - Added new core modules `offices.rs` and `factions.rs`.
  - Implemented `OfficeData`, `SuccessionLaw`, `EligibilityRule`, `FactionData`, and `FactionPurpose`.
  - Implemented `Component` for `OfficeData` and `FactionData`.
  - Exported the new modules and public types from `worldwake-core::lib`.
  - Added focused component-bound and bincode roundtrip tests for the new types.
- Deviations from original plan:
  - Updated this ticket before implementation to reflect current architecture more accurately.
  - Kept ECS/schema registration out of scope for this ticket and explicitly deferred it to `E16OFFSUCFAC-002`; this ticket now establishes authoritative component types, not attachable registered components.
- Verification results:
  - `cargo test -p worldwake-core` ✅
  - `cargo clippy --workspace --all-targets -- -D warnings` ✅
  - `cargo test --workspace` ✅
