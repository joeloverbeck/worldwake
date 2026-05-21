# S159 — Candidate-Generation Schema-Owned Extractor Authority

**Status:** Draft
**Type:** Pure refactor (behavior-preserving; removes a fossil seam and an
out-of-band emitter)
**Priority:** Medium — cleanup, not safety-critical. Sequence after archived S158.
**Foundations:** FND-20, FND-27, FND-28, FND-29

## Problem Statement

### Motivation

Candidate generation in `crates/worldwake-ai/src/candidate_generation.rs` is the
single place desires/opportunities become goal candidates. The goal-schema
consolidation (`GoalSchema.candidate_extractors`) was meant to make the schema the
authority for which extractors run. That consolidation only half-landed. Ticket
`archive/tickets/S159CANGENSCH-001.md` has now removed the fossil ordering name;
the remaining live out-of-band candidate source is still ticket
`S159CANGENSCH-002` scope:

1. Before `archive/tickets/S159CANGENSCH-001.md`, a constant literally named
   **`LEGACY_EXTRACTOR_ORDER`** owned extractor *execution order*. That live
   fossil name has been replaced with `CANDIDATE_EXTRACTOR_ORDER`, documented as
   the single declared top-level execution order, while preserving behavior
   (FND-28).
2. `emit_exploration_candidates_for_blocked_self_care` runs **out-of-band**,
   after the main `for extractor_id in ordered_candidate_extractors_from_goal_schemas()`
   loop and after suppression filtering. It is a hidden candidate source outside
   the declared extractor registry (FND-28; FND-20 emergence-via-declared-paths).

### Evidence (verified against code on 2026-05-21)

- `CANDIDATE_EXTRACTOR_ORDER: [CandidateExtractorId; 20]` is the live canonical
  order in `crates/worldwake-ai/src/candidate_generation.rs`. Its member set and
  order are identical to `CandidateExtractorId::ALL`
  (`crates/worldwake-core/src/agent_schema_context_profile.rs`), so the
  schema-membership filter below currently filters nothing — this is the
  behavior-preserving state landed by `archive/tickets/S159CANGENSCH-001.md`.
- `ordered_candidate_extractors_from_goal_schemas()` collects the schema-declared
  extractor set into a `BTreeSet` (membership only — per-key declaration order is
  discarded), then filters `CANDIDATE_EXTRACTOR_ORDER` by it. Used in the main
  extractor loop. The per-key schema lists are reached via
  `GoalDispatchKey::declaration().candidate_extractors`
  (`crates/worldwake-ai/src/goal_schema.rs`).
- `emit_exploration_candidates_for_blocked_self_care(...)` is called at L798,
  **after** the first `filter_suppressed_candidates` pass (L788). It consumes
  `diagnostics.fully_blocked_desires` produced by that suppression pass (L795),
  early-returns unless every surviving candidate is a self-care fallback
  (L3786), and its output is run through a **second, separate**
  `filter_suppressed_candidates` pass (L806) before being appended. This
  post-suppression dependency is why the emitter cannot be naively folded into
  the single pre-suppression extractor loop — see Deliverable 2.
- `CandidateGenerationResult` (L222–239) also carries `pending_violations`,
  `pending_discrepancies`, `pending_source_reliability_failures`, and
  `pending_acquisition_exhaustion_resets`. **This is documented as
  side-effect-free** (the caller applies them in the write phase). The audit's
  "candidate gen mixes emission with anomaly detection" concern is real as a
  responsibility-breadth smell but is **explicitly out of scope here** — moving
  observation/anomaly interpretation out of candidate emission is a larger
  perception-architecture change, not a fossil-seam removal. It may warrant a
  future spec; this spec does not touch it.

### Provenance surfaces (relevant to Deliverable 3)

Candidate provenance is already represented twice in the codebase, and the two
representations are **not** isomorphic — this shapes what Deliverable 3 can and
cannot do:

- **`CandidateExtractorId`** (`crates/worldwake-core/src/agent_schema_context_profile.rs:6`,
  20 variants) — the **registry/dispatch identity**. It declares which extractors
  exist, gates which run (`disabled_extractors`), and orders the loop. It is a
  static policy table, transient at runtime. This spec consolidates *this*
  surface as the emission authority.
- **`EmitterTag`** (`crates/worldwake-core/src/decision_event_payload.rs:110`,
  18 variants) — the **authoritative, persisted per-candidate provenance**. It is
  a field on `GoalOfferedPayload` (`decision_event_payload.rs:37`), recorded into
  the append-only event log (`crates/worldwake-core/src/event_record.rs`) and
  serialized through `crates/worldwake-sim/src/save_load.rs`. It already answers
  FND-29's "why did this agent consider this goal?" through persisted causal
  history (FND-29A).

