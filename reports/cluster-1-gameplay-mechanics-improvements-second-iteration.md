# **1. Executive Verdict**

**Cluster 1 is promising but collision-incomplete.** It is no longer thin in the old sense: the current repository has real embodied needs, real acquisition and consumption, real travel costs, real Wash discovery/budget closure, real sleep episode partial recovery, real self-care interruption traces, real Wash/Latrine occupancy, real resource depletion and some item decay, and long-running survival goldens that exercise baseline, scattered, contested, trade, theft, patrol, combat, escort, justice, item decay, and final-integration pressure.

But it is **not yet mature** and definitely not “done enough” for the harsh survival standard in the prompt. The current model proves that agents can usually keep themselves alive under authored pressure. It does **not yet prove** that ordinary degradation lawfully produces unsafe rest, exhaustion collapse, water failure, blocked sanitation, scarcity-driven rationing/hoarding/flight, abandonment, social breakdown, or repeated failed recovery paths that remain legible after the fact.

The prior S172/S173-style work appears landed and should not be re-proposed as missing. The current `main` commit is exactly the intended SHA `299d64c25fb45dc9ab69b295162949d8c8442606`; that merge commit is titled “Implement s173 self care interruption occupancy.” Active code now has `SelfCareOccupancy`, self-care interruption trace detail, Wash/Latrine occupancy release, and queue-system recognition of Wash/Latrine as self-care contention kinds.

The most important remaining problem is **Sleep**. `SleepEpisode` exists, sleep quality exists, and partial recovery exists, but sleep is still mostly “fatigue meter chooses sleep somewhere.” It lacks explicit sleep surfaces, rest-site occupancy, safe-camp creation, night danger, unsafe-rest interruption, exposure, exhaustion collapse, and strong player-facing explanation of why rest succeeded, failed, or only partially recovered.

The second major problem is **harsh-world degradation**. There is real item decay, washbasin refill from concrete water sources, latrine fullness, waste, dirtiness, and resource source depletion, but these are still narrow carriers. They do not yet form a full survival ecology where dirty basins, depleted wells, blocked latrines, spoiled food, unsafe routes, bad sleep, exhaustion, theft, rationing, debt, flight, death, and abandonment emerge routinely from pressure.

**Deepen first:**

1. **Shelter, Sleep Surfaces, and Safe Rest.** This is the highest-leverage next spec theme. Make fatigue recovery situated, scarce, interruptible, occupiable, unsafe, and traceable.  
2. **Concrete Scarcity and Degradation Cascades.** Expand food/water/facility degradation from isolated mechanics into lawful shortage and recovery loops.  
3. **Fatigue Collapse and Harsh Failure Traceability.** Hunger and thirst have deprivation wounds/death paths; fatigue has an `exhaustion_collapse_ticks` profile field but no comparable consequence path in the fetched needs system.  
4. **Player/author legibility for survival failure.** The current forensic surfaces are good, but not sufficient for explaining unsafe rest, depletion, blocked facilities, failed recovery opportunities, and why theft/flight/collapse happened.

**Do not deepen yet:**

Do not add full disease ecology, bathroom shame, complex hygiene etiquette, full weather simulation, predator population dynamics, global settlement health, or a hidden “drama director.” Those are realism traps unless they become concrete consequence carriers. The next iteration should stay ruthlessly focused on **consequence density, local causality, replayable explanation, and symmetric action legality**.

# **2. Evidence Base**

## **Current commit and branch SHA used**

The intended commit was:

`299d64c25fb45dc9ab69b295162949d8c8442606`

I verified through the Git app that current `main` compares identical to that SHA: `ahead_by = 0`, `behind_by = 0`, status `identical`. The fetched commit confirms the merge title and SHA.

## **Uploaded manifest status**

The uploaded manifest was useful as a file inventory, but it is **not reliable as evidence**. It contains paths that exact-SHA targeted fetches could not retrieve, including `specs/S173-self-care-interruption-occupancy.md` and `specs/IMPLEMENTATION-ORDER.md`. The active evidence below therefore comes from targeted exact-SHA fetches, not from uploaded file copies or stale manifest assumptions.

## **Active repo evidence fetched**

Core active docs:

* `docs/FOUNDATIONS.md`  
* `docs/gameplay-mechanic-deepening-roadmap.md`  
* `docs/scenario-roadmap.md`

Active workflows:

* `.github/workflows/golden-survival.yml`  
* `.github/workflows/golden-drive-escalation.yml`  
* `.github/workflows/golden-item-decay.yml`  
* `.github/workflows/golden-simulation-gaps.yml`

Active implementation files fetched:

* `crates/worldwake-core/src/needs.rs`  
* `crates/worldwake-core/src/sleep_episode.rs`  
* `crates/worldwake-core/src/self_care_occupancy.rs`  
* `crates/worldwake-core/src/place_dirtiness.rs`  
* `crates/worldwake-systems/src/needs.rs`  
* `crates/worldwake-systems/src/needs_actions.rs`  
* `crates/worldwake-systems/src/sleep_synthesis.rs`  
* `crates/worldwake-systems/src/travel_actions.rs`  
* `crates/worldwake-systems/src/facility_queue.rs`  
* `crates/worldwake-systems/src/facility_queue_actions.rs`  
* `crates/worldwake-systems/src/item_decay.rs`  
* `crates/worldwake-ai/src/goal_schema.rs`  
* `crates/worldwake-ai/src/candidate_generation.rs`  
* `crates/worldwake-ai/src/pressure.rs`  
* `crates/worldwake-ai/src/interrupts.rs`  
* `crates/worldwake-ai/src/survival_forensics.rs`  
* `crates/worldwake-sim/src/action_trace.rs`  
* `crates/worldwake-sim/src/interrupt_abort.rs`

Active scenarios and tests fetched:

* `scenarios/survival-baseline.ron`  
* `scenarios/survival-contested.ron`  
* `scenarios/survival-trade.ron`  
* `crates/worldwake-ai/tests/scenarios/survival_baseline.rs`  
* `crates/worldwake-ai/tests/scenarios/survival_contested.rs`  
* `crates/worldwake-ai/tests/scenarios/sleep_episode.rs`  
* `crates/worldwake-ai/tests/scenarios/survival_trade.rs`  
* `crates/worldwake-ai/tests/scenarios/survival_items_decay.rs`  
* `crates/worldwake-ai/tests/scenarios/survival_theft.rs`  
* `crates/worldwake-ai/tests/scenarios/simulation_gaps.rs`

Held active specs fetched as candidate prior art only:

* S60 Persistent Site Occupancy  
* S61 Predator Ecology and Dens  
* S62 Boundary Processes and Remote Shocks  
* S63 Contested Evidence and Warrants  
* S64 Scarcity Response — Debt, Rationing, and Substitution  
* S65 Social Aftermath Memory  
* S66 Settlement Decline and Reoccupation

## **External sources consulted**

I used external sources for design criteria, not as a mandate to copy mechanics. The most useful comparisons were:

* **The Long Dark**: official survival-mode description emphasizes Hunger, Thirst, Fatigue, Cold, calorie cost, no hand-holding, hostile wildlife, afflictions, and permadeath.  
* **RimWorld**: official description frames the game as a story generator where needs, wounds, illness, mood, death, darkness, sleeping outside, corpses, weather, and breakdowns create colony drama.  
* **Project Zomboid**: official materials emphasize survival beyond zombie combat: starvation, depression, boredom, infection, crafting, defending, and inevitable death.  
* **Cataclysm: Dark Days Ahead**: official description stresses a persistent harsh world, scavenging food/equipment/vehicles, and other survivors wanting what you have.  
* **GOAP/BDI/HTN design references**: used only for high-level alignment with Worldwake’s belief/intention/planning architecture: explicit preconditions/effects, local beliefs, active intentions, and decomposition into executable tasks.

# **3. Current Cluster 1 Model After Recent Fixes**

## **Foundations constraints that matter most here**

