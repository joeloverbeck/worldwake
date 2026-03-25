# E17CRITHEJUS-015: Replace tuple-overloaded SocialObservation subjects with typed evidence detail

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — core belief schema refactor plus downstream observation constructor/query updates
**Deps**: E17CRITHEJUS-002

## Problem

`SocialObservation` currently stores `kind` plus a generic `subjects: (EntityId, EntityId)` tuple in `crates/worldwake-core/src/belief.rs`. That shape is too ambiguous for crime evidence. The live E17 tickets already diverge on what `SocialObservationKind::SuspectedTheft` should mean: `E17CRITHEJUS-007` proposes `(missing_entity, expected_place)`, while `E17CRITHEJUS-008` assumes accusation can validate that same observation against an accused agent.

That ambiguity violates the foundations:

1. P3 / P16 require concrete, inspectable evidence state, not tuple conventions that must be reinterpreted differently per variant.
2. P13 requires evidence to remain attributable when relayed or consulted later.
3. P24 argues against pushing variant-specific decoding logic into every consumer.

## Assumption Reassessment (2026-03-25)

1. `SocialObservation` in `crates/worldwake-core/src/belief.rs` is currently:
   - `kind: SocialObservationKind`
   - `subjects: (EntityId, EntityId)`
   - `place`, `observed_tick`, `source`
2. Existing non-crime call sites already rely on implicit tuple semantics:
   - `crates/worldwake-systems/src/perception.rs` records `(actor, target)` for conflict/cooperation/obligation/telling observations.
   - `crates/worldwake-systems/src/investigate_actions.rs` records `(missing_entity, place)` for `WitnessedAbsence`.
   - `crates/worldwake-systems/src/office_actions.rs` and multiple focused/golden tests compare `observation.subjects == (...)` directly.
3. No active ticket currently changes the `SocialObservation` data model. `E17CRITHEJUS-007`, `E17CRITHEJUS-008`, `E17CRITHEJUS-011`, `E17CRITHEJUS-012`, and `E17CRITHEJUS-013` all consume or depend on it, but none currently fix the schema contradiction.
4. The live `tell` system in `crates/worldwake-systems/src/tell_actions.rs` relays `known_entities` and institutional beliefs via `TellActionPayload { subject_entity }`; it does not relay `SocialObservation` records. That relay gap is real, but distinct from this ticket. This ticket should fix the evidence shape first; relay plumbing belongs in a follow-up.
5. Focused coverage already exists for tuple-based observation behavior in:
   - `crates/worldwake-core/src/belief.rs`
   - `crates/worldwake-systems/src/perception.rs`
   - `crates/worldwake-systems/src/investigate_actions.rs`
   - `crates/worldwake-ai/tests/golden_social.rs`
   - `crates/worldwake-ai/tests/golden_emergent.rs`
6. This is a cross-layer schema ticket, not a "tests only" cleanup. Leaving the tuple in place would force crime-specific aliasing or magic positional conventions into later E17 tickets.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. Mismatch: the current E17 ticket set assumes `SocialObservationKind::SuspectedTheft` can simultaneously encode `(missing_entity, expected_place)` and "evidence against accused X." That is impossible with the live tuple schema. Correct scope is to introduce typed social-evidence detail before the remaining crime tickets implement observation production/consumption.
12. N/A.

## Architecture Check

1. The cleaner architecture is to replace positional tuple semantics with typed observation detail, so each observation variant names its concrete fields directly. That keeps evidence inspectable, composable, and safe for future relay, accusation, and debugging work.
2. This is better than adding one-off helper conventions like "for `SuspectedTheft`, tuple slot 0 means missing item and tuple slot 1 sometimes means place, sometimes suspect" because that would encode crime logic as undocumented ordering rules rather than world state.
3. No backwards-compatibility aliasing or parallel schemas should be introduced. Replace the old tuple field outright and update consumers.

## Verification Layers

