# Golden E2E Suite: Scenario Catalog

**Date**: 2026-03-12 (updated 2026-03-18, locality offices added 2026-03-18, S13-002 social-political emergence added 2026-03-18, S13-003 wounded-politician ordering added 2026-03-19, E15c social coverage aligned 2026-03-19, S14 conversation-memory emergence added 2026-03-19, S08 care start-abort regression added 2026-03-19, S16 spatial multi-hop coverage added 2026-03-21, inventory generation added 2026-03-21, S17 wound-lifecycle scenarios aligned 2026-03-21, S18 craft-restock supply-chain scenario added 2026-03-21)
**Scope**: `crates/worldwake-ai/tests/golden_*.rs`
**Purpose**: Detailed reference for what each golden test proves. Consult when you need to understand a specific scenario or verify whether a behavior is already tested. For coverage gaps and matrices, see [golden-e2e-coverage.md](golden-e2e-coverage.md).
**Conventions**: For assertion patterns and trace usage, see [golden-e2e-testing.md](golden-e2e-testing.md).
**Inventory source**: The live per-file counts and current `golden_*` names live in [generated/golden-e2e-inventory.md](generated/golden-e2e-inventory.md). Validate this catalog with `python3 scripts/golden_inventory.py --write --check-docs`.

---

Every active golden test uses the real AI loop (`AgentTickDriver` + `AutonomousControllerRuntime`) and real system dispatch. The canonical counts and per-file test inventory are generated, not maintained in this document by hand; see [generated/golden-e2e-inventory.md](generated/golden-e2e-inventory.md). The social slice locks down autonomous Tell, suppression under survival pressure, bystander locality, entity-missing discovery, stale-belief correction, unchanged-repeat suppression through explicit told-memory, lawful re-tell after subject-belief change, lawful re-tell after conversation-memory expiry, trace-visible social re-enablement, chain-length gossip cutoff, agent diversity via social_weight, and the full rumor→wasted-trip→discovery lifecycle. The determinism slice now includes a 200-tick 4-agent world-runs-without-observers proof. The AI decisions slice now includes utility-weight-driven goal divergence (Principle 20, survival vs enterprise), the BanditCamp multi-hop route, and the default-budget VillageSquare branchy-hub spatial route plus trace-enabled smoke coverage. The care slice now also proves the recoverable pre-start wound-disappearance race: a lawful `TreatWounds` plan can be selected, the patient's wounds can disappear before authoritative input drain, and the result is `StartFailed` plus blocked-intent persistence instead of a crash. The production and trade slices now also prove the same S08/S15 start-failure contract in ordinary contested-harvest and stale-local-trade scenarios, where the losing actor records a lawful `StartFailed`, the next AI tick clears the stale branch, and recovery continues through a remote food path. The emergent slice (golden_emergent.rs) now proves both care-centered and political cross-system emergence: wound-vs-hunger priority resolution via concrete utility weights, repeated deprivation-firing consolidation into one persistent starvation wound, wounded-politician mixed-layer ordering between `heal` and `declare_support`, care_weight diversity producing divergent behavior, care+travel to remote patients, loot→self-care chains, combat death cascading into authoritative office vacancy and later force-law succession, autonomous Tell propagating a remote office fact into the ordinary `ClaimOffice` travel-and-succession path, same-place office facts still requiring Tell before political planning appears, listener-aware resend suppression happening before tell-candidate truncation so an untold office fact can still unlock downstream politics, and a remote office-claim race where the delayed claimant loses gracefully through the shared start-failure path instead of looping forever. The offices slice now also proves both halves of political locality: remote office claims stay inert without office knowledge, and shared self-care suppression can defer political ambition without any office-specific suppression code. The combat slice also includes action-trace integration coverage for the loot lifecycle, wound bleed/clot/recovery, and the recovery-aware "eat before wash" ranking/recovery chain. All behavior is emergent.

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
**File**: `golden_care.rs` | **Tests**: `golden_healing_wounded_agent`, `golden_healing_wounded_agent_replays_deterministically`, `golden_healer_acquires_ground_medicine_for_patient`, `golden_healer_acquires_ground_medicine_for_patient_replays_deterministically`, `golden_healer_acquires_remote_ground_medicine_for_patient`, `golden_healer_acquires_remote_ground_medicine_for_patient_replays_deterministically`, `golden_self_care_with_medicine`, `golden_self_care_with_medicine_replays_deterministically`, `golden_self_care_acquires_ground_medicine`, `golden_self_care_acquires_ground_medicine_replays_deterministically`, `golden_indirect_report_does_not_trigger_care`, `golden_indirect_report_does_not_trigger_care_replays_deterministically`, `golden_care_goal_invalidation_when_patient_heals`, `golden_care_goal_invalidation_when_patient_heals_replays_deterministically`, `golden_care_pre_start_wound_disappearance_records_blocker`, `golden_care_pre_start_wound_disappearance_records_blocker_replays_deterministically`
**Systems exercised**: AI (candidate generation, planning, `TreatWounds` goal family), Care action domain, Combat/wound treatment, Perception (direct-observation gate), Conservation, deterministic replay
**Setup**: Eight care sub-scenarios are covered:
1. **Third-party care with medicine**: Healthy healer with medicine, co-located wounded patient. Healer directly observes wounds via passive perception and treats.
2. **Third-party care with ground medicine**: Healer without medicine, ground medicine available beside wounded patient. Healer picks up medicine then treats.
3. **Third-party care with remote ground medicine**: Healer and wounded patient start at Village Square, but the only known medicine lot is on the ground at Orchard Farm. The healer must travel out, pick up medicine, return, and heal.
4. **Self-care with medicine**: Single wounded agent with own medicine. Agent emits `TreatWounds { patient: self }` and self-treats.
5. **Self-care with ground medicine acquisition**: Single wounded agent, no medicine in inventory, ground medicine at same place. Agent picks up medicine and self-treats.
6. **Indirect report does NOT trigger care**: Observer at Village Square with medicine. Wounded patient at Orchard Farm. Observer has Report-sourced belief about patient's wounds. Observer does NOT consume medicine and does NOT travel to patient — only `DirectObservation` triggers care.
7. **Care goal invalidation**: Patient with medicine and healer without medicine, co-located. Patient self-treats. Healer's `TreatWounds { patient }` goal is satisfied by patient's self-healing (healer never acquires medicine).
8. **Care pre-start wound disappearance recovery**: Healer lawfully selects `TreatWounds { patient }`, then the patient's wounds are cleared after input production but before authoritative input drain. The queued `heal` request records `StartFailed`, and the next AI tick persists blocked intent memory instead of crashing or leaving a stale in-flight step.
**Emergent behavior proven**:
- Healer generates `TreatWounds { patient }` from directly-observed wounded target via passive perception.
- When the healer lacks medicine but a wounded local target exists, candidate generation emits the care goal and the planner resolves acquisition through `pick_up` before healing.
- When the only known medicine is remote, the same care goal survives multi-leg travel and procurement before the healer returns to treat the patient.
- Self-care is lawful: wounded agents emit `TreatWounds { patient: self }` and consume own medicine.
- Self-care acquisition works: wounded agents pick up ground medicine and self-treat.
- Report-sourced wound beliefs do NOT trigger `TreatWounds` — only `PerceptionSource::DirectObservation` does (Principle 7 locality).
- When a patient self-heals, the healer's `TreatWounds` goal is satisfied (patient pain reaches zero) and drops cleanly.
- A lawful pre-start wound disappearance becomes recoverable start failure plus blocked-intent persistence, proving the real care path reaches the structured S08 failure handoff.
- Planner selects the care-domain heal action through the real action registry.
- Heal executes through the normal lifecycle: medicine is consumed and the patient's wound load decreases.
- Two runs with the same seed produce identical world and event-log hashes for all care scenarios.
**Cross-system chain**: Wound state → passive perception seeds `DirectObservation` belief → `TreatWounds` goal emission (self or other) → planner resolves supply (local pickup, remote pickup, or held medicine) → heal action → medicine consumption → wound severity/bleed reduction. Report-sourced beliefs are filtered by the direct-observation gate. Goal invalidation propagates when patient pain reaches zero. If wounds disappear after planning but before authoritative start, the action records `StartFailed` and the next AI tick turns that structured failure into blocked-intent memory rather than a crash.

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

