# Golden E2E Suite: Scenario Catalog

**Date**: 2026-03-12 (updated 2026-03-18)
**Scope**: `crates/worldwake-ai/tests/golden_*.rs` (split across domain files, shared harness in `golden_harness/mod.rs`)
**Purpose**: Detailed reference for what each golden test proves. Consult when you need to understand a specific scenario or verify whether a behavior is already tested. For coverage gaps and matrices, see [golden-e2e-coverage.md](golden-e2e-coverage.md).

---

The golden suite contains 91 tests across 9 domain files. Every test uses the real AI loop (`AgentTickDriver` + `AutonomousControllerRuntime`) and real system dispatch. The social slice locks down autonomous Tell, suppression under survival pressure, bystander locality, entity-missing discovery, stale-belief correction, chain-length gossip cutoff, agent diversity via social_weight, and the full rumor→wasted-trip→discovery lifecycle. The determinism slice now includes a 200-tick 4-agent world-runs-without-observers proof. The AI decisions slice now includes utility-weight-driven goal divergence (Principle 20, survival vs enterprise). The emergent slice (golden_emergent.rs) proves cross-system care interactions: wound-vs-hunger priority resolution via concrete utility weights, care_weight diversity producing divergent behavior, care+travel to remote patients, and loot→self-care chains. All behavior is emergent.

---

### Scenario 1: Goal Invalidation by Another Agent
**File**: `golden_ai_decisions.rs` | **Test**: `golden_goal_invalidation_by_another_agent`
**Systems exercised**: Needs, Production (resource source), Travel, AI (candidate generation, planning)
**Setup**: Two critically hungry agents (Alice, Bob) at Village Square. Alice has 1 bread. Orchard Farm has apples.
**Emergent behavior proven**:
- Alice eats the bread (ConsumeOwnedCommodity goal).
- Bob, finding no local food, travels to Orchard Farm to harvest apples (AcquireCommodity goal with travel sub-plan).
- Conservation: bread lots never increase.
**Cross-system chain**: Needs pressure → goal generation → plan search → action execution → resource consumption.

### Scenario 2: Priority-Based Interrupt
**File**: `golden_ai_decisions.rs` | **Test**: `golden_priority_based_interrupt`
**Systems exercised**: Needs (metabolism), AI (goal switching, interrupt evaluation)
**Setup**: Single agent (Cara) with high fatigue (pm(800)), low hunger (pm(300)), but extremely fast hunger metabolism (pm(50)/tick). Has 2 bread.
**Emergent behavior proven**:
- Agent starts sleeping (fatigue is the highest-priority need initially).
- Metabolism drives hunger past critical threshold (pm(900)) during sleep.
- AI interrupts sleep action and switches to eating bread.
**Cross-system chain**: Metabolism tick → need escalation → interrupt evaluation → goal switch → action termination → new action start.

### Scenario 2b: Buyer-Driven Trade Acquisition
**File**: `golden_trade.rs` | **Tests**: `golden_buyer_driven_trade_acquisition`, `golden_buyer_driven_trade_acquisition_replays_deterministically`
**Systems exercised**: Needs, AI (candidate generation, planning), Trade, Conservation, deterministic replay
**Setup**: Hungry buyer and sated seller co-located at Village Square. Seller advertises bread via `MerchandiseProfile`; buyer holds coins and a `TradeDispositionProfile`.
**Emergent behavior proven**:
- Buyer generates `AcquireCommodity { commodity: Bread, purpose: SelfConsume }` from hunger pressure.
- Planner resolves the acquire goal through a local trade barrier rather than unrelated travel branches.
- Trade executes through the real action handler: bread transfers to the buyer and coins transfer to the seller.
- Buyer then consumes the acquired bread, reducing hunger.
- Bread lots never increase and coin lots remain exactly conserved.
- Two runs with the same seed produce identical world and event-log hashes for the trade scenario.
**Cross-system chain**: Need pressure → seller discovery via `MerchandiseProfile` → planner trade barrier selection → trade valuation/exchange → consumption.

### Scenario 2c: Care Domain — Third-Party, Self-Care, Observation Gate, and Goal Invalidation
**File**: `golden_care.rs` | **Tests**: `golden_healing_wounded_agent`, `golden_healing_wounded_agent_replays_deterministically`, `golden_healer_acquires_ground_medicine_for_patient`, `golden_healer_acquires_ground_medicine_for_patient_replays_deterministically`, `golden_self_care_with_medicine`, `golden_self_care_with_medicine_replays_deterministically`, `golden_self_care_acquires_ground_medicine`, `golden_self_care_acquires_ground_medicine_replays_deterministically`, `golden_indirect_report_does_not_trigger_care`, `golden_indirect_report_does_not_trigger_care_replays_deterministically`, `golden_care_goal_invalidation_when_patient_heals`, `golden_care_goal_invalidation_when_patient_heals_replays_deterministically`
**Systems exercised**: AI (candidate generation, planning, `TreatWounds` goal family), Care action domain, Combat/wound treatment, Perception (direct-observation gate), Conservation, deterministic replay
**Setup**: Six care sub-scenarios are covered:
1. **Third-party care with medicine**: Healthy healer with medicine, co-located wounded patient. Healer directly observes wounds via passive perception and treats.
2. **Third-party care with ground medicine**: Healer without medicine, ground medicine available beside wounded patient. Healer picks up medicine then treats.
3. **Self-care with medicine**: Single wounded agent with own medicine. Agent emits `TreatWounds { patient: self }` and self-treats.
4. **Self-care with ground medicine acquisition**: Single wounded agent, no medicine in inventory, ground medicine at same place. Agent picks up medicine and self-treats.
5. **Indirect report does NOT trigger care**: Observer at Village Square with medicine. Wounded patient at Orchard Farm. Observer has Report-sourced belief about patient's wounds. Observer does NOT consume medicine and does NOT travel to patient — only `DirectObservation` triggers care.
6. **Care goal invalidation**: Patient with medicine and healer without medicine, co-located. Patient self-treats. Healer's `TreatWounds { patient }` goal is satisfied by patient's self-healing (healer never acquires medicine).
**Emergent behavior proven**:
- Healer generates `TreatWounds { patient }` from directly-observed wounded target via passive perception.
- When the healer lacks medicine but a wounded local target exists, candidate generation emits the care goal and the planner resolves acquisition through `pick_up` before healing.
- Self-care is lawful: wounded agents emit `TreatWounds { patient: self }` and consume own medicine.
- Self-care acquisition works: wounded agents pick up ground medicine and self-treat.
- Report-sourced wound beliefs do NOT trigger `TreatWounds` — only `PerceptionSource::DirectObservation` does (Principle 7 locality).
- When a patient self-heals, the healer's `TreatWounds` goal is satisfied (patient pain reaches zero) and drops cleanly.
- Planner selects the care-domain heal action through the real action registry.
- Heal executes through the normal lifecycle: medicine is consumed and the patient's wound load decreases.
- Two runs with the same seed produce identical world and event-log hashes for all care scenarios.
**Cross-system chain**: Wound state → passive perception seeds `DirectObservation` belief → `TreatWounds` goal emission (self or other) → planner resolves supply (pick_up/trade) → heal action → medicine consumption → wound severity/bleed reduction. Report-sourced beliefs are filtered by the direct-observation gate. Goal invalidation propagates when patient pain reaches zero.

