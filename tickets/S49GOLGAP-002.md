# S49GOLGAP-002: Vacancy-notice political uptake golden

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S45UNISOCART-006 (S45 golden tests complete)

## Problem

Scenario 73 (`golden_offices.rs:1265`) proves remote record consultation as the prerequisite for political action, and Scenario 46 (`golden_emergent.rs:1580`) proves Tell-based political knowledge propagation. But no golden test proves that an office-vacancy notice — as a social artifact perceived locally — can unlock political action without record consultation or Tell relay. This is a materially different information path that the S45 artifact substrate enables.

## Assumption Reassessment (2026-04-05)

1. Scenario 73 exists in `crates/worldwake-ai/tests/golden_offices.rs:1265` — proves remote record consultation for political action. Confirmed.
2. Scenario 46 exists in `crates/worldwake-ai/tests/golden_emergent.rs:1580` — proves Tell-based political knowledge. Confirmed.
3. Scenario 107 exists in `crates/worldwake-ai/tests/golden_integration.rs:6025` — proves threat-warning notice route diversion, not political uptake. Confirmed.
4. `NoticeTopic::OfficeVacancy { office: EntityId }` exists in `crates/worldwake-core/src/social_artifact.rs` — confirmed.
5. `InstitutionalClaim::OfficeHolder` exists in `crates/worldwake-core/src/institutional.rs:18-60` — used for vacancy belief internalization (`holder: None`). Confirmed.
6. `GoalKind::ClaimOffice { office: EntityId }` exists in `crates/worldwake-core/src/goal.rs:72-74` — confirmed.
7. `GoalKind::SupportCandidateForOffice { office, candidate }` exists in `crates/worldwake-core/src/goal.rs:75-78` — confirmed.
8. `consult_record` action exists in `crates/worldwake-systems/src/consult_record_actions.rs:36` — confirmed. This scenario must prove political action happens WITHOUT this action.
9. `BelievedArtifactState` exists in `crates/worldwake-core/src/belief.rs:712-726` — perception integration populates this for notice artifacts. Confirmed.
10. `PerceptionProfile` required on agents that need to observe notice artifacts.

## Architecture Check

1. This is a test-only ticket. No production code changes. The golden test proves that the S45 notice-artifact perception path feeds the existing institutional-belief and political-action lanes (Principle 26) without requiring record consultation as a prerequisite.
2. The test must ensure the claimant has NO pre-seeded office-holder belief and NO consulted office register — political action is unlocked solely through artifact perception.
3. No backward-compatibility shims.

## Verification Layers

1. Notice perception → belief store assertion (`believed_artifact` with `NoticeTopic::OfficeVacancy`)
2. Vacancy belief internalized through artifact path → institutional belief store assertion (`InstitutionalClaim::OfficeHolder { holder: None }` for the office)
3. Political candidate emitted without `consult_record` → decision trace (ClaimOffice or SupportCandidateForOffice appears) + action trace absence (no consult_record action started)
4. Local political action starts or commits → action trace (political action committed through normal politics surface)
5. Cross-layer: artifact perception (systems) → institutional belief (core) → candidate generation (AI) → political action (systems)

## What to Change

### 1. Add golden scenario: vacancy-notice political uptake

In `crates/worldwake-ai/tests/golden_offices.rs`:

**Setup**:
- 1 place: TownSquare.
- 1 vacant office at TownSquare with a succession law that gives one clean local political action path (e.g., force claim with no rival).
- 1 AI claimant at TownSquare with: PerceptionProfile, UtilityProfile (social_weight or enterprise_weight sufficient to generate political goals), ReasoningProfile. NO pre-seeded office-holder belief. NO pre-consulted office register.
- 1 human issuer who posts `NoticeTopic::OfficeVacancy { office }` artifact at TownSquare.

**Execution**: Tick simulation with bounded limit until claimant takes political action.

**Assertions**:
- Claimant perceived notice artifact at TownSquare (`believed_artifact.kind == Notice`, `believed_artifact.notice_topic == OfficeVacancy`).
- Claimant internalized vacancy belief: `InstitutionalClaim::OfficeHolder` with `holder: None` in institutional beliefs for the office entity.
- Claimant generated political goal (ClaimOffice or SupportCandidateForOffice) — decision trace.
- No `consult_record` action started by claimant — action trace absence.
- Political action committed through normal politics surface — action trace.
- Plan proceeds directly to local political action, not via detour to record consultation.

### 2. Add deterministic replay companion

Same scenario with identical seed — assert identical outcome.

## Files to Touch

- `crates/worldwake-ai/tests/golden_offices.rs` (modify)

## Out of Scope

- Record consultation tests (already covered by Scenario 73)
- Tell-based political propagation (already covered by Scenario 46)
- Threat-warning notice tests (already covered by Scenario 107)
- Multi-candidate succession scenarios
- Production code changes

## Acceptance Criteria

### Tests That Must Pass

1. Vacancy-notice golden: artifact perception → internalized vacancy belief → political action without consult_record
2. Deterministic replay companion produces identical outcome
3. No consult_record action in action trace for the claimant
4. Existing suite: `cargo test --workspace`

### Invariants

1. Claimant has no pre-seeded vacancy knowledge — all political knowledge from artifact perception (Principle 7)
2. Notice artifacts feed existing institutional-belief and political-action lanes — no special-case planner hook (Principle 26)
3. Notice persists as world state with real downstream consequences (Principle 18)
4. Deterministic: same seed → same outcome

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_offices.rs` — Vacancy-notice political uptake golden scenario + replay companion

### Commands

1. `cargo test -p worldwake-ai -- golden_offices`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