### Scenario 2d-craft: Merchant Restock via Prerequisite-Aware Craft
**File**: `golden_supply_chain.rs` | **Tests**: `golden_merchant_restocks_via_prerequisite_aware_craft`, `golden_merchant_restocks_via_prerequisite_aware_craft_replays_deterministically`
**Systems exercised**: Enterprise AI signals, prerequisite-aware planning, Travel, Production (harvest + craft), action traces, decision traces, deterministic replay
**Setup**: Merchant starts at General Store with `MerchandiseProfile` advertising bread, zero bread stock, and remembered unmet bread demand at the home market. The only firewood source is remote at Orchard Farm, and the only mill is local to the home market. The scenario uses a minimal test-local recipe registry that exposes `Harvest Firewood` and `Bake Bread` so the golden exercises a real remote `ResourceSource` path under the live action registry.
**Emergent behavior proven**:
- Merchant generates `RestockCommodity { Bread }` from concrete remembered demand instead of from an abstract threshold.
- Tick-0 decision traces show the planner using prerequisite-aware spatial guidance toward Orchard Farm rather than only local search.
- Merchant completes the full lawful chain: travel to Orchard Farm, harvest remote firewood, pick it up, return to the home market, and commit `craft:Bake Bread`.
- The durable contract is home-market bread stock, not forced carried inventory. The golden proves that restocked bread exists at the destination market after the remote prerequisite chain completes.
- Two runs with the same seed produce identical world and event-log hashes for the craft-restock scenario.
**Cross-system chain**: Demand memory at home market → enterprise restock signal → prerequisite-aware route selection toward remote recipe input → remote harvest/pickup → return travel → local craft → bread stock appears at home market.