### Scenario 2d: Merchant Restock and Return to Home Market
**File**: `golden_trade.rs` | **Tests**: `golden_merchant_restock_return_stock`, `golden_merchant_restock_return_stock_replays_deterministically`
**Systems exercised**: Enterprise AI signals, Travel, Production, Transport/cargo continuity, deterministic replay
**Setup**: Merchant starts at General Store with `MerchandiseProfile` advertising apples, zero apple stock, and remembered unmet apple demand at the home market. Orchard Farm has an apple resource source via OrchardRow workstation + `ResourceSource`.
**Emergent behavior proven**:
- Merchant generates the enterprise `RestockCommodity { Apple }` path from concrete remembered demand rather than from a magic stock threshold.
- Merchant leaves General Store, reaches Orchard Farm, and acquires apples through the real harvest path.
- Merchant controls apples away from the home market and later returns that stock to General Store, exercising `MoveCargo`.
- The current cargo architecture does not require a terminal `put_down` step here. Destination-local controlled stock is sufficient to satisfy the restock delivery path, and focused cargo tests already lock that invariant in.
- The scenario exposed a planner-budget gap: the default search node-expansion budget was too low for the branch-heavy restock route from Village Square. Raising the default node-expansion budget fixed the real runtime path without adding special cases.
- Two runs with the same seed produce identical world and event-log hashes for the merchant restock scenario.
**Cross-system chain**: Demand memory at home market → enterprise restock signal → multi-leg travel → harvest/materialization → cargo return to home market.

### Scenario 2e: Social Belief Sharing, Locality, and Discovery
**File**: `golden_social.rs` | **Tests**: `golden_agent_autonomously_tells_colocated_peer`, `golden_survival_needs_suppress_social_goals`, `golden_rumor_chain_degrades_through_three_agents`, `golden_stale_belief_travel_reobserve_replan`, `golden_skeptical_listener_rejects_told_belief`, `golden_bystander_sees_telling_but_gets_no_belief`, `golden_entity_missing_discovery_does_not_teleport_belief`, `golden_chain_length_filtering_stops_gossip`, `golden_agent_diversity_in_social_behavior`, `golden_rumor_leads_to_wasted_trip_then_discovery`
**Systems exercised**: Perception/beliefs, Tell actions, AI social candidate generation and ranking, self-care suppression, planner payload wiring, travel, zero-motive ranking filter, deterministic replay
**Setup**: Ten focused social scenarios cover colocated speaker/listener tell behavior, survival-need suppression of gossip, three-agent relay chains, stale harvest beliefs contradicted by local re-observation, hard listener rejection via `acceptance_fidelity: Permille(0)`, bystander witnessing without belief transfer, entity-missing discovery from a violated place expectation, speaker-side chain-length filtering that prevents infinite gossip propagation, agent diversity via `social_weight` (Principle 20), and the full rumor→wasted-trip→discovery information lifecycle.
**Emergent behavior proven**:
- Speakers can autonomously select `ShareBelief`, execute Tell, and cause listeners to replan from reported information.
- Critically hungry agents relieve survival pressure before any told-belief transfer can resolve, proving `ShareBelief` stays suppressed under high self-care pressure in the real AI loop.
- Relay chains degrade provenance and confidence across hops rather than duplicating the original direct observation unchanged.
- A stale believed `ResourceSource` quantity can pull an agent to travel, then be contradicted by local observation, forcing abandonment of the invalid harvest path.
- A skeptical listener can reject a told belief cleanly without mutating belief state or producing follow-up travel.
- A bystander can witness the same-place social act without receiving the underlying told belief content, preserving information locality.
- An agent can discover that an expected entity is absent from the locally re-observed place without the belief system inventing a replacement location.
- Chain-length filtering is speaker-side: a relay agent with `max_relay_chain_len=1` cannot relay a chain_len=2 rumor, preventing infinite gossip propagation through the 4-agent chain A→B→C→(blocked)→D.
- Agent diversity (Principle 20): agents with different `social_weight` values produce distinct social behavior — high-weight agents tell early, medium-weight agents tell eventually, zero-weight agents never tell because the zero-motive filter in `rank_candidates()` excludes goals with `motive_score == 0`.
- The full information lifecycle: a Rumor received via autonomous Tell drives travel to a depleted orchard, where passive observation emits a resource-source discrepancy discovery event, replacing the Rumor-sourced belief with DirectObservation of the actual empty state.
- The social slice exposed and fixed three architectural gaps: share-belief plans were only partially wired through planner payload/progress semantics, perception contradicted entity/location state but not stale `ResourceSource` quantities, and goals with zero motive score could still be planned and executed (now prevented by the zero-motive filter in ranking).
**Cross-system chain**: Belief pressure/opportunity → `ShareBelief` candidate generation and ranking → Tell execution and report propagation → listener replanning, while bystanders receive only witnessed-social evidence and local re-observation emits discovery when expectations are violated. The rumor lifecycle chains Tell → belief → travel plan → perception → discovery → belief correction → replan.
**Deferred boundary**: Social Tell currently propagates `BelievedEntityState`, not `DemandMemory`. A future market-demand communication feature should introduce its own explicit information carrier rather than coupling enterprise restock directly to Tell payloads. `GoalKind::InvestigateMismatch` (agent travels to verify a rumor proactively) is acknowledged as future work — substantial enough for a separate spec.

### Scenario 3: Resource Contention with Conservation
**File**: `golden_production.rs` | **Test**: `golden_resource_contention_with_conservation`
**Systems exercised**: Needs, Production, Travel, Conservation verification
**Setup**: Two critically hungry agents at Village Square. Alice has 1 bread. Orchard Farm has apples.
**Emergent behavior proven**:
- Both agents act concurrently under the same tick loop.
- Authoritative commodity totals (apple, bread) never increase — only decrease via consumption.
- Alice eats her bread. Event log grows (non-trivial simulation).
**Invariant enforced**: Per-tick authoritative conservation for both apple and bread commodities.

### Scenario 3d: Resource Exhaustion Race
**File**: `golden_production.rs` | **Tests**: `golden_resource_exhaustion_race`, `golden_resource_exhaustion_race_replays_deterministically`
**Systems exercised**: Needs, Production, AI planning/runtime, reservation handling, Conservation, deterministic replay
**Setup**: Four critically hungry agents start co-located at Orchard Farm with no food. One OrchardRow workstation exposes a finite `ResourceSource` with exactly `Quantity(4)` apples and no regeneration.
**Emergent behavior proven**:
- Multiple hungry agents can queue against the same finite source through the real AI loop without crashing the tick when same-tick reservation contention occurs.
- The orchard source is observed stepping `4 -> 2 -> 0`, proving exactly two harvest commits exhaust the stock.
- Apple lots materialize from the source and at least one agent completes the downstream harvest/pick-up/eat chain.
- Authoritative apple quantity never exceeds the initial 4, and the same-seed run replays to identical world and event-log hashes.
- The scenario exposed and fixed an engine gap: autonomous same-tick start requests were previously strict, so a second valid-from-snapshot harvest request could fail with `ReservationUnavailable` and abort the whole tick. The input path now distinguishes strict manual requests from best-effort autonomous requests, allowing the AI runtime to reconcile contention on the next tick without shims.
**Cross-system chain**: Shared hunger pressure at one place → multiple harvest plans from one snapshot → reservation-backed start contention → finite source depletion across two harvest commits → materialization/pick-up/eat → deterministic replay.

### Scenario 3b: Multi-Hop Travel Plan
**File**: `golden_ai_decisions.rs` | **Test**: `golden_multi_hop_travel_plan`
**Systems exercised**: Needs, AI (candidate generation, planning, replanning), Travel, Production
**Setup**: Critically hungry agent starts at Bandit Camp. Orchard Farm has apples via OrchardRow workstation + ResourceSource. The shortest route is `BanditCamp -> ForestPath -> NorthCrossroads -> EastFieldTrail -> OrchardFarm` (4 edges, 14 travel ticks).
**Emergent behavior proven**:
- Agent leaves Bandit Camp and traverses a real multi-edge route to the distant food source.
- AI replans cleanly after intermediate travel progress instead of reusing a stale pre-travel route prefix.
- Agent reaches Orchard Farm, harvests apples there, and reduces hunger.
**Cross-system chain**: Need pressure → distant acquire-goal emission → multi-hop plan search → sequential travel execution → harvest/materialization → downstream hunger relief.

