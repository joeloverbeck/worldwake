# AGEFOOREP-001: Agents replenish food under hunger pressure when stock is exhausted

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Large
**Engine Changes**: Yes — scenario bootstrap for anonymous resource-source harvest affordances
**Deps**: Follows from `archive/specs/S178-perishable-food-spoilage.md` (the change that surfaced this gap). Follow-up contracts were split to `archive/tickets/AGEFOOREP-002.md` and `tickets/AGEFOOREP-003.md`.

## Problem

Agents in direct-harvest survival scenarios could emit `AcquireCommodity { Apple, SelfConsume }` after owned food stock was exhausted, but could not execute harvest against anonymous scenario resource sources. Those sources were spawned as `Facility` + `ResourceSource` only; they lacked the `WorkstationMarker` required by canonical harvest recipes such as `Harvest Apples`.

This was latent and harmless until S178 (perishable food spoilage) landed. Before S178, apples gave full hunger relief regardless of age, so a fixed starting stock of ~9 apples lasted almost exactly the full run and the agent stayed (barely) fed. After S178, the same apples give progressively less relief as they stale (`Stale` relief scales linearly toward zero near the spoiled threshold; `Spoiled` relief floors at `Permille(150)`), so the stock is exhausted ~340 ticks early and the agent then **starves for the remainder of the run**.

**Localization (per-tick goal/action trace of `survival-combat`, Sentinel Rowan):**

- While the agent owns apples it pursues `GoalKind::ConsumeOwnedCommodity { Apple }`.
- The agent eventually emits `GoalKind::AcquireCommodity { Apple, purpose: SelfConsume }`, but **cannot execute it** (no `harvest` action is ever committed; `committed = []`) because the anonymous Apple source has no `WorkstationTag::OrchardRow`.
- The scenario authors a regenerating Apple resource source at both places (`regeneration_ticks_per_unit: 4`, capacity 80) and grants the agent `known_recipes: ["Harvest Apples", "Harvest Water"]`, so harvesting fresh food is in principle available once the scenario bootstrap exposes the correct workstation tag.

The contrast that proves the gap is real and not a spoilage bug: on `main` the agent behaves identically (stock monotonically decreases, never replenished) and the survival assertion passes only because durable food coincidentally lasts the whole run. The S178 spoilage feature is correct — its own goldens (`survival-food-spoilage-*`, Golden Item Decay) are green. It merely removed the durable-food cushion that was silently compensating for this missing behavior.

Note that `survival_baseline` survives *with* spoilage enabled, which shows agents **can** stay fed under spoilage when they have a lawful food path. The narrowed gap for this ticket is: direct-harvest scenarios authored anonymous resource sources that were planner-visible as sources but not action-startable as recipe workstations.

## Interim mitigation already shipped (do not re-discover)

To recover CI on the S178 PR, the five affected scenarios were opted **out** of spoilage by authoring an empty `commodity_perish_profile: {}` in their `.ron` files (restoring pre-S178 durable-food behavior for exactly those non-food scenarios). Reassessment split that broad group into three founded contracts:

Direct-harvest scenarios owned by this ticket:

1. `scenarios/survival-combat.ron` — `survival_combat_proves_combat_and_bandit_camp_abandonment`
2. `scenarios/survival-escort.ron` — `survival_escort_proves_coordinated_care_travel`
3. `scenarios/final-integration.ron` — `final_integration_proves_full_stack_coexistence`

Deferred founded contracts:

1. `scenarios/survival-trade.ron` — market restock/restage under spoilage, completed by `archive/tickets/AGEFOOREP-002.md`
2. `scenarios/survival-theft.ron` — theft survival under spoilage, owned by `tickets/AGEFOOREP-003.md`

## Assumption Reassessment (2026-06-02)

1. **Live goal family under test**: `GoalKind::ConsumeOwnedCommodity` (owned-food consumption) and `GoalKind::AcquireCommodity { purpose: SelfConsume }` for direct harvest-backed Apple acquisition. Reassessment showed the goal can be emitted; the failure boundary is authoritative action start/search binding against a source that lacks the recipe's workstation tag.
2. **Shared abstraction boundary**: scenario-authored resource-source definitions become authoritative `Facility`, `ResourceSource`, `ResourceExtractionQueues`, and, for anonymous harvest sources, the `WorkstationMarker` needed by canonical harvest recipes.
3. **Belief substrate**: planner-visible source evidence is lawful only when backed by local observation/belief of a source. This ticket does not add omniscient planner reads; it aligns the authoritative bootstrap with the existing source/workstation contract.
4. **Why `AcquireCommodity` failed to execute**: no `harvest` action committed because anonymous Apple resource sources did not carry `WorkstationTag::OrchardRow`, while `Harvest Apples` requires that tag.
5. **Mismatch + correction**: the original broad ticket grouped five scenarios. Reassessment found three founded contracts: direct harvest replenishment (this ticket), market supply restock/restage (`archive/tickets/AGEFOOREP-002.md`), and theft survival under spoilage (`tickets/AGEFOOREP-003.md`).
6. **FOUNDATIONS alignment**: this ticket preserves FND-3/FND-4 concrete source state and source/sink accounting, FND-8 action preconditions, FND-14B planner-visible input boundaries, FND-20 reusable planning over ordinary harvest affordances, and FND-31 causal proof rather than scenario-only survival.
7. **Deferred opt-outs at AGEFOOREP-001 closeout**: `survival-trade` and `survival-theft` retained explicit `commodity_perish_profile: {}` containment comments pointing at their follow-up tickets. The direct-harvest scenarios removed the opt-out in this closeout. `survival-trade` was later resolved by `archive/tickets/AGEFOOREP-002.md`; `survival-theft` remains owned by `tickets/AGEFOOREP-003.md`.

