# S34GENEPIACT-014: Rename surviving epistemic barrier types after verify_belief removal

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` epistemic type renaming, downstream `worldwake-ai`/`worldwake-sim`/`worldwake-systems` call-site cleanup, S34 spec correction
**Deps**: [archive/tickets/completed/S34GENEPIACT-013.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-013.md), [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md)

## Problem

After S34GENEPIACT-013 removed `verify_belief`, the remaining cross-layer names are now misleading:

- `VerificationSubject`
- `VerificationDispositionProfile`
- `belief_verification_threshold`

Those names were reasonable when the architecture still modeled explicit verification as a first-class action. After removal, the remaining live semantics are narrower: stale-evidence gating, travel-side arrival refresh, and `ask_witness` social querying. Keeping verification-centric names would leave semantic fossils in the core contract even after the dead action path is gone.

## Assumption Reassessment (2026-03-28)

1. S34GENEPIACT-013 has landed and removed the dormant `verify_belief` action/path. The remaining names now overstate the breadth of the live contract rather than merely risking future drift.
2. The live shared abstraction boundary under audit is the cross-layer stale-subject identity and per-agent stale-evidence disposition profile currently defined in [epistemic.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/epistemic.rs), registered through the core component schema/table layer, and then consumed through AI planning/belief views plus the `ask_witness` runtime:
   - core identity/schema layer: `VerificationSubject`, `VerificationDispositionProfile`, `ComponentKind::VerificationDispositionProfile`, and related schema/table/world fixture surfaces in [epistemic.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/epistemic.rs), [component_schema.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_schema.rs), [component_tables.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/component_tables.rs), [delta.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/delta.rs), and [world.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world.rs)
   - AI planning layer: stale-subject extraction, barrier matching, and profile reads in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs), [planning_state.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs), [planning_snapshot.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs), and [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
   - sim/runtime read layer: verification-profile accessors and duration semantics in [belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs), [per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs), and [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs)
   - systems/runtime action layer: authoritative `ask_witness` validation and commit behavior in [epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs)
3. Existing focused coverage already proves these names as concrete serializable core contracts:
   - `verification_subject_entity_location_roundtrips_through_bincode`
   - `verification_subject_supply_availability_roundtrips_through_bincode`
   - `epistemic_disposition_profile_roundtrips_through_bincode`
   Those tests will need renaming and value-contract updates rather than silent removal.
4. Existing focused coverage is broader than the original ticket claimed. In addition to the core round-trip tests, the current repo already has:
   - focused AI proof surfaces for the live stale-barrier contract:
     - `goal_model::tests::grounded_goal_epistemic_subjects_extract_stale_subjects_from_originating_goal_evidence`
     - `goal_model::tests::grounded_goal_epistemic_barrier_matches_only_matching_payloads`
     - `goal_model::tests::search_restock_goal_returns_travel_barrier_for_remote_stale_source`
     - `goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload`
     - `candidate_generation::tests::low_confidence_evidence_keeps_originating_goal_without_standalone_epistemic_goal`
     - `candidate_generation::tests::stale_resource_source_stays_on_restock_goal_without_standalone_epistemic_goal`
   - focused systems/runtime `ask_witness` proof surfaces:
     - `epistemic_actions::tests::register_ask_witness_action_creates_expected_definition`
     - `epistemic_actions::tests::ask_witness_transfers_belief_with_report_provenance_and_records_memory`
     - `epistemic_actions::tests::ask_witness_noop_still_records_ask_memory`
     - `epistemic_actions::tests::ask_witness_affordance_suppresses_recent_reask`
     - `epistemic_actions::tests::ask_witness_rejects_payload_without_topic`
     - `epistemic_actions::tests::ask_witness_aborts_when_target_moves_before_commit`
     - `epistemic_actions::tests::ask_witness_aborts_when_target_dies_before_commit`
     - `epistemic_actions::tests::ask_witness_aborts_when_target_becomes_incapacitated_before_commit`
   - focused sim/runtime payload and trace proof surfaces:
     - `action_payload::tests::ask_witness_payload_roundtrips_through_bincode`
     - `action_payload::tests::ask_witness_entity_only_payload_roundtrips_through_bincode`
     - `action_trace::tests::detail_from_payload_extracts_ask_witness_identity`
     - `action_trace::tests::summary_includes_ask_witness_detail_when_present`
5. This is not an information-path behavior change by itself. It is a contract-language cleanup that should happen only after the dormant action path is removed. The canonical information path after S34GENEPIACT-013 remains unchanged: stale subject -> travel-side arrival refresh or `ask_witness`.
6. The live `GoalKind` and scenario surface remain the same as in S34GENEPIACT-013: `GoalKind::RestockCommodity { commodity: Bread }` with travel-side barrier and `AskWitness` focused proof surfaces. This ticket must not reopen planner semantics; it is a naming and contract-shape cleanup.
7. Because the renamed profile is a live component contract, this ticket necessarily owns the schema/table/world/delta rename fallout as well as AI/sim/systems call-site updates. The original file list was too narrow.
8. Coverage gap classification:
   - focused/unit coverage for the renamed core types exists and should be updated
   - focused AI/runtime coverage already exists and should mostly be renamed or kept green rather than replaced
   - no new golden/E2E coverage is inherently required if the rename is behavior-preserving
   - this is not a golden/E2E ticket
9. No heuristic/filter is being removed here. The missing substrate concern was addressed by S34GENEPIACT-012 and the dead-path removal by S34GENEPIACT-013. This ticket only ensures the surviving API names no longer misdescribe the architecture.
10. Adjacent contradiction classification:
   - confirmed consequence after S34GENEPIACT-013: the remaining names no longer honestly describe the contract and should be renamed
   - this ticket owns that rename cleanly
   - future cleanup beyond this ticket: only if a new explicit inspection action later appears, at which point names may need to widen again under a new ticket
11. Reassessment result: S34GENEPIACT-013 kept `VerificationSubject` and `VerificationDispositionProfile` only as temporary survivals during behavior removal, not because those names remained ideal. This ticket should stay open and perform the rename rather than close as churn.

## Architecture Check

1. Renaming after removal is cleaner than preserving verification-centric names for a non-verification architecture. It keeps the type layer honest and prevents future contributors from inferring a broader contract than the engine actually supports.
2. Separating this from S34GENEPIACT-013 is cleaner than mixing behavior removal and naming cleanup in one large patch. The first ticket removes the dead path; this ticket only lands if the remaining names are actually misleading after that change.
3. No backwards-compatibility type aliases should be introduced. If the names change, update all call sites and tests in-scope rather than leaving deprecated synonyms behind.

## Verification Layers

1. Renamed core epistemic contracts still round-trip through serde/bincode and remain component-safe -> focused `worldwake-core` unit tests
2. Renamed AI stale-subject extraction and barrier matching still preserve the travel-side and `AskWitness` contract -> focused `worldwake-ai` unit tests in `goal_model.rs` and `candidate_generation.rs`
3. Renamed systems/sim runtime surfaces still preserve `ask_witness` payload, trace, and authoritative behavior -> focused `worldwake-systems` `epistemic_actions` tests plus `worldwake-sim` `action_payload` and `action_trace` tests
4. Active specs/tickets no longer preserve the misleading verification-centric terminology -> targeted doc/ticket diff review
5. No behavior change is introduced beyond renaming and field-name cleanup -> unchanged focused AI golden/runtime tests from S34GENEPIACT-013 remain green
6. This is primarily a cross-layer contract ticket, so focused unit coverage plus compile/test verification is the right proof surface; no extra golden mapping is needed unless reassessment during implementation finds an accidental behavior change

## What to Change

### 1. Reassess the surviving names after S34GENEPIACT-013 lands

Adopt names that describe the surviving live semantics rather than the removed explicit verification action:

- `EpistemicSubject`
- `EpistemicDispositionProfile`
- `stale_evidence_barrier_threshold`

### 2. Rename the core contract without aliases

Update the defining types in `worldwake-core`, all downstream imports/usages, generated component accessors/variants, and the associated tests. Do not keep `type VerificationSubject = ...` or any similar compatibility alias.

### 3. Correct S34 docs and ticket references

Update the S34 spec and any still-pending tickets that refer to the old names so the documentation matches the surviving architecture after removal.

## Files to Touch

- `crates/worldwake-core/src/epistemic.rs` (modify — rename surviving core epistemic types/fields if reassessment confirms it is warranted)
- `crates/worldwake-core/src/lib.rs` (modify — update re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — update component registration strings/types)
- `crates/worldwake-core/src/component_tables.rs` (modify — update typed storage imports/usages)
- `crates/worldwake-core/src/delta.rs` (modify — update component/delta fixtures and enum variants if type names change)
- `crates/worldwake-core/src/world.rs` (modify — update world fixture/default helper usage and component accessors/tests)
- `crates/worldwake-ai/src/goal_model.rs` (modify — update type names)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — update profile/field names)
- `crates/worldwake-ai/src/planning_state.rs` (modify — update belief-view/profile access points)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — update stored snapshot/profile type names)
- `crates/worldwake-ai/src/ranking.rs` (modify — update profile type names used in test/support views)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — update witness-duration/profile field names used by action semantics)
- `crates/worldwake-sim/src/belief_view.rs` (modify — update belief-view/profile type names)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — update per-agent view type names)
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify — update surviving `ask_witness` and epistemic profile references)
- `specs/S34-general-epistemic-actions.md` (modify — correct the surviving terminology)
- `tickets/S34GENEPIACT-010.md` and any later pending S34 tickets (modify if needed — keep terminology consistent)

## Out of Scope

- removing `verify_belief` itself; that belongs to S34GENEPIACT-013
- introducing new epistemic behaviors or fact classes
- keeping transitional type aliases, deprecated wrappers, or duplicate field names

## Acceptance Criteria

### Tests That Must Pass

1. Focused core tests prove the renamed types/fields still serialize and behave as component-safe value contracts
2. Existing focused AI coverage for stale barriers still passes unchanged in behavior
3. Existing focused sim/systems `ask_witness` payload/trace/runtime coverage still passes unchanged in behavior
3. `cargo test -p worldwake-core`
4. `cargo test -p worldwake-ai`
5. `cargo test -p worldwake-systems epistemic_actions`
6. `cargo test -p worldwake-sim action_payload`
7. `cargo test -p worldwake-sim action_trace`
8. `cargo clippy -p worldwake-core -p worldwake-ai -p worldwake-sim -p worldwake-systems --all-targets -- -D warnings`

### Invariants

1. Surviving cross-layer epistemic names honestly describe the live architecture after `verify_belief` removal.
2. No compatibility aliases or duplicate names survive the rename.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/epistemic.rs` — rename and update the serializable/value-bound tests for the surviving epistemic core types
   Rationale: prove the renamed contract remains deterministic and serializable.
