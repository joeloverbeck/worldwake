# AITRACE-003: Comparative route-choice traceability and planner-contract doc sync

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` planner/decision trace enrichment for perceived travel choice provenance, plus planner-contract documentation updates
**Deps**: archive/tickets/completed/AITRACEPLAN-001-selected-plan-traceability.md, archive/tickets/completed/AITRACEPLAN-002-epistemic-root-omission-provenance.md, archive/tickets/completed/E18BANDYN-009.md

## Problem

Worldwake can already prove that the planner prefers a safer longer route and that T22 merchants eventually change behavior after bandit threat decays, but the current trace surface still makes “why this route beat that route” harder than it should be. Decision traces expose selected plan shape, root omissions, and duration-skip diagnostics, yet they do not surface comparative perceived travel cost / threat contributors for the rival travel branches that lost. At the same time, `docs/planner-contracts.md` is now stale after recent planner and E18 work, so the written contract no longer matches the live duration dependency inventory or the live terminal synthesis surface.

## Assumption Reassessment (2026-03-30)

1. The live planner route-preference boundary is planner-local perceived travel cost, not authoritative travel duration alone. The governing symbols are [`PlanningSnapshot::direct_perceived_travel_cost()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs), [`route_threat_estimate()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/route_threat.rs), and the A* search path in [`crates/worldwake-ai/src/search/heuristic.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/heuristic.rs) and [`crates/worldwake-ai/src/search/frontier.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/frontier.rs).
2. The current decision trace already explains some planner boundaries well: root operator omission via [`RootOperatorOmissionTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs), surfaced root-candidate skip reasons via [`RootCandidateTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs), and final selection via [`SelectionTrace`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs). The missing piece is comparative provenance for travel branch choice, not generic planner trace absence.
3. Focused coverage for the live route-choice behavior already exists in `search::tests::search_prefers_longer_low_threat_route_over_shorter_dangerous_route`. That test proves the planner behavior, but not the developer-facing explanation surface for the losing branch.
4. The mixed-layer proof surface already exists in [`golden_t22_bandit_camp_destruction`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs), which proves the downstream merchant-route change after threat decay. The intended invariant for this ticket is narrower: the trace should explain the route-choice divergence at the planner boundary instead of forcing inference from eventual arrival alone.
5. `docs/planner-contracts.md` is stale against the live code. The current duration dependency inventory in [`PlannerDurationDependency`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_duration_contract.rs) now includes `BanditCampEstablishmentProfile` and `ActorWitnessQueryDisposition`, while the document’s inventory still omits them.
6. `docs/planner-contracts.md` is also stale on the exact-goal terminal surface. The live code in [`GroundedGoal::synthesized_root_candidate_targets()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) now synthesizes `GoalKind::EstablishBanditCamp` into `PlannerOpKind::EstablishCamp`, but the doc’s “live synthesized terminal families” section does not reflect that.
7. The shared abstraction boundary under audit is planner-local route preference plus its debugging contract. This ticket should not change route-threat math, merchant ranking, or the travel heuristic itself unless reassessment during implementation finds that the trace cannot faithfully represent the current contract without a small structural cleanup.
8. This is not a learned-preferences ticket. The missing explanation is about current belief-backed perceived cost, not new persistent route-learning architecture.
9. This is not a golden-only ticket. The golden should remain a downstream mixed-layer proof; the primary implementation surface is focused planner traceability plus docs sync.
10. Mismatch + correction: the current repo docs say “use planner contracts when AI traceability is involved,” but the contract document is no longer current after E18. This ticket should fix the doc, not create a second planner-traceability doc.

## Architecture Check

1. Adding comparative route-choice provenance at the planner boundary is cleaner than relying on later authoritative arrival or ad hoc debug dumps because the planner itself owns the decision substrate.
2. The clean design is to expose concrete inputs for the chosen and competing travel branches: raw travel ticks, perceived threat contribution, and resulting perceived cost. That keeps the trace aligned with `docs/FOUNDATIONS.md` principles around explainable emergence and concrete state.
3. Updating `docs/planner-contracts.md` in the same ticket is cleaner than allowing ticket lore to drift away from the code again. One canonical contract doc is better than tribal knowledge spread across archived tickets.
4. No backwards-compatibility aliasing/shims introduced. The trace work should enrich the existing decision/search trace surfaces, not add a separate route-debug subsystem.

