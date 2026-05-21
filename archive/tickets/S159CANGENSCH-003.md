# S159CANGENSCH-003: Provenance guard — no out-of-band candidate source

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — transient provenance capture in candidate-generation diagnostics (AI; no authoritative state)
**Deps**: archive/tickets/S159CANGENSCH-002.md

## Problem

After `archive/tickets/S159CANGENSCH-002.md` folded blocked-self-care into the
declared extractor pipeline, the required invariant was that every candidate
originated from a registry-declared `CandidateExtractorId`. This ticket added the
guard that proves the invariant and keeps it from regressing: a test asserting
that no candidate is emitted outside the declared extractor path (no untracked
candidate) and that every contributing extractor is a member of the canonical
order (no undeclared source). This is S159 Deliverable 3.

The guard keys on `CandidateExtractorId` — the registry authority — rather than
adding a provenance field to `GoalOffer`. Per-candidate provenance is already
carried authoritatively by `EmitterTag` on the persisted `GoalOfferedPayload`
(FND-29A); a second, non-isomorphic, transient provenance field on `GoalOffer`
would be a parallel representation of the same fact (FND-27/FND-28).

## Assumption Reassessment (2026-05-21)

1. `GoalOffer` (`crates/worldwake-ai/src/goal_model.rs:2227`) carries no
   extractor-id field, confirming the guard cannot read per-`GoalOffer`
   provenance directly. The pipeline loop binds `extractor_id` for each
   `extractor.extract(...)` call in both the pre-suppression and
   post-suppression phases, so the capture happens at emission time inside the
   pipeline.
2. `EmitterTag` (`crates/worldwake-core/src/decision_event_payload.rs:110`, 18
   variants) is a field on the authoritative, persisted `GoalOfferedPayload`
   (`decision_event_payload.rs:37`, serialized in
   `crates/worldwake-sim/src/save_load.rs`). It is not isomorphic to the 20→21
   variant `CandidateExtractorId`. Confirmed: adding a `GoalOffer.source_extractor`
   field would create a parallel transient duplicate of this authoritative fact
   (FND-27/28), so the guard reads the registry identity already present in the
   pipeline instead.
3. Shared boundary under audit: `CandidateGenerationResult` /
   `CandidateGenerationDiagnostics` (`candidate_generation.rs:222`, `:179`), both
   transient derived computations (FND-3) constructed within
   `generate_candidates_with_*`. The capture is added here, never promoted to
   authoritative world/belief state.
4. This ticket depends on `archive/tickets/S159CANGENSCH-002.md` having removed
   the out-of-band call; the guard's "no untracked candidate" assertion is now
   meaningful because blocked-self-care fallbacks flow through the declared
   pipeline.
5. Coverage-gap classification (precision rule 3): no existing test asserts the
   no-out-of-band-source invariant — verified by grepping the
   `candidate_generation.rs` `#[cfg(test)]` block for "provenance" / "untracked" /
   "extractor source" (none found). This is a missing focused/unit guard; the
   intended verification layer is candidate-generation focused/unit coverage, not
   golden E2E.

## Architecture Check

1. Keying the guard on `CandidateExtractorId` aligns the proof with the registry
   authority S159 consolidates, and adds no durable or authoritative surface — the
   capture lives in the transient result/diagnostics (FND-3, FND-27). This is
   cleaner than adding a `GoalOffer` field (parallel provenance, FND-27/28) and
   far smaller than reconciling `EmitterTag` (authoritative event-log surface,
   deferred to a future spec per S159 Non-Goals).
2. No backward-compatibility concern — this is net-new instrumentation plus a
   test; nothing is aliased or shimmed.

## Verified Layers

1. No candidate emitted outside the declared pipeline (no untracked candidate)
   -> focused unit guard test reading the transient per-candidate
   `CandidateExtractorId` capture (decision-trace-adjacent focused coverage).
2. Every contributing extractor ∈ canonical order (no undeclared source) ->
   same focused guard test, cross-checking against `CANDIDATE_EXTRACTOR_ORDER`.
3. Single-layer (AI candidate-generation focused coverage). No action-trace /
   event-log layer applies because the capture is transient and no authoritative
   mutation or action lifecycle is involved; stated explicitly per the
   single-layer rule.

## Landed Changes

### 1. Captured per-candidate extractor provenance (transient)

`crates/worldwake-ai/src/candidate_generation.rs` now records the emitting
`CandidateExtractorId` for each surviving candidate in the transient
`CandidateGenerationDiagnostics::extractor_sources` map. The capture is keyed by
`OpportunityKey`, covers the pre-suppression phase and the post-suppression
blocked-self-care phase, and is pruned through suppression plus redundant
opportunity-compiler removal so it stays aligned to the final candidate vector.
No `GoalOffer` field or authoritative state was added.

### 2. Added the provenance guard test

Added `candidate_generation::tests::every_candidate_traces_to_a_declared_extractor`.
The test runs candidate generation on a representative belief view and asserts
that every surviving candidate has a recorded contributing extractor and every
recorded `CandidateExtractorId` is a member of `CANDIDATE_EXTRACTOR_ORDER`.

## Landed Files

- `crates/worldwake-ai/src/candidate_generation.rs`

## Out of Scope

- Adding any provenance field to `GoalOffer` (FND-27/28 — `EmitterTag` is the
  authoritative provenance surface).
- Unifying `EmitterTag` with `CandidateExtractorId` (S159 Non-Goal; future spec).
- Any change to the emitted candidate set, ranking, or plans.

## Acceptance Result

### Test Results

1. `every_candidate_traces_to_a_declared_extractor` passes: no untracked
   candidate, every contributing extractor is in `CANDIDATE_EXTRACTOR_ORDER`.
2. Existing suite `cargo test -p worldwake-ai` passes.

### Verified Invariants

1. Every emitted candidate traces to a registry-declared `CandidateExtractorId`
   in the canonical order; no out-of-band source exists.
2. The provenance capture is transient (lives on the derived
   result/diagnostics), never promoted to authoritative world or belief state.
3. `GoalOffer` gains no provenance field (single authoritative provenance surface
   remains `EmitterTag`).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — added
   `every_candidate_traces_to_a_declared_extractor` plus the transient capture it
   reads.

### Commands Run

1. `cargo test -p worldwake-ai --lib candidate_generation::tests::every_candidate_traces_to_a_declared_extractor -- --exact`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-21.

- Added transient `CandidateGenerationDiagnostics::extractor_sources` keyed by
  `OpportunityKey` to record the registry extractor that contributed each
  surviving candidate.
- Populated the capture for both pre-suppression extractors and the
  post-suppression blocked-self-care extractor phase.
- Kept the capture aligned with the final candidate set by pruning suppressed
  candidates and redundant opportunity-compiler candidates.
- Added the focused provenance guard test without adding any `GoalOffer`
  provenance field or authoritative persisted state.

## Deviations

- The landed capture uses `CandidateGenerationDiagnostics` rather than
  `CandidateGenerationResult`; this preserves the ticket's transient-proof
  contract while keeping provenance beside the existing candidate diagnostics.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::every_candidate_traces_to_a_declared_extractor -- --exact`
- Passed `cargo test -p worldwake-ai`
- Passed `./scripts/verify.sh`
