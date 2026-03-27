# CRIMECASEARCH-001: Reassess Proposed First-Class Institutional Crime Case Artifact

**Status**: NOT IMPLEMENTED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E17 (crime/theft/justice), E16c (institutional beliefs), `archive/tickets/completed/S32CRIMEMEGOLSUI-001.md`, `archive/tickets/completed/S32CRIMEMEGOLSUI-002.md`, `archive/tickets/completed/S32CRIMEMEGOLSUI-003.md`, `docs/FOUNDATIONS.md`

## Problem

This ticket originally proposed introducing a new first-class authoritative crime-case world artifact because it assumed the current crime/justice architecture still lacked a durable institutional case identity.

That assumption must be corrected before any code changes. The repository has moved since the ticket was drafted, and the current code already has a stable institutional lane for crime cases through the append-only crime register plus explicit accusation-to-verdict supersession.

## Assumption Reassessment (2026-03-27)

1. The exact shared abstraction boundary under audit is not "local evidence id vs missing world artifact." It is the authoritative handoff from local evidence into institutional record state:
   - accusation filing in `crates/worldwake-systems/src/justice_actions.rs::{start_accuse, commit_accuse}`
   - consulted institutional knowledge in `crates/worldwake-systems/src/consult_record_actions.rs`
   - planner consumption in `crates/worldwake-ai/src/candidate_generation.rs`
2. The ticket was correct that institutional claims still carry `violation_id` today:
   - `InstitutionalClaim::Accusation { accused, violation_id, theft, .. }`
   - `InstitutionalClaim::Verdict { accused, violation_id, .. }`
   - `InstitutionalBeliefKey::CrimeCase { accused, violation_id }`
   - `InstitutionalTellTopicKey::CrimeCase { accused, violation_id }`
   in `crates/worldwake-core/src/{institutional.rs,belief.rs}`.
3. The ticket was wrong to treat that residual `violation_id` coupling as proof that the architecture lacks a durable institutional case boundary. The live durable institutional lane already exists in authoritative state:
   - `RecordData.entries`
   - `InstitutionalRecordEntry.entry_id`
   - `InstitutionalRecordEntry.supersedes`
   - `InstitutionalKnowledgeSource::RecordConsultation { record, entry_id }`
   These are the stable institutional references that later punishment logic actually consumes.
4. Live justice logic already avoids the specific duplicate-filing failure mode the ticket presents. `start_accuse` / `commit_accuse` call `crime_case_already_recorded(record_data, accused, theft)`, and `RecordData::has_accusation_case_for` matches on concrete `TheftFacts`, not on `ViolationId`.
5. Live planner coverage already proves that same-theft convergence works even when local evidence ids differ. Current focused tests include:
   - `crates/worldwake-systems/src/justice_actions.rs::tests::duplicate_accusation_for_same_theft_rejects_at_start_even_with_different_violation_id`
   - `crates/worldwake-ai/src/candidate_generation.rs::tests::justice_candidates_suppress_duplicate_accusation_when_same_theft_is_already_recorded_under_different_violation_id`
   - `crates/worldwake-ai/tests/golden_emergent.rs::golden_dual_discovery_converges_without_double_accusation`
6. The original ticket also claimed S32 golden work was still pending. That is stale. The repository already contains:
   - `golden_exile_punishment_when_fine_is_not_locally_collectible`
   - `golden_witness_deterrence_suppresses_theft_candidate`
   - `golden_dual_discovery_converges_without_double_accusation`
   in `crates/worldwake-ai/tests/golden_emergent.rs`, and the corresponding S32 tickets are already archived under `archive/tickets/completed/`.
7. The foundations citation in the original ticket was partially stale. Current `docs/FOUNDATIONS.md` has 23 principles, not 24. The relevant live principles here are persistent identity and explicit transfer, concrete state over abstract scores, records/evidence as world state, and first-class social artifacts.
8. Information-path analysis, corrected: the same theft fact still has two lawful transport paths today.
   - Path A: agent-local `ViolationMemory` drives `GoalKind::Accuse`
   - Path B: institutional record consultation / tell traffic carries `InstitutionalClaim::{Accusation,Verdict}`
   The canonical institutional path is already the crime register lane. Local `ViolationId` remains an evidence-binding handle for subjective proof and candidate binding, not the authoritative punishment lookup boundary.
9. The intended invariant is therefore narrower than the original ticket claimed: one institutional proceeding for one concrete theft should flow through one crime-register lane, even if multiple local evidence records point at it. The live code already satisfies that invariant at the authoritative register boundary.
10. Adjacent contradiction exposed during reassessment: `violation_id` still leaks into institutional knowledge-grouping and tell-topic surfaces. That is a real cleanup opportunity, but it does not justify adding a second authoritative artifact that would mirror the register lane. If pursued later, the cleaner follow-up is to rekey those knowledge surfaces around the existing record/entry lane rather than introduce a new world object plus duplicate mutable state.
11. Mismatch + correction: this is not a production implementation ticket for a new `CrimeCase` entity/component anymore. The correct outcome is to retire that proposal and keep the current append-only register architecture.

