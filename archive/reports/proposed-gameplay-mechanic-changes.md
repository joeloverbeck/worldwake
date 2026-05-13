**Status**: COMPLETED

## **Verdict**

The baseline proves **causal substrate sufficiency**, not deep survival gameplay. That is a good result, but it should not be mistaken for a “full” mechanic set. The agents discover resources, satisfy all five needs, recover from simple reservation blockers, and stay far below critical thresholds; but the loop becomes mostly solved after the first few ticks: food and water are abundant, travel is free, sleep is a repeated recovery commit, relieving and washing are periodic maintenance checks, perception mostly admits local truth into memory, and contention becomes a blocker cooldown rather than a visible social/physical situation.

The target should be: **every survival action becomes a causal investment with explicit opportunity cost, partial failure, local information, visible aftermath, and future strategic consequences**, while still obeying Worldwake’s rules against authored drama, omniscient planning, abstract truth scores, and hidden shortcut systems.

Research points in the same direction. MDA treats mechanics as data/algorithm rules that generate runtime dynamics and then player experience, so the fix is not “add more content,” but strengthen the causal rules that create richer dynamics. Salen and Zimmerman’s “meaningful play” criterion is especially relevant: action/outcome relationships must be both **discernable** and **integrated** into later play. Doug Church’s formal design tools push the same idea through **intention** and **perceived consequence**: players can plan when world reactions are consistent and understandable. Adams and Dormans frame mechanics as an internal economy of resources, sources, drains, converters, feedback loops, and strategic interrelationships. Game theory adds the missing multi-agent angle: a situation becomes strategically rich when each agent’s outcome depends on other agents’ choices, beliefs, and repeated future interactions.

So the blunt diagnosis is: **the current survival mechanics are too close to independent counters with obvious remedies.** They need to become a coupled, inspectable survival economy.

---

# **1. Replace threshold-driven self-care with projected homeostatic planning**

Right now, the agents mostly react to current need pressure. That is passable, but shallow. A genuinely intelligent survival agent should reason about **time-to-trouble**, not just “need value × weight.”

### **Fix**

Represent each need as a projected curve:

NeedState

- current_level

- base_rate

- active_modifiers

- comfort_band

- high_band

- critical_band

- projected_tick_of_high

- projected_tick_of_critical

- last_satisfaction_event

- recovery_debt_or_surplus

- current_assumptions

The planner should evaluate actions by asking:

Will this plan keep me inside my acceptable band long enough

to complete my other commitments under my current beliefs?

That makes “eat now,” “harvest extra first,” “sleep first,” “walk to water now,” and “wait for the orchard slot” genuinely different. Current scoring seems to produce reasonable interruptions, such as sleep outscoring apple acquisition when fatigue rises, but it still behaves like local motive comparison rather than survival horizon management.

### **Concrete improvements**

Make each active intention carry explicit assumptions:

PlanAssumptions

- expected_need_levels_at_completion

- expected_resource_availability

- expected_travel_duration

- expected_facility_access

- expected_inventory_state

- expected_wake_conditions

When an assumption breaks, the plan revises. This aligns perfectly with the Foundations rule that intentions are revisable commitments, not rails.

### **Acceptance tests**

Run the same map with more agents and lower source capacity. A strong implementation should produce agents who sometimes:

* harvest before sleeping because projected hunger will breach during the sleep window,  
* sleep before harvesting because the resource is not expected to be contested,  
* carry a reserve before leaving a known water source,  
* abandon a reserve plan when queue time makes the reserve unaffordable,  
* wake early because another need’s projected curve invalidates the sleep commitment.

The key is not more desperation. The key is **planning under time pressure**.

---

# **2. Deepen Eat and Drink into reserve management, not one-unit loops**

Eat and Drink currently work because the world has concrete Water and Apple lots, harvest recipes, inventory, and consumption commits. That is the right substrate. But the behavior is too regular: harvest one, pick up, eat or drink, repeat. The wells never exhaust, the orchard remains sufficient, and agents rarely have reason to think beyond the next unit.

### **Fix**

Make `AcquireCommodity` quantity-aware.

Instead of:

