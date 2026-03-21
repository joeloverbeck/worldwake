# S12PLAPREAWA-008: Stop proxying reachable production through `RecipeInput` acquire goals

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `candidate_generation.rs` production-path evidence and focused/golden AI coverage
**Deps**: archive/specs/S12-planner-prerequisite-aware-search.md, archive/tickets/completed/S12PLAPREAWA-003-combined-places-and-search-signature.md

## Problem

The remaining architectural mismatch is no longer in planner search. It is in top-level goal emission.

Today, when a recipe output serves a live need or enterprise restock gap but the actor lacks some recipe inputs, `emit_produce_goals(...)` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) suppresses `GoalKind::ProduceCommodity { recipe_id }` and emits `AcquireCommodity { purpose: RecipeInput(recipe_id) }` via `emit_missing_recipe_input_goals(...)`.

That keeps the old workaround alive above the now-capable S12 planner:

1. the selected top-level goal names the prerequisite instead of the desired world condition,
2. candidate generation still substitutes a proxy goal for a lawful downstream production branch,
3. simply emitting both candidates would still leave the proxy architecture in place because current ranking gives `RecipeInput` acquire goals recipe-output-driven priority/motive in [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs).

This conflicts with Principle 18 in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): the top-level goal should express the world condition the agent wants, and procurement should emerge beneath that goal through planning.

## Assumption Reassessment (2026-03-21)

1. The current suppression path is still live in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs): `emit_produce_goals(...)` emits `ProduceCommodity` only when `recipe_path_evidence(...)` succeeds, and otherwise falls through to `emit_missing_recipe_input_goals(...)`.
2. The focused unit test `candidate_generation::tests::missing_recipe_input_emits_acquire_goal_and_suppresses_produce_goal` in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) still codifies that proxy behavior as expected.
3. The planner-side S12 substrate is already present, so the original ticket assumption that search still lacks prerequisite-aware spatial guidance is stale. Live code already includes `GoalKindPlannerExt::prerequisite_places(...)` in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), `combined_relevant_places(...)` in [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs), and focused tests such as `goal_model::tests::prerequisite_places_produce_commodity_include_missing_input_places` and `search::tests::combined_places_include_remote_medicine_lot_for_treat_wounds`.
4. Existing golden/runtime coverage already proves the engine can execute multi-step prerequisite-aware production chains once the right top-level goal reaches search: [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs) contains `golden_multi_recipe_craft_path`, `golden_materialization_barrier_chain`, and `golden_acquire_commodity_recipe_input`; [golden_care.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_care.rs) contains `golden_healer_acquires_remote_ground_medicine_for_patient`.
5. This is not only a candidate-presence issue. If `ProduceCommodity` were emitted alongside `AcquireCommodity { purpose: RecipeInput(..) }` without narrowing the proxy path, current ranking would still preserve the proxy architecture for many scenarios: `AcquireCommodity { purpose: RecipeInput(recipe_id) }` uses `recipe_output_priority(...)` / `recipe_output_motive_score(...)`, while `ProduceCommodity` uses the generic production scoring path in [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs).
6. The cleaner scope is therefore narrower than “search + candidate generation” but broader than “emit one extra candidate”: fix the production evidence boundary so reachable production emits `ProduceCommodity`, and stop using `RecipeInput` acquire goals as a competing top-level substitute for that same reachable production branch.
7. `CommodityPurpose::RecipeInput(recipe_id)` still exists in shared AI surfaces such as [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs), [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs), and runtime tests. This ticket does not need to remove the type globally unless local production candidate generation no longer needs it after reassessment.
8. Isolation choice for focused coverage: the core scenario should include a known workstation, a reachable missing-input acquisition path, and an output whose motive comes from hunger or restock. Competing direct-fix branches such as already-owned finished goods or unrelated enterprise cargo should be excluded from setup.
9. Test-name reassessment was dry-run checked with `cargo test -p worldwake-ai -- --list` on 2026-03-21. The commands named below target real current tests/targets.
10. Mismatch + correction: the old ticket treated this as a pure candidate-generation substitution issue. The corrected scope is “candidate generation plus proxy removal for reachable production,” because leaving the proxy as a competing top-level candidate would keep the architecture and ranking behavior misaligned with S12’s intended goal semantics.

## Architecture Check

1. The cleaner architecture is to emit `ProduceCommodity { recipe_id }` whenever the actor has a believed lawful production path, even if that path includes acquiring missing inputs first. Planning should decompose procurement and workstation access beneath that goal.
2. For those reachable-production cases, candidate generation should stop emitting `AcquireCommodity { purpose: RecipeInput(recipe_id) }` as a competing top-level proxy. Removing the proxy at the boundary is cleaner than patching ranking to keep two semantically overlapping candidates in sync forever.
3. This aligns with Principle 1 and Principle 18 in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): author the lawful substrates, then let multi-step consequence chains emerge rather than hard-coding prerequisite-goal substitutions.
4. No backwards-compatibility aliasing/shims. Do not keep an old-vs-new mode flag, and do not preserve the proxy branch as a silent fallback for cases where the downstream production goal is already believed reachable.

## Verification Layers

