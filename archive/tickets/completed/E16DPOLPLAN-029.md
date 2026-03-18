# E16DPOLPLAN-029: Focused AI pipeline regression — Force-law offices never enter political AI planning

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — focused AI/planning regression coverage only
**Deps**: E16DPOLPLAN-015, E16DPOLPLAN-007

## Problem

The original ticket assumption is stale. Current production behavior is stronger than "no support-based political planning": Force-law offices are excluded from political candidate generation entirely, so they never enter the political AI pipeline at all. What is still missing is a focused `agent_tick` decision-trace regression that proves the runtime AI pipeline preserves that invariant when the actor has direct office knowledge and the full action registry is available.

Today that guarantee is spread across:
- candidate-generation focused coverage in `crates/worldwake-ai/src/candidate_generation.rs`
- authoritative action validation in `crates/worldwake-systems/src/office_actions.rs`
- golden E2E coverage in `crates/worldwake-ai/tests/golden_offices.rs`

What is still missing is a narrow decision-trace assertion at the runtime AI pipeline layer.

## Assumption Reassessment (2026-03-18)

1. `emit_political_candidates` in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) skips any office whose `OfficeData.succession_law != SuccessionLaw::Support`, so Force-law offices emit neither `GoalKind::ClaimOffice` nor `GoalKind::SupportCandidateForOffice` today — confirmed at [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs).
2. Focused candidate-generation coverage already exists in `candidate_generation::tests::political_candidates_skip_force_law_offices` in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs). The ticket must not claim that this layer is currently untested.
3. `AgentTickDriver` tracing is capable of proving candidate-generation absence and plan-search absence through `DecisionOutcome::Planning`, `planning.candidates.generated`, and `planning.planning.attempts` — confirmed in existing trace tests in [`crates/worldwake-ai/src/agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs).
4. Existing golden Scenario 18 in [`crates/worldwake-ai/tests/golden_offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs) already proves the end-to-end Force-law outcome and the absence of committed `declare_support` actions, but it is intentionally broader and slower than a focused runtime AI regression.
5. `declare_support` authoritative rejection for Force-law offices already exists in `validate_declare_support_context_in_world` in [`crates/worldwake-systems/src/office_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs). This ticket should not duplicate that system-layer coverage.

## Architecture Check

1. The current architecture is cleaner than introducing any Force-law alias of the support-based office path. `SuccessionLaw::Force` is currently system-resolved, while `ClaimOffice` and `DeclareSupport` belong to support-law politics only.
2. A focused `agent_tick` decision-trace regression is still beneficial because it verifies the runtime integration boundary: belief view -> candidate generation -> plan search. That catches regressions that the pure candidate-generation unit test cannot, without paying the cost of a golden scenario.
3. The test should assert the semantic invariant at the strongest relevant layer for this ticket: decision traces for candidate absence and planning-attempt absence, not only missing action commits.
4. If the design later wants agents to actively contest Force-law offices, that should be modeled as an explicit Force-law goal/action path, not by reusing `ClaimOffice` or `DeclareSupport`. That broader architectural change is out of scope here.

## What to Change

### 1. Add a focused `agent_tick` regression for Force-law offices

- Add a new test under `crates/worldwake-ai/src/agent_tick.rs` using the existing local harness and decision tracing.
- Setup:
  - AI-controlled, sated, politically ambitious actor
  - visible vacant office with `SuccessionLaw::Force`
  - visible rival candidate with positive loyalty from the actor
  - direct belief about the office
- Assertions:
  - `DecisionOutcome::Planning` is produced
  - `planning.candidates.generated` does not contain `GoalKind::ClaimOffice { office }`
  - `planning.candidates.generated` does not contain `GoalKind::SupportCandidateForOffice { .. }`
  - `planning.planning.attempts` does not contain attempts whose `goal.kind` is `ClaimOffice` or `SupportCandidateForOffice`

### 2. Keep the test explicitly at the AI/planning layer

- Do not assert office installation, event-log outcomes, or action commits here.
- If an action-trace or golden assertion is needed, that belongs to existing coverage, not this ticket.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify)

## Out of Scope

- Golden/E2E office scenario changes
- Changes to `candidate_generation.rs`
- Changes to `office_actions.rs`
- Changes to Force-law succession behavior
- E16b force-control architecture

## Acceptance Criteria

### Tests That Must Pass

1. New `agent_tick` regression proves a Force-law office never enters political candidate generation or plan search in the runtime AI pipeline.
2. Existing targeted trace tests in `crates/worldwake-ai/src/agent_tick.rs` still pass.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Force-law offices do not produce `ClaimOffice` or `SupportCandidateForOffice` candidates in the AI pipeline.
2. The planning layer does not attempt political planning for Force-law offices when the law gate excludes those goals at candidate generation.
3. The test proves candidate-generation / planning absence directly from decision traces, not indirectly from missing world mutations.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — new Force-law runtime decision-trace regression proving candidate absence and no political planning attempts.

### Commands

1. `cargo test -p worldwake-ai agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-18
- **What actually changed**:
  - Reassessed the ticket against current code and corrected the stale assumption that Force-law offices only skip support-based subpaths. Current architecture skips political AI candidate generation for Force-law offices entirely.
  - Added a focused runtime decision-trace regression in `crates/worldwake-ai/src/agent_tick.rs` proving that an informed actor with a visible Force-law office and a visible rival still generates no `ClaimOffice` or `SupportCandidateForOffice` candidates and produces no political plan-search attempts for that office.
- **Deviations from original plan**:
  - No production behavior changes were needed.
  - The ticket now explicitly acknowledges existing focused candidate-generation coverage in `candidate_generation::tests::political_candidates_skip_force_law_offices`; the new work is the runtime `agent_tick` integration trace, not a first test for the invariant.
  - The architectural conclusion remains to keep Force-law succession system-driven. If future design wants active Force-law contest behavior, it should be introduced as a distinct goal/action path rather than reusing support-law politics.
- **Verification results**:
  - `cargo test -p worldwake-ai agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace` ✅
