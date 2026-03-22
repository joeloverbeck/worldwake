# S15STAFAIEME-009: Expose Office Availability Phases In Political Trace

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` political trace model, `worldwake-systems` office succession trace emission, targeted political trace assertions
**Deps**: S15STAFAIEME-003, E16

## Problem

The current political trace records per-office succession evaluation outcomes, but it still makes a key support-law debugging question too indirect:

When exactly is an office still claimable versus already closed to new claims?

Today, reviewers must infer that from a combination of:

- `vacancy_since_before`
- `holder_before`
- support declarations
- and the specific `OfficeSuccessionOutcome`

That is enough for the engine, but it is weaker than it should be for architecture-facing traceability. Support-law scenarios especially need a direct trace surface for states like:

- visibly vacant and claimable
- vacant with declarations pending while succession is unresolved
- no longer visibly vacant because a holder exists
- closed by tie/reset or no-eligible-declaration reset

Without that direct surface, political goldens and focused tests still have to reason from code instead of from an explicit authoritative trace fact.

## Assumption Reassessment (2026-03-20)

1. The current political trace model in [politics_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) records `OfficeSuccessionTrace { jurisdiction, succession_law, holder_before, vacancy_since_before, outcome, support_declarations, force_candidates }`. It exposes branch outcomes, but it does not expose a first-class authoritative phase that tells a reader whether the office is still openly claimable, waiting on closure, already closed, or reopened after a reset.
2. The authoritative closure boundary for this ticket lives in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs): `evaluate_office_succession()`, `resolve_support_succession()`, and `resolve_force_succession()` decide whether the office is occupied, timer-gated, unresolved with declarations in play, or reset into a fresh vacancy window. Any new trace phase must be derived from those branches rather than inventing a second authority path.
3. AI political candidate generation already uses a direct visible-vacancy boundary in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs): `office_is_visibly_vacant()` requires `office_data.vacancy_since.is_some()` and `view.office_holder(office).is_none()`. The omission reason `PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant` already exists. The architecture therefore already has a clean AI-side closure vocabulary; the missing piece is an authoritative politics-trace equivalent.
4. Existing focused coverage is narrower than the previous ticket text claimed. I confirmed today with `cargo test -p worldwake-systems --lib -- --list` that [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) contains `offices::tests::succession_trace_records_vacancy_activation_and_timer_wait`, `offices::tests::support_succession_trace_records_tie_reset_and_filtered_votes`, and `offices::tests::force_succession_trace_records_install_and_blocked_cases`. There is also `offices::tests::support_succession_ignores_ineligible_declarations_and_resets_timer_on_no_valid_votes`, but it asserts authoritative state only, not the direct politics-trace phase for the no-eligible reset branch.
5. Existing golden coverage already proves the motivating behavior in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs): `golden_remote_office_claim_start_failure_loses_gracefully` uses action trace, request-resolution trace, decision trace, and authoritative office-holder state. The current gap is trace ergonomics and architectural directness, not missing behavior. The golden still has to infer office closure from several facts instead of reading one authoritative phase field.
6. This ticket is not a request-resolution or scheduler-boundary change. The first live rejection in the motivating golden still occurs at authoritative start validation for `declare_support`; this work stays in the authoritative politics-trace layer.
7. Ordering is not the contract. The needed improvement is semantic legibility: a trace consumer should be able to read the office's post-evaluation availability/closure phase directly instead of reconstructing it from `holder_before`, `vacancy_since_before`, declaration counts, and `OfficeSuccessionOutcome`.
8. Scope correction: the new field should be a derived phase on the existing `OfficeSuccessionTrace`, not an AI-only helper and not a new parallel sink.
9. Scope correction: focused tests must explicitly cover the no-eligible reset branch and the occupied/closed branch. The previous ticket text overstated existing direct trace coverage for those cases.

## Architecture Check

1. A derived authoritative availability/closure phase on `OfficeSuccessionTrace` is cleaner than forcing every consumer to reverse-engineer office availability from low-level branch details.
2. `OfficeSuccessionOutcome` should keep the branch-specific detail. The new phase should summarize the stable closure state that multiple consumers repeatedly need.
3. No backwards-compatible aliasing or parallel trace path should be introduced. Extend the existing trace struct and existing emitters in place.

## Verification Layers

1. Authoritative succession evaluation emits a direct derived availability/closure phase on every `OfficeSuccessionTrace` -> focused systems tests in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs).
2. Tie/reset/no-eligible cases expose reopening directly instead of making tests infer it from `vacancy_since_before` plus outcome kind -> focused political trace tests in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs).
3. Occupied and pending-declaration branches expose the direct closure state without reconstruction -> focused systems tests in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs).
4. The motivating golden can assert the authoritative closure phase directly, while action trace and decision trace continue to prove start failure and AI reconciliation -> targeted golden update in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs).
5. Office-holder installation remains a separate durable world-state fact. It must not replace the new trace phase when the trace contract itself is under test.

## What to Change

### 1. Extend the political trace model with a derived office-availability phase

Add an explicit derived phase or closure summary to `OfficeSuccessionTrace` in [politics_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) that covers at least:

- visibly vacant and claimable
- vacant but waiting on succession timer
- vacant with declarations present while succession is still unresolved
- closed because a holder exists
- reset/reopened vacancy window after no-eligible or tie outcomes

The exact enum names can differ, but the semantics must be direct and stable. If the final naming is broader than "claimability" so it also reads cleanly for force-law offices, prefer the broader name.

### 2. Emit the new phase from the authoritative succession system

Update [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) so each political trace event computes the claimability phase from current office state and the just-produced outcome without adding a second authority path.

### 3. Strengthen focused tests and one political golden if needed

Add or update focused office trace tests so support-law and occupied-office scenarios prove the new phases explicitly, including the no-eligible reset branch that currently lacks a direct politics-trace assertion. Update the existing political start-failure golden with one direct politics-trace assertion if it materially improves the proof surface.

## Files to Touch

- `crates/worldwake-sim/src/politics_trace.rs` (modify)
- `crates/worldwake-systems/src/offices.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify, only if a direct politics-trace assertion materially improves the golden)
- `crates/worldwake-ai/tests/golden_harness/timeline.rs` (modify, if trace struct construction requires the new field)

