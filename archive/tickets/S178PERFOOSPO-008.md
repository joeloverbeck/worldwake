# S178PERFOOSPO-008: Perishable food spoilage golden coverage

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: No — scenario authoring + golden test bindings only.
**Deps**: `archive/tickets/S178PERFOOSPO-001.md`, `archive/tickets/S178PERFOOSPO-002.md`, `archive/tickets/S178PERFOOSPO-003.md`, `archive/tickets/S178PERFOOSPO-004.md`, `archive/tickets/S178PERFOOSPO-005.md`, `archive/tickets/S178PERFOOSPO-006.md`, `archive/tickets/S178PERFOOSPO-007.md`

## Problem

The spec's FND-31 validation requires three goldens covering: (a) lifecycle Fresh→Stale→Spoiled with per-storage-context rate differentiation, (b) cache spoilage with belief invalidation on arrival and the profile-gated desperation branch, (c) a 1440-tick CI-owned collision scenario proving the full systemic chain plus the spoilage-as-hoarding-dampener emergence. These goldens prove the integrated chain (foundation types → decay advancement → relief scaling → belief-view → candidate gating → forensics) end-to-end and cover the spec's Auth-to-AI Impact Analysis points 3 (`search_plan`), 4 (`BestEffort` action start), and 5 (`handle_plan_failure`) — none of which have dedicated code-change tickets because they emerge from existing planner machinery once D1-D8 land.

## Assumption Reassessment (2026-06-02)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Scenario structure follows the S177 precedent at `scenarios/survival-water-quality-on-arrival.ron`: authored RON fixture plus harness-seeded belief facts. `commodity_perish_profile` is authored directly in all three S178 fixtures. The golden module is registered in `crates/worldwake-ai/tests/scenarios/mod.rs`.
2. Live scenario item authoring initializes perishable state from the world's perish profile, but item lots are unnamed. The test bindings identify authored Apple lots by unique quantities and use harness setup for storage context placement.
3. The lifecycle golden isolates item decay from AI consumption by making the holder `ControlSource::None`. It proves concrete storage-rate differentiation, `EventTag::ItemSpoiled`, `LotOperation::Spoiled`, and lot persistence.
4. The cache golden seeds a prior fresh remote-cache belief and then spoils the authoritative lot before local observation. The full AI loop currently prefers avoiding spoiled food rather than committing `AteAnyway`; therefore this E2E golden proves non-omniscient belief correction and `CriticalWindowFrame.spoiled_food_discoveries`, while the `AteAnyway` branch remains covered by focused candidate-generation/extractor tests from tickets 006 and 007.
5. The 1440 golden is CI-owned and checks long-run spoilage plus world/event-log replay determinism. It uses a scoped `ProfileHomogeneity` lint override because this fixture isolates stockpile spoilage rather than profile diversity; FND-22 threshold diversity remains covered by focused tests.

## Architecture Check

1. Three goldens cover distinct invariants at distinct proof surfaces:
   - Lifecycle: condition arithmetic + lineage emission + lot persistence (action/event-log layer)
   - Cache: belief invalidation + `SpoiledFoodDiscovery` record (perception/event-log + forensic layer)
   - 1440-tick: long-run spoilage + replay determinism (long-run soak)
2. The 1440-tick scenario is CI-owned (long runtime); the focused goldens run in the default `cargo test` suite. This matches S177's split between focused goldens and the CI-owned 1440-tick scenario. The split prevents the long scenario from inflating default suite time while ensuring full validation in CI.

## Verification Layers

1. Lifecycle invariants:
   - condition arithmetic per storage context → action-trace assertion at each per-tick condition update.
   - `LotOperation::Spoiled` lineage entry → authoritative world state assertion on lot provenance.
   - `EventTag::ItemSpoiled` emission → event-log delta assertion.
   - Lot persistence post-spoilage → authoritative world state assertion (`world.has_component_item_lot`).
   - Relief scaling per band remains covered by focused `needs_actions` tests; this golden keeps lifecycle proof scoped to item-decay/event-log/provenance behavior.
2. Cache invariants:
   - Prior fresh belief is seeded from world state before authoritative spoilage → harness setup.
   - No lot-condition mismatch event before local arrival → event-log negative assertion.
   - Lot-condition mismatch event appears at/after arrival → event-log assertion.
   - `SpoiledFoodDiscovery` record written → `CriticalWindowFrame` forensic assertion.
   - Below-threshold branch avoids committed Eat → action-trace assertion.