### Scenario 3c: Goal Switching During Multi-Leg Travel
**File**: `golden_ai_decisions.rs` | **Test**: `golden_goal_switching_during_multi_leg_travel`
**Systems exercised**: Needs (metabolism), AI (candidate generation, ranking, replanning), Travel
**Setup**: Hungry agent starts at Bandit Camp with 1 carried water and no food. Orchard Farm remains the distant food source. Thirst starts low but escalates quickly enough to become critical only after the first travel leg completes.
**Emergent behavior proven**:
- Agent begins the distant hunger-driven journey and leaves Bandit Camp.
- Runtime establishes an active journey commitment to Orchard Farm, records travel progress after leg completion, and later exposes the same commitment as suspended during the local thirst detour.
- The penalty-interruptible travel action continues while thirst rises through the subcritical medium/high bands.
- The agent does not consume carried water before thirst reaches the critical threshold.
- Once thirst becomes critical, the running travel plan is interrupted and the agent consumes carried water at an intermediate concrete place on the route.
- After the detour, the runtime reactivates the original Orchard Farm commitment, unless the detour resolves at Orchard Farm itself.
- The journey is not treated as a rigid commitment to the original destination.
**Cross-system chain**: Hunger pressure → distant `AcquireCommodity` travel plan → journey commitment established → metabolism escalates thirst during journey → intermediate arrival triggers replanning and commitment suspension → `ConsumeOwnedCommodity { Water }` → commitment reactivation or arrival at destination.

### Scenario 4: Materialization Barrier Chain
**File**: `golden_production.rs` | **Test**: `golden_materialization_barrier_chain`
**Systems exercised**: Production (harvest), Transport (pick-up), Needs (eat), AI (multi-step replanning)
**Setup**: Agent (Dana) at Orchard Farm, critically hungry, no food. OrchardRow workstation with 20 apples in ResourceSource.
**Emergent behavior proven**:
- Agent plans and executes harvest action → apple lots materialize on the ground at workstation.
- Agent replans to pick up the materialized apples (transport action).
- Agent replans again to eat the acquired apples.
- Conservation: total apple authoritative quantity never exceeds initial 20.
**Cross-system chain**: Harvest (production output on ground) → replan → pick-up (transport) → replan → eat (needs). This is the longest emergent action chain in the suite.

### Scenario 5: Blocked Intent Memory with TTL Expiry
**File**: `golden_ai_decisions.rs` | **Test**: `golden_blocked_intent_memory_with_ttl_expiry`
**Systems exercised**: Production (resource regeneration), AI (blocked intent memory, TTL expiry)
**Setup**: Agent (Eve) at Orchard Farm, critically hungry. ResourceSource is depleted (available_quantity=0) but regenerates at 1 unit per 5 ticks.
**Emergent behavior proven**:
- With depleted source, agent cannot harvest immediately.
- Resource regeneration system restores apples over time (5 ticks/unit → 10 ticks to reach Quantity(2)).
- After regeneration, agent successfully harvests.
- BlockedIntentMemory may be recorded if a plan fails (observational, not required).
**Cross-system chain**: Depleted resource → failed/deferred plan → resource regeneration ticks → successful harvest.

### Scenario 6: Deterministic Replay Fidelity
**File**: `golden_determinism.rs` | **Test**: `golden_deterministic_replay_fidelity`
**Systems exercised**: All (determinism across entire stack)
**Setup**: Two agents (Alice, Bob), both hungry, at Village Square. Alice has bread. Orchard Farm has apples. Run for 50 ticks with same seed twice.
**Emergent behavior proven**:
- Identical seeds produce identical `StateHash` for both world and event log.
- World state differs from initial (non-trivial simulation occurred).
**Invariant enforced**: Full-stack determinism (ChaCha8Rng, BTreeMap ordering, no floats, no wall-clock).

### Scenario 6e: Save/Load Round-Trip Under AI
**File**: `golden_determinism.rs` | **Test**: `golden_save_load_round_trip_under_ai`
**Systems exercised**: AI loop, save/load, scheduler continuation, deterministic RNG continuation
**Setup**: The standard two-agent deterministic golden scenario runs for 20 ticks under the real AI loop, snapshots through `SimulationState`, round-trips through `save_to_bytes()` / `load_from_bytes()`, then resumes for 30 more ticks with a fresh `AgentTickDriver`. A parallel uninterrupted run continues for the same total 50 ticks.
**Emergent behavior proven**:
- The save boundary occurs after non-trivial AI progress rather than at an idle initial state.
- Save/load preserves the authoritative scheduler, controller state, recipe registry, and deterministic RNG continuation needed for resumed simulation.
- A freshly reconstructed AI controller runtime is sufficient to resume coherent behavior; no serialized `AgentDecisionRuntime` or `AutonomousControllerRuntime` is required.
- The resumed run reaches identical final world and event-log hashes, and matching scheduler/controller/RNG state, relative to uninterrupted execution.
**Cross-system chain**: AI planning/runtime progress → authoritative simulation snapshot → save/load round-trip → fresh controller reconstruction → identical continuation.

### Scenario 6a: Recipe-Input Acquisition Chain
**File**: `golden_production.rs` | **Test**: `golden_acquire_commodity_recipe_input`
**Systems exercised**: AI (candidate generation, ranking, planning), Transport, Production (craft), Needs, Conservation, deterministic replay
**Setup**: Hungry baker starts at Village Square with the `Bake Bread` recipe, a local mill, and no firewood. A single unpossessed firewood lot is available locally.
**Emergent behavior proven**:
- Candidate generation/ranking now surfaces `AcquireCommodity { commodity: Firewood, purpose: RecipeInput(bake_bread) }` as the missing bridge for the hunger-driven bread path.
- Baker first acquires the unpossessed firewood lot through the standard acquire path.
- Baker then crafts bread via the normal production action and consumes it to reduce hunger.
- Firewood is consumed exactly once, bread is produced and then consumed, and the same-seed run replays to identical world and event-log hashes.
**Cross-system chain**: Hunger pressure → missing recipe-input acquire goal → acquire local input lot → craft progress barrier → consume crafted output.

### Scenario 6b: Multi-Recipe Craft Path
**File**: `golden_production.rs` | **Test**: `golden_multi_recipe_craft_path`
**Systems exercised**: Production (craft with inputs), Transport, Needs, AI (recipe selection)
**Setup**: Agent (Miller) at Village Square with 1 firewood. Knows 3 recipes (harvest apples, harvest grain, bake bread). Mill workstation at Village Square.
**Emergent behavior proven**:
- Agent selects the bake bread recipe (requires firewood input, produces bread).
- Crafting consumes 1 firewood, produces 1 bread (conservation verified both ways).
- Agent consumes the crafted bread, reducing hunger.
- Deterministic replay: two runs with same seed produce identical hashes.
**Cross-system chain**: Recipe selection → craft action (input consumption + output materialization) → replan → eat.

### Scenario 6c: Capacity-Constrained Ground-Lot Pickup
**File**: `golden_production.rs` | **Test**: `golden_capacity_constrained_ground_lot_pickup`
**Systems exercised**: Production (harvest), Transport (partial pick-up split), Needs, AI (post-barrier replanning)
**Setup**: Agent (Porter) at Orchard Farm, critically hungry, carry capacity only 1 load unit. OrchardRow workstation has 10 apples; harvest outputs 2-apple ground lots.
**Emergent behavior proven**:
- Agent harvests apples, materializing a 2-apple ground lot.
- Because only 1 apple fits, the follow-up `pick_up` executes against the authoritative ground lot and materializes a split-off carried lot.
- Agent consumes one carried apple; one apple remains on the ground.
- Conservation checkpoints hold: 10 authoritative apples after harvest, then 9 after one apple is consumed.
- Deterministic replay: two runs with the same seed produce identical hashes.
**Cross-system chain**: Harvest materialization → replan → constrained pick-up split → consume.

