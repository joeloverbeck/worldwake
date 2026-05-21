# S159CANGENSCH-002: Fold blocked-self-care into registry as a post-suppression phase

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `CandidateExtractorId` enum (core), candidate-generation pipeline and extractor registry (AI)
**Deps**: archive/tickets/S159CANGENSCH-001.md

## Problem

`emit_exploration_candidates_for_blocked_self_care` runs **out-of-band**
(`crates/worldwake-ai/src/candidate_generation.rs:798`), after the main extractor
loop and after the first suppression pass. It is a hidden candidate source
outside the declared extractor registry (FND-28; FND-20 emergence-via-declared-paths).

This ticket folds it into the declared registry as a blocked-self-care
`CandidateExtractorId` variant that runs inside the pipeline. Because the emitter
depends on **post-suppression** state (it consumes `diagnostics.fully_blocked_desires`
produced by the first `filter_suppressed_candidates` pass, gated only over its
phase-local fallback-candidate vector in the pre-refactor live call shape, and is
itself suppression-filtered in a separate pass), it must be modeled as a declared
**post-suppression phase**
of the pipeline — not naively merged into the single pre-suppression loop, which
would change behavior. The emitted candidate set must remain identical.

## Assumption Reassessment (2026-05-21)

1. `emit_exploration_candidates_for_blocked_self_care` is defined at
   `crates/worldwake-ai/src/candidate_generation.rs:3779` and called at L798,
   after `filter_suppressed_candidates` (L788). It reads
   `diagnostics.fully_blocked_desires` (L795), but the live call passes a fresh
   empty `blocked_fallback_candidates` vector into the helper, so the helper's
   `goal_is_self_care_fallback` check at L3786 is phase-local and currently
   vacuous for the sole fallback emitter. Its output is run through a second
   `filter_suppressed_candidates` pass (L806) before being appended. The
   pre-suppression in-loop extractors (L768–786) read
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
7. Reassessment correction: the drafted "every surviving candidate is a
   self-care fallback" gate described a stronger behavior than live code proves.
   For FOUNDATIONS alignment, this behavior-preserving ticket keeps the live
   phase-local gate and creates a follow-up to investigate whether the stronger
   surviving-candidate gate should become a deliberate behavior change.

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

## Verified Layers

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

## Landed Changes

### 1. Added the blocked-self-care variant (core)

Added `BlockedSelfCareExploration` after `OpportunityCompiler` in
`CandidateExtractorId`, appended it to `CandidateExtractorId::ALL`, and updated
`candidate_extractor_id_all_covers_variant_set` to assert the 21-member set.

### 2. Introduced a post-suppression phase in the canonical order (AI)

Added `CandidateExtractorPhase`, mapped the first 20 extractors to
pre-suppression, mapped `BlockedSelfCareExploration` to post-suppression, and
kept `CANDIDATE_EXTRACTOR_ORDER` as the single 21-entry canonical order checked
against `CandidateExtractorId::ALL`.

### 3. Registered the phase-2 extractor

Added `BlockedSelfCareExplorationExtractor`, its static registry entry, and the
`extractor_for` match arm. `ExtractorContext` now carries
`fully_blocked_desires`, with pre-suppression extractors receiving an empty slice
and the post-suppression extractor receiving the cloned diagnostics from the
first suppression pass. The post-suppression phase preserves the live
phase-local fallback-candidate gate and the separate suppression pass over
phase-2 output.

### 4. Removed the out-of-band call

Deleted the direct out-of-band
`emit_exploration_candidates_for_blocked_self_care(...)` pipeline invocation.
Blocked-self-care fallback candidates now originate through the declared
post-suppression extractor path.

### 5. Added the schema declaration

Added `BlockedSelfCareExploration` to `GoalDispatchKey::ExploreLocation`'s
`candidate_extractors` declaration and updated the schema example test.

Merge note: No `SAVE_FORMAT_VERSION` bump (currently 96). The new
`CandidateExtractorId` variant is appended at the enum end, preserving bincode
variant indices; pre-existing serialized `disabled_extractors` sets never contain
it and deserialize unchanged.

## Landed Files