AcquireCommodity { Apple, SelfConsume }

use something closer to:

AcquireCommodity

- commodity: Apple

- purpose: SelfConsume | ReserveForSelf

- desired_min_units

- desired_target_units

- horizon_ticks

- known_or_believed_sources

- acceptable_travel_cost

- acceptable_wait_cost

The agent should sometimes want one apple and sometimes want three, depending on predicted need curves, carrying limits, source distance, observed contention, and whether it expects to return soon.

### **Resource source improvements**

The current capacity/regeneration model is fine for a baseline, but for depth each resource source should expose more causal state:

ResourceSource

- source_id

- commodity

- harvestable_units

- unavailable_regrowing_units

- extraction_slots

- extraction_duration

- last_harvest_events

- local_visible_depletion_state

- local_visible_recovery_state

For the orchard, “regeneration” should eventually become a source process, not just `+1 unit every 2 ticks`. You do not need a full botany sim. You need enough concrete state for agents to reason: “this orchard was heavily picked recently,” “there are only unripe apples visible,” “I can wait,” “I should look elsewhere,” or “I should take more now because everyone is using this place.”

For wells, represent draw/extraction friction even if water supply is effectively abundant. A well can be abundant but still have one bucket, one access point, a short draw duration, and visible occupancy. That creates depth without fake scarcity.

### **Consumption improvements**

Eating and drinking should be partial, not pure reset actions:

ConsumeCommodityOutcome

- consumed_lot_id

- quantity_consumed

- need_delta

- time_to_digest_or_absorb

- leftover_lot_id

- satisfaction_quality

- side_effects

An apple might reduce hunger by a concrete amount and produce a temporary satiety buffer. Drinking might reduce thirst quickly but increase bladder pressure later. That is not a new gameplay feature; it is making Eat and Drink actually connected to the other survival needs.

### **Why this matters**

A deep mechanic has multiple viable strategies. Burgun’s practical definition of depth as the number of viable strategies and tactics is useful here: the current system has breadth of actions, but too few viable survival strategies once a stable food/water loop is discovered.

### **Acceptance tests**

In a six-agent version of this scenario, you should see divergent but explainable strategies:

* one agent harvests one apple and eats immediately,  
* one harvests extra because she has repeatedly lost orchard races,  
* one waits because carrying extra would delay water,  
* one sleeps because projected need curves say the orchard will still be safe later,  
* one explores because both known acquisition paths are blocked or stale.

No scripted personality beats. Just different concrete beliefs, needs, and costs.

---

# **3. Turn Sleep from repeated commits into interruptible sleep episodes**

Sleep is the clearest mechanical smell in the report. Agents committed sleep 143–146 times each, and Agent A produced repeated `sleep → sleep` loop flags. The report correctly calls this benign, but it still reveals that Sleep is currently modeled too much like a short recovery action and not enough like a duration-bearing commitment.

### **Fix**

Replace repeated single sleep commits with `SleepEpisode`.

SleepEpisode

- sleeper_id

- place_id

- start_tick

- intended_min_duration

- intended_max_duration

- recovery_curve

- sleep_quality

- wake_conditions

- interruption_events

- end_reason

Sleep should be one long action with internal ticks, not 140 separate “sleep” choices.

### **Wake conditions**

Sleep should remain interruptible, but only through local or internal causes:

WakeCondition

- projected_need_crosses_high

- projected_need_crosses_critical

- scheduled_commitment_due

- local_disturbance_perceived

- facility_or_place_no_longer_safe_or_valid

- enough_fatigue_recovered

Do not add danger or social interruptions if those systems are inactive. The baseline can still support meaningful sleep interruption through hunger, thirst, bladder, dirtiness, and resource-access assumptions.

### **Sleep quality**

Make place matter without over-simulating bedding:

SleepQualityInputs

- place shelter tag

- ground comfort tag

- weather exposure if active later

- crowding/occupancy if active

- dirtiness/waste presence if hygiene state exists

In the current scenario, sleeping at Fertile Fields, Riverside Camp, Forest Clearing, and Hillside Shelter should not all be identical by default. Hillside Shelter being resource-poor but sleep-good would deepen the existing topology without adding a new “feature.” It would give that dormant place a survival reason while staying inside Sleep and place affordances.

