# E17CRITHEJUS-011: Implement emit_justice_candidates()

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new candidate generation function in AI crate
**Deps**: Live goal, crime, and justice action surfaces already exist in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs), [`crates/worldwake-core/src/belief.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs), and [`crates/worldwake-systems/src/justice_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs)

## Problem

Agents cannot form accusation or punishment goals. No candidate generation function exists for `GoalKind::Accuse` or `GoalKind::PunishAccused`. Without `emit_justice_candidates()`, agents with evidence of theft and institutional authority cannot pursue justice.

## Assumption Reassessment (2026-03-26)

1. Shared boundary under audit: justice candidate generation must compose three already-live subjective surfaces without reaching around them:
   `ViolationMemory` unresolved theft cases passed into [`generate_candidates_with_travel_horizon()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs),
   typed theft testimony in [`SocialObservationDetail::SuspectedTheft`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs),
   and believed institutional crime-case claims in [`BelievedInstitutionalClaim`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs).
2. [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) does follow the established `emit_*` pattern, and `emit_crime_candidates()` currently emits only theft. Adding justice candidate emission there matches the live file structure.
3. Typed theft social evidence and relay support already exist. The ticket no longer depends on future `E17CRITHEJUS-015`/`016` work: `TellTopic::SocialObservation` is live, and `SocialObservationDetail::SuspectedTheft` is already relayable through the existing social candidate pipeline.
4. [`GoalKind::PunishAccused`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs) is richer than the original ticket text assumed. It already requires `office`, `accused`, `accusation_entry`, and `punishment`. Candidate generation therefore must derive a concrete office and consulted accusation entry, not just an accused agent plus punishment kind.
5. Mismatch: the original ticket claimed accusation should use any concrete theft evidence including possession evidence. The live authoritative `accuse` contract in [`actor_has_subjective_accusation_evidence()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs) is narrower: lawful accusation requires an unresolved `ViolationKind::SuspectedTheft` case plus either `suspect: Some(accused)` in that violation record or matching typed `SocialObservationDetail::SuspectedTheft` testimony. Possession-only evidence is not yet an accusation surface and is out of scope here.
6. Mismatch: the original ticket described punishment authority as a generic control query. The live authoritative contract is office-specific. [`validate_office_authority_at_place()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs) requires a concrete office whose `jurisdiction` matches the current place and whose current holder is the punisher. Candidate generation should therefore use believed office-holder reads, not `can_exercise_control()`.
7. Mismatch: punishment cannot be generated from arbitrary crime rumors. `PunishAccused` needs a concrete `accusation_entry`, and that only exists when the actor knows an accusation through `InstitutionalKnowledgeSource::RecordConsultation { record, entry_id }`. Report-only crime-case beliefs are insufficient for this goal and should not generate punishment candidates.
8. Mismatch: the original "Fine otherwise Exile" scope was too broad. The live `exile` action additionally requires the office to govern a faction and the accused to belong to that faction. Candidate generation must therefore emit exile only when it can derive a governed faction membership from the actor's beliefs; otherwise the candidate should be withheld rather than inventing an unverifiable exile target.
9. Profile-driven motive ordering already exists in [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs). This ticket should add candidate surfacing only and must not duplicate ranking logic inside candidate generation.

## Architecture Check

1. The clean architecture is to keep justice candidate generation belief-bound and action-contract-shaped: compose the exact same subjective theft and institutional record surfaces that the live justice handlers already validate, instead of inventing a broader candidate-only evidence model that the actions cannot execute.
2. `emit_justice_candidates()` should live under the existing crime-domain emitter and should not add parallel helper paths for accusation or punishment state. One candidate-generation surface is cleaner than action-specific fallbacks or duplicated record scans elsewhere.
3. Punishment candidate generation should require record-consulted accusation provenance up front. That keeps `GoalKind::PunishAccused` exact-bound to the consulted case entry and avoids aliasing "a punishment rumor" with "a punishable case artifact."
4. No backwards-compatibility aliasing.

## Verification Layers

1. Accusation evidence contract at candidate layer -> focused `candidate_generation` unit tests over `ViolationMemory` + typed social observation support
2. Duplicate-case suppression -> focused `candidate_generation` unit tests over current believed `CrimeCase` topics
3. Punishment authority and consulted-entry derivation -> focused `candidate_generation` unit tests over `BelievedInstitutionalClaim` plus office-holder beliefs
4. Punishment kind selection -> focused `candidate_generation` unit tests proving `Fine` only when the believed accessible amount satisfies the live fine contract, and `Exile` only when a governed faction membership exists
5. Goal/ranking separation -> `cargo test -p worldwake-ai` to ensure new candidate surfacing still cooperates with existing ranking, planner, and feasibility surfaces

## What to Change

### 1. New `emit_justice_candidates()` in `candidate_generation.rs`

Guard: return early if agent has no `JusticeDispositionProfile` component.

**Accusation sub-algorithm**:
1. Scan unresolved `ViolationMemory` records for `ViolationKind::SuspectedTheft`.
2. Derive accusation-worthy suspects only from the live authoritative subjective evidence surface:
   - `suspect: Some(entity)` directly on the unresolved theft case
   - matching typed `SocialObservationDetail::SuspectedTheft { theft, suspect: Some(entity) }` in the actor's belief store for the same `TheftFacts`
3. For each `(accused, violation_id)` pair: suppress the candidate if the actor already knows a current `CrimeCase` belief for that accused and violation, whether that current claim is an `Accusation` or a superseding `Verdict`
3. Emit `GroundedGoal { kind: GoalKind::Accuse { accused, violation_id }, motive: accusation_motive_weight, priority_class: GoalPriorityClass::Low }` via `emit_candidate_with_trace()`

**Punishment sub-algorithm**:
1. Scan the actor's current believed `CrimeCase` topics for live `InstitutionalClaim::Accusation` claims whose provenance is `InstitutionalKnowledgeSource::RecordConsultation { record, entry_id }`
2. Derive the concrete punishment office from `record_data(record).issuer`; require that record to still be a `CrimeRegister`
3. Require believed office-holder authority for that office and actor, plus a living accused
4. Determine punishment using the live action contract:
   - `Fine` only when `JusticeDispositionProfile.fine_severity` yields a non-zero amount and the accused is believed to hold at least that amount of the stolen commodity
   - otherwise `Exile` only when the office governs a faction that the accused is believed to belong to
   - if neither punishment can be bound lawfully, emit no punishment candidate
5. Emit `GroundedGoal { kind: GoalKind::PunishAccused { office, accused, accusation_entry, punishment }, ... }`

### 2. Wire into candidate generation dispatch

Add call to `emit_justice_candidates()` in the main dispatch function.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Theft candidate generation (E17CRITHEJUS-010)
- Accuse/Fine/Exile action handlers (E17CRITHEJUS-008/009)
- CrimeRegister entity setup (test infrastructure)
- Guard patrol crime response (E19)
- Appeal or contest logic (future spec)
- Refactoring institutional Tell topics; handled by `E17CRITHEJUS-017`

## Acceptance Criteria

### Tests That Must Pass

1. Agent with `JusticeDispositionProfile`, unresolved `SuspectedTheft`, and named suspect evidence from either the violation record or matching typed theft testimony -> `Accuse { accused: x }` candidate emitted
2. Agent with unresolved `SuspectedTheft { suspect: None }` and no matching typed theft testimony -> no accusation candidate
3. Agent without `JusticeDispositionProfile` -> no justice candidates at all
4. Current believed `CrimeCase` already present for the same accused + violation -> no duplicate accusation candidate
5. Agent with believed office-holder authority and record-consulted unresolved accusation -> `PunishAccused { office, accused, accusation_entry, ... }` candidate emitted
6. Agent without believed authority, or with report-only accusation knowledge lacking a consulted entry -> no punishment candidate
7. Punishment kind: `Fine` only when the believed fine amount is non-zero and affordable by the accused; otherwise `Exile` only when a governed faction membership can be derived
8. Motive from `JusticeDispositionProfile.accusation_motive_weight`
9. All candidates remain at `GoalPriorityClass::Low` through existing ranking
10. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Only agents with `JusticeDispositionProfile` ever generate justice candidates
2. Accusation requires the live subjective evidence contract naming a suspect; unknown-suspect theft evidence alone cannot produce `Accuse` (P14)
3. Punishment requires believed institutional office authority plus a consulted accusation entry (P21, P23)
4. Motive is profile-driven (P2)
5. No `HashMap`/`HashSet` in candidate scanning

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused tests for `emit_justice_candidates()` covering accusation and punishment sub-algorithms

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy -p worldwake-ai`

## Outcome

- Completed: 2026-03-26
- What changed:
  - Added `emit_justice_candidates()` under the existing crime candidate-generation domain in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs).
  - Accusation candidates now surface from the live subjective evidence contract only: unresolved `ViolationKind::SuspectedTheft` plus either a named suspect on that record or matching typed `SocialObservationDetail::SuspectedTheft` testimony.
  - Punishment candidates now surface only from record-consulted current `CrimeCase` accusations, deriving concrete `office`, `accusation_entry`, and `punishment` bindings from believed `CrimeRegister` data and office-holder beliefs.
  - Added focused `candidate_generation` coverage for accusation testimony, duplicate-case suppression, consulted-case punishment, report-only suppression, exile fallback, and withholding punishment when no lawful binding exists.
- Deviations from original plan:
  - Did not add possession-only accusation evidence. Reassessment showed the live authoritative `accuse` action does not accept that evidence surface yet.
  - Did not generate punishment from report-only crime beliefs. `GoalKind::PunishAccused` requires a concrete consulted accusation entry, so report-only knowledge remains insufficient by design.
  - Exile generation is narrower than the original ticket text: it now requires a governed faction membership instead of treating exile as an unconditional fallback.
- Verification results:
  - `cargo test -p worldwake-ai` passed.
  - `cargo clippy -p worldwake-ai -- -D warnings` passed.
