# S159CANGENSCH-002: Fold blocked-self-care into registry as a post-suppression phase

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `CandidateExtractorId` enum (core), candidate-generation pipeline and extractor registry (AI)
**Deps**: S159CANGENSCH-001

## Problem

`emit_exploration_candidates_for_blocked_self_care` runs **out-of-band**
(`crates/worldwake-ai/src/candidate_generation.rs:798`), after the main extractor
loop and after the first suppression pass. It is a hidden candidate source
outside the declared extractor registry (FND-28; FND-20 emergence-via-declared-paths).

This ticket folds it into the declared registry as a blocked-self-care
`CandidateExtractorId` variant that runs inside the pipeline. Because the emitter
depends on **post-suppression** state (it consumes `diagnostics.fully_blocked_desires`
produced by the first `filter_suppressed_candidates` pass, gates on whether every
surviving candidate is a self-care fallback, and is itself suppression-filtered
in a separate pass), it must be modeled as a declared **post-suppression phase**
of the pipeline — not naively merged into the single pre-suppression loop, which
would change behavior. The emitted candidate set must remain identical.

## Assumption Reassessment (2026-05-21)

1. `emit_exploration_candidates_for_blocked_self_care` is defined at
   `crates/worldwake-ai/src/candidate_generation.rs:3779` and called at L798,
   after `filter_suppressed_candidates` (L788). It reads
   `diagnostics.fully_blocked_desires` (L795), early-returns unless every surviving
   candidate satisfies `goal_is_self_care_fallback` (L3786), and its output is run
   through a second `filter_suppressed_candidates` pass (L806) before being
   appended. The pre-suppression in-loop extractors (L768–786) read
   `prior_candidates: &candidates` (L778), which is the *unsuppressed*
   accumulation — confirming the fold cannot be a single-phase merge.
2. `CandidateExtractorId` is defined in `crates/worldwake-core/src/agent_schema_context_profile.rs:6`
   with `ALL: [Self; 20]` (L30) and is serialized via
   `AgentSchemaContextProfile.disabled_extractors: BTreeSet<CandidateExtractorId>`
   (L56). The only exhaustive match on the enum is `extractor_for`
   (`candidate_generation.rs:462–484`); other use sites (`scenario/mod.rs:4267`,
   `save_load.rs:379`, `per_agent_belief_view.rs:2350`) construct
   `disabled_extractors` sets via `.insert(...)` and need no new arm.
3. Mixed-layer boundary under audit: the AI candidate-generation pipeline
   (`generate_candidates_with_*` family in `candidate_generation.rs`) and the
   core registry-identity enum (`CandidateExtractorId`). The shared contract is
   `CANDIDATE_EXTRACTOR_ORDER` (renamed in ticket 001) plus the per-extractor
   `CandidateExtractor` trait (`candidate_generation.rs:269`).
4. Existing focused tests under modification/preservation:
   - `candidate_extractor_id_all_covers_variant_set`
     (`agent_schema_context_profile.rs:95`) asserts `ALL.len() == 20` — must be
     updated to 21.
   - `canonical_extractor_order_covers_every_registered_extractor_once`
     (renamed in ticket 001, `candidate_generation.rs`) — must be updated so the
     canonical order and `ALL` both include the new variant.
   - `fully_blocked_self_care_source_emits_exploration_fallback`
     (`candidate_generation.rs:11727`) exercises the blocked-self-care emitter —
     must still pass after the fold (behavior preservation).
5. Heuristic-removal discipline (precision rule 12): this ticket does not remove
   the blocked-self-care logic — it relocates the out-of-band call into a declared
   phase-2 extractor. The substrate it stands in for (post-suppression
   fully-blocked-desire detection) is preserved exactly; the change does not
   reopen unrelated regressions because the gate, input, and separate suppression
   pass are reproduced.
6. Adjacent-contradiction classification: the `EmitterTag` ↔ `CandidateExtractorId`
   non-isomorphism (separate provenance taxonomies) is a known smell explicitly
   deferred to a future spec per S159 Non-Goals; it is **not** in scope here.

## Architecture Check

1. Modeling blocked-self-care as a declared post-suppression phase makes the
   schema/registry the single authority for *all* candidate emission (FND-20),
   eliminating the last out-of-band source (FND-28). The phase concept is the
   minimal honest representation of the existing two-stage behavior — it does not
   invent new behavior, it names a structure that already exists implicitly.
2. The new variant is **appended** to `CandidateExtractorId` and `ALL` so bincode
   variant indices are preserved; existing serialized `disabled_extractors` sets
   continue to deserialize and never reference the new variant. No
   `SAVE_FORMAT_VERSION` bump is required.
3. No backward-compatibility shim: the out-of-band free-function call site is
   removed, not aliased. The free function may be retained as the phase-2
   extractor's `extract` body (relocated into the trait impl), but the out-of-band
   invocation in the pipeline is deleted.

## Verification Layers

1. Identical emitted candidate set (incl. blocked-self-care fallbacks) ->
   existing `worldwake-ai` goldens + `fully_blocked_self_care_source_emits_exploration_fallback`.
