# S178PERFOOSPO-008: Perishable food spoilage golden coverage

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No — scenario authoring + golden test bindings only.
**Deps**: `archive/tickets/S178PERFOOSPO-001.md`, `archive/tickets/S178PERFOOSPO-002.md`, `archive/tickets/S178PERFOOSPO-003.md`, `archive/tickets/S178PERFOOSPO-004.md`, `archive/tickets/S178PERFOOSPO-005.md`, `archive/tickets/S178PERFOOSPO-006.md`, `tickets/S178PERFOOSPO-007.md`

## Problem

The spec's FND-31 validation requires three goldens covering: (a) lifecycle Fresh→Stale→Spoiled with per-storage-context rate differentiation, (b) cache spoilage with belief invalidation on arrival and the profile-gated desperation branch, (c) a 1440-tick CI-owned collision scenario proving the full systemic chain plus the spoilage-as-hoarding-dampener emergence. These goldens prove the integrated chain (foundation types → decay advancement → relief scaling → belief-view → candidate gating → forensics) end-to-end and cover the spec's Auth-to-AI Impact Analysis points 3 (`search_plan`), 4 (`BestEffort` action start), and 5 (`handle_plan_failure`) — none of which have dedicated code-change tickets because they emerge from existing planner machinery once D1-D8 land.

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Scenario structure follows the S177 precedent at `scenarios/survival-water-quality-on-arrival.ron` (lines 1-20: places, edges, agents, known recipes, commodity-decay map). Existing perishable-precedent scenario: `scenarios/survival-items-decay.ron` shows the `commodity_decay` authoring pattern. The new `commodity_perish_profile` `ScenarioDef` field (ticket 001) is the integration point; scenarios author it parallel to `commodity_decay`. Golden harness location: `crates/worldwake-ai/tests/scenarios/*.rs` modules with assertion bindings; `crates/worldwake-ai/tests/golden_harness/mod.rs` registers the modules.
2. Spec scenarios verified against current `specs/S178-perishable-food-spoilage.md` "Scenario Validation" section. Illegal-paths list: food relief unaffected by condition; a spoiled lot vanishing instead of transforming; eating spoiled food producing a wound/sickness (deferred); a planner candidate for a remote lot's freshness with no belief carrier; a global `food_freshness` aggregate; candidate-generation reading `world.get_component_perishable_state` directly for a remote lot. Each illegal path gets at least one negative assertion in the goldens. `LotOperation::Spoiled` is no longer an illegal "still unused" path after ticket `archive/tickets/S178PERFOOSPO-003.md` landed the item-decay emitter.
3. Shared abstraction boundary (precision-rules §5 — Verification Surface Mapping): the golden harness's scenario-execution surface, the action-trace assertion API, the decision-trace assertion API, and the `LocalSurvivalStateSummary.spoiled_food_discoveries` field from ticket 007. Each invariant maps to its strongest proof surface; no collapsing into generic "trace" assertions.
4. Scenario isolation (precision-rules §8): the lifecycle golden isolates a single agent + single perishable lot to prove condition arithmetic without contention; the cache golden isolates belief invalidation by having the agent travel a known distance during which the cache spoils; the 1440-tick golden permits multi-agent contention to surface the hoarding-waste dampener. Lawful competing affordances intentionally excluded from the focused scenarios (single-agent setups); the 1440-tick scenario permits them as part of the contract under test.

## Architecture Check

1. Three goldens cover distinct invariants at distinct proof surfaces:
   - Lifecycle: condition arithmetic + lineage emission + lot persistence (action/event-log layer)
   - Cache: belief invalidation + desperation gate + `SpoiledFoodDiscovery` outcome (decision-trace + forensic layer)
   - 1440-tick: full systemic chain + hoarding dampener + storage-context differentiation (long-run soak)
2. The 1440-tick scenario is CI-owned (long runtime); the focused goldens run in the default `cargo test` suite. This matches S177's split between focused goldens and the CI-owned 1440-tick scenario. The split prevents the long scenario from inflating default suite time while ensuring full validation in CI.

## Verification Layers