### Scenario 6d: Materialized Output Theft Forces Fresh Replanning
**File**: `golden_production.rs` | **Test**: `golden_materialized_output_theft_forces_replan`
**Systems exercised**: Production (craft), Needs, AI runtime (progress barriers, fresh replanning), Travel, Conservation
**Setup**: Two hungry agents share Village Square. Crafter has firewood, knows `Bake Bread` and `Harvest Apples`, and has a local mill. Thief has no recipes and waits for local food. Orchard Farm provides the distant fallback food source.
**Emergent behavior proven**:
- Crafter crafts bread locally and the output materializes as an unowned ground lot.
- Thief opportunistically consumes that bread before Orchard Farm stock is touched, proving the local crafted output was actually contested.
- Crafter does not carry a stale bread-follow-up plan across the craft progress barrier and therefore does not record a stale `MissingInput(Bread)` blocker for this case.
- Crafter instead replans from updated authoritative state, travels to Orchard Farm, and recovers hunger there.
**Cross-system chain**: Local craft/materialization → opportunistic theft by another agent → progress-barrier replan from fresh state → distant harvest fallback → hunger relief.

### Scenario 3f: Faction-Owned Production — Member Retrieval vs Outsider Blocking
**File**: `golden_production.rs` | **Tests**: `golden_faction_ownership_producer_owner_delegation`, `golden_faction_ownership_producer_owner_delegation_replays_deterministically`
**Systems exercised**: Production (`resolve_output_owner` with `ProducerOwner`), Ownership (`can_exercise_control` institutional delegation via `factions_of`), AI (affordance filtering, GOAP planning, replan after blocked pickup), Travel, Needs, Conservation
**Setup**: Faction "River Pact" owns an orchard at Orchard Farm with `ProductionOutputOwner::ProducerOwner` policy. Agent Kael (faction member) and Agent Wren (outsider) are both critically hungry at Orchard Farm. A fallback Actor-policy orchard exists at Village Square.
**Emergent behavior proven**:
- Harvested apple lots materialize with the faction as owner (not the harvesting actor), proving `resolve_output_owner` with `ProducerOwner` routes ownership to the workstation's owner entity.
- Faction member picks up faction-owned apples via institutional delegation in `can_exercise_control` (faction membership check via `factions_of`).
- Member completes the full harvest → pickup → eat chain and reduces hunger.
- Outsider is blocked from picking up faction-owned apples (affordance filtering rejects the pickup), leaves Orchard Farm, and replans toward the fallback orchard.
- Outsider finds and eats from the Actor-policy fallback orchard at Village Square.
- Deterministic replay: two runs with the same seed produce identical world and event-log hashes.
- Per-tick conservation: authoritative apple total never exceeds the combined stock of both orchards (40).
**Cross-system chain**: ProducerOwner policy → faction-owned output materialization → institutional delegation pickup (member) / affordance blocking (outsider) → outsider travel + replan → fallback harvest → hunger relief for both agents.
**S01 code paths covered**: `resolve_output_owner()` ProducerOwner branch, `create_item_lot_with_owner()`, `believed_owner_of()`, `can_exercise_control()` faction delegation, affordance filtering by ownership, GOAP divergent planning based on faction membership.

### Scenario 7: Deprivation Cascade
**File**: `golden_ai_decisions.rs` | **Test**: `golden_deprivation_cascade`
**Systems exercised**: Needs (metabolism-driven escalation), AI (threshold-crossing detection)
**Setup**: Agent (Felix) starts with pm(0) hunger, fast metabolism (pm(20)/tick), has 1 bread.
**Emergent behavior proven**:
- Metabolism pushes hunger from 0 upward over time.
- When hunger crosses low threshold (pm(250)), AI generates a consume goal.
- Agent eats the bread.
**Cross-system chain**: Metabolism system → state change → AI threshold detection → goal generation → plan → action.

### Scenario 7a: Thirst-Driven Acquisition
**File**: `golden_ai_decisions.rs` | **Test**: `golden_thirst_driven_acquisition`
**Systems exercised**: Needs (metabolism-driven escalation), AI (threshold-crossing detection), Needs actions (drink)
**Setup**: Agent (Talia) starts with pm(0) thirst, fast thirst metabolism (pm(20)/tick), has 1 water.
**Emergent behavior proven**:
- Metabolism pushes thirst from 0 upward over time.
- When thirst crosses low threshold (pm(200)), AI generates a water consume goal.
- Agent drinks the water.
**Cross-system chain**: Metabolism system → thirst escalation → AI threshold detection → consume goal generation → drink action.

### Scenario 7e: Wash Action
**File**: `golden_ai_decisions.rs` | **Test**: `golden_wash_action`
**Systems exercised**: Needs, AI (candidate generation, planning), Needs actions (`wash`), Conservation
**Setup**: Agent at Village Square with high dirtiness and 1 controlled Water. All other needs are low.
**Emergent behavior proven**:
- High dirtiness emits the `Wash` goal through the real AI loop.
- Agent executes the real `wash` action without manual queueing.
- Dirtiness decreases and Water is consumed.
- Water lot totals never increase during the scenario.
**Cross-system chain**: Dirtiness pressure + local Water → `Wash` goal → wash action → dirtiness relief + Water consumption.

### Scenario 7b: Bladder Relief with Travel
**File**: `golden_ai_decisions.rs` | **Test**: `golden_bladder_relief_with_travel`
**Systems exercised**: Needs (bladder pressure), AI (candidate generation, planning), Travel, Needs actions (`toilet`)
**Setup**: Agent (Mira) starts at Village Square with elevated bladder pressure. Public Latrine is reachable in the prototype topology and `toilet` is only available at latrine-tagged places.
**Emergent behavior proven**:
- Bladder pressure emits the `Relieve` goal.
- Agent leaves Village Square and reaches Public Latrine instead of relieving locally.
- Relief completes at the latrine, reducing bladder pressure and materializing waste there without taking the bladder-accident path.
**Cross-system chain**: Bladder pressure → `Relieve` goal → travel to latrine-tagged place → `toilet` action → bladder relief + waste materialization.

### Scenario 7d: Three-Way Need Competition
**File**: `golden_ai_decisions.rs` | **Test**: `golden_three_way_need_competition`
**Systems exercised**: Needs, AI (candidate generation, ranking, replanning), Needs actions (`eat`, `drink`, `sleep`)
**Setup**: Single agent at Village Square with critical hunger, thirst, and fatigue. The agent carries 2 bread and 2 water, and its `UtilityProfile` weights hunger above thirst above fatigue.
**Emergent behavior proven**:
- The first started self-care action is `eat`, confirming that the runtime opens on the highest-weight hunger path under simultaneous local pressure.
- The agent also consumes water in the same local scenario rather than stalling on a single need.
- Fatigue is eventually reduced after thirst has been handled, proving the runtime keeps re-ranking instead of deadlocking.
- The scenario exposed a planner bug: hypothetical consume transitions were commodity-blind, so a `drink` step could satisfy a bread-consume goal in planning. Tightening consume transitions to require a commodity match fixed that without adding shims.
**Cross-system chain**: Simultaneous hunger + thirst + fatigue pressure → candidate generation → ranking picks hunger path first → interruptible local consume actions update needs → replanning handles thirst → later rest reduces fatigue.

### Scenario 7c: Hostility-Driven Living Combat
**File**: `golden_combat.rs` | **Tests**: `golden_combat_between_living_agents`, `golden_seed_sensitivity_living_combat_different_outcomes`, `golden_combat_between_living_agents_replays_deterministically`
**Systems exercised**: AI (hostility-driven candidate generation, planning), Combat (attack resolution, wounds), Conservation, deterministic replay
**Setup**: Two sated agents at Village Square with a concrete hostility relation. The attacker has a stronger combat profile; both carry coin so conservation can be checked across the fight.
**Emergent behavior proven**:
- The attacker generates a dedicated hostile-engagement goal from concrete local hostility and commits to the combat-domain `attack` action through the normal planner/runtime path.
- The defender responds through the live combat loop once the first attack is underway.
- At least one wound is inflicted on a living participant without either actor being pre-scripted via manual queueing.
- Coin totals remain exactly conserved throughout the encounter.
- A fixed set of distinct seeds produces more than one valid world/event-log outcome for the same living-combat setup, proving that this golden path is stochastic where the production architecture says it should be.
- Two runs with the same seed produce identical world and event-log hashes for the scenario.
**Cross-system chain**: Hostility relation → hostile-engagement goal → attack action start → wound infliction → defender response → deterministic replay + conservation checks.