1. Typed social observation detail serializes and round-trips correctly -> focused unit tests in `crates/worldwake-core/src/belief.rs`
2. Existing perception-driven social observations still record the correct participants after the schema change -> focused unit tests in `crates/worldwake-systems/src/perception.rs`
3. Investigate aftermath still records the correct absence detail with explicit fields rather than tuple decoding -> focused unit tests in `crates/worldwake-systems/src/investigate_actions.rs`
4. Existing social/office tests and goldens still match observations through explicit typed fields -> focused runtime coverage plus existing golden/E2E suites
5. Additional relay-layer mapping is not applicable here because this ticket intentionally stops at schema and local production/consumption updates. Relay is covered by follow-up ticket `E17CRITHEJUS-016`.

## What to Change

### 1. Replace tuple subjects with typed observation detail in `belief.rs`

Refactor `SocialObservation` to carry typed detail instead of `subjects: (EntityId, EntityId)`.

Recommended shape:

```rust
pub struct SocialObservation {
    pub detail: SocialObservationDetail,
    pub place: EntityId,
    pub observed_tick: Tick,
    pub source: PerceptionSource,
}

pub enum SocialObservationDetail {
    WitnessedCooperation { actor: EntityId, counterpart: EntityId },
    WitnessedConflict { actor: EntityId, target: EntityId },
    WitnessedObligation { actor: EntityId, target: EntityId },
    WitnessedTelling { speaker: EntityId, listener: EntityId },
    CoPresence { other: EntityId },
    WitnessedAbsence { missing_entity: EntityId, expected_place: EntityId },
    SuspectedTheft {
        missing_entity: EntityId,
        expected_place: EntityId,
        suspect: Option<EntityId>,
    },
}
```

Retaining `SocialObservationKind` as a derived helper is acceptable if it is computed from `detail`, not stored as a second source of truth.

### 2. Update observation constructors and queries

Update all current producers/consumers to use explicit fields:

- `crates/worldwake-systems/src/perception.rs`
- `crates/worldwake-systems/src/investigate_actions.rs`
- `crates/worldwake-systems/src/office_actions.rs`
- focused tests and existing golden assertions that currently compare raw tuples

### 3. Add helper accessors only where they remove repetition

If repeated matching emerges, add explicit helpers like `kind()`, `participants()`, or `missing_entity()` on `SocialObservationDetail`. Do not add compatibility shims that preserve raw tuple access.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-systems/src/perception.rs` (modify)
- `crates/worldwake-systems/src/investigate_actions.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)
- Existing focused/golden tests that assert on `SocialObservation.subjects` (modify)

## Out of Scope

- Relaying social observations through Tell or conversation memory (`E17CRITHEJUS-016`)
- Accuse/Fine/Exile action logic
- New crime AI candidate generation logic
- New golden scenarios beyond what is needed to keep existing coverage compiling and precise

## Acceptance Criteria

### Tests That Must Pass

1. `SocialObservation` round-trip preserves typed detail for all existing observation families
2. `SocialObservationDetail::SuspectedTheft` explicitly preserves `missing_entity`, `expected_place`, and `suspect`
3. Perception-focused tests still prove conflict/cooperation/obligation/telling observations with explicit typed participants
4. Investigate-focused tests still prove absence observation aftermath with explicit typed fields
5. Existing suite: `cargo test -p worldwake-core`
6. Existing suite: `cargo test -p worldwake-systems`
7. Existing suite: `cargo test -p worldwake-ai`
8. Existing suite: `cargo build --workspace`
9. Existing suite: `cargo clippy --workspace`

### Invariants

1. Social evidence remains concrete world-state memory, not tuple-decoding convention (P3, P16)
2. Observation meaning is explicit at the data-model boundary, not inferred ad hoc by consumers
3. No compatibility field like `subjects` remains in parallel with the typed detail representation

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — typed social-observation serialization and helper coverage
2. `crates/worldwake-systems/src/perception.rs` — existing observation-generation tests updated to assert explicit typed detail
3. `crates/worldwake-systems/src/investigate_actions.rs` — observation aftermath tests updated to assert explicit typed absence detail
4. Existing office/social golden assertions updated where they currently compare `subjects` tuples

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai`
4. `cargo build --workspace`
5. `cargo clippy --workspace`
