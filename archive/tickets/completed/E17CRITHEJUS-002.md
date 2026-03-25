# E17CRITHEJUS-002: Extend ViolationKind with SuspectedTheft

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — core enum extensions plus downstream exhaustive-match updates in AI/systems
**Deps**: E17CRITHEJUS-001 (needs `EntityId` only, which already exists; no actual blocker)

## Problem

When an owner investigates a missing entity, the system records `WitnessedAbsence` but cannot distinguish "item depleted/moved" from "owned item missing under theft suspicion." E17 needs a `SuspectedTheft` violation kind to bridge S27 violation state into the crime pipeline, plus a matching `SocialObservationKind::SuspectedTheft` so the suspicion is shareable via Tell.

## Assumption Reassessment (2026-03-25)

1. `ViolationKind` lives in `crates/worldwake-core/src/violation.rs` with variants `EntityMissing`, `SupplyDepleted`, and `EntityDead`. The live workspace contains explicit `ViolationKind` matching in `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-systems/src/investigate_actions.rs`, so this ticket is not core-only in practice.
2. `SocialObservationKind` lives in `crates/worldwake-core/src/belief.rs` and currently includes `WitnessedAbsence`. Most downstream call sites compare individual variants rather than exhaustively matching, so the main code change here is the enum extension plus focused serialization coverage.
3. `ViolationMemory` stores `RecordedViolation { id, kind, observed_tick, resolved_tick, expires_tick }` in `crates/worldwake-core/src/violation.rs`. `SuspectedTheft` should use the same identity/TTL machinery; no new storage path is needed.
4. The original "pure type additions in core" scope is stale. To keep the architecture coherent, downstream exhaustive matches must be updated with explicit semantics:
   - `worldwake-ai/src/candidate_generation.rs`: `SuspectedTheft` must NOT emit another generic `InvestigateViolation` goal.
   - `worldwake-systems/src/investigate_actions.rs`: `SuspectedTheft` must NOT be considered investigable by the generic investigate action.
5. N/A.
6. N/A.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. Mismatch: the original ticket claimed placeholder no-op downstream arms and deferred all AI/system impact. Correct scope is narrower and cleaner: add the new core variants, then explicitly mark `SuspectedTheft` as a non-generic-violation terminal state in current AI/system seams so it cannot loop back into S27 investigation.
12. N/A.

## Architecture Check

1. Adding `ViolationKind::SuspectedTheft` and `SocialObservationKind::SuspectedTheft` is architecturally better than overloading `EntityMissing`/`WitnessedAbsence` because theft suspicion is a different semantic fact: the absence has already been investigated and reclassified. Keeping it explicit preserves clean state for later accusation/punishment work and avoids cargo-cult boolean flags or tuple overloading in unrelated code.
2. No backwards-compatibility aliasing introduced.

## Verification Layers

1. Core data contract for `ViolationKind::SuspectedTheft` -> focused unit test in `crates/worldwake-core/src/violation.rs`
2. Core data contract for `SocialObservationKind::SuspectedTheft` -> focused unit test in `crates/worldwake-core/src/belief.rs`
3. Generic investigation pipeline does not recurse on `SuspectedTheft` -> focused unit test in `crates/worldwake-ai/src/candidate_generation.rs`
4. Investigate action does not enumerate/bind `SuspectedTheft` as a generic investigable violation -> focused unit test in `crates/worldwake-systems/src/investigate_actions.rs`
5. Additional layer mapping beyond focused unit/runtime coverage is not applicable because this ticket deliberately stops before the actual investigate-commit reclassification work in `E17CRITHEJUS-007`.

## What to Change

### 1. Add `SuspectedTheft` to `ViolationKind` in `violation.rs`

```rust
SuspectedTheft {
    missing_entity: EntityId,
    expected_place: EntityId,
    suspect: Option<EntityId>,
}
```

Update explicit `ViolationKind` matches across the workspace with non-recursive semantics:
- `candidate_generation.rs`: return early for `SuspectedTheft` in generic violation-goal emission
- `investigate_actions.rs`: treat `SuspectedTheft` as non-investigable in the generic investigate binding path

### 2. Add `SuspectedTheft` to `SocialObservationKind` in `belief.rs`

Add a unit variant `SuspectedTheft` to the `SocialObservationKind` enum.

