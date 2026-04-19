# S108PERACTBIN-005: Integration + golden regression tests for binding strictness

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — this ticket is test-only. It exercises the dispatch/start-failure gate wired by T-002 and the trace field from T-004 through executable golden scenarios extended with live stale-target and fungible-fallback setups.
**Deps**: archive/tickets/S108PERACTBIN-002.md, archive/tickets/S108PERACTBIN-003.md, archive/tickets/S108PERACTBIN-004.md (requires the dispatch-side gate from T-002, the T-003 planner-side contract correction, and the trace field from T-004).

## Problem

Spec S108's design succeeds only if the sim-side dispatch gate and the trace snapshot produce coherent end-to-end behavior on real scenarios. Unit tests in T-001 through T-004 prove each surface in isolation, but the Authoritative-to-AI Impact Rule checklist (CLAUDE.md) still requires verification across the full cycle: `get_affordances` → `generate_candidates` → `search_plan` → BestEffort action start/failure → `handle_plan_failure` → golden pass.

This ticket narrows the drafted golden expansion to the executable end-to-end slices on the live branch: `loot` exact-identity stale-binding refusal using an AI-selected corpse binding carried through a stale external request, consume-pipeline fungible preservation under a stale-lot change, and real `SelectedPlanTrace` binding-strictness assertions. The drafted `travel` equivalent-route golden is not AI-visible on the current architecture, and `accuse` does not expose a meaningful stale-target refusal boundary because the live action contract already allows remote absent suspects as long as the exact accused identity still exists.