### **Acceptance tests**

A good Sleep mechanic should produce:

* fewer sleep action commits,  
* longer inspectable sleep episodes,  
* early waking when thirst/bladder projection invalidates sleep,  
* different sleep-site preferences between agents,  
* no `sleep → sleep` artifact,  
* causal answers to “why did she sleep here?” and “why did she wake then?”

---

# **4. Make Relieve and Wash environmental mechanics, not checklist actions**

Relieve and Wash are currently functional but thin. The best sign is that wilderness relief creates Waste lots; the bad sign is that those Waste lots do not seem to matter much downstream. Agent C relieved in the wilderness twenty-six times and never used a toilet because she spent most of the run at Fertile Fields, which lacks a Latrine tag. That is causally legible, but not yet rich.

### **Fix Relieve**

Relieve should move bladder pressure into concrete world aftermath.

RelieveOutcome

- agent_id

- place_id

- facility_id optional

- waste_lot_id

- dirtiness_delta_to_agent

- dirtiness_delta_to_place_or_facility

- duration

- privacy_or_exposure_state if social systems later care

For now, ignore privacy and social judgment if social systems are inactive. Focus on physical consequences:

* wilderness relief creates a Waste lot at the place,  
* toilet relief moves waste into a latrine container/capacity,  
* latrine use occupies the facility,  
* latrines can become dirty/full if no disposal/maintenance system exists yet,  
* wilderness relief can increase personal dirtiness or local place dirtiness.

Do not solve this with a hidden `wilderness_penalty`. If wilderness relief is worse, the world should show why.

### **Fix Wash**

Wash should consume or occupy a real facility state.

WashBasinState

- clean_water_units or usable_wash_capacity

- dirtiness_level

- current_user_claim

- recovery_or_refill_process

WashOutcome

- agent_dirtiness_delta

- basin_dirtiness_delta

- water_consumed_or_capacity_used

- duration

- partial_success_if_interrupted

A washbasin should not be a magic tag. It should be a small physical process: use basin, consume wash capacity, reduce personal dirtiness, worsen basin state or consume water, recover/refill through an explicit process.

### **Why this matters**

Relieve and Wash become compelling only when they create **tradeoffs**:

* stay near food but get dirtier from wilderness relief,  
* travel to a latrine but spend time and expose other needs,  
* wash now because dirtiness impairs comfort or sleep quality,  
* postpone washing because water access is contested,  
* avoid a dirty place because it worsens future hygiene.

That is depth inside existing survival mechanics. No disease system required yet.

### **Acceptance tests**

Stress Fertile Fields with all three agents staying there. You should see:

* visible accumulation of Waste lots or place dirtiness,  
* some agents choosing to travel for toilet/wash access,  
* hygiene-sensitive agents behaving differently from hygiene-tolerant agents,  
* washbasin contention when multiple dirty agents arrive together,  
* inspectable causal history for “why is this place dirty?”

---

# **5. Replace hidden blocker cooldowns with explicit contention artifacts**

This is the most important fix.

The baseline’s most interesting event is Agent B losing Camp Well access to Agent A, then exploring Fertile Fields and discovering the food path. That is exactly the kind of emergent substitution Worldwake wants. But the mechanism is still too invisible: “reservation conflict blocker expiring twenty ticks later” is useful planner machinery, but it should not be the authoritative world model of contention.

### **Fix**

Make resource and facility contention a concrete world process.

UseClaim

- claim_id

- claimant_id

- target_entity_id

- action_kind

- state: approaching | reserved | occupying | queued | abandoned | expired | fulfilled

- created_tick

- valid_until_tick

- priority_basis if any

- visible_at_place

- invalidators

FacilityQueue

- queue_id

- target_entity_id

- policy: first_arrival | explicit_line | reservation_only | contested_race

- entries

- current_grant

- expiry_rules

For the Camp Well and North Orchard, the source should expose extraction slots. If there is one slot, only one agent can occupy it. Others can observe occupancy, queue, wait, leave, sleep, explore, or try another source.