`GoalOffer` itself (`crates/worldwake-ai/src/goal_model.rs:2227`) carries **no**
extractor-id field. The `EmitterTag` enum is non-isomorphic to
`CandidateExtractorId` (`HomeostaticNeeds` vs `Need`; an `EpistemicSensing` tag
with no clean extractor counterpart; no `OpportunityCompiler`/`ReportFound`
tags). Reconciling the two provenance taxonomies into one would touch the
authoritative append-only event record, save/load, and replay — that is a
provenance-unification change, not a fossil-seam removal, and is a **Non-Goal**
here (see below).

### Key scoping decisions (brainstorm 2026-05-21)

- Behavior must be **preserved**: the emitted candidate set (and resulting
  rankings/plans) for every existing golden is unchanged. This is a naming and
  wiring cleanup, not a behavioral change.

## Non-Goals

- **Unifying `EmitterTag` with `CandidateExtractorId`.** The two provenance
  taxonomies (registry identity vs. persisted event provenance) are non-isomorphic
  and `EmitterTag` is authoritative append-only history (FND-29A). Collapsing them
  touches the event log, save/load, and replay — out of scope for a
  behavior-preserving fossil-seam removal. Candidate for a future spec.
- **Moving anomaly/observation interpretation out of candidate emission.** The
  `pending_*` collections on `CandidateGenerationResult` stay where they are; the
  responsibility-breadth smell is a larger perception-architecture change (see
  Evidence above).

## Deliverables

1. **Rename the ordering authority — completed by
   `archive/tickets/S159CANGENSCH-001.md`.** The accepted shape is a canonical
   `CANDIDATE_EXTRACTOR_ORDER` constant documented as the single declared
   execution order, with a test asserting it is exactly the set the `GoalSchema`
   declarations require (no orphan members, no missing members). This shape
   **updated the existing completeness test** rather than adding a new one — see
   the completeness-test note below.
   - a) **Accepted, lower-risk.** `CANDIDATE_EXTRACTOR_ORDER`.
   - b) Move ordering onto the schema declarations themselves, eliminating the
     standalone constant entirely. **Caveat:** the current schema surface cannot
     supply a total order. `candidate_extractors` is a `&'static [CandidateExtractorId]`
     *per* `GoalDispatchKey`, and `ordered_candidate_extractors_from_goal_schemas`
     collects them into a `BTreeSet` (membership only; per-key order discarded).
     Multiple dispatch keys declare overlapping extractors, so no global total
     order is recoverable from per-key lists — option (b) therefore requires
     **introducing an explicit per-extractor order annotation** on the schema, not
     merely "moving" data that already exists. Weigh this added state against the
     simplicity of (a).

   The accepted shape removes the "legacy" fossil name from the live Rust path
   (FND-28) without adding a new ordering state model.

   **Completeness test (update, not new).** The ordering-completeness invariant
   is now `canonical_extractor_order_covers_every_registered_extractor_once`,
   which asserts `ordered_candidate_extractors_from_goal_schemas()` equals
   `CANDIDATE_EXTRACTOR_ORDER`, and that the registry and canonical order cover
   `CandidateExtractorId::ALL`.

2. **Fold blocked-self-care into the registry as a declared second-phase
   extractor.** `blocked-self-care exploration` must become a declared
   `CandidateExtractorId` that runs inside the ordered extractor pipeline,
   consuming `fully_blocked_desires` as a lawful input. No candidate may be
   emitted outside the declared extractor path.

   The fold is **not** trivially behavior-preserving, because the current emitter
   depends on state that does not exist during the pre-suppression loop. Preserve
   behavior by modeling it as a **declared post-suppression phase** of the
   extractor pipeline (Q2 resolution):

   - The canonical order (Deliverable 1) tags each extractor with a **phase**:
     pre-suppression (the existing main loop) or post-suppression. The pipeline
     runs the pre-suppression phase, applies the first `filter_suppressed_candidates`
     pass, then runs the post-suppression phase over its declared extractor subset.
   - The blocked-self-care extractor is a post-suppression extractor. Its
     `ExtractorContext` (or an equivalent declared input) must carry the
     post-suppression `fully_blocked_desires`, and the pipeline must preserve:
     (a) the all-surviving-candidates-are-self-care-fallback gate (current L3786),
     and (b) the **separate** suppression filtering of its output (current L806).
   - Net effect: identical candidate set to today, but every candidate — including
     blocked-self-care fallbacks — now originates from a registry-declared
     extractor with an explicit ordering position. No out-of-band call remains.

   **Cross-crate variant-addition checklist** (adding the blocked-self-care
   `CandidateExtractorId` variant; [Pattern: New Enum Variant on Cross-Crate
   Enum]). The variant lives in `worldwake-core`, so the addition touches:
   - `CandidateExtractorId` enum + `ALL: [Self; 20]` → `[Self; 21]`
     (`agent_schema_context_profile.rs:6,30`);
   - the `candidate_extractor_id_all_covers_variant_set` test asserting
     `len() == 20` → `21` (`agent_schema_context_profile.rs:95–101`);
   - the exhaustive `extractor_for` match arm + a new `*_EXTRACTOR` static
     (`candidate_generation.rs:462–484`);
   - the renamed canonical order constant (Deliverable 1) and the post-suppression
     phase tagging;
   - the relevant `GoalDispatchKey` schema declaration's `candidate_extractors`
     (`goal_schema.rs`);
   - `build_extractor_registry` coverage (`candidate_generation.rs:487`).

   The non-`match` `.insert(...)` use sites
   (`crates/worldwake-cli/src/scenario/mod.rs:4267`,
   `crates/worldwake-sim/src/save_load.rs:379`,
   `crates/worldwake-sim/src/per_agent_belief_view.rs:2350`) construct
   `disabled_extractors` sets and need **no** new arm.

