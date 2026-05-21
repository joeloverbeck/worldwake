# S159CANGENSCH-001: Rename fossil ordering authority to canonical schema-owned order

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — candidate-generation extractor ordering (AI crate, internal; behavior-preserving)
**Deps**: None

## Problem

Before this ticket, candidate generation's extractor *execution order* was owned
by a constant literally named `LEGACY_EXTRACTOR_ORDER`
(`crates/worldwake-ai/src/candidate_generation.rs`).
The goal-schema consolidation was meant to make `GoalSchema.candidate_extractors`
the authority for which extractors run, but `ordered_candidate_extractors_from_goal_schemas`
only used the schema as a membership *filter* over that fossil constant. This was
a fossil authority name in a live path (FND-28). The existing
ordering-completeness test also hard-coded the phrase "preserve the legacy
top-level extractor order" in its assertion message.

This ticket renamed the constant to a canonical, non-"legacy" name and updated
the existing completeness test, with no behavioral change to the emitted
candidate set.

## Assumption Reassessment (2026-05-21)

1. At reassessment before implementation,
   `LEGACY_EXTRACTOR_ORDER: [CandidateExtractorId; 20]` existed at
   `crates/worldwake-ai/src/candidate_generation.rs:495` and was referenced at
   exactly two sites in that file (the definition at L495 and the use inside
   `ordered_candidate_extractors_from_goal_schemas` at L524). Workspace-wide grep
   confirms no other crate references the symbol — the rename blast radius is one
   file.
2. The spec deliverable D1 (`specs/S159-candidate-generation-schema-owned-extractor-authority.md`)
   offers two shapes; this ticket implements the recommended option (a): a canonical
   `CANDIDATE_EXTRACTOR_ORDER` constant. The spec's option (b) caveat (per-key
   schema lists collapse to a membership `BTreeSet`, so a total order is not
   recoverable without new ordering state) is the reason (a) is chosen.
3. Existing focused test under modification:
   `schema_derived_extractor_order_covers_every_registered_extractor_once`
   (`crates/worldwake-ai/src/candidate_generation.rs:17908`) asserts
   `ordered_candidate_extractors_from_goal_schemas() == CandidateExtractorId::ALL.to_vec()`
   and `build_extractor_registry().keys() == ALL`, with the assertion message
   "preserve the legacy top-level extractor order". This is the ordering-completeness
   invariant; it must be renamed, re-pointed at the canonical constant, and have
   the "legacy" wording dropped — not duplicated by a new test.
4. Mismatch + correction: none. The current order set in `LEGACY_EXTRACTOR_ORDER`
   is identical (members and order) to `CandidateExtractorId::ALL`, so the rename
   is a pure identifier change; the schema-membership filter currently filters
   nothing, and that remains true after the rename.

## Architecture Check

1. Renaming the constant to `CANDIDATE_EXTRACTOR_ORDER` removes a fossil
   authority name from a live path (FND-28) without introducing any shim, alias,
   or dual representation — the old name is deleted, not aliased.
2. Option (a) (canonical constant + completeness test) is cleaner than option (b)
   (ordering-on-schema) because the current schema surface cannot express a total
   order; (b) would require adding new ordering state, which contradicts the
   behavior-preserving, cleanup-only intent of this ticket.

## Verified Layers

1. Identical emitted candidate ordering after rename -> existing
   `worldwake-ai` goldens (behavior preservation is the regression guard).
2. Canonical order ≡ schema-declared extractor set (no orphan/missing member)
   -> renamed focused unit test
   `canonical_extractor_order_covers_every_registered_extractor_once`.
3. Single-layer (pure rename within the AI candidate-generation module); no
   action-trace or event-log layer is involved because no authoritative state or
   action lifecycle changes.

## Landed Changes

### 1. Renamed the ordering constant

Renamed `LEGACY_EXTRACTOR_ORDER` to `CANDIDATE_EXTRACTOR_ORDER` at its definition
and use site inside `ordered_candidate_extractors_from_goal_schemas`. Added a
doc-comment stating it is the single declared top-level execution order for
candidate extractors, and that membership must match the schema-declared set.

### 2. Updated the completeness test

Renamed `schema_derived_extractor_order_covers_every_registered_extractor_once`
to `canonical_extractor_order_covers_every_registered_extractor_once`, re-pointed
the expected order to `CANDIDATE_EXTRACTOR_ORDER`, and replaced the legacy
assertion message with canonical-order wording.

## Landed Files

- `crates/worldwake-ai/src/candidate_generation.rs`
- `specs/S159-candidate-generation-schema-owned-extractor-authority.md`
- `specs/IMPLEMENTATION-ORDER.md`

## Out of Scope

- Folding `emit_exploration_candidates_for_blocked_self_care` into the registry
  (ticket S159CANGENSCH-002).
- Adding the blocked-self-care `CandidateExtractorId` variant (ticket 002).
- The provenance guard test (ticket S159CANGENSCH-003).
- Any change to the emitted candidate set, ranking, or plans.

## Acceptance Result

### Verified Tests

1. `canonical_extractor_order_covers_every_registered_extractor_once` — passed
   with the renamed constant and updated message.
2. No Rust symbol named `LEGACY_EXTRACTOR_ORDER` remains in `crates/`; historical
   prose still mentions the former name where it documents the pre-ticket state.
3. Existing suite: `cargo test -p worldwake-ai` passed.

### Invariants

1. The canonical order constant's member set equals `CandidateExtractorId::ALL`
   (no orphan, no missing extractor).
2. The rename is behavior-preserving; the package suite and wrapper gate passed
   with no candidate-generation behavior edits beyond the identifier/test rename.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — renamed and re-pointed the
   existing completeness test; no new test file.

## Outcome

Completed on 2026-05-21.

- Replaced the live Rust ordering authority name with
  `CANDIDATE_EXTRACTOR_ORDER`; no alias or compatibility shim was left behind.
- Re-pointed the existing completeness test at the canonical constant and removed
  legacy wording from the assertion message.
- Truth-synced the active S159 spec and active implementation-order row so they
  describe ticket 001 as landed and leave the post-suppression extractor fold to
  ticket 002.

## Deviations

- The drafted zero-match criterion said no `LEGACY_EXTRACTOR_ORDER` mention would
  remain anywhere in the workspace. Live reassessment showed historical prose in
  specs, triage docs, and archived tickets still lawfully records the former
  symbol. The completed invariant is therefore scoped to live Rust symbols under
  `crates/`, while historical Markdown may keep pre-ticket evidence.

## Verification Result

- Passed `cargo test -p worldwake-ai canonical_extractor_order_covers_every_registered_extractor_once`
- Passed `cargo test -p worldwake-ai`
- Passed `./scripts/verify.sh`
