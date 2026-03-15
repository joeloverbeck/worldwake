# E16OFFSUCFAC-001: Add OfficeData, FactionData Components and Supporting Enums

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new components and enums in worldwake-core
**Deps**: None (builds on existing `EntityKind::Office` and `EntityKind::Faction`)

## Problem

E16 introduces offices and factions as first-class entities with data components. The `EntityKind::Office` and `EntityKind::Faction` variants already exist, but there are no corresponding data components to hold office metadata (title, jurisdiction, succession law, eligibility, vacancy tracking) or faction metadata (name, purpose). These must exist before any other E16 ticket can proceed.

## Assumption Reassessment (2026-03-15)

1. `EntityKind::Office` and `EntityKind::Faction` already exist in `crates/worldwake-core/src/entity.rs` — confirmed, no need to add them.
2. `Permille` newtype exists in `crates/worldwake-core/src/numerics.rs` — confirmed, used for typed values.
3. `Tick` type exists in `crates/worldwake-core/src/ids.rs` — confirmed, needed for `vacancy_since`.
4. `EntityId` exists in `crates/worldwake-core/src/ids.rs` — confirmed, needed for jurisdiction and faction membership references.
5. No `OfficeData` or `FactionData` structs exist yet — confirmed by codebase search.
6. No `SuccessionLaw`, `EligibilityRule`, or `FactionPurpose` enums exist yet — confirmed.

## Architecture Check

1. Placing new components in `worldwake-core` follows the established pattern: all authoritative component types live in core.
2. Structs and enums are simple data containers with `Serialize`/`Deserialize` derives, matching existing component patterns (e.g., `CombatProfile`, `AgentData`).
3. No backward-compatibility shims needed — these are entirely new types.

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

### 3. Wire into `crates/worldwake-core/src/lib.rs`

Add `pub mod offices;` and `pub mod factions;` declarations and re-export the public types.

## Files to Touch

- `crates/worldwake-core/src/offices.rs` (new)
- `crates/worldwake-core/src/factions.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add module declarations and re-exports)

## Out of Scope

- Component table storage registration (E16OFFSUCFAC-002)
- Component schema entries (E16OFFSUCFAC-002)
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
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo test --workspace`

### Invariants

1. No existing types or modules are modified — these are purely additive.
2. All new types derive `Serialize, Deserialize` for save/load compatibility.
3. No `f32`/`f64` — all numeric values use `u64` or `Permille`.
4. All collections use deterministic ordering (Vec for eligibility_rules is ordered by insertion).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/offices.rs` — serde roundtrip tests for `OfficeData`, `SuccessionLaw`, `EligibilityRule`.
2. `crates/worldwake-core/src/factions.rs` — serde roundtrip tests for `FactionData`, `FactionPurpose`.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
