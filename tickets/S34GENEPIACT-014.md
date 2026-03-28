# S34GENEPIACT-014: Rename surviving epistemic query types after verify_belief removal

**Status**: PENDING
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
2. The live shared abstraction boundary under audit is the cross-layer stale-subject identity and per-agent disposition profile currently defined in [epistemic.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/epistemic.rs) and re-exported through `worldwake-core`, then consumed in `worldwake-ai`, `worldwake-sim`, and `worldwake-systems`.
3. Existing focused coverage currently proves these names as concrete serializable core contracts:
   - `verification_subject_entity_location_roundtrips_through_bincode`
   - `verification_subject_supply_availability_roundtrips_through_bincode`
   - `verification_disposition_profile_roundtrips_through_bincode`
   Those tests will need renaming and value-contract updates rather than silent removal.
4. This is not an information-path behavior change by itself. It is a contract-language cleanup that should happen only after the dormant action path is removed. The canonical information path after S34GENEPIACT-013 remains unchanged: stale subject -> travel-side arrival refresh or `ask_witness`.
5. The live `GoalKind` and scenario surface remain the same as in S34GENEPIACT-013: `GoalKind::RestockCommodity { commodity: Bread }` with travel-side barrier and `AskWitness` focused proof surfaces. This ticket must not reopen planner semantics; it is a naming and contract-shape cleanup.
6. Coverage gap classification:
   - focused/unit coverage for the renamed core types exists and should be updated
   - no new runtime trace/integration or golden coverage is inherently required if the rename is behavior-preserving
   - this is not a golden/E2E ticket
7. No heuristic/filter is being removed here. The missing substrate concern was addressed by S34GENEPIACT-012 and the dead-path removal by S34GENEPIACT-013. This ticket only ensures the surviving API names no longer misdescribe the architecture.
8. Adjacent contradiction classification:
   - confirmed consequence after S34GENEPIACT-013: the remaining names no longer honestly describe the contract and should be renamed
   - this ticket owns that rename cleanly
   - future cleanup beyond this ticket: only if a new explicit inspection action later appears, at which point names may need to widen again under a new ticket
9. Reassessment result: S34GENEPIACT-013 kept `VerificationSubject` and `VerificationDispositionProfile` only as temporary survivals during behavior removal, not because those names remained ideal. This ticket should stay open and perform the rename rather than close as churn.

## Architecture Check

1. Renaming after removal is cleaner than preserving verification-centric names for a non-verification architecture. It keeps the type layer honest and prevents future contributors from inferring a broader contract than the engine actually supports.
2. Separating this from S34GENEPIACT-013 is cleaner than mixing behavior removal and naming cleanup in one large patch. The first ticket removes the dead path; this ticket only lands if the remaining names are actually misleading after that change.
3. No backwards-compatibility type aliases should be introduced. If the names change, update all call sites and tests in-scope rather than leaving deprecated synonyms behind.

## Verification Layers

1. Renamed core epistemic contracts still round-trip through serde/bincode and remain component-safe -> focused `worldwake-core` unit tests
2. Downstream AI/sim/systems code compiles and still passes existing focused behavior coverage -> focused crate tests plus package test runs
3. No behavior change is introduced beyond renaming and field-name cleanup -> unchanged focused AI golden/runtime tests from S34GENEPIACT-013 remain green
4. Active specs/tickets no longer preserve the misleading verification-centric terminology -> targeted doc/ticket diff review
5. This is primarily a cross-layer contract ticket, so focused unit coverage plus compile/test verification is the right proof surface; no extra golden mapping is needed unless reassessment during implementation finds an accidental behavior change

## What to Change

### 1. Reassess the surviving names after S34GENEPIACT-013 lands

Confirm whether the remaining contract is better described by names such as:

- `EpistemicQuerySubject`
- `EpistemicDispositionProfile`
- `epistemic_barrier_threshold`

The exact names may differ, but they must describe the surviving live semantics rather than the removed verification action. Reassessment after S34GENEPIACT-013 now strongly suggests that a rename is warranted rather than optional.

### 2. Rename the core contract without aliases

Update the defining types in `worldwake-core`, all downstream imports/usages, and the associated tests. Do not keep `type VerificationSubject = ...` or any similar compatibility alias.

### 3. Correct S34 docs and ticket references

Update the S34 spec and any still-pending tickets that refer to the old names so the documentation matches the surviving architecture after removal.

## Files to Touch

- `crates/worldwake-core/src/epistemic.rs` (modify — rename surviving core epistemic types/fields if reassessment confirms it is warranted)
- `crates/worldwake-core/src/lib.rs` (modify — update re-exports)
- `crates/worldwake-ai/src/goal_model.rs` (modify — update type names)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — update profile/field names)
- `crates/worldwake-ai/src/planning_state.rs` (modify — update belief-view/profile access points)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — update stored snapshot/profile type names)
- `crates/worldwake-ai/src/ranking.rs` (modify — update profile type names used in test/support views)
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
2. Existing focused AI/runtime coverage for stale barriers and `ask_witness` still passes unchanged in behavior
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
2. `None — behavior should remain covered by existing focused AI/runtime tests named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai`
3. `cargo test -p worldwake-systems epistemic_actions`
4. `cargo test -p worldwake-sim action_payload`
5. `cargo test -p worldwake-sim action_trace`
6. `cargo clippy -p worldwake-core -p worldwake-ai -p worldwake-sim -p worldwake-systems --all-targets -- -D warnings`