2. `crates/worldwake-ai/src/goal_model.rs` and `crates/worldwake-ai/src/candidate_generation.rs` — rename/update stale-subject extraction and barrier-matching tests only where symbol names appear
   Rationale: prove the AI contract is unchanged while the renamed core contract still binds the same stale-evidence behavior.
3. `crates/worldwake-systems/src/epistemic_actions.rs` — rename/update the existing `ask_witness` authoritative tests where profile/type names appear
   Rationale: prove the authoritative social-query contract still uses the renamed profile without behavior drift.
4. `crates/worldwake-sim/src/action_payload.rs` and `crates/worldwake-sim/src/action_trace.rs` — keep the existing `ask_witness` payload/trace tests green if renamed core terminology propagates into labels or fixtures
   Rationale: prove runtime payload/trace identity remains stable across the rename.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-systems epistemic_actions`
4. `cargo test -p worldwake-sim action_payload`
5. `cargo test -p worldwake-sim action_trace`
6. `cargo clippy -p worldwake-core -p worldwake-ai -p worldwake-sim -p worldwake-systems --all-targets -- -D warnings`

## Outcome

Completed: 2026-03-28

What actually changed:
- Renamed the surviving cross-layer epistemic contract from `VerificationSubject` / `VerificationDispositionProfile` / `belief_verification_threshold` to `EpistemicSubject` / `EpistemicDispositionProfile` / `stale_evidence_barrier_threshold`.
- Renamed the generated component/schema/world accessors and delta/component variants so the authoritative API no longer preserves verification-era naming fossils after `verify_belief` removal.
- Updated the downstream AI, sim, and systems call sites to the new names without changing the live planner/runtime behavior.
- Corrected the active S34 spec to use the surviving terminology.

Deviations from original plan:
- The live rename surface extended into generated component APIs, world fixtures, and delta variants, so the implementation touched a slightly broader core boundary than the original ticket first claimed.
- No pending S34 ticket beyond this one required terminology edits after reassessment; the active terminology drift lived in this ticket and the S34 spec.

Verification results:
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-systems epistemic_actions`
- Passed `cargo test -p worldwake-sim action_payload`
- Passed `cargo test -p worldwake-sim action_trace`
- Passed `cargo clippy -p worldwake-core -p worldwake-ai -p worldwake-sim -p worldwake-systems --all-targets -- -D warnings`
