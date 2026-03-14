# E15RUMWITDIS-004: Add MismatchKind Enum and Discovery Event Evidence

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new evidence type in core
**Deps**: E15RUMWITDIS-002 (requires EventTag::Discovery)

## Problem

Belief mismatch detection (deliverable 3 of E15) needs a `MismatchKind` enum to describe what changed between a prior belief and a new observation. This enum is carried as evidence on Discovery events emitted into the event log. The evidence infrastructure must exist before the mismatch detection logic can be implemented.

## Assumption Reassessment (2026-03-14)

1. Event evidence is currently modeled via `EvidenceRef` enum in `crates/worldwake-core/src/event_record.rs` — currently has one variant: `Wound { entity, wound_id }`. MismatchKind needs either a new `EvidenceRef` variant or a separate evidence attachment mechanism.
2. `CommodityKind` and `Quantity` types exist in `crates/worldwake-core/src/items.rs` — needed for `InventoryDiscrepancy`.
3. `EntityId` is in `crates/worldwake-core/src/ids.rs` — needed for `PlaceChanged`.
4. `PendingEvent` in `crates/worldwake-core/src/event_record.rs` accepts a `Vec<EvidenceRef>` as the evidence field.

## Architecture Check

1. Adding a new `EvidenceRef::Mismatch` variant is the cleanest approach — it follows the established evidence pattern and requires no new infrastructure.
2. Alternative: a separate `discovery_evidence` field on EventRecord. Rejected — adds structural complexity for no gain; EvidenceRef is already the extensible evidence mechanism.
3. No backwards-compatibility shims.

## What to Change

### 1. Define `MismatchKind` enum

In `crates/worldwake-core/src/belief.rs` (or a closely related module), add:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — add MismatchKind enum)
- `crates/worldwake-core/src/event_record.rs` (modify — add EvidenceRef::Mismatch variant)
- `crates/worldwake-core/src/lib.rs` (modify — export MismatchKind)

## Out of Scope

- Mismatch detection logic in perception system
- Discovery event emission logic
- Tell action or TellProfile
- Any AI/planner changes
- Modifying existing EvidenceRef variants
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
8. Existing suite: `cargo test --workspace`
9. `cargo clippy --workspace`

### Invariants

1. Existing `EvidenceRef::Wound` variant unchanged
2. All existing event emission paths unaffected
3. MismatchKind uses `Quantity` and `CommodityKind` from core (no new numeric types)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` or `event_record.rs` — unit tests for MismatchKind construction and serialization roundtrip
2. `crates/worldwake-core/src/event_record.rs` — unit test for EvidenceRef::Mismatch construction

### Commands

1. `cargo test -p worldwake-core`
2. `cargo clippy --workspace`
3. `cargo test --workspace`
