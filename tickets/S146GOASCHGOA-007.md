# S146GOASCHGOA-007: Migration validation tests + `golden_per_goal_budget` golden

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Tests only — no production code changes
**Deps**: archive/tickets/S146GOASCHGOA-005.md, tickets/S146GOASCHGOA-006.md

## Problem

S146's migration (ticket 005) and per-goal budget application (ticket 006) must not silently regress candidate emission. This ticket adds the validation surface required by spec D9: extractor-output parity tests against captured pre-migration fixtures, the registry-coverage runtime test (echoed here for golden completeness, alongside the in-source test from ticket 004), per-preset clamp tests, and a new `golden_per_goal_budget.rs` that proves per-goal differentiation with an elevated `CognitiveProfile`.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. After ticket 005 lands, the 20 `CandidateExtractor` impls have replaced the legacy `emit_*` functions. After ticket 006 lands, `effective_budget` is computed at search dispatch and recorded on every `PlanAttemptTrace`. Existing focused tests in `candidate_generation.rs` (17 named tests, see ticket 005 Assumption Reassessment item 1) and existing trace tests in `decision_trace.rs` (`repair_attempt_trace_roundtrips_through_bincode:2715`, etc.) are passing — this ticket adds new validation surface above them.
2. Per `specs/S146-goal-schema-and-per-goal-budgets.md` D9 + Test Plan: four validation surfaces are added — (a) `goal_schema_registry_covers_all_keys` (already added in ticket 004 — confirm no duplication here), (b) `extractor_outputs_match_legacy_emit_*` parity tests against captured fixtures, (c) `per_goal_budget_caps_below_cognitive_ceiling` (already added in ticket 006 — confirm no duplication), (d) `golden_per_goal_budget.rs` with elevated cognitive profile. The parity-fixture capture protocol: pre-implementation capture of `emit_*` outputs on `survival-baseline.ron`, serialized to JSON fixtures committed under `crates/worldwake-ai/tests/fixtures/s146_extractor_parity/`. Post-migration parity test deserializes and compares.
3. Shared abstraction boundary under audit: the parity contract is "for every per-tick agent state derivable from `survival-baseline.ron`, each `CandidateExtractor` impl produces a `Vec<GoalOffer>` byte-identical to the legacy `emit_*` function's output." This is enforced by fixture comparison. The fixtures are temporary — removed (or replaced with golden-style invariants) after the migration is verified stable; the ticket explicitly authorizes their post-stability removal.
4. Failing-golden / invariant restatement: `golden_per_goal_budget.rs` proves the intended per-goal differentiation invariant. Setup: agent A with `CognitiveProfile { max_plan_depth: 24, max_node_expansions: 768, .. }` plus a scenario authoring a `ProduceCommodity` (bread recipe) and an `Eat` goal. Assertions on `PlanAttemptTrace.goal_budget` (ticket 006's field): the `ProduceCommodity` attempt's `goal_budget.max_depth == 16` (PRODUCTION preset), the `Eat` attempt's `goal_budget.max_depth == 6` (SELF_CARE preset).
5. Live `GoalKind` surface under test: `GoalKind::ProduceCommodity { recipe_id }` and `GoalKind::ConsumeOwnedCommodity { commodity }` (or `AcquireCommodity` for self-consume — confirm via the populated `GoalSchema` entries from ticket 004 which dispatch through `Eat` semantics). Live operator surface is the existing `PlannerOpKind::Consume` (for eating, `planner_ops.rs:16`) and the production op chain.
6. AI-regression layer: candidate-generation focused/unit coverage (parity tests), runtime `agent_tick` decision-trace coverage (golden_per_goal_budget asserts on traces), golden E2E coverage (golden_per_goal_budget itself). Full action registries required because the golden exercises production + need-satisfaction goals end-to-end.
7. Ordering layer: the golden exercises plan-search ordering with differentiated budgets. The divergence between `ProduceCommodity` (depth 16) and `Eat` (depth 6) depends on **delayed system resolution** (the planner explores deeper for production goals because the schema's preset allows it under the elevated cognitive ceiling). Branch symmetry is NOT claimed — the goals are intentionally asymmetric to prove differentiation.
12. Scenario isolation: `golden_per_goal_budget.rs` scenario is intentionally narrow. Intended branch under test: per-goal budget differentiation under elevated cognitive ceiling. Lawful competing affordances explicitly excluded from setup: combat, political, bounty, social goals (no other agents, no offices, no bounties, no enemies). Unrelated lawful branches removed because they would dilute the budget-attribution signal — the contract here is "per-goal budget reaches search and is recorded on the trace," not "agent navigates a rich world."
13. Adjacent contradictions:
   - Tests (a) registry-coverage and (c) per-goal-budget-caps are scoped to tickets 004 and 006 respectively; this ticket does NOT duplicate them. Classified as **separate ticket scope** — confirmed.
   - Parity fixtures introduce a temporary scaffolding (fixtures committed, then removed). Classified as **future cleanup that must become its own ticket** — fixture removal can be a follow-up after the migration is verified stable; not required as part of S146.

## Architecture Check

1. FND-31 (validation and falsification are first-class): the parity contract is the primary falsification surface — any extractor impl that diverges from its legacy `emit_*` output fails the parity test, exposing the regression at the candidate-generation phase rather than downstream.
2. FND-12 (performance compresses computation, not causality): parity fixtures are causal-equivalence proofs — same agent state, same belief view, same scenario load → byte-identical candidate Vec. This is the strongest possible parity claim for a refactor.
3. Golden uses elevated cognitive profile explicitly per Q3=(a) resolution — does NOT change the project's cognitive defaults, isolates the per-goal differentiation invariant to scenarios that opt in.

## Verification Layers

1. Each migrated extractor's output is byte-identical to its pre-migration `emit_*` output for `survival-baseline.ron` agent states → fixture-based parity test
2. Registry coverage invariant holds in tests (echoed via the existing test from ticket 004; this ticket does not duplicate, only verifies it remains green)
3. Per-goal differentiation reaches search-attempt traces → `golden_per_goal_budget.rs` asserts on `PlanAttemptTrace.goal_budget` (decision-trace layer per FND-29)
4. Per-goal differentiation has the expected effect on plan-search outcomes when budgets are tight → `golden_per_goal_budget.rs` runs a multi-tick scenario and asserts the `ProduceCommodity` plan completes (depth 16 needed) while a hypothetical depth-8-clamped version would have exhausted

## What to Change

### 1. Parity-fixture capture protocol (pre-implementation step)

Before any code migration in ticket 005 was applied, the legacy `emit_*` outputs on `survival-baseline.ron` should have been captured. If the capture wasn't done as part of ticket 005's implementation, the implementer of THIS ticket re-runs the capture against the pre-ticket-005 commit (via `git checkout` of the parent commit, running the capture, copying fixtures into the current branch, returning to HEAD):

```bash
# Capture script (one-time, run against pre-ticket-005 commit):
cargo test -p worldwake-ai --test capture_extractor_parity_fixtures -- --ignored
```

The capture writes JSON fixtures (one per extractor family) to `crates/worldwake-ai/tests/fixtures/s146_extractor_parity/`. Format: `{tick: u64, agent: EntityId, extractor: CandidateExtractorId, candidates: Vec<GoalOfferSnapshot>}` lines. The snapshot type captures the GoalKey, OpportunityAnchor, and Evidence — enough to compare for byte-level equivalence.

### 2. Parity test: `extractor_outputs_match_legacy_emit_*`

New file `crates/worldwake-ai/tests/extractor_parity.rs`:

```rust
#[test]
fn need_extractor_matches_legacy_emit_need_candidates_fixture() {
    let fixtures = load_fixture("need.json");
    let scenario = load_scenario_file("scenarios/survival-baseline.ron").unwrap();
    let mut sim = spawn_scenario(scenario).unwrap();
    for record in fixtures {
        // advance sim to record.tick, get agent state, run NeedExtractor::extract,
        // assert candidates == record.candidates
    }
}
// ... 19 more, one per extractor family
```

### 3. `golden_per_goal_budget.rs`

New file `crates/worldwake-ai/tests/golden_per_goal_budget.rs`:

```rust
#[test]
fn golden_per_goal_budget_production_uses_depth_16() {
    // Setup: scenario with single agent + bread production chain
    // Agent cognitive_profile authored with max_plan_depth: 24, max_node_expansions: 768
    // Tick the sim until ProduceCommodity attempt registered in decision trace
    // Assert: trace's PlanAttemptTrace.goal_budget.max_depth == 16
    //         trace's PlanAttemptTrace.goal_budget.max_node_expansions == 384
}

#[test]
fn golden_per_goal_budget_eat_uses_depth_6() {
    // Same elevated cognitive profile
    // Assert: Eat goal's PlanAttemptTrace.goal_budget.max_depth == 6
    //         max_node_expansions == 96
}
```

The scenario can be a new `scenarios/per-goal-budget-golden.ron` (preferred — isolation per FND-31) or inline within the golden test (for tightness). Choose new RON for legibility.

### 4. Update generated golden inventory

Run `python3 scripts/golden_inventory.py --write --check-docs` to regenerate `docs/generated/golden-e2e-inventory.md`, `docs/generated/golden-scenario-index.md`, and `docs/generated/golden-scenario-details/`.

## Files to Touch

- `crates/worldwake-ai/tests/extractor_parity.rs` (new — 20 parity tests, one per extractor)
- `crates/worldwake-ai/tests/fixtures/s146_extractor_parity/*.json` (new — 20 fixture files)
- `crates/worldwake-ai/tests/golden_per_goal_budget.rs` (new — 2 goldens minimum: production depth-16, eat depth-6)
- `scenarios/per-goal-budget-golden.ron` (new — minimal scenario authoring elevated cognitive profile and bread production chain)
- `docs/generated/golden-e2e-inventory.md` (modify — regenerated)
- `docs/generated/golden-scenario-index.md` (modify — regenerated)
- `docs/generated/golden-scenario-details/per-goal-budget-golden.md` (new — per-golden detail file from regen)

## Out of Scope

- Modifying ticket 004's registry-coverage test or ticket 006's clamp tests — both already exist in their respective tickets; this ticket only adds new test surface.
- Removing parity fixtures — future cleanup ticket after migration stability is verified.
- Changing `CognitiveProfile` defaults — explicitly NOT done per Q3=(a) resolution; the golden authors elevated profile per-agent.
- Asserting on aggregate `PlanningMetrics` exhaustion-by-preset (S144 surface) — that aggregation is owned by S144's diagnostics machinery, exercised separately.
- Per-agent `budget_overrides` exercise — `AgentSchemaContextProfile.budget_overrides` is a defined field but no S146 ticket reads it from the search layer; deferred (see ticket 006 Out of Scope).

## Acceptance Criteria

### Tests That Must Pass

1. 20 new parity tests in `extractor_parity.rs` — each asserts byte-equivalent `Vec<GoalOffer>` for the captured fixture set
2. `golden_per_goal_budget_production_uses_depth_16` and `golden_per_goal_budget_eat_uses_depth_6` (the 2 minimum new goldens)
3. Existing golden suite: `cargo test -p worldwake-ai --test 'golden_*'`
4. `cargo test --workspace`
5. Clippy clean: `cargo clippy --workspace --all-targets -- -D warnings`
6. `python3 scripts/golden_inventory.py --check-docs` succeeds (regen consistent)

### Invariants

1. Parity contract: every extractor impl produces a `Vec<GoalOffer>` byte-identical to its pre-migration `emit_*` output for the captured `survival-baseline.ron` fixture set.
2. Per-goal budget reaches search and is recorded on `PlanAttemptTrace.goal_budget` (decision-trace layer per FND-29).
3. Elevated cognitive profile is the per-scenario opt-in for goals to plan past depth 8 — defaults remain unchanged (per Q3=(a)).
4. Scenario isolation per FND-31: `per-goal-budget-golden.ron` excludes lawful competing branches (combat, political, social, bounty, exploration) to keep the budget-attribution signal clean.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/extractor_parity.rs` — 20 parity tests
2. `crates/worldwake-ai/tests/golden_per_goal_budget.rs` — 2 goldens (production + eat)
3. `scenarios/per-goal-budget-golden.ron` — supporting scenario

### Commands

1. `cargo test -p worldwake-ai --test extractor_parity`
2. `cargo test -p worldwake-ai --test golden_per_goal_budget`
3. `cargo test -p worldwake-ai --test 'golden_*'` (full golden suite regression)
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `python3 scripts/golden_inventory.py --write --check-docs`
7. `scripts/verify.sh`