`FOUNDATIONS.md` is extremely clear: no hidden quest logic, no abstract scarcity score, no global truth leaks, no planner intent as reservation, no invisible rescue, no decorative realism, no player/AI split in simulation law. Important actions require preconditions, duration, cost, occupancy, interruption, and aftermath; failure must leave state; boundary pressures must enter through explicit boundary processes; player UI may only reveal what the controlled agent can lawfully know.

Cluster 1 therefore cannot be considered complete just because hunger, thirst, fatigue, bladder, and dirtiness values exist. A full mechanic must create **lawful world-state pressure** that other systems can observe, interrupt, exploit, repair, misinterpret, or remember.

## **Existing authoritative state**

The current homeostatic state is five needs: hunger, thirst, fatigue, bladder, and dirtiness. The profile layer includes rates, rest efficiency, sleep durations, wash/toilet durations, starvation tolerance, dehydration tolerance, exhaustion collapse ticks, bladder accident tolerance, travel multipliers, and wilderness relief dirtiness penalty.

The fetched needs tick system advances needs, applies body costs, tracks deprivation exposure, applies drive escalation, produces starvation/dehydration deprivation wounds, handles bladder accidents, and kills agents when deprivation wounds become fatal.

That is a strong substrate. The notable hole is fatigue: `exhaustion_collapse_ticks` exists in profile state, but the fetched needs system applies concrete deprivation consequences for starvation, dehydration, and bladder accidents, not an equivalent exhaustion collapse path.

## **Existing action families**

The active Needs action registry includes `eat`, `drink`, `sleep`, `toilet`, `wash`, and `relieve_wilderness`. Eat and drink consume controlled lots and reduce hunger/thirst. Toilet resets bladder and creates waste/latrine fullness. Wilderness relief creates waste, evidence, dirtiness, and resets bladder. Wash consumes clean basin water, reduces dirtiness, dirties the basin, and can produce partial relief if water is insufficient. Sleep creates a `SleepEpisode` and applies tick-by-tick fatigue recovery.

## **What S172/S173-style work appears to have fixed**

The current repo has `SelfCareOccupancy` with occupant, use kind, started tick, and goal key. The enum contains Wash, LatrineRelief, Eat, Drink, WildernessRelief, and Sleep, but the current action code writes occupancy only where there is a concrete target to occupy: Wash basin and Latrine place.

The queue system now recognizes self-care Wash and Latrine as promotable contention kinds. It matches Wash against a `WashBasin` workstation facility and Latrine against a `PlaceTag::Latrine`.

The action trace has `ActionTraceDetail::SelfCareInterrupted`, explicitly stating that the authoritative causal record remains `EventTag::ActionAborted` while the trace detail carries the self-care family and optional facility/place target. The interrupt/abort system writes `ActionInterrupted` or `ActionAborted` through normal action termination.

Those are real improvements. They should be treated as landed.

## **Existing sleep model**

Sleep is materially better than a stub. `SleepEpisode` stores place, min/max ticks, target recovery, accumulated recovery, recovery modifier, and wake conditions. `SleepQualityProfile` includes shelter tag, ground comfort, and recovery modifier. Wake conditions can include intended duration, target recovery, projected non-fatigue need breach, scheduled commitment, and local disturbance.

The regular sleep golden proves lifecycle, projected hunger wake, place-quality recovery modifier, partial recovery after interruption, and choosing a higher-quality known sleep place.

However, sleep is still collision-incomplete. The active goal schema marks Sleep feasibility as `AlwaysLikely`, with only `Sleep` as a relevant op. That is a tell: the planner currently treats sleep as broadly feasible, rather than as a situated chain involving a known surface, safe camp, shelter occupancy, route safety, night danger, exposure, or local disturbance risk.

## **Existing degradation model**

Cluster 1 already has important concrete degradation state:

* `LatrineFullness`: fill, fill per use, critical threshold.  
* `PlaceDirtiness`: value, decay, dirtiness per use.  
* `WashBasinState`: clean water, max water, refill rate, full-wash units, dirtiness level, dirtiness per use.

The item decay system archives ground item lots after commodity-specific decay, decays place dirtiness, and refills wash basins by transferring water from a co-located water source into the basin. That last point is excellent: basin refill is a concrete source/sink process, not a magic meter.

But this degradation layer is still too narrow. Basin dirtiness does not yet appear to matter much downstream. Latrine fullness creates dirtiness after a threshold but does not block use, force emptying, contaminate nearby water, or produce sustained local failure. Food decay proof is narrow and centered on a carried Waste lot dropped to the ground and later archived, not on stored/cached food spoilage causing scarcity.

## **Existing scenario proof**

The proof stack is much stronger than normal unit coverage.

`survival-baseline` runs 1440 ticks, requires Eat/Drink/Sleep/Relieve/Wash, checks survival, critical run bounds, self-care action families, explorer discovery, budget exhaustion, and stuck idle windows.

`survival-contested` runs 1440 ticks with four agents, tight resources, two water sources, chokepoint topology, and Wash co-located with one well. It proves all agents survive, all perform self-care, both water sources are used, both camp sides reach food, Wash payloads appear, no survival budget exhaustion occurs, and no stuck elevated-need idle windows occur.

`survival-trade` proves substitute trade and a real facility queue/grant branch at a Market Square well.

`survival-theft` proves a hungry thief with no coin/no harvest fallback can steal a staged apple lot, eat afterward, and leave evidence/testimony consequences.

`simulation_gaps` includes starvation death traceability: hunger deprivation death, death event, post-death AI `DecisionOutcome::Dead`, and no post-death started actions.

That is not thin. But the roadmap itself distinguishes landed rows from collision-proven maturity and flags remaining gaps around severe degradation, multi-agent rest/relief contention, injury/pursuit disruption, and shelter/safety constraints around sleep.

# **4. Research Synthesis: What Makes Harsh Survival Compelling**

The prompt’s strong opinion is correct:

A survival mechanic becomes compelling when bodily vulnerability forces situated prioritization under imperfect knowledge and degrading conditions, and when failure produces world-state consequences that other systems can see and react to. It becomes busywork when it is only a timer that demands periodic maintenance.

The external comparisons sharpen that into Worldwake-specific criteria.

**Good survival friction joins body, time, place, and uncertainty.** The Long Dark’s official survival-mode framing is useful because it does not present Hunger, Thirst, Fatigue, and Cold as isolated refill bars. They are bound to calories, route choice, weather, hostile wildlife, afflictions, scarce time, no hand-holding, and permadeath. Worldwake should take the pattern, not the exact mechanics: needs become interesting when meeting one need worsens another risk, consumes time, requires travel, depends on unreliable memory, or exposes the agent to danger.

**The best colony/survival systems make failure socially legible.** RimWorld’s official description emphasizes that hunger, fatigue, wounds, illness, darkness, sleeping outside, death, unburied corpses, and other conditions become mood and story pressure, not just private meter changes. Worldwake should not import a mood system just to copy RimWorld, but it should adopt the core principle: bodily failure should leave public or semi-public carriers that other systems can respond to.

**Harsh worlds need ordinary bad outcomes, not cinematic exceptions.** Project Zomboid’s official framing leans into starvation, depression, infection, crafting, defense, and inevitable death rather than heroic rescue. Worldwake’s equivalent should be sustained local causality: if the well fails, the latrine blocks, the merchant hoards, the road becomes unsafe, and the only shelter is occupied, the collapse should follow from ordinary action legality and failed recovery opportunities.

**Long-horizon survival is about recovery routes, not just punishment.** Cataclysm: Dark Days Ahead’s official description stresses scavenging, food, equipment, vehicles, other survivors, and securing long-term food/water/safety. The important criterion for Worldwake is that harsh survival should not be a linear death spiral unless the world state really leaves no lawful recovery. Agents should be able to switch from eat → acquire → trade → steal → ration → flee → collapse through lawful plans and beliefs.

**Planning architecture matters.** GOAP-style systems rely on preconditions/effects and replanning from sensed world state; BDI separates beliefs, desires, and active intentions; HTN decomposes high-level tasks into executable primitives. Worldwake already has the right foundation: belief-backed candidate generation, intention frames, action preconditions, duration, interruption, and traces. Future Cluster 1 deepening should not add hidden evaluators. It should add **new consequence carriers** that the existing planner can discover, reason over, and fail against.

