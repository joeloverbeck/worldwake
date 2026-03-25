# E17CRITHEJUS-015: Replace tuple-overloaded SocialObservation subjects with typed evidence detail

**Status**: COMPLETED
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
3. `ViolationKind::SuspectedTheft { missing_entity, expected_place, suspect }` already exists in `crates/worldwake-core/src/violation.rs`, and `investigate_actions::tests::suspected_theft_violation_is_not_exposed_as_generic_investigate_affordance` plus `candidate_generation::tests::suspected_theft_record_does_not_emit_generic_investigate_goal` already treat it as a first-class authoritative/theft-specific record. The unresolved contradiction is specifically the belief-side `SocialObservation` payload shape.
4. No active implementation ticket currently changes the `SocialObservation` data model. `E17CRITHEJUS-007`, `E17CRITHEJUS-008`, `E17CRITHEJUS-011`, `E17CRITHEJUS-012`, and `E17CRITHEJUS-013` all consume or depend on it, but none currently remove the tuple ambiguity themselves.
5. The live `tell` system in `crates/worldwake-systems/src/tell_actions.rs` enumerates relay topics from `listener_aware_relayable_subjects(view.known_entity_beliefs(actor), ...)` and only emits `TellActionPayload { listener, subject_entity }`. `worldwake-sim/src/social_relay.rs` likewise only ranks entity-belief subjects. The E17 spec paragraph claiming `SocialObservation(SuspectedTheft)` is already shareable through Tell is therefore false in the current codebase. That relay gap is real, but distinct from this ticket; relay plumbing belongs in `E17CRITHEJUS-016`.
6. Focused coverage already exists for tuple-based observation behavior in:
   - `belief::tests::record_social_observation_appends_to_list`
   - `belief::tests::agent_belief_store_roundtrips_through_bincode_with_institutional_beliefs`
   - `perception::tests::social_observations_for_event_maps_actor_targets_and_tick`
   - `investigate_actions::tests::investigate_action_commits_witnessed_absence_and_extends_violation_memory`
   - `worldwake-ai/tests/golden_social.rs` witnessed-telling assertions
   - `worldwake-ai/tests/golden_emergent.rs` witnessed-absence aftermath assertions
7. `cargo test -p worldwake-core belief::tests:: -- --list`, `cargo test -p worldwake-systems investigate_actions::tests:: -- --list`, and `cargo test -p worldwake-ai -- --list` all confirm the relevant current test targets exist. The ticket should name those real surfaces instead of only broad package commands.
8. This remains a cross-layer schema ticket, not a "tests only" cleanup. Leaving the tuple in place would force crime-specific aliasing or positional conventions into later E17 tickets and would duplicate meaning already modeled more explicitly in `ViolationKind::SuspectedTheft`.
11. Mismatch: the active E17 material diverges in two places. First, the current ticket set assumes `SocialObservationKind::SuspectedTheft` can simultaneously encode `(missing_entity, expected_place)` and "evidence against accused X)," which is impossible with the live tuple schema. Second, `specs/E17-crime-theft-justice.md` is internally contradictory: it both documents `subjects: (missing_entity, expected_place)` and claims no Tell changes are needed for sharing `SocialObservation(SuspectedTheft)`, while the live Tell path is entity-belief-only. Correct scope for this ticket is to replace tuple-overloaded social observation payloads with typed detail first; relay and accusation consumption remain follow-up work in `E17CRITHEJUS-016` and downstream tickets.
12. N/A.

## Architecture Check

1. The cleaner architecture is to replace positional tuple semantics with typed observation detail, so each observation variant names its concrete fields directly. That keeps evidence inspectable, composable, and safe for future relay, accusation, and debugging work.
2. This is better than adding one-off helper conventions like "for `SuspectedTheft`, tuple slot 0 means missing item and tuple slot 1 sometimes means place, sometimes suspect" because that would encode crime logic as undocumented ordering rules rather than world state.
3. No backwards-compatibility aliasing or parallel schemas should be introduced. Replace the old tuple field outright and update consumers.

## Verification Layers