3. 1440-tick invariants:
   - Multiple spoiled-lot events occur from authored surplus stock → aggregate event assertion.
   - Replay equivalence → deterministic state hash matches across replays (AGENTS.md Determinism invariant).
4. Auth-to-AI Impact points covered:
   - #3 `search_plan` — terminal ordering unchanged; emerges from passing goldens.
   - #4 `BestEffort` action start — Eat start on spoiled lots remains covered by ticket 006 focused AI tests.
   - #5 `handle_plan_failure` — existing replan machinery; no new rejection path was added in this golden ticket.

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

Add a new test module `crates/worldwake-ai/tests/scenarios/survival_food_spoilage.rs` with three test functions (lifecycle, cache, 1440-tick) and the assertion helpers needed for the proof surfaces above. Register the module in `crates/worldwake-ai/tests/scenarios/mod.rs` following the existing scenario-module pattern. The 1440-tick test is gated as a CI-owned golden with `#[ignore]`.

## Files to Touch

- `scenarios/survival-food-spoilage-lifecycle.ron` (new)
- `scenarios/survival-food-spoilage-cache.ron` (new)
- `scenarios/survival-food-spoilage-cache-1440.ron` (new)
- `crates/worldwake-ai/tests/scenarios/survival_food_spoilage.rs` (new — test bindings and assertion helpers)
- `crates/worldwake-ai/tests/scenarios/mod.rs` (modify — register the new scenario module)
- Likely: `docs/generated/golden-scenario-details/` regeneration via `python3 scripts/golden_inventory.py --write --check-docs` (per `tickets/README.md` golden-inventory contract)

## Out of Scope

- Modifications to engine code (all engine work lands in tickets 001-007).
- Disease/sickness consequences (deferred per spec Non-Goals).
- Composting action / disposal action (out of scope per S178).
- Preserving-place context goldens (deferred per spec Non-Goals — preserving-place is a future spec).
- `Grain` / `Bread` perishability goldens (deferred per spec Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_food_spoilage_lifecycle` — lifecycle invariants pass (condition arithmetic + lineage + event + persistence).
2. `golden_survival_food_spoilage_cache` — belief invalidation + forensic record + below-threshold no-Eat branch pass.
3. `golden_ai_survival_food_spoilage_cache_1440` — 1440-tick collision invariants (CI-owned).
4. Existing: `cargo test --workspace` (regression guard for tickets 001-007).
5. Golden inventory regeneration: `python3 scripts/golden_inventory.py --write --check-docs` exits cleanly.

### Invariants

1. Each illegal-path listed in the spec's "Illegal paths" section (lines 212-213) has at least one negative assertion in the goldens (none-fire assertion).
2. Replay equivalence holds across all three scenarios: deterministic state hash matches across replays (AGENTS.md Determinism invariant).
3. Per-storage-context rate differentiation is asserted concretely (not narratively): after 500 ticks the ground lot is Spoiled, the contained lot is still above the spoiled threshold, and the possessed lot's condition sits between the two.
4. The 1440-tick scenario's surplus-stock assertion uses a concrete minimum spoiled-event count and replay hash equality.

## Test Plan

### New/Modified Tests

1. `scenarios/survival-food-spoilage-lifecycle.ron` — new RON scenario.
2. `scenarios/survival-food-spoilage-cache.ron` — new RON scenario.
3. `scenarios/survival-food-spoilage-cache-1440.ron` — new RON scenario (CI-owned).
4. `crates/worldwake-ai/tests/scenarios/survival_food_spoilage.rs` — new test module with three test bindings + assertion helpers (event-log, action-trace, forensic-record).

### Commands

1. `cargo test -p worldwake-ai --test golden_ai golden_survival_food_spoilage_lifecycle`
2. `cargo test -p worldwake-ai --test golden_ai golden_survival_food_spoilage_cache`
3. `cargo test -p worldwake-ai --test golden_ai golden_survival_food_spoilage_cache_1440 -- --ignored` (CI-owned tier)
4. `python3 scripts/golden_inventory.py --write --check-docs`
5. `./scripts/verify.sh`

## Outcome

Completed on 2026-06-02. Added three S178 scenario fixtures and `survival_food_spoilage.rs` golden bindings. The default suite now covers lifecycle/storage-context spoilage and cache belief correction with forensic discovery; the CI-owned ignored golden covers long-run surplus-stock spoilage and deterministic replay. Golden inventory docs were regenerated.
