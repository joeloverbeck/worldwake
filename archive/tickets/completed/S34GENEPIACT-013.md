# S34GENEPIACT-013: Remove dormant verify_belief substrate after arrival-observable barrier cleanup

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-core` epistemic profile cleanup, `worldwake-sim` payload/trace/duration cleanup, `worldwake-systems` action registry and handler removal, `worldwake-ai` planner-op/failure-path cleanup, S34 spec correction
**Deps**: [archive/tickets/completed/S34GENEPIACT-012.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-012.md), [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md), [tickets/S34GENEPIACT-010.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-010.md)

## Problem

After S34GENEPIACT-012, the live planner contract no longer uses `verify_belief` for the only currently modeled `VerificationSubject` variants (`EntityLocation`, `SupplyAvailability`). The action, payload, planner op, trace detail, profile field, and registry wiring still exist as a dormant alternate path. That leaves a misleading alias path in the architecture: the codebase still advertises a first-class verification action for a fact class whose canonical refresh path is now travel plus lawful arrival perception.

That violates the repo’s architectural bar in two ways:

- the same information contract still survives in two functional representations
- a duration-bearing action remains in the runtime without distinct causal work in the current world model

## Assumption Reassessment (2026-03-28)

1. The live AI/planning layer no longer requires `verify_belief` for arrival-observable stale facts. The focused planner surface is now `goal_model::tests::search_restock_goal_returns_travel_barrier_for_remote_stale_source` in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and the strengthened golden `golden_stale_prerequisite_belief_discovery_replan` in [golden_supply_chain.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) proves the stale-source path does not commit `verify_belief`.
2. Existing focused AI coverage still names and exercises stale-evidence derivation through `VerificationSubject`, `AskWitness`, and the travel-side barrier:
   - `goal_model::tests::grounded_goal_epistemic_subjects_extract_stale_subjects_from_originating_goal_evidence`
   - `goal_model::tests::grounded_goal_epistemic_barrier_matches_only_matching_payloads`
   - `goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload`
   - `candidate_generation::tests::low_confidence_evidence_keeps_originating_goal_without_standalone_epistemic_goal`
   - `candidate_generation::tests::stale_resource_source_stays_on_restock_goal_without_standalone_epistemic_goal`
3. Existing authoritative/runtime coverage in [epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs) still proves both `ask_witness` and `verify_belief`. This is not a missing-tests-only cleanup. The production registry still exposes `register_verify_belief_action()`, `verify_belief` still appears in the full action catalog in [action_registry.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/action_registry.rs), and focused tests still cover `register_verify_belief_action_creates_expected_definition` plus six `verify_belief_*` behavior tests.
4. The exact shared abstraction boundary under audit is mixed-layer and centered on the now-dormant verification action contract:
   - core/profile layer: `VerificationDispositionProfile::verify_belief_duration_ticks` and `VerificationSubject` in [epistemic.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/epistemic.rs)
   - sim transport/trace layer: `ActionPayload::VerifyBelief`, `VerifyBeliefPayload`, and `ActionTraceDetail::VerifyBelief` in [action_payload.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_payload.rs) and [action_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs)
   - systems/runtime layer: `register_verify_belief_action()` and `commit_verify_belief()` in [epistemic_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/epistemic_actions.rs)
   - AI/planner layer: `PlannerOpKind::VerifyBelief` and its downstream failure-handling/planner classification in [planner_ops.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) and [failure_handling.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs)
5. The live `GoalKind` under test remains `GoalKind::RestockCommodity { commodity: Bread }`. The exact current prerequisite surface is stale-evidence derivation in `grounded_goal_epistemic_subjects()` plus travel-to-place or `ask_witness` selection. There is no longer a live planner/search surface that lawfully requires `PlannerOpKind::VerifyBelief`.
6. This ticket is not motivated by a failing golden. The invariant is architectural: once arrival-observable stale facts moved to travel-side barriers, the old verification action ceased to describe any live fact class. The ticket therefore targets production cleanup plus focused coverage replacement, not golden isolation.
7. Coverage gap classification:
   - existing focused/unit coverage for `verify_belief` is present and will need removal or replacement
   - existing runtime trace/integration coverage for `verify_belief` is present in `worldwake-sim` action trace tests and `worldwake-systems` unit tests
   - golden/E2E already proves the corrected no-`verify_belief` path for arrival-observable stale-source recovery; no new golden is required here unless cleanup breaks traceability
8. The current heuristic/filter substrate is not being weakened here. S34GENEPIACT-012 already supplied the missing architectural substrate by making travel-to-place the barrier for arrival-observable facts. This ticket removes the old alias path after that substrate exists; it does not bypass a still-needed guard.
9. This is an information-path refactor. The same fact currently still has two functional representations in code:
   - canonical live path: stale subject -> travel/arrival refresh or `ask_witness`
   - dormant alias path: `verify_belief` action payload/registry/trace/profile support
   The canonical end state after this ticket should be one path only. The dormant alias path must be removed in-scope rather than left as a compatibility substrate.
10. Adjacent contradictions:
   - required consequence of this ticket: remove `verify_belief` from runtime catalogs, payload contracts, planner classifications, and tests
   - required ticket correction: the live `ask_witness` deduplication substrate is `AgentBeliefStore::asked_witnesses`, `AskWitnessMemoryKey`, and `AskWitnessMemory` in [belief.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/belief.rs), not Tell-memory reuse as the S34 spec currently claims
   - likely follow-up cleanup, not in scope here: once removal lands, remaining `Verification*` names may become semantically misleading for the surviving social-query contract and should be evaluated separately
11. Mismatch + correction: `specs/S34-general-epistemic-actions.md` still mentions `verify_belief` as a lower-layer substrate that may remain for future fact classes. If this ticket removes the substrate entirely, the spec must say that no explicit inspection action exists in the live architecture and any future reintroduction requires a new fact class plus a new ticket.
12. Mismatch + correction: `VerificationDispositionProfile` currently stores both `belief_verification_threshold` and `verify_belief_duration_ticks`. After removal, the threshold may still be lawful because the AI still decides when stale evidence becomes barrier-worthy for travel or `ask_witness`, but the action-duration field has no live consumer and must not survive as dead schema.
13. Mismatch + correction: [specs/IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md) still advertises the older "`inspect_place`, `ask_witness`, `verify_location` actions + `VerifyBelief` goal kind" wave summary. That roadmap line must be corrected in-scope so active planning material does not contradict the post-cleanup architecture.
14. Verification command reassessment: the current `cargo test -- --list` surface confirms dedicated `worldwake-sim` `action_payload::*verify_belief*`, `action_trace::*verify_belief*`, and `worldwake-systems` `epistemic_actions::*verify_belief*` tests exist. A focused runtime cleanup ticket therefore needs explicit `action_payload` coverage and duration/registry proof, not only `action_trace`.

## Architecture Check

1. Full removal is cleaner than keeping `verify_belief` as a dormant substrate. A dormant action still acts as a false architectural promise, invites future alias paths, and weakens debugging by advertising a contract the planner no longer uses.
2. Full removal is cleaner than inventing an inspection-only fact class to justify the current action. That would be speculative scaffolding with no current world substrate, violating the repo’s preference for concrete state over anticipatory abstraction.
3. No backwards-compatibility aliasing or deprecation shim should survive. If `verify_belief` is removed, all cross-layer references must be removed in-scope: action registration, payload variants, planner op classification, trace detail, profile field, and focused tests.
4. Full removal is still architecturally preferable to the current shape. The current architecture advertises a first-class explicit inspection action even though the only live stale-fact families already resolve through lawful arrival perception or social querying. Keeping that unused action substrate would preserve a false second transport path rather than a real extension point.

## Verification Layers

1. Arrival-observable stale facts still replan through travel-side barriers without `verify_belief` -> decision trace in `worldwake-ai` focused tests plus existing stale-source golden
2. `ask_witness` remains the only explicit epistemic action in the live catalog -> focused `worldwake-systems` action-registry/runtime tests and planner-op classification tests
3. No `verify_belief` action identity remains in runtime traces or payload transport -> focused `worldwake-sim` action payload and action trace tests
4. Removal does not break AI failure reconciliation for surviving epistemic paths -> focused `worldwake-ai` failure-handling and search tests
5. No dead action-duration consumer remains for the removed action -> focused `worldwake-sim` action-semantics/belief-view tests plus clippy/typecheck on removed enum variants
6. Active planning docs no longer advertise the removed path -> spec and implementation-order doc diff review
7. The strongest proof that the old path is gone is lower-layer focused coverage and registry/payload absence, not downstream golden behavior alone; the stale-source golden remains only a regression guard for the canonical travel-side path
8. This is mixed-layer, so additional layer mapping is required and included above

## What to Change

### 1. Remove `verify_belief` from the sim/runtime contract

Delete the sim/runtime transport pieces that only exist to support the dormant action:

- remove `ActionPayload::VerifyBelief` and `VerifyBeliefPayload`
- remove `ActionTraceDetail::VerifyBelief`
- remove action-duration resolution for `verify_belief`
- remove related serialization/accessor/round-trip tests

If `VerificationSubject` still remains as the canonical stale-subject identity for `ask_witness` matching, keep it for now. Do not rename it in this ticket unless removal makes the old name technically impossible to retain without aliasing.

### 2. Remove the authoritative `verify_belief` action and registry exposure

Delete the action definition, handler registration, affordance enumeration, authoritative validators, and focused tests for `verify_belief` from `worldwake-systems`.

The full action catalog should no longer include `"verify_belief"`. `ask_witness` remains.

### 3. Remove `PlannerOpKind::VerifyBelief` and AI references

Delete planner classification and any downstream assumptions that a `VerifyBelief` op might still appear:

- `PlannerOpKind::VerifyBelief`
- `classify_action_def()` mapping for `"verify_belief"`
- semantics-table entries and tests
- failure-handling branches that still mention `VerifyBelief`
- any goal-model or search helper logic that still treats `VerifyBelief` as a possible epistemic barrier

The surviving AI contract should be: stale subject -> travel-side barrier or `AskWitness`, never `VerifyBelief`.

### 4. Remove dead profile/schema surface and correct the S34 docs

Delete `VerificationDispositionProfile::verify_belief_duration_ticks` and update any world/delta fixtures that still serialize it.

Update [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md) so it no longer describes `verify_belief` as a live or dormant substrate. The spec should explicitly say that a future inspection-only action requires a future fact class and a new ticket; it is not preserved in today’s schema “just in case.”

Correct the stale `ask_witness` memory narrative in that spec to match the live contract: `ask_witness` currently uses dedicated `AskWitnessMemory*` state rather than Tell-memory reuse, and this ticket does not change that substrate.

Update [specs/IMPLEMENTATION-ORDER.md](/home/joeloverbeck/projects/worldwake/specs/IMPLEMENTATION-ORDER.md) so active roadmap text no longer advertises a removed `VerifyBelief`/`verify_location` path.

## Files to Touch

- `crates/worldwake-core/src/epistemic.rs` (modify — remove dead `verify_belief_duration_ticks`; keep only surviving core epistemic contract)
- `crates/worldwake-core/src/lib.rs` (modify — remove exported dead `VerifyBeliefPayload`-adjacent surface if required by type re-exports)
- `crates/worldwake-core/src/world.rs` (modify — update fixtures/defaults using the profile)
- `crates/worldwake-core/src/delta.rs` (modify — update delta fixtures/serde expectations)
- `crates/worldwake-sim/src/action_payload.rs` (modify — remove `VerifyBelief` payload transport)
- `crates/worldwake-sim/src/action_trace.rs` (modify — remove `VerifyBelief` trace detail support)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — remove verification-duration resolution)
- `crates/worldwake-sim/src/belief_view.rs` (modify — remove any remaining verification-duration read path)
- `crates/worldwake-sim/src/lib.rs` (modify — remove `VerifyBeliefPayload` export)
- `crates/worldwake-systems/src/epistemic_actions.rs` (modify — remove `verify_belief` action definition/handler/tests)
- `crates/worldwake-systems/src/action_registry.rs` (modify — remove registry exposure and full-catalog expectation)
- `crates/worldwake-systems/src/lib.rs` (modify — remove exported `register_verify_belief_action`)
- `crates/worldwake-ai/src/planner_ops.rs` (modify — remove planner op/classification)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — remove dead failure branches)
- `crates/worldwake-ai/src/goal_model.rs` (modify — remove stale `VerifyBelief` references from semantics/tests)
- `specs/S34-general-epistemic-actions.md` (modify — correct the live contract after full removal)
- `specs/IMPLEMENTATION-ORDER.md` (modify — remove stale roadmap wording about `VerifyBelief` / `verify_location`)

## Out of Scope

- inventing a new inspection-only fact class to replace the removed action
- renaming the remaining `Verification*` types if they can still function without aliasing; that naming cleanup belongs in a follow-up if needed
- adding new golden scenarios beyond the existing stale-source regression unless cleanup reveals a real traceability gap
- broad epistemic/planner refactors unrelated to removing the dormant action path

## Acceptance Criteria

### Tests That Must Pass

1. Focused AI coverage proves the surviving epistemic barrier contract is travel-to-place or `AskWitness`, with no `VerifyBelief` planner op remaining
2. Focused sim/systems coverage proves `verify_belief` is gone from action payloads, traces, and the full action registry
3. Existing stale-source recovery coverage still passes under the canonical travel-side barrier contract
4. `cargo test -p worldwake-ai`
5. `cargo test -p worldwake-sim action_payload`
6. `cargo test -p worldwake-sim action_trace`
7. `cargo test -p worldwake-systems epistemic_actions`
8. `cargo clippy -p worldwake-ai -p worldwake-sim -p worldwake-systems --all-targets -- -D warnings`

### Invariants

1. No live cross-layer `verify_belief` alias path remains after arrival-observable stale facts moved to travel-side barriers.
2. The live explicit epistemic action contract is not broader than the current world substrate justifies.
3. The surviving per-agent epistemic profile contains only fields with live semantic consumers.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` — remove or replace any focused tests that still mention `VerifyBelief` as a possible barrier
   Rationale: prove the AI contract no longer advertises a dead planner op.
