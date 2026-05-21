# S159CANGENSCH-003: Provenance guard — no out-of-band candidate source

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — transient provenance capture in the candidate-generation result (AI; no authoritative state)
**Deps**: archive/tickets/S159CANGENSCH-002.md

## Problem

After `archive/tickets/S159CANGENSCH-002.md` folded blocked-self-care into the
declared extractor pipeline, every candidate should originate from a
registry-declared `CandidateExtractorId`. This ticket adds the guard that
*proves* the invariant and keeps it from regressing: a test asserting that no
candidate is emitted outside the declared extractor path (no untracked
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
   `extractor.extract(...)` call (`candidate_generation.rs:768`; after ticket 002
   the post-suppression phase binds it too), so the capture happens at emission
   time inside the pipeline.
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

## Verification Layers

1. No candidate emitted outside the declared pipeline (no untracked candidate)
   -> focused unit guard test reading the transient per-candidate
   `CandidateExtractorId` capture (decision-trace-adjacent focused coverage).
2. Every contributing extractor ∈ canonical order (no undeclared source) ->
   same focused guard test, cross-checking against `CANDIDATE_EXTRACTOR_ORDER`.
3. Single-layer (AI candidate-generation focused coverage). No action-trace /
   event-log layer applies because the capture is transient and no authoritative
   mutation or action lifecycle is involved; stated explicitly per the
   single-layer rule.

## What to Change

### 1. Capture per-candidate extractor provenance (transient)

In `crates/worldwake-ai/src/candidate_generation.rs`: record, inside the
generation pipeline, the emitting `CandidateExtractorId` for each emitted
candidate (e.g., a `BTreeMap<OpportunityKey, CandidateExtractorId>` or a parallel
`Vec` aligned with the candidate vector) on `CandidateGenerationResult` or
`CandidateGenerationDiagnostics`. The capture must cover both the pre-suppression
phase and the post-suppression phase added in ticket 002, so that every surviving
candidate has a recorded contributing extractor. Use a `Default`-friendly field so
existing construction sites in this file (the dead-agent early return and the main
result assembly) need no value-bearing edits beyond the capture itself.

### 2. Add the provenance guard test

Add a focused unit test (e.g., `every_candidate_traces_to_a_declared_extractor`)
that runs candidate generation on a representative belief view and asserts:
(i) every emitted candidate has a recorded contributing extractor (no untracked
candidate), and (ii) every recorded `CandidateExtractorId` is a member of
`CANDIDATE_EXTRACTOR_ORDER`. The test fails if a future change reintroduces an
out-of-band append (untracked candidate) or an undeclared extractor.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Adding any provenance field to `GoalOffer` (FND-27/28 — `EmitterTag` is the
  authoritative provenance surface).
- Unifying `EmitterTag` with `CandidateExtractorId` (S159 Non-Goal; future spec).
- Any change to the emitted candidate set, ranking, or plans.

## Acceptance Criteria

### Tests That Must Pass

1. `every_candidate_traces_to_a_declared_extractor` — passes: no untracked
   candidate, every contributing extractor ∈ `CANDIDATE_EXTRACTOR_ORDER`.
2. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Every emitted candidate traces to a registry-declared `CandidateExtractorId`
   in the canonical order; no out-of-band source exists.
2. The provenance capture is transient (lives on the derived
   result/diagnostics), never promoted to authoritative world or belief state.
3. `GoalOffer` gains no provenance field (single authoritative provenance surface
   remains `EmitterTag`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — add
   `every_candidate_traces_to_a_declared_extractor` focused guard test plus the
   transient capture it reads.

### Commands

1. `cargo test -p worldwake-ai every_candidate_traces_to_a_declared_extractor`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`
