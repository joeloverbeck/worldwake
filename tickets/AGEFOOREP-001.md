# AGEFOOREP-001: Agents replenish food under hunger pressure when stock is exhausted

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — `worldwake-ai` candidate generation / goal ranking (`AcquireCommodity` / `ProduceCommodity` emission under hunger pressure), agent belief about reachable food sources during multi-activity scenarios
**Deps**: Follows from `archive/specs/S178-perishable-food-spoilage.md` (the change that surfaced this gap). No spec authored yet — this ticket may need to be promoted to an S-series spec given its blast radius (see Architecture Check).

## Problem

Agents do not reliably pursue food **production / acquisition** under hunger pressure once their owned food stock is gone. In the five survival scenarios below, the agent survives the full 1440-tick run purely on its **durable starting apple stock** — it never harvests or otherwise replenishes food, even when starving.

This was latent and harmless until S178 (perishable food spoilage) landed. Before S178, apples gave full hunger relief regardless of age, so a fixed starting stock of ~9 apples lasted almost exactly the full run and the agent stayed (barely) fed. After S178, the same apples give progressively less relief as they stale (`Stale` relief scales linearly toward zero near the spoiled threshold; `Spoiled` relief floors at `Permille(150)`), so the stock is exhausted ~340 ticks early and the agent then **starves for the remainder of the run**.

**Localization (per-tick goal trace of `survival-combat`, Sentinel Rowan):**

- While the agent owns apples it pursues `GoalKind::ConsumeOwnedCommodity { Apple }`.
- The moment the apples are gone (t≈660), the agent generates **`goal = none` for ~500 consecutive ticks at hunger = 1000 (maximal)** — it emits no food-acquisition goal at all.
- It finally emits `GoalKind::AcquireCommodity { Apple, purpose: SelfConsume }` at t≈1180, but **cannot execute it** (no `harvest` action is ever committed; `committed = []`).
- The scenario authors a regenerating Apple resource source at both places (`regeneration_ticks_per_unit: 4`, capacity 80) and grants the agent `known_recipes: ["Harvest Apples", "Harvest Water"]`, so harvesting fresh food is in principle available — the agent simply does not pursue it.

The contrast that proves the gap is real and not a spoilage bug: on `main` the agent behaves identically (stock monotonically decreases, never replenished) and the survival assertion passes only because durable food coincidentally lasts the whole run. The S178 spoilage feature is correct — its own goldens (`survival-food-spoilage-*`, Golden Item Decay) are green. It merely removed the durable-food cushion that was silently compensating for this missing behavior.

Note that `survival_baseline` survives *with* spoilage enabled, which shows agents **can** stay fed under spoilage when they are not occupied with competing activities (combat, escort travel, trade negotiation, theft). The gap is specifically: agents busy with non-food goals do not interleave food replenishment before their stock is exhausted, and once exhausted do not reliably plan/execute acquisition.

## Interim mitigation already shipped (do not re-discover)

To recover CI on the S178 PR, the five affected scenarios were opted **out** of spoilage by authoring an empty `commodity_perish_profile: {}` in their `.ron` files (restoring pre-S178 durable-food behavior for exactly those non-food scenarios; FND-28-aligned — the S178 spec anticipated updating goldens that depend on old archive-only behavior). The `survival_baseline` scenario-diagnostics fixture was regenerated (it survives spoilage; its diagnostics legitimately drifted). This ticket is the deferred *real* fix: make agents replenish food so these scenarios can run **with** spoilage enabled, after which the `commodity_perish_profile: {}` opt-outs should be removed.

Affected scenarios (all opted out, all must be re-enabled when this lands):

1. `scenarios/survival-combat.ron` — `survival_combat_proves_combat_and_bandit_camp_abandonment`
2. `scenarios/survival-escort.ron` — `survival_escort_proves_coordinated_care_travel`
3. `scenarios/survival-trade.ron` — `survival_trade_proves_substitute_market_branch`
4. `scenarios/survival-theft.ron` — `survival_theft_proves_concealed_staged_lot_branch`
5. `scenarios/final-integration.ron` — `final_integration_proves_full_stack_coexistence`

## Assumption Reassessment (2026-06-02)

