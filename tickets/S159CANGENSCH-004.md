# S159CANGENSCH-004: Decide blocked-self-care surviving-candidate gate semantics

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — candidate-generation gate semantics if investigation proves a behavior change is warranted
**Deps**: archive/tickets/S159CANGENSCH-002.md

## Problem

S159CANGENSCH-002 preserves the live blocked-self-care fallback behavior while
folding the emitter into the declared extractor pipeline. During reassessment,
the ticket/spec claim that the helper gates on "every surviving candidate is a
self-care fallback" was disproven: the live pre-refactor call passed a fresh
empty fallback-candidate vector, making the helper's
`goal_is_self_care_fallback` check phase-local and vacuous for the sole fallback
emitter.

This ticket investigates whether that gate should remain phase-local or become a
deliberate non-vacuous gate over all post-suppression surviving candidates. That
is a behavior question, not part of the behavior-preserving fossil-seam removal.

## Assumption Reassessment (2026-05-21)

1. Live pre-S159CANGENSCH-002 behavior called
   `emit_exploration_candidates_for_blocked_self_care` after the first
   `filter_suppressed_candidates` pass with an empty
   `blocked_fallback_candidates` vector. Therefore the helper's
   `goal_is_self_care_fallback` gate did not inspect the surviving candidate set.
2. S159CANGENSCH-002 intentionally preserves that behavior while moving the
   fallback into `CandidateExtractorId::BlockedSelfCareExploration` and the
   declared post-suppression phase.
3. Shared boundary under audit: AI candidate generation's post-suppression phase
   in `crates/worldwake-ai/src/candidate_generation.rs`, specifically whether
   blocked-self-care fallback admission should depend only on
   `fully_blocked_desires` plus exploration profile inputs, or also on the
   non-self-care surviving candidate set.
4. Intended invariant before any behavior change: if a self-care desire is fully
   blocked, the agent may emit a need-driven exploration fallback from the
   declared post-suppression extractor. The excluded competing branch is changing
   that fallback merely because another unrelated candidate survived suppression.
5. Adjacent contradiction classification: the stale "all surviving candidates"
   wording was corrected in S159CANGENSCH-002 as a behavior-preserving scope
   fix. Any stronger gate must be justified and verified here.

## Architecture Check

1. Investigation first is cleaner than smuggling a semantic change into a
   refactor. FND-20 requires agent decisions to remain explainable from beliefs
   and priorities, and FND-31 requires proof that a changed gate fails or passes
   for the right reason.
2. No backward-compatibility shim is involved. The outcome should be either a
   retained phase-local gate with explicit proof, or a single deliberate gate
   change with old behavior removed.

## Verification Layers

1. Gate input contract -> focused candidate-generation unit test that constructs
   a fully blocked self-care desire plus at least one unrelated surviving
   candidate.
2. Fallback admission or suppression reason -> decision-trace or focused
   diagnostics assertion over emitted candidates and suppression/omission
   diagnostics.
3. Behavior preservation or behavior change fallout -> `cargo test -p
   worldwake-ai`, plus affected golden coverage if the focused test shows the
   selected branch changes in an existing scenario.

## What to Change

### 1. Investigate the desired gate contract

Use the live post-suppression phase from S159CANGENSCH-002 to compare two cases:
a fully blocked self-care desire with no unrelated surviving candidates, and the
same desire with a lawful unrelated survivor. Decide whether the fallback should
emit in both cases or only the first.

### 2. Land the chosen behavior if needed

If the phase-local gate is correct, add/keep focused proof and close this ticket
as an investigation result with no production behavior change. If the
surviving-candidate gate is correct, change the post-suppression extractor input
contract explicitly and update tests/goldens that prove the new branch.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `tickets/S159CANGENSCH-004.md` (modify closeout)
- affected `crates/worldwake-ai/tests/golden_*.rs` only if the chosen behavior
  changes existing scenario outcomes

## Out of Scope

- Reopening the registry/phase fold from S159CANGENSCH-002.
- Unifying `EmitterTag` with `CandidateExtractorId`.
- Moving anomaly/observation interpretation out of candidate generation.

## Acceptance Criteria

### Tests That Must Pass

1. Focused candidate-generation test covering fully blocked self-care with an
   unrelated surviving candidate.
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`

### Invariants

1. The selected gate contract is documented in this ticket's Outcome.
2. Any behavior change is backed by a focused proof that distinguishes
   phase-local fallback admission from surviving-candidate suppression.
3. Candidate emission remains routed through declared extractors only.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — focused test for the
   disputed gate input contract.

### Commands

1. `cargo test -p worldwake-ai <new_gate_contract_test>`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`