1. Typed social observation detail serializes and round-trips correctly -> `belief::tests::*` focused unit coverage in `crates/worldwake-core/src/belief.rs`
2. Existing perception-driven social observations still record the correct participants after the schema change -> `perception::tests::*` focused runtime coverage in `crates/worldwake-systems/src/perception.rs`
3. Investigate aftermath still records the correct absence detail with explicit fields rather than tuple decoding -> `investigate_actions::tests::*` focused runtime coverage in `crates/worldwake-systems/src/investigate_actions.rs`
4. Existing office/social/golden assertions still match observations through explicit typed fields -> focused runtime coverage in `crates/worldwake-systems/src/office_actions.rs` plus existing `golden_social` / `golden_emergent` suites
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
5. Existing suite: `cargo test -p worldwake-core belief::tests::`
6. Existing suite: `cargo test -p worldwake-systems perception::tests::`
7. Existing suite: `cargo test -p worldwake-systems investigate_actions::tests::`
8. Existing suite: `cargo test -p worldwake-systems --test e15_information_integration bystander_observes_witnessed_telling_without_receiving_subject_belief`
9. Existing suite: `cargo test -p worldwake-ai golden_bystander_sees_telling_but_gets_no_belief`
10. Existing suite: `cargo test -p worldwake-ai golden_same_place_concurrent_violations_stay_distinct`
11. Existing suite: `cargo test -p worldwake-ai golden_entity_missing_triggers_investigation`
12. Existing suite: `cargo build --workspace`
13. Existing suite: `cargo clippy --workspace`

### Invariants

1. Social evidence remains concrete world-state memory, not tuple-decoding convention (P3, P16)
2. Observation meaning is explicit at the data-model boundary, not inferred ad hoc by consumers
3. No compatibility field like `subjects` remains in parallel with the typed detail representation

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — added typed social-observation detail serialization and derived-kind coverage; updated sample/store tests to use explicit detail instead of tuple fields
2. `crates/worldwake-systems/src/perception.rs` — updated perception observation-generation tests to assert explicit typed social detail for cooperation, conflict, obligation, and telling evidence
3. `crates/worldwake-systems/src/investigate_actions.rs` — updated investigate aftermath test to assert explicit typed witnessed-absence detail
4. `crates/worldwake-systems/tests/e15_information_integration.rs` — updated the bystander witnessed-telling integration assertion to use typed social detail
5. `crates/worldwake-ai/tests/golden_social.rs` — updated the bystander-telling golden to assert typed witnessed-telling detail
6. `crates/worldwake-ai/tests/golden_emergent.rs` — updated same-place violation and entity-missing goldens to assert typed witnessed-absence detail

### Commands

1. `cargo test -p worldwake-core belief::tests::`
2. `cargo test -p worldwake-systems perception::tests::`
3. `cargo test -p worldwake-systems investigate_actions::tests::`
4. `cargo test -p worldwake-systems --test e15_information_integration bystander_observes_witnessed_telling_without_receiving_subject_belief`
5. `cargo test -p worldwake-ai golden_bystander_sees_telling_but_gets_no_belief`
6. `cargo test -p worldwake-ai golden_same_place_concurrent_violations_stay_distinct`
7. `cargo test -p worldwake-ai golden_entity_missing_triggers_investigation`
8. `cargo build --workspace`
9. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-25
- What actually changed:
  - Replaced `SocialObservation.subjects: (EntityId, EntityId)` with `SocialObservation.detail: SocialObservationDetail`
  - Added derived `kind()` helpers so observation-family queries stay explicit without keeping a duplicate source of truth
  - Updated perception/investigate observation producers and the focused/integration/golden assertions that depended on raw tuple semantics
- Deviations from original plan:
  - No relay-path work was added. The ticket stayed at the belief-schema and local producer/consumer boundary, leaving Tell relay to `E17CRITHEJUS-016`
  - The ticket now explicitly records the live spec contradiction around Tell sharing instead of treating the spec as fully aligned with code
- Verification results:
  - Passed `cargo test -p worldwake-core belief::tests::`
  - Passed `cargo test -p worldwake-systems perception::tests::`
  - Passed `cargo test -p worldwake-systems investigate_actions::tests::`
  - Passed `cargo test -p worldwake-systems --test e15_information_integration bystander_observes_witnessed_telling_without_receiving_subject_belief`
  - Passed `cargo test -p worldwake-ai golden_bystander_sees_telling_but_gets_no_belief`
  - Passed `cargo test -p worldwake-ai golden_same_place_concurrent_violations_stay_distinct`
  - Passed `cargo test -p worldwake-ai golden_entity_missing_triggers_investigation`
  - Passed `cargo build --workspace`
  - Passed `cargo clippy --workspace`
