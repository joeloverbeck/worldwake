# E17CRITHEJUS-003: Extend institutional claims with Accusation, Verdict, CrimeRegister

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — enum extensions in core institutional types
**Deps**: E17CRITHEJUS-001 (needs `PunishmentKind`)

## Problem

No institutional record type exists for crimes. The accusation and punishment system requires `RecordKind::CrimeRegister`, `InstitutionalClaim::Accusation`, and `InstitutionalClaim::Verdict` following the same append-only record pattern as `OfficeRegister`, `FactionRoster`, and `SupportLedger`.

## Assumption Reassessment (2026-03-25)

1. `RecordKind` enum in `crates/worldwake-core/src/institutional.rs` currently has `OfficeRegister`, `FactionRoster`, `SupportLedger`. `InstitutionalClaim` has `OfficeHolder`, `FactionMembership`, `SupportDeclaration`, `ForceControl`.
2. `RecordData` with `InstitutionalRecordEntry` and `RecordEntryId` types exist and support append/supersede. New claim types reuse this infrastructure.
3. `ViolationId` exists in `crates/worldwake-core/src/violation.rs` (added by S27).
4. `PunishmentKind` will be added by E17CRITHEJUS-001.
5. N/A.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatches found.
12. N/A.

## Architecture Check

1. Extending `RecordKind` and `InstitutionalClaim` follows the exact pattern used for E16c's record architecture. CrimeRegister reuses the same `RecordData` entity structure — no new entity kind or storage mechanism needed.
2. No backwards-compatibility aliasing introduced.

## Verification Layers

1. `InstitutionalClaim::Accusation` serde round-trip -> focused unit test
2. `InstitutionalClaim::Verdict` serde round-trip with both `PunishmentKind` variants -> focused unit test
3. `RecordData` append + supersede with new claim types -> focused unit test
4. Single-crate ticket; no cross-layer mapping needed.

## What to Change

### 1. Add `CrimeRegister` to `RecordKind` in `institutional.rs`

New variant `CrimeRegister`. Update exhaustive matches.

### 2. Add `Accusation` and `Verdict` to `InstitutionalClaim` in `institutional.rs`

```rust
Accusation {
    accuser: EntityId,
    accused: EntityId,
    violation_id: ViolationId,
    effective_tick: Tick,
},
Verdict {
    accused: EntityId,
    punishment: PunishmentKind,
    effective_tick: Tick,
    supersedes_accusation: RecordEntryId,
},
```

Update all exhaustive matches on `InstitutionalClaim` across the workspace.

### 3. Update match arms in downstream crates

Add placeholder arms for the new `RecordKind` and `InstitutionalClaim` variants in any exhaustive matches (E16c consultation, belief processing, etc.).

## Files to Touch

- `crates/worldwake-core/src/institutional.rs` (modify)
- Any file with exhaustive match on `RecordKind` or `InstitutionalClaim` (modify — add match arms)

## Out of Scope

- Creating `CrimeRegister` entities in world setup / test_utils (done in action tickets)
- Accuse action handler (E17CRITHEJUS-008)
- Fine/Exile action handlers (E17CRITHEJUS-009)
- AI consultation of CrimeRegister (E17CRITHEJUS-011)

## Acceptance Criteria

### Tests That Must Pass

1. `InstitutionalClaim::Accusation` serde round-trip preserves all fields
2. `InstitutionalClaim::Verdict` with `PunishmentKind::Fine` serde round-trip
3. `InstitutionalClaim::Verdict` with `PunishmentKind::Exile` serde round-trip
4. `RecordData` of kind `CrimeRegister` can be constructed and entries appended
5. Supersede chain works: `Verdict` supersedes `Accusation` entry
6. Existing suite: `cargo test -p worldwake-core`
7. Existing suite: `cargo build --workspace` (all match arms compile)

### Invariants

1. `InstitutionalClaim` remains `Clone + Debug + Eq + PartialEq + Serialize + Deserialize`
2. `RecordKind` remains `Copy + Clone + Debug + Eq + PartialEq + Ord + PartialOrd + Serialize + Deserialize`
3. Append-only record semantics preserved (no mutation of existing entries)
4. Existing record types (`OfficeRegister`, etc.) completely unaffected

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/institutional.rs` — serde round-trip, append, and supersede tests for `Accusation` and `Verdict`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo build --workspace`
3. `cargo clippy --workspace`
