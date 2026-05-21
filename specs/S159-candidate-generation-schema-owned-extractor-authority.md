# S159 — Candidate-Generation Schema-Owned Extractor Authority

**Status:** Draft
**Type:** Pure refactor (behavior-preserving; removes a fossil seam and an
out-of-band emitter)
**Priority:** Medium — cleanup, not safety-critical. Sequence after S158.
**Foundations:** FND-20, FND-28, FND-29

## Problem Statement

### Motivation

Candidate generation in `crates/worldwake-ai/src/candidate_generation.rs` is the
single place desires/opportunities become goal candidates. The goal-schema
consolidation (`GoalSchema.candidate_extractors`) was meant to make the schema the
authority for which extractors run. That consolidation only half-landed:

1. A constant literally named **`LEGACY_EXTRACTOR_ORDER`** still owns extractor
   *execution order*. The schema is filtered through it, so a new schema
   declaration does not actually own its place in the generation order — it only
   gets to run if `LEGACY_EXTRACTOR_ORDER` already lists it. This is a fossil
   authority name in a live path (FND-28).
2. `emit_exploration_candidates_for_blocked_self_care` runs **out-of-band**,
   after the main `for extractor_id in ordered_candidate_extractors_from_goal_schemas()`
   loop and after suppression filtering. It is a hidden candidate source outside
   the declared extractor registry (FND-28; FND-20 emergence-via-declared-paths).

### Evidence (verified against code on 2026-05-21)

- `LEGACY_EXTRACTOR_ORDER: [CandidateExtractorId; 20]` defined ~L495–516.
- `ordered_candidate_extractors_from_goal_schemas()` (~L518–528) collects the
  schema-declared extractor set, then **filters `LEGACY_EXTRACTOR_ORDER` by it** —
  the legacy const is the ordering authority, the schema only the membership
  filter. Used in the main loop ~L768.
- `emit_exploration_candidates_for_blocked_self_care(...)` called ~L798, outside
  the extractor loop, fed by `fully_blocked_desires` from the main pass; its
  output is suppression-filtered separately and appended.
- `CandidateGenerationResult` (~L222–239) also carries `pending_violations`,
  `pending_discrepancies`, `pending_source_reliability_failures`, and
  `pending_acquisition_exhaustion_resets`. **This is documented as
  side-effect-free** (the caller applies them in the write phase). The audit's
  "candidate gen mixes emission with anomaly detection" concern is real as a
  responsibility-breadth smell but is **explicitly out of scope here** — moving
  observation/anomaly interpretation out of candidate emission is a larger
  perception-architecture change, not a fossil-seam removal. It may warrant a
  future spec; this spec does not touch it.

### Key scoping decisions (brainstorm 2026-05-21)

- Behavior must be **preserved**: the emitted candidate set (and resulting
  rankings/plans) for every existing golden is unchanged. This is a naming and
  wiring cleanup, not a behavioral change.

## Deliverables

1. **Rename the ordering authority.** Replace `LEGACY_EXTRACTOR_ORDER` with a
   canonical, non-"legacy" schema-owned ordering. Two acceptable shapes (decide
   at ticket time):
   - a) A canonical `CANDIDATE_EXTRACTOR_ORDER` constant documented as the single
     declared execution order, with a test asserting it is exactly the set the
     `GoalSchema` declarations require (no orphan members, no missing members); or
   - b) Move ordering onto the schema declarations themselves (each extractor's
     order derived from `GoalDispatchKey` declaration), eliminating the standalone
     constant entirely.
   Whichever shape, the result must make the schema the authority and remove the
   "legacy" fossil name from the live path (FND-28).
2. **Fold blocked-self-care into the registry.** `blocked-self-care exploration`
   must become a declared `CandidateExtractorId` that runs inside the ordered
   extractor loop, consuming `fully_blocked_desires` as a lawful input. No
   candidate may be emitted outside the declared extractor path. If it genuinely
   needs the result of the first pass, model it as a declared second-phase
   extractor with an explicit ordering position, not an out-of-band call.
3. **Guard test.** Add a test asserting every emitted `GoalOffer`'s
   `CandidateExtractorId` provenance is a declared extractor in the canonical
   order — i.e. no candidate has an undeclared or out-of-band source.

## FND-01 Section H Analysis

Pure refactor; introduces no new system, state, component, action, or feedback
loop, and is required to be behavior-preserving.

- **Information-path analysis:** Not applicable. No information path changes;
  candidate inputs (beliefs, memories, recipes, opportunities) are unchanged.
- **Positive-feedback analysis:** Not applicable. No amplifying loop.
- **Concrete dampeners:** Not applicable.
- **Stored-state vs. derived read-model list:** No authoritative state.
  `CandidateExtractorId` ordering is a static policy table; the
  `CandidateGenerationResult` remains a transient derived computation (FND-3). The
  rename/folding does not promote any derived value to truth.
- **Planner-formalism analysis:** No formalism change. Candidate generation feeds
  ranking → portfolio → planning identically; only the *internal authority* for
  which/what-order extractors run is consolidated. No goal becomes method-required.

### Proof surface (FND-31)

- All existing `worldwake-ai` goldens pass unchanged (behavior preservation is the
  primary regression guard).
- New: candidate-provenance guard test (Deliverable 3) — fails if any candidate is
  emitted outside the declared extractor registry.
- New: ordering-completeness test — fails if the canonical order and the
  schema-declared extractor set diverge (no orphan/missing extractor).
```