### Scenario 7f: ReduceDanger Defensive Mitigation Under Active Threat
**File**: `golden_combat.rs` | **Test**: `golden_reduce_danger_defensive_mitigation`
**Systems exercised**: AI (danger pressure, candidate generation, planning), Combat (`attack`, `defend`), authoritative belief/runtime integration, Conservation
**Setup**: Two sated agents at Village Square. The attacker has a concrete hostility relation toward the defender and a stronger combat profile. The defender has no outgoing hostility relation and is therefore purely reactive.
**Emergent behavior proven**:
- The attacker opens combat through the normal `EngageHostile` path.
- The defender observes live attack pressure through the real runtime-aware belief view instead of relying on a test shim or queue injection.
- The defender does not alias incoming hostility into proactive `EngageHostile`; instead it enters a real `ReduceDanger` mitigation path and manifests concrete defensive behavior (`defend`/defending stance).
- Coin totals remain exactly conserved throughout the encounter.
**Cross-system chain**: Outgoing hostility → attack action start → current-attacker danger signal → `ReduceDanger` emission → defensive mitigation action.

### Scenario 7g: Wound Bleed, Clotting, and Natural Recovery
**File**: `golden_combat.rs` | **Tests**: `golden_wound_bleed_clotting_natural_recovery`, `golden_wound_bleed_clotting_natural_recovery_replays_deterministically`
**Systems exercised**: Combat (authoritative wound progression and pruning), Needs (physiology thresholds remaining below recovery gate), deterministic replay
**Setup**: Single sated agent at Village Square with one injected bleeding wound (`severity pm(50)`, `bleed_rate pm(100)`) and otherwise default golden-harness physiology/combat parameters.
**Emergent behavior proven**:
- The wound progresses through the authoritative combat tick instead of a unit-level helper or direct function call.
- Severity rises while the wound is bleeding, and bleed rate falls each tick under `natural_clot_resistance`.
- Recovery does not begin before bleed rate reaches zero.
- Once clotting completes and physiology stays below the hunger/thirst/fatigue `high()` thresholds, severity falls and the wound is eventually pruned from `WoundList`.
- Two runs with the same seed produce identical world and event-log hashes for the full wound-lifecycle scenario.
**Cross-system chain**: Authoritative wound state → combat-system bleed progression → natural clotting → physiology-gated recovery → wound pruning → deterministic replay.

### Scenario 8: Death Cascade and Opportunistic Loot
**File**: `golden_combat.rs` | **Test**: `golden_death_cascade_and_opportunistic_loot`
**Companion replay test**: `golden_death_cascade_and_opportunistic_loot_replays_deterministically`
**Systems exercised**: Needs (deprivation wounds), Combat (wound accumulation, death), Corpse actions (`loot`), Conservation, deterministic replay
**Setup**: Fragile victim (wound_capacity pm(200), existing pm(150) starvation wound, fast hunger metabolism, 2 hunger-critical-exposure ticks) with 5 coins. Second agent (Looter) at same location, healthy.
**Emergent behavior proven**:
- Victim dies from deprivation wounds exceeding wound_capacity.
- Looter opportunistically loots the corpse within the 100-tick observation window (hard assertion).
- Coin lot conservation holds every tick throughout the death + loot sequence.
- Two runs with the same seed produce identical world and event-log hashes for the death-and-loot scenario.
**Cross-system chain**: Metabolism → deprivation exposure → wound infliction → wound accumulation → death → corpse creation → loot goal generation → loot action.

### Scenario 8b: Corpse Burial
**File**: `golden_combat.rs` | **Test**: `golden_bury_corpse`
**Systems exercised**: AI (candidate generation, planning), Corpse actions (`bury`), containment/access rules, Conservation
**Setup**: A dead agent and a living burier are co-located at Village Square with a concrete `GravePlot` facility and no competing self-care pressure.
**Emergent behavior proven**:
- The local corpse + local grave plot emit the `BuryCorpse` path through the normal AI stack.
- The agent completes the real `bury` action rather than a scripted queue injection.
- Burial creates a concrete grave container and moves the corpse into containment.
- The corpse remains a persistent entity at the same place, but it is no longer directly targetable by the normal loot path.
**Cross-system chain**: Local corpse evidence + local grave-site evidence → `BuryCorpse` goal → planner leaf selection → bury action → concrete containment-based inaccessibility.

### Scenario 8c: Loot Suppression Under Self-Care Pressure
**File**: `golden_combat.rs` | **Tests**: `golden_loot_suppressed_under_self_care_pressure`, `golden_loot_suppressed_under_self_care_pressure_replays_deterministically`
**Systems exercised**: Needs, AI (candidate generation, ranking suppression, planning), Corpse actions (`loot`), Conservation, deterministic replay
**Setup**: A hungry scavenger (`hunger pm(800)`) stands at Village Square with one carried bread. A pre-seeded corpse with direct coin possession is co-located. No other self-care or danger pressures compete.
**Emergent behavior proven**:
- While hunger remains at the default `high()` threshold or above, the scavenger never gains corpse coins.
- The scavenger begins the real `eat` action before any corpse loot resolves.
- One bread relieves hunger below the `high()` threshold, after which the scavenger proceeds to loot the corpse.
- Coin lot conservation holds throughout the suppression-then-lift sequence.
- Two runs with the same seed produce identical world and event-log hashes for the scenario.
**Cross-system chain**: High hunger pressure + local corpse evidence → corpse-loot suppression in ranking → local self-care action (`eat`) → hunger relief below `high()` → loot candidate becomes behaviorally available → deterministic replay.

### Scenario 8d: Death While Traveling
**File**: `golden_combat.rs` | **Test**: `golden_death_while_traveling`
**Companion replay test**: `golden_death_while_traveling_replays_deterministically`
**Systems exercised**: Needs (metabolism, deprivation exposure), AI (distant acquire planning), Travel, Combat (fatal wound resolution), Conservation, deterministic replay
**Setup**: Fragile traveler starts at Bandit Camp with critical hunger and 5 coins. Orchard Farm is the only food source. The route requires real multi-hop travel through Forest Path and beyond.
**Emergent behavior proven**:
- Traveler leaves Bandit Camp and enters real in-transit travel on the hunger-driven route.
- The traveler dies from deprivation before reaching Orchard Farm.
- In the deterministic scenario, death resolves at the first intermediate route place (`ForestPath`), proving the body remains at a concrete grounded location instead of vanishing or reaching the destination.
- After death resolves, the agent has no active action and no lingering in-transit state.
- Coin lot conservation holds every tick throughout the journey and death sequence.
- Two runs with the same seed produce identical world and event-log hashes for the death-while-traveling scenario.
**Cross-system chain**: Hunger pressure → distant acquire-goal emission → travel departure → continued metabolism on route → deprivation wound infliction → death before destination → concrete body placement on route.

### Scenario 9: Exclusive Facility Queue Contention
**File**: `golden_production.rs` | **Tests**: `golden_exclusive_queue_contention_uses_queue_grants_and_rotates_first_turns`, `golden_exclusive_queue_contention_replays_deterministically`
**Systems exercised**: Production (exclusive facility policy, resource source), AI (candidate generation, planning with queue barriers), FacilityQueue (queue_for_facility_use action, facility_queue_system tick), Conservation, deterministic replay
**Setup**: Four critically hungry agents (Aster, Bram, Cara, Dara) co-located at Orchard Farm. A single OrchardRow workstation with `ExclusiveFacilityPolicy` and `FacilityUseQueue`, containing `Quantity(4)` apples and no regeneration. Grant expiry window is 3 ticks.
**Emergent behavior proven**:
- Multiple agents generate harvest goals simultaneously, but the exclusive facility policy forces them through `queue_for_facility_use` before harvesting.
- A real waiting line materializes on the workstation (`max_waiting_len >= 2`), proving queue contention occurs under the AI loop.
- The `facility_queue_system()` tick system (prune → expire → promote) grants access to one agent at a time. At least two distinct agents receive grants and complete harvest turns.
- The first two promoted actors are different agents, proving fair queue rotation rather than one agent monopolizing the facility.
- The exclusive orchard source is fully exhausted (`Quantity(0)`) after the two granted harvest turns.
- Authoritative apple conservation holds every tick throughout the contention sequence.
- Two runs with the same seed produce identical world and event-log hashes.
**Cross-system chain**: Multi-agent hunger pressure at exclusive facility → queue_for_facility_use action → facility_queue_system promotion → granted harvest → resource depletion → queue rotation to next agent → deterministic replay + conservation.