### Scenario 2e: Social Belief Sharing, Conversation Memory, Locality, and Discovery
**File**: `golden_social.rs` | **Tests**: `golden_agent_autonomously_tells_colocated_peer`, `golden_survival_needs_suppress_social_goals`, `golden_rumor_chain_degrades_through_three_agents`, `golden_stale_belief_travel_reobserve_replan`, `golden_skeptical_listener_rejects_told_belief`, `golden_bystander_sees_telling_but_gets_no_belief`, `golden_entity_missing_discovery_does_not_teleport_belief`, `golden_agent_does_not_repeat_same_unchanged_tell_to_same_listener`, `golden_agent_retells_after_subject_belief_changes`, `golden_agent_retells_after_conversation_memory_expiry`, `golden_decision_trace_explains_social_candidate_reenabled_after_belief_change_or_expiry`, `golden_chain_length_filtering_stops_gossip`, `golden_agent_diversity_in_social_behavior`, `golden_rumor_leads_to_wasted_trip_then_discovery`
**Systems exercised**: Perception/beliefs, conversation memory, Tell actions, AI social candidate generation and ranking, self-care suppression, planner payload wiring, decision tracing, travel, zero-motive ranking filter, deterministic replay
**Setup**: Fourteen focused social scenarios cover colocated speaker/listener tell behavior, survival-need suppression of gossip, three-agent relay chains, stale harvest beliefs contradicted by local re-observation, hard listener rejection via `acceptance_fidelity: Permille(0)`, bystander witnessing without belief transfer, entity-missing discovery from a violated place expectation, unchanged-repeat suppression via explicit told-memory, lawful re-tell after subject-belief change, lawful re-tell after conversation-memory expiry, decision-trace-visible social re-enablement, speaker-side chain-length filtering that prevents infinite gossip propagation, agent diversity via `social_weight` (Principle 20), and the full rumor→wasted-trip→discovery information lifecycle. Several of these setups also seed the speaker with an explicit belief about the intended listener; co-location alone is not always sufficient when the scenario uses blind or otherwise isolated perception.
**Emergent behavior proven**:
- Speakers can autonomously select `ShareBelief`, execute Tell, and cause listeners to replan from reported information.
- Critically hungry agents relieve survival pressure before any told-belief transfer can resolve, proving `ShareBelief` stays suppressed under high self-care pressure in the real AI loop.
- Relay chains degrade provenance and confidence across hops rather than duplicating the original direct observation unchanged.
- A stale believed `ResourceSource` quantity can pull an agent to travel, then be contradicted by local observation, forcing abandonment of the invalid harvest path.
- A skeptical listener can reject a told belief cleanly without mutating belief state or producing follow-up travel.
- A bystander can witness the same-place social act without receiving the underlying told belief content, preserving information locality.
- An agent can discover that an expected entity is absent from the locally re-observed place without the belief system inventing a replacement location.
- Explicit conversation memory suppresses unchanged repeat telling to the same listener without relying on same-place subject location.
- A material change in the speaker's shareable belief content lawfully re-enables `ShareBelief`, refreshes told-memory state, and updates the listener's belief content through a second real Tell.
- Conversation-memory expiry lawfully re-enables telling even when the underlying belief content has not changed, proving retention-aware reads rather than write-only cleanup semantics.
- Decision traces expose both halves of the E15c resend contract: omission as `SpeakerHasAlreadyToldCurrentBelief` and later re-enablement after belief change or expiry.
- Chain-length filtering is speaker-side: a relay agent with `max_relay_chain_len=1` cannot relay a chain_len=2 rumor, preventing infinite gossip propagation through the 4-agent chain A→B→C→(blocked)→D.
- Agent diversity (Principle 20): agents with different `social_weight` values produce distinct social behavior — high-weight agents tell early, medium-weight agents tell eventually, zero-weight agents never tell because the zero-motive filter in `rank_candidates()` excludes goals with `motive_score == 0`.
- The full information lifecycle: a Rumor received via autonomous Tell drives travel to a depleted orchard, where passive observation emits a resource-source discrepancy discovery event, replacing the Rumor-sourced belief with DirectObservation of the actual empty state.
- The social slice exposed and fixed three architectural gaps: share-belief plans were only partially wired through planner payload/progress semantics, perception contradicted entity/location state but not stale `ResourceSource` quantities, and goals with zero motive score could still be planned and executed (now prevented by the zero-motive filter in ranking).
**Cross-system chain**: Belief pressure/opportunity plus explicit conversation memory → `ShareBelief` candidate generation and ranking → Tell execution and report propagation → listener replanning, while bystanders receive only witnessed-social evidence and local re-observation emits discovery when expectations are violated. The E15c resend lifecycle chains initial Tell → told-memory write → omitted repeat via decision trace → belief change or memory expiry → lawful re-enable → second Tell. The rumor lifecycle chains Tell → belief → travel plan → perception → discovery → belief correction → replan.
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

### Scenario 3f: Spatial Multi-Hop Plan from the VillageSquare Hub
**File**: `golden_ai_decisions.rs` | **Tests**: `golden_spatial_multi_hop_plan`, `golden_spatial_multi_hop_plan_replays_deterministically`
**Systems exercised**: Needs, AI (candidate generation, plan selection tracing, planning), Travel, Production, deterministic replay
**Setup**: Critically hungry agent starts at Village Square, the branchiest hub in the prototype topology. No local edible commodities are available, and Orchard Farm is the only known food source via OrchardRow workstation + `ResourceSource`. The intended route is `VillageSquare -> SouthGate -> EastFieldTrail -> OrchardFarm`.
**Emergent behavior proven**:
- Tick-0 decision tracing exposes the selected path boundary directly: the chosen plan is a fresh search selection whose next step is `Travel` toward `SouthGate`, not a vague "eventually arrived somewhere" assertion.
- The agent leaves Village Square, enters real in-transit travel, reaches Orchard Farm, and starts the remote harvest lifecycle there.
- Hunger relief occurs only after the remote acquisition chain completes, proving the branchy-hub route remained reachable under the default planning budget.
- Two runs with the same seed produce identical world and event-log hashes for the VillageSquare spatial scenario.
**Cross-system chain**: Need pressure + world belief about remote food → default-budget branchy-hub plan search → selected travel-led path toward South Gate → sequential travel execution → Orchard Farm harvest → downstream hunger relief.

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
- Candidate generation and ranking keep the bread-production path live even though the baker starts without recipe inputs.
- Baker first acquires the unpossessed local firewood lot.
- Baker then crafts bread via the normal production action and consumes it to reduce hunger.
- Firewood is consumed exactly once, bread is produced and then consumed, and the same-seed run replays to identical world and event-log hashes.
**Cross-system chain**: Hunger pressure → reachable local production path → local input acquisition → craft progress barrier → consume crafted output.

### Scenario 6a-remote: Remote Recipe-Input Acquisition Chain
**File**: `golden_production.rs` | **Tests**: `golden_remote_acquire_commodity_recipe_input`, `golden_remote_acquire_commodity_recipe_input_replays_deterministically`
**Systems exercised**: AI (candidate generation, ranking, prerequisite-aware planning), Travel, Transport, Production (craft), Needs, Conservation, deterministic replay
**Setup**: Hungry baker starts at Village Square with the `Bake Bread` recipe and a local mill, but the only known firewood lot is on the ground at Orchard Farm. No direct bread seller/source is available.
**Emergent behavior proven**:
- Candidate generation keeps `ProduceCommodity { Bake Bread }` live as the top-level need-serving branch even when the required input and the workstation are separated across places.
- The planner selects the full remote chain: multi-leg travel out to Orchard Farm, `pick_up` of the remote firewood lot, multi-leg return to Village Square, then `craft:Bake Bread`.
- The real AI loop executes the full chain, then picks up and consumes the crafted bread to reduce hunger.
- Decision traces expose prerequisite-aware search guidance on tick 0, and the same-seed run replays to identical world and event-log hashes.
**Cross-system chain**: Hunger pressure → reachable remote production path → multi-leg travel to remote input → remote `pick_up` → multi-leg return to workstation → craft progress barrier → local output pickup → consume crafted output.

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

