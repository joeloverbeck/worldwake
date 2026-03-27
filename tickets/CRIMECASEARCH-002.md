# CRIMECASEARCH-002: Reassess Residual Institutional `ViolationId` Coupling In Crime Knowledge Surfaces

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core`, `worldwake-systems`, `worldwake-ai`, docs/tests
**Deps**: `archive/tickets/not-implemented/CRIMECASEARCH-001.md`, E16c (institutional beliefs), E17 (crime/theft/justice), `docs/FOUNDATIONS.md`, `docs/planner-contracts.md`

## Problem

`CRIMECASEARCH-001` was retired because the repository already has a durable institutional crime-case lane through the append-only `CrimeRegister` plus record-entry supersession. That makes this ticket's original dependency and end-state stale.

There is still a possible cleanup here: some institutional crime knowledge grouping surfaces continue to use `(accused, violation_id)` even though the authoritative institutional lane is already the record-entry / supersession lineage. If that cleanup is worth doing, it should align those knowledge surfaces to the existing record lane rather than inventing a new case artifact.

## Assumption Reassessment (2026-03-27)

1. The exact shared abstraction boundary under audit is institutional knowledge transport and grouping for crime cases:
   - `crates/worldwake-core/src/{institutional.rs,belief.rs}`
   - `crates/worldwake-systems/src/{tell_actions.rs,consult_record_actions.rs}`
   - planner-facing reads in `crates/worldwake-ai/src/candidate_generation.rs`
   - trace formatting in `crates/worldwake-ai/src/decision_trace.rs`
2. The original dependency is stale. [CRIMECASEARCH-001](/home/joeloverbeck/projects/worldwake/archive/tickets/not-implemented/CRIMECASEARCH-001.md) was archived as not implemented because a new first-class crime-case artifact would duplicate the existing authoritative register lane instead of improving it.
3. Live crime knowledge grouping still uses `violation_id` on institutional surfaces:
   - `InstitutionalBeliefKey::CrimeCase { accused, violation_id }` in `crates/worldwake-core/src/institutional.rs`
   - `InstitutionalTellTopicKey::CrimeCase { accused, violation_id }` in `crates/worldwake-core/src/belief.rs`
   - keying helpers in `crates/worldwake-systems/src/{tell_actions.rs,consult_record_actions.rs}`
4. Live planner punishment behavior is not actually keyed by those grouping enums. The strongest current institutional boundary is already the consulted record source:
   - `InstitutionalKnowledgeSource::RecordConsultation { record, entry_id }`
   - `GoalKind::PunishAccused { office, accused, accusation_entry, punishment }`
   - justice runtime validation against `payload.accusation_entry`
   So the current architectural issue is narrower than the original ticket claimed.
5. Information-path analysis, corrected: today the same crime fact still has multiple lawful paths.
   - Path A: local subjective evidence uses `ViolationId` through `ViolationMemory`, `GoalKind::Accuse`, and accuse payload binding
   - Path B: institutionalized knowledge flows through `CrimeRegister` entries, consultation provenance, and later punishment selection
   The canonical institutional path is already Path B. This ticket should only tighten the residual grouping surfaces around that existing lane.
6. The intended invariant is not "replace `ViolationId` everywhere." It is: once a crime is institutionalized, knowledge grouping and traceability should not pretend the local evidence id is the authoritative case identity when a stronger institutional lane already exists.
7. The cleanest possible follow-up is not a new world artifact. It is a narrower rekeying around the existing record lane, likely some explicit institutional case-lane key derived from the crime register lineage. That key must be defined carefully because `RecordConsultation { record, entry_id }` alone is not enough for accusation-to-verdict grouping: verdict entries use a newer `entry_id` and refer back through `supersedes`.
8. That means the original "just replace `(accused, violation_id)` with case artifact id" plan is no longer implementable as written. A viable follow-up would first need to define the real canonical institutional grouping key, for example a root accusation lane in the record lineage.
9. Existing tests already prove the most important behavior without this cleanup:
   - duplicate filing suppression across differing local evidence ids
   - punishment generation from consulted accusation entries
   - S32 Scenarios 41-43 in `crates/worldwake-ai/tests/golden_emergent.rs`
   So this ticket is now architecture-cleanup work, not a correctness blocker.
10. Adjacent contradiction exposed during reassessment: if we do this cleanup, report-only institutional claims may still lack a stable record-lane anchor. That is not a reason to keep the stale dependency on `CRIMECASEARCH-001`; it is a design constraint that must be resolved explicitly before code changes begin.
11. Mismatch + correction: this ticket no longer depends on a first-class case artifact. If it proceeds, it should rekey residual institutional crime grouping to an explicit key derived from the existing `CrimeRegister` lineage, or be retired if that key does not pull its weight.

## Architecture Check

1. The corrected architecture is cleaner than the original ticket: keep one authoritative institutional source of truth, the crime register lane, and if needed align grouping surfaces to that lane.
2. Repeating the `CRIMECASEARCH-001` proposal here would be worse architecture. A new sidecar case object would duplicate accused/theft/procedural state that the record lane already carries.
3. This follow-up is only worth doing if it produces a single explicit institutional grouping key that is stronger than `(accused, violation_id)` and materially improves reasoning, transport, or debugging. Otherwise the current state is acceptable because the real authoritative runtime path is already record-entry based.
4. No backwards-compatibility aliasing or dual identity lanes should be introduced. If this refactor happens, the old institutional `CrimeCase { accused, violation_id }` grouping should be removed, not mirrored.

## Verification Layers

1. institutional crime-topic grouping aligns to an explicit record-lane-derived key -> focused `worldwake-core` / `worldwake-systems` tests
2. punishment and accusation planner behavior still bind to lawful institutional inputs after regrouping -> focused `worldwake-ai` candidate-generation and conformance tests
3. trace formatting remains legible and uses the canonical institutional lane wording -> focused `worldwake-ai/src/decision_trace.rs` tests
4. existing crime goldens remain deterministic and behaviorally unchanged -> `crates/worldwake-ai/tests/golden_emergent.rs` Scenarios 38, 41, and 43
5. if reassessment cannot define a clean canonical key, strongest proof remains the existing accusation-entry/runtime boundary and this ticket should be retired rather than forced through

## What to Change

### 1. Remove the stale dependency and stale artifact narrative

Treat `CRIMECASEARCH-001` as an archived reassessment, not as a missing prerequisite implementation.

### 2. If implemented, define the real canonical institutional grouping key first

Before any production refactor, define the exact key that represents one institutional crime lane under the existing register architecture. It must account for accusation-to-verdict supersession rather than blindly reusing `entry_id`.

### 3. Rekey only the residual knowledge/grouping surfaces

If the new key is justified, update:

- `InstitutionalBeliefKey::CrimeCase`
- `InstitutionalTellTopicKey::CrimeCase`
- tell / consult grouping helpers
- any crime-specific trace wording that still claims the institutional lane is a `violation_id`

Do not redesign local evidence binding or runtime punishment validation in this ticket.

### 4. Preserve the local-vs-institutional split

`ViolationId` may remain on local subjective evidence-binding surfaces such as `ViolationMemory`, `GoalKind::Accuse`, and accuse payload binding. The goal is to stop overloading it as nominal institutional identity where a stronger record lane exists.

## Files to Touch

- `tickets/CRIMECASEARCH-002.md` (modify)
- `crates/worldwake-core/src/institutional.rs` (modify, if implemented)
- `crates/worldwake-core/src/belief.rs` (modify, if implemented)
- `crates/worldwake-systems/src/tell_actions.rs` (modify, if implemented)
- `crates/worldwake-systems/src/consult_record_actions.rs` (modify, if implemented)
- `crates/worldwake-ai/src/decision_trace.rs` (modify, if implemented)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify only if grouping fallout requires it)
- `crates/worldwake-ai/tests/planner_conformance.rs` (modify, if implemented)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify, if implemented)

## Out of Scope

- creating a new `CrimeCase` world artifact or `CrimeCaseId`
- changing the authoritative punishment runtime away from `accusation_entry`
- redesigning non-crime institutional grouping
- smearing local evidence identity and institutional identity back together
- keeping both a new grouping key and the old `violation_id` institutional grouping in parallel

## Acceptance Criteria

### Tests That Must Pass

1. the ticket dependency and architectural narrative accurately match the archived `CRIMECASEARCH-001` outcome
2. if production code is changed, focused tests prove crime-topic grouping uses the new explicit record-lane-derived key rather than institutional `violation_id`
3. `cargo test -p worldwake-core`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. There is only one authoritative institutional crime-case lane: the crime register lineage.
2. Local evidence identity and institutional identity remain distinct and explicitly named.
3. This ticket must not depend on a first-class crime-case artifact that the repository has explicitly rejected.

## Test Plan

### New/Modified Tests

1. None yet. This ticket correction is documentation/scope-only. If the production cleanup proceeds later, add focused grouping and trace tests at that time.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges_without_double_accusation -- --exact`
2. `cargo test -p worldwake-systems justice_actions::tests::duplicate_accusation_for_same_theft_rejects_at_start_even_with_different_violation_id -- --exact`
3. `cargo test -p worldwake-ai candidate_generation::tests::justice_candidates_suppress_duplicate_accusation_when_same_theft_is_already_recorded_under_different_violation_id -- --exact`