## Assumption Reassessment (2026-04-18)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The existing golden inventory in `docs/generated/golden-e2e-inventory.md` (regenerate with `python3 scripts/golden_inventory.py --write --check-docs`) names the canonical `golden_*` tests. Accuse, loot, heal, eat, drink, and travel goldens exist under `crates/worldwake-ai/tests/golden_*.rs`. The specific test names and their scenarios must be enumerated at implementation time via `cargo test -p worldwake-ai -- --list` to match real targets per `tickets/README.md` check #7. The goldens listed in the spec are representative; implementation verifies actual test names.
2. Spec S108's Validation and Falsification section (items 7–10) enumerates the intended goldens. The reassessed spec explicitly included the 7-point Authoritative-to-AI checklist as part of this ticket's scope.
3. Shared abstraction boundary: the end-to-end agent decision cycle. Verification layers are decision-trace (for revalidation behavior and trace-snapshot assertions), action-trace (for dispatch start-failure recording), and focused runtime coverage (for the request-resolution trace's `ExactIdentityRequired` reason when the request shape itself is malformed/underbound).
4. If a failing golden is what motivates debugging during this ticket, restate the intended invariant: "`ExactIdentity` actions refuse silent substitution across the end-to-end cycle, but planner-side same-target payload/specific-entity revalidation remains lawful. Request resolution rejects underbound or malformed requests, and authoritative start-time validation still rejects stale concrete targets that survive to dispatch. `FungibleEquivalentCommodity` continues substituting; all existing goldens unchanged under the default classifications."
5. Live `GoalKind`s under test in the existing goldens: `Accuse`, `LootCorpse`, `TreatWounds`, `ConsumeOwnedCommodity` (or similar per current ranking decl), and `TravelToPlace` (or the live Travel goal name). The existing operator/affordance surface each scenario relies on remains unchanged — this ticket adds setup, not alters goal routing.
6. AI regression intended layer spans candidate generation (unchanged), runtime `agent_tick` (exercises revalidation gate from T-003), and golden E2E. Harness boundary: goldens require full action registries (not a local needs-only harness), per the existing golden architecture.
7. Ordering: this ticket does not introduce new ordering contracts. Existing goldens already define their tick ordering and cross-agent sequencing; the strictness gates fire inside a single request-resolution path within a single tick, not across tick boundaries.
8. Not applicable — no heuristic removal.
9. First failure boundary per scenario:
   - `accuse` with substituted suspect: revalidation or later dispatch/start-time validation may refuse depending on whether the original same-target step still survives past the primary revalidation miss; T-003 established that `revalidate_exact_target_step` itself is a lawful same-target path, not a substitution gate.
   - `loot` with moved corpse: the planner-side helpers do not retarget it; the relevant refusal surface is the existing revalidation miss or the T-002/T-005 dispatch/start-time path rather than a new T-003 gate.
   - `eat`/`drink` with substitute item available: revalidation succeeds via `FungibleEquivalentCommodity` path; action completes.
   - `travel` with alternate route: existing behavior under `EquivalentRouteStep`; action completes.
10. Not applicable.
11. Not applicable.
12. Scenario isolation for the `loot` regression test: the intended branch under test is "ExactIdentity refuses BestEffort substitution when the planned corpse is no longer present but another corpse of the same kind is at the same place." Competing lawful affordances the architecture would otherwise allow: the planner could pick the substitute corpse in a fresh plan (legitimate), or contention/grant mechanics could prefer the substitute. The scenario isolates the refusal branch by ensuring the planned `LootCorpse { corpse: X }` commitment is active at the moment `X` is removed, forcing the revalidation path to be exercised directly rather than the fresh-plan path.
13. Adjacent contradiction: if revalidation refuses but dispatch's T-002 gate was not yet live, the AI would replan but the next plan might still target the same gone entity. Because T-002 is a dependency of this ticket, that gap is closed.
14. Reassessment found two executable-surface mismatches that narrow this ticket:
   - `travel` binds to destination place, not route-edge identity, so an alternate edge to the same destination is not a distinct AI-visible substitution boundary in current goldens.
   - `accuse` validates exact suspect identity but does not require co-location or liveness, so moving or killing the suspect does not produce a meaningful stale-target refusal golden.
15. Not applicable.

## Architecture Check

1. Test-only ticket. Verification surfaces are decision trace (for `binding_strictness` snapshot and plan-failure causes), action trace (for authoritative start failure on stale fully bound steps), and the request-resolution trace (for `ExactIdentityRequired` on underbound or malformed requests). Using the strongest available lower-layer proof per precision rule #5 — decision-trace assertions for AI reasoning, action-trace for dispatch lifecycle, and focused request-resolution-trace coverage for the typed rejection reason.
2. No backward compatibility shim.

## Verification Layers

1. AI reasoning (plan selection before the interposed world change) -> decision trace assertions in golden tests.
2. Dispatch/start-time refusal -> action trace for a stale fully bound exact-identity loot request, with request-resolution trace proving the stale corpse binding was preserved instead of silently rebound.
3. Fungible negative control -> request-resolution trace showing non-exact `pick_up` handling under a stale-lot change, followed by successful self-care completion.
4. Trace snapshot correctness (`PlannedStepSummary.binding_strictness`) -> decision trace assertion on the AI-selected exact-identity loot step and the AI-selected fungible `pick_up` step.

## What to Change

### 1. Add a `loot` exact-identity golden
Add a golden in the strongest existing AI decision suite where:
- Agent A selects an exact-identity `LootCorpse(X)` path and records the chosen corpse in the decision trace.
- After the queue/grant setup is in place and before the stale `loot` request executes, corpse X is moved away while another corpse Y remains available.
- Carry the AI-selected stale binding through a BestEffort external `loot` request and assert that request resolution preserves X rather than silently rebinding to Y. The action trace should then show start failure or equivalent exact-identity refusal.

### 2. Add a consume-pipeline fungible negative control

Add an end-to-end golden in the strongest existing needs/decision suite where:
- Agent A plans a remote `AcquireCommodity(SelfConsume)` path with travel first and the selected plan binds a specific fungible `pick_up` lot X.
- After the travel leg completes but before the stale `pick_up` request executes, lot X is moved away while another same-commodity lot remains at the destination place.
- Carry the AI-selected stale binding through a BestEffort external `pick_up` request, assert that the request follows the non-exact fallback path, then return the agent to AI control and prove the consume pipeline still reaches `eat` (or `drink`) without an exact-identity-style failure.

### 3. Assert the trace snapshot

In the new `loot` exact-identity golden and the new consume-pipeline fungible golden, assert the selected-step snapshot matches the expected class. This verifies T-004's population site under a real decision cycle.

### 4. Authoritative-to-AI Impact Rule checklist pass

Run the full 7-point verification from CLAUDE.md:
1. `get_affordances` — unaffected (enumeration does not consult strictness).
2. `generate_candidates` — unaffected.
3. `search_plan` — unaffected.
4. BestEffort action start — document the exact request-resolution/action-trace pattern produced by the live stale `loot` refusal and the consume-pipeline stale `pick_up` handling.
5. `handle_plan_failure` — not directly re-proven in this narrowed golden slice because the live stale-request refusal is exercised through the carried external binding, not a same-tick autonomous failure branch.
6. Payload revalidation — not directly under test here. T-003 already established the same-target planner-side contract, and this ticket must not claim to re-prove a planner-side substitution gate that does not exist.
7. Golden tests — full `cargo test -p worldwake-ai` pass.

## Files to Touch

- `crates/worldwake-ai/tests/golden_ai_decisions.rs` and/or the strongest existing golden owner discovered at implementation time (modify — add stale-corpse refusal and consume-pipeline fungible fallback)
- Additional golden-support modules under `crates/worldwake-ai/tests/golden_harness/` or equivalent (modify — if new fixture helpers are needed; runtime types from `worldwake-ai/src/` should not need changes)
- `docs/generated/golden-e2e-inventory.md` (regenerate via `python3 scripts/golden_inventory.py --write --check-docs` once new tests are added)

## Out of Scope

- Changes to sim-side gate, revalidation gate, or trace field — delivered in T-002, T-003, T-004.
- Consolidation of `revalidate_exact_target_step` — spec's Open Migration Work.
- S109-specific discrepancy classification assertions — separate spec.
- Observer binary output extensions — separate tooling-only spec if warranted.

## Acceptance Criteria

### Tests That Must Pass

1. New `loot` golden: an AI-selected stale corpse binding is carried through a BestEffort request, and request-resolution/action trace proves no substitute corpse was rebound.
2. New consume-pipeline fungible golden: an interposed lot change still follows the non-exact `pick_up` path, and the agent completes self-care after AI resumes.
3. Trace-snapshot assertion: `SelectedPlanTrace.next_step.binding_strictness` matches `ActionDef::binding_strictness` for one `ExactIdentity` golden and one `FungibleEquivalentCommodity` golden.
4. Full Authoritative-to-AI checklist passes across the in-scope items above.
5. Existing suite: `cargo test -p worldwake-ai && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. The stale `loot` request preserves the original corpse binding under BestEffort instead of silently rebinding to a substitute corpse.
2. The consume pipeline preserves non-exact stale-lot handling and still reaches self-care completion.
3. `SelectedPlanTrace.next_step.binding_strictness` is populated from `ActionDef::binding_strictness` for every traced plan step that reaches the `next_step` assignment.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_ai_decisions.rs` or the strongest existing owner — new stale-corpse exact-identity refusal golden.
2. `crates/worldwake-ai/tests/golden_ai_decisions.rs` or the strongest existing owner — new consume-pipeline fungible fallback golden.
3. `docs/generated/golden-e2e-inventory.md` — regenerated via `python3 scripts/golden_inventory.py --write --check-docs`.

### Commands

1. `cargo test -p worldwake-ai -- --list | rg '(loot|eat|drink|pick_up|binding_strictness)'` (enumerate real test names before writing assertions)
2. Focused exact test commands for the new loot and consume-pipeline goldens
3. `cargo test -p worldwake-ai`
4. `cargo build --workspace`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `python3 scripts/golden_inventory.py --write --check-docs`

## Outcome

Completed on 2026-04-19.

- Added two new golden regressions in `crates/worldwake-ai/tests/golden_ai_decisions.rs`:
  - `golden_loot_refuses_substitute_corpse_after_remote_travel_commitment`
  - `golden_consume_pipeline_rebinds_pick_up_after_remote_lot_change`
- Regenerated the golden inventory/docs via `python3 scripts/golden_inventory.py --write --check-docs`.
- The generator refresh widened the touched docs surface beyond the owning scenario page set and also created `docs/generated/golden-scenario-details/drive-escalation-wash-priority.md`; that broader generated churn is part of the landed handoff for this ticket.
- Updated this ticket and `specs/S108-per-action-binding-strictness.md` to match the landed proof shape.

### Deviations

1. The exact-identity loot proof landed as a hybrid golden slice: the AI-selected corpse binding is captured from decision trace, then carried through a stale external BestEffort `loot` request. The live autonomous branch fresh-replans once the corpse disappears, so a same-tick autonomous stale-request refusal was not an executable golden surface.
2. The fungible consume proof landed as a hybrid consume-pipeline slice: the AI-selected stale `pick_up` binding is carried through an external BestEffort request, then AI resumes and proves self-care completion. The request-resolution trace proves non-exact fallback handling and the end-to-end outcome, but does not expose a stable alternate-lot id assertion on this branch.

### Verification Result

Passed:

1. `cargo test -p worldwake-ai golden_loot_refuses_substitute_corpse_after_remote_travel_commitment -- --exact`
2. `cargo test -p worldwake-ai golden_consume_pipeline_rebinds_pick_up_after_remote_lot_change -- --exact`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test -p worldwake-ai`
5. `cargo build --workspace`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`
