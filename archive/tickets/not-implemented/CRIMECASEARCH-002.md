# CRIMECASEARCH-002: Retire Residual Institutional `ViolationId` Cleanup Until A Real Record-Lane Key Exists

**Status**: NOT IMPLEMENTED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: `archive/tickets/not-implemented/CRIMECASEARCH-001.md`, `archive/tickets/completed/S32CRIMEMEGOLSUI-001.md`, `archive/tickets/completed/S32CRIMEMEGOLSUI-002.md`, `archive/tickets/completed/S32CRIMEMEGOLSUI-003.md`, E16c (institutional beliefs), E17 (crime/theft/justice), `docs/FOUNDATIONS.md`, `docs/planner-contracts.md`, `archive/specs/S32-crime-emergence-golden-suites.md`

## Problem

This ticket originally assumed the repository still needed a production cleanup that would replace residual institutional `ViolationId` grouping with a stronger canonical institutional case key.

That assumption must be corrected before any implementation work. The live crime/justice architecture already has one authoritative institutional procedure lane through `CrimeRegister` entries plus accusation-to-verdict supersession, but the repository does not yet expose a narrower replacement key that is clearly better than the current `(accused, violation_id)` grouping surfaces.

## Assumption Reassessment (2026-03-27)

1. The exact shared abstraction boundary under audit is institutional crime knowledge grouping and transport, not authoritative punishment identity:
   - grouping keys in `crates/worldwake-core/src/{institutional.rs,belief.rs}`
   - record-consult projection in `crates/worldwake-systems/src/consult_record_actions.rs`
   - tell-memory/topic grouping in `crates/worldwake-systems/src/tell_actions.rs`
   - planner consumption in `crates/worldwake-ai/src/candidate_generation.rs`
2. The live institutional grouping surfaces still use `ViolationId` exactly where this ticket claims:
   - `InstitutionalBeliefKey::CrimeCase { accused, violation_id }` in [institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs)
   - `InstitutionalTellTopicKey::CrimeCase { accused, violation_id }` and `institutional_claim_same_memory_lane()` in [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs)
   - consult/tell grouping helpers in [consult_record_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/consult_record_actions.rs) and [tell_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/tell_actions.rs)
3. The ticket’s larger architectural claim is stale. The live authoritative punishment lane is already keyed by record consultation and accusation entry, not by those grouping enums:
   - `InstitutionalKnowledgeSource::RecordConsultation { record, entry_id }`
   - `GoalKind::PunishAccused { office, accused, accusation_entry, punishment }`
   - `active_accusation_case(record_data, accusation_entry)` in [justice_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs)
4. The intended invariant, corrected: one institutional proceeding for one concrete theft should flow through one `CrimeRegister` lineage even if multiple local `ViolationMemory` records point at it. The live runtime already enforces that at the authoritative register boundary through `RecordData::has_accusation_case_for(accused, theft)` in [institutional.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/institutional.rs) and the duplicate-filing checks in [justice_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs).
5. The live `GoalKind` under audit is `GoalKind::PunishAccused`. Its current prerequisite surface is consulted `InstitutionalClaim::Accusation` knowledge whose source must be `RecordConsultation`, and its terminal runtime binding is `accusation_entry`, not an institutional `ViolationId`, in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs).
6. Information-path analysis, corrected:
   - Path A: local subjective evidence still uses `ViolationId` through `ViolationMemory`, `GoalKind::Accuse`, and accuse payload binding.
   - Path B: institutionalized knowledge flows through `CrimeRegister` entries, `RecordConsultation`, and accusation-entry-based punishment.
   The canonical institutional path is already Path B.
7. The original follow-up plan is not currently implementable as a clean architectural improvement. The record lane has stable entries and supersession, but the repository does not yet define a first-class "institutional crime lineage root" helper or type that would let tell/grouping surfaces replace `(accused, violation_id)` without either:
   - keying on a single consulted `entry_id`, which is too narrow because verdict entries supersede accusation entries, or
   - introducing a new sidecar case identity lane, which would duplicate the current authoritative register path.
8. Existing tests already prove the most important behavior at the stronger lower layers:
   - `justice_actions::tests::duplicate_accusation_for_same_theft_rejects_at_start_even_with_different_violation_id`
   - `candidate_generation::tests::justice_candidates_suppress_duplicate_accusation_when_same_theft_is_already_recorded_under_different_violation_id`
   - `golden_dual_discovery_converges_without_double_accusation`
   - `golden_exile_punishment_when_fine_is_not_locally_collectible`
   - `golden_witness_deterrence_suppresses_theft_candidate`
9. `archive/specs/S32-crime-emergence-golden-suites.md` is now reference material, not missing implementation scope. The S32 crime goldens it describes already exist in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) and their delivery tickets are already archived.
10. Adjacent contradiction exposed during reassessment: residual institutional `violation_id` grouping is real, but solving it cleanly requires first defining a true record-lineage grouping abstraction. That is future cleanup, not a justified in-scope refactor today.
11. Mismatch + correction: this ticket should not proceed with production changes. The correct outcome is to retire the cleanup until the codebase has a real record-lineage key that is stronger than `(accused, violation_id)` and does not create a second institutional identity lane.

## Architecture Check