### Scenario 6d: Materialized Output Ownership Prevents Theft
**File**: `golden_production.rs` | **Test**: `golden_materialized_output_ownership_prevents_theft`
**Systems exercised**: Production (craft/materialization ownership), Needs, AI runtime (progress barriers, fresh replanning), Ownership/control, Conservation
**Setup**: Two hungry agents share Village Square. Crafter has firewood, knows `Bake Bread`, and has a local mill. A second hungry agent is present locally and would consume opportunistically if the bread output were unowned and controllable.
**Emergent behavior proven**:
- Crafter crafts bread locally and the output materializes with ownership/control that keeps the result on the crafter's lawful path instead of exposing it as free local theft.
- The competing hungry agent does not lawfully steal the freshly materialized crafted output.
- The craft follow-through remains coherent across the progress barrier because ownership is preserved at materialization time rather than patched afterward.
**Cross-system chain**: Local craft/materialization ownership policy → lawful control boundary at the produced lot → downstream self-consumption without opportunistic theft.

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

### Scenario 29: Deprivation Wound Worsening Consolidates Not Duplicates
**File**: `golden_emergent.rs` | **Tests**: `golden_deprivation_wound_worsening_consolidates_not_duplicates`, `golden_deprivation_wound_worsening_consolidates_not_duplicates_replays_deterministically`
**Systems exercised**: Needs (deprivation exposure, repeated critical-threshold firing), Combat (`WoundList` identity and severity accumulation), deterministic replay
**Setup**: Single starving agent at Village Square with hunger already above critical, a pre-seeded `DeprivationExposure` one tick short of firing, and a custom `CombatProfile`/`MetabolismProfile` that allows two starvation fires without death or recovery. No food, no other agents, and no competing need growth.
**Emergent behavior proven**:
- The first starvation threshold fire creates one deprivation wound.
- The second starvation threshold fire worsens that same wound instead of creating a duplicate.
- `WoundId` is preserved while severity increases and `inflicted_at` advances.
- The agent stays alive long enough for the second fire, proving the live cumulative arithmetic rather than a one-fire death path.
- Two runs with the same seed produce identical world and event-log hashes.
**Cross-system chain**: Critical hunger persists → `DeprivationExposure` reaches tolerance → needs system fires starvation damage → first wound created → exposure resets and accumulates again → second fire worsens the existing wound → persistent wound identity preserved.

### Scenario 30: Recovery-Aware Priority Boost Eats Before Wash
**File**: `golden_combat.rs` | **Tests**: `golden_recovery_aware_boost_eats_before_wash`, `golden_recovery_aware_boost_eats_before_wash_replays_deterministically`
**Systems exercised**: AI ranking (`promote_for_clotted_wound_recovery`), Needs (`ConsumeOwnedCommodity`, `Wash`), Combat (`recovery_conditions_met`, natural recovery), decision/action tracing, deterministic replay
**Setup**: Single agent at Village Square with one clotted wound, hunger at the High threshold band, dirtiness also High, carried bread and water, equal default utility weights, and recovery enabled. The live arithmetic is intentionally asymmetric: wash starts with the higher motive score because dirtiness pressure exceeds hunger pressure, while eat becomes `Critical` only through recovery-aware promotion.
**Emergent behavior proven**:
- Initial ranking contains both eat and wash, with wash carrying the higher motive score.
- Recovery-aware promotion lifts eat from `High` to `Critical`, so the agent eats before washing.
- Eating drops hunger below the recovery gate threshold.
- Once recovery conditions are met, wound severity begins decreasing through natural recovery.
- The agent can still wash later; the scenario proves ordering and downstream recovery, not permanent suppression of wash.
- Two runs with the same seed produce identical world and event-log hashes.
**Cross-system chain**: Clotted wound + High hunger → recovery-aware ranking promotion → eat selected over higher-motive wash branch → hunger relief opens recovery gate → combat system reduces wound severity → later wash remains lawful.

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

### Scenario 14: Threaten with Courage Diversity (Principle 20)
**File**: `golden_offices.rs` | **Test**: `golden_threaten_with_courage_diversity`
**Systems exercised**: Threaten (courage comparison → yield/resist), Succession (support counting, coalition majority, installation), AI (candidate generation for ClaimOffice, coalition-aware GOAP planning with Threaten + DeclareSupport, SupportCandidateForOffice from threat-induced loyalty), Agent diversity (Principle 20)
**Setup**: Vacant office (Support law, period=5). Agent A (attack_skill=pm(800), enterprise_weight=pm(900)) at VillageSquare. Agent B (courage=pm(200), social_weight=pm(600)) at VillageSquare — should yield. Agent C (courage=pm(900), social_weight=pm(600)) at VillageSquare — should resist. Agent D (competitor, enterprise_weight=pm(800)) at OrchardFarm — already self-declared support, creates contested scenario. D placed at different location so planner cannot target D with Threaten.
**Emergent behavior proven**:
- A generates ClaimOffice goal. DeclareSupport alone would tie with D (ProgressBarrier). Coalition-aware planner finds Threaten(B) viable (800 > 200) but not Threaten(C) (800 < 900).
- A threatens B → B yields → loyalty increase. B autonomously generates SupportCandidateForOffice(A).
- A's coalition (self + B = 2) exceeds D's (self = 1). Succession system installs A.
- C does not gain loyalty to A (high courage prevents yield). Agent diversity: same Threaten action, different courage → divergent outcomes.
**Foundation alignment**: Principle 20 (agent diversity — per-agent courage produces divergent behavioral outcomes), Principle 10 (belief-only planning), Principle 1 (maximal emergence — threat → yield → loyalty → support is emergent, not scripted).
**Cross-system chain**: AI goal → coalition-aware planner Threaten op → courage comparison → yield/resist divergence → loyalty increase → target AI SupportCandidateForOffice → DeclareSupport → support counting → decisive installation.