2. `crates/worldwake-sim/src/action_payload.rs`, `crates/worldwake-sim/src/action_trace.rs`, and `crates/worldwake-sim/src/action_semantics.rs` — replace `verify_belief` payload/trace expectations with absence-focused coverage and surviving `ask_witness` coverage
   Rationale: prove the runtime transport and duration layer no longer carries the removed action identity.
3. `crates/worldwake-systems/src/action_registry.rs` and `crates/worldwake-systems/src/epistemic_actions.rs` — replace `verify_belief` registration/behavior tests with surviving `ask_witness` and catalog assertions
   Rationale: prove the action catalog and authoritative runtime no longer expose the removed path.

### Commands

1. `cargo test -p worldwake-ai -- --list | rg "search_restock_goal_returns_travel_barrier_for_remote_stale_source|ask_witness|standalone_epistemic_goal"`
2. `cargo test -p worldwake-sim action_payload`
3. `cargo test -p worldwake-sim action_trace`
4. `cargo test -p worldwake-systems epistemic_actions`
5. `cargo test -p worldwake-ai`
6. `cargo clippy -p worldwake-ai -p worldwake-sim -p worldwake-systems --all-targets -- -D warnings`

## Outcome

Completed: 2026-03-28

What actually changed:
- Removed the dormant `verify_belief` path across core, sim, systems, and AI: no payload variant, no trace detail, no action registration/handler, no planner op, and no duration/profile field remain.
- Preserved the canonical architecture for currently modeled stale facts: travel-side arrival refresh for arrival-observable subjects, plus `ask_witness` as the only explicit epistemic action.
- Corrected active planning docs so `specs/S34-general-epistemic-actions.md` and `specs/IMPLEMENTATION-ORDER.md` no longer advertise the removed path, and updated the S34 spec to describe the live `AskWitnessMemory` deduplication contract.

Deviations from original plan:
- The cleanup reached a few additional AI/sim surfaces not named in the original ticket file list: planner duration inventory, AI search candidate widening, AI observation bookkeeping, and golden assertions that still named `VerifyBelief`.
- `crates/worldwake-core/src/lib.rs` did not require a change after reassessment; the dead exported surface lived in `worldwake-sim/src/lib.rs`, not core.

Verification results:
- Passed `cargo test -p worldwake-sim action_payload`
- Passed `cargo test -p worldwake-sim action_trace`
- Passed `cargo test -p worldwake-systems epistemic_actions`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy -p worldwake-ai -p worldwake-sim -p worldwake-systems --all-targets -- -D warnings`
