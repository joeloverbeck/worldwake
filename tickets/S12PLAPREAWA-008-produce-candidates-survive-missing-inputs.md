# S12PLAPREAWA-008: Stop suppressing `ProduceCommodity` when recipe inputs are missing

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `candidate_generation.rs` production candidate emission and focused AI/runtime coverage
**Deps**: specs/S12-planner-prerequisite-aware-search.md, archive/tickets/completed/S12PLAPREAWA-003-combined-places-and-search-signature.md

## Problem

The runtime decision pipeline still blocks the clean S12 architecture at the candidate-generation layer.

Today, when a recipe output serves a live need or restock gap but the actor lacks some recipe inputs, `emit_produce_goals(...)` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) suppresses `GoalKind::ProduceCommodity { recipe_id }` and instead emits `AcquireCommodity { purpose: RecipeInput(recipe_id) }` through `emit_missing_recipe_input_goals(...)`.

That means the planner improvements from S12 do not fully reach the runtime AI pipeline:

1. the agent never gets to rank or select the downstream `ProduceCommodity` goal that names the desired world condition,
2. the system falls back to a prerequisite proxy goal rather than the actual production goal,
3. the candidate-generation layer still encodes a workaround for the old planner limitation.

This is now an architectural mismatch with Principle 18 in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): goals should name desired world conditions, and enabling acquisition should be planned beneath them rather than replacing them.

## Assumption Reassessment (2026-03-21)

1. The current behavior is implemented in `emit_produce_goals(...)` and `emit_missing_recipe_input_goals(...)` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs). When `recipe_path_evidence(...)` returns `None`, `emit_produce_goals(...)` does not emit `ProduceCommodity`; it delegates to `emit_missing_recipe_input_goals(...)`.
2. The focused test `candidate_generation::tests::missing_recipe_input_emits_acquire_goal_and_suppresses_produce_goal` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) currently codifies that suppression as expected behavior.
3. This is a candidate-generation-layer issue, not a ranking tie and not a planner failure. The divergence happens before ranking and planning run, so there is no branch symmetry to assess in motive arithmetic; the filter removes the `ProduceCommodity` branch outright.
4. The removed/superseded filter is the missing-input suppression in `emit_produce_goals(...)`. It stood in for the old planner limitation where production goals with remote prerequisites could not plan lawfully. S12 search changes now add the missing substrate: search can compute dynamic prerequisite places from hypothetical state, including `ProduceCommodity` recipe inputs.
5. The completed S12 search ticket already added production prerequisite places in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) and moved dynamic relevant-place computation into [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs). The remaining gap is upstream candidate emission.
6. Existing golden coverage already proves the runtime can complete the broader production branch once the pipeline gets there: `golden_multi_recipe_craft_path` and `golden_materialization_barrier_chain` in [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs).
7. The remaining active S12 tickets do not own this engine issue:
   - [S12PLAPREAWA-004-agent-tick-call-site-update.md](/home/joeloverbeck/projects/worldwake/tickets/S12PLAPREAWA-004-agent-tick-call-site-update.md) is a stale signature-follow-up ticket.
   - [S12PLAPREAWA-005-decision-trace-enrichment.md](/home/joeloverbeck/projects/worldwake/tickets/S12PLAPREAWA-005-decision-trace-enrichment.md) is trace-only.
   - [S12PLAPREAWA-006-unit-tests.md](/home/joeloverbeck/projects/worldwake/tickets/S12PLAPREAWA-006-unit-tests.md) is test-only.
   - [S12PLAPREAWA-007-golden-e2e-tests.md](/home/joeloverbeck/projects/worldwake/tickets/S12PLAPREAWA-007-golden-e2e-tests.md) is test-only and explicitly keeps planner internals/candidate-generation changes out of scope.
8. `CommodityPurpose::RecipeInput(recipe_id)` still exists in shared AI surfaces such as [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs) and [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs). This ticket does not need to remove that type to fix the runtime architecture at the candidate-generation boundary.
9. Isolation choice for focused coverage: the core scenario should include a known workstation, a reachable missing-input acquisition path, and a recipe output that directly serves hunger or enterprise restock. Unrelated lawful branches such as direct local need relief from already-controlled finished goods should be intentionally excluded from setup.
10. Mismatch + correction: the current code still treats recipe-input acquisition as a substitute for production-candidate emission. After S12 search improvements, that substitute should stop suppressing the actual production goal.

## Architecture Check

