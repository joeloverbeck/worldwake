# E15RUMWITDIS-004: Add MismatchKind Enum and Discovery Event Evidence

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new evidence type in core
**Deps**: None

## Problem

Belief mismatch detection (deliverable 3 of E15) still lacks a typed payload describing what changed between a prior belief and a new observation. `EventTag::Discovery` and the surrounding social-information scaffolding already exist, but discovery events cannot yet carry structured mismatch evidence. This ticket establishes that core evidence model so the later perception tickets can emit discovery records without inventing ad hoc payloads.

## Assumption Reassessment (2026-03-14)

1. `EventTag::Discovery` already exists in `crates/worldwake-core/src/event_tag.rs`; this ticket is not responsible for adding the tag.
2. Event evidence is still modeled via `EvidenceRef` in `crates/worldwake-core/src/event_record.rs`, and it still only has `Wound { entity, wound_id }`.
3. `PendingEvent` and `EventRecord` already carry `Vec<EvidenceRef>`, so no new attachment mechanism is required.
4. `TellProfile`, `ActionDomain::Social`, and `SocialObservationKind::WitnessedTelling` already exist; they are not part of this ticket.
5. `CommodityKind`, `Quantity`, and `EntityId` are already available from core and remain the correct concrete types for mismatch payloads.

## Architecture Check

1. Add `MismatchKind` alongside belief types in `crates/worldwake-core/src/belief.rs`. It describes violated expectations between a prior belief and a new observation, so it belongs closer to belief state than to event-log plumbing.
2. Extend `EvidenceRef` with `Mismatch { observer, subject, kind }`. This keeps discovery evidence in the existing append-only evidence channel instead of creating a special-case event field.
3. Update the small number of exhaustive `EvidenceRef` matches outside core so the new variant participates in entity extraction cleanly. This is a direct migration, not a compatibility shim.
4. Alternative: a separate `discovery_evidence` field on `EventRecord`. Rejected because it duplicates the existing evidence mechanism and makes future evidence kinds less uniform.
5. No backwards-compatibility shims, aliases, or wrapper types.

## What to Change

### 1. Define `MismatchKind` enum

In `crates/worldwake-core/src/belief.rs` (or a closely related module), add:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum MismatchKind {
    EntityMissing,
    AliveStatusChanged,
    InventoryDiscrepancy {
        commodity: CommodityKind,
        believed: Quantity,
        observed: Quantity,
    },
    PlaceChanged {
        believed_place: EntityId,
        observed_place: EntityId,
    },
}
```

### 2. Add `EvidenceRef::Mismatch` variant

In `crates/worldwake-core/src/event_record.rs`, extend the `EvidenceRef` enum:

```rust
Mismatch {
    observer: EntityId,
    subject: EntityId,
    kind: MismatchKind,
},
```

### 3. Export MismatchKind

In `crates/worldwake-core/src/lib.rs`, add `MismatchKind` to public exports.

### 4. Update exhaustive `EvidenceRef` consumers

Any code that pattern-matches `EvidenceRef` exhaustively must be updated for the new variant. Today that includes `crates/worldwake-systems/src/perception.rs::observed_entities()`. Keep the change minimal and structural: include the mismatch-linked entities in the extracted entity set so downstream event readers continue to treat evidence refs as entity-bearing metadata.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add MismatchKind enum)
- `crates/worldwake-core/src/event_record.rs` (modify — add EvidenceRef::Mismatch variant)
- `crates/worldwake-core/src/lib.rs` (modify — export MismatchKind)
- `crates/worldwake-systems/src/perception.rs` (modify — update exhaustive `EvidenceRef` match for the new variant)

## Out of Scope

- Mismatch detection logic in perception system
- Discovery event emission logic
- Tell action or TellProfile
- Any AI/planner changes
- Any new event-log attachment mechanism beyond `EvidenceRef`
- EntityMissing detection logic

## Acceptance Criteria

### Tests That Must Pass

1. `MismatchKind::EntityMissing` constructs correctly
2. `MismatchKind::AliveStatusChanged` constructs correctly
3. `MismatchKind::InventoryDiscrepancy` holds commodity, believed, observed
4. `MismatchKind::PlaceChanged` holds believed_place, observed_place
5. `EvidenceRef::Mismatch` constructs with observer, subject, kind
6. MismatchKind serializes and deserializes correctly
7. EvidenceRef::Mismatch serializes and deserializes correctly
8. Existing `EvidenceRef::Wound` behavior remains unchanged
9. Exhaustive `EvidenceRef` consumers compile and preserve deterministic ordering semantics
10. Existing suite: `cargo test --workspace`
11. `cargo clippy --workspace`

### Invariants

1. Existing `EvidenceRef::Wound` variant remains unchanged
2. `MismatchKind` uses `Quantity`, `CommodityKind`, and `EntityId` from core; no new numeric abstraction or alias is introduced
3. The change does not introduce a second evidence transport path
4. No legacy aliasing or compatibility wrappers are added

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — unit tests for `MismatchKind` construction, ordering, and serialization roundtrip
2. `crates/worldwake-core/src/event_record.rs` — unit tests for `EvidenceRef::Mismatch` construction, ordering, deduplication, and serialization roundtrip

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`

## Outcome

- Completed: 2026-03-14
- Actual changes:
  - Added `MismatchKind` to `worldwake-core::belief` with deterministic ordering/hash derives suitable for event evidence sorting and deduplication.
  - Added `EvidenceRef::Mismatch { observer, subject, kind }` and exported `MismatchKind` from `worldwake-core`.
  - Updated `worldwake-systems::perception::observed_entities()` for the new exhaustive `EvidenceRef` shape.
  - Added core tests for mismatch construction, ordering, deduplication, and bincode roundtrips.
- Deviations from original plan:
  - `EventTag::Discovery`, `ActionDomain::Social`, `TellProfile`, and `WitnessedTelling` had already landed, so this ticket was narrowed to typed mismatch evidence rather than broader E15 scaffolding.
  - No discovery emission logic was implemented here; that remains with the later perception tickets.
- Verification:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-systems perception`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
