# S159CANGENSCH-001: Rename fossil ordering authority to canonical schema-owned order

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — candidate-generation extractor ordering (AI crate, internal; behavior-preserving)
**Deps**: None

## Problem

Candidate generation's extractor *execution order* is owned by a constant
literally named `LEGACY_EXTRACTOR_ORDER` (`crates/worldwake-ai/src/candidate_generation.rs:495`).
The goal-schema consolidation was meant to make `GoalSchema.candidate_extractors`
the authority for which extractors run, but `ordered_candidate_extractors_from_goal_schemas`
only uses the schema as a membership *filter* over `LEGACY_EXTRACTOR_ORDER` — the
fossil constant still owns the ordering. This is a fossil authority name in a
live path (FND-28). The existing ordering-completeness test even hard-codes the
phrase "preserve the legacy top-level extractor order" in its assertion message.

This ticket renames the constant to a canonical, non-"legacy" name and updates
the existing completeness test, with no behavioral change to the emitted
candidate set.

## Assumption Reassessment (2026-05-21)

1. `LEGACY_EXTRACTOR_ORDER: [CandidateExtractorId; 20]` exists at
   `crates/worldwake-ai/src/candidate_generation.rs:495` and is referenced at
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

## Verification Layers

1. Identical emitted candidate ordering after rename -> existing
   `worldwake-ai` goldens (behavior preservation is the regression guard).
2. Canonical order ≡ schema-declared extractor set (no orphan/missing member)
   -> renamed focused unit test
   `canonical_extractor_order_covers_every_registered_extractor_once`.
3. Single-layer (pure rename within the AI candidate-generation module); no
   action-trace or event-log layer is involved because no authoritative state or
   action lifecycle changes.

## What to Change

### 1. Rename the ordering constant

Rename `LEGACY_EXTRACTOR_ORDER` → `CANDIDATE_EXTRACTOR_ORDER` at its definition
(`candidate_generation.rs:495`) and its use site inside
`ordered_candidate_extractors_from_goal_schemas` (L524). Add a doc-comment on the
constant stating it is the single declared top-level execution order for
candidate extractors, and that membership must match the schema-declared set
(asserted by the completeness test).

### 2. Update the completeness test

Rename `schema_derived_extractor_order_covers_every_registered_extractor_once`
to `canonical_extractor_order_covers_every_registered_extractor_once`, re-point
any reference from the old constant name to `CANDIDATE_EXTRACTOR_ORDER`, and
replace the "preserve the legacy top-level extractor order" assertion message
with non-"legacy" wording (e.g., "canonical top-level extractor order matches the
schema-declared set").

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)

## Out of Scope

- Folding `emit_exploration_candidates_for_blocked_self_care` into the registry
  (ticket S159CANGENSCH-002).
- Adding the blocked-self-care `CandidateExtractorId` variant (ticket 002).
- The provenance guard test (ticket S159CANGENSCH-003).
- Any change to the emitted candidate set, ranking, or plans.

## Acceptance Criteria

### Tests That Must Pass

1. `canonical_extractor_order_covers_every_registered_extractor_once` — passes
   with the renamed constant and updated message.
2. No symbol named `LEGACY_EXTRACTOR_ORDER` remains anywhere in the workspace.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. The canonical order constant's member set equals `CandidateExtractorId::ALL`
   (no orphan, no missing extractor).
2. The emitted candidate set for every existing golden is byte-identical to
   pre-rename behavior (the rename is purely an identifier change).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — rename and re-point the
   existing completeness test; no new test file.

### Commands

1. `cargo test -p worldwake-ai canonical_extractor_order_covers_every_registered_extractor_once`
2. `cargo test -p worldwake-ai`
3. `./scripts/verify.sh`