## Architecture Check

The clean design is to make scenario-authored anonymous harvest sources complete authoritative affordances: a source that supports Apple harvest should also expose the canonical OrchardRow workstation marker required by the recipe. This is not a survival shortcut; it repairs a mismatch between authored concrete source state and the existing action preconditions. Trade and theft are split because making those goldens pass requires different causal machinery and would otherwise turn this ticket into a workaround.

## Verified Layers

- Anonymous Apple/Grain/Water resource sources carry canonical harvest workstation markers -> focused scenario spawn unit test.
- Harvest action actually executes in direct-harvest survival scenarios -> workflow-shaped release golden filters for combat, escort, and final-integration.
- Agent stays under the authored critical-hunger run with spoilage enabled in direct-harvest scenarios -> same golden filters.
- Determinism preserved -> each affected direct-harvest scenario's `*_replays_deterministically` golden.
- Trade/theft opt-outs were explicit and linked to follow-up tickets at AGEFOOREP-001 closeout -> scenario file comments and ticket dependency references. `survival-trade` was later resolved by `archive/tickets/AGEFOOREP-002.md`; `survival-theft` remains linked to `tickets/AGEFOOREP-003.md`.

## Test Result

- Re-enabled spoilage in the direct-harvest scenarios: `survival-combat`, `survival-escort`, and `final-integration`.
- Kept `survival-trade` and `survival-theft` opt-outs in place at AGEFOOREP-001 closeout with comments naming `AGEFOOREP-002` and `AGEFOOREP-003`. `survival-trade` was later resolved by `archive/tickets/AGEFOOREP-002.md`.
- Added focused scenario spawn coverage that anonymous Apple sources have `WorkstationTag::OrchardRow`.
- Verified release golden filters for the three direct-harvest scenarios, including deterministic replay tests.

## Landed Changes

- `crates/worldwake-cli/src/scenario/mod.rs` — when spawning an anonymous `ResourceSourceDef` facility, attach the canonical harvest `WorkstationMarker` for Apple (`OrchardRow`), Grain (`FieldPlot`), and Water (`Well`).
- `scenarios/{survival-combat,survival-escort,final-integration}.ron` — removed the interim `commodity_perish_profile: {}` opt-out.
- `scenarios/{survival-trade,survival-theft}.ron` — kept the scoped opt-out at AGEFOOREP-001 closeout and pointed it to the follow-up ticket that owns the founded behavior. `survival-trade` was later resolved by `archive/tickets/AGEFOOREP-002.md`.
- `archive/tickets/AGEFOOREP-002.md` / `tickets/AGEFOOREP-003.md` — recorded the split follow-up contracts.

## Outcome

Completion date: 2026-06-02.

Direct harvest replenishment now has a complete authoritative scenario bootstrap path: anonymous Apple, Grain, and Water resource sources carry the canonical workstation tags required by their harvest recipes. The direct-harvest survival scenarios run with spoilage enabled. At AGEFOOREP-001 closeout, trade and theft remained explicitly contained by scoped opt-outs and active follow-up tickets because their founded contracts required market restock/restage and theft-survival design, not the anonymous-source bootstrap fix.

Outcome amended: 2026-06-02. The trade follow-up is now complete and archived at `archive/tickets/AGEFOOREP-002.md`; `survival-trade` runs with spoilage enabled through merchant restock/restage. Theft survival remains explicitly contained and owned by `tickets/AGEFOOREP-003.md`.

## Verification Result

- Passed `cargo test -p worldwake-cli --lib scenario::tests::test_spawn_facilities_and_sources -- --exact`
- Passed `cargo test -p worldwake-cli --lib`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_combat::`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_escort::`
- Passed `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::final_integration::`
- Waived `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_trade::` for AGEFOOREP-001 closeout; it was later completed by `archive/tickets/AGEFOOREP-002.md`.
- Waived `cargo test --release -p worldwake-ai --test golden_ai -- --ignored --test-threads=1 scenarios::survival_theft::` for AGEFOOREP-001 closeout; it remains owned by `tickets/AGEFOOREP-003.md`.