## **Good friction versus chore meter**

A **chore meter** says:

* Hunger high → eat.  
* Fatigue high → sleep.  
* Dirtiness high → wash.  
* Repeat every N ticks.

**Good friction** says:

* Hunger is high, but the known food source is depleted.  
* The alternate food source is beyond a route that worsens fatigue/thirst.  
* The only safe sleep site is occupied.  
* Sleeping rough is legal but likely to recover poorly or be interrupted.  
* The merchant has food but is hoarding or rationing.  
* Stealing food is possible but leaves evidence and social aftermath.  
* Fleeing the settlement avoids one failure but abandons obligations, offices, facilities, or kin.  
* Death or collapse is not a boolean; it is a causal chain visible through failed recovery attempts.

Worldwake should aim for the second form.

# **5. Revised Definition of a Full Cluster 1 Mechanic**

After S172/S173, a full Cluster 1 mechanic should meet this standard:

A Cluster 1 mechanic is full when it creates **situated bodily vulnerability** that competes with time, place, safety, social duty, scarcity, and other agents through lawful world state, and when both recovery and failure leave traceable downstream consequences.

For each self-care family, “full” means:

1. **Concrete authoritative state.** The need, affordance, resource, facility, wound, waste, rest site, or degradation carrier has identity where downstream systems care.  
2. **Belief-backed planning.** Planner-visible inputs come from self-knowledge, direct local observation, remembered beliefs, records, testimony, or lawful same-tick local observation—not global truth.  
3. **Preconditions and acquisition chain.** The action is not just available because a meter is high. It has lawful location, resource, control, safety, occupancy, or fallback conditions.  
4. **Duration and costs.** It takes time and can worsen other needs or risks.  
5. **Occupancy/contention where shared.** Wells, basins, latrines, beds, shelters, camps, fires, and guarded rest places cannot be silently shared by planner intent.  
6. **Interruption and recovery.** Interruptions have causes, cleanup, partial progress where meaningful, and replan paths.  
7. **Partial failure.** Poor rest, partial wash, dirty water, blocked latrine, spoiled food, unsafe camp, or failed trade creates intermediate state, not instant success/fail.  
8. **Degradation and shortage.** Use, neglect, travel, weather, theft, crowding, or time can make affordances worse through concrete state.  
9. **Harsh outcomes.** Sustained failure can cause wounds, collapse, death, theft, rationing, hoarding, flight, abandonment, vacancy, or social memory where warranted.  
10. **Traceability.** A future reader can reconstruct why the agent survived, stole, fled, collapsed, or died through event logs, action traces, decision traces, beliefs, and failed opportunities.  
11. **Deterministic replay.** Randomness, if used, is seeded and grounded in local/boundary processes.  
12. **Player/AI symmetry.** The player may see a different presentation, but not a different reality.

The current Cluster 1 meets parts of this standard for Eat/Drink/Wash/Relieve. It does not yet meet it for Sleep, exposure, severe degradation, harsh scarcity, or collapse/social-breakdown cascades.

# **6. Gap Analysis**

## **6.1 Remaining self-care completeness gaps**

### **Eat**

Eat is relatively mature as an action family. It consumes controlled food lots, reduces hunger, applies bladder fill from consumables, has duration, action traces, and is scenario-proven across baseline, trade, theft, and item-decay runs.

The remaining gap is not “can agents eat?” It is **what happens when food is bad, unavailable, hoarded, rationed, stolen, or spoiled**. Current proof includes substitute trade and survival theft, but not broad scarcity responses or long-term food degradation. The item-decay proof is exact-lot and concrete, but it focuses on waste decay rather than food spoilage causing survival pressure.

Verdict: **action mature; scarcity/degradation incomplete.**

### **Drink**

Drink is also fairly mature in the basic loop. The contested scenario proves both water sources are used under multi-agent pressure, and the trade scenario proves a real queue/grant branch for water harvesting at a well.

The missing pieces are **dirty water, unreliable wells, depleted sources, source trust, and water conflict**. The current system can represent ResourceSource depletion and basin refill from water, but it does not yet appear to model water quality, contamination, well failure, drought-like reduction, or survival-driven rationing/monopolization as regular consequences.

Verdict: **basic loop mature; harsh water ecology incomplete.**

### **Sleep**

Sleep is the weakest major family. It has a strong episode carrier and regular goldens, but it lacks the world-process richness that would make fatigue dramatic.

Current sleep has:

* `SleepEpisode`  
* partial recovery  
* `SleepQualityProfile`  
* projected need breach wake  
* commitment deadline wake  
* local disturbance wake  
* higher-quality place preference

Current sleep lacks:

* explicit sleep surface identity  
* bed/camp/shelter occupancy  
* safe/unsafe rest places  
* camp creation  
* night danger  
* environmental exposure  
* pain/wound sleep disruption  
* guard/social duty rest interruptions beyond scheduled commitment  
* exhaustion collapse  
* rest-site memory beyond quality preference  
* “why did this sleep fail?” player-facing explanation

The current goal schema’s `AlwaysLikely` sleep feasibility is the clearest architectural smell: Sleep is not yet a situated affordance in the same sense Wash is.

Verdict: **implementation exists; gameplay mechanic is collision-incomplete.**

### **Toilet / Latrine relief**

Toilet is significantly improved by S173. It has a place tag, duration, occupancy, waste creation, latrine fullness, and commit/abort cleanup.

The gaps are downstream degradation and proof. A full latrine mechanic should let sustained use produce blocked, filthy, unsafe, or avoided sanitation, with cleaning/emptying as concrete recovery. Current fullness creates dirtiness after threshold, but does not yet seem to force a blocked-latrine decision, create maintenance pressure, or make multi-agent contention a core scenario proof.

Verdict: **mechanically grounded; degradation and collision proof incomplete.**

### **Wilderness relief**

Wilderness relief is lawful and concrete: it creates waste, evidence, dirtiness, and resets bladder.

It is currently a useful fallback, not a rich mechanic. That is acceptable. It should remain simple unless sanitation/exposure/site-trace systems need it as a consequence carrier. The obvious future deepening is not etiquette or shame; it is **persistent waste and site degradation** where repeated wilderness relief near a camp makes the camp less safe, less comfortable, or more discoverable.

Verdict: **adequate fallback; can become site-degradation input later.**

### **Wash**

Wash has improved substantially. It requires a WashBasin facility, clean water, occupancy, duration, partial wash when water is insufficient, basin dirtiness, clean-water consumption, and event payloads. Basins refill from co-located water sources through a concrete transfer.

The remaining gap is that basin dirtiness and water-source degradation are not yet harsh enough. A basin can become dirty, but the fetched evidence does not show that dirty basin state forces cleaning, reduces recovery, spreads downstream consequences, or creates failed Wash recovery paths beyond clean water shortage.

Verdict: **promising; needs degradation consequences and collision scenarios.**

### **Travel for self-care**

Travel is real and costly. It uses topology, duration, in-transit state, body-cost override from metabolism travel multipliers, evidence, and route experience.

The current gap is that self-care travel does not yet fully interact with shelter safety, exposure, route degradation, or hostile rest interruptions. The route can become “experienced as hostile” through combat, but there is not yet a minimal exposure/safe-camp route model.

Verdict: **strong substrate; needs survival-environment coupling.**

### **Discovery/acquisition of self-care affordances**

This is one of the stronger parts of Cluster 1. The baseline/scattered/contested tests prove discovery and remote resource behavior, while the contested Wash regression proves no Wash plan for an unseen remote basin under local-only belief.

The missing discovery domain is rest: safe shelters, sleep surfaces, camps, fire sites, dry sites, hostile routes, and remembered unsafe sites.

Verdict: **good for food/water/Wash; incomplete for rest/shelter.**

### **Self-care under facility contention**

The facility contention substrate is real. The trade scenario proves queue/grant behavior at a well, and the queue system recognizes self-care Wash/Latrine contention kinds.