3. **Provenance guard test (structural, keyed on `CandidateExtractorId`).** Add a
   test that proves no candidate is emitted outside the declared extractor
   pipeline. Mechanism (Q1 resolution — option (a), tightened against
   FOUNDATIONS):

   - The guard keys on `CandidateExtractorId` — the registry authority this spec
     consolidates — by capturing, **inside the generation pipeline**, the emitting
     `extractor_id` for each candidate (the loop already binds `extractor_id` at
     `candidate_generation.rs:768`; after Deliverable 2 the post-suppression phase
     binds it too). The capture lives in the transient `CandidateGenerationResult` /
     diagnostics, never promoted to authoritative state (FND-3/FND-27).
   - The test asserts: (i) **no untracked candidate** — every emitted candidate
     traces to a pipeline extractor (this is what actually falsifies a re-introduced
     out-of-band append; FND-31); and (ii) every contributing `extractor_id` is a
     member of the canonical order from Deliverable 1 (FND-20).
   - **Do not** add a `source_extractor` field to `GoalOffer`. Per-candidate
     provenance is already carried authoritatively by `EmitterTag` on the
     persisted `GoalOfferedPayload` (FND-29A); a second, non-isomorphic, transient
     provenance field would be a parallel representation of the same fact
     (FND-27/FND-28). The guard reads the registry identity already present in the
     pipeline instead.

## FND-01 Section H Analysis

Pure refactor; introduces no new system, state, component, action, or feedback
loop, and is required to be behavior-preserving. (One new `CandidateExtractorId`
variant is added in Deliverable 2, but it is a registry-identity entry on an
existing static policy enum, not new authoritative world/belief state.)

- **Information-path analysis:** Not applicable. No information path changes;
  candidate inputs (beliefs, memories, recipes, opportunities) are unchanged. The
  post-suppression phase consumes the same `fully_blocked_desires` the out-of-band
  call consumes today.
- **Positive-feedback analysis:** Not applicable. No amplifying loop.
- **Concrete dampeners:** Not applicable.
- **Stored-state vs. derived read-model list:** No authoritative state.
  `CandidateExtractorId` ordering is a static policy table; the
  `CandidateGenerationResult` (including any per-extractor provenance capture added
  for the Deliverable 3 guard) remains a transient derived computation (FND-3).
  The rename/folding/guard does not promote any derived value to truth, and does
  not add a second authoritative provenance surface beside `EmitterTag` (FND-27/28).
- **Planner-formalism analysis:** No formalism change. Candidate generation feeds
  ranking → portfolio → planning identically; only the *internal authority* for
  which/what-order extractors run is consolidated, and the blocked-self-care
  emitter moves from an out-of-band call to a declared post-suppression phase
  with no change to its inputs, gate, or output filtering. No goal becomes
  method-required.

### Proof surface (FND-31)

- All existing `worldwake-ai` goldens pass unchanged (behavior preservation is the
  primary regression guard).
- New: candidate-provenance guard test (Deliverable 3) — fails if any candidate is
  emitted outside the declared extractor pipeline (untracked candidate) or from an
  extractor absent from the canonical order.
- Updated: the existing ordering-completeness test is now
  `canonical_extractor_order_covers_every_registered_extractor_once` and is
  re-pointed at `CANDIDATE_EXTRACTOR_ORDER` — it fails if the canonical order and
  the schema-declared extractor set diverge (no orphan/missing extractor),
  including the later blocked-self-care variant once ticket 002 adds it.