### Scenario 15: Travel to Distant Jurisdiction for Office Claim
**File**: `golden_offices.rs` | **Test**: `golden_travel_to_distant_jurisdiction_for_claim`
**Systems exercised**: Travel (multi-hop pathfinding), DeclareSupport, Succession (installation), AI (ClaimOffice goal generation, multi-step Travel + DeclareSupport planning)
**Setup**: Vacant office (Support law, period=5, no eligibility) at VillageSquare. Single sated agent at BanditCamp (3 hops / 12 travel ticks away via BanditCamp → ForestPath → NorthCrossroads → VillageSquare). enterprise_weight=pm(800). Agent has beliefs about the remote office.
**Emergent behavior proven**:
- Agent generates ClaimOffice goal from beliefs about a remote vacant office.
- Planner correctly identifies that DeclareSupport requires co-location at the jurisdiction (Principle 7 locality guard). Plans multi-hop Travel + DeclareSupport.
- Agent traverses the 3-hop route to VillageSquare, then declares support for self.
- Succession system installs agent as holder after succession period.
**Foundation alignment**: Principle 7 (locality — political actions require co-location at jurisdiction), Principle 8 (preconditions — travel has duration and occupancy), Principle 10 (belief-only planning), Principle 1 (maximal emergence — travel + office claim is emergent, not scripted).
**Cross-system chain**: AI goal from remote belief → multi-hop travel planning → sequential travel execution → arrival at jurisdiction → DeclareSupport → succession resolution → office installation.

### Scenario 16: Political Office Facts Remain Local Until Belief Update
**File**: `golden_offices.rs` | **Tests**: `golden_information_locality_for_political_facts`, `golden_information_locality_for_political_facts_replays_deterministically`
**Systems exercised**: AI political candidate generation, decision tracing, belief acquisition via explicit report seeding, travel, political actions (`declare_support`), succession, deterministic replay
**Setup**: Vacant office ("Village Elder") at Village Square with `SuccessionLaw::Support`. A politically ambitious agent starts at Bandit Camp with no belief about the office. After an initial phase proving political inactivity, the test injects an explicit `PerceptionSource::Report` office belief from an informant at Village Square.
**Emergent behavior proven**:
- Before the office belief exists, the remote agent never generates `ClaimOffice` or office-specific political goals in decision traces.
- Before the office belief exists, the remote agent stays at Bandit Camp and does not begin the office-claim travel chain.
- The belief update enters as an explicit report rather than an impossible remote direct observation.
- After the office belief arrives, ordinary office-planning behavior appears: `ClaimOffice` is generated, travel to Village Square occurs, `declare_support` executes, and succession installs the claimant.
- Two runs with the same seed produce identical world and event-log hashes.
**Foundation alignment**: Principle 7 (information locality), Principle 10 (belief-only planning), Principle 13 (knowledge acquisition path matters), Principle 27 (decision traces make the negative AI contract inspectable).
**Cross-system chain**: No office belief → no political candidate generation → explicit reported office belief update → `ClaimOffice` candidate → travel to jurisdiction → `declare_support` → succession resolution.

### Scenario 17: Survival Pressure Suppresses Political Goals
**File**: `golden_offices.rs` | **Tests**: `golden_survival_pressure_suppresses_political_goals`, `golden_survival_pressure_suppresses_political_goals_replays_deterministically`
**Systems exercised**: Needs/self-care (`eat`), AI (ClaimOffice candidate generation, shared goal-policy suppression, ranking, action sequencing), Political actions (`declare_support`), Succession (installation), action tracing, deterministic replay
**Setup**: Vacant office (Support law, period=5, no eligibility) at VillageSquare. Single agent ("Hungry Claimant") at VillageSquare with enterprise_weight=pm(800), one owned bread, and DirectObservation belief about the office. Hunger starts exactly at the agent's `High` threshold so the shared self-care suppression rule applies immediately.
**Emergent behavior proven**:
- ClaimOffice still exists as a lawful political path, but ranking suppresses it while self-care pressure remains `High-or-above`.
- Agent commits `eat` before any `declare_support` commit.
- No `declare_support` commit occurs while hunger remains at or above the `High` threshold.
- Once hunger relief is achieved, the same office-claim architecture proceeds normally: the agent declares support for self and is installed after the succession period.
- Two runs with the same seed produce identical world and event-log hashes for the suppression scenario.
**Foundation alignment**: Principle 10 (belief-only planning), Principle 20 (enterprise ambition remains real but is subordinated to concrete self-care pressure), Principle 24 (no office system special-case; shared goal-policy suppression coordinates the behavior through state).
**Cross-system chain**: Believed vacant office + enterprise motive → ClaimOffice candidate → shared self-care suppression in ranking → eat commit → suppression lift → DeclareSupport → succession resolution → office installation.

### Scenario 18: Faction Eligibility Filters Office Claim
**File**: `golden_offices.rs` | **Test**: `golden_faction_eligibility_filters_office_claim`
**Systems exercised**: Factions (`member_of` relation), Succession (support-law installation), AI (belief-driven ClaimOffice candidate generation, decision tracing), Political actions (`declare_support`), action tracing
**Setup**: Vacant office ("Village Elder") at Village Square with `EligibilityRule::FactionMember(faction)`. Agent A ("Faction Claimant") and Agent B ("Unaffiliated Rival") are both sated, colocated, politically ambitious, and have DirectObservation beliefs about the office. Only A belongs to the required faction.
**Emergent behavior proven**:
- The eligible faction member generates `ClaimOffice` while the office is visibly vacant.
- The ineligible rival never generates `ClaimOffice` in decision traces, proving the filter happens at candidate generation rather than only at action-time rejection.
- The ineligible rival never commits `declare_support`.
- The eligible faction member becomes office holder after the succession period.
**Foundation alignment**: Principle 10 (agents plan from beliefs, not omniscient world shortcuts), Principle 20 (shared ambition still respects concrete eligibility constraints), Principle 24 (AI filtering and authoritative validation coordinate through state rather than special-case cross-calls).
**Cross-system chain**: Faction membership state + believed vacant office → AI eligibility gate on `ClaimOffice` candidate generation → only lawful claimant plans `DeclareSupport` → succession resolves to eligible office holder.

