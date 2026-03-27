# CRIMECASEARCH-002: Remove Residual Institutional `ViolationId` Coupling From Knowledge, Planner, And Trace Surfaces

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-core`, `worldwake-systems`, `worldwake-ai`, docs/tests
**Deps**: `tickets/CRIMECASEARCH-001.md`, E16c (institutional beliefs), E17 (crime/theft/justice), `docs/FOUNDATIONS.md`, `docs/planner-contracts.md`

## Problem

Once crime cases are first-class institutional artifacts, the rest of the knowledge pipeline must stop speaking the old language of `(accused, violation_id)` for institutional case identity. If tell-topic grouping, institutional belief keys, planner traces, or golden assertions keep using `violation_id` as the durable case lane, the architecture stays split-brained: authoritative state says the case is first-class, but transport and planner surfaces still act as if local evidence bookkeeping is the real identity.

## Assumption Reassessment (2026-03-27)

1. The exact shared abstraction boundary under audit is institutional knowledge transport and planner visibility for crime cases:
   `crates/worldwake-core/src/belief.rs`,
   `crates/worldwake-systems/src/{tell_actions.rs,consult_record_actions.rs}`,
   `crates/worldwake-ai/src/{candidate_generation.rs,decision_trace.rs,goal_model.rs}`,
   and the related goldens / conformance tests.
2. Live tell and consultation grouping still use `InstitutionalTellTopicKey::CrimeCase { accused, violation_id }` and `InstitutionalBeliefKey::CrimeCase { accused, violation_id }`.
3. Live planner and trace surfaces still print or compare institutional crime lanes using `violation_id` inside institutional claims and summaries. That is acceptable only until a first-class case artifact exists. After that point, keeping those surfaces on `violation_id` would be a stale parallel identity path.
4. This is an information-path refactor. Current code has multiple lawful transport paths for the same fact:
   local evidence uses `ViolationId`,
   institutional knowledge uses crime-record claims,
   tell/consult grouping uses institutional “crime case” topics.
   After this ticket, the canonical institutional transport path should be keyed by the first-class crime case artifact from `CRIMECASEARCH-001`, while local evidence surfaces retain `ViolationId` only where they bind subjective evidence.
5. The intended invariant is:
   planner-visible institutional case knowledge, tell-topic grouping, consultation grouping, and trace summaries all identify the same institutional case by the same durable artifact.
6. This ticket should not broaden into a full planner redesign. `GoalKind::Accuse` may still use `violation_id` to bind local evidence if that remains the best local input surface. The change here is to remove residual institutional `violation_id` identity from transport, topic grouping, and traceability.
7. If traces prove the behavior but not enough provenance, this ticket should prefer strengthening planner and institutional trace surfaces rather than adding ad hoc debug output. `docs/planner-contracts.md` already establishes that planner-facing contracts should be explicit, not ticket lore.
8. Adjacent contradiction likely to appear during implementation: existing tests and helper builders may hardcode `InstitutionalBeliefKey::CrimeCase { accused, violation_id }`. Those are required consequences of this refactor, not separate bugs.
9. No backwards-compatibility lane should remain after completion. There should not be one “new case_id topic” plus one “old violation_id topic” for the same institutional case.

## Architecture Check

1. This refactor is necessary to finish the architecture that `CRIMECASEARCH-001` starts. Without it, the codebase would still maintain two nominal institutional identities for one case, which violates the “no workaround architecture” rule in `docs/FOUNDATIONS.md`.
2. The clean architecture is:
   local evidence may differ and expire,
   but once institutionalized, one case artifact becomes the canonical carrier of social consequence.
   Tell, consultation, planner visibility, and traces should all align to that artifact.
3. This is more robust than leaving mixed identity surfaces in place because future systems such as appeals, reopening, or multi-step adjudication will need one shared case lane across transport, records, and AI reasoning.
4. No backwards-compatibility aliasing or shims should be introduced.

## Verification Layers

1. Tell and consult grouping use institutional case artifact identity, not institutional `violation_id` identity -> focused `worldwake-core` / `worldwake-systems` tests
2. Planner-visible institutional case reads still surface lawful punishment and accusation behavior -> focused `worldwake-ai` candidate-generation / conformance tests
3. Decision traces and helper summaries identify crime cases by the durable institutional case artifact -> focused `worldwake-ai` trace tests
4. Goldens that involve crime information transport remain legible and deterministic -> Scenarios 38, 41, and 43
5. Existing local investigation and accuse payload binding still work without omniscient shortcuts -> focused `worldwake-systems` and planner conformance tests

## What to Change

### 1. Rekey institutional knowledge grouping to the case artifact

Update:

- `InstitutionalBeliefKey::CrimeCase`
- `InstitutionalTellTopicKey::CrimeCase`
- tell / consult relay grouping helpers

so that the canonical institutional grouping key is the crime-case artifact from `CRIMECASEARCH-001`, not `(accused, violation_id)`.

### 2. Update planner-facing crime surfaces

Update planner and AI-facing crime knowledge surfaces so any planner-visible institutional case logic reads and traces the case artifact identity consistently.

This includes candidate-generation helpers, planner conformance tests, and decision-trace formatting that currently expose institutional case lanes through `violation_id` wording.

### 3. Remove residual stale wording and assertions

Update focused tests, goldens, and relevant docs or helper comments so they no longer imply that institutional case identity is the same as local violation identity.

### 4. Keep local evidence binding precise

Where local evidence still lawfully requires `ViolationId` — for example `InvestigateViolation` and the accuse action payload binding to subjective evidence — keep that boundary explicit and documented rather than smearing institutional and local identity back together.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify)
- `crates/worldwake-core/src/institutional.rs` (modify if key types live there after ticket 001)
- `crates/worldwake-systems/src/tell_actions.rs` (modify)
- `crates/worldwake-systems/src/consult_record_actions.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify if planner payload/provenance wording requires update)
- `crates/worldwake-ai/tests/planner_conformance.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `tickets/CRIMECASEARCH-002.md` (new)

## Out of Scope

- Creating the first-class case artifact itself; that belongs to `CRIMECASEARCH-001`
- Redesigning non-crime tell-topic or institutional-belief grouping
- New punishment kinds, appeal logic, prison logic, or confiscation mechanics
- Retaining dual institutional identity paths for compatibility

## Acceptance Criteria

### Tests That Must Pass

1. Focused tests prove crime-case tell/consult grouping uses the case artifact identity
2. Focused tests prove planner / trace surfaces no longer describe institutional crime lanes through `violation_id`
3. `cargo test -p worldwake-core`
4. `cargo test -p worldwake-systems`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. There is exactly one canonical institutional identity lane for a crime case after institutionalization.
2. Local evidence identity and institutional case identity remain distinct and explicitly named.
3. Tell, consult, planner, and trace surfaces all align to the same institutional case artifact.
4. No compatibility alias path keeps institutional `violation_id` identity alive after the refactor.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — update institutional topic-grouping tests for crime cases
2. `crates/worldwake-systems/src/tell_actions.rs` and `crates/worldwake-systems/src/consult_record_actions.rs` — add grouping / relay tests keyed by institutional case artifact
3. `crates/worldwake-ai/src/decision_trace.rs` — update trace-format tests for crime-case identity output
4. `crates/worldwake-ai/tests/planner_conformance.rs` — update accuse / punish conformance coverage to the case artifact surface
5. `crates/worldwake-ai/tests/golden_emergent.rs` — keep crime goldens asserting the canonical institutional case path

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-systems`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