### **Important rule**

Do not keep two live authoritative contention systems.

If `queue_for_facility_use` exists but blocker cooldowns actually decide everything, the mechanic is split-brained. The Foundations explicitly reject obsolete compatibility paths in live authority.

### **What changes in behavior**

Agent B’s tick-1 outcome should become:

Agent B sees or discovers:

- Camp Well extraction slot occupied/reserved by Agent A

- expected wait: unknown or estimated

- own thirst/hunger projection

- Fertile Fields frontier belief

Then she chooses:

wait_for_well

queue_for_well

sleep_briefly

explore_fertile_fields

seek_other_water_source if believed

Her exploration choice becomes stronger, not weaker, because it is now based on visible world facts rather than planner-private blocker state.

### **Repeated-game intelligence**

The same agents meet at the same orchard for hundreds of ticks. This should become a repeated strategic situation. Game theory’s repeated-game point is that future encounters alter present choices. Agent A should be able to learn, locally and fallibly:

“Agent B often reaches the orchard first in the morning.”

“Waiting here costs about 12 ticks.”

“Carrying an extra apple avoids the next conflict.”

That is not a social system. It is local observation of recurring resource contention.

### **Acceptance tests**

With two agents and one well:

* the same-tick conflict is resolved by declared tie-breaking,  
* the winner holds a visible claim,  
* the loser can inspect or perceive the claim,  
* the loser’s belief records who blocked them, when, where, and for what,  
* queue entries expire if the agent leaves, sleeps too long, dies, or changes goal,  
* no dead claimant can block access,  
* replay with same seed produces the same grant order.

---

# **6. Rebase need-driven exploration on value of information**

Need-driven exploration worked well in the baseline. Agent B’s blocked water plan became exploration under hunger pressure, and that produced the run’s strongest emergent moment.

But the mechanic should not remain “need crosses threshold, frontier exists, apply exploration boost.” That risks becoming an abstract curiosity lever.

### **Fix**

Exploration should be an information-seeking plan with a concrete hypothesis.

ExploreLocation

- target_place_id

- motivating_need

- hypothesis: may_contain_food | may_contain_water | may_contain_latrine | may_contain_wash | may_offer_sleep_site

- belief_basis

- expected_cost

- expected_information_gain

- expected_survival_value

- abandonment_conditions

The agent should be able to explain:

“I went to Fertile Fields because I was hungry,

I knew it was a nearby field,

and I believed fields might contain food.”

Not:

“The exploration emitter gave me 700.”

The boost can remain as an agent-local preference for novelty, but the selection should mostly derive from belief, topology, place kind, need pressure, and uncertainty.

### **Survey records**

Arrival should produce an explicit `SurveyRecord`.

SurveyRecord

- agent_id

- place_id

- started_tick

- completed_tick

- observed_entities

- searched_for

- found

- not_found_with_confidence

- confidence

This lets an agent become disappointed without global correction. If Hillside Shelter contains no food, an agent who explored it should know “I looked and did not find food,” while another agent who never visited it should remain ignorant.

### **Do not force Hillside Shelter**

The fact that Hillside Shelter remained dormant is not automatically a flaw. In a causal simulation, unused space is acceptable when no agent has a reason to go there. The fix is not “make agents visit every authored place.” The fix is: make sure agents can form a lawful reason to visit it when existing mechanics justify it.

Valid reasons might include:

* needing a better sleep site,  
* avoiding dirty Fertile Fields,  
* escaping resource contention,  
* checking a frontier because known sources became unreliable,  
* following stale belief from prior exploration.

Invalid reason:

“Unvisited location should get content.”

That violates the whole project.

### **Acceptance tests**

Create a run where the orchard is temporarily depleted or heavily queued. Agents with different beliefs should diverge:

* one waits because she trusts the orchard will recover,  
* one explores Hillside Shelter because she incorrectly expects camp resources,  
* one returns to Riverside Camp because she knows it has water/wash/latrine,  
* one stays because her hunger projection is still safe.

All outcomes should be traceable to beliefs, not map omniscience.

---

# **7. Turn perception into a prediction-error engine**