### Scenario 19: Force Succession Installs Sole Living Eligible Contender
**File**: `golden_offices.rs` | **Test**: `golden_force_succession_sole_eligible`
**Systems exercised**: Succession (force-law installation), death-state filtering via `DeadAt`, AI/action suppression for support-law political paths, action tracing
**Setup**: Vacant office ("War Chief") at Village Square with `SuccessionLaw::Force`. Agent A ("Force Claimant") is sated, politically ambitious, and has a direct belief about the office. Agent B ("Dead Rival") is colocated and otherwise similar but already has `DeadAt(Tick(0))`.
**Emergent behavior proven**:
- The force-law succession path installs the sole living eligible contender after the succession period without relying on support counting.
- Dead contenders are filtered out of the eligibility set.
- Even when the live contender is informed and politically ambitious, no `declare_support` commit occurs for the Force-law office.
**Foundation alignment**: Principle 3 (office succession follows concrete alive/dead state rather than abstract weighting), Principle 8 (office resolution still follows explicit preconditions and timing), Principle 24 (AI/political behavior is constrained by office state rather than cross-system special cases).
**Cross-system chain**: visible vacant Force-law office + colocated contenders + one contender marked dead → AI suppresses support-law office goals → succession system resolves sole living eligible contender → office holder relation updates deterministically.

### Scenario 19b: Force Succession Deterministic Replay
**File**: `golden_offices.rs` | **Test**: `golden_force_succession_deterministic_replay`
**Systems exercised**: Same as Scenario 18 + deterministic replay verification.
**Setup**: Same as Scenario 19, run twice with identical seed.
**Assertion focus**: Both runs produce identical world and event-log hashes while still yielding a non-trivial office-holder transition.

### Scenario 21: Combat Death Triggers Force-Law Succession
**File**: `golden_emergent.rs` | **Tests**: `golden_combat_death_triggers_force_succession`, `golden_combat_death_triggers_force_succession_replays_deterministically`
**Systems exercised**: Combat (`attack` lifecycle, wound/death), Politics (`succession_system`, vacancy mutation, force-law installation), action tracing, event-log delta inspection, deterministic replay
**Setup**: Occupied office ("War Chief") at Village Square with `SuccessionLaw::Force` and succession period 5. The incumbent office holder starts alive and assigned to the office. A colocated hostile challenger has a lethal combat profile; the incumbent has a fragile combat profile. Both have perception profiles, and hostility is seeded so the real `EngageHostile` path produces attack execution.
**Emergent behavior proven**:
- The challenger commits real `attack` actions and kills the incumbent through the ordinary combat/death path, producing an authoritative `DeadAt` mutation.
- The political system later observes that the living holder is gone, emits a vacancy mutation that removes the office-holder relation, and only after the configured succession delay installs the surviving challenger.
- No `declare_support` action commits occur anywhere in the chain, proving the result comes from force-law office semantics rather than support-law political AI.
- The test asserts mixed ordering at the correct architectural layers: attack via action trace, death/vacancy/installation via event-log deltas and authoritative state.
- Two runs with the same seed produce identical world and event-log hashes.
**Foundation alignment**: Principle 1 (combat aftermath propagates into politics without scripting), Principle 9 (death leaves downstream aftermath), Principle 24 (combat and politics interact only through state and event history), Principle 26 (no synthetic compatibility action is added for succession).
**Cross-system chain**: hostility + colocated perception → `attack` commit → combat death (`DeadAt`) → political vacancy mutation → succession delay elapses → force-law office-holder installation.

### Scenario 22: Social Tell Propagates Remote Political Knowledge Into Office Claim
**File**: `golden_emergent.rs` | **Tests**: `golden_tell_propagates_political_knowledge`, `golden_tell_propagates_political_knowledge_replays_deterministically`
**Systems exercised**: Social (`tell` lifecycle), belief transfer/provenance degradation, AI political candidate generation, travel, political action execution (`declare_support`), succession, deterministic replay
**Setup**: Vacant office ("Village Elder") at Village Square with `SuccessionLaw::Support` and succession period 5. The informant and ambitious listener start colocated at Bandit Camp, away from the office jurisdiction. The informant has high `social_weight`, Tell capability, and a direct belief about the remote office plus a direct listener belief. The ambitious listener is Tell-capable for reception and starts without any office belief.
**Emergent behavior proven**:
- Before Tell commits, the listener generates no `ClaimOffice` candidate in decision traces and therefore cannot enter the political path through omniscient access to world truth.
- The informant commits a real `tell` action that transfers the office belief as `PerceptionSource::Report { from: informant, chain_len: 1 }`.
- After the told belief arrives, the listener generates `ClaimOffice`, travels from Bandit Camp to Village Square, commits `declare_support`, and becomes office holder through the ordinary support-law succession path.
- The test proves the social system can act as the lawful carrier for a remote political fact without any test-only belief injection or change to same-place Tell suppression.
- Two runs with the same seed produce identical world and event-log hashes.
**Foundation alignment**: Principle 1 (the social-to-political chain is emergent rather than scripted), Principle 7 (knowledge stays local and colocated), Principle 13 (belief provenance matters), Principle 24 (social and political systems interact only through transferred belief state).
**Cross-system chain**: remote office belief held by informant + colocated listener at Bandit Camp → `tell` commit → reported office belief in listener memory → `ClaimOffice` candidate generation → travel to Village Square → `declare_support` → succession installation.

### Scenario 23: Wounded Politician Care-vs-Politics Ordering
**File**: `golden_emergent.rs` | **Tests**: `golden_wounded_politician_pain_first`, `golden_wounded_politician_enterprise_first`, `golden_wounded_politician_replays_deterministically`
**Systems exercised**: Care (`heal`), AI candidate generation and ranking, political action execution (`declare_support`), succession, action tracing, decision tracing, deterministic replay
**Setup**: A single agent at Village Square starts wounded, carries one medicine, and has a direct belief about a vacant support-law office at the same jurisdiction. The scenario keeps natural recovery at zero so only the real `heal` action can reduce wounds. Variant A uses a medium-pain wound with high `pain_weight`; Variant B uses a low-pain wound with higher `enterprise_weight`.
**Emergent behavior proven**:
- Both variants generate the two lawful branches: `TreatWounds { self }` and `ClaimOffice`.
- In the medium-pain variant, the agent commits `heal` before `declare_support`.
- In the low-pain variant, the agent commits `declare_support` before `heal`.
- Both variants still converge to the same durable end state: medicine is consumed lawfully, wound load decreases, and succession later installs the agent as office holder.
- Two runs with the same seed produce identical world and event-log hashes.
**Foundation alignment**: Principle 3 (ordering comes from concrete wound state plus utility profile, not domain exceptions), Principle 20 (different concrete agent parameters and state produce different choices), Principle 24 (care and politics interact only through shared state and the generic ranking pipeline).
**Cross-system chain**: wounded agent + medicine + believed vacant office → decision traces show both `TreatWounds` and `ClaimOffice` candidates → shared ranking picks either `heal` or `declare_support` first depending on concrete pain band and utility profile → remaining branch executes later → succession installs office holder after the support-law delay.