However, the proof is not yet collision-proven for Wash/Latrine contention as a gameplay pattern, and there is no equivalent rest-site/sleep-surface contention.

Verdict: **substrate landed; Cluster 1 contention proof incomplete.**

### **Self-care under interruption by adjacent pressure**

The generic interrupt system and self-care trace details are strong. Critical survival and danger can trigger interrupts, and action traces now distinguish self-care interruption.

But the world needs more ordinary causes of interruption: unsafe camp, night danger, exposure, hostile proximity, pain, social obligation, occupied shelter, blocked facility, or route failure.

Verdict: **interruption plumbing improved; interruption ecology incomplete.**

### **Self-care under sustained degradation and scarcity**

This is the largest systemic gap after sleep. The repository can prove survival under pressure and specific scarcity branches, but not sustained harsh-world collapse. The current tests mostly show agents surviving within bounds. They do not yet prove that ordinary world degradation regularly yields rationing, hoarding, theft, flight, abandonment, repeated collapse, or death through broad lawful processes.

Verdict: **thin for harsh-world collapse.**

## **6.2 Sleep, shelter, and safe rest**

Cluster 1 now needs explicit shelter/safe-rest mechanics. This is not optional if the target is “rich, harsh, systemic survival.”

Sleep is not complete merely because `SleepEpisode` exists. Sleep needs to answer:

* Where can I rest?  
* Is that rest site known?  
* Is it sheltered?  
* Is it occupied?  
* Is it safe enough?  
* What will interrupt me?  
* How much recovery did I actually get?  
* Why was recovery poor?  
* What did I do after waking or being forced awake?

Current `SleepQualityProfile` is a good seed, not a full mechanic. It should become one input into a larger **Rest Site** or **Sleep Surface** model.

## **6.3 Exposure and environment**

Exposure should be introduced, but minimally and only after safe-rest has a concrete carrier.

The minimal model should cover cold/heat/wetness as **local or boundary-origin pressure**, not a full weather sim. Exposure should interact with:

* shelter  
* sleep quality  
* travel route choice  
* fatigue  
* thirst  
* wounds  
* camp/fire/clothing mitigation

The model should not simulate weather for realism. It should exist because “sleeping wet in the open during cold” is one of the cleanest ways to turn rest from a meter reset into a situated survival decision.

## **6.4 Scarcity and degradation**

The current model has concrete seeds but lacks harsh cascade depth.

Needed concrete degradation carriers:

* food freshness/spoilage where food storage matters  
* water source quantity and quality  
* well reliability and recharge  
* washbasin dirtiness consequences  
* latrine blocked/overfull state  
* shelter/sleep site wear/crowding  
* route danger/degradation  
* facility closure/vacancy  
* ration orders, debts, hoards, refusals  
* departure/flight and abandoned sites

The held specs S60–S66 are relevant as prior art, but they should not be imported wholesale into Cluster 1. S64 and S66 are especially important later support mechanics, while S60 provides useful shelter/camp/site identity concepts.

## **6.5 Collapse, death, and social breakdown traceability**

Current death proof is useful but too narrow. The starvation traceability test proves hunger-deprivation death, death event, post-death AI outcome, and no post-death actions.

That is not enough for rich harsh survival. Cluster 1 should eventually prove:

* starvation after failed food acquisition/trade/theft attempts  
* dehydration after failed water acquisition and dirty/depleted source decisions  
* exhaustion collapse after repeated unsafe/occupied/interrupted sleep  
* exposure wounds/death after failed shelter/camp/fire decisions  
* collapse under wound/pursuit/rest conflict  
* theft/rationing/hoarding from concrete scarcity  
* flight/departure after repeated failed recovery  
* abandoned facilities after death/flight  
* social aftermath from theft, death, refusal, or rationing

## **6.6 Player-facing legibility**

The debug/author side is relatively strong. `SurvivalForensicExtractor` captures critical windows, selected goals, competitors, active actions, exhaustion/blockers, and local survival state.

But future player-facing legibility needs a clean POV boundary:

* The player can see what the controlled agent perceives: “this bed is occupied,” “this camp is exposed,” “the basin looks dirty,” “the well seems dry,” “you remember this route had an attack.”  
* The player should not see hidden truth: “remote well has 3 water units,” “camp will be attacked tonight,” “merchant is hoarding X unless discovered.”  
* Author/debug views can be omniscient, but must be explicitly separate.

## **6.7 Adjacent-cluster collision seams**

Cluster 1 needs narrow changes in adjacent systems, not redesigns:

* **Travel:** route safety/exposure/rest-site discovery.  
* **Trade:** scarcity substitution, rationing, debt, hoarding/refusal.  
* **Theft:** survival-driven theft already exists; broaden to water, shelter, medicine, food hoards.  
* **Combat/patrol/escort:** danger interrupts rest and self-care.  
* **Justice/evidence:** theft/rationing/abandonment leave evidence and records.  
* **Obligations:** commitments can interrupt sleep; collapse can cause missed obligations.  
* **Institutions:** ration orders and facility closures later.

# **7. Broad Menu of Future Cluster 1 Deepening Directions**

## **P0 — Safe Rest, Sleep Surfaces, and Shelter Occupancy**

**Gameplay purpose:** Make fatigue recovery a situated survival decision, not a universal meter reset.

**FOUNDATIONS rationale:** Adds concrete state, occupancy, interruption, partial failure, and traceable aftermath for one of the weakest current mechanics. Aligns with P3, P8, P10, P19, P29.

**Likely state carriers:**

* `SleepSurface` or `RestSiteAffordance`  
* `RestSiteOccupancy` or reuse `SelfCareOccupancy::Sleep`  
* `CampState`  
* rest-safety observable traces  
* extended `SleepEpisodeStarted/Ended` payloads

**Systems touched:** core sleep/rest state, needs actions, sleep synthesis, candidate generation, goal schema, survival forensics, scenario harness, visualizer/CLI later.

**Scenario proof shape:** one shelter/surface, multiple tired agents, unsafe fallback, partial recovery, interruption, deterministic replay, trace reason.

**Risks:** accidental abstract “safety score”; making sleep impossible too often; hidden omniscient safety evaluation.

**Cluster classification:** Cluster 1 proper.

## **P0 — Fatigue Collapse and Poor-Rest Consequences**

**Gameplay purpose:** Make repeated failed rest dangerous.

**FOUNDATIONS rationale:** Existing `exhaustion_collapse_ticks` profile implies a consequence path, but the fetched needs system does not apply it.

**Likely state carriers:**

* exhaustion exposure counter already conceptually present through `DeprivationExposure`  
* collapse/incapacitation wound or condition  
* event payload for exhaustion collapse

**Systems touched:** needs tick, wounds/death, decision/trace surfaces, scenarios.

**Scenario proof shape:** repeated unsafe/occupied/interrupted sleep → fatigue critical run → collapse → failed recovery trace → possible death/no-post-death-action proof.

**Risks:** death spiral too fast; collapse needs recoverability if others can help.

**Cluster classification:** Cluster 1 proper.

## **P0 — Concrete Survival Degradation: Water, Latrine, Basin, Food**

**Gameplay purpose:** Turn “resources exist” into “resources can fail, dirty, block, spoil, refill, recover, or cause conflict.”

**FOUNDATIONS rationale:** The roadmap explicitly calls out severe degradation of food/water/sleep/latrine/wash as not yet proven enough.

**Likely state carriers:**

* water source quality/reliability  
* food freshness/spoilage  
* blocked latrine state  
* basin cleanliness consequence  
* cleaning/refill/repair actions  
* source reliability memory

**Systems touched:** item decay, resource sources, needs actions, production/trade, candidate generation, survival forensics.

**Scenario proof shape:** source depletes/dirty basin/blocked latrine/spoiled food → agents replan through recovery/substitution/theft/rough fallback → trace proves exact failure.

**Risks:** overcomplicated sanitation; hidden “dirty means sick” jump without carrier.

**Cluster classification:** Cluster 1 proper plus support mechanics.

## **P0 — Survival Failure Forensics and Player-Legible Causal Reports**