## Architecture Check

1. Not adding a new crime-case entity is cleaner than duplicating the already-authoritative register lane with a second mutable artifact. Today the durable institutional procedure already lives where it should: in the append-only `CrimeRegister` plus explicit supersession.
2. A new `CrimeCase` world object would create parallel sources of truth:
   - record entries would still exist because the ledger is required
   - the new artifact would need to mirror accused, theft facts, status, and procedural history
   That would make the architecture less robust, not more robust.
3. The ideal long-term architecture, if residual `violation_id` leakage becomes painful, is to tighten knowledge-grouping around the existing record lane (`record` + `entry_id` / accusation-entry supersession lineage). That preserves one authoritative institutional source of truth instead of inventing another.
4. No backwards-compatibility aliasing, shim ids, or "case artifact plus mirrored violation id" path should be introduced.

## Verification Layers

1. duplicate accusation suppression at authoritative filing time -> `crates/worldwake-systems/src/justice_actions.rs` focused tests
2. planner-side suppression when the same theft is already institutionally recorded under another local evidence id -> `crates/worldwake-ai/src/candidate_generation.rs` focused tests
3. accusation-to-verdict institutional flow through one recorded lane -> `crates/worldwake-ai/tests/golden_emergent.rs` Scenarios 41 and 43
4. current architecture already provides the durable institutional boundary without a new artifact -> ticket reassessment against `institutional.rs`, `justice_actions.rs`, `consult_record_actions.rs`, and `candidate_generation.rs`
5. strongest lower-layer proof surface remains the register-entry / supersession boundary itself; no new traceability ticket is required for this reassessment

## What to Change

### 1. Do not introduce a new crime-case world artifact

Reject the proposed `CrimeCase` entity/component implementation for this ticket.

### 2. Archive this ticket as not implemented

Record that the original assumptions were stale and that the proposed architecture is not a net improvement over the current crime-register design.

## Files to Touch

- `tickets/CRIMECASEARCH-001.md` (modify)
- `archive/tickets/not-implemented/CRIMECASEARCH-001.md` (move/archive destination)

## Out of Scope

- adding a new `CrimeCaseId`, `CrimeCaseData`, or entity-backed crime-case artifact
- reworking `InstitutionalClaim::{Accusation,Verdict}` to duplicate register identity with a second authoritative object
- broad redesign of non-crime institutional artifacts
- follow-up cleanup of residual institutional `violation_id` grouping surfaces

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

1. The ticket must describe the current architecture accurately: the authoritative institutional case lane is the crime register entry/supersession lineage, not a missing sidecar object.
2. Local `ViolationId` may remain on subjective evidence-binding surfaces without forcing a new authoritative artifact.
3. No second institutional source of truth is introduced alongside the append-only crime register.

## Test Plan

### New/Modified Tests

1. None. This is a reassessment and archival decision; no production or test code changes are warranted.

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
- What changed:
  - corrected the ticket assumptions to match the live crime/justice architecture, current focused tests, and already-delivered S32 golden coverage
  - removed the proposed implementation scope for a new first-class crime-case artifact
  - archived the ticket as not implemented because that extra artifact would duplicate the existing authoritative crime-register lane
- Deviations from original plan:
  - the original plan proposed new core/system/AI architecture and test rewrites
  - after reassessment, the deeper architectural claim was found to be overstated: the repository already has a stable institutional case lane through register entries and supersession
  - the remaining issue is only residual `violation_id` leakage in knowledge-grouping surfaces, which is a separate follow-up question and should be solved by tightening the existing record lane rather than by creating a new world object
- Verification results:
  - `cargo test -p worldwake-systems justice_actions::tests::duplicate_accusation_for_same_theft_rejects_at_start_even_with_different_violation_id -- --exact` passed
  - `cargo test -p worldwake-ai candidate_generation::tests::justice_candidates_suppress_duplicate_accusation_when_same_theft_is_already_recorded_under_different_violation_id -- --exact` passed
  - `cargo test -p worldwake-ai --test golden_emergent golden_exile_punishment_when_fine_is_not_locally_collectible -- --exact` passed
  - `cargo test -p worldwake-ai --test golden_emergent golden_witness_deterrence_suppresses_theft_candidate -- --exact` passed
  - `cargo test -p worldwake-ai --test golden_emergent golden_dual_discovery_converges_without_double_accusation -- --exact` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