### Scenario 24: Same-Place Office Fact Still Requires Tell
**File**: `golden_emergent.rs` | **Tests**: `golden_same_place_office_fact_still_requires_tell`, `golden_same_place_office_fact_still_requires_tell_replays_deterministically`
**Systems exercised**: Social (`tell` lifecycle), belief transfer/provenance, AI political candidate generation, political action execution (`declare_support`), succession, action tracing, decision tracing, deterministic replay
**Setup**: Vacant support-law office at Village Square. Informant and ambitious listener both start at Village Square with Tell enabled, but both use blind perception so co-location with the office does not passively seed office knowledge. The informant is explicitly seeded with a direct belief about the listener so `ShareBelief { listener, subject: office }` is lawful once the office fact is learned; the listener starts colocated with the office but without the office belief.
**Emergent behavior proven**:
- Before Tell, decision traces show no `ClaimOffice` generation for the listener even though the listener already shares the office's place.
- After the informant learns the office fact, the informant generates and commits a real `tell` for that same-place office subject.
- The listener receives the office fact as `PerceptionSource::Report { from: informant, chain_len: 1 }`, then generates `ClaimOffice`.
- The listener commits `declare_support` only after the Tell commit and becomes office holder through the ordinary support-law succession path.
- Two runs with the same seed produce identical world and event-log hashes.
**Foundation alignment**: Principle 7 (co-location is not omniscience), Principle 12 (world state is not belief state), Principle 13 (knowledge must travel through a lawful carrier), Principle 24 (politics unlocks only from changed listener belief state).
**Cross-system chain**: same-place office + blind listener without office belief → no political candidate despite co-location → informant learns office fact → `tell` commit at the same place → reported office belief enters listener memory → `ClaimOffice` generation → `declare_support` → succession installation.

### Scenario 25: Already-Told Recent Subject Does Not Crowd Out Untold Office Fact
**File**: `golden_emergent.rs` | **Tests**: `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact`, `golden_already_told_recent_subject_does_not_crowd_out_untold_office_fact_replays_deterministically`
**Systems exercised**: Social (`tell` lifecycle), conversation-memory resend suppression, decision-trace social omission status, belief transfer, AI political candidate generation, travel, political action execution (`declare_support`), succession, deterministic replay
**Setup**: Informant and ambitious listener start colocated at Bandit Camp with blind perception and Tell enabled. The informant is explicitly seeded with a direct belief about the listener so `ShareBelief` candidates can materialize under the isolated setup. The informant has `max_tell_candidates = 1`, first learns and tells a more recent non-office subject (`OrchardFarm` rather than an agent, to avoid unrelated extra social branches), then later learns an older untold office fact for Village Square. The listener starts without the office belief and away from the office jurisdiction.
**Emergent behavior proven**:
- The first Tell delivers the more recent lawful subject, establishing conversation-memory state without yet unlocking politics.
- After the office fact is added, decision traces explicitly omit the already-told recent subject as `SpeakerHasAlreadyToldCurrentBelief` and still generate the untold office `ShareBelief` goal.
- The informant commits exactly two Tell actions before the office fact arrives on the listener: the original recent subject, then the older office fact. There is no duplicate resend commit for the recent subject in between, and the committed action trace exposes the concrete `listener`/`subject` payload for both tells.
- The listener generates no `ClaimOffice` before receiving the office fact, then generates `ClaimOffice`, travels to Village Square, commits `declare_support`, and becomes office holder afterward.
- Two runs with the same seed produce identical world and event-log hashes.
**Foundation alignment**: Principle 13 (conversation memory governs what knowledge is worth repeating), Principle 18 (bounded reasoning and truncation still preserve lawful untold content), Principle 19 (new evidence revises the next social action), Principle 24 (downstream politics unlocks only through the changed belief state), Principle 27 (decision traces make the omission reason inspectable).
**Cross-system chain**: recent subject told once → conversation memory records told state → office fact later enters informant belief store → stale recent subject omitted before truncation → untold office fact still told → listener gains reported office belief → `ClaimOffice` generation → travel to Village Square → `declare_support` → succession installation.

### Scenario 26: Contested Harvest Start Failure Recovers Via Remote Fallback
**File**: `golden_production.rs` | **Tests**: `golden_contested_harvest_start_failure_recovers_via_remote_fallback`, `golden_contested_harvest_start_failure_recovers_via_remote_fallback_replays_deterministically`
**Systems exercised**: Needs, Production (`harvest`), request-resolution tracing, action tracing, decision tracing, blocked-intent memory, travel, conservation, deterministic replay
**Setup**: Two equally hungry agents start at Village Square with production perception and seeded world beliefs for both a single-batch local orchard and a larger remote orchard at Orchard Farm. The setup intentionally keeps only one contested local harvest opportunity plus one lawful remote fallback so the post-selection loss branch stays reviewable instead of being masked by unrelated local food options.
**Emergent behavior proven**:
- At tick 0, both contenders select the same local `Harvest` step against the Village Square orchard from the same belief snapshot.
- Request-resolution traces show both queued harvest requests bind through the shared runtime path and reach authoritative start attempt against the same local workstation; this is not a pre-start rejection case.
- Action traces then show the winner starts the local harvest while the loser records exactly one `StartFailed` harvest due to authoritative `ReservationUnavailable`.
- On the next AI tick, decision traces expose the structured start failure, confirm the failed step was not retained as the current plan, and the loser records a local reservation-conflict blocker rather than crashing or livelocking.
- The winner exhausts the single local batch, while the loser later travels to Orchard Farm, harvests remotely, and reduces hunger.
- Authoritative apple totals remain bounded by the combined seeded stock, and same-seed runs replay to identical world and event-log hashes.
**Foundation alignment**: Principle 1 (the recovery chain emerges from lawful contention rather than scripting), Principle 8 (contention is resolved through a concrete reservation-backed start boundary), Principle 19 (intent is revisable and does not reserve the orchard), Principle 24 (production contention and AI recovery interact through shared state and traces, not domain-specific glue).
**Cross-system chain**: shared hunger pressure + shared local orchard belief → same-snapshot local harvest selection → request binds and authoritative start contention → losing `StartFailed` harvest → next-tick blocker/replan → travel to remote orchard → remote harvest → hunger relief.
**Distinct from Scenario 3d**: Scenario 3d proves finite-source contention and conservation under same-tick pressure. Scenario 26 proves the stronger S08/S15 contract: the losing branch reaches authoritative start, records a structured `StartFailed`, clears the stale plan on the next AI tick, and then recovers through a different downstream branch.