1. The cleaner architecture is to emit `ProduceCommodity { recipe_id }` whenever the actor has a believed lawful path to the output, including paths that require acquiring missing inputs. The planner should decompose procurement beneath that goal instead of candidate generation swapping in a prerequisite proxy.
2. This aligns with Principle 18 and Principle 1 in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): the AI pursues the desired world condition and lets lawful intermediate steps emerge through planning.
3. No backwards-compatibility aliasing/shims. Do not preserve the old suppression path alongside the new one as a mode or fallback flag.
4. `CommodityPurpose::RecipeInput` can remain as a shared type for now, but runtime production candidate generation should stop depending on it as the primary path when the downstream production goal is itself known and reachable.

## Verification Layers

1. Missing recipe inputs no longer suppress `ProduceCommodity` when a full believed path exists -> focused `candidate_generation.rs` unit test
2. Candidate generation still withholds `ProduceCommodity` when there is no lawful believed production path (for example, no workstation path) -> focused `candidate_generation.rs` unit test
3. Ranked/planned runtime can still execute the broader production chain once emitted -> existing golden production coverage in [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs)
4. If a focused runtime proof is added, candidate presence and final execution should be kept separate: candidate presence via focused unit/runtime test, action lifecycle via golden/action-trace assertions
5. This is primarily a candidate-generation ticket; no additional authoritative mutation layer mapping is required beyond the existing golden regression gate

## What to Change

### 1. Emit `ProduceCommodity` for lawful missing-input paths

In [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs):

- Refactor `emit_produce_goals(...)` so it no longer treats missing recipe inputs as an automatic reason to suppress `GoalKind::ProduceCommodity`.
- Add a helper or extend `recipe_path_evidence(...)` so it can prove a full believed production path even when some recipe inputs are missing, provided:
  - the actor knows the recipe,
  - required tools are available,
  - a relevant workstation path exists,
  - each missing input has a lawful acquisition path in beliefs.
- The emitted evidence should include both the workstation side and the missing-input acquisition side so the downstream planning snapshot can carry the relevant places/entities.

### 2. Narrow or remove the proxy suppression path

- Stop using `emit_missing_recipe_input_goals(...)` as a replacement for the downstream production goal in the path above.
- If any recipe-input acquire goal emission remains, it must be justified as an additional candidate rather than a suppressing substitute.
- The preferred direction is:
  - emit `ProduceCommodity` when the full production chain is believed reachable,
  - do not suppress it merely because prerequisites are not yet owned.

### 3. Update focused tests around the corrected boundary

- Replace the stale test that currently expects `AcquireCommodity` and no `ProduceCommodity` for missing-input production scenarios.
- Add focused coverage for:
  - missing-input production path still emits `ProduceCommodity`,
  - no-workstation or no-acquisition-path scenarios still withhold `ProduceCommodity`,
  - optional coexistence rules if recipe-input acquire goals are intentionally retained as additional candidates.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/tests/golden_production.rs` (modify only if focused runtime/golden adjustment is actually needed)

## Out of Scope

- Removing `CommodityPurpose::RecipeInput` from shared AI types
- Search heuristic changes in [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs)
- Goal-model changes in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) already covered by S12 search work
- Decision-trace schema changes
- New materialization-barrier modeling beyond current search/runtime behavior

## Acceptance Criteria

### Tests That Must Pass

1. Focused candidate-generation coverage proves a recipe with missing inputs can still emit `GoalKind::ProduceCommodity { recipe_id }` when the full believed production path exists
2. Focused candidate-generation coverage proves `ProduceCommodity` is still withheld when a lawful believed production path does not exist
3. Existing golden regression: `cargo test -p worldwake-ai golden_multi_recipe_craft_path`
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing lint gate: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Candidate generation names downstream desired world conditions rather than replacing them with prerequisite proxy goals
2. Production candidate emission remains belief-driven and locality-respecting; no authoritative world shortcutting is introduced
3. If recipe-input acquire candidates remain, they do not suppress a lawful `ProduceCommodity` candidate

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — replace the stale missing-input suppression test with production-candidate coverage for reachable missing-input paths
2. `crates/worldwake-ai/src/candidate_generation.rs` — add a negative-path test showing production is still withheld without a workstation or without acquisition evidence
3. `crates/worldwake-ai/tests/golden_production.rs` — none required by default; use existing golden regression unless a focused runtime gap is discovered during implementation

### Commands

1. `cargo test -p worldwake-ai missing_recipe_input`
2. `cargo test -p worldwake-ai golden_multi_recipe_craft_path`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