1. **Live goal family under test**: `GoalKind::ConsumeOwnedCommodity` (owned-food consumption), `GoalKind::AcquireCommodity { purpose: SelfConsume }`, and `GoalKind::ProduceCommodity { recipe_id }` in `crates/worldwake-ai/src/candidate_generation.rs`. The `AcquireCommodity` emitter exists and fires (3× in the combat run) but not during the t660–1180 starvation window; confirm the exact emission gate (`candidate_generation.rs` ~line 971, ~line 4278 "skip AcquireCommodity emission" path) and why hunger=1000 with zero owned food does not emit it for ~500 ticks.
2. **Belief substrate**: determine whether the agent lacks a belief about a reachable Apple source after the combat/escort/travel sequence (information-locality: the source belief may have decayed or never been seeded for the post-combat location), versus the candidate being emitted but rejected in `search_plan` / binding. Name the canonical belief accessor (`GoalBeliefView` resource-source / `known_harvest_recipe_supports_source`, `candidate_generation.rs:7097-7100`) before implementation.
3. **Why `AcquireCommodity` at t≈1180 fails to execute**: no `harvest` action committed. Trace whether the plan is found-but-unstartable (`BestEffort` start failure), found-but-replan-looping (`handle_plan_failure`), or never planned (terminal/operator surface). This decides whether the fix is in candidate generation, planning, or belief.
4. **Blast radius**: any change to hunger-driven food-acquisition emission triggers CLAUDE.md's Authoritative-to-AI Impact Rule and affects ALL survival goldens. Re-run the entire gated `golden-survival.yml` family, not just the five above.
5. **Mismatch + correction**: the original S178 implementation treated these five scenarios as unaffected by spoilage; reassessment shows they depend on durable-food behavior and were not updated. The opt-out is an interim correction, not the architectural fix.

## Architecture Check

The clean design is for agents under sustained hunger pressure, with owned food exhausted (or projected to be exhausted within a need horizon) and a believed/reachable food source, to emit and prioritize a food-acquisition goal (`AcquireCommodity`/`ProduceCommodity` → harvest) ahead of idling. This must be emergent (hunger pressure × belief about sources × recipe feasibility), not a scripted "harvest when hungry" trigger (FND-2), and must respect information locality (FND-7/FND-14) — the agent acts on *belief* about food sources, and "no reachable believed source" remains a legitimate reason to fail rather than reading world state.

Because this likely spans candidate generation, goal ranking, and belief about resource sources across multi-activity scenarios, evaluate whether it warrants promotion to an S-series spec rather than a single ticket.

## Verification Layers

- Food-acquisition goal emitted under hunger pressure when stock exhausted → decision trace (`generated_contains_goal(AcquireCommodity{food})` true in the starvation window; no ~500-tick `goal=none` run).
- Harvest action actually executes → action trace (`harvest` committed, owned apple count rises).
- Agent stays under the authored critical-hunger run with spoilage **enabled** → the five survival goldens with `commodity_perish_profile: {}` **removed**.
- No regression in sibling survival scenarios → full `golden-survival.yml` family re-run.
- Determinism preserved → each affected scenario's `*_replays_deterministically` golden.

## Tests

- Re-enable spoilage in the five scenarios (delete the `commodity_perish_profile: {}` opt-out + its comment) and require each survival assertion to pass.
- Focused candidate-generation test: agent at hunger ≥ critical with zero owned food and a believed reachable harvestable source emits `AcquireCommodity{food}` (guards against the `goal=none` regression).
- Focused test: the emitted acquisition goal yields a committed `harvest` (guards against the found-but-unstartable t≈1180 failure).

## What to Change

- `crates/worldwake-ai/src/candidate_generation.rs` — the hunger-driven `AcquireCommodity` / `ProduceCommodity` emission gate (suspect sites: ~line 971, ~line 4278, and `known_harvest_recipe_supports_source` ~line 7097). Determine and fix why emission is suppressed for ~500 ticks at maximal hunger with no owned food.
- Belief seeding / resource-source belief during multi-activity scenarios (if reassessment point 2 shows the agent lacks a source belief at its post-combat location).
- `scenarios/{survival-combat,survival-escort,survival-theft,survival-trade,final-integration}.ron` — remove the interim `commodity_perish_profile: {}` opt-out once agents survive with spoilage enabled.