### Scenario 9b: Facility Queue Patience Timeout
**File**: `golden_production.rs` | **Tests**: `golden_facility_queue_patience_timeout`, `golden_facility_queue_patience_timeout_replays_deterministically`
**Systems exercised**: Production (exclusive facility policy, resource source), AI runtime (patience expiry, blocked-facility memory, replanning), FacilityQueue, Travel, Conservation
**Setup**: Hungry agent starts at Orchard Farm with a per-agent `FacilityQueueDispositionProfile`. Facility A is a local exclusive orchard already monopolized by another actor's long-lived grant. Facility B is a reachable alternative exclusive orchard at Village Square.
**Emergent behavior proven**:
- The hungry agent initially queues at the local facility A through the real `queue_for_facility_use` path.
- When the queue does not progress within the configured patience window, the runtime removes the agent from facility A's authoritative queue instead of merely marking a dirty replan hint.
- The queue disappearance flows through the existing blocked-facility pipeline, recording `ExclusiveFacilityUnavailable` for facility A rather than introducing a special-case alias path.
- The agent then replans to the alternative facility B, uses it, and reduces hunger there.
- The monopolized facility A remains unused by the waiting agent; the alternative facility B is the one whose stock decreases.
- Two runs with the same seed produce identical world and event-log hashes for this timing-sensitive queue abandonment path.
**Cross-system chain**: Local hunger pressure → queue_for_facility_use at local exclusive facility → patience-based authoritative dequeue → blocked-facility planning memory → alternative-facility travel/queue/use → harvest/materialization → hunger relief.

### Scenario 9c: Grant Expiry Before Intended Action
**File**: `golden_production.rs` | **Test**: `golden_grant_expiry_before_intended_action`
**Systems exercised**: Production (exclusive facility policy, resource source), Needs (metabolism + local water relief), AI runtime (goal switching, queue/grant transition handling, replanning), FacilityQueue, Conservation
**Setup**: Hungry agent starts at Orchard Farm with 1 carried water beside a single exclusive OrchardRow workstation. The workstation uses `grant_hold_ticks = 1`, and the agent's thirst metabolism spikes high enough that thirst becomes the higher-priority local goal immediately after the first queue promotion.
**Emergent behavior proven**:
- The agent enters the real exclusive-facility queue path and receives a real grant.
- Before any harvest starts, the agent takes a same-place water-consumption detour because thirst legitimately becomes the higher-priority need.
- The original grant expires unused and emits `QueueGrantExpired` while the orchard stock remains untouched, proving grant expiry is authoritative rather than inferred from resource depletion.
- After the detour resolves, the agent re-enters the normal queue path, receives a second real promotion, and eventually harvests/eats to reduce hunger.
- No engine-specific grant recovery shim was required; the existing queue/grant transition runtime already handles the dirty/replan path cleanly.
**Cross-system chain**: Hunger pressure → queue_for_facility_use → queue promotion → metabolism-driven thirst spike → local water detour → authoritative grant expiry → normal queue recovery/promotion → harvest/materialization → hunger relief.

### Scenario 9d: Dead Agent Pruned from Facility Queue
**File**: `golden_production.rs` | **Tests**: `golden_dead_agent_pruned_from_facility_queue`, `golden_dead_agent_pruned_from_facility_queue_replays_deterministically`
**Systems exercised**: Needs (deprivation exposure, metabolism), Combat (fatal deprivation wound resolution), Production (exclusive facility policy, resource source), FacilityQueue (authoritative prune + later promotion), deterministic replay
**Setup**: Exclusive OrchardRow workstation at Orchard Farm starts with an existing grant holder blocking access. A fragile hungry waiter queues first with pre-existing starvation damage, and a second healthy hungry waiter queues behind them for the same orchard.
**Emergent behavior proven**:
- Both hungry waiters enter the real exclusive-facility queue while the initial grant blocks access.
- The fragile waiter dies from deprivation while still queued and never receives a grant.
- `prune_invalid_waiters()` removes the dead waiter from the authoritative queue without any AI-side death/queue shim.
- The next living waiter becomes queue head and later receives a real `QueueGrantPromoted` event.
- Two runs with the same seed produce identical world and event-log hashes for the death-in-queue scenario.
**Cross-system chain**: Hunger pressure under blocked exclusive access → queue_for_facility_use → deprivation death while queued → authoritative queue prune → next-waiter promotion → deterministic replay.

### Scenario S02: World Runs Without Observers (Principle 6)
**File**: `golden_determinism.rs` | **Tests**: `golden_world_runs_without_observers`, `golden_world_runs_without_observers_replays_deterministically`
**Systems exercised**: Needs (metabolism, consumption), Production (harvest, resource regeneration), Travel, Enterprise (restock signal), Trade (merchant setup), AI (candidate generation, planning, multi-agent coordination), Conservation, deterministic replay
**Setup**: Four agents across three places running for 200 ticks under the full AI loop with no human intervention:
- **Farmer** at Orchard Farm: hungry (pm(800)), OrchardRow workstation with regenerating apple source (qty=20, regen=1/5 ticks), PerceptionProfile, knows harvest recipe.
- **Merchant** at General Store: enterprise-focused (enterprise_weight=pm(800)), coins(10), MerchandiseProfile(Apple), enterprise TradeDispositionProfile, DemandMemory with apple demand, beliefs about orchard workstation, PerceptionProfile.
- **Villager** at Village Square: hungry (pm(700)), thirsty (pm(500)), bread(1), water(2), coins(5).
- **Wanderer** at Village Square: thirsty (pm(800)), fatigued (pm(600)), water(1), thirst_weight=pm(800), fatigue_weight=pm(600).
**Emergent behavior proven**:
- The world hash differs from the initial state after 200 ticks (non-trivial simulation).
- The event log grows by at least 20 events.
- No agent dies (the world is provisioned and sustainable over 200 ticks).
- Per-tick authoritative conservation holds for Bread, Water, and Coin (Apple is regenerating so excluded from strict per-tick conservation).
- At least one agent enters transit (travel system engaged).
- At least one consumable commodity total decreases (needs system engaged via consumption).
- Two runs with the same seed produce identical world and event-log hashes, proving 200-tick multi-agent determinism far beyond the existing 50-tick 2-agent test.
**Cross-system chain**: Needs→AI→action, Production→regeneration→harvest, Travel→location change, Enterprise→restock gap, Conservation across 200 ticks of multi-system interaction. Proves Principle 6 (world runs without observers) comprehensively.

### Scenario S02b: Utility Weight Diversity in Need Selection (Principle 20)
**File**: `golden_ai_decisions.rs` | **Test**: `golden_utility_weight_diversity_in_need_selection`
**Systems exercised**: Needs (hunger), Enterprise (restock signal), AI (candidate generation, ranking, goal selection divergence), Travel, Production
**Setup**: Two agents at Village Square with identical initial conditions but divergent UtilityProfile weights:
- **HungerDriven**: critically hungry (pm(900)), hunger_weight=pm(800), enterprise_weight=pm(100), has bread(2) locally.
- **EnterpriseDriven**: no hunger (pm(0)), hunger_weight=pm(100), enterprise_weight=pm(900), MerchandiseProfile(Apple), enterprise TradeDispositionProfile, DemandMemory with apple demand, beliefs about orchard workstation, PerceptionProfile. Orchard Farm has apple source.
**Emergent behavior proven**:
- HungerDriven eats bread locally under hunger pressure (hunger decreases).
- EnterpriseDriven leaves Village Square to pursue the enterprise restock goal despite having no survival pressure.
- The two agents make observably different first choices — one stays to eat, one travels to restock — proving that UtilityProfile weight divergence produces distinct goal selection.
**Cross-system chain**: UtilityProfile weights → divergent candidate ranking → HungerDriven selects ConsumeOwnedCommodity path while EnterpriseDriven selects RestockCommodity path → different first actions under different weight profiles. Proves Principle 20 (agent diversity through concrete variation) in the survival-vs-enterprise domain.