1. Lifecycle invariants:
   - condition arithmetic per storage context → action-trace assertion at each per-tick condition update.
   - `LotOperation::Spoiled` lineage entry → authoritative world state assertion on lot provenance.
   - `EventTag::ItemSpoiled` emission → event-log delta assertion.
   - Lot persistence post-spoilage → authoritative world state assertion (`world.has_component_item_lot`).
   - Relief scaling per band → action-commit hunger-delta assertion.
2. Cache invariants:
   - Pre-arrival decision-trace shows candidate emitted with `believed_condition == Fresh` evidence → decision-trace assertion.
   - Post-arrival decision-trace shows candidate re-ranked (no candidate if hunger < threshold; emitted Spoiled candidate if hunger ≥ threshold) → decision-trace assertion at the boundary tick.
   - `SpoiledFoodDiscovery` record written with `outcome` matching agent's choice → forensic-record assertion on `LocalSurvivalStateSummary`.
   - No direct `world.get_component_perishable_state` call in pre-arrival decision-trace (FND-14B compliance) → decision-trace negative assertion.
3. 1440-tick invariants:
   - Exact per-lot condition lineage (Spoiled provenance at expected tick per storage context) → lineage assertion.
   - Storage-context rate differentiation (container lots spoil at ~2× ground time per `storage_rates.container=500`) → per-storage-context lineage tick count assertion.
   - Hoarding-waste dampener: total spoiled-lot count correlates with over-acquisition above a numeric threshold → aggregate assertion (Section H #9a dampener).
   - Desperation-eat fires only above per-agent threshold (per-agent FND-22 differentiation via per-agent `spoiled_food_hunger_threshold`) → forensic-record outcomes split between `AteAnyway` and `TraveledToFallback` per profile.
   - Replay equivalence → deterministic state hash matches across replays (AGENTS.md Determinism invariant).
4. Auth-to-AI Impact points covered:
   - #3 `search_plan` — terminal ordering unchanged; emerges from passing goldens.
   - #4 `BestEffort` action start — Eat starts lawfully on spoiled lots when candidate is selected; verified by cache golden's AteAnyway branch.
   - #5 `handle_plan_failure` — existing replan machinery; exercised by cache golden's `TraveledToFallback` branch.

## What to Change

### 1. `survival-food-spoilage-lifecycle.ron`

A single agent on a single place observes a single perishable Apple lot across ~1000 ticks. The lot ages Fresh→Stale→Spoiled at the ground baseline rate (~720 ticks from Fresh to Spoiled per the pinned profile). Variant scenarios in the same file (or separate fixture sub-cases) exercise container and possessed storage contexts to verify rate multipliers. Asserts:
- Condition advances at the expected per-tick delta for each storage context (action-trace).
- `LotOperation::Spoiled` appended at the threshold-crossing tick (authoritative lineage).
- `EventTag::ItemSpoiled` emitted exactly once per lot (event-log delta).
- Lot persists post-spoilage (`world.has_component_item_lot(lot) == true`).
- Relief scales down at each band when the agent eats (Fresh full, Stale reduced, Spoiled floor — verified via hunger-delta).

### 2. `survival-food-spoilage-cache.ron`

A single agent at place A remembers a fresh Apple cache at place B. Travel distance is tuned so the cache crosses the `spoiled_threshold` during travel (e.g., travel time = ~500 ticks for Apple at ground baseline). The agent travels; on arrival observes spoilage. Asserts:
- Pre-arrival decision-trace shows candidate emitted with `believed_condition` Fresh band.
- Post-arrival decision-trace shows the FND-17 expectation mismatch resolved:
  - With `spoiled_food_hunger_threshold` set so the agent's hunger is below threshold at arrival → spoiled-Eat candidate suppressed; agent re-plans (e.g., travels to fallback or idles).
  - With profile tuned so hunger exceeds threshold at arrival → spoiled-Eat candidate emitted; agent eats anyway.
- `SpoiledFoodDiscovery` record written with appropriate `outcome` per branch.
- No `world.get_component_perishable_state` call appears in pre-arrival decision-trace (FND-14B regression guard).

### 3. `survival-food-spoilage-cache-1440.ron`

Multiple agents (3-4) draw from perishable stock distributed across ground, container, and possession contexts over 1440 ticks. Each agent has a different `spoiled_food_hunger_threshold` per FND-22 diversity. Asserts:
- Exact per-lot condition lineage with `Spoiled` provenance entries at expected ticks per storage context.
- Storage-context rate differentiation (container lots' `Spoiled` lineage entries arrive at ~2× the ground lots' ticks).
- Hoarding-waste dampener: aggregate `Spoiled`-event count correlates with over-acquisition (a concrete numeric threshold proves the dampener fires when stockpiled supply exceeds consumption capacity).
- Desperation-eat outcomes split by per-agent profile: agents with low `spoiled_food_hunger_threshold` produce `AteAnyway` records; agents with high threshold produce `TraveledToFallback` or `GaveUp` records.
- Replay equivalence: deterministic state hash matches across replays.

### 4. Golden test bindings

Add a new test module `crates/worldwake-ai/tests/scenarios/food_spoilage.rs` with three test functions (lifecycle, cache, 1440-tick) and the assertion helpers needed for the proof surfaces above. Register the module in `crates/worldwake-ai/tests/golden_harness/mod.rs` following the S177 binding pattern. The 1440-tick test is gated as a CI-owned golden following the existing S177 pattern (e.g., `#[ignore]` with a CI feature flag, or a dedicated CI test binary — verify the precedent during implementation).

## Files to Touch

- `scenarios/survival-food-spoilage-lifecycle.ron` (new)
- `scenarios/survival-food-spoilage-cache.ron` (new)
- `scenarios/survival-food-spoilage-cache-1440.ron` (new)
- `crates/worldwake-ai/tests/scenarios/food_spoilage.rs` (new — test bindings and assertion helpers)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify — register the new scenario module per S177's binding pattern)
- Likely: `docs/generated/golden-scenario-details/` regeneration via `python3 scripts/golden_inventory.py --write --check-docs` (per `tickets/README.md` golden-inventory contract)

## Out of Scope

- Modifications to engine code (all engine work lands in tickets 001-007).
- Disease/sickness consequences (deferred per spec Non-Goals).
- Composting action / disposal action (out of scope per S178).
- Preserving-place context goldens (deferred per spec Non-Goals — preserving-place is a future spec).
- `Grain` / `Bread` perishability goldens (deferred per spec Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. `golden_ai_survival_food_spoilage_lifecycle` — lifecycle invariants pass (condition arithmetic + lineage + event + persistence + relief scaling).
2. `golden_ai_survival_food_spoilage_cache` — belief invalidation + desperation gate (both above-threshold and below-threshold branches) + forensic outcome correct.
3. `golden_ai_survival_food_spoilage_cache_1440` — 1440-tick collision invariants (CI-owned).
4. Existing: `cargo test --workspace` (regression guard for tickets 001-007).
5. Golden inventory regeneration: `python3 scripts/golden_inventory.py --write --check-docs` exits cleanly.

### Invariants

1. Each illegal-path listed in the spec's "Illegal paths" section (lines 212-213) has at least one negative assertion in the goldens (none-fire assertion).
2. Replay equivalence holds across all three scenarios: deterministic state hash matches across replays (AGENTS.md Determinism invariant).
3. Per-storage-context rate differentiation is asserted concretely (not narratively): container Stale-threshold-crossing tick = 2 × ground Stale-threshold-crossing tick within tolerance.
4. The 1440-tick scenario's hoarding-dampener assertion uses a concrete numeric threshold tied to the scenario's authored stockpile vs. consumption rate, not a narrative "lots spoiled because of hoarding" claim.

## Test Plan

### New/Modified Tests

1. `scenarios/survival-food-spoilage-lifecycle.ron` — new RON scenario.
2. `scenarios/survival-food-spoilage-cache.ron` — new RON scenario.
3. `scenarios/survival-food-spoilage-cache-1440.ron` — new RON scenario (CI-owned).
4. `crates/worldwake-ai/tests/scenarios/food_spoilage.rs` — new test module with three test bindings + assertion helpers (action-trace, decision-trace, forensic-record).

### Commands

1. `cargo test -p worldwake-ai --test golden_ai survival_food_spoilage_lifecycle`
2. `cargo test -p worldwake-ai --test golden_ai survival_food_spoilage_cache`
3. `cargo test -p worldwake-ai --test golden_ai survival_food_spoilage_cache_1440` (CI-owned tier; locally via the configured ignored-test invocation)
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `./scripts/verify.sh`