Activation-decay perception currently gates what enters belief stores. That is useful, but the baseline mostly uses perception for direct discovery: agents see wells, orchard, item lots, peers, and places. The report notes that no told/heard/social/institutional branches fired.

For this scenario, perception should become more than admission control. It should create **expectations**, and then create **surprise** when observations contradict those expectations.

### **Fix**

Every important observation should preserve:

BeliefObservation

- observer_id

- entity_id

- observed_property

- observed_value

- observation_tick

- place_id

- confidence

- source: direct_observation

- freshness_policy

Examples:

North Orchard had 3 visible apples at tick 210.

Camp Well was occupied by Agent A at tick 1.

Fertile Fields had WasteLot#312 near the orchard at tick 600.

Forest Washbasin was clean at tick 540.

Plans should consume beliefs with timestamps, not eternal truths.

### **Expectation violations**

When an agent arrives expecting something and sees otherwise, emit:

ExpectationViolation

- agent_id

- belief_id

- expected

- observed

- place_id

- tick

- plan_invalidated_id

- resulting_belief_update_id

This is central to Worldwake. Surprise should come from violated expectation, not omniscient event notification.

### **Perception and need salience**

Need salience should bias attention, not fabricate truth.

A thirsty agent should be more likely to notice wells, containers, water lots, or wet ground. A hungry agent should attend to orchards, food lots, and other agents carrying food. But an agent should not learn about a remote well because thirst is high.

### **Acceptance tests**

* Agent observes orchard with apples, leaves, another agent harvests them, first agent returns and finds none.  
* First agent updates only on return observation.  
* Second agent who never saw the depletion still believes the old report if no information carrier reached them.  
* Planner logs show the stale belief caused the wasted trip.  
* Debug view can answer both “what happened to the apples?” and “why did Agent A think apples were there?”

---

# **8. Add concrete habit and learning records for survival behavior**

The agents have different weights and rates, but the run still converges quickly: everyone finds the orchard/water loop and maintains needs comfortably. Concrete variation exists, but it is not yet producing enough behavioral character.

### **Fix**

Add agent-local learning records tied to survival experiences.

BlockedIntentRecord

- agent_id

- target_entity_id

- action_kind

- blocker_kind

- blocker_agent_id optional

- tick

- place_id

- resolved_tick

- chosen_fallback

SourceReliabilityMemory

- agent_id

- source_id

- commodity

- observed_successes

- observed_failures

- average_wait_ticks

- last_observed_capacity

- confidence

- decay

SurvivalHabit

- agent_id

- trigger_condition

- preferred_response

- strength

- origin_event_ids

- decay

These records let behavior change without hidden “AI learned” magic.

### **Examples**

Agent A repeatedly loses orchard access to Agent B:

Habit: harvest earlier when hunger projection crosses medium.

Agent C repeatedly commutes between Forest Clearing and Fertile Fields:

Habit: drink before leaving Forest Clearing if thirst projection will cross high before next return.

Agent B learns that staying at Fertile Fields creates latrine/wash problems:

Habit: combine Riverside Camp water trip with toilet and wash when dirtiness/bladder projections justify travel.

This is exactly the kind of concrete adaptation the Foundations require: learning as inspectable state with origin, scope, revision, and decay.

### **Acceptance tests**

Two agents with the same current needs but different histories should choose differently, and the explanation should cite different records:

Agent A waited because prior queue waits were short.

Agent B explored because prior orchard blockers lasted long.

Agent C carried water because prior thirst commute nearly breached high threshold.

---

# **9. Make the topology mechanically expressive without adding new feature families**

The four-place map is already good for this. The problem is that the place differences are not yet deep enough.

Current place identities:

* Riverside Camp: water, washbasin, latrine.  
* Fertile Fields: apples.  
* Forest Clearing: water, washbasin.  
* Hillside Shelter: latrine only, no harvestables.

### **Fix**

Make existing affordance differences matter more.

Riverside Camp should be the strongest all-round survival hub because it has water, wash, and toilet. Fertile Fields should be food-rich but hygiene-poor. Forest Clearing should be a water/wash fallback but toilet-poor. Hillside Shelter should be resource-poor but potentially useful for sleep, toilet, or low-contention recovery.