### 3. Update match arms in downstream crates

Update the current downstream `ViolationKind` matches that would otherwise incorrectly treat the new variant as part of the generic S27 investigation loop. Do not add wildcard arms.

## Files to Touch

- `crates/worldwake-core/src/violation.rs` (modify)
- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)

## Out of Scope

- The investigate commit handler extension that PRODUCES `SuspectedTheft` after ownership-aware investigation (E17CRITHEJUS-007)
- AI candidate generation consuming `SuspectedTheft` (E17CRITHEJUS-010/011)
- Steal action definition (E17CRITHEJUS-006)
- Golden tests

## Acceptance Criteria

### Tests That Must Pass

1. `ViolationKind::SuspectedTheft` serde round-trip with `suspect: Some(id)` and `suspect: None`
2. `SocialObservationKind::SuspectedTheft` serde round-trip
3. `ViolationMemory` can store and retrieve a `SuspectedTheft` entry with TTL
4. `Ord` ordering is stable for `ViolationKind` with new variant
5. Generic violation candidate generation does not emit `InvestigateViolation` for `SuspectedTheft`
6. Investigate payload enumeration/validation does not treat `SuspectedTheft` as generically investigable
7. Existing suite: `cargo test -p worldwake-core`
8. Relevant focused downstream suites pass
9. Existing suite: `cargo build --workspace` (all explicit match arms compile)

### Invariants

1. `ViolationKind` remains `Eq + Ord + Serialize + Deserialize`
2. Downstream `ViolationKind` handling remains explicit; do not hide the new state behind `_ =>` wildcards
3. `SuspectedTheft` is a terminal reclassification for the generic S27 investigate loop until later E17 tickets intentionally consume it
4. No behavioral change to existing detection of `EntityMissing`, `SupplyDepleted`, or `EntityDead`

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/violation.rs` — serde round-trip, ordering, and `ViolationMemory` storage for `SuspectedTheft`
2. `crates/worldwake-core/src/belief.rs` — serde round-trip for `SocialObservationKind::SuspectedTheft`
3. `crates/worldwake-ai/src/candidate_generation.rs` — focused regression proving `SuspectedTheft` does not generate a generic `InvestigateViolation`
4. `crates/worldwake-systems/src/investigate_actions.rs` — focused regression proving `SuspectedTheft` is not treated as a generic investigate payload

### Commands

1. `cargo test -p worldwake-core suspected_theft`
2. `cargo test -p worldwake-ai suspected_theft`
3. `cargo test -p worldwake-systems suspected_theft`
4. `cargo test -p worldwake-core`
5. `cargo build --workspace`
6. `cargo clippy --workspace`

## Outcome

Completion date: 2026-03-25

What actually changed:

1. Added `ViolationKind::SuspectedTheft { missing_entity, expected_place, suspect }` in `crates/worldwake-core/src/violation.rs`.
2. Added `SocialObservationKind::SuspectedTheft` in `crates/worldwake-core/src/belief.rs`.
3. Updated the live generic-investigation seams so the new violation state is explicit and terminal for S27-style investigate handling:
   - `crates/worldwake-ai/src/candidate_generation.rs` now returns early for `SuspectedTheft` in generic violation-goal emission.
   - `crates/worldwake-systems/src/investigate_actions.rs` now keeps `SuspectedTheft` out of generic investigate payload binding and authoritative validation.
4. Added focused regression coverage in core, AI, and systems for serialization/storage and for the "do not recurse through generic investigation" boundary.

Deviations from original plan:

1. The original ticket claimed a pure core-only change with placeholder downstream no-op arms. The live code required explicit AI/system handling to keep `SuspectedTheft` from being treated as another generic investigable violation.
2. No `SocialObservationKind` downstream exhaustive-match updates were required beyond focused test coverage; current call sites compare individual variants rather than exhaustively matching the enum.

Verification results:

1. `cargo test -p worldwake-core suspected_theft` — passed
2. `cargo test -p worldwake-ai suspected_theft` — passed
3. `cargo test -p worldwake-systems suspected_theft` — passed
4. `cargo test -p worldwake-core` — passed
5. `cargo test -p worldwake-ai` — passed
6. `cargo test -p worldwake-systems` — passed
7. `cargo build --workspace` — passed
8. `cargo clippy --workspace` — passed
