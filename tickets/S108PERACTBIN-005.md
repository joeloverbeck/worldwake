# S108PERACTBIN-005: Integration + golden regression tests for binding strictness

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — this ticket is test-only. It exercises the gate wired by T-002, the revalidation gate by T-003, and the trace field from T-004 through existing golden scenarios extended with `ExactIdentity`-substitution-attempt setups.
**Deps**: archive/tickets/S108PERACTBIN-002.md, tickets/S108PERACTBIN-003.md, tickets/S108PERACTBIN-004.md (requires all three feature paths to be live).

## Problem

Spec S108's design succeeds only if the sim-side dispatch gate, the AI-side revalidation gate, and the trace snapshot produce coherent end-to-end behavior on real scenarios. Unit tests in T-001 through T-004 prove each surface in isolation, but the Authoritative-to-AI Impact Rule checklist (CLAUDE.md) requires verification across the full cycle: `get_affordances` → `generate_candidates` → `search_plan` → BestEffort action start → `handle_plan_failure` → payload revalidation → golden pass.

This ticket extends the goldens listed in spec D1-Validation items 7–10 (`accuse`, `loot`, `eat`/`drink`, `travel`) with setups that attempt BestEffort substitution and assert the correct behavior: `ExactIdentity` refuses, `FungibleEquivalentCommodity` substitutes, `EquivalentRouteStep` substitutes.