**Gameplay purpose:** Make “why did this agent die/steal/flee/collapse?” answerable.

**FOUNDATIONS rationale:** P29/P29A require debuggability and causal history.

**Likely state carriers:**

* expanded critical-window frames  
* failed recovery opportunity records  
* sleep/rest failure reason  
* source/facility failure classifications  
* player POV filtered explanation

**Systems touched:** survival forensics, action trace, decision trace, CLI/visualizer.

**Scenario proof shape:** scenario asserts not merely final outcome, but causal chain: known source depleted, route unsafe, rest surface occupied, action aborted, recovery failed.

**Risks:** trace bloat; leaking omniscient info to player.

**Cluster classification:** Cluster 1 support.

## **P1 — Minimal Environmental Exposure**

**Gameplay purpose:** Make shelter, route choice, camp, clothing/fire, and sleep safety matter.

**FOUNDATIONS rationale:** Exposure is justified if it is concrete local/boundary pressure with mitigation carriers, not weather as ambience.

**Likely state carriers:**

* `ExposureState` on agents  
* `ExposureSource` on places/edges or boundary/weather event  
* shelter/camp/fire/clothing mitigation  
* exposure wound causes

**Systems touched:** needs, travel, sleep, topology, scenario loader, candidate generation, forensics.

**Scenario proof shape:** cold/wet route + open camp + shelter alternative → poor rest/exposure wound unless mitigated.

**Risks:** sprawling weather simulation; abstract climate score.

**Cluster classification:** Cluster 1 support mechanic, possibly new roadmap seam.

## **P1 — Rest-Site Memory and Safe-Route Preference**

**Gameplay purpose:** Let agents learn that some places/routes are good or bad for recovery.

**FOUNDATIONS rationale:** Supports belief/locality and replayable explanation without omniscience.

**Likely state carriers:**

* rest experience memory  
* route experience already exists for hostile encounters; extend to exposure/rest outcomes  
* testimony/records for safe camps

**Systems touched:** travel experience, sleep end events, candidate ranking.

**Scenario proof shape:** agent avoids a previously interrupted/exposed camp and chooses a worse-but-safer shelter.

**Risks:** overfitting ranking; too much hidden scoring.

**Cluster classification:** Cluster 1 support.

## **P1 — Multi-Agent Contention for All Survival Affordances**

**Gameplay purpose:** Make survival social: shared wells, latrines, basins, beds, shelters, fires, and food stock become conflict surfaces.

**FOUNDATIONS rationale:** P8 demands explicit contention for contested affordances.

**Likely state carriers:** existing contention queues, self-care occupancy, new sleep/rest occupancy.

**Systems touched:** facility queue, needs actions, candidate generation, scenario tests.

**Scenario proof shape:** several agents compete for one latrine, one basin, one shelter surface; proves wait/replan/rough fallback.

**Risks:** queue deadlocks; too much waiting.

**Cluster classification:** Cluster 1 proper.

## **P2 — Scarcity Response: Rationing, Debt, Hoarding, Refusal**

**Gameplay purpose:** Make social survival pressure arise from concrete stockouts.

**FOUNDATIONS rationale:** S64 is strong prior art: scarcity responses should emerge from stock depletion, not a scarcity event.

**Likely state carriers:** debt records, ration orders, hoards, refusal decisions, priority lists.

**Systems touched:** trade, social artifacts, institutions, AI candidate generation.

**Scenario proof shape:** depleted food/water → ration order or hoarding → one agent fails lawful purchase → borrows/steals/flees.

**Risks:** too institutional too soon; should follow concrete depletion first.

**Cluster classification:** support mechanic / adjacent cluster seam.

## **P2 — Social Aftermath of Survival Failure**

**Gameplay purpose:** Let theft, refusal, abandonment, rescue, and death matter socially.

**FOUNDATIONS rationale:** S65’s provenance-tracked social memory is relevant once survival failure creates concrete events.

**Likely state carriers:** grudges, gratitude, obligations, debt links.

**Systems touched:** social memory, theft, justice, trade, care, escort.

**Scenario proof shape:** survival theft creates grudge; rescue creates gratitude; ration refusal creates social edge.

**Risks:** adding emotion-like abstraction instead of event-proven social memory.

**Cluster classification:** adjacent support.

## **P2 — Facility Closure, Flight, Vacancy, Reoccupation**

**Gameplay purpose:** Make sustained survival failure change settlements.

**FOUNDATIONS rationale:** S66 is strong prior art: decline should emerge from individual departure, facility closure, office vacancy, and reoccupation, not a settlement health bar.

**Likely state carriers:** facility vacancy, departure reason, abandoned goods, occupancy claim.

**Systems touched:** trade, offices, travel, site occupancy, social memory.

**Scenario proof shape:** repeated shortage → merchant closes/flees → facility vacant → stockout worsens → squatter/scavenger reoccupies.

**Risks:** too large for current Cluster 1 unless scoped as validation seam.

**Cluster classification:** adjacent cluster with Cluster 1 validation seam.

## **P2 — Persistent Camps and Sites**

**Gameplay purpose:** Let shelters and camps persist, degrade, become occupied, abandoned, discovered, or contested.

**FOUNDATIONS rationale:** S60 offers site occupancy and trace prior art.

**Likely state carriers:** site profile, occupancy claim, site traces, camp state.

**Systems touched:** topology, shelter, travel, perception, evidence.

**Scenario proof shape:** camp created, used, dirtied, abandoned, later discovered.

**Risks:** scope creep into dungeons/interiors.

**Cluster classification:** support mechanic.

## **P3 — Predator/Night Danger Ecology**

**Gameplay purpose:** Give unsafe rest and travel a nonhuman danger source.

**FOUNDATIONS rationale:** S61 is aligned because predators are normal agents with hunger/territory, not encounter spawns.

**Likely state carriers:** predator profile, den, tracks, carcasses.

**Systems touched:** combat, perception, travel, rest interruption.

**Scenario proof shape:** predator signs near camp → agent chooses shelter or is interrupted.

**Risks:** too much before safe-rest substrate exists.

**Cluster classification:** later adjacent support.

## **P3 — Boundary Shocks**

**Gameplay purpose:** Introduce failed shipments, refugee pressure, drought-like inflow failure, and external disruptions lawfully.

**FOUNDATIONS rationale:** S62 directly matches boundary-process rules and forbids hidden spawners/drama dials.

**Likely state carriers:** source regions, boundary channels, scheduled inflows, disruption events.

**Systems touched:** trade, production, records, perception, institutions.

**Scenario proof shape:** expected shipment fails → local stockout → rationing/theft/flight.

**Risks:** too external before internal scarcity is robust.

**Cluster classification:** support/adjacent, later.

# **8. Highest-Leverage Next Proposal Candidates**

## **Candidate 1 — Shelter, Sleep Surfaces, and Safe-Rest Consequence Carrier**

**Recommendation:** first.

**One spec or multiple:** one focused spec, but with strict boundaries. It should include rest-site/surface identity, occupancy, poor-rest recovery, explicit wake/failure reasons, and scenario proof. It should not include full exposure, predators, or settlement decline except as stubbed future inputs.

**Dependencies:** current `SleepEpisode`, `SleepQualityProfile`, `SelfCareOccupancy`, action trace, survival forensics.

**Why first:** Sleep is the largest false-positive maturity area. It touches fatigue, shelter, safety, interruption, and future exposure. Until this exists, Cluster 1 cannot plausibly be called embodied survival.

## **Candidate 2 — Fatigue Collapse and Failed-Rest Traceability**

**Recommendation:** pair with Candidate 1 or immediately follow it.

**One spec or multiple:** could be a second small spec if Candidate 1 is already large.

**Dependencies:** safe-rest interruption proof, needs deprivation exposure, wound/death system.

**Why:** fatigue currently has recovery but weak consequence. A harsh survival cluster needs exhaustion to be dangerous.

## **Candidate 3 — Concrete Survival Degradation: Water, Latrine, Basin, Food**

**Recommendation:** second major theme.