## Verification Layers

1. Perceived route-choice comparison at planner boundary -> focused planner/decision-trace test.
2. Existing route-preference behavior remains unchanged -> `search::tests::search_prefers_longer_low_threat_route_over_shorter_dangerous_route`.
3. Downstream merchant route change after bandit threat decay -> `golden_t22_bandit_camp_destruction` remains the mixed-layer proof.
4. Planner-contract documentation matches live symbols -> docs diff plus command-based docs sync verification.
5. If traces still prove the outcome but not enough route-comparison provenance after this ticket, the remaining gap should become its own follow-up instead of broadening goldens with weaker downstream assertions.

## What to Change

### 1. Add comparative travel-choice provenance to planner traces

Extend the planner/decision trace model so a developer can inspect why one candidate travel branch beat another in terms of the current live planner substrate.

The trace should stay concrete and limited to live data, such as:

- next-step destination under comparison
- raw travel ticks
- perceived threat or penalty contribution
- resulting perceived travel cost
- whether the branch won, lost on cost, or lost later for a distinct planner reason

Do not invent a second route-scoring model or generic “route quality” scalar divorced from the current perceived-cost contract.

### 2. Add focused route-trace coverage

Strengthen focused planner trace tests so the explanation surface is asserted directly, not inferred from final arrival.

### 3. Sync the planner-contract docs

Update `docs/planner-contracts.md` so its duration dependency inventory and live synthesized terminal families match current code, including the E18 additions.

If the new route-comparison trace adds a new planner trace contract worth documenting, add it there and, only if needed, add one short clarification to `docs/golden-e2e-testing.md`.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/search/heuristic.rs` and/or `crates/worldwake-ai/src/search/frontier.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)
- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify if one narrow mixed-layer assertion is warranted)
- `docs/planner-contracts.md` (modify)
- `docs/golden-e2e-testing.md` (modify only if the new trace surface changes testing guidance)

## Out of Scope

- changing route-threat arithmetic or belief-confidence decay
- learned route/source preferences (`S38`)
- merchant ranking policy changes
- adding a new authoritative route-danger state
- creating a second planner-contract doc

## Acceptance Criteria

### Tests That Must Pass

1. A focused planner trace test proves the chosen travel branch records concrete comparative perceived-cost provenance against at least one losing rival branch.
2. `search::tests::search_prefers_longer_low_threat_route_over_shorter_dangerous_route` still passes unchanged in behavior.
3. If a T22 golden assertion is added, it proves the route-choice explanation at the planner boundary rather than replacing it with a weaker downstream-only assertion.
4. `docs/planner-contracts.md` correctly lists the live planner duration dependency inventory and live synthesized terminal surfaces present in code.
5. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Route-choice traceability remains belief-backed and planner-local; the ticket must not introduce omniscient or authoritative route-danger reads.
2. Trace fields stay concrete and causally meaningful, matching `docs/FOUNDATIONS.md` rather than inventing abstract route scores.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — add focused comparative route-choice trace assertions at the planner boundary.
2. `crates/worldwake-ai/src/decision_trace.rs` — add focused summary/formatting coverage for the new route-choice provenance.
3. `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` — only if needed, add one narrow assertion proving the downstream scenario can now cite the stronger planner boundary directly.

### Commands

1. `cargo test -p worldwake-ai search::tests::search_prefers_longer_low_threat_route_over_shorter_dangerous_route -- --nocapture`
2. `cargo test -p worldwake-ai decision_trace::tests::summary_planning_includes_root_candidate_omissions_and_dependency_diagnostics -- --nocapture`
3. `cargo test -p worldwake-ai golden_t22_bandit_camp_destruction -- --nocapture`
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `cargo test -p worldwake-ai`
6. `cargo clippy --workspace --all-targets -- -D warnings`
