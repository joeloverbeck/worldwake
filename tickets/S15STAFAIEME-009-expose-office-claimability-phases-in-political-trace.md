# S15STAFAIEME-009: Expose Office Claimability Phases In Political Trace

**Status**: PENDING
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

## Assumption Reassessment (2026-03-19)

1. The current political trace model in [politics_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) records `OfficeSuccessionTrace { jurisdiction, succession_law, holder_before, vacancy_since_before, outcome, support_declarations, force_candidates }`. It does not expose an explicit office-claimability phase or closure reason.
2. The authoritative office rules in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) currently distinguish vacancy activation, timer waiting, support installation, tie resets, no-eligible resets, and force-law outcomes. Those are the right authoritative sources for any claimability summary; this ticket must derive its trace facts from them, not add a second source of truth.
3. AI political candidate generation already uses a direct visible-vacancy concept in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs): `office_is_visibly_vacant()` returns true only when `office_data.vacancy_since.is_some()` and the belief view has no office holder. The omission reason `PoliticalCandidateOmissionReason::OfficeNotVisiblyVacant` already exists. The trace gap is that the authoritative politics trace does not surface a similarly direct phase label.
4. Existing focused political trace coverage is present in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs), including `offices::tests::succession_trace_records_vacancy_activation_and_timer_wait` and `offices::tests::support_succession_trace_records_tie_reset_and_filtered_votes`. I confirmed those exact names today with `cargo test -p worldwake-systems --lib -- --list`.
5. Existing golden coverage already proves office locality, support-law claims, and the new political start-failure scenario, including `golden_remote_office_claim_start_failure_loses_gracefully` in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs). The gap is not missing behavior; it is missing a direct authoritative trace fact about office claimability phases.
6. This ticket is about authoritative political traceability, not request-resolution tracing. The first live rejection in the motivating golden still occurs at authoritative action start; this ticket should not blur that with request-resolution boundaries.
7. Ordering is not the main contract. The needed improvement is semantic: a trace reader should be able to tell whether the office was still claimable, already closed by holder occupation, pending unresolved declarations, or reset into a fresh vacancy window without reconstructing that state manually.
8. Scope correction: do not add AI-only or golden-only helper flags. Extend the shared political trace model so all political tests and debugging tools can use the same claimability vocabulary.

## Architecture Check

1. A derived claimability-phase summary inside the authoritative political trace is cleaner than forcing every consumer to duplicate the same office-law inference logic.
2. No backwards-compatible duplicate trace path should be introduced. The new claimability data should live inside the existing `OfficeSuccessionTrace` model and be emitted by the existing succession system.

## Verification Layers

1. Authoritative office evaluations emit an explicit claimability phase consistent with support-law and force-law rules -> focused systems tests in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs).
2. Tie/reset/no-eligible cases expose the correct closure or reopening phase -> focused political trace tests in [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs).
3. A golden political start-failure scenario can assert office closure or pending phase directly from politics trace instead of inferring from code -> targeted golden update in [golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) if needed.
4. Later office-holder installation remains a separate downstream durable fact. It must not replace the new authoritative trace phase when the contract is claimability before or during succession.

## What to Change

### 1. Extend the political trace model with office-claimability phase data

Add an explicit derived phase or closure summary to `OfficeSuccessionTrace` in [politics_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/politics_trace.rs) that covers at least:

- visibly vacant and claimable
- vacant but waiting on succession timer
- vacant with declarations present while succession is still unresolved
- closed because a holder exists
- reset/reopened vacancy window after no-eligible or tie outcomes

The exact enum names can differ, but the semantics must be direct and stable.

### 2. Emit the new phase from the authoritative succession system

Update [offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs) so each political trace event computes the claimability phase from current office state and the just-produced outcome without adding a second authority path.

### 3. Strengthen focused tests and one political golden if needed

Add or update focused office trace tests so support-law scenarios prove the new phases explicitly. If the existing political start-failure golden becomes materially clearer with one direct politics-trace assertion, add it there as well.

## Files to Touch

- `crates/worldwake-sim/src/politics_trace.rs` (modify)
- `crates/worldwake-systems/src/offices.rs` (modify)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify, only if a direct politics-trace assertion materially improves the golden)

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

1. Political traceability must expose office claimability phases directly from authoritative office state rather than forcing consumers to reconstruct them ad hoc.
2. The new phase data must remain derived trace information, not a second source of truth for office law.
3. Support-law and force-law offices must continue to use the same shared trace architecture without political-domain hacks layered on top.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` — prove claimability-phase emission for vacancy activation, timer waiting, tie reset, and no-eligible reset cases.
2. `crates/worldwake-systems/src/offices.rs` — prove occupied-office traces expose a closed/not-claimable phase directly.
3. `crates/worldwake-ai/tests/golden_emergent.rs` — if needed, prove the political start-failure golden can assert office closure or pending state through the new politics-trace phase rather than by manual inference.

### Commands

1. `cargo test -p worldwake-systems offices::tests::succession_trace_records_vacancy_activation_and_timer_wait -- --exact`
2. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
3. `cargo test -p worldwake-systems`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace`