**One spec or multiple:** multiple specs are cleaner: Water/Drink degradation; Sanitation/Wash/Latrine degradation; Food spoilage/storage.

**Dependencies:** existing `ResourceSource`, `WashBasinState`, `LatrineFullness`, item decay, place dirtiness.

**Why:** without concrete degradation, survival remains maintenance loops plus isolated authored scarcity branches.

## **Candidate 4 — Harsh Failure Cascades and Survival Response**

**Recommendation:** after concrete degradation.

**One spec or multiple:** multiple support specs. Start with narrow survival-driven hoarding/ration/refusal or theft expansion, then flight/abandonment later.

**Dependencies:** degradation, trade, theft, S64/S66 prior art.

**Why:** collapse/social breakdown must arise from concrete shortage, not abstract panic.

## **Candidate 5 — Minimal Exposure and Shelter Mitigation**

**Recommendation:** design now, implement after rest substrate.

**One spec or multiple:** one support spec.

**Dependencies:** safe-rest surface/shelter, route/travel, wound system.

**Why:** exposure is the cleanest way to make shelter matter, but it will sprawl if introduced before shelter/rest carriers exist.

# **9. Shelter, Sleep, and Safe-Rest Proposal**

## **Requirements**

Sleep must stop being merely “fatigue high → sleep somewhere.” A future spec should require:

* **Known rest affordance:** sleeping well requires a place, surface, or camp the agent knows or directly observes.  
* **Emergency fallback:** sleeping rough remains legal in many places, but it is lower quality, more interruptible, and more exposure-prone.  
* **Explicit occupancy:** scarce surfaces, shelters, fires, and camps must have occupancy/capacity state.  
* **Safety is concrete:** unsafe rest comes from local hostile presence, route experience, exposure source, lack of cover, noise/disturbance, visible tracks/evidence, or known prior interruption—not a hidden safety score.  
* **Partial recovery is preserved:** current `SleepEpisode` partial recovery should remain the model.  
* **Poor rest is explainable:** sleep end events and traces must explain why recovery was low or interrupted.  
* **Player/AI symmetry:** the same rest legality applies to human and AI agents.

## **Mechanics**

The minimal model should split sleep into three lawful paths:

1. **Sleep at known rest site.** Best recovery, may require surface/camp/shelter availability.  
2. **Create or use camp.** Medium recovery, may require time/material/fire/shelter conditions if those carriers exist.  
3. **Sleep rough.** Always or broadly available if the actor is alive and not in transit, but poor recovery and greater interruption/exposure risk.

This avoids over-hard-gating sleep while making good sleep scarce and meaningful.

## **Potential components/state types**

Candidate state carriers:

* `RestSiteAffordance`  
  * entity/place identity  
  * surface kind  
  * capacity  
  * shelter tag  
  * comfort modifier  
  * exposure protection  
  * maintenance/dirtiness/wear  
* `SleepSurfaceOccupancy`  
  * occupant  
  * started tick  
  * intended use  
  * optional goal key  
  * capacity slot  
* `CampState`  
  * creator/claimant  
  * created tick  
  * shelter quality  
  * fire/protection if later added  
  * current occupants  
  * wear/dirtiness  
* `RestExperienceMemory`  
  * agent’s remembered rest outcome at place/surface  
  * interruption reason  
  * recovery quality  
  * last used tick

`SelfCareOccupancy::Sleep` already exists as an enum case, but current fetched action code does not write occupancy for Sleep. A future spec should decide whether to reuse `SelfCareOccupancy` for sleep surfaces or create a more specific rest occupancy component. Reuse is attractive for symmetry; a specific component may be cleaner if capacity slots matter.

## **Planner and belief implications**

Sleep candidates should be generated from:

* self fatigue and projected need breaches  
* current place direct observation  
* known rest sites and surfaces  
* remembered rest outcomes  
* known shelter quality  
* known occupancy or local observation  
* known danger/exposure signs  
* scheduled commitments

Sleep should not use global safety truth. The planner can rank known rest options using belief-backed properties. Unknown danger remains unknown.

The existing `NeedWithFacilities(Fatigue)` invalidation strategy is a useful hook, but `FeasibilityStrategy::AlwaysLikely` is too permissive for high-quality sleep. It can remain only for the “sleep rough” fallback.

## **Occupancy/contention implications**

Sleep surfaces should be contested like basins and latrines when they are scarce. A shelter with two bedrolls cannot silently sleep five agents because all intend to use it. Occupancy must be written at action start and cleared on commit/abort/death/incapacitation.

Queueing should be optional. Sometimes the correct behavior is to wait for a bed. Sometimes it is to sleep rough because fatigue is critical. That choice should emerge from utility, need pressure, safety, and time.

## **Interruption/recovery implications**

Sleep interruption reasons should be structured:

* projected hunger/thirst/bladder/dirtiness breach  
* scheduled commitment  
* local disturbance  
* hostile proximity or attack  
* exposure breach  
* surface/camp invalidated  
* actor wounded/incapacitated  
* shelter became unsafe/occupied through lawful state change

`WakeReason::LocalDisturbance` is too coarse for future player-facing explanation. Keep it for compatibility if necessary, but extend event payloads or trace detail with a more specific cause.

## **Trace/event implications**

Sleep traces should answer:

* Where did the agent sleep?  
* What surface/camp/shelter was used?  
* What did the agent believe about it?  
* What recovery modifier was applied and why?  
* What interrupted sleep?  
* What recovery was accumulated?  
* What did the agent choose next?

Author/debug view can expose full causal state. Player POV should expose only observed or inferred facts.

## **Scenario/golden proof**

Add scenario-backed proof, not just unit goldens:

* **Safe rest contention:** two or three fatigued agents, one roofed surface, one rough camp. Prove one agent occupies shelter, another waits or sleeps rough, recovery differs, and traces explain why.  
* **Interrupted unsafe sleep:** an agent sleeps rough in an unsafe place and is interrupted by a concrete local disturbance or hostile. Prove partial recovery and replan.  
* **Sleep under need projection:** combine existing projected hunger wake with rest-site choice so an agent chooses shorter/nearer rest because hunger will breach.  
* **Sleep after injury/pursuit:** a wounded or pursued agent attempts rest; danger interrupts or reprioritizes.  
* **Deterministic replay:** repeat safe-rest scenario and assert identical outcome.

## **Risks and rejected alternatives**

Rejected:

* A hidden `safe_to_sleep` boolean.  
* A global night danger roll.  
* Hotel/bed ownership or etiquette as a first pass.  
* Disease, shame, or privacy mechanics.  
* Full weather simulation before minimal exposure carriers exist.

Accepted risk:

* Sleep may become too constrained. The emergency rough-sleep fallback prevents total planner deadlock.

# **10. Scarcity, Degradation, and Harsh-World Failure Proposal**

## **Requirements**

Cluster 1 needs a degradation model that produces ordinary bad outcomes through concrete state:

* food can become unavailable, spoiled, hoarded, rationed, stolen, or abandoned  
* water can deplete, fail to refill, become unsafe, or be monopolized  
* wash basins can dry or become too dirty to be effective  
* latrines can fill, block, or force wilderness fallback  
* sleep sites can crowd, degrade, become unsafe, or be abandoned  
* repeated failure can cause wounds, collapse, death, theft, rationing, debt, flight, or vacancy  
* all recovery attempts must be traceable

## **Mechanics**

### **Water degradation**

Start with water because it already anchors Drink and Wash.

Add or extend concrete state:

* quantity  
* recharge/refill  
* clean/dirty/unsafe state  
* last successful extraction  
* failed extraction trace  
* reliability memory per agent

The current Wash basin refill already transfers water from a co-located source, which is exactly the right pattern. Future water degradation should reuse that source/sink discipline.

### **Food degradation**

Extend item decay from ground archival to survival-relevant food freshness only where it creates decisions:

* stored/cached food may spoil slower than ground food  
* spoiled food can become inedible or risky  
* agents can prefer fresh food, accept spoiled food under desperation, or seek substitutes  
* spoilage emits traceable item-lot state, not a global food score

### **Latrine and sanitation degradation**