That does not require merchants, enemies, weather, trade, disease, or quests. It only requires the existing self-care affordances to have sharper consequences.

### **Good emergent patterns to target**

* Agent B lives near the orchard but periodically makes bundled Riverside trips: water + toilet + wash.  
* Agent C prefers Forest Clearing for water/wash but sometimes detours if bladder pressure makes a latrine worth the longer route.  
* Agent A uses Riverside as a recovery base and Fertile Fields as a food expedition.  
* Hillside Shelter remains unused in easy runs but becomes relevant when sleep quality, latrine access, or crowding pressure makes it worthwhile.

### **Bad pattern to avoid**

Do not add a hidden “visit Hillside Shelter” utility. If no concrete need, belief, route, or affordance makes Hillside useful, it should remain unused.

---

# **10. Use dynamic feedback, not arbitrary difficulty knobs**

The report’s needs never become dangerous: hunger peaks at 503, thirst at 309, bladder at 540, dirtiness at 369, and fatigue stays far below critical.

The wrong fix is:

hunger_rate *= 3

resource_capacity /= 2

That may make the run harsher, but it does not make the mechanics fuller.

### **Fix**

Add feedback through concrete world dampeners and amplifiers.

Examples inside current mechanics:

Resource contention loop:

more agents use orchard -> longer waits -> more reserve harvesting or exploration -> pressure redistributes.

Hygiene loop:

more wilderness relief at Fertile Fields -> place gets dirtier -> sleep/wash preference changes -> agents travel to facilities.

Fatigue loop:

longer travel and waiting -> more fatigue -> more sleep -> missed resource windows -> need projection shifts.

Reserve loop:

repeated blockers -> agent carries more reserve -> less frequent contention -> carrying burden or opportunity cost limits hoarding.

Adams and Dormans’ discussion of positive and negative feedback is useful here: feedback should shape system behavior, but runaway loops need balancing structures, ideally dynamic rather than static. Worldwake’s version of that rule is stricter: every feedback loop needs a physical dampener, not an invisible clamp.

### **Acceptance tests**

For each loop, name the dampener:

* Hoarding dampener: carry capacity, spoilage if item decay later exists, travel burden, opportunity cost.  
* Queue dampener: agents switch sources, wait, sleep, carry reserves, or explore.  
* Dirtiness dampener: washing, latrine use, place avoidance, cleanup only when disposal exists.  
* Fatigue dampener: sleep, reduced activity, better sleep sites.

No `max_pressure = 1.0` style clamps should be doing design work.

---

# **11. Add survival-specific partial failures**

The current action failures are mostly start failures from reservation conflicts. Agent C has zero failures. That is clean, but too clean.

### **Fix**

Every survival action should have degraded outcomes.

Not random failure for drama. Concrete partial outcomes from concrete causes.

#### **Harvest**

Full success: harvested desired quantity.

Partial success: harvested fewer units because source depleted during action.

Interrupted success: harvested but did not pick up.

Failed start: no slot, no source, no tool if tools exist later.

Aftermath: source quantity changed, claim released, item lot created.

#### **Eat/Drink**

Full success: consumed enough.

Partial success: consumed less than needed.

Wasteful success: consumed more than useful, increasing bladder or reducing future reserve.

Failed start: item absent, inaccessible, stale belief.

Aftermath: item lot reduced/destroyed, body state changed.

#### **Sleep**

Full success: reached intended recovery.

Partial success: woke early due to need projection.

Bad sleep: low-quality place recovered less.

Interrupted sleep: local disturbance or invalidated assumptions.

Aftermath: fatigue changed, time passed, other needs rose.

#### **Wash**

Full success: dirtiness reduced.

Partial success: basin capacity insufficient.

Failed start: basin occupied or unusable.

Aftermath: basin state dirtier/emptier, agent state cleaner.

#### **Relieve**

Full success: bladder reduced.

Partial success: interrupted or unsuitable place.

Degraded success: wilderness relief increases dirtiness/place waste.

Failed start: facility occupied, inaccessible, or invalid.

Aftermath: waste state created/transferred.

