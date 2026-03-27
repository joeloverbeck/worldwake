# S32CRIMEMEGOLSUI-005: Consolidate Theft Witness-Deterrence Arithmetic

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` theft candidate generation and ranking
**Deps**: E17 (crime/theft/justice), S32CRIMEMEGOLSUI-002

## Problem

The witness-deterrence arithmetic for theft currently exists in two AI phases:

- candidate generation in `crates/worldwake-ai/src/candidate_generation.rs::emit_theft_candidates()`
- ranking in `crates/worldwake-ai/src/ranking.rs::theft_motive()`

Both sites independently compute the same substrate: locally observed living co-located witnesses, `witness_risk_penalty * witness_count`, and the resulting theft motive after deterrence. This duplication is architecturally brittle. If either side drifts, the AI can lawfully enter a contradictory state where candidate emission and theft ranking no longer describe the same world facts.

That violates the repo’s stated standard for clean, robust, extensible architecture in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): the deterrence rule is one concrete causal fact and should have one canonical implementation path inside the AI layer.

## Assumption Reassessment (2026-03-27)

1. The exact duplicated arithmetic is present today in `crates/worldwake-ai/src/candidate_generation.rs:1923` and `crates/worldwake-ai/src/ranking.rs:587`. Both count co-located living agents other than the actor and apply `witness_risk_penalty` against `theft_motive_weight`.
2. This is an AI-layer duplication, not an authoritative/action-layer duplication. No theft validation, transport action precondition, or world-state mutation code currently reimplements the same witness gate. The scope is therefore `worldwake-ai` only.
3. Coverage already exists at the focused-test layer for each side independently:
   - `crates/worldwake-ai/src/candidate_generation.rs::tests::theft_candidate_respects_preconditions_and_witness_gate`
   - `crates/worldwake-ai/src/ranking.rs::tests::theft_goal_is_zero_motive_when_witness_penalty_cancels_profile_weight`
4. Golden/E2E coverage is no longer missing for the scenario-level contract. `crates/worldwake-ai/tests/golden_emergent.rs::golden_witness_deterrence_suppresses_theft_candidate` and its replay companion already exist and prove that witness deterrence can suppress theft in the live runtime. This ticket therefore targets a focused AI-architecture duplication beneath already-delivered golden coverage, not a missing scenario implementation from `specs/S32-crime-emergence-golden-suites.md`.
5. What is still missing is an explicit proof that candidate generation and ranking share one canonical deterrence substrate. Current focused tests and the golden scenario prove behavior, but they would not necessarily catch future drift if one formula changed and the other did not.
6. Shared abstraction boundary under audit: the AI theft-deterrence substrate that maps `(TheftDispositionProfile, locally observed co-located living agents)` to an effective theft motive. This boundary is consumed by candidate generation and ranking, which are distinct phases under `docs/precision-rules.md`.
7. Live goal surface under audit is `GoalKind::StealItem { target_item }`. The change is not about lawful target filtering, plan search, or authoritative `steal` execution; it is strictly about the pre-search AI motive/gating substrate.
8. Verification is primarily focused/unit coverage inside `worldwake-ai`, with the existing golden scenario retained as a regression layer because it already exercises the same rule end-to-end.
9. Ordering is not the contract here. The issue is phase consistency across candidate generation and ranking, not strict tick ordering, action lifecycle ordering, or event-log ordering.
10. No missing architectural substrate needs to be invented. The deterrence rule already exists and is conceptually sound under Principle 10; the issue is duplication of that existing rule, not an absent model.
11. Foundations alignment:
   - Principle 3: the deterrence score should stay traceable to concrete local witnesses and a concrete theft profile, not drift into two separate AI truths.
   - Principle 7: witness pressure is local co-location information, so the shared helper must remain belief/locality based, not reach for omniscient state.
   - Systems-interact-through-state invariant: this should remain a state-derived AI calculation, not a new direct cross-system call path. This repo still has older documents that call this “Principle 24,” but `docs/FOUNDATIONS.md` is now numbered through Principle 22, so this ticket should cite the invariant text rather than the stale number.
12. Adjacent contradiction classification: if reassessment during implementation finds one copy already semantically differs from the other in a way that changes behavior, that is an in-scope architectural correction for this ticket, not a separate bug.
13. Mismatch + correction: prior commentary described this as a “note for follow-up cleanup.” Reassessment confirms it should be a real engine ticket because the duplication sits on an active causal rule used by two AI phases, not a cosmetic helper extraction.

## Architecture Check

1. The clean architecture is a single canonical theft-deterrence computation inside `worldwake-ai` that both candidate generation and ranking call. That removes drift risk without changing the underlying model.
2. This is better than leaving two “equivalent by convention” copies because the deterrence rule is one causal fact. One helper makes future changes auditable, testable, and locally explainable.
3. This is better than moving the arithmetic into `worldwake-core` because the computation depends on AI belief-view queries such as local observation, alive filtering, and entity-kind reads. The right abstraction boundary is shared AI logic, not core world state.
4. No backwards-compatibility aliasing or shims introduced. The old duplicated logic should be removed, not wrapped and retained in parallel.

## Verification Layers

1. Candidate generation and ranking use the same canonical theft-deterrence substrate -> focused/unit tests in `worldwake-ai`
2. Candidate suppression still occurs when witness penalty cancels motive -> focused/unit candidate-generation test
3. Theft motive still drops to zero under the same witness conditions -> focused/unit ranking test
4. Existing runtime scenario still suppresses theft under witness deterrence -> `golden_witness_deterrence_suppresses_theft_candidate`
5. Broader AI behavior remains regression-free -> `cargo test -p worldwake-ai`
6. Workspace-wide safety net remains green -> `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`
7. Single-layer engine ticket plus retained runtime regression: no additional action-trace, event-log, or authoritative world-state mapping is required for the architectural fix itself because the contract is AI-local pre-search consistency, but the existing golden scenario remains useful as an end-to-end guardrail.

## What to Change

### 1. Extract a canonical theft-deterrence helper in `worldwake-ai`

Introduce one shared helper in the AI crate that computes the effective theft motive from:

- actor
- current place / observed local entities
- `TheftDispositionProfile`
- locally observed living agent count

The helper should expose the single canonical arithmetic used by both phases. Keep it belief-view based; do not route it through authoritative world-only shortcuts.

### 2. Replace the duplicated call sites

- Update `emit_theft_candidates()` to use the shared helper for the witness gate.
- Update `theft_motive()` in ranking to use the same helper.
- Remove the duplicated inline witness-count and penalty arithmetic from both call sites.

### 3. Strengthen focused tests around phase consistency

Add or update focused tests so the repo explicitly proves:

- the shared helper returns the expected motive under zero, partial, and fully-deterring witness counts
- candidate generation still suppresses `StealItem` exactly when the shared helper yields zero motive
- ranking still reports zero theft motive under the same conditions
- the existing golden scenario continues to suppress theft at runtime without requiring new golden coverage

If the cleanest test shape is to add one helper-focused test plus small adjustments to existing candidate/ranking tests, prefer that over building redundant new scenarios.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-ai/src/theft.rs` (new shared AI helper module)
- `crates/worldwake-ai/tests/golden_emergent.rs` (verify only; no code changes required)