Use `LatrineFullness` as the basis. Add:

* blocked/overfull state  
* empty/clean action  
* fallback to wilderness relief  
* place dirtiness as camp/shelter quality input  
* traces explaining “latrine unavailable because blocked/occupied/full”

Avoid disease unless later required.

### **Washbasin degradation**

Use `WashBasinState.dirtiness_level` as consequence carrier. Add:

* dirty basin reduces wash effectiveness or requires cleaning  
* dirty basin can contaminate wash water if exposure/disease later exists  
* clean/refill actions with source/sink lineage

### **Rest-site degradation**

Add crowding/dirtiness/wear to shelters/camps. A frequently used camp without cleaning becomes poor rest, more visible, or less safe.

### **Route degradation**

Route danger should be concrete:

* hostile encounter memories already exist through route experience  
* extend route experience to exposure or rest-related outcomes  
* route “unsafe” is belief-backed memory, not global truth

## **How collapse/death/theft/rationing/flight/abandonment emerge lawfully**

A harsh-world chain should look like this:

1. Food stock is depleted or spoiled.  
2. Agent tries known local source; action fails or source is empty.  
3. Agent tries trade; merchant has no stock or refuses/hoards.  
4. Agent seeks substitute; none known or route too costly.  
5. Agent steals from a visible lot or hoard if survival pressure beats deterrence.  
6. Theft leaves evidence and social/institutional consequences.  
7. If no recovery path remains, hunger/dehydration/fatigue/exposure worsens into wound/collapse/death.  
8. Repeated failures create debt/rationing/flight/closure/abandonment when those support systems are present.

The important point: no step uses a global scarcity score or scenario-specific rescue.

## **Adjacent systems needing small changes**

* **Trade:** support refusal/hoarding/ration once concrete stock state is low.  
* **Theft:** expand survival theft beyond apple lots to hoarded food/water/rest-critical items.  
* **Justice:** evidence and testimony already work; broaden proofs.  
* **Travel:** routes can become known-bad through exposure/hostiles.  
* **Institutions:** ration orders from S64 only after stockout evidence exists.  
* **Settlement decline:** S66 only after departure/facility vacancy becomes necessary.

## **Trace/event proof**

Every harsh failure scenario should assert:

* what source/facility/rest site failed  
* what the agent believed before trying  
* what action was attempted  
* why it failed or was interrupted  
* what fallback was considered  
* what downstream consequence happened  
* that no hidden rescue or remote truth injection occurred

## **Risks and rejected alternatives**

Rejected:

* `SettlementScarcityScore`  
* `FoodCrisisEvent`  
* hidden “force theft” scenario rails  
* global water-quality truth in planner  
* disease from dirtiness without an explicit carrier and proof need

Accepted:

* Some bad outcomes will be ugly: death, abandonment, theft, refusal, and flight should be ordinary under sustained pressure.

# **11. Exposure and Environment Assessment**

## **Should exposure be added now?**

**Yes, but only minimally and only as a support mechanic attached to shelter/rest/travel.**

Exposure should not be a sprawling weather simulation. It should be a concrete consequence carrier for shelter, sleep, travel, and fatigue.

## **Minimal FOUNDATIONS-aligned model**

Add only what is needed:

* `ExposureSource` on place, route edge, or boundary/local weather event:  
  * kind: cold, heat, wetness, smoke, storm, etc.  
  * magnitude  
  * active tick range  
  * source/cause  
  * observability  
* `ExposureState` on agent:  
  * cold exposure  
  * heat exposure  
  * wetness  
  * accumulated critical ticks  
* Mitigation:  
  * shelter/rest site protection  
  * camp/fire if implemented  
  * clothing if already represented or later added  
  * route choice  
* Consequences:  
  * poor sleep recovery  
  * sleep interruption  
  * fatigue/thirst acceleration  
  * exposure wounds/death only after sustained critical exposure

## **What not to add**

Do not add:

* global weather mood  
* hidden random storm drama dial  
* seasonal simulator  
* biome climate system  
* disease ecology

## **Roadmap seam**

Exposure belongs as a **Cluster 1 support mechanic** with seams to Travel and Boundary Processes. If boundary shocks later create storms or droughts, S62 is the right prior-art direction because it requires explicit source regions/channels/disruptions rather than hidden spawners.

# **12. Scenario Validation Plan**

The next validation wave should move from “survival remains viable” to “survival collision is proven.”

## **New scenario: `survival-safe-rest.ron`**

Purpose: prove sleep surfaces, shelter quality, and unsafe rest.

Shape:

* two or three tired agents  
* one good shelter/surface  
* one rough open camp  
* rising hunger/thirst so long sleep has tradeoffs  
* optional local disturbance source  
* no hidden safety knowledge

Assertions:

* agent selects known better rest site when available  
* occupancy prevents simultaneous impossible use  
* rough sleep is legal but lower recovery or interrupted  
* trace explains recovery modifier and wake reason  
* deterministic replay

## **New scenario: `survival-sleep-contention.ron`**

Purpose: prove multi-agent rest contention.

Shape:

* limited shelter capacity  
* multiple critical-fatigue agents  
* route to alternate site  
* one agent waits, one travels, one sleeps rough

Assertions:

* no global knowledge of remote surface unless believed  
* occupancy clears on abort/commit/death  
* no stuck idle under elevated fatigue  
* critical windows show failed/contended rest opportunities

## **New scenario: `survival-rest-interrupted-by-danger.ron`**

Purpose: prove self-care interruption by adjacent pressure.

Shape:

* wounded/tired agent  
* visible or remembered hostile danger  
* rest attempt interrupted by concrete hostile/local event  
* replan to flee, defend, seek safer shelter, or sleep rough

Assertions:

* sleep ended early with specific cause  
* partial recovery preserved  
* danger trace competes with survival trace  
* no hidden scripted attack outside ordinary hostile process

## **New scenario: `survival-scarcity-degradation.ron`**

Purpose: prove concrete source degradation.

Shape:

* water source depletes or basin dries from use  
* food stock spoils/depletes  
* latrine reaches blocked/full condition  
* agents must choose trade, travel, wait, clean, steal, or ration if support exists

Assertions:

* source state changes before failed acquisition  
* failed action/replan traces cite depleted/dirty/full/blocked source  
* fallback is lawful and belief-backed  
* no rescue refill unless concrete source/refill action exists

## **New scenario: `survival-exhaustion-collapse.ron`**

Purpose: prove fatigue failure.

Shape:

* fatigue rises  
* safe rest unavailable/occupied/unsafe  
* repeated rest attempts fail or only partially recover  
* exhaustion collapse occurs

Assertions:

* collapse follows sustained fatigue critical exposure  
* trace includes failed rest opportunities  
* dead/incapacitated agents do not start actions  
* recovery by another agent, if included, is lawful

## **New scenario: `survival-breakdown-cascade.ron`**

Purpose: later, after scarcity response support.

Shape:

* food/water failure  
* merchant hoards or ration order issues  
* hungry agent steals or borrows  
* another agent flees or abandons facility  
* social/evidence aftermath persists

Assertions:

* ration/hoard/debt/theft/flight each follows concrete pressure  
* no global settlement score  
* facility vacancy or abandonment is a concrete component  
* deterministic replay

# **13. Spec-Ready Requirements**

## **MUST**

* MUST preserve player/AI simulation symmetry.  
* MUST use concrete state carriers for rest sites, surfaces, camps, degradation, exposure, wounds, waste, ration/debt/flight records, and vacancies where downstream systems care.  
* MUST keep world state distinct from belief state.  
* MUST require belief-backed or lawful local observation for planner-visible rest/shelter/source safety.  
* MUST define preconditions, duration, costs, occupancy, interruption, commit, abort, partial failure, and aftermath for every new self-care action.  
* MUST make unsafe sleep legal where plausible but meaningfully worse.  
* MUST add fatigue collapse or explicitly defer it with rationale, because current profile state already names exhaustion collapse.  
* MUST keep degradation source/sink explicit.  
* MUST prove authored causal branches, not just survival or death.  
* MUST include deterministic replay assertions.  
* MUST expose author/debug traces and player-POV filtered explanations separately.

