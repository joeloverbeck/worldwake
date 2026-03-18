# E16DPOLPLAN-029: Focused AI pipeline regression — Force-law offices never enter support-based political planning

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — focused AI/planning regression coverage only
**Deps**: E16DPOLPLAN-015, E16DPOLPLAN-007

## Problem

Force-law office behavior is now protected by candidate-generation gating and authoritative `declare_support` validation, but there is no focused AI pipeline regression that proves an informed, ambitious agent never enters the support-based political planning path for a `SuccessionLaw::Force` office.

Today that guarantee is spread across:
- candidate-generation focused coverage in `crates/worldwake-ai/src/candidate_generation.rs`
- authoritative action validation in `crates/worldwake-systems/src/office_actions.rs`
- golden E2E coverage in `crates/worldwake-ai/tests/golden_offices.rs`

What is still missing is a narrow decision-trace assertion at the AI pipeline layer.

## Assumption Reassessment (2026-03-18)

1. `emit_political_candidates` in `crates/worldwake-ai/src/candidate_generation.rs` now skips offices whose `OfficeData.succession_law != SuccessionLaw::Support` — confirmed from the production branch added in E16DPOLPLAN-015.
2. `AgentTickDriver` tracing is already capable of proving candidate generation and plan-search absence via `DecisionOutcome::Planning`, `planning.candidates.generated`, and `planning.planning.attempts` — confirmed in `crates/worldwake-ai/src/agent_tick.rs` (`trace_planning_outcome_for_hungry_agent`) and in existing decision-trace use from `crates/worldwake-ai/tests/golden_offices.rs`.
3. Existing golden Scenario 18 in `crates/worldwake-ai/tests/golden_offices.rs` proves the end-to-end Force-law outcome, but it is intentionally broader and slower than a focused AI regression — corrected scope: this ticket is about the AI pipeline only, not the full authoritative outcome.
4. `declare_support` authoritative rejection for Force-law offices already exists in `crates/worldwake-systems/src/office_actions.rs` (`validate_declare_support_context_in_world`) — confirmed. This ticket should not duplicate that system-layer coverage.

## Architecture Check

1. A focused decision-trace regression is cleaner than relying only on a golden scenario because it localizes the invariant to the AI pipeline: no Force-law political candidate generation and no plan-search attempt for support-based office goals.
2. This avoids backwards-compatibility shims or alternate code paths. It only strengthens coverage around the current architecture.
3. The test should assert the semantic invariant at the strongest relevant layer: decision traces for candidate absence and planning absence, not only missing events.

## What to Change

### 1. Add a focused `agent_tick` regression for Force-law offices

- Add a new test under `crates/worldwake-ai/src/agent_tick.rs` using the existing local harness and decision tracing.
- Setup:
  - AI-controlled, sated, politically ambitious actor
  - visible vacant office with `SuccessionLaw::Force`
  - direct belief about the office
- Assertions:
  - `DecisionOutcome::Planning` is produced
  - `planning.candidates.generated` does not contain `GoalKind::ClaimOffice { office }`
  - `planning.candidates.generated` does not contain `GoalKind::SupportCandidateForOffice { .. }`
  - `planning.planning.attempts` does not contain office-political candidates or `declare_support`-driven attempts

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

1. New `agent_tick` regression proves a Force-law office never enters support-based political candidate generation or planning.
2. Existing targeted trace tests in `crates/worldwake-ai/src/agent_tick.rs` still pass.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Force-law offices do not produce `ClaimOffice` or `SupportCandidateForOffice` candidates in the AI pipeline.
2. The planning layer does not attempt support-based office planning when the law gate excludes those goals.
3. The test proves candidate-generation / planning absence directly from decision traces, not indirectly from missing world mutations.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — new Force-law decision-trace regression proving candidate absence and no planning attempts.

### Commands

1. `cargo test -p worldwake-ai trace_force_law_office_skips_support_based_political_planning`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace`
