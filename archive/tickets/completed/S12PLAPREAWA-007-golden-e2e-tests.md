# S12PLAPREAWA-007: Golden E2E tests for prerequisite-aware multi-hop planning

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `ranking.rs` and `candidate_generation.rs` must align `ProduceCommodity` semantics with recipe-output self-care behavior and reachable remote workstation evidence
**Deps**: S12PLAPREAWA-001 through S12PLAPREAWA-006 (all implementation and unit tests complete)

## Problem

The prerequisite-aware search enhancement needs end-to-end golden proof for the concrete branches promised by the S12 spec: remote care procurement and remote recipe-input procurement. The original ticket assumed both were still missing. Current code no longer matches that premise.

## Assumption Reassessment (2026-03-21)

1. Golden infrastructure exists in `crates/worldwake-ai/tests/` and exposes the needed harness and trace surfaces. Confirmed via current golden suites and helpers in `crates/worldwake-ai/tests/golden_harness.rs`.
2. The S12 planner architecture is already live in production code:
   - `GoalKindPlannerExt::prerequisite_places(...)` in `crates/worldwake-ai/src/goal_model.rs`
   - `PlanningBudget::max_prerequisite_locations` in `crates/worldwake-ai/src/budget.rs`
   - dynamic `combined_relevant_places(...)` and `SearchExpansionSummary.{combined_places_count, prerequisite_places_count}` in `crates/worldwake-ai/src/search.rs`
3. Focused coverage for that architecture is already live:
   - `goal_model::tests::prerequisite_places_treat_wounds_include_remote_controllable_medicine_lot`
   - `goal_model::tests::prerequisite_places_produce_commodity_include_missing_input_places`
   - `search::tests::search_expansion_summary_counts_prerequisite_places_for_remote_treat_wounds`
4. The remote care golden is already delivered in `crates/worldwake-ai/tests/golden_care.rs`:
   - `golden_healer_acquires_remote_ground_medicine_for_patient`
   - `golden_healer_acquires_remote_ground_medicine_for_patient_replays_deterministically`
5. The production-side golden named in current docs is not the same scenario promised here. `golden_acquire_commodity_recipe_input` in `crates/worldwake-ai/tests/golden_production.rs` is a local `VillageSquare` pickup of a local firewood lot, not a remote multi-hop prerequisite-acquisition branch.
6. The current generated golden inventory count is 137, confirmed by `python3 scripts/golden_inventory.py --write --check-docs`.
7. Corrected scope: this ticket no longer needs to add remote-care golden coverage. The remaining gap is the remote production/crafting golden plus any doc sync required by that addition.
8. The new remote production golden exposed two architectural contradictions in production code:
   - On 2026-03-21, a remote `Bake Bread` scenario generated `ProduceCommodity { recipe_id }` at tick 0, but ranking assigned it zero motive while `AcquireCommodity { commodity: Bread, purpose: SelfConsume }` exhausted search without goal places.
   - After the first outbound travel leg, reachable remote workstation evidence disappeared because recipe feasibility was tied to the actor's current place rather than the reachable prerequisite horizon.
   Corrected scope: this ticket now includes the ranking and candidate-generation fixes required for the remote production branch to remain selectable through the full multi-hop chain.

## Architecture Check

1. The current dynamic combined-place planner architecture is cleaner than the pre-S12 static terminal-only heuristic because it adds prerequisite guidance at the search boundary instead of hard-coding goal-specific scripted subplans. That is the right long-term shape for extensible GOAP search.
2. The ticket's original "add both goldens" scope is no longer architecturally honest. Remote care is already covered; duplicating it would just create redundant goldens.
3. The remaining missing branch is the production-side remote prerequisite path. Adding that golden is still valuable because it validates that the same search architecture works across a second goal family under the real AI loop.
4. The clean engine fix is to align `GoalKind::ProduceCommodity` ranking with recipe-output need semantics and keep reachable workstation evidence tied to the search horizon, not to reintroduce proxy `RecipeInput` goals, increase budgets blindly, or add special-case remote-production fallback logic.
5. No backward-compatibility layer is needed. Correct the live ranking and candidate-generation paths directly.

## Verification Layers

1. Remote care prerequisite search remains live -> existing golden care scenario and replay companion
2. Remote production prerequisite search becomes selectable through the real AI loop -> new golden production scenario with authoritative world-state and trace assertions
3. Search trace still exposes prerequisite-aware guidance -> focused `search.rs` coverage plus new golden decision-trace assertion if needed
4. `ProduceCommodity` ranking now reflects recipe-output hunger/thirst drive when the recipe serves self-consume -> focused `ranking.rs` coverage
5. Deterministic replay remains stable -> replay companion for the new remote production scenario
6. Golden docs remain truthful -> generated inventory plus any necessary `docs/golden-e2e*` updates
7. All 137 current golden tests remain passing after the new additions -> regression gate

## What to Change

### 1. Verify the already-landed remote care branch

Do not add another remote-care golden. Treat the existing `golden_healer_acquires_remote_ground_medicine_for_patient` coverage in `crates/worldwake-ai/tests/golden_care.rs` as the delivered S12 care-side proof and verify it still passes.

### 2. Add the missing remote production golden

Add a production-side remote prerequisite scenario in `crates/worldwake-ai/tests/golden_production.rs`.

**Scenario**: Agent starts at the local crafting place with the required workstation, but the recipe input lot is only available at a remote place. The setup must remove local lawful alternatives so the intended branch is truly remote prerequisite acquisition.