## **SHOULD**

* SHOULD reuse `SleepEpisode` for accumulated recovery.  
* SHOULD reuse or extend `SelfCareOccupancy` where sleep/rest surfaces need exclusive use.  
* SHOULD reuse existing route experience for safe-route/safe-camp memory when possible.  
* SHOULD keep exposure minimal: cold/heat/wetness only, concrete local source only.  
* SHOULD make poor rest a first-class trace outcome before adding complex sleep health effects.  
* SHOULD use held specs S60–S66 as prior art, not proof.

## **MUST NOT**

* MUST NOT add hidden rescue scripting.  
* MUST NOT add a global settlement health/scarcity/safety score as authority.  
* MUST NOT let planner use remote authoritative truth.  
* MUST NOT add weather as a drama dial.  
* MUST NOT add disease, hygiene shame, bathroom politics, or etiquette unless a future spec proves they are necessary consequence carriers.  
* MUST NOT hide facility occupancy behind intention frames.  
* MUST NOT make player UI omniscient.

## **Acceptance criteria**

A safe-rest spec is acceptable only if a scenario proves:

* an agent knows or observes multiple rest options  
* one option is better but occupied/unsafe/unavailable  
* the agent lawfully chooses wait, travel, rough sleep, or camp  
* poor rest or interruption creates partial recovery  
* traces explain why  
* replay is deterministic

A scarcity/degradation spec is acceptable only if a scenario proves:

* concrete source/facility state degrades before the failed action  
* the agent attempts lawful recovery  
* fallback or failure follows from belief and action traces  
* downstream consequence leaves world state  
* no hidden rescue occurs

A harsh-collapse spec is acceptable only if a scenario proves:

* collapse/death follows sustained unmet need  
* failed recovery opportunities are visible  
* no post-death actions start  
* another agent can discover/respond only through lawful perception

## **Profile-driven parameter guidance**

Thresholds should live in profiles or authored scenario contracts, not hardcoded scenario hacks:

* sleep surface recovery modifiers  
* rough sleep recovery floor  
* exhaustion collapse ticks  
* exposure accumulation rate  
* water source recharge/depletion  
* latrine blocked threshold  
* basin dirtiness effectiveness threshold  
* food spoilage ticks by commodity/container  
* rest-site memory decay  
* queue patience under critical needs

# **14. Roadmap Edit Recommendations**

Update `docs/gameplay-mechanic-deepening-roadmap.md` in Cluster 1.

## **Add to Cluster 1 mechanics**

Add:

* **Shelter, Sleep Surfaces, and Safe Rest**  
  * Sleep surfaces/rest sites  
  * shelter quality  
  * safe/unsafe rest  
  * rest occupancy  
  * poor rest and specific wake reasons  
  * exhaustion collapse  
* **Survival Facility Degradation**  
  * blocked/dirty latrines  
  * dirty/dry wash basins  
  * water source reliability/quality  
  * food spoilage/storage  
  * sleep-site wear/crowding  
* **Survival Failure Traceability**  
  * failed recovery opportunities  
  * collapse/death causal chain  
  * theft/ration/flight causal chain  
  * player/author explanation surfaces

## **Add support mechanic seam**

Add:

* **Environmental Exposure and Shelter Mitigation**  
  * Cluster 1 support mechanic  
  * seams to Travel and Boundary Processes  
  * cold/heat/wetness only  
  * no full weather simulation initially

## **Reclassify held specs as support prior art**

* **S60 Persistent Site Occupancy:** support for camps, shelters, abandoned sites, rest-site traces.  
* **S61 Predator Ecology:** later danger source for unsafe routes/rest, not P0.  
* **S62 Boundary Processes:** external shortage/weather/refugee pressure seam, later.  
* **S63 Contested Evidence:** adjacent justice support for survival theft/false accusation, later.  
* **S64 Scarcity Response:** high-relevance support for rationing, debt, hoarding, substitution once concrete stockout/degradation is stronger.  
* **S65 Social Aftermath:** support after survival events create theft/refusal/death/rescue memories.  
* **S66 Settlement Decline:** later adjacent support for facility vacancy, flight, abandonment, and reoccupation.

## **Add proof maturity language**

Add a requirement that Cluster 1 rows should move toward **collision-proven coverage**, not just landed scenario coverage. The roadmap already distinguishes proof strength; make that distinction actionable for Cluster 1 exit criteria.

# **15. Risks, Tradeoffs, and Open Questions**

## **Design risks**

**Risk: safety becomes an abstract score.**  
 Mitigation: safety must decompose into concrete known factors: shelter, occupancy, hostile traces, exposure, route experience, local disturbance, camp condition.

**Risk: sleep becomes over-gated.**  
 Mitigation: keep rough sleep legal but poor.

**Risk: exposure sprawls into weather simulation.**  
 Mitigation: only add exposure sources that directly affect rest/travel and have concrete observability/mitigation.

**Risk: sanitation becomes bathroom politics.**  
 Mitigation: use sanitation only as facility degradation and waste consequence; no shame/etiquette.

**Risk: social breakdown arrives before material breakdown.**  
 Mitigation: implement concrete scarcity/degradation before rationing/debt/flight.

## **Implementation unknowns a future spec must verify before coding**

* Whether sleep occupancy should reuse `SelfCareOccupancy` or require a capacity-aware rest-specific component.  
* Exact candidate generation sites for rest options and rough sleep fallback.  
* Whether current facility queue action path can directly serve self-care Wash/Latrine in all intended cases or whether additional queue payload support is needed.  
* How trace volume scales if critical-window forensics include failed recovery opportunities.  
* How visualizer/CLI should split player POV from author/debug views.  
* Whether `exhaustion_collapse_ticks` is intentionally reserved or accidentally unimplemented.

## **Open design questions**

1. Should rough sleep be legal everywhere except in-transit/combat, or should some places forbid it?  
2. Should rest-site safety be entirely deterministic, or can seeded probabilistic local disturbance exist once grounded in concrete local threats?  
3. Should poor rest only reduce recovery, or also increase exposure/wound risk?  
4. Should dirty water be an immediate Drink quality issue, or deferred until minimal illness/poisoning carriers exist?  
5. Should blocked latrines force wilderness relief, or allow risky overuse with worse dirtiness?  
6. Should flight/departure be part of Cluster 1 exit criteria or a later Cluster 1 validation seam with S66?

# **16. Final Recommendation**

The next proposal/theme should be:

**S174 — Shelter, Sleep Surfaces, and Safe-Rest Consequence Carrier**

It should be **one focused spec**, not an implementation ticket set. It should cover:

* rest-site/sleep-surface identity  
* sleep occupancy/capacity  
* safe versus unsafe rest  
* rough-sleep fallback  
* poor-rest recovery  
* specific wake/interruption reasons  
* trace/player explanation  
* one or two scenario-backed collision proofs

It should **not** include full exposure, predators, rationing, settlement decline, or disease. Those should be registered as support seams and activated after safe rest proves the carrier.

The best first implementation slice, once a future spec moves to implementation, would be:

**Known rest surfaces + sleep occupancy + poor-rest trace + one long-running safe-rest contention scenario.**

Cluster 1 is “done enough” to move toward Cluster 2 only when:

* Eat, Drink, Sleep, Relieve, Wash all have lawful acquisition/discovery, interruption, and recovery.  
* Scarce shared affordances have explicit occupancy/contention.  
* Sleep is no longer a universal reset and can fail safely, poorly, or catastrophically through local state.  
* Food/water/sanitation/rest degradation can produce sustained pressure.  
* Hunger, thirst, fatigue, and possibly exposure can collapse or kill through traceable failed recovery paths.  
* Theft, rationing, flight, abandonment, and social aftermath can emerge from ordinary scarcity without scripts.  
* Player and AI obey identical simulation law.  
* Scenarios prove the authored causal branch, not just broad survival or death.  
* Debug and player-facing explanations can reconstruct why an agent survived, struggled, stole, fled, rationed, collapsed, or died.

