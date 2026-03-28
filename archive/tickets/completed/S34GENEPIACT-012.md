# S34GENEPIACT-012: Remove duplicate epistemic paths for arrival-observable stale facts

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai` grounded-goal/search barrier classification, optional `worldwake-systems` de-scoping only if removal of planner exposure requires it, S34 spec/ticket follow-up correction
**Deps**: [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md), [archive/tickets/completed/S34GENEPIACT-011.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-011.md), [tickets/S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md)

## Problem

After S34GENEPIACT-011, the planner correctly treats stale evidence as an originating-goal epistemic barrier, but some stale facts still have two lawful refresh paths:

- travel to the place and passively refresh the belief through ordinary co-location perception
- travel to the place and then commit `verify_belief` for the same already-arrival-observable fact

That duplicate path is architecturally weaker than the foundations require. It turns `verify_belief` into decorative realism for facts that lawful local perception already reveals on arrival, and it weakens the causal meaning of duration-bearing epistemic actions.

## Assumption Reassessment (2026-03-28)

1. The live stale-evidence barrier substrate now lives in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) via `grounded_goal_epistemic_subjects()` and `grounded_goal_matches_epistemic_barrier()`, and search consumes it in [search/candidates.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/candidates.rs) and [search/transition.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/transition.rs).
2. The live `VerificationSubject` surface in [epistemic.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/epistemic.rs) still only distinguishes `EntityLocation` and `SupplyAvailability`. No current live variant proves a fact class that requires deliberate post-arrival inspection beyond ordinary co-location perception.
3. The exact shared abstraction boundary under audit is mixed-layer and centered on the epistemic fact-refresh contract:
   - planner/search layer: grounded stale-evidence barrier selection in `worldwake-ai`
   - authoritative observation layer: passive local belief refresh via perception
   - authoritative action layer: explicit `verify_belief` / `ask_witness` commits in [epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs)
4. The motivating stale-source scenario’s live `GoalKind` remains `GoalKind::RestockCommodity { commodity: Bread }`, and the final focused planner surface is `goal_model::tests::search_restock_goal_returns_travel_barrier_for_remote_stale_source`.
5. The current golden regression `golden_stale_prerequisite_belief_discovery_replan` in [golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) already shows the adjacent lawful branch: after travel arrival, same-place passive perception can refresh the stale source belief before a `verify_belief` commit is required.
6. This is an information-path refactor. The same fact currently has multiple lawful transport paths:
   - path A: `Travel` -> ordinary local perception refresh on arrival
   - path B: `Travel` -> `VerifyBelief` action commit
   The canonical end state after this ticket should be one path per fact class:
   - arrival-observable facts refresh through `Travel` plus ordinary local perception
   - deliberate epistemic actions remain only for facts that require targeted inspection or social querying
7. The foundations favor that split:
   - [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) Principle 7 and Principle 13 require knowledge to be acquired locally and lawfully when co-located
   - Principle 5 forbids decorative subsystems that do not add downstream consequence
   - Principle 8 requires explicit actions to justify their duration/cost/occupancy with real consequential work
8. The live `verify_belief` handler in [epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs) still refreshes beliefs for both `EntityLocation` and `SupplyAvailability`, even though those same facts already have lawful arrival-side refresh in the current runtime.
9. Ordering-sensitive claims here are mixed-layer and must stay separated:
   - whether search inserts `VerifyBelief` or stops at `Travel` -> focused planner/search tests
   - whether arrival perception refreshes the fact -> authoritative belief-state/runtime tests
   - whether a deliberate epistemic action still exists and commits for non-arrival-observable facts -> action trace plus authoritative aftermath
10. Adjacent contradiction classification:
   - required consequence of this ticket: remove duplicate `verify_belief` use from the live AI/search contract for arrival-observable facts
   - corrected scope: do not invent a hidden or inspection-only fact class to justify the existing path
   - future cleanup if needed: either remove any now-dormant lower-layer `verify_belief` substrate or introduce a real inspection-only fact class with a distinct authoritative contract
11. Mismatch + correction: the pending [S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md) still assumes deliberate `VerifyBelief` goldens should cover stale entity-location and stale supply-source refresh generically. After reassessment, that scope is too broad unless those facts are first proven to require targeted inspection rather than lawful arrival perception.
12. This ticket should not "fix" the duplication by suppressing passive co-location refresh while a verification barrier is active. That would violate the foundations more directly by hiding lawful local observation until an extra action finishes.

## Architecture Check

1. The clean design is fact-sensitive barrier selection, not runtime perception suppression. If arrival lawfully reveals the fact, `Travel` to the believed place is the epistemic barrier and re-evaluation happens after arrival. If the fact requires a social information carrier, `AskWitness` remains the explicit barrier action.
2. This is cleaner than forcing `verify_belief` to commit before passive observation, because that would make local perception less truthful in order to preserve a planner artifact.
3. This is cleaner than leaving both paths live, because one fact should not have two canonical refresh mechanisms with different costs unless the world explicitly models a difference between them.
4. Current reassessment does not justify preserving `VerifyBelief` in the active AI contract for `EntityLocation` or `SupplyAvailability`. If the lower-layer action remains temporarily for non-planner reasons, it must no longer define the live canonical path for these facts.

## Verification Layers

1. arrival-observable stale facts no longer synthesize `VerifyBelief`; the barrier is the travel step to the believed place -> focused planner/search tests in `worldwake-ai`
2. arrival-observable stale facts still refresh lawfully on co-location -> authoritative belief-state/runtime tests and existing golden recovery coverage
3. deliberate epistemic actions remain explicit only where they still add distinct causal work; today that is at least `AskWitness`, and any surviving `VerifyBelief` usage must be justified separately -> focused AI/runtime tests plus action trace where applicable
4. stale-source recovery remains explainable without overclaiming action commits -> updated golden and decision-trace-facing tests in `worldwake-ai`
5. if trace surfaces still cannot explain why a fact is classified as arrival-observable versus inspection-only, add a follow-up traceability ticket instead of weakening lower-layer proof

## What to Change

### 1. Add a fact-observability classification to the epistemic barrier substrate

Refine the `worldwake-ai` stale-evidence substrate so it distinguishes:

- facts refreshable by ordinary lawful arrival perception
- facts refreshable through `ask_witness`

The exact type name can vary, but it must be grounded-goal/search-local and explainable from current evidence, anchor, and belief reads rather than by reintroducing a top-level goal family.

### 2. Stop synthesizing `verify_belief` for arrival-observable facts

For arrival-observable facts such as the currently modeled stale location/source-presence cases, search should stop at `Travel` to the believed place and replan after arrival refresh rather than insisting on a post-arrival `VerifyBelief` action.

If the current `VerificationSubject` variants are too coarse to express that split, refine or replace the AI-local barrier substrate rather than preserving the coarse dual-path behavior.

### 3. Narrow `verify_belief` to work that still carries distinct consequence

Keep `verify_belief` only where it still adds real causal work relative to passive observation. Current reassessment does not prove any such fact class. Remove or de-scope the action from the affected planner path instead of preserving it as a decorative extra step.

### 4. Correct S34 follow-up documents to the new contract

Update [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md) and any still-pending follow-up tickets that generically require `VerifyBelief` for arrival-observable stale facts.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — refine grounded-goal epistemic barrier classification)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — stop surfacing `VerifyBelief` for arrival-observable facts)
- `crates/worldwake-ai/src/search/transition.rs` (modify — treat travel-to-believed-place as the progress barrier for arrival-observable stale facts)
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify only if needed — de-scope planner-facing `verify_belief` exposure rather than preserving it as a canonical refresh path)
- `crates/worldwake-ai/tests/golden_supply_chain.rs` (modify — keep stale-source recovery aligned with the canonical path)
- `specs/S34-general-epistemic-actions.md` (modify — correct the fact-sensitive epistemic contract)
- `tickets/S34GENEPIACT-010.md` or a replacement follow-up ticket (modify — correct stale golden assumptions if still pending)

## Out of Scope

- inventing hidden/inspection-only fact classes with no current world substrate
- broad perception-system rewrites unrelated to the stale epistemic barrier contract
- suppressing lawful co-location perception to make `verify_belief` look more necessary
- unrelated social/tell/investigation refactors

## Acceptance Criteria

### Tests That Must Pass

1. Focused planner coverage proves arrival-observable stale facts now terminate at travel-to-place rather than `Travel -> VerifyBelief`
2. Focused planner/runtime coverage proves originating goals still re-evaluate correctly after arrival refresh
3. Any remaining `verify_belief` path is proven to correspond to a fact class not already lawfully satisfied by arrival perception, or else `verify_belief` is fully de-scoped from the affected planner path
4. Existing stale-prerequisite recovery coverage still passes under the corrected contract
5. `cargo test -p worldwake-ai`
6. `cargo test -p worldwake-systems epistemic_actions`
7. `cargo clippy -p worldwake-ai -p worldwake-systems --all-targets -- -D warnings`

### Invariants

1. A single fact has one canonical refresh path under the live architecture.
2. Passive lawful co-location perception is not suppressed to preserve planner expectations.
3. Explicit epistemic actions remain only when they add distinct cost-bearing causal work beyond ordinary arrival perception.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — replace the current remote stale-source `Travel -> VerifyBelief` assertion with the corrected travel-as-barrier contract.
2. `crates/worldwake-ai/tests/golden_supply_chain.rs` — strengthen the stale-source recovery regression around arrival refresh as the canonical path for arrival-observable facts.
3. `crates/worldwake-systems/src/epistemic_actions.rs` — change focused tests only if planner-facing `verify_belief` exposure is explicitly de-scoped there; otherwise keep authoritative behavior coverage limited to the lower-layer action substrate.

### Commands

1. `cargo test -p worldwake-ai goal_model::tests::search_restock_goal_returns_travel_barrier_for_remote_stale_source -- --exact`
2. `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan -- --exact`
3. `cargo test -p worldwake-ai`
4. `cargo test -p worldwake-systems epistemic_actions`
5. `cargo clippy -p worldwake-ai -p worldwake-systems --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - changed the grounded-goal epistemic barrier contract in `worldwake-ai` so arrival-observable stale facts now terminate at travel-to-place instead of synthesizing a post-arrival `VerifyBelief` step
  - kept `AskWitness` as the live explicit epistemic barrier action for social information transfer
  - strengthened the stale-source golden to prove the corrected travel-side barrier contract and to prove no `verify_belief` commit occurs on that arrival-observable path
  - corrected `specs/S34-general-epistemic-actions.md` and `tickets/S34GENEPIACT-010.md` to the fact-sensitive contract
- Deviations from original plan:
  - no `worldwake-systems` production change was needed; the lower-layer `verify_belief` action substrate remains covered by existing focused tests but is no longer the canonical planner path for the currently modeled arrival-observable facts
- Verification results:
  - `cargo test -p worldwake-ai goal_model::tests::grounded_goal_epistemic_barrier_matches_only_matching_payloads -- --exact` passed
  - `cargo test -p worldwake-ai goal_model::tests::search_restock_goal_returns_travel_barrier_for_remote_stale_source -- --exact` passed
  - `cargo test -p worldwake-ai goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload -- --exact` passed
  - `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan -- --exact` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test -p worldwake-systems epistemic_actions` passed
  - `cargo clippy -p worldwake-ai -p worldwake-systems --all-targets -- -D warnings` passed