## Assumption Reassessment (2026-04-18)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The existing golden inventory in `docs/generated/golden-e2e-inventory.md` (regenerate with `python3 scripts/golden_inventory.py --write --check-docs`) names the canonical `golden_*` tests. Accuse, loot, heal, eat, drink, and travel goldens exist under `crates/worldwake-ai/tests/golden_*.rs`. The specific test names and their scenarios must be enumerated at implementation time via `cargo test -p worldwake-ai -- --list` to match real targets per `tickets/README.md` check #7. The goldens listed in the spec are representative; implementation verifies actual test names.
2. Spec S108's Validation and Falsification section (items 7–10) enumerates the intended goldens. The reassessed spec explicitly included the 7-point Authoritative-to-AI checklist as part of this ticket's scope.
3. Shared abstraction boundary: the end-to-end agent decision cycle. Verification layers are decision-trace (for revalidation refusal and trace-snapshot assertions), action-trace (for dispatch start-failure recording), and focused runtime coverage (for the request-resolution trace's `ExactIdentityRequired` reason).
4. If a failing golden is what motivates debugging during this ticket, restate the intended invariant: "`ExactIdentity` actions refuse silent substitution across the end-to-end cycle: revalidation should refuse stale fully bound steps, request resolution should reject underbound or malformed requests, and authoritative start-time validation should still reject stale concrete targets that survive to dispatch. `FungibleEquivalentCommodity` continues substituting; all existing goldens unchanged under the default classifications."
5. Live `GoalKind`s under test in the existing goldens: `Accuse`, `LootCorpse`, `TreatWounds`, `ConsumeOwnedCommodity` (or similar per current ranking decl), and `TravelToPlace` (or the live Travel goal name). The existing operator/affordance surface each scenario relies on remains unchanged — this ticket adds setup, not alters goal routing.
6. AI regression intended layer spans candidate generation (unchanged), runtime `agent_tick` (exercises revalidation gate from T-003), and golden E2E. Harness boundary: goldens require full action registries (not a local needs-only harness), per the existing golden architecture.
7. Ordering: this ticket does not introduce new ordering contracts. Existing goldens already define their tick ordering and cross-agent sequencing; the strictness gates fire inside a single request-resolution path within a single tick, not across tick boundaries.
8. Not applicable — no heuristic removal.
9. First failure boundary per scenario:
   - `accuse` with substituted suspect: revalidation refuses (T-003 gate in `revalidate_exact_target_step`, which already covered `accuse` via its all-`SpecificEntity` TargetSpec; now covered authoritatively by the strictness gate).
   - `loot` with moved corpse: revalidation refuses (T-003 gate — newly covered by strictness; the legacy `revalidate_exact_target_step` did not apply to `loot` because its TargetSpec is `EntityAtActorPlace`).
   - `eat`/`drink` with substitute item available: revalidation succeeds via `FungibleEquivalentCommodity` path; action completes.
   - `travel` with alternate route: existing behavior under `EquivalentRouteStep`; action completes.
10. Not applicable.
11. Not applicable.
12. Scenario isolation for the `loot` regression test: the intended branch under test is "ExactIdentity refuses BestEffort substitution when the planned corpse is no longer present but another corpse of the same kind is at the same place." Competing lawful affordances the architecture would otherwise allow: the planner could pick the substitute corpse in a fresh plan (legitimate), or contention/grant mechanics could prefer the substitute. The scenario isolates the refusal branch by ensuring the planned `LootCorpse { corpse: X }` commitment is active at the moment `X` is removed, forcing the revalidation path to be exercised directly rather than the fresh-plan path.
13. Adjacent contradiction: if revalidation refuses but dispatch's T-002 gate was not yet live, the AI would replan but the next plan might still target the same gone entity. Because T-002 is a dependency of this ticket, that gap is closed.
14. No mismatch discovered during reassessment.
15. Not applicable.

## Architecture Check

1. Test-only ticket. Verification surfaces are decision trace (for `binding_strictness` snapshot and plan-failure causes), action trace (for authoritative start failure on stale fully bound steps), and the request-resolution trace (for `ExactIdentityRequired` on underbound or malformed requests). Using the strongest available lower-layer proof per precision rule #5 — decision-trace assertions for AI reasoning, action-trace for dispatch lifecycle, and focused request-resolution-trace coverage for the typed rejection reason.
2. No backward compatibility shim.

## Verification Layers

1. AI reasoning (candidate re-selection after `ExactIdentity` refusal) -> decision trace assertions in golden tests.
2. Dispatch/start-time refusal -> action trace for stale fully bound exact-identity steps, plus `RequestResolutionOutcome::RejectedBeforeStart { reason: ExactIdentityRequired }` in request-resolution trace only when the request shape itself is underbound or malformed.
3. Revalidation refusal (tick before dispatch) -> decision trace showing the plan was invalidated and replanning occurred.
4. Fungible/route substitution success (negative control) -> action trace showing the step started with `RequestBindingKind::BestEffortFallback` for the non-`ExactIdentity` classes.
5. Trace snapshot correctness (`PlannedStepSummary.binding_strictness`) -> decision trace assertion in one golden per class asserting the snapshot matches the authoritative `ActionDef::binding_strictness`.

## What to Change

### 1. Extend the existing `accuse` golden

Identify the active `accuse` golden (enumerate via `cargo test -p worldwake-ai -- --list | rg accuse`). Add a setup variant where:
- Agent A plans `Accuse(B)`.
- Between plan selection and step execution, B moves/dies/is removed and a different Agent C (same `EntityKind::Agent`) is co-located.
- Assert that Agent A does not execute the step against C. Decision trace shows the plan invalidated via `revalidate_next_step` returning `false`. Request-resolution trace (if the dispatch path is reached on a later tick) shows `ExactIdentityRequired`.

### 2. Extend the existing `loot` golden

Identify the active `loot` golden. Add a setup variant where:
- Agent A plans `LootCorpse(X)`.
- Corpse X is removed (e.g., by another looter, bury, decay) while another corpse Y is present at the same place.
- Assert that A does not loot Y. The refusal surfaces at revalidation (primary) or dispatch (secondary).

### 3. Extend the existing `eat`/`drink` golden (negative control)

Identify the active needs goldens. Confirm that:
- When the planned item-lot X is consumed by someone else but another item-lot Y of the same commodity is at the same place, the planner/dispatcher substitutes Y via `FungibleEquivalentCommodity`.
- Action trace shows the step started successfully (no refusal).

### 4. Extend the existing `travel` golden (negative control)

Identify the active travel golden. Confirm that:
- When the planned route edge is blocked but an alternate edge reaches the same destination, `EquivalentRouteStep` permits the substitution.
- Action trace shows the step started successfully.

### 5. Assert the trace snapshot

In one extended golden per class (at minimum `accuse` for `ExactIdentity` and `eat` for `FungibleEquivalentCommodity`), assert `SelectedPlanTrace.next_step.binding_strictness` equals the expected class. This verifies T-004's population site under a real decision cycle.

### 6. Authoritative-to-AI Impact Rule checklist pass

Run the full 7-point verification from CLAUDE.md:
1. `get_affordances` — unaffected (enumeration does not consult strictness).
2. `generate_candidates` — unaffected.
3. `search_plan` — unaffected.
4. BestEffort action start — document the exact action-trace or request-resolution-trace pattern produced by the live `ExactIdentity` surface under test, depending on whether the scenario reaches revalidation only, authoritative start-time validation, or an underbound request-resolution rejection.
5. `handle_plan_failure` — document that a refused `ExactIdentity` step routes through `handle_plan_failure` via `BlockingFact::AssumptionFailed` (pre-S109 mapping); assert in one golden.
6. Payload revalidation — confirm the strictness gate precedes the payload validator in `revalidate_best_effort_payload_override_step`; assert the ordering in a unit test at the T-003 level if not already present there.
7. Golden tests — full `cargo test -p worldwake-ai` pass.

## Files to Touch

- `crates/worldwake-ai/tests/golden_accuse*.rs` (modify — add substitution-refusal variant; enumerate exact file via `ls crates/worldwake-ai/tests/ | rg accuse` at implementation time)
- `crates/worldwake-ai/tests/golden_loot*.rs` or combat goldens (modify — add substitute-corpse-refusal variant)
- `crates/worldwake-ai/tests/golden_needs*.rs` or consume goldens (modify — verify fungible substitution still succeeds)
- `crates/worldwake-ai/tests/golden_travel*.rs` (modify — verify route substitution still succeeds)
- Additional golden-support modules under `crates/worldwake-ai/tests/golden_harness/` or equivalent (modify — if new fixture helpers are needed; runtime types from `worldwake-ai/src/` should not need changes)
- `docs/generated/golden-e2e-inventory.md` (regenerate via `python3 scripts/golden_inventory.py --write --check-docs` once new tests are added)

## Out of Scope

- Changes to sim-side gate, revalidation gate, or trace field — delivered in T-002, T-003, T-004.
- Consolidation of `revalidate_exact_target_step` — spec's Open Migration Work.
- S109-specific discrepancy classification assertions — separate spec.
- Observer binary output extensions — separate tooling-only spec if warranted.

## Acceptance Criteria

### Tests That Must Pass

1. Extended `accuse` golden: agent refuses to accuse a substitute suspect; decision trace shows revalidation refusal.
2. Extended `loot` golden: agent refuses to loot a substitute corpse; request-resolution trace (or action trace) records `ExactIdentityRequired` if the refusal path reaches dispatch.
3. Extended `eat`/`drink` goldens: fungible substitution continues to succeed under `FungibleEquivalentCommodity`; action trace shows successful `BestEffortFallback` binding.
4. Extended `travel` golden: route substitution continues to succeed under `EquivalentRouteStep`.
5. Trace-snapshot assertion: `SelectedPlanTrace.next_step.binding_strictness` matches `ActionDef::binding_strictness` for at least one `ExactIdentity` and one `FungibleEquivalentCommodity` golden.
6. Full Authoritative-to-AI checklist passes across all 7 items.
7. Existing suite: `cargo test -p worldwake-ai && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. For all `ExactIdentity` actions, the full agent decision cycle (revalidation → dispatch) refuses silent substitution under BestEffort.
2. For all non-`ExactIdentity` actions, existing substitution behavior is preserved (negative controls pass).
3. `SelectedPlanTrace.next_step.binding_strictness` is populated from `ActionDef::binding_strictness` for every traced plan step that reaches the `next_step` assignment.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_accuse_*.rs` — new substitution-refusal variant asserting decision-trace invalidation and `ExactIdentityRequired` in request-resolution trace.
2. `crates/worldwake-ai/tests/golden_combat_*.rs` or `golden_loot_*.rs` — new substitute-corpse-refusal variant.
3. `crates/worldwake-ai/tests/golden_needs_*.rs` — extended fungible-substitution negative-control assertion (if not already covered).
4. `crates/worldwake-ai/tests/golden_travel_*.rs` — extended route-substitution negative-control assertion.
5. `docs/generated/golden-e2e-inventory.md` — regenerated via `python3 scripts/golden_inventory.py --write --check-docs`.

### Commands

1. `cargo test -p worldwake-ai -- --list | rg '(accuse|loot|eat|drink|travel)'` (enumerate real test names before writing assertions)
2. `cargo test -p worldwake-ai golden_accuse`
3. `cargo test -p worldwake-ai` (full golden suite pass)
4. `cargo build --workspace && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
5. `python3 scripts/golden_inventory.py --write --check-docs`
