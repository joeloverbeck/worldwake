# S159CANGENSCH-004: Decide blocked-self-care surviving-candidate gate semantics

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — focused candidate-generation proof; no production
behavior change
**Deps**: archive/tickets/S159CANGENSCH-002.md

## Problem

S159CANGENSCH-002 preserves the live blocked-self-care fallback behavior while
folding the emitter into the declared extractor pipeline. During reassessment,
the ticket/spec claim that the helper gates on "every surviving candidate is a
self-care fallback" was disproven: the live pre-refactor call passed a fresh
empty fallback-candidate vector, making the helper's
`goal_is_self_care_fallback` check phase-local and vacuous for the sole fallback
emitter.

This ticket investigated two possible contracts: phase-local admission, or a
deliberate non-vacuous gate over all post-suppression surviving candidates. The
result kept the phase-local contract.

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
2. No backward-compatibility shim is involved. The outcome retained the
   phase-local gate with explicit proof and did not add a compatibility path.

## Verified Layers

1. Gate input contract -> focused candidate-generation unit test that constructs
   a fully blocked self-care desire plus at least one unrelated surviving
   candidate.
2. Fallback admission or suppression reason -> decision-trace or focused
   diagnostics assertion over emitted candidates and suppression/omission
   diagnostics.
3. Behavior preservation -> `cargo test -p worldwake-ai` and `./scripts/verify.sh`.
   No affected golden coverage changed because the selected branch preserved
   candidate behavior.

## Landed Changes

### 1. Investigated the desired gate contract

The live post-suppression phase now has focused proof for the disputed case: a
fully blocked self-care desire with a lawful unrelated survivor. The selected
contract is phase-local fallback admission. A surviving unrelated candidate does
not suppress a need-driven exploration fallback for a still-blocked self-care
desire.

### 2. Kept production behavior unchanged

No gate semantics changed. The added focused test proves the behavior preserved
by S159CANGENSCH-002 is intentional: the post-suppression extractor receives its
own phase-local candidate list, while `fully_blocked_desires` remains the input
that authorizes need-driven exploration fallback emission.

## Landed Files

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `archive/tickets/S159CANGENSCH-004.md` (modify closeout)
- No `crates/worldwake-ai/tests/golden_*.rs` changes were needed because the
  selected contract preserves candidate behavior.

## Out of Scope

- Reopening the registry/phase fold from S159CANGENSCH-002.
- Unifying `EmitterTag` with `CandidateExtractorId`.
- Moving anomaly/observation interpretation out of candidate generation.

## Acceptance Result

### Tests Passed

1. Added and passed
   `blocked_self_care_fallback_survives_unrelated_post_suppression_candidate`.
2. Passed `cargo test -p worldwake-ai`.
3. Passed `./scripts/verify.sh`.

### Invariants

1. The selected gate contract is documented in this ticket's Outcome.
2. No behavior change was made; the focused proof distinguishes phase-local
   fallback admission from surviving-candidate suppression.
3. Candidate emission remains routed through declared extractors only.

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` —
   `blocked_self_care_fallback_survives_unrelated_post_suppression_candidate`.

### Commands Run

1. Passed `cargo test -p worldwake-ai blocked_self_care_fallback_survives_unrelated_post_suppression_candidate`.
2. Passed `cargo test -p worldwake-ai`.
3. Passed `./scripts/verify.sh`.

## Outcome

Completed on 2026-05-21.

- Added focused candidate-generation proof for the disputed blocked-self-care
  gate input contract.
- Retained the phase-local gate semantics from S159CANGENSCH-002: unrelated
  surviving candidates do not suppress a need-driven exploration fallback for a
  fully blocked self-care desire.
- Made no production behavior or golden changes.

## Deviations

- The investigation selected the no-production-change branch. The stronger
  surviving-candidate gate was rejected because the focused proof shows the
  unrelated survivor is not a lawful reason to suppress exploration for a still
  blocked self-care need.

## Verification Result

- Passed `cargo test -p worldwake-ai blocked_self_care_fallback_survives_unrelated_post_suppression_candidate`.
- Passed `cargo test -p worldwake-ai`.
- Passed `./scripts/verify.sh`.
