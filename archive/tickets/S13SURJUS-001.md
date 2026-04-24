# S13SURJUS-001: Preserve suspected-theft promotion after owner transfer

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — investigation-to-justice evidence transport
**Deps**: `docs/scenario-roadmap.md` row 13 `survival-justice`

## Problem

`survival-justice` could not retain an accusation-ready theft case once the stolen lot lawfully changed owner. `investigate` only promoted an `EntityMissing` violation into `SuspectedTheft` when the investigator still currently believed they owned the missing lot, so a witnessed real theft could disappear into generic missing-item churn before `GoalKind::Accuse` ever had a lawful record to consume.

## Assumption Reassessment (2026-04-24)

1. Live roadmap row 13 in `docs/scenario-roadmap.md` remains `In Progress`; the retained golden seam in `crates/worldwake-ai/tests/golden_survival_justice.rs` is still only `steal -> investigate` under the survival envelope.
2. The shared boundary under audit is the theft-case transport path from `worldwake-systems::investigate_actions::commit_investigate` into the unresolved `ViolationMemory` records that `worldwake-ai::candidate_generation::emit_accusation_candidates` consumes.
3. Candidate generation already supports accusation when it sees an unresolved `ViolationKind::SuspectedTheft` plus a suspect from either the violation record or matching `SocialObservationDetail::SuspectedTheft`; the missing transport was earlier than candidate synthesis.
4. Focused repro from the temporary row-13 golden probe showed `Merchant Sera` repeatedly committing `investigate` without ever reaching `accuse`, while final `ViolationMemory` contained only `EntityMissing` churn and no retained theft case.
5. Live code inspection found the blocking gate in `crates/worldwake-systems/src/investigate_actions.rs`: `commit_investigate` only promoted to `SuspectedTheft` when `belief.believed_owner_of(subject) == Some(actor)`.
6. That gate is too strong for lawful theft aftermath. After a real theft, ownership has already transferred to the thief, so the victim can still hold direct subjective theft evidence while no longer currently believing they own the stolen lot.
7. The truthful narrow fix is to let investigation promote a missing case into `SuspectedTheft` from existing subjective theft evidence even after owner transfer, while preserving the existing non-owner/no-evidence suppression path.
8. Reassessment exposed two broader row-13 contradictions that are not solved by this transport fix alone: the authored accusation scenario still produces too much unrelated `EntityMissing` churn to retain the intended theft case, and the search/report branch is separately stuck on repeated `ask_about_person` start failures. Those now belong to follow-up tickets.
9. Mismatch + correction: the original ticket overclaimed a full row-13 scenario landing. The live complete slice for this pass is the lower-layer production fix that preserves theft suspicion through lawful owner transfer; the scenario-level accusation/search proof remains follow-up work.

## Architecture Check

1. Preserving subjective theft evidence at the investigation seam is cleaner than hard-coding accusation helpers or scenario-only seeding because it fixes the real transport contradiction where lawful world state changed but the investigator's evidence still exists.
2. No compatibility shims were added. The same `SuspectedTheft` carrier remains canonical; the change only broadens the lawful conditions under which `investigate` writes it.

## Verification Layers

1. Investigation can promote a missing case into `SuspectedTheft` after owner transfer when matching subjective theft evidence exists -> focused unit coverage in `crates/worldwake-systems/src/investigate_actions.rs`
2. Owner-without-transfer and non-owner-without-subjective-evidence behaviors remain truthful -> focused unit coverage in `crates/worldwake-systems/src/investigate_actions.rs`
3. The existing retained row-13 scenario seam (`steal -> investigate` under survival pressure) still passes after the transport fix -> `crates/worldwake-ai/tests/golden_survival_justice.rs`

## What to Change

### 1. Widen the lawful promotion gate in `investigate`

When `investigate` resolves an `EntityMissing` case, allow it to reuse matching `SocialObservationDetail::SuspectedTheft` evidence for the same missing entity/place so the action can record `ViolationKind::SuspectedTheft` even after ownership has lawfully transferred away from the investigator.

### 2. Add focused regression coverage

Prove the new owner-transfer case and keep the old non-owner suppression path green at the `investigate_actions` unit boundary.

## Files to Touch

- `crates/worldwake-systems/src/investigate_actions.rs` (modify)
- `archive/tickets/S13SURJUS-001.md` (modify)

## Out of Scope

- Re-landing row 13 as a full accusation/punishment golden
- Fixing `survival-justice` search/report behavior
- Rewriting the roadmap row status to `Landed`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems investigate_actions::tests::investigate_promotes_missing_case_to_suspected_theft_when_subjective_theft_evidence_exists_after_owner_change -- --exact`
2. `cargo test -p worldwake-systems investigate_actions::tests::owner_investigating_missing_owned_entity_records_suspected_theft -- --exact`
3. `cargo test -p worldwake-systems investigate_actions::tests::non_owner_investigating_missing_entity_does_not_record_suspected_theft -- --exact`
4. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_theft_investigation_substrate -- --ignored --exact --test-threads=1`

### Invariants

1. A real theft case must remain promotable into `SuspectedTheft` after lawful owner transfer when the investigator still has matching subjective theft evidence.
2. Generic missing-item investigation must not start emitting theft accusations for actors who still lack ownership/evidence support.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/investigate_actions.rs` — prove post-transfer subjective theft evidence still promotes into `SuspectedTheft`
2. `crates/worldwake-ai/tests/golden_survival_justice.rs` — existing partial row-13 seam remains green after the production fix

### Commands

1. `cargo test -p worldwake-systems investigate_actions::tests::investigate_promotes_missing_case_to_suspected_theft_when_subjective_theft_evidence_exists_after_owner_change -- --exact`
2. `cargo test -p worldwake-systems investigate_actions::tests::owner_investigating_missing_owned_entity_records_suspected_theft -- --exact`
3. `cargo test -p worldwake-systems investigate_actions::tests::non_owner_investigating_missing_entity_does_not_record_suspected_theft -- --exact`
4. `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_theft_investigation_substrate -- --ignored --exact --test-threads=1`

## Outcome

Completed on 2026-04-24.

- `investigate` now promotes a missing case into `ViolationKind::SuspectedTheft` when the actor has matching subjective theft evidence, even if the stolen lot's lawful owner has already changed.
- Added focused unit coverage for that post-transfer evidence path.
- Kept the existing owner path and non-owner suppression path green.

## Deviations

- Reassessment narrowed this ticket from a full row-13 scenario landing to the lower-layer production transport fix that the scenario actually depended on.
- Follow-up owners now carry the still-false scenario seams:
  - `archive/tickets/S13SURJUS-002.md` for accusation/fine retained-case isolation
  - `tickets/S13SURJUS-003.md` for search/report stale `ask_about_person` blocking

## Verification Result

- Passed `cargo test -p worldwake-systems investigate_actions::tests::investigate_promotes_missing_case_to_suspected_theft_when_subjective_theft_evidence_exists_after_owner_change -- --exact`
- Passed `cargo test -p worldwake-systems investigate_actions::tests::owner_investigating_missing_owned_entity_records_suspected_theft -- --exact`
- Passed `cargo test -p worldwake-systems investigate_actions::tests::non_owner_investigating_missing_entity_does_not_record_suspected_theft -- --exact`
- Passed `cargo test -p worldwake-ai --test golden_survival_justice survival_justice_proves_theft_investigation_substrate -- --ignored --exact --test-threads=1`