### Scenario S03a: Multi-Corpse Loot Binding (S03 — matches_binding)
**File**: `golden_combat.rs` | **Tests**: `golden_multi_corpse_loot_binding`, `golden_multi_corpse_loot_binding_replays_deterministically`
**Systems exercised**: AI (candidate generation for LootCorpse, ranking, plan search with matches_binding, execution), Corpse (loot action), Conservation, deterministic replay
**Setup**: Two dead agents (CorpseA with Coin(5), CorpseB with Bread(3)) and a sated Looter at Village Square. Looter has local beliefs seeded at Tick(0).
**Emergent behavior proven**:
- Candidate generation produces LootCorpse goals for both corpses.
- Ranking picks one deterministically. Plan search uses `matches_binding()` — only affordances targeting the selected corpse pass.
- Agent loots one corpse first. While the first corpse's items are being transferred, the other corpse's inventory remains untouched (sequential binding).
- Agent then loots the second corpse, gaining both Coin and Bread.
- Coin and Bread conservation holds every tick.
- Two runs with the same seed produce identical world and event-log hashes.
**Cross-system chain**: LootCorpse candidate generation → ranking selects one target → matches_binding filters to correct corpse → sequential loot execution → second corpse looted after first completes → conservation throughout.

### Scenario S03b: Bury Suppressed Under Stress (S02 — evaluate_suppression for BuryCorpse)
**File**: `golden_combat.rs` | **Tests**: `golden_bury_suppressed_under_stress`, `golden_bury_suppressed_under_stress_replays_deterministically`
**Systems exercised**: Needs (hunger metabolism), AI (candidate generation, suppression evaluation, goal switching), Corpse (bury action), Production (GravePlot workstation), deterministic replay
**Setup**: Dead agent (no loot) at Village Square with GravePlot workstation. Burier at Village Square with hunger=pm(800) (above High threshold of pm(750)) and Bread(1). Local beliefs seeded at Tick(0).
**Emergent behavior proven**:
- Hunger(800) >= High(750) → `evaluate_suppression()` suppresses BuryCorpse.
- ConsumeOwnedCommodity(Bread) goal fires — agent eats bread.
- Hunger drops below 750 → suppression lifts.
- Agent plans and executes BuryCorpse → corpse gets placed into a grave container at Village Square.
- While hunger remains at or above the high threshold, the corpse has no container (burial actively suppressed).
- Agent eats before burying, and hunger relief precedes burial completion.
- Two runs with the same seed produce identical world and event-log hashes.
**Cross-system chain**: Hunger pressure above High → BuryCorpse suppression → ConsumeOwnedCommodity goal → eat bread → hunger relief → suppression lift → BuryCorpse execution → corpse in grave container.

### Scenario S03c: Suppression Then Binding Combined (S02 + S03 interaction)
**File**: `golden_combat.rs` | **Tests**: `golden_suppression_then_binding_combined`, `golden_suppression_then_binding_combined_replays_deterministically`
**Systems exercised**: Needs (hunger metabolism), AI (candidate generation, suppression evaluation, ranking, plan search with matches_binding, goal switching), Corpse (loot action), Conservation, deterministic replay
**Setup**: Two dead agents (CorpseA with Coin(5), CorpseB with Coin(3)) and a Scavenger with hunger=pm(800) (above High threshold) and Bread(1) at Village Square. Local beliefs seeded at Tick(0).
**Emergent behavior proven**:
- Hunger High → both LootCorpse goals suppressed (scavenger gains no coins while hunger >= high).
- Agent eats bread → hunger drops → suppression lifts.
- Ranking picks one loot goal → `matches_binding()` ensures correct target selection.
- Agent loots the first corpse. While the first corpse still has remaining coins, the other corpse's coins remain intact (sequential binding correctness).
- Agent then loots the second corpse, gaining all 8 coins total.
- Coin conservation holds every tick.
- Two runs with the same seed produce identical world and event-log hashes.
**Cross-system chain**: Hunger pressure above High → LootCorpse suppression on both targets → ConsumeOwnedCommodity goal → eat bread → hunger relief → suppression lift → matches_binding selects correct corpse → sequential loot → both corpses looted → conservation throughout.

---

### Scenario S07a: Wound vs Hunger Priority Resolution (Pain First)
**File**: `golden_emergent.rs` | **Test**: `golden_wound_vs_hunger_pain_first`
**Systems exercised**: Needs (hunger metabolism), Care (self-treatment), AI (candidate generation, ranking with UtilityProfile weights)
**Setup**: Single agent at Village Square, wounded (severity pm(400), clotted, zero natural recovery), hungry (pm(700)). Has both Apple(2) and Medicine(1). pain_weight=pm(800), hunger_weight=pm(400). DirectObservation beliefs seeded.
**Emergent behavior proven**:
- Agent heals before eating (first state-delta is wound_load decrease, not hunger decrease).
- Both needs eventually addressed (wound_load decreases AND hunger decreases).
- Conservation: apple + medicine lots non-increasing.
**Foundation alignment**: Principle 3 (concrete utility weights drive priority, not abstract tiers), Principle 20 (agent diversity via profile weights).
**Cross-system chain**: Pain pressure (weighted 800) outranks hunger pressure (weighted 400) → TreatWounds{self} selected → medicine consumed → wound reduced → then eat.

### Scenario S07b: Wound vs Hunger Priority Resolution (Hunger First)
**File**: `golden_emergent.rs` | **Test**: `golden_wound_vs_hunger_hunger_first`
**Systems exercised**: Needs (hunger metabolism), Care (self-treatment), AI (candidate generation, ranking with UtilityProfile weights)
**Setup**: Identical to S07a but pain_weight=pm(300), hunger_weight=pm(800).
**Emergent behavior proven**:
- Agent eats before healing (first state-delta is hunger decrease, not wound_load decrease).
- Both needs eventually addressed.
- Conservation: apple + medicine lots non-increasing.
**Foundation alignment**: Principle 3, Principle 20. Same concrete-weight mechanism as S07a but reversed outcome proves weights, not hardcoded tiers, determine priority.
**Cross-system chain**: Hunger pressure (weighted 800) outranks pain pressure (weighted 300) → ConsumeOwnedCommodity selected → eat → hunger reduced → then self-heal.

### Scenario S07c: Care Weight Divergence Under Observation
**File**: `golden_emergent.rs` | **Tests**: `golden_care_weight_divergence_under_observation`, `golden_care_weight_divergence_replays_deterministically`
**Systems exercised**: Needs (hunger metabolism), Care (third-party healing), AI (candidate generation, ranking with care_weight), Perception (DirectObservation belief seeding)
**Setup**: Patient at Village Square wounded (severity pm(500), zero natural recovery). Altruist (care_weight=pm(800), low hunger, Medicine(1)). Selfish agent (care_weight=pm(100), hunger_weight=pm(800), hunger=pm(500), Medicine(1), Apple(2)). Both agents have DirectObservation beliefs about the patient.
**Emergent behavior proven**:
- Altruist's first action is healing the patient (medicine consumed).
- Selfish agent's first action is eating (hunger decreases before medicine is consumed).
- Patient wound_load decreases (healed by altruist).
- Same stimulus (observing wounded patient), different response, driven by concrete profile weights.
- Two runs with the same seed produce identical hashes.
**Foundation alignment**: Principle 20 (agent diversity), Principle 3 (concrete weights, not abstract altruism tiers), Principle 7 (both agents observe via DirectObservation).
**Cross-system chain**: DirectObservation of wounded patient + divergent care_weight → Altruist: TreatWounds{patient} → heal; Selfish: ConsumeOwnedCommodity → eat → (may heal later).