- `crates/worldwake-core/src/agent_schema_context_profile.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/goal_schema.rs`
- `archive/specs/S159-candidate-generation-schema-owned-extractor-authority.md`
- `archive/tickets/S159CANGENSCH-002.md`
- `archive/tickets/S159CANGENSCH-004.md`

## Out of Scope

- The provenance guard test (the now-archived
  `archive/tickets/S159CANGENSCH-003.md`).
- Unifying `EmitterTag` with `CandidateExtractorId` (S159 Non-Goal; future spec).
- Moving anomaly/observation interpretation out of candidate emission (S159 Non-Goal).
- Any change to the emitted candidate set, ranking, or plans.

## Acceptance Result

### Tests

1. `candidate_extractor_id_all_covers_variant_set` asserts 21 variants.
2. `canonical_extractor_order_covers_every_registered_extractor_once` asserts the
   canonical order across both phases equals `CandidateExtractorId::ALL`.
3. `fully_blocked_self_care_source_emits_exploration_fallback` still passes via
   the declared phase-2 extractor.
4. `blocked_self_care_phase_is_registry_gated_after_suppression` proves the
   phase-2 fallback is controlled by the declared extractor registry after
   `fully_blocked_desires` is produced.
5. `cargo test -p worldwake-ai` passed.

### Invariants

1. No blocked-self-care candidate is emitted outside the declared extractor
   pipeline (the out-of-band call is removed); full no-untracked-candidate guard
   later landed in `archive/tickets/S159CANGENSCH-003.md`.
2. `CandidateExtractorId::ALL` and the canonical order remain in sync (no
   orphan/missing extractor).
3. Bincode variant indices for pre-existing `CandidateExtractorId` variants are
   unchanged (new variant appended); save/load round-trips of prior data succeed.
4. Emitted candidate set for every existing golden is unchanged.

## Test Plan Result

### Modified Tests

1. `crates/worldwake-core/src/agent_schema_context_profile.rs` — updated
   `candidate_extractor_id_all_covers_variant_set` to 21.
2. `crates/worldwake-ai/src/candidate_generation.rs` — updated the
   canonical-order completeness test, preserved
   `fully_blocked_self_care_source_emits_exploration_fallback`, and added
   `blocked_self_care_phase_is_registry_gated_after_suppression`.

### Command Status

1. `cargo test -p worldwake-ai fully_blocked_self_care_source_emits_exploration_fallback`
2. `cargo test -p worldwake-core candidate_extractor_id_all_covers_variant_set`
3. `cargo test -p worldwake-ai`
4. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-21.

- Added `CandidateExtractorId::BlockedSelfCareExploration` as the declared
  registry identity for blocked-self-care fallback emission.
- Split candidate extraction into pre-suppression and post-suppression phases,
  with blocked-self-care as the sole post-suppression extractor.
- Removed the out-of-band blocked-self-care fallback call from the candidate
  pipeline while preserving the live phase-local gate and separate suppression
  pass.
- Added the now-archived `archive/tickets/S159CANGENSCH-004.md` to investigate
  whether the phase-local gate should become a non-vacuous surviving-candidate
  gate as a deliberate behavior change.

## Deviations

- The draft said the helper gated on every surviving candidate being self-care
  fallback. Live reassessment showed the old call passed an empty fallback vector,
  so this ticket preserved that phase-local gate for behavior preservation and
  split the stronger gate question to `archive/tickets/S159CANGENSCH-004.md`.
- The no-untracked-candidate provenance guard remains out of scope for this
  ticket and later landed in `archive/tickets/S159CANGENSCH-003.md`.

## Verification Result

- Passed `cargo test -p worldwake-core candidate_extractor_id_all_covers_variant_set`
- Passed `cargo test -p worldwake-ai canonical_extractor_order_covers_every_registered_extractor_once`
- Passed `cargo test -p worldwake-ai fully_blocked_self_care_source_emits_exploration_fallback`
- Passed `cargo test -p worldwake-ai blocked_self_care_phase_is_registry_gated_after_suppression`
- Passed `cargo test -p worldwake-ai`
- Passed `./scripts/verify.sh`