## Out of Scope

- changing office claim semantics
- changing `office_is_visibly_vacant()` in AI candidate generation
- changing request-resolution or action-trace infrastructure
- introducing a parallel political trace sink or compatibility wrapper

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems offices::tests::succession_trace_records_vacancy_activation_and_timer_wait -- --exact`
2. `cargo test -p worldwake-systems offices::tests::support_succession_trace_records_tie_reset_and_filtered_votes -- --exact`
3. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
4. Existing suites: `cargo test -p worldwake-systems` and `cargo test -p worldwake-ai`

### Invariants

1. Political traceability must expose office availability/closure phases directly from authoritative office state rather than forcing consumers to reconstruct them ad hoc.
2. The new phase data must remain derived trace information, not a second source of truth for office law.
3. Support-law and force-law offices must continue to use the same shared trace architecture without political-domain hacks layered on top.

## Test Plan

### New/Modified Tests

1. `offices::tests::succession_trace_records_vacancy_activation_and_timer_wait` in `crates/worldwake-systems/src/offices.rs` — prove direct phase emission for vacancy activation and timer-wait branches.
2. `offices::tests::succession_trace_records_pending_declarations_before_timer_elapses` in `crates/worldwake-systems/src/offices.rs` — prove pending declarations surface a direct unresolved phase before succession closes.
3. `offices::tests::support_succession_trace_records_tie_reset_and_filtered_votes` in `crates/worldwake-systems/src/offices.rs` — prove tie resets surface reopening directly.
4. `offices::tests::support_succession_trace_records_no_eligible_reset_phase` in `crates/worldwake-systems/src/offices.rs` — prove no-eligible resets surface reopening directly and preserve filtered declaration details.
5. `offices::tests::living_holder_trace_records_closed_occupied_phase` in `crates/worldwake-systems/src/offices.rs` — prove occupied offices emit a direct closed phase.
6. `offices::tests::force_succession_trace_records_install_and_blocked_cases` in `crates/worldwake-systems/src/offices.rs` — prove the shared phase model still reads cleanly for force-law install and contested branches.
7. `golden_remote_office_claim_start_failure_loses_gracefully` in `crates/worldwake-ai/tests/golden_emergent.rs` — prove the political start-failure golden can assert authoritative office closure through politics trace instead of only by manual inference.

### Commands

1. `cargo test -p worldwake-systems offices::tests::succession_trace_records_vacancy_activation_and_timer_wait -- --exact`
2. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
3. `cargo test -p worldwake-systems`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-20
- What actually changed:
  - Added `OfficeAvailabilityPhase` to `OfficeSuccessionTrace` in `crates/worldwake-sim/src/politics_trace.rs` and surfaced it through the shared `worldwake-sim` export surface.
  - Centralized authoritative politics-trace construction in `crates/worldwake-systems/src/offices.rs` so the new phase remains derived from existing office-law branches instead of becoming a second truth source.
  - Strengthened focused succession-trace coverage for pending declarations, no-eligible reset reopening, occupied closure, and force-law contested/install branches.
  - Updated `golden_remote_office_claim_start_failure_loses_gracefully` in `crates/worldwake-ai/tests/golden_emergent.rs` to assert direct authoritative closure through politics trace.
- Deviations from original plan:
  - The implemented type uses the broader name `OfficeAvailabilityPhase` instead of a narrower claimability-only label so the shared trace vocabulary also reads cleanly for force-law offices.
  - `crates/worldwake-ai/tests/golden_harness/timeline.rs` was updated to construct the expanded trace struct.
  - Workspace `clippy` surfaced pre-existing overlong test functions in `crates/worldwake-sim/src/tick_step.rs` and `crates/worldwake-ai/src/agent_tick.rs`; those were resolved with targeted lint allowances so the requested workspace lint gate passes.
- Verification results:
  - Passed `cargo test -p worldwake-sim`
  - Passed `cargo test -p worldwake-systems`
  - Passed `cargo test -p worldwake-ai`
  - Passed `cargo clippy --workspace --all-targets -- -D warnings`