## Out of Scope

- New golden tests or golden ticket updates (Scenario 42 is already implemented)
- Changes to authoritative `steal` action validation or transport semantics
- Changes to `TheftDispositionProfile` shape or crime balancing values
- Moving AI belief-view logic into `worldwake-core`
- Broader crime/justice refactors unrelated to the duplicated deterrence arithmetic

## Acceptance Criteria

### Tests That Must Pass

1. Focused/unit tests covering the shared theft-deterrence helper and its candidate/ranking consumers
2. Existing `golden_witness_deterrence_suppresses_theft_candidate` coverage still passes
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Candidate generation and ranking derive theft deterrence from one canonical AI helper, not duplicated arithmetic.
2. The deterrence rule remains grounded in locally observed living witnesses plus `TheftDispositionProfile`, with no omniscient or non-local shortcut.
3. No behavior change is introduced beyond removing divergence risk; the live witness-threshold semantics remain the same.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` focused theft tests — confirm candidate suppression still matches the canonical helper
2. `crates/worldwake-ai/src/ranking.rs` focused theft tests — confirm ranking still yields the same theft motive through the canonical helper
3. `crates/worldwake-ai/src/theft.rs` shared helper tests — prove zero, partial, and fully-deterring witness-count arithmetic directly
4. `crates/worldwake-ai/tests/golden_emergent.rs::golden_witness_deterrence_suppresses_theft_candidate` — existing runtime regression that proves the focused refactor still preserves the live end-to-end behavior

### Commands

1. `cargo test -p worldwake-ai theft_deterrence_assessment_supports_zero_partial_and_full_deterrence`
2. `cargo test -p worldwake-ai theft_candidate_respects_preconditions_and_witness_gate`
3. `cargo test -p worldwake-ai theft_goal_is_zero_motive_when_witness_penalty_cancels_profile_weight`
4. `cargo test -p worldwake-ai golden_witness_deterrence_suppresses_theft_candidate`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-27
- What actually changed:
  - corrected the ticket assumptions before implementation to reflect that Scenario 42 golden coverage already exists in `crates/worldwake-ai/tests/golden_emergent.rs`
  - extracted the duplicated witness-deterrence arithmetic into a shared AI-local module at `crates/worldwake-ai/src/theft.rs`
  - updated `emit_theft_candidates()` and `theft_motive()` to consume the same canonical theft-deterrence assessment
  - added focused helper coverage for zero, partial, and fully-deterring witness counts
- Deviations from original plan:
  - no new golden test was added because the S32 witness-deterrence runtime scenario was already implemented and passing
  - existing focused candidate-generation and ranking tests remained valid without further edits once the helper extraction landed
- Verification results:
  - `cargo test -p worldwake-ai theft_deterrence_assessment_supports_zero_partial_and_full_deterrence` passed
  - `cargo test -p worldwake-ai theft_candidate_respects_preconditions_and_witness_gate` passed
  - `cargo test -p worldwake-ai theft_goal_is_zero_motive_when_witness_penalty_cancels_profile_weight` passed
  - `cargo test -p worldwake-ai golden_witness_deterrence_suppresses_theft_candidate` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