### Scenario S07d: Care Travel to Remote Patient
**File**: `golden_emergent.rs` | **Tests**: `golden_care_travel_to_remote_patient`, `golden_care_travel_to_remote_patient_replays_deterministically`
**Systems exercised**: Care (third-party healing), Travel, AI (GOAP plan decomposition: Travel + Heal), Perception (remote belief seeding)
**Setup**: Patient at Orchard Farm wounded (severity pm(500), zero natural recovery). Healer at Village Square with Medicine(1), care_weight=pm(800), PerceptionProfile. Healer has DirectObservation belief about patient (artificially seeded for remote patient).
**Emergent behavior proven**:
- Healer travels from Village Square (effective_place changes).
- Healer consumes medicine to heal the patient.
- Patient wound_load decreases after healer arrives.
- Healing takes >3 ticks (travel time as natural dampener — Principle 10).
- Two runs with the same seed produce identical hashes.
**Foundation alignment**: Principle 1 (causal chain: belief → travel → heal), Principle 7 (belief-seeded, not omniscient), Principle 10 (travel time naturally dampens healing throughput).
**Cross-system chain**: DirectObservation of remote wounded patient → TreatWounds{patient} goal → GOAP decomposes to Travel(Orchard Farm) + Heal → travel time elapses → healer arrives → medicine consumed → wound reduced.

### Scenario S07e: Loot Corpse Self-Care Chain
**File**: `golden_emergent.rs` | **Tests**: `golden_loot_corpse_self_care_chain`, `golden_loot_corpse_self_care_chain_replays_deterministically`
**Systems exercised**: Corpse (loot action), Transport (item transfer), Care (self-treatment), AI (goal sequencing: LootCorpse → TreatWounds{self})
**Setup**: Wounded scavenger at Village Square (severity pm(400), zero natural recovery, no medicine). Pre-killed corpse at Village Square carrying Medicine(2). Scavenger has pain_weight=pm(700), care_weight=pm(600), PerceptionProfile. DirectObservation beliefs seeded.
**Emergent behavior proven**:
- Scavenger acquires medicine from looting the corpse (commodity qty > 0).
- Scavenger wound_load decreases after acquiring medicine (self-care).
- Medicine conservation holds every tick.
- Two runs with the same seed produce identical hashes.
**Foundation alignment**: Principle 1 (maximal emergence — no quest logic drives the loot→heal chain), Principle 3 (concrete wounds, concrete medicine), Principle 24 (systems interact only through state mutation).
**Cross-system chain**: Observe corpse → LootCorpse goal → loot action transfers medicine → TreatWounds{self} goal emerges → medicine consumed → wound reduced.

### Scenario 11: Simple Office Claim via DeclareSupport
**File**: `golden_offices.rs` | **Test**: `golden_simple_office_claim_via_declare_support`
**Systems exercised**: Succession (office resolution), AI (candidate generation for ClaimOffice, GOAP planning for DeclareSupport), Political actions (declare_support)
**Setup**: Single sated agent ("Claimant") at Village Square with enterprise_weight=pm(800) and PerceptionProfile. Vacant office ("Village Elder") with SuccessionLaw::Support, succession_period_ticks=5, no eligibility rules. Agent has DirectObservation beliefs about the office.
**Emergent behavior proven**:
- Agent autonomously generates ClaimOffice goal from enterprise_weight and believed vacant office.
- GOAP planner finds DeclareSupport(self) plan.
- Agent executes DeclareSupport action (1-tick, non-interruptible).
- After succession period elapses, succession_system installs agent as office holder.
- Event log contains Political events from declaration and installation.
**Foundation alignment**: Principle 10 (agent plans from beliefs about the office, not world state), Principle 20 (enterprise_weight drives political ambition).
**Cross-system chain**: Enterprise weight → ClaimOffice candidate → DeclareSupport plan → action execution → succession_system resolution → office installation.

### Scenario 11b: Simple Office Claim Deterministic Replay
**File**: `golden_offices.rs` | **Test**: `golden_simple_office_claim_deterministic_replay`
**Systems exercised**: Same as Scenario 11 + determinism verification.
**Setup**: Same as Scenario 11, run twice with identical seed.
**Emergent behavior proven**:
- Two runs with the same seed produce identical world and event log hashes.
- World state differs from initial (non-trivial simulation occurred).

### Scenario 12: Competing Claims with Loyal Supporter
**File**: `golden_offices.rs` | **Test**: `golden_competing_claims_with_loyal_supporter`
**Systems exercised**: Succession (support counting, tie rejection, installation), AI (candidate generation for ClaimOffice and SupportCandidateForOffice, GOAP planning, zero-motive filtering), Political actions (declare_support)
**Setup**: Three agents at Village Square. Agent A ("Claimant Alpha") and Agent B ("Claimant Beta") both with enterprise_weight=pm(800). Agent C ("Loyal Supporter") with enterprise_weight=0, social_weight=pm(600), loyalty to A at pm(650). Vacant office ("Village Elder") with SuccessionLaw::Support, succession_period_ticks=5, no eligibility rules. All agents have PerceptionProfile and beliefs about the office. C also has beliefs about A.
**Emergent behavior proven**:
- A and B autonomously generate ClaimOffice goals (enterprise_weight drives motive).
- C generates both ClaimOffice (zero-motive filtered due to enterprise_weight=0) and SupportCandidateForOffice(A) (driven by loyalty + social_weight).
- C's SupportCandidateForOffice(A) wins ranking since ClaimOffice is filtered.
- All three agents independently execute DeclareSupport actions.
- A gets 2 declarations (self + C), B gets 1 (self). Succession system rejects ties, so without C's support the office would remain vacant.
- succession_system installs A as unique winner, then clears all declarations.
**Foundation alignment**: Principle 10 (agents plan from beliefs), Principle 20 (enterprise_weight vs social_weight drives divergent political behavior — claimants vs supporters).
**Cross-system chain**: Loyalty → SupportCandidateForOffice candidate → zero-motive ClaimOffice filtering → DeclareSupport plan → multi-agent concurrent declarations → support counting → decisive installation.

### Scenario 13: Bribe -> Support Coalition (Full-Quantity Transfer)
**File**: `golden_offices.rs` | **Test**: `golden_bribe_support_coalition`
**Systems exercised**: Bribe (commodity transfer + loyalty increase), Succession (support counting, coalition majority, installation), AI (candidate generation for ClaimOffice, coalition-aware GOAP planning with Bribe + DeclareSupport multi-step plan, SupportCandidateForOffice from bribe-induced loyalty), Conservation
**Setup**: Agent A ("Briber Alpha") at Village Square with enterprise_weight=pm(900), holds 5 bread. Agent B ("Bribe Target") at Village Square with social_weight=pm(600), enterprise_weight=0 (won't claim office). Agent C ("Competitor") at Orchard Farm (different place, not bribable) with enterprise_weight=pm(800), has pre-declared self-support. Vacant office ("Village Elder") with SuccessionLaw::Support, succession_period_ticks=5. Wider beam_width=16 on the planning budget since the prototype world's adjacency graph creates many equal-cost travel candidates that can push Bribe nodes past the default beam cutoff.
**Emergent behavior proven**:
- A generates ClaimOffice goal. DeclareSupport alone would tie with C (ProgressBarrier). Coalition-aware planner finds Bribe(B, bread) + DeclareSupport(self) as GoalSatisfied.
- A bribes B — all 5 bread transferred (full-stock). B's loyalty to A increases.
- B autonomously generates SupportCandidateForOffice(A) from bribe-induced loyalty.
- A's coalition (self + B = 2) exceeds C's (self = 1). Succession system installs A.
- Commodity conservation holds: A's bread drops 5→0, B's bread rises 0→5, total unchanged.
**Foundation alignment**: Principle 10 (belief-only planning), Principle 1 (maximal emergence — bribe → loyalty → support is emergent, not scripted), conservation invariant.
**Cross-system chain**: AI goal → coalition-aware planner Bribe op → commodity transfer → conservation → loyalty increase → target AI SupportCandidateForOffice → DeclareSupport → support counting → decisive installation.