**Expected emergent behavior**:
1. Travel(local craft place → remote input place)
2. PickUp(remote input lot)
3. Travel(remote input place → local craft place)
4. Craft(local recipe)
5. Replan/consume output if the scenario uses a hunger-driven production chain

**Assertions**:
- authoritative outcome proves the recipe input was acquired remotely and later consumed by crafting
- action trace proves the remote Travel→PickUp→Travel→Craft sequence in order
- decision trace proves the initial search exposed prerequisite guidance for the remote input branch
- replay companion proves deterministic world/event-log replay

### 3. Fix `ProduceCommodity` ranking and candidate generation so the remote branch is actually selectable

In `crates/worldwake-ai/src/ranking.rs`, align `GoalKind::ProduceCommodity { recipe_id }` with the recipe-output drive semantics already used by the old `RecipeInput` proxy path:

- use `recipe_output_priority(...)` instead of flat medium priority when the recipe output serves self-consume
- use `recipe_output_motive_score(...)` / `recipe_output_provenance(...)` instead of enterprise-only opportunity scoring when the recipe output serves self-consume
- preserve enterprise behavior for purely enterprise recipes

In `crates/worldwake-ai/src/candidate_generation.rs`, remove the stale self-consume acquire proxy for recipe-backed remote production and keep workstation feasibility tied to reachable places within the travel horizon rather than the actor's current place only.

This keeps the top-level goal truthful (`ProduceCommodity`) while letting the remote prerequisite search do the decomposition beneath it.

### 4. Update golden docs and generated inventory if the new scenario lands

Synchronize `docs/generated/golden-e2e-inventory.md` and update `docs/golden-e2e-scenarios.md` / `docs/golden-e2e-coverage.md` only where the current text is no longer truthful.

## Files to Touch

- `crates/worldwake-ai/tests/golden_production.rs` (modify — add remote prerequisite production golden + replay companion)
- `crates/worldwake-ai/src/ranking.rs` (modify — align `ProduceCommodity` ranking with recipe-output drive semantics)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — remove recipe-backed self-consume acquire proxy and preserve reachable remote workstation evidence)
- `docs/generated/golden-e2e-inventory.md` (generated refresh)
- `docs/golden-e2e-scenarios.md` and/or `docs/golden-e2e-coverage.md` if scenario descriptions or counts need correction

## Out of Scope

- Re-adding already-delivered remote-care goldens
- Reworking planner/search architecture already covered by S12PLAPREAWA-001 through 006
- `GoldenHarness` refactors unrelated to the new scenario
- Trade→Craft materialization-barrier chains that require hypothetical materialization beyond current S12 scope

## Acceptance Criteria

### Tests That Must Pass

1. Existing `golden_healer_acquires_remote_ground_medicine_for_patient` and replay companion still pass
2. New remote production prerequisite golden passes
3. New remote production replay companion passes
4. Focused ranking coverage proves `ProduceCommodity` now carries self-consume motive/priority for recipe outputs
5. Current golden suite plus new additions passes: `cargo test -p worldwake-ai golden`
6. Full workspace passes: `cargo test --workspace`

### Invariants

1. Agents plan from beliefs only — no omniscient setup shortcut
2. Remote care remains covered by the existing canonical golden
3. `ProduceCommodity` no longer loses to a zero-motive ranking path when the recipe output is the real self-care target, and remote self-consume production no longer falls back to a stale `AcquireCommodity` proxy
4. Remote production now has canonical golden coverage with deterministic replay
5. Conservation holds for the chosen recipe-input commodity and crafted output
6. Perception/knowledge setup is explicit for agents that must observe remote state or post-craft output

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_production.rs` — add 1 remote-production golden + 1 replay companion
2. `crates/worldwake-ai/src/ranking.rs` — add focused ranking regression coverage for self-consume and enterprise production motive/priority
3. `crates/worldwake-ai/src/candidate_generation.rs` — add focused regression coverage for proxy removal and reachable remote workstation evidence
4. `docs/generated/golden-e2e-inventory.md` — refresh generated count
5. `docs/golden-e2e-scenarios.md` / `docs/golden-e2e-coverage.md` — update stale narrative text and counts

### Commands

1. `cargo test -p worldwake-ai golden_healer_acquires_remote_ground_medicine_for_patient`
2. `cargo test -p worldwake-ai golden_acquire_commodity_recipe_input`
3. `cargo test -p worldwake-ai golden_remote_acquire_commodity_recipe_input`
4. `cargo test -p worldwake-ai produce_commodity`
5. `cargo test -p worldwake-ai golden`
6. `python3 scripts/golden_inventory.py --write --check-docs`
7. `cargo test --workspace`
8. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Delivered more than the original stale ticket described, because the missing remote production golden exposed two real engine contradictions that had to be corrected for the S12 architecture to remain honest.

1. Verified the existing remote-care S12 proof instead of duplicating it.
2. Added `golden_remote_acquire_commodity_recipe_input` plus its deterministic replay companion.
3. Fixed `ProduceCommodity` ranking so recipe outputs inherit self-care drive semantics when appropriate while preserving enterprise scoring for purely enterprise outputs.
4. Removed the stale self-consume acquire proxy for recipe-backed remote production and kept reachable workstation evidence tied to the travel horizon, so the production goal survives the outbound travel leg instead of disappearing mid-chain.
5. Refreshed golden docs and inventory to 137 tests and corrected the scenario narratives to distinguish the old local recipe-input golden from the new remote multi-hop production chain.
