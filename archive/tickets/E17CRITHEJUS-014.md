# E17CRITHEJUS-014: Workspace verification, closeout, and archival

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: `specs/E17-crime-theft-justice.md`, archived E17 implementation/golden closeout tickets under `archive/tickets/`

## Problem

E17 implementation is already present in the codebase, but the final closeout artifacts are not aligned with the live repository state. The active ticket still assumes a verification-only scope, `specs/IMPLEMENTATION-ORDER.md` still lists E17 as active, the generated/manual golden docs do not fully match the shipped E17 scenario inventory, and the E17 spec itself is still active instead of archived. This ticket closes that loop cleanly.

## Assumption Reassessment (2026-03-27)

1. `specs/IMPLEMENTATION-ORDER.md` is still the live planning authority and still lists [`E17-crime-theft-justice.md`](/home/joeloverbeck/projects/worldwake/specs/E17-crime-theft-justice.md) as an active Step 13 spec even though the code and focused/golden coverage for theft, accusation, punishment, and crime ranking already exist.
2. The active E17 spec assumptions are materially realized in live code. Current symbols include `worldwake_core::crime::{TheftDispositionProfile, JusticeDispositionProfile, PunishmentKind}`, `GoalKind::{StealItem, Accuse, PunishAccused}`, `RecordKind::CrimeRegister`, and `InstitutionalClaim::{Accusation, Verdict}` across [`crates/worldwake-core/src/crime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/crime.rs), [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs), and [`crates/worldwake-systems/src/justice_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/justice_actions.rs).
3. Live focused and golden coverage already exists, so this is not a code-feature implementation ticket. Verified live test names include `candidate_generation::tests::local_owned_item_emits_theft_goal`, `ranking::tests::crime_goals_use_profile_driven_motive_scores`, `planner_conformance::conformance_accuse`, `golden_theft_leads_owner_to_local_suspected_theft_discovery`, `golden_witnessed_theft_accusation_chain`, `golden_traceability_explains_stale_fine_branch_without_source_diving`, and `golden_materialized_output_ownership_prevents_theft`.
4. Shared abstraction boundary under audit: E17 closeout documentation connecting source-declared golden scenario metadata, generated golden docs, and implementation-order/spec archival state. This ticket should not reopen the runtime theft/justice architecture unless reassessment finds the closeout docs are lying about shipped behavior.
5. The current active ticket scope is stale. It says “verification-only” and explicitly excludes archiving the E17 spec, but the user-requested closeout and the archival workflow require ticket/spec archival once verification succeeds. The scope must therefore include archival.
6. Generated golden docs already exist at [`docs/generated/golden-e2e-inventory.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-e2e-inventory.md) and [`docs/generated/golden-scenario-map.md`](/home/joeloverbeck/projects/worldwake/docs/generated/golden-scenario-map.md). Reassessment found a live mismatch between source-declared scenario metadata in [`crates/worldwake-ai/tests/golden_emergent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) and the generated/manual docs: Scenario 38 is now the witnessed-theft accusation chain, Scenario 39 is the stale-fine traceability proof, and the supply-depletion scenario must move to a later identifier instead of reusing 39.
7. The original dependency note is imprecise. The relevant follow-up ticket `E17CRITHEJUS-021` exists, but only in the archive at [`archive/tickets/E17CRITHEJUS-021.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/E17CRITHEJUS-021.md), not in `tickets/`. This ticket should cite archived dependencies explicitly rather than implying missing active tickets.
8. Architectural reassessment of the proposed closeout work: updating the source-declared golden metadata and generated docs is more beneficial than leaving the stale documentation in place because the project treats golden inventories and scenario maps as debugging/authoring contracts, not optional commentary. The clean architecture is one authoritative source declaration per scenario and regenerated docs derived from it, not hand-maintained parallel narratives.
9. Phase 3 remains in progress even if E17 is archived. `S13` is still active and the Phase 3 gate still lists unresolved items such as `T10`, `T11`, and `T25`. This ticket must mark E17 complete without falsely claiming the whole phase gate is closed.
10. Separate follow-up note: aggregate lawful-control helper alignment remains tracked in archived [`archive/tickets/completed/CTRLAGG-001-align-aggregate-control-helpers.md`](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/CTRLAGG-001-align-aggregate-control-helpers.md). This closeout ticket should not silently absorb that distinct control-contract work.
11. Mismatch + correction: the prior ticket referred to nonexistent active follow-up paths (`tickets/E17CRITHEJUS-021.md`, `tickets/S27INVPROF-001.md`, `tickets/CTRLAGG-001-align-aggregate-control-helpers.md`). The corrected scope cites either archived paths or drops those references when they are not needed for E17 closeout.
12. No new engine architecture is proposed here. The architectural judgment is that the current E17 runtime design is cleaner than the pre-E17 architecture because theft, evidence, accusation, and punishment now travel through concrete stateful carriers: possession/ownership state, `ViolationMemory`, `SocialObservationDetail::SuspectedTheft`, `CrimeRegister` record entries, and punishment actions. That is better than any hypothetical shortcut path that would fuse crime discovery or punishment into direct world queries.

## Architecture Check

1. The clean closeout is to make the repository say exactly what the code already does: one active-planning source (`specs/IMPLEMENTATION-ORDER.md`), one active spec inventory, source-declared golden scenario metadata in test files, generated golden docs derived from that metadata, and archived completed tickets/specs with explicit outcomes. Leaving E17 half-active in planning docs while the implementation already shipped is stale state, not extensibility.
2. Regenerating golden docs from corrected source annotations is cleaner than hand-editing generated files or tolerating duplicate scenario IDs. It preserves a single canonical source for scenario metadata.
3. No backwards-compatibility aliasing or parallel “active but actually complete” documentation paths should remain after this ticket.

## Verification Layers

1. Source-declared E17 scenario numbering and titles match the intended golden scenarios -> focused review of [`crates/worldwake-ai/tests/golden_emergent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) plus regenerated golden docs
2. E17 golden behaviors still execute on the live runtime -> focused golden tests in `golden_emergent.rs` and `golden_production.rs`
3. Workspace closeout remains regression-safe -> `cargo test --workspace`
4. Lint/build health for the archived closeout state -> `cargo clippy --workspace --all-targets -- -D warnings` and `cargo build --workspace`
5. Planning/archive state is truthful -> doc diff plus archival move under `archive/tickets/` and `archive/specs/`