1. Reachable missing-input production emits `ProduceCommodity` and not the `RecipeInput` proxy -> focused `candidate_generation.rs` unit test
2. Unreachable production still withholds `ProduceCommodity` when workstation/access path evidence is absent -> focused `candidate_generation.rs` unit test
3. Prerequisite-aware search substrate remains intact beneath the corrected candidate boundary -> existing `goal_model.rs` / `search.rs` focused tests already in tree
4. Runtime still completes missing-input production chain after candidate-boundary correction -> existing golden regression in [golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs), especially `golden_acquire_commodity_recipe_input` and `golden_multi_recipe_craft_path`
5. This ticket does not change authoritative action semantics, so no additional event-log ordering proof surface is required beyond existing golden production coverage

## What to Change

### 1. Teach production candidate emission about reachable missing-input paths

In [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs):

- Replace the current “owned inputs only” production evidence boundary with a helper that proves a full believed production path for candidate emission.
- The proof should include:
  - required-tool availability,
  - workstation/path evidence,
  - lawful believed acquisition evidence for each missing input,
  - merged evidence places/entities so downstream planning keeps the relevant substrate.
- Preserve belief-only locality. Do not read authoritative world state directly.

### 2. Remove the top-level proxy substitution for reachable production

- Stop using `emit_missing_recipe_input_goals(...)` as the competing emitted goal for cases where the downstream production path is already believed reachable.
- If `emit_missing_recipe_input_goals(...)` becomes dead after the reassessed implementation, remove it rather than leaving an unused alias path behind.
- Do not add a ranking-side workaround for duplicated semantic candidates unless the code review proves there is a remaining legitimate non-overlapping use case.

### 3. Update focused coverage around the corrected boundary

- Replace the stale suppression test with coverage that asserts reachable missing-input production emits `ProduceCommodity`.
- Add negative coverage showing `ProduceCommodity` remains absent when the actor lacks a lawful production path, such as missing workstation evidence.
- Keep the runtime/golden regression focused on end-to-end execution rather than candidate introspection.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify)
- `crates/worldwake-ai/tests/golden_production.rs` (modify only if existing golden coverage proves insufficient during implementation)

## Out of Scope

- Search heuristic changes in [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs)
- Goal-model changes in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) already delivered by S12 search work
- Global removal of `CommodityPurpose::RecipeInput`
- Decision-trace schema changes
- New materialization-barrier semantics beyond current planner/runtime behavior

## Acceptance Criteria

### Tests That Must Pass

1. Focused candidate-generation coverage proves a recipe with reachable missing-input procurement emits `GoalKind::ProduceCommodity { recipe_id }`
2. Focused candidate-generation coverage proves that same scenario does not emit `AcquireCommodity { purpose: CommodityPurpose::RecipeInput(recipe_id) }` as a competing top-level proxy
3. Focused candidate-generation coverage proves `ProduceCommodity` is still withheld when there is no lawful believed production path
4. Existing golden regression: `cargo test -p worldwake-ai golden_acquire_commodity_recipe_input`
5. Existing golden regression: `cargo test -p worldwake-ai golden_multi_recipe_craft_path`
6. Existing suite: `cargo test -p worldwake-ai`
7. Existing lint gate: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Candidate generation names downstream desired world conditions rather than substituting prerequisite proxy goals when the full production path is believed reachable
2. Production candidate emission remains belief-driven and locality-respecting; no authoritative shortcutting is introduced
3. Missing-input procurement, workstation access, and barrier handling remain planner/runtime concerns beneath the emitted production goal

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — replace the stale missing-input suppression test with a reachable-path test that asserts `ProduceCommodity` is emitted and the `RecipeInput` proxy is absent
2. `crates/worldwake-ai/src/candidate_generation.rs` — add a negative-path test showing production is still withheld when missing-input production lacks workstation or acquisition evidence
3. `crates/worldwake-ai/tests/golden_production.rs` — none by default; reuse existing end-to-end production regressions unless implementation exposes a real runtime gap

### Commands

1. `cargo test -p worldwake-ai candidate_generation::tests::missing_recipe_input`
2. `cargo test -p worldwake-ai golden_acquire_commodity_recipe_input`
3. `cargo test -p worldwake-ai golden_multi_recipe_craft_path`
4. `cargo test -p worldwake-ai`
5. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-21
- What changed:
  - `emit_produce_goals(...)` now emits `ProduceCommodity` when missing inputs are still lawfully reachable through believed acquisition or sub-recipe paths.
  - The top-level `RecipeInput` proxy substitution was removed from production candidate generation instead of being preserved and rebalanced in ranking.
  - Focused candidate-generation coverage now asserts the corrected boundary, including a recursive sub-recipe case and a no-workstation negative case.
- Deviations from original plan:
  - No ranking changes were needed because the cleaner fix was to remove the competing proxy candidate at emission time.
  - No golden file changes were required; existing golden production coverage already proved the runtime path once the candidate boundary was corrected.
- Verification results:
  - `cargo test -p worldwake-ai candidate_generation::tests::missing_recipe_input`
  - `cargo test -p worldwake-ai golden_acquire_commodity_recipe_input`
  - `cargo test -p worldwake-ai golden_multi_recipe_craft_path`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