### Scenario 27: Local Trade Start Failure Recovers Via Production Fallback
**File**: `golden_trade.rs` | **Tests**: `golden_local_trade_start_failure_recovers_via_production_fallback`, `golden_local_trade_start_failure_recovers_via_production_fallback_replays_deterministically`
**Systems exercised**: Needs, Trade (`trade`), Care warmup/self-care occupancy, request-resolution tracing, action tracing, decision tracing, blocked-intent memory, Production fallback, deterministic replay
**Setup**: A seller, a winning buyer, and a losing buyer start at Village Square with a single edible local trade opportunity and a remote orchard fallback at Orchard Farm. The losing buyer is intentionally occupied with lawful self-care first, so the winner can consume the only local bread and make the later queued trade request stale. Competing unrelated local food branches are removed so the intended stale-trade recovery path remains visible.
**Emergent behavior proven**:
- The winner completes the only local bread trade while the loser is still lawfully occupied with `heal`, so the local market opportunity disappears through ordinary world action rather than test-side mutation.
- When the stale loser trade request is later retried, request-resolution tracing shows it still binds through the shared runtime path and reaches start attempt against the seller.
- Action traces then show exactly one loser `trade` `StartFailed`, and authoritative failure data records `HolderLacksAccessibleCommodity` for the seller's missing bread.
- On the next AI tick, decision traces expose that structured failure, prove the stale local trade plan is not retained, and record seller-specific out-of-stock blocker memory rather than blocking all food acquisition.
- The loser then switches to the remote orchard path, harvests there, reduces hunger, and does not loop on repeated stale local trade starts.
- Same-seed runs replay to identical world and event-log hashes.
**Foundation alignment**: Principle 1 (market opportunity drift becomes a new causal chain), Principle 8 (the local trade affordance is not silently reserved by planning), Principle 19 (the stale trade plan is revisable once the world changes), Principle 24 (trade failure propagates into production fallback through shared state and AI recovery).
**Cross-system chain**: hunger pressure + local seller belief + temporary self-care occupancy → winner consumes single local trade opportunity → stale loser trade reaches authoritative start and fails → next-tick blocker/replan → remote harvest fallback → hunger relief.
**Distinct from Scenario 2b**: Scenario 2b proves successful buyer-driven trade acquisition. Scenario 27 proves lawful loss of a local trade opportunity after planning, structured `StartFailed` recovery through the shared runtime path, and downstream pivot into production instead of repeated stale market retries.

### Scenario 28: Remote Office Claim Start Failure Loses Gracefully
**File**: `golden_emergent.rs` | **Tests**: `golden_remote_office_claim_start_failure_loses_gracefully`, `golden_remote_office_claim_start_failure_loses_gracefully_replays_deterministically`
**Systems exercised**: Care (`heal`), AI political candidate generation, action tracing, decision tracing, political action execution (`declare_support`), support-law succession, deterministic replay
**Setup**: A vacant support-law office exists at Village Square. Both the eventual winner and the delayed claimant receive lawful reported beliefs about the office. The delayed claimant is intentionally wounded and starts by lawfully prioritizing self-care, while a supporter has already declared for the winner so the winner can close the office first through the ordinary `declare_support` path. This isolates post-selection political opportunity loss without removing the real political substrate.
**Emergent behavior proven**:
- Before the office closes, decision traces show the delayed claimant still generates `ClaimOffice`, but concrete self-care pressure selects `TreatWounds { self }` first, making the political request stale through lawful occupancy rather than scripting.
- The winner commits `declare_support` through the ordinary action path and closes the vacancy before the delayed claimant retries.
- When the delayed claimant later retries `declare_support`, action traces show a single `StartFailed` political action, and the authoritative failure reason is `PreconditionFailed(... not vacant ...)`.
- On the next AI tick, decision traces expose that failure, confirm the stale political plan is not retained, and show the occupied office disappearing from generated `ClaimOffice` candidates with an explicit `OfficeNotVisiblyVacant` omission reason.
- The delayed claimant records exactly one political start failure and does not loop on repeated stale claim attempts. Same-seed runs replay to identical world and event-log hashes.
**Foundation alignment**: Principle 12 (claim generation still depends on local belief, not omniscience), Principle 19 (intent to claim does not reserve the office), Principle 21 (office succession remains a real world process), Principle 24 (care and politics interact only through shared state and generic runtime recovery).
**Cross-system chain**: reported office belief + self-care pressure → delayed claimant generates `ClaimOffice` but heals first → winner commits `declare_support` and installs through support-law succession → stale delayed `declare_support` hits authoritative `StartFailed` → next-tick political candidate removal and no-retry behavior.
**Distinct from Scenarios 22 and 24**: Scenarios 22 and 24 prove that lawful office knowledge can unlock political planning and success. Scenario 28 proves the opposite branch: once another actor lawfully closes the opportunity first, the delayed claimant loses cleanly through the same shared start-failure architecture instead of crashing or retaining a stale office claim forever.
