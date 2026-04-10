# S81GLDGAP-004: Golden test S81-A -- multi-agent convergence

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: None

## Problem

No golden test verifies multi-agent behavior under resource scarcity at scale. S76-B (`golden_max_idle_under_remote_resource_scarcity`) tests 1 agent for 300 ticks. The observer report shows qualitatively different failure at 3+ agents / 600+ ticks due to candidate explosion and contention effects. This gap means multi-agent behavioral collapse (prolonged sleep+relieve loops) could regress undetected.

## Assumption Reassessment (2026-04-09)

1. S76-B test exists at `crates/worldwake-ai/tests/golden_simulation_gaps.rs:387` (`golden_max_idle_under_remote_resource_scarcity`). Helper at line 203 (`run_max_idle_under_remote_resource_scarcity`). Confirmed via grep. Uses 1 agent, 300 ticks, single remote resource location.
2. Golden test infrastructure: `golden_simulation_gaps.rs` already exists and is the correct file for S81 golden tests. S81-A test will be added to this file.
3. `GoalKind::AcquireCommodity`, `ConsumeOwnedCommodity`, `Sleep`, `Relieve` all exist in `crates/worldwake-core/src/goal.rs`. Confirmed via grep.
4. `ActionDomain::Travel`, `Production`, `Needs` all exist in `crates/worldwake-core/src/action_domain.rs`. Confirmed via grep.
5. `WorkstationTag::Well` and `OrchardRow` exist in `crates/worldwake-core/src/production.rs`. `KnownRecipes` component exists for recipe knowledge. Confirmed via grep.
6. Live owning suite check: `crates/worldwake-ai/tests/golden_simulation_gaps.rs` already owns Scenario 126 (`golden_remote_travel_when_local_supply_exhausted`) and Scenario 127 (`golden_max_idle_under_remote_resource_scarcity`), both single-agent scarcity cases. `S81GLDGAP-004` therefore extends the existing file with a materially different multi-agent contract rather than creating a new golden file.
7. Live recipe check: the golden harness default `seed_agent()` path seeds `KnownRecipes::with([RecipeId(0)])` in `crates/worldwake-ai/tests/golden_harness/mod.rs:609-627`, which only guarantees `Harvest Apples`. If S81-A uses a real water resource source rather than a pre-placed water lot, the test must resolve and seed the `Harvest Water` recipe explicitly (for example via `h.recipes.recipe_by_name(\"Harvest Water\")`) instead of assuming the default harness recipes cover it.
8. Golden-proof correction: the live S76-B helper does not literally prove "only sleep and relieve actions"; it measures a bounded consecutive stretch with no action lifecycle activity and no active action in progress (`golden_simulation_gaps.rs:266-305`). S81-A should keep that stronger architecture-honest idle proof shape per `docs/golden-e2e-testing.md` rather than asserting a scheduler-coupled narrative about exact sleep/relieve-only ticks.
9. Golden-doc handoff correction: adding a new `// Scenario N:` block under `crates/worldwake-ai/tests/golden_simulation_gaps.rs` changes the generated golden inventory/docs. Per `docs/golden-e2e-testing.md`, the ticket's verification surface must include `python3 scripts/golden_inventory.py --write --check-docs`, and the scenario number must be chosen from the live repo-global sequence rather than assumed local to this file.
12. Scenario isolation: this test intentionally seeds beliefs about remote resources to isolate the travel-decision branch. Without seeded beliefs, agents cannot know about remote resources (P14) and would only sleep/relieve. The test verifies agents ACT on those beliefs, not that they discover resources independently (that is S80 exploration mechanics).

## Architecture Check

1. Multi-agent golden test extends the existing S76-B pattern. Reuses the same golden test infrastructure (world builder, harness, assertion helpers). No new framework needed.
2. No backward-compatibility shims.

## Verification Layers

1. No agent enters >200 consecutive ticks with no lifecycle activity and no active action -> action trace plus active-action state
2. At least one agent starts travel within 300 ticks -> action trace (ActionDomain::Travel committed)
3. At least one agent reaches resource location by tick 600 -> authoritative world state (effective_place query)
4. Single-layer golden E2E ticket: the contract is emergent multi-system behavior, not a single system invariant.

## What to Change

### 1. Add S81-A golden test to golden_simulation_gaps.rs

In `crates/worldwake-ai/tests/golden_simulation_gaps.rs`, add:

- Helper function `run_multi_agent_convergence(seed: Seed)`:
  - Create 3 agents at a barren indoor location (e.g., VILLAGE_SQUARE or equivalent with no food/water sources)
  - Create remote resource affordances for both food and water, using the live golden harness recipe/workstation setup
  - Seed each agent with beliefs about at least one remote resource location
  - Give each agent the exact live recipe IDs needed for the chosen remote resource setup (do not assume the default `seed_agent()` recipe set already includes water harvest)
  - Set elevated hunger/thirst needs
  - Run for 600 ticks
  - Track per-agent consecutive ticks with no action lifecycle activity and no active action
  - Assert: no agent exceeds 200 such consecutive idle ticks
  - Assert: at least one travel action started within 300 ticks
  - Assert: at least one agent at a resource-bearing location by tick 600

- Test function `golden_multi_agent_convergence()` with `#[test]`
- Deterministic replay test `golden_multi_agent_convergence_replays_deterministically()` following the existing pattern

### 2. Follow existing S76-B setup patterns

Use the same world-builder utilities, topology setup, and assertion patterns from `run_max_idle_under_remote_resource_scarcity` as a template. Scale up: 3 agents instead of 1, 600 ticks instead of 300, multiple remote resource locations.

### 3. Keep golden metadata and docs in sync

- Add the scenario header/comments in the live generator-friendly format used by other `golden_*` tests.
- Choose the next free repo-global `// Scenario N:` id at implementation time instead of assuming a local file number.
- Regenerate and validate the golden inventory/docs as part of the ticket handoff.

## Files to Touch

- `crates/worldwake-ai/tests/golden_simulation_gaps.rs` (modify -- add S81-A test)

## Out of Scope

- Fixing root causes of idle loops (S79, S80)
- Plan search budget tuning (CognitiveProfile already supports this)
- Observer tooling improvements
- Single-agent scarcity testing (already covered by S76-B)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_multi_agent_convergence` -- no agent >200 consecutive idle ticks, travel within 300 ticks, resource location reached by 600
2. `golden_multi_agent_convergence_replays_deterministically` -- same seed produces same outcome
3. Generated golden docs updated cleanly for the new scenario metadata
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Agents plan from beliefs only, never world state (P14) -- beliefs are seeded, not derived from omniscient access
2. All agent actions use the same lawful affordances (P19 agent symmetry)
3. Deterministic replay under same seed (ChaCha8Rng, BTreeMap)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_simulation_gaps.rs` -- 2 new test functions (golden + replay)
2. Generated docs under `docs/generated/` refreshed by the canonical golden inventory script

### Commands

1. `cargo test -p worldwake-ai --test golden_simulation_gaps golden_multi_agent_convergence`
2. `cargo test -p worldwake-ai --test golden_simulation_gaps golden_multi_agent_convergence_replays_deterministically`
3. `python3 scripts/golden_inventory.py --write --check-docs`
4. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-04-09.

- Added Scenario 130 in `crates/worldwake-ai/tests/golden_simulation_gaps.rs` covering three-agent remote-scarcity convergence with explicit remote food and water resource beliefs, exact harvest recipe seeding, bounded no-lifecycle idle assertions, travel-start observation, and OrchardFarm arrival proof.
- Added the matching deterministic replay test for the same scenario and seed.
- Regenerated the golden inventory/docs so the new scenario metadata is reflected in the generated coverage artifacts under `docs/generated/`.

## Deviations

- Reassessment showed the default `GoldenHarness::new(seed)` recipe registry only includes `Harvest Apples`. To keep the scenario aligned with the ticket's real remote water-resource setup, the implementation used `GoldenHarness::with_recipes(seed, build_multi_recipe_registry())` and seeded the resolved `Harvest Apples` and `Harvest Water` recipe IDs explicitly.
- Reassessment also corrected the idle proof to the live S76-B architecture-honest form: bounded consecutive ticks with no lifecycle activity and no active action, rather than a stronger but inaccurate “sleep/relieve only” narrative.

## Verification Result

- Passed focused tests:
  - `cargo test -p worldwake-ai --test golden_simulation_gaps golden_multi_agent_convergence`
  - `cargo test -p worldwake-ai --test golden_simulation_gaps golden_multi_agent_convergence_replays_deterministically`
- Passed `python3 scripts/golden_inventory.py --write --check-docs`
- Passed `cargo test -p worldwake-ai`
