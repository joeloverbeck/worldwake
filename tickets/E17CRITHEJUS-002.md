# E17CRITHEJUS-002: Extend ViolationKind with SuspectedTheft

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — enum extensions in core
**Deps**: E17CRITHEJUS-001 (needs `EntityId` only, which already exists; no actual blocker)

## Problem

When an owner investigates a missing entity, the system records `WitnessedAbsence` but cannot distinguish "item depleted" from "item stolen." E17 needs a `SuspectedTheft` violation kind to bridge violation detection to the crime pipeline, and a matching `SocialObservationKind::SuspectedTheft` so the suspicion is shareable via Tell.

## Assumption Reassessment (2026-03-25)

1. `ViolationKind` enum lives in `crates/worldwake-core/src/violation.rs` with variants `EntityMissing`, `SupplyDepleted`, `EntityDead`. All match arms across the workspace must be updated.
2. `SocialObservationKind` enum lives in `crates/worldwake-core/src/belief.rs` with variants including `WitnessedAbsence`. Same exhaustive-match update requirement.
3. `ViolationMemory` stores `RecordedViolation` entries keyed by `ViolationId`. `SuspectedTheft` entries will use the same storage and TTL mechanism.
4. Not an AI ticket — pure type additions in core.
5. N/A.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatches found.
12. N/A.

## Architecture Check

1. Adding enum variants to `ViolationKind` and `SocialObservationKind` follows the established extension pattern (S27 added `EntityMissing`/`WitnessedAbsence` the same way). The compiler enforces exhaustive match updates.
2. No backwards-compatibility aliasing introduced.

## Verification Layers

1. `ViolationKind::SuspectedTheft` serde round-trip -> focused unit test
2. `SocialObservationKind::SuspectedTheft` serde round-trip -> focused unit test
3. `ViolationMemory` can record and retrieve `SuspectedTheft` entries -> focused unit test
4. Single-crate ticket; cross-layer mapping not applicable.

## What to Change

### 1. Add `SuspectedTheft` to `ViolationKind` in `violation.rs`

```rust
SuspectedTheft {
    missing_entity: EntityId,
    expected_place: EntityId,
    suspect: Option<EntityId>,
}
```

Update all exhaustive matches on `ViolationKind` across the workspace.

### 2. Add `SuspectedTheft` to `SocialObservationKind` in `belief.rs`

Add a unit variant `SuspectedTheft` to the `SocialObservationKind` enum. Update all exhaustive matches.

### 3. Update match arms in downstream crates

Grep for `ViolationKind::` and `SocialObservationKind::` match arms in worldwake-ai (candidate_generation, feasibility, etc.) and add placeholder arms that do nothing for now (the AI integration is E17CRITHEJUS-005/010/011).

## Files to Touch

- `crates/worldwake-core/src/violation.rs` (modify)
- `crates/worldwake-core/src/belief.rs` (modify)
- Any file with exhaustive match on `ViolationKind` or `SocialObservationKind` (modify — add match arms)

## Out of Scope

- The investigate commit handler extension that produces `SuspectedTheft` (E17CRITHEJUS-007)
- AI candidate generation consuming `SuspectedTheft` (E17CRITHEJUS-010/011)
- Steal action definition (E17CRITHEJUS-006)
- Golden tests

## Acceptance Criteria

### Tests That Must Pass

1. `ViolationKind::SuspectedTheft` serde round-trip with `suspect: Some(id)` and `suspect: None`
2. `SocialObservationKind::SuspectedTheft` serde round-trip
3. `ViolationMemory` can store and retrieve a `SuspectedTheft` entry with TTL
4. `Ord` ordering is stable for `ViolationKind` with new variant
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo build --workspace` (all match arms compile)

### Invariants

1. `ViolationKind` remains `Eq + Ord + Serialize + Deserialize`
2. All existing exhaustive matches compile without `_ =>` wildcards
3. No behavioral change to existing violation detection or social observation handling

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/violation.rs` — serde round-trip and ViolationMemory storage for `SuspectedTheft`
2. `crates/worldwake-core/src/belief.rs` — serde round-trip for new `SocialObservationKind` variant

### Commands

1. `cargo test -p worldwake-core`
2. `cargo build --workspace`
3. `cargo clippy --workspace`