## What to Change

### 1. Run full workspace verification

- Run the relevant focused E17 golden tests first, then full workspace verification.
- Use the archival workflow only after the verification state is green.

### 2. Update `specs/IMPLEMENTATION-ORDER.md`

- Mark E17 as completed in Step 13 and the Phase 3 summary while keeping Phase 3 itself in progress.
- Remove the E17 spec from the active spec inventory once the spec is archived.
- Do not falsely check unresolved Phase 3 gate items such as `T25` without live proof.

### 3. Reconcile golden source metadata and golden docs

- Correct the duplicate/stale E17-adjacent scenario numbering in `golden_emergent.rs`.
- Regenerate the generated golden docs from source declarations.
- Update any hand-maintained golden coverage notes that still point to the pre-closeout numbering.

### 4. Archive the completed E17 artifacts

- Mark this ticket completed with an `Outcome` section.
- Mark the E17 spec completed with an `Outcome` section.
- Move both into the appropriate `archive/` directories.

## Files to Touch

- `tickets/E17CRITHEJUS-014.md` (modify, then archive)
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `specs/IMPLEMENTATION-ORDER.md` (modify)
- `docs/golden-e2e-coverage.md` (modify)
- `docs/generated/golden-e2e-inventory.md` (regenerate)
- `docs/generated/golden-scenario-map.md` (regenerate)
- `specs/E17-crime-theft-justice.md` (modify, then archive)

## Out of Scope

- New theft/justice runtime features
- Reopening already-shipped E17 architecture unless verification proves it broken
- Starting E19 or other E17-dependent work
- Phase 3 gate sign-off for non-E17 items (for example `OmniscientBeliefView` closeout, `T10`, `T11`, `T25`)

## Acceptance Criteria

### Tests That Must Pass

1. Focused E17 goldens and conformance/ownership checks named in the test plan pass
2. `cargo test --workspace` — zero failures
3. `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings
4. `cargo build --workspace` — clean build

### Invariants

1. Source-declared golden scenario metadata and generated docs agree on the shipped E17 scenario inventory
2. `specs/IMPLEMENTATION-ORDER.md` accurately reflects E17 completion without overstating overall Phase 3 gate closure
3. The E17 ticket and E17 spec are archived with accurate outcome summaries
4. No backwards-compatibility aliasing or duplicate documentation paths are introduced during closeout

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — update source-declared scenario numbering so the generated scenario map reflects the shipped witnessed-theft and stale-fine E17 proofs accurately.
2. `None` — no new runtime behavior tests are added in this closeout ticket because the live E17 behavior is already covered; verification relies on the existing focused/golden tests named below.

### Commands

1. `cargo test -p worldwake-ai --test golden_emergent golden_theft_leads_owner_to_local_suspected_theft_discovery -- --exact`
2. `cargo test -p worldwake-ai --test golden_emergent golden_witnessed_theft_accusation_chain -- --exact`
3. `cargo test -p worldwake-ai --test golden_emergent golden_traceability_explains_stale_fine_branch_without_source_diving -- --exact`
4. `cargo test -p worldwake-ai --test golden_production golden_materialized_output_ownership_prevents_theft -- --exact`
5. `python3 scripts/golden_inventory.py --write --check-docs`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `cargo build --workspace`

## Outcome

- Completed: 2026-03-27
- What actually changed:
  - Corrected the ticket scope from stale verification-only wording to real E17 closeout plus archival.
  - Updated [`crates/worldwake-ai/tests/golden_emergent.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) so the source-declared E17 scenario metadata matches the shipped witnessed-theft, stale-fine, and supply-depletion scenario layout.
  - Regenerated the golden inventory/docs from source and updated [`docs/golden-e2e-coverage.md`](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) plus [`specs/IMPLEMENTATION-ORDER.md`](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md) to reflect the completed E17 state.
  - Added narrow `#[allow(clippy::result_large_err)]` annotations to three `worldwake-systems` integration-test harness helpers and normalized three `golden_emergent.rs` assertion format strings so workspace clippy passes under `-D warnings`.
- Deviations from original plan:
  - Reassessment showed this could not remain a documentation-only verification ticket because the repository still needed active/archived state correction and generator-aligned scenario metadata cleanup.
  - Verification also exposed pre-existing lint blockers in test harness code, so the closeout included minimal test-only lint remediation even though no runtime theft/justice behavior changed.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_emergent golden_theft_leads_owner_to_local_suspected_theft_discovery -- --exact`
  - `cargo test -p worldwake-ai --test golden_emergent golden_witnessed_theft_accusation_chain -- --exact`
  - `cargo test -p worldwake-ai --test golden_emergent golden_traceability_explains_stale_fine_branch_without_source_diving -- --exact`
  - `cargo test -p worldwake-ai --test golden_production golden_materialized_output_ownership_prevents_theft -- --exact`
  - `cargo test -p worldwake-systems --test e09_needs_integration --test e10_production_transport_integration --test e12_combat_integration`
  - `python3 scripts/golden_inventory.py --write --check-docs`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo build --workspace`