This is the fastest way to make the mechanics feel less binary while staying entirely inside the current feature set.

---

# **12. Instrument the mechanics as authored history, not debug residue**

The report is useful, but it exposes a gap: belief summaries are end-state-heavy, and some causal details are inferred from event logs. For Worldwake, every meaningful mechanic should produce queryable causal and knowledge history.

### **Fix**

Emit stable event identities for:

NeedThresholdCrossed

NeedProjectionChanged

PlanAssumptionFormed

PlanAssumptionInvalidated

UseClaimCreated

UseClaimGranted

UseClaimExpired

QueueEntered

QueueLeft

ResourceObserved

ResourceHarvested

CommodityConsumed

SleepEpisodeStarted

SleepEpisodeEnded

WasteCreated

WashFacilityUsed

ExpectationViolation

HabitUpdated

Each should link to:

- actor

- place

- target entities

- prior belief or prior state

- resulting state

- visible evidence created

- downstream plan affected

The Foundations are explicit that emergence without introspection is indistinguishable from bugs. This is not optional tooling; it is part of the mechanic.

### **Acceptance queries**

The simulation should answer:

Why did Agent B explore Fertile Fields at tick 1?

Who held the Camp Well claim?

What did Agent B know about that claim?

Why did Agent C never use a toilet?

Why did Agent A sleep at tick 1400?

Which beliefs caused Agent A to seek apples at tick 1085?

Which Waste lots exist because of Agent C?

Why did no one visit Hillside Shelter?

If any answer requires source-code archaeology, the mechanic is under-instrumented.

---

# **Recommended implementation order**

## **P0 — Architectural corrections**

1. **Make contention authoritative through world-state claims/queues.**  
    This is the biggest emergence multiplier because it turns A/B’s conflicts into visible, learnable, repeated strategic situations.  
2. **Convert Sleep into `SleepEpisode`.**  
    This removes the `sleep → sleep` artifact and gives sleep real duration, assumptions, interruption, and place quality.  
3. **Add observation/history records for resource beliefs and expectation violations.**  
    This lets stale knowledge, failed trips, and belief correction exist even before social information systems are active.

## **P1 — Survival depth**

4. **Make Eat/Drink quantity-aware with reserve planning.**  
    Agents should manage food/water buffers over time, not just consume the next unit.  
5. **Give Relieve/Wash concrete environmental aftermath.**  
    Waste, dirtiness, basin use, and latrine capacity should become carriers of consequence.  
6. **Rework exploration around explicit hypotheses and survey records.**  
    Keep Agent B’s emergent discovery behavior, but ground it in value-of-information reasoning rather than a flat boost.

## **P2 — Intelligence and validation**

7. **Add survival habit records from repeated local experience.**  
    Agents should adapt to blockers, source reliability, commute timing, and facility access through inspectable memories.  
8. **Run adversarial survival sweeps.**  
    Vary agent count, source capacity, travel duration, queue policy, observation fidelity, and metabolism profiles. Measure plan diversity, queue waits, stale-belief trips, partial failures, source depletion, and need breach windows.

---

# **What I would not do**

Do **not** deepen survival by adding hidden event chances, forced exploration, invisible scarcity flags, or authored emergencies. That would move the game away from explainable emergence.

Do **not** make Hillside Shelter “interesting” by fiat. Make its existing affordances matter, then let agents decide whether it matters.

Do **not** preserve reservation blockers and queues as parallel live systems. Pick one authoritative contention model and migrate callers to it.

Do **not** solve easy survival by merely increasing need rates. Pressure without richer response modes just produces brittle agents.

Do **not** let need salience become omniscience. A hungry agent may notice food better; she may not know where remote food exists unless she has a lawful information path.

The deepest version of this baseline is not a harsher tamagotchi loop. It is a small local world where hunger, thirst, fatigue, bladder, dirtiness, perception, exploration, and facility contention continuously reshape each other through concrete state.

## Outcome

- Completion date: 2026-05-13
- What changed: Archived as exploited after the proposal's findings had been consumed by follow-on work.
- Deviations from original plan: None.
- Verification results: Archival-only change; no code verification required.
