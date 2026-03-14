# Phase 2 Dampening Audit

Date: 2026-03-14

This audit covers the implemented Phase 2 domains in `worldwake-systems` and `worldwake-ai`:

- Needs and care actions
- Production and transport-adjacent production actions
- Trade and merchant restock support
- Combat and healing
- AI enterprise candidate generation, ranking, and failure handling

The goal is to verify Principle 10 from [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md): every positive feedback loop in the simulation needs a concrete dampener in the world.

## Method

The audit reviewed the current code paths in:

- [needs.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs)
- [needs_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs_actions.rs)
- [production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production.rs)
- [production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs)
- [trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade.rs)
- [trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs)
- [combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs)
- [enterprise.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/enterprise.rs)
- [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs)
- [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs)
- [budget.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs)
- [failure_handling.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs)

The audit also used the existing unit, integration, and golden suites for these domains.

## Summary

No undamped Phase 2 simulation loop was found that requires a code change.

The main correction to the original ticket is architectural:

- The current trade layer does not yet model seller-side profit compounding. `SellCommodity` remains deferred to S04, so the real trade loop today is remembered unmet demand driving restock behavior.
- AI planning budgets and blocked-intent TTLs are real and useful, but they are planner guardrails, not physical dampeners. They should be documented separately, not counted as substitutes for world-state mechanisms.

## Needs

### Loop: unmet needs -> higher pressure -> impaired condition -> more unmet needs

This is the main degradative spiral in the needs system.

Physical dampeners:

- Consumables are concrete lots and are consumed explicitly by `eat` and `drink`.
- Self-care actions take duration-bearing actions rather than instant state flips.
- `sleep` reduces fatigue incrementally instead of resetting everything at once.
- `toilet` and `wash` require place/tool context and also consume time.
- Bladder overflow creates concrete waste and resets bladder pressure instead of creating an unbounded numeric climb.
- Death ends further escalation for that actor.

Evidence in code:

- Basal progression and exposure accumulation live in [needs.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs.rs).
- Relief actions live in [needs_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs_actions.rs).

Assessment:

- The loop is causally grounded.
- The dampeners are physical and embodied.
- No numeric clamp is acting as the sole stopping mechanism.

## Production

### Loop: successful harvesting/crafting -> more available goods -> more production opportunities

This is a productive amplification loop, but it is materially constrained.

Physical dampeners:

- Harvest is limited by `ResourceSource.available_quantity`.
- Regeneration is rate-limited by `regeneration_ticks_per_unit`; sources do not refill instantly.
- Crafting consumes staged input lots explicitly and archives them after use.
- Workstations are occupancy-gated through reservations and exclusive facility grants.
- Recipes require explicit tools, known recipes, inputs, and matching workstations.
- Work takes `work_ticks` and can be interrupted.
- Carry capacity and container capacity constrain how much output can move through the world at once.

Evidence in code:

- Regeneration is in [production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production.rs).
- Harvest/craft action legality, staging, and occupancy are in [production_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/production_actions.rs).

Assessment:

- Production currently scales only through concrete resources, tools, facilities, and actor time.
- There is no abstract throughput multiplier or runaway shortcut.

## Trade

### Loop: unmet demand observations -> enterprise restock goals -> stock moved to destination -> restock gap closes

This is the real Phase 2 trade amplification loop. The original ticket’s profit-compounding example does not match the current architecture because seller-side selling is still deferred.

Physical dampeners:

- `DemandMemory` ages out via per-agent retention windows, so stale observations decay away.
- Restock pressure disappears once stock is physically present at the destination.
- Cargo movement is limited by carry/load capacity.
- Markets are separated by travel time; stock cannot teleport.
- Trade transfers coin and commodity lots conservatively; no synthetic value is created.
- Trade only commits when both sides locally accept the bundle.

Evidence in code:

- Demand-memory pruning and restock candidate derivation are in [trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade.rs).
- Concrete trade transfer logic is in [trade_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/trade_actions.rs).
- Destination-local restock-gap analysis is in [enterprise.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/enterprise.rs).

Assessment:

- The loop is materially damped by memory decay, stock placement, locality, and conservation.
- No additional code change is warranted before S04.

## Combat

### Loop: wounds -> vulnerability -> more danger/combat exposure -> more wounds

Combat contains a clear positive injury spiral, but it is already constrained by concrete bodily state.

Physical dampeners:

- Dead agents are removed from further combat escalation.
- Actors who are dead, incapacitated, or in transit cannot legally initiate combat actions.
- Bleeding naturally clots over time.
- Natural recovery only occurs outside combat and only when hunger, thirst, and fatigue are below high thresholds.
- Healing requires concrete medicine, co-location, and treatment duration.
- Attack/heal actions are same-place, duration-bearing actions rather than abstract toggles.

Evidence in code:

- Wound progression, clotting, recovery, and fatalities are in [combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs).

Assessment:

- The combat spiral is bounded by mortality, incapacity, recovery preconditions, and explicit treatment costs.
- The original ticket’s reference to generic weapon depletion is not accurate for the implemented code and should not drive changes.

## AI Enterprise And Planning Stability

### World-facing loop: remembered demand -> enterprise pressure -> candidate generation -> stock movement -> gap closure

Physical dampeners:

- Enterprise signals are derived from concrete `DemandMemory`.
- Opportunity and restock signals collapse when demand disappears or destination stock closes the gap.
- Movement and restock remain bound by cargo, inventory, and travel constraints from the world model.

Evidence in code:

- Signal derivation is in [enterprise.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/enterprise.rs).
- Candidate emission uses those derived restock signals in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs).
- Enterprise motive ranking lives in [ranking.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs).

Assessment:

- This loop is already tied to physical state and locality.

### Planner churn risks: repeated failed goals, oscillation, and search explosion

These are not Principle 10 world loops. They are planning-stability concerns.

Planner guardrails:

- `PlanningBudget` limits candidates, search depth, beam width, node expansions, and travel horizon.
- `switch_margin_permille` reduces goal thrash at decision boundaries.
- `BlockedIntentMemory` suppresses recently failed goals until their TTL expires.
- Failure handling records concrete blocking facts such as no seller, missing input, workstation busy, or danger too high.

Evidence in code:

- Budget limits are in [budget.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/budget.rs).
- Blocking-fact derivation and TTL-based suppression are in [failure_handling.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/failure_handling.rs).

Assessment:

- These mechanisms are good architecture and should stay.
- They should not be mislabeled as physical dampeners.
- No redesign is needed here; the cleaner architecture is to keep planner guardrails separate from world-state causality.

## Cross-System Amplification Checks

### Needs x Combat

Low hunger/thirst/fatigue is required for natural wound recovery. This creates a coupling where deprivation can prolong injury, but the loop remains damped by concrete self-care actions, medicine, and mortality.

### Trade x Production

Production can create goods that later satisfy trade demand, but production remains bounded by source depletion, recipe inputs, facilities, and travel. Trade cannot create stock from abstract demand alone.

### Trade x AI Enterprise

Enterprise pressure is derived from remembered unmet demand, but those memories decay and destination-local stock closes the loop. This is the correct causal shape for Phase 2.

## Undamped Loops Requiring Changes

None found in the current Phase 2 implementation.

## Gate Note For Later Specs

All Phase 3+ specs should continue the FND-01 Section H pattern:

- identify amplifying loops
- identify the physical dampener for each simulation loop
- separate derived read-models and planner guardrails from authoritative world mechanisms