2. Blocked-self-care variant present in registry and canonical order ->
   `candidate_extractor_id_all_covers_variant_set` (21) and
   `canonical_extractor_order_covers_every_registered_extractor_once`.
3. Post-suppression phase runs after first suppression and before final result
   assembly, consuming `fully_blocked_desires` -> focused unit test asserting the
   phase-2 extractor receives the post-suppression desire set (decision-trace /
   focused runtime coverage, not a downstream golden proxy).
4. Single authoritative-state change is the candidate set only; no action
   lifecycle or event-log ordering changes, so no action-trace layer applies.

## What to Change

### 1. Add the blocked-self-care variant (core)

In `crates/worldwake-core/src/agent_schema_context_profile.rs`: append a new
variant (e.g., `BlockedSelfCareExploration`) to `CandidateExtractorId` (after
`OpportunityCompiler`) and to `ALL` (now `[Self; 21]`). Update
`candidate_extractor_id_all_covers_variant_set` to assert `len() == 21`.

### 2. Introduce a post-suppression phase in the canonical order (AI)

In `crates/worldwake-ai/src/candidate_generation.rs`: tag each entry in
`CANDIDATE_EXTRACTOR_ORDER` with a phase (pre-suppression / post-suppression),
or introduce a sibling `POST_SUPPRESSION_EXTRACTOR_ORDER` slice consumed after
the first `filter_suppressed_candidates` pass. Keep the existing 20 extractors in
the pre-suppression phase; the new variant is the sole post-suppression entry.
Update `canonical_extractor_order_covers_every_registered_extractor_once` so the
canonical order (across both phases) equals `CandidateExtractorId::ALL`.

### 3. Register the phase-2 extractor

Add the `extractor_for` match arm for the new variant and a corresponding
`*_EXTRACTOR` static implementing `CandidateExtractor`, with its `extract` body
relocated from `emit_exploration_candidates_for_blocked_self_care`. Ensure
`build_extractor_registry` covers it (it iterates `CandidateExtractorId::ALL`, so
coverage follows automatically once the match arm exists). The phase-2
`ExtractorContext` (or an equivalent declared input) must carry the
post-suppression `fully_blocked_desires`, and the pipeline must preserve the
all-surviving-candidates-are-self-care-fallback gate and the separate suppression
pass over the phase-2 output.

### 4. Remove the out-of-band call

Delete the out-of-band `emit_exploration_candidates_for_blocked_self_care(...)`
invocation in the pipeline (current L798) once the phase-2 extractor produces the
same candidates through the declared path.

### 5. Add the schema declaration

Add the new variant to the relevant `GoalDispatchKey` schema declaration's
`candidate_extractors` (`crates/worldwake-ai/src/goal_schema.rs`) so the
schema-membership filter admits it (matching the GoalDispatchKey whose family the
blocked-self-care fallback serves).

Merge note: No `SAVE_FORMAT_VERSION` bump (currently 96). The new
`CandidateExtractorId` variant is appended at the enum end, preserving bincode
variant indices; pre-existing serialized `disabled_extractors` sets never contain
it and deserialize unchanged.

## Files to Touch

- `crates/worldwake-core/src/agent_schema_context_profile.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/src/goal_schema.rs` (modify)

## Out of Scope

- The provenance guard test (ticket S159CANGENSCH-003).
- Unifying `EmitterTag` with `CandidateExtractorId` (S159 Non-Goal; future spec).
- Moving anomaly/observation interpretation out of candidate emission (S159 Non-Goal).
- Any change to the emitted candidate set, ranking, or plans.

## Acceptance Criteria

### Tests That Must Pass

1. `candidate_extractor_id_all_covers_variant_set` — asserts 21 variants.
2. `canonical_extractor_order_covers_every_registered_extractor_once` — canonical
   order across both phases equals `CandidateExtractorId::ALL` (21).
3. `fully_blocked_self_care_source_emits_exploration_fallback` — still passes via
   the declared phase-2 extractor.
4. New focused test: phase-2 extractor consumes the post-suppression
   `fully_blocked_desires` and reproduces the prior fallback candidates.
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No candidate is emitted outside the declared extractor pipeline (the
   out-of-band call is removed); enforced structurally and proven by ticket 003.
2. `CandidateExtractorId::ALL` and the canonical order remain in sync (no
   orphan/missing extractor).
3. Bincode variant indices for pre-existing `CandidateExtractorId` variants are
   unchanged (new variant appended); save/load round-trips of prior data succeed.
4. Emitted candidate set for every existing golden is unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/agent_schema_context_profile.rs` — update
   `candidate_extractor_id_all_covers_variant_set` to 21.
2. `crates/worldwake-ai/src/candidate_generation.rs` — update the canonical-order
   completeness test; keep `fully_blocked_self_care_source_emits_exploration_fallback`
   green; add a focused test that the phase-2 extractor receives the
   post-suppression desire set.

### Commands

1. `cargo test -p worldwake-ai fully_blocked_self_care_source_emits_exploration_fallback`
2. `cargo test -p worldwake-core candidate_extractor_id_all_covers_variant_set`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh`
