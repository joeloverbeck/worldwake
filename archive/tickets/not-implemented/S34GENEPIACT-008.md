# S34GENEPIACT-008: Reframe deliberate verification as a prerequisite barrier, not a competing top-level goal

**Status**: NOT IMPLEMENTED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No change landed under this ticket after reassessment
**Deps**: S34GENEPIACT-001 through S34GENEPIACT-007

## Problem

The ticket assumed the repo still lacked focused verification coverage and deliberate stale-prerequisite proof surfaces. That assumption is now stale. The live code still uses `GoalKind::VerifyBelief` as a standalone ranked goal, but the repo already contains focused `verify_belief` planner coverage, typed action-trace coverage, and a golden stale-prerequisite replan scenario.

That means this ticket is no longer a clean "implement the missing architecture" task. The proposed destination may still be architecturally preferable, but only if delivered as a single replacement of the standalone `VerifyBelief` authority path. Adding a second prerequisite-barrier path beside the existing top-level verification goal would violate the repo's no-alias/no-dual-path rule.

## Assumption Reassessment (2026-03-28)

1. The exact shared abstraction boundary under audit is the AI search/goal contract for stale evidence:
   - candidate emission and ranking of standalone `GoalKind::VerifyBelief` in `crates/worldwake-ai/src/candidate_generation.rs`, `crates/worldwake-ai/src/ranking.rs`, and `crates/worldwake-ai/src/goal_policy.rs`
   - planner/search handling of explicit epistemic terminals in `crates/worldwake-ai/src/goal_model.rs` and `crates/worldwake-ai/src/search/`
   - typed action identity in `crates/worldwake-sim/src/action_trace.rs`
   - committed epistemic actions in `crates/worldwake-systems/src/epistemic_actions.rs`
2. The live code still uses a competing top-level verification goal:
   - `emit_verify_belief_goals()` emits standalone `GoalKind::VerifyBelief` candidates from stale evidence dependencies in `crates/worldwake-ai/src/candidate_generation.rs`
   - `GoalKind::VerifyBelief` has its own ranking provenance and motive in `crates/worldwake-ai/src/ranking.rs`
   - `GoalKind::VerifyBelief` is still a separately suppressible goal family in `crates/worldwake-ai/src/goal_policy.rs`
3. The original ticket's coverage assumptions are wrong against the current repo:
   - focused candidate-generation tests already exist for low-confidence verification emission in `crates/worldwake-ai/src/candidate_generation.rs`
   - focused planner tests already exist for `Travel -> VerifyBelief` and `AskWitness` progress barriers in `crates/worldwake-ai/src/goal_model.rs`
   - typed trace tests already exist in `crates/worldwake-sim/src/action_trace.rs`
   - a golden stale-prerequisite replan scenario already exists in `crates/worldwake-ai/tests/golden_supply_chain.rs` as `golden_stale_prerequisite_belief_discovery_replan`
4. The live golden proof is not the same as the ticket's desired architecture:
   - `golden_stale_prerequisite_belief_discovery_replan` proves the originating `RestockCommodity` goal can recover from a stale prerequisite belief via travel, local discovery, and same-goal branch replacement
   - it does not prove that deliberate verification is modeled as a prerequisite barrier inside the originating goal path before that branch is spent
5. The ticket's proposed end state is only beneficial if it fully replaces the current authority path:
   - keeping standalone `VerifyBelief` candidate generation while also letting originating goals own epistemic barriers would create two live paths for the same contract
   - the repo explicitly forbids that kind of dual representation
6. The current AI search contract is broader to change than the ticket claims:
   - the live planner/search logic mostly reasons from `GoalKind`, while stale-evidence choice is grounded in `GroundedGoal.evidence_entities`
   - a clean replacement therefore needs a broader shared-contract refactor, not a small local patch
7. Corrected scope:
   - this ticket does not land code
   - this ticket records that the repo has already moved past its original assumptions and that a clean prerequisite-barrier redesign must be done as a broader one-shot replacement ticket, not as an incremental patch beside the current top-level `VerifyBelief` path

## Architecture Check

1. The proposed destination architecture is better than the current architecture only if it is delivered as a single canonical path:
   - originating world-condition goals own stale-belief verification as explicit `verify_belief` / `ask_witness` progress barriers
   - standalone top-level `VerifyBelief` candidate generation is removed in the same change
2. This ticket as written is not better than the current architecture:
   - it understates the amount of live proof already in the repo
   - it does not budget for removing the existing standalone `VerifyBelief` authority path
   - implementing it partially would create a cleaner-looking ticket but a dirtier architecture
3. The ideal long-term architecture is:
   - grounded-goal stale evidence drives epistemic barrier admission
   - explicit epistemic actions remain first-class and traceable
   - no separate top-level verification goal competes with the originating intention

## Verification Layers

1. standalone verification candidate emission still exists -> focused candidate-generation coverage in `crates/worldwake-ai/src/candidate_generation.rs`
2. explicit epistemic terminal planning still exists -> focused planner coverage in `crates/worldwake-ai/src/goal_model.rs`
3. typed committed `verify_belief` / `ask_witness` identity -> `crates/worldwake-sim/src/action_trace.rs`
4. originating-goal stale-prerequisite discovery and same-goal branch replacement -> `crates/worldwake-ai/tests/golden_supply_chain.rs`

## What Changed

1. Reassessed the ticket against the live repo and corrected its assumptions.
2. Determined that the ticket is stale as an implementation vehicle.
3. Archived it instead of forcing a partial architectural fork.

## Files Touched

- `tickets/S34GENEPIACT-008.md`

## Out of Scope

- changing production code under `worldwake-ai`, `worldwake-sim`, or `worldwake-systems`
- adding duplicate prerequisite-barrier behavior beside the existing standalone `VerifyBelief` path
- rewriting the live S34 epistemic design under this ticket without a broader replacement ticket

## Acceptance Outcome

1. Reassessment completed against current code, focused tests, and goldens.
2. Ticket assumptions and scope corrected to match the live repo.
3. Relevant tests and lint rerun successfully.
4. Ticket archived as `NOT IMPLEMENTED` because the clean architectural change requires a broader replacement than this ticket describes.

## Tests

### New/Modified Tests

1. None.
   Rationale: no production-code change was landed under this ticket; the work was ticket reassessment plus archival after confirming the repo already contains the focused and golden proof surfaces the ticket claimed were missing.

### Verification Commands

1. `cargo test -p worldwake-ai`
2. `cargo test -p worldwake-sim action_trace`
3. `cargo clippy -p worldwake-ai -p worldwake-sim --all-targets -- -D warnings`

## Outcome

- Date: 2026-03-28
- What actually changed: reassessed the ticket against live code/tests, corrected its assumptions, and archived it as stale/not implemented.
- Deviation from original plan: no code was changed because the repo already contains the focused and golden proof surfaces the ticket claimed were still missing, while the remaining architectural shift would require removing the standalone `VerifyBelief` path in one broader replacement change.
- Verification results: `cargo test -p worldwake-ai`, `cargo test -p worldwake-sim action_trace`, and `cargo clippy -p worldwake-ai -p worldwake-sim --all-targets -- -D warnings` all passed on 2026-03-28.