1. Retiring this ticket is cleaner than forcing a premature rekey. The current architecture already has one authoritative institutional lane in the append-only `CrimeRegister`; adding a second identity path or half-canonical helper now would make the system less robust.
2. The proposed cleanup is not more beneficial than the current architecture yet because the missing piece is not "rename the key." The missing piece is a true canonical lineage abstraction over accusation entry plus supersession. Without that, replacing `violation_id` would either weaken grouping semantics or introduce duplication.
3. The ideal long-term architecture, if this cleanup becomes worth doing, is still a single institutional identity lane derived from the existing record lineage. That should be introduced only when there is a concrete shared abstraction, likely in `worldwake-core`, that multiple consumers need and that can represent accusation-to-verdict continuity without aliasing.
4. No backwards-compatibility aliases, dual keys, or sidecar case objects should be introduced.

## Verification Layers

1. authoritative duplicate suppression for same-theft accusations -> focused `worldwake-systems` test on `start_accuse` / `commit_accuse`
2. planner-side suppression when institutional knowledge already covers the theft under a different local `ViolationId` -> focused `worldwake-ai` candidate-generation test
3. accusation-to-verdict institutional flow through one consulted record lane -> crime goldens in `golden_emergent.rs`
4. residual `violation_id` coupling is limited to knowledge grouping, not authoritative punishment binding -> reassessment against `institutional.rs`, `belief.rs`, `consult_record_actions.rs`, `tell_actions.rs`, `candidate_generation.rs`, and `justice_actions.rs`
5. strongest proof surface remains the existing record-entry / supersession boundary; broadening production code without a stronger lower-layer key would be architectural drift, not progress
6. no new verification layer mapping is needed because this ticket ends in retirement rather than implementation

## What to Change

### 1. Correct the ticket assumptions and scope

Update this ticket so it accurately reflects the live register-based architecture, current crime goldens, and the fact that the proposed refactor does not yet have a clean canonical replacement key.

### 2. Do not change production code

Do not rekey institutional crime grouping to a new helper, alias, or artifact in this ticket. Retire the cleanup until a real record-lineage abstraction exists and clearly improves multiple consumers.

### 3. Archive the ticket as not implemented

Record that the remaining architectural opportunity is future cleanup around a canonical record-lineage key, not a safe or beneficial implementation under the current model.

## Files to Touch

- `tickets/CRIMECASEARCH-002.md` (modify)
- `archive/tickets/not-implemented/CRIMECASEARCH-002.md` (move/archive destination)

## Out of Scope

- changing `InstitutionalClaim::{Accusation, Verdict}` payload structure
- introducing a new `CrimeCaseId`, world artifact, or parallel institutional identity lane
- replacing local `ViolationId` evidence binding on `ViolationMemory`, `GoalKind::Accuse`, or accuse payloads
- redesigning non-crime institutional grouping
- adding a dual-key transition path that preserves both old and new institutional case grouping in parallel

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems justice_actions::tests::duplicate_accusation_for_same_theft_rejects_at_start_even_with_different_violation_id -- --exact`
2. `cargo test -p worldwake-ai candidate_generation::tests::justice_candidates_suppress_duplicate_accusation_when_same_theft_is_already_recorded_under_different_violation_id -- --exact`
3. `cargo test -p worldwake-ai --test golden_emergent golden_exile_punishment_when_fine_is_not_locally_collectible -- --exact`
4. `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft_candidate -- --exact`
5. `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges_without_double_accusation -- --exact`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. The authoritative institutional crime lane remains the `CrimeRegister` entry/supersession lineage.
2. Local evidence identity and institutional identity remain distinct; local `ViolationId` does not become a fake canonical institutional id.
3. This ticket does not introduce a second institutional source of truth or an alias path.

## Test Plan

### New/Modified Tests

1. `None — documentation-only reassessment and archival decision; verification relies on existing focused and golden coverage named above.`

### Commands

1. `cargo test -p worldwake-systems justice_actions::tests::duplicate_accusation_for_same_theft_rejects_at_start_even_with_different_violation_id -- --exact`
2. `cargo test -p worldwake-ai candidate_generation::tests::justice_candidates_suppress_duplicate_accusation_when_same_theft_is_already_recorded_under_different_violation_id -- --exact`
3. `cargo test -p worldwake-ai --test golden_emergent golden_exile_punishment_when_fine_is_not_locally_collectible -- --exact`
4. `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft_candidate -- --exact`
5. `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges_without_double_accusation -- --exact`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-27
- What actually changed:
  - corrected the ticket to match the live record-entry / supersession architecture
  - narrowed the issue from "missing institutional crime case identity" to "residual grouping still uses `violation_id`"
  - retired the proposed production cleanup because the repository does not yet expose a stronger canonical record-lineage key
- Deviations from original plan:
  - no production or test code changes were made
  - the reassessment found that the proposed rekey is not cleaner than the current architecture yet
  - the ideal future cleanup would derive a single institutional lineage key from the existing record lane rather than invent a new artifact
- Verification results:
  - `cargo test -p worldwake-systems justice_actions::tests::duplicate_accusation_for_same_theft_rejects_at_start_even_with_different_violation_id -- --exact` passed
  - `cargo test -p worldwake-ai candidate_generation::tests::justice_candidates_suppress_duplicate_accusation_when_same_theft_is_already_recorded_under_different_violation_id -- --exact` passed
  - `cargo test -p worldwake-ai --test golden_emergent golden_exile_punishment_when_fine_is_not_locally_collectible -- --exact` passed
  - `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft_candidate -- --exact` passed
  - `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges_without_double_accusation -- --exact` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
