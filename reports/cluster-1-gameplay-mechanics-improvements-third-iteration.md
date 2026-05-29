# **. Executive Verdict**

**Verdict: Cluster 1 is now promising and materially stronger, but it is still collision-incomplete.** It is no longer thin. S172–S175 moved it from “needs as timers” toward embodied survival: rest sites now have capacity and quality, rough sleep is distinct from known shelter sleep, sleep can be interrupted, failed rest leaves forensic records, and fatigue can escalate into exhaustion wounds and death. Active core state now includes five homeostatic needs, deprivation exposure counters, rest efficiency, rough-sleep recovery floors, sleep-quality profiles, rest capacity/occupancy, and self-care occupancy for Wash, relief, Eat, Drink, Sleep, and related self-care actions.

**What S172–S175 fixed:** the old holes around Wash discovery/budget closure, self-care interruption and facility occupancy, safe-rest surface identity, known-rest-site versus rough-sleep branching, sleep interruption, failed-rest forensics, fatigue-collapse wounds, recovery dampening, and fatigue death attribution are substantially landed. The active scenarios and tests prove real rest-site contention, rough fallback, hostile sleep interruption, repeated failed-rest windows, exhaustion collapse, and exhaustion recovery.

**What is not good enough:** Cluster 1 still does not yet generate harsh survival drama from degrading material conditions at settlement scale. Food, water, shelter, latrine, basin, and route/supply conditions are not yet deep enough as ordinary world processes. The current system can make an agent hungry, thirsty, dirty, tired, interrupted, wounded, and dead; it does not yet reliably make a world where caches spoil, wells run dry, basins become unusable, latrines fill, shelters decay, merchants refuse, rationing begins, theft becomes a survival fallback, households flee, shops close, and a settlement declines through lawful local causality.

**The P0 before new mechanics is proof integrity.** S175 exhaustion tests are present, registered in the golden test module, and marked `#[ignore]` with comments saying they run via `golden-survival.yml`; but the active `golden-survival.yml` matrix does **not** include `survival_exhaustion_collapse` or `survival_exhaustion_recovery`. That means the S175 CI-ownership claim is false on the evidence I fetched. This is not a minor wording issue: it downgrades S175 from “CI-owned focused proof” to “present but not actually workflow-owned ignored proof.”

**The next real frontier should be internal material degradation plus scarcity response, not full weather, not predator ecology, and not CLI/TUI polish.** Survival becomes compelling when bodily vulnerability collides with concrete sources, stocks, travel, danger, time, memory, and other agents. The repository already has partial carriers for this: `ResourceSource`, commodity decay, `PlaceDirtiness`, `LatrineFullness`, `WashBasinState`, item decay, basin refill from colocated water sources, and long-running item-decay survival proof. Those are the right substrate. Deepen them before introducing a broad exposure/weather system.

**What should explicitly not be deepened yet:** complex bathroom etiquette, hygiene shame, full disease ecology, broad predator ecology, a global settlement-health score, a full weather simulation, or CLI polish. These are either not Cluster 1’s immediate bottleneck or risk violating the project’s concrete-state/no-drama-dial foundations. Exposure should be designed now, but implemented only as a minimal consequence carrier after degradation and scarcity prove themselves.

---

# **2. Evidence Base**

## **Commit, branch, and connector status**

I used the intended exact commit:

`cef985cf521e5715af4a7784b3b0cfe59cc39a68`

Repository discovery found `joeloverbeck/worldwake`, default branch `main`, public, non-archived, repository id `1176423721`.

However, the GitHub connector behaved inconsistently after discovery. Repository metadata and branch lookup calls misrouted to `joeloverbeck/one-more-branch`, and branch SHA verification failed. Exact-commit blob fetches for `joeloverbeck/worldwake/blob/cef985.../<path>` worked. Therefore I **cannot honestly claim that live `main` still equals `cef985cf521e5715af4a7784b3b0cfe59cc39a68`**. I used exact-SHA targeted file fetches only and did not use GitHub code search, stale snippets, cloning, or old memory.

The uploaded prompt and manifest were used as the user’s stated mission and intended file inventory. Because branch verification failed, I did **not** treat the manifest as verified proof of current live `main`; I used it only as an inventory pointer for exact-SHA fetches.

## **Active repo evidence inspected**

I inspected the active foundations and roadmap docs, including `docs/FOUNDATIONS.md`, `docs/gameplay-mechanic-deepening-roadmap.md`, and `docs/scenario-roadmap.md`. The foundations emphasize explainable emergence, no hidden story triggers, concrete world state instead of abstract scores, persistent identity, source/sink lineage, explicit action preconditions/duration/cost/occupancy, visible aftermath, belief/world separation, lawful boundary processes, and player/AI symmetry.

The gameplay roadmap already distinguishes implementation, regular golden coverage, focused auxiliary golden coverage, long-running scenario-backed coverage, registered roadmap coverage, CI-owned long-running coverage, and collision-proven coverage; it also states that S174/S175 are focused proof, not long-running collision-proven Cluster 1 completion.

The scenario roadmap §5.19 correctly registers safe-rest/rest-site/exhaustion as auxiliary behavior coverage and explicitly says it is not a survival-coexistence landing and not collision-proven, but it overstates S175 workflow ownership.

I inspected workflow ownership: `ci.yml`, `scripts/verify.sh`, `golden-survival.yml`, `golden-item-decay.yml`, `golden-drive-escalation.yml`, and `golden-simulation-gaps.yml`.

I inspected active Cluster 1 tests and scenarios around S174/S175, including `survival_safe_rest`, `survival_sleep_contention`, `survival_rest_interrupted_by_danger`, `survival_failed_rest_cascade`, `survival_exhaustion_collapse`, `survival_exhaustion_recovery`, `survival-safe-rest.ron`, and `survival-exhaustion-collapse.ron`.

I inspected core and systems files for current model shape: needs, sleep episodes, rest sites, self-care occupancy, place dirtiness, resource sources, item lots/decay, item-decay system, needs actions, needs system, sleep synthesis, facility contention, wounds, and survival forensics.

I inspected generated golden coverage docs only as structural inventory. They are generated, not behavioral proof. The generated index counts scenario blocks and lists structural metadata; the coverage matrix derives metadata annotations, not live workflow proof or behavioral assertions.

I inspected held specs S60–S66 as prior art, not landed proof. They remain draft held specs and should be edited/split rather than blindly implemented.

## **External sources consulted**

External comparisons used for design criteria included `The Long Dark`, `Don’t Starve`, `RimWorld`, `Project Zomboid`, `Cataclysm: Dark Days Ahead`, `Frostpunk`, GOAP, HTN planning, and BDI agent architecture. These were used to extract survival-pressure patterns, not to import mechanics wholesale.

---

# **3. Proof-Integrity Audit**

## **S174 status**

S174 rest/shelter tests are active and non-ignored. They are part of the `golden_ai` test binary because `golden_ai.rs` includes `mod scenarios`, and `scenarios/mod.rs` registers the S174 modules. `scripts/verify.sh` runs `cargo test --workspace`, so these non-ignored tests should run under ordinary CI through `ci.yml`/verify.

S174 proof is real but focused. It proves specific authored branches: one-slot shelter contention and rough fallback, rest-site queue promotion, hostile sleep interruption, and repeated failed-rest forensics.

It is **not** long-running collision proof. `survival-safe-rest.ron` is a compact two-agent scenario whose authored purpose is “one sleeps in shelter; the other is rejected and rough-sleeps.” It is exactly the right focused proof, but it does not prove 1440-tick coexistence with trade, combat, patrol, justice, supply failure, and long-run recovery.

## **S175 status**

S175 tests are present and registered, but ignored. Both `survival_exhaustion_collapse` and `survival_exhaustion_recovery` are in `scenarios/mod.rs`, and their source files describe focused S175 proof.

The scenario itself is well-authored as a deterministic focused collapse proof: Aster starts critically fatigued, rough-sleep recovery is zero, Watcher is co-located and hostile but human-controlled and never commanded, so the only authored pressure is repeated hostile-proximity sleep interruption. The scenario comments explicitly state that death should be exhaustion collapse, not combat.

The problem is ownership. The ignored test comments say these are CI-only and run via `golden-survival.yml`, but `golden-survival.yml` does not include filters for `scenarios::survival_exhaustion_collapse::` or `scenarios::survival_exhaustion_recovery::`. Its matrix includes other survival modules such as baseline, contested, items decay, theft, trade, etc., and runs ignored tests by matrix filter; S175 is absent.

**Recommendation: P0.** Either add S175 filters to `golden-survival.yml` or create a dedicated `golden-safe-rest.yml`/`golden-exhaustion.yml` workflow that runs:

`scenarios::survival_exhaustion_collapse::`  
 `scenarios::survival_exhaustion_recovery::`

Then update `docs/scenario-roadmap.md` §5.19 to say “CI-owned focused auxiliary coverage” only after that workflow exists. Until then, the honest label is **ignored focused tests present, not CI-owned**.

## **Roadmap honesty**

`docs/scenario-roadmap.md` is mostly honest: it says §5.19 is auxiliary behavior coverage, not a survival-coexistence landing, and that long-running collision proof remains unproven. That is good.

The overclaim is narrower: the S175 “run via golden-survival workflow” claim is stale. Fixing it should be a proof-integrity correction, not a gameplay redesign.

`docs/gameplay-mechanic-deepening-roadmap.md` is directionally accurate in treating S174/S175 as focused proof rather than collision-proven Cluster 1 completion, but it should explicitly call out the workflow gap until corrected.

Generated coverage should remain structural-only. The generated index/matrix are useful inventory and coverage maps, but they do not prove behavior, workflow ownership, or causal branches.

---

# **4. Current Cluster 1 Model After S172–S175**

## **Authoritative bodily state**

Current embodied physiology has five homeostatic needs: Hunger, Thirst, Fatigue, Bladder, and Dirtiness. Each agent can have metabolism rates, rest efficiency, deprivation tolerances, toilet/wash durations, minimum sleep duration, rough-sleep recovery floor, travel multipliers, and wilderness-relief dirtiness penalty.

Deprivation exposure tracks sustained critical ticks per need. The needs system increments needs, applies action body costs, updates critical exposure, creates starvation/dehydration/exhaustion wounds after tolerance windows, and attributes death by pressure among hunger/thirst/fatigue.

Wounds have stable ids, body parts, deprivation/combat causes, severity, inflicted tick, and bleed rate. Deprivation now includes Exhaustion, and wound load determines incapacitation/fatality through combat profiles.

## **Eat and Drink**

Eat and Drink are concrete actions over item lots with consumable profiles. `CommodityKind` includes Apple, Grain, Bread, Water, and other goods. Consumable profiles provide consumption ticks and hunger/thirst/bladder effects. Eat/Drink require target item lot existence, actor control, and the relevant consumable effect.

**Maturity:** medium. They are not mere meters; they require concrete goods and control. But their deeper survival drama still depends on whether food/water sources degrade, deplete, spoil, become contested, or become socially restricted.

## **Sleep / Rest**

Sleep is now duration-bearing and stateful. `SleepEpisode` tracks place, start tick, intended min/max, target recovery, accumulated recovery, recovery modifier, and wake conditions. Sleep quality tracks shelter, ground comfort, and recovery modifier. Wake conditions include duration reached, target recovery reached, projected non-fatigue need breach, scheduled commitment, and local disturbance.

Sleep synthesis uses intention-frame assumptions and expectation-store deadlines to include projected need breaches and scheduled commitments, then always includes local disturbance and target-recovery conditions where appropriate.

**Maturity:** high focused mechanics, medium systemic proof. The model is good. The proof is still too focused.

## **Known rest-site sleep and rough sleep**

Known rest sites are places with `RestCapacity`; places without capacity are not known rest sites, although rough sleep can still occur without consuming a rest slot. `RestOccupancy` tracks occupants deterministically.

The safe-rest scenario proves that two critically tired agents see a one-slot shelter: one gets shelter sleep, the other is rejected and rough-sleeps.

**Maturity:** strong as a local branch. Not yet long-run rich because shelter does not yet degrade, become cold/wet, require fuel, become monopolized, or become part of settlement decline.

## **Self-care occupancy and contention**

Self-care occupancy exists for Wash, LatrineRelief, Eat, Drink, WildernessRelief, and Sleep, carrying occupant, use kind, started tick, and goal key.

The facility contention system supports promotable kinds including SelfCareWash, SelfCareLatrine, and RestSite, prunes invalid waiters, handles patience, expires stale grants, and promotes ready heads with contention payloads.

**Maturity:** good substrate. It needs harsher scenarios where contention happens under scarcity and degradation, not just focused rest/queue proof.

## **Toilet / Latrine / Wilderness relief / Wash**

The hygiene substrate is better than it may look at first glance. `PlaceDirtiness` has value/decay/use dirtiness. `LatrineFullness` has fill, fill per use, and critical threshold. `WashBasinState` has clean water units, max clean water, refill per tick, units per wash, dirtiness level, and dirtiness per use.

Needs actions register toilet, wash, and wilderness relief with durations, preconditions, reservation requirements, and event tags. Wash requires a WashBasin target with clean units. Wilderness relief creates same-place visible waste/dirtiness consequences.

The item-decay system already decays place dirtiness and refills wash basins from colocated water sources, consuming source quantity.

**Maturity:** strong carriers, incomplete harshness. Latrine fullness and basin cleanliness exist, but Cluster 1 still needs proof that blocked/dirty/dry sanitation causes meaningful self-care failure, alternative planning, cleanup/refill labor, and downstream dirtiness/rest/trade consequences.

## **Deprivation, collapse, death, and forensics**

Survival forensics captures critical windows, selected goals, active actions, blockers, local survival affordance summaries, failed-rest opportunities, and an exhaustion-collapse observed signal derived from wounds/death rather than authoritative state.

This is one of Cluster 1’s best-aligned pieces: it explains why an agent was in trouble without turning the report itself into authority. It also points toward the TUI/debug future: player-facing views should expose lawful local evidence and personal memory, while author/debug views can expose forensic windows.

**Maturity:** good focused traceability. Needs extension to food/water/source degradation, dirty water, blocked sanitation, rationing/refusal, flight, theft, and abandonment.

---

# **5. Research Synthesis: What Makes Harsh Survival Compelling**

The strong opinion in the prompt is correct: **a survival mechanic becomes compelling when bodily vulnerability forces situated prioritization under imperfect knowledge and degrading conditions, and when failure produces world-state consequences that other systems can see and react to. It becomes busywork when it is only a timer demanding periodic maintenance.**

The external examples support that, but the lesson is not “copy their systems.” The lesson is pressure coupling.

`The Long Dark` makes cold, fatigue, hunger, fire, wildlife, illness, injury, and day/night mutually meaningful. The important pattern is not “add cold”; it is that temperature, fatigue, food, travel, fire, shelter, and night danger constrain each other. Fire is warmth and cooking; night is colder; injuries change survival options.

`Don’t Starve` makes food spoilage strategically relevant because stored food is not a perfect escape from hunger. Spoiled food still exists but has degraded consequences: it restores less and damages health/sanity. The key pattern for Worldwake is not a sanity meter; it is **stored resources decay into changed affordances instead of disappearing as invisible timer failure**.

`RimWorld` shows why needs become systemic only when unmet bodily and comfort needs spill into behavior: pawns require food, rest, shelter, recreation, clothing, and unmet needs can cause mental breaks such as wandering, arson, or violence. Worldwake should not copy mood breaks, but it should copy the causal principle: unmet self-care must leak into ordinary behavior and social systems.

`Project Zomboid` ties survival to long-horizon world degradation: water/electricity shutoffs, erosion/overgrowth, weather, seasons, clothing, fatigue, boredom, hunger, and stress all matter to survival over time. The useful lesson is not feature breadth; it is that a world becomes harsher through ordinary infrastructure decline, not just faster hunger ticks.

`Cataclysm: Dark Days Ahead` tracks hunger, thirst, morale, illness, temperature, sleep, seasons, weather, persistent worlds, vehicles, shelter, and long-term survival goals. The useful Worldwake lesson is that survival pressure can remain systemic when it is embedded into persistent world state and agent practice rather than a single survival UI loop.

`Frostpunk` demonstrates settlement-scale harshness: warmth, food, coal, labor, health, discontent, hope, emergency shifts, insulation, stockpiles, and law choices create survival politics. Worldwake should not add a hope/discontent score now, but it should adopt the pattern that scarcity becomes dramatic when it changes distribution, labor, institutional choices, and social response.

GOAP/HTN/BDI research supports Worldwake’s existing direction: agents should select goals, plan through preconditions/effects, decompose tasks, act from beliefs rather than omniscience, and replan when local conditions change. F.E.A.R.’s GOAP lesson is that action dependencies are discovered from preconditions/effects at runtime; HTN’s lesson is decomposing high-level tasks into executable primitives; BDI’s lesson is separating beliefs, desires, intentions, and plan execution under bounded rationality.

Worldwake-specific design criteria:

**Good friction** is concrete, local, and choice-producing: “The well is low; the basin is dirty; the shelter is occupied; the apple cache is spoiling; the merchant refuses; the agent remembers a fallback source but it costs dangerous travel.”

**Bad friction** is meter maintenance: “Hunger +2 per tick, click Eat every N ticks.”

**Good harshness** creates lawful recovery chances before terminal outcomes: failed rest → fallback rough sleep → partial recovery → fatigue critical window → exhaustion wound → death only after sustained failed recovery.

**Bad harshness** is hidden punishment: sudden collapse without prior local affordance, belief, trace, or failed recovery chain.

**Good legibility** is post-hoc causal reconstruction: the player/author can see what the agent believed, tried, lost, consumed, failed to access, and why the next action was lawful.

**Bad legibility** is omniscient player rescue or opaque AI magic.

---

# **6. Revised Definition of a Full Cluster 1 Mechanic**

A full Cluster 1 mechanic should meet this standard:

1. It creates **moment-to-moment decisions** between bodily need, travel, danger, time, contested affordance, duty, social cost, and recovery opportunity.  
2. It participates in **long-horizon decline and recovery**, not only immediate need relief.  
3. It has **lawful acquisition/discovery chains** through local observation, belief, testimony, records, memory, and stale/fallible information.  
4. It uses **concrete state carriers**: item lots, resource sources, wounds, basins, latrines, rest sites, shelters, debts, ration records, vacancies, source reliability, failed-rest records.  
5. It has **preconditions, duration, cost, occupancy/contention, interruption, abort behavior, commit behavior, partial progress, and aftermath**.  
6. It has **degradation and shortage consequences**: sources deplete, stores spoil, basins dry/dirty, latrines fill, shelters become degraded, routes become risky, sellers refuse.  
7. It has **failure paths** that can lead to collapse, death, theft, rationing, debt, flight, abandonment, or social breakdown only through ordinary lawful processes.  
8. It has **recovery dampeners**: fallback sources, substitutes, travel, aid, rationing, debt, repair, cleaning, rest, retreat, migration.  
9. It is **player/AI symmetric**. Player control changes presentation, not action legality or world law.  
10. It has **traceable evidence**: action traces, event logs, forensics, cause/source/sink, belief provenance, failed recovery attempts.  
11. It is **deterministic and replayable** under seed.  
12. It is **scenario-proven at several levels**: focused goldens for branch correctness, then 1440-tick CI-owned collision scenarios for systemic coexistence.

By that standard, current Sleep/Rest is close on local mechanics but not full on long-run collision. Eat/Drink are functional but not yet rich. Wash/Relief have carriers but need harsher degradation proof. Scarcity/settlement decline are still mostly future seams.

---

# **7. Gap Analysis**

## **A. Remaining self-care completeness gaps**

**Eat:** Mature as concrete consumption; thin as harsh survival. It lacks food quality, spoilage consequences, stock preservation choices, and scarcity-driven social response. Current item decay can archive ground apples, but perishable food still needs richer lot-level condition rather than mostly “exists until removed.”

**Drink:** Mature as controlled item consumption and water resource extraction substrate; thin as water survival. Water sources have quantity and regeneration, but not quality, contamination, reliability tiers, drought/dryness, or dirty-water tradeoffs.

**Sleep/Rest:** Strong. It has duration, episodes, quality, wake conditions, rough sleep, rest capacity, occupancy, contention, interruption, and failed-rest forensics. Missing: long-running proof, shelter degradation, exposure mitigation, night danger beyond hostile proximity, and multi-pressure collision with obligations/travel/combat/trade.

**Known rest-site sleep:** Strong local branch; missing long-horizon scarcity/degradation proof.

**Rough sleep:** Strong as fallback; missing harsh environmental consequences. Rough sleep currently mainly means lower/zero recovery; later it should interact with exposure, danger, wetness/cold, and poor-ground injury only if those are concrete carriers.

**Sleep interruption:** Strong for hostile proximity and local disturbance; missing broader lawful causes such as weather/exposure, crowding, injury flare, fire failure, obligation alarm, facility raid, or collapsing shelter.

**Fatigue collapse/exhaustion:** Mechanically landed but proof-wiring incomplete. S175 must be CI-owned before it is trusted.

**Toilet/Latrine relief:** Has action and `LatrineFullness`; missing strong consequences when full, blocked, dirty, or contended. No bathroom politics; only consequence-carrying overflow/blockage matters.

**Wilderness relief:** Good as lawful fallback with dirtiness/waste; missing long-run dirtiness ecology where repeated fallback makes a camp worse and forces cleaning, avoidance, or social response.

**Wash:** Has basin state and clean-water precondition. Missing full basin lifecycle proof: dry basin, dirty basin, water-source depletion, cleaning/refill actions, degraded wash recovery, and queue pressure.

**Travel for self-care:** Present through planning and survival scenarios, but needs harsher cases where travel is chosen because a local source is depleted/dirty/occupied/unsafe, and the agent uses belief-backed fallback rather than global truth.

**Discovery/acquisition:** Stronger after previous work, but degradation will demand updated beliefs: “I knew a well existed” is not enough; the agent must know or discover whether it is currently usable, depleted, dirty, or contested.

**Facility contention:** Good substrate; needs scarcity collision proof across water, wash, latrine, sleep, and food in the same run.

**Sustained degradation/scarcity:** The biggest gap. The repo has the pieces, but Cluster 1 is not yet a degrading world.

## **B. Internal material degradation gaps**

Already present:

* Commodity decay map with Apple and Waste defaults.  
* Ground-since decay.  
* `LotOperation::Spoiled` in lineage.  
* `ResourceSource` with available/max quantity and regeneration.  
* Wash basin state and refill consuming colocated water.  
* Place dirtiness decay.  
* Latrine fullness.  
* Item decay events and 1440-tick item-decay survival proof.

Still missing:

* Food freshness/condition as per-lot state.  
* Spoiled-but-still-existing food affordances.  
* Stored/cached food degradation, not just loose ground decay.  
* Water quality and contamination.  
* Source reliability memory.  
* Basins becoming dirty and producing reduced wash relief.  
* Latrine critical/full behavior as action precondition and aftermath.  
* Shelter/rest-site condition/degradation.  
* Route/facility degradation only where survival needs it.

## **C. Exposure/environment gaps**

Current shelter tags affect sleep quality, not environmental exposure. There is no minimal cold/heat/wetness exposure model. That is acceptable for now, but it prevents shelter from becoming a broader survival carrier.

Exposure should be introduced later as a minimal model, not a weather simulation.

## **D. Scarcity/degradation gaps**

The held S64 spec is directionally correct: stockout should lead to substitution, rationing, debt, hoarding, refusal, and aid. But it depends too heavily on S62 boundary shocks as upstream cause. Cluster 1 can and should create scarcity internally first: source depletion, spoilage, basin/latrine failure, and rest-site degradation are enough to drive survival scarcity before full boundary processes.

## **E. Collapse, death, abandonment, theft, social breakdown gaps**

Death traceability is improving. Exhaustion has focused collapse/recovery. Hunger/thirst starvation/dehydration exist as wounds, but need scenario proof as part of degrading-resource worlds. Social aftermath after collapse/death is not yet Cluster 1 mature.

Theft exists elsewhere, but survival-driven theft must become an ordinary fallback when food/water acquisition, trade, borrowing, or rationing fail—not a scripted drama beat.

Flight/abandonment and settlement decline are still held-spec territory. S66 is too broad to absorb whole right now, but minimal flight/vacancy/abandonment seams are needed once scarcity deepens.

## **F. Player/AI symmetry and future TUI implications**

The future TUI should not become a rescue interface. It should expose:

* controlled-agent needs, pain, wounds, local affordances, remembered sources, and known obligations;  
* why an action is legal or blocked;  
* why the agent selected a need over another;  
* known versus authoritative state separation;  
* local traces of degradation.

Author/debug views can expose omniscient forensic windows, generated coverage, causal graphs, and event chains. Player POV must stay belief-bounded.

## **G. Scenario validation gaps**

The main missing proof is **collision-proof Cluster 1**: 1440-tick scenarios where self-care, scarcity, degradation, trade, theft, travel, danger, rest, and social duty coexist without rescue rails.

---

# **8. Broad Menu of Future Cluster 1 Deepening Directions**

## **P0 — Proof ownership and roadmap honesty**

**Purpose:** make current S174/S175 claims truthful before adding complexity.

**Rationale:** ignored tests that are not workflow-owned are not CI proof.

**State carriers:** no new simulation state.

**Systems touched:** workflows and docs only.

**Proof shape:** `golden-survival.yml` or new workflow runs S175 filters; roadmap updates distinguish focused auxiliary proof from collision proof.

**Risk:** low.

**Classification:** proof integrity.

## **P1 — Internal Material Degradation: food, water, basin, latrine, shelter**

**Purpose:** turn survival from recurring need satisfaction into a degrading-world problem.

**Rationale:** aligns with FOUNDATIONS by using concrete sources/sinks and carriers of consequence instead of abstract scarcity.

**Likely state carriers:** `PerishableState`, `WaterQuality`, `SourceReliability`, `BasinCleanliness`, `LatrineBlocked/Overflow`, `RestSiteCondition`.

**Systems touched:** core items/production/place dirtiness/rest site; item decay; needs actions; perception; candidate generation; survival forensics; scenario diagnostics.

**Scenario proof:** focused branch goldens plus 1440-tick `survival-degrading-water`, `survival-food-spoilage-cache`, `survival-sanitation-breakdown`, `survival-rest-site-degradation`.

**Risks:** busywork, too many maintenance actions, hidden failure if source condition is not legible.

**Classification:** Cluster 1 proper plus support mechanics.

## **P1 — Scarcity Response: refusal, rationing, debt, aid, survival theft**

**Purpose:** make resource failure visible socially and economically.

**Rationale:** harsh survival becomes systemic when unmet bodily needs cause actions other agents can observe and react to.

**Likely state carriers:** `DebtRecord`, `RationOrder`, seller refusal memory, demand failure memory, hoard intent, aid request records.

**Systems touched:** trade, social artifacts/records, candidate generation, theft, evidence, survival forensics.

**Scenario proof:** 1440-tick scarcity response where failed lawful acquisition leads to substitution, refusal, rationing/debt/aid, theft or flight only when lawful alternatives fail.

**Risks:** scope creep into full economy/politics.

**Classification:** Cluster 1 support mechanic; partial absorption of S64.

## **P1 — Long-running rest/exhaustion collision proof**

**Purpose:** graduate S174/S175 from focused proof toward collision proof.

**Rationale:** current rest proof is good but isolated.

**State carriers:** mostly existing.

**Systems touched:** scenarios, workflows, forensics assertions.

**Scenario proof:** 1440-tick rest scarcity under travel/trade/danger/obligation with trace assertions.

**Risks:** flaky long runs if assertions are too broad.

**Classification:** Cluster 1 proof maturation.

## **P2 — Minimal Exposure and Shelter Consequence Carrier**

**Purpose:** make shelter matter beyond sleep recovery.

**Rationale:** external survival research strongly supports exposure as meaningful when tied to shelter/fire/clothing/travel, but a full weather sim would be premature.

**State carriers:** `PlaceExposureProfile`, `ShelterProtection`, `AgentExposureState`, `HeatSource`, maybe clothing insulation.

**Systems touched:** needs/wounds, travel, sleep, production/firewood, perception, route preference.

**Scenario proof:** cold-night rough sleep versus shelter/fire/clothing, exposure wound/recovery/death with no drama dial.

**Risks:** turning into weather simulation.

**Classification:** Cluster 1 proper, but after degradation/scarcity.

## **P2 — Flight, abandonment, and vacancy seam**

**Purpose:** allow sustained survival failure to remove agents and close facilities.

**Rationale:** harsh worlds should produce flight and abandonment, but not via settlement-health bars.

**State carriers:** departure reason records, facility vacancy, abandoned stock, last operator.

**Systems touched:** travel, offices/succession, trade, production, scenarios.

**Scenario proof:** repeated failed self-care/acquisition causes one agent to flee and one facility to close; downstream agents observe vacancy.

**Risks:** broad settlement simulation.

**Classification:** Cluster 1 seam; reduced S66 slice.

## **P3 — Predator/night danger ecology**

**Purpose:** make unsafe camps/routes more dangerous.

**Rationale:** plausible, but not the next bottleneck. Rest interruption already has human hostile proximity. Predator ecology is a later support layer.

**State carriers:** predator agents, tracks, dens, territory.

**Risks:** huge scope and easy drama-spawner violation.

**Classification:** adjacent cluster / later seam.

## **P3 — Boundary shocks**

**Purpose:** external supply failure.

**Rationale:** useful later, but internal degradation can create scarcity first. S62 should not block Cluster 1.

**Classification:** adjacent support mechanic.

---

# **9. Highest-Leverage Next Proposal Candidates**

## **Candidate 1 — “Cluster 1 Material Degradation and Source Reliability”**

**Should be:** one broad spec split into 2–3 implementation slices later.

**Dependencies:** existing `ResourceSource`, `ItemLot`, `CommodityDecayMap`, `PlaceDirtiness`, `WashBasinState`, `LatrineFullness`, `RestCapacity`, `SleepQualityProfile`.

**Affected S60–S66:** partially absorbs S60 structural-decay/rest-site relevance; does not require full site occupancy. Does not require S62. Prepares S64.

**Roadmap/workflow changes:** add Cluster 1 roadmap row for material degradation; add focused goldens and at least one 1440 CI workflow row.

**Recommendation:** first.

## **Candidate 2 — “Scarcity Response: Refusal, Aid, Debt, Rationing, and Survival Theft”**

**Should be:** multiple specs. Do not implement the entire held S64 at once.

**Dependencies:** Candidate 1; existing trade/theft/social artifact systems; demand memory.

**Affected S60–S66:** split S64. Absorb immediate survival-relevant refusal/aid/debt/rationing; defer macro institutional scarcity politics.

**Roadmap/workflow changes:** add scarcity-response 1440 scenario row and workflow filter.

**Recommendation:** second.

## **Candidate 3 — “Rest and Exhaustion Collision Maturation”**

**Should be:** one proof/spec cleanup theme, not a mechanic spec.

**Dependencies:** P0 workflow fix; current S174/S175.

**Affected S60–S66:** none.

**Roadmap/workflow changes:** add 1440 rest-scarcity/collapse scenario row.

**Recommendation:** do in parallel with Candidate 1 if cheap.

## **Candidate 4 — “Minimal Exposure and Shelter Consequences”**

**Should be:** one spec, but only minimal cold/wet/heat carrier.

**Dependencies:** material degradation; shelter/rest site proof; likely firewood/heat source support.

**Affected S60–S66:** touches S60 only if shelter condition/occupancy becomes site-like. S61 not needed.

**Roadmap/workflow changes:** new Cluster 1 seam row.

**Recommendation:** third.

## **Candidate 5 — “Flight, Abandonment, and Facility Vacancy”**

**Should be:** one narrow seam spec; do not implement full S66.

**Dependencies:** scarcity response and at least one sustained failure scenario.

**Affected S60–S66:** split/reduce S66; maybe absorb S65 minimal aftermath only.

**Recommendation:** later P2.

---

# **10. Internal Degradation Proposal**

## **Requirements**

Cluster 1 needs degradation because without it, survival tends to become “find renewable source, repeat action.” Degradation makes the world push back lawfully.

The requirements:

* Food, water, sanitation, wash, rest, and shelter affordances must have concrete condition where downstream systems care.  
* Degradation must be observable locally or learned through reports/records; agents must not use live remote truth.  
* Degradation must produce partial states, not only binary usable/unusable.  
* Failure must leave evidence: spoiled food, empty source, dirty basin, full latrine, degraded rest site, failed wash, failed acquisition.  
* Recovery must require concrete action: refill, clean, repair, preserve, relocate, trade, borrow, ration, or abandon.  
* No abstract scarcity score.

## **Food spoilage / item decay**

Current item decay archives ground items by commodity decay map; Apple defaults to decay and Waste decays. `LotOperation::Spoiled` already exists in lineage, which is a strong hint that spoilage should become richer than archiving.

Recommended model:

* Add per-lot freshness/condition for perishable lots.  
* Decay condition over time depending on storage context.  
* Fresh food provides normal relief.  
* Stale food provides reduced relief.  
* Spoiled food remains a lot or transforms into Waste/compost/unsafe food, with provenance.  
* Eating spoiled food should not automatically imply a full disease ecology. At most, it may create a simple `FoodSickness` or `DigestiveDistress` wound/condition later if it becomes a meaningful consequence carrier.

Rejected alternative: “Food disappears after N ticks.” That is too thin and erases story.

## **Water depletion / dirty water / well reliability**

Current `ResourceSource` supports concrete available/max quantity, regeneration, extraction slots, and extraction duration. This is already the right base.

Recommended model:

* Add water quality to water sources: Clean, Stale, Muddy, Unsafe/Contaminated.  
* Add reliability memory: an agent can remember “this source was depleted/dirty at tick T.”  
* Drinking unsafe water can reduce thirst but cause later consequence only if modeled concretely.  
* Wells can be reliable, slow-regenerating, dry, or contaminated.  
* Basin refill should prefer clean water; dirty water should either fail wash preconditions or reduce wash effectiveness.

Rejected alternative: abstract “water scarcity level.” The source itself should be scarce.

## **Washbasin degradation / refill / cleaning**

`WashBasinState` already has clean water units, max clean water, refill rate, units per wash, dirtiness level, and dirtiness per use. `item_decay_system` refills basin clean water from colocated water source and reduces source quantity. This is excellent substrate.

Deepening requirements:

* Dirty basin should reduce wash relief or fail at high dirtiness.  
* Dry basin should block wash or produce only partial relief.  
* Refill should consume concrete water from clean/usable sources.  
* Cleaning a basin should be an action with duration, occupancy, and waste aftermath.  
* Basin events should be visible in forensics when Wash fails or gives poor recovery.

## **Latrine fullness / blocked sanitation**

`LatrineFullness` exists but needs harsher use.

Recommended model:

* Latrine use increases fill.  
* Above critical threshold, latrine becomes blocked/degraded.  
* Full latrine either rejects relief, causes place dirtiness/overflow, or forces wilderness relief.  
* Empty/clean latrine action should take time and produce Waste or dirty-place aftermath.  
* No etiquette/shame layer.

## **Shelter/rest-site degradation**

Sleep quality and rest capacity exist, but shelter condition does not.

Recommended minimal model:

* Add rest-site condition: Clean/Usable/Degraded/Unsafe or a numeric condition with tags.  
* Overuse, violence, fire, neglect, dirtiness, or exposure events can degrade condition.  
* Degraded rest site reduces recovery modifier, capacity, or interruption risk.  
* Repair/clean rest site can restore condition.  
* For now, avoid structural building simulation.

## **Route/facility degradation**

Defer general route degradation unless a scenario needs it. Route risk should mostly come through experience, threat, exposure, or blockage. Facility degradation should start only with survival-critical facilities: basin, latrine, well, shelter.

## **Planner/belief implications**

Every degraded affordance needs three views:

1. authoritative state;  
2. local same-tick observation when co-located;  
3. remembered/believed state with freshness/provenance.

Remote stale beliefs must be allowed to be wrong. A hungry agent may travel to a remembered cache and find it spoiled. A thirsty agent may reach a remembered well and find it dry. That failure should update memory and drive fallback.

## **Trace/event implications**

Add or reuse trace/event concepts:

* `FoodSpoiled`  
* `FoodConditionChanged`  
* `WaterSourceDepleted`  
* `WaterQualityObserved`  
* `WashBasinDry`  
* `WashBasinDirty`  
* `WashEffectReduced`  
* `LatrineFull`  
* `LatrineOverflow`  
* `ShelterConditionChanged`  
* `FailedRecoveryOpportunity`

These should feed survival forensics without becoming authority.

## **Scenario proof**

Required focused proof:

* Food lot spoils and remains traceable through provenance.  
* Agent rejects spoiled food if safer alternative exists.  
* Agent eats spoiled food under severe hunger only when profile/pressure allows.  
* Water source depletes; agent observes failure and uses lawful fallback.  
* Basin dries/dirty; Wash fails or partially succeeds and emits trace.  
* Latrine fills; agent chooses wilderness relief or cleanup.

Required 1440 proof:

* Multi-agent degrading water/food/sanitation scenario with survival-health assertions and causal trace assertions.

---

# **11. Exposure and Environment Assessment**

**Recommendation: defer full exposure, but design the minimal model now.**

External survival comparisons strongly support exposure as a powerful survival mechanic when it couples to shelter, fire, clothing, travel, injury, and time of day. `The Long Dark` ties body temperature, hunger, fatigue, fire, cooking, wildlife, illness/injury, and colder nights into one survival web; `Project Zomboid` makes seasons/weather and gear matter; CDDA tracks temperature alongside hunger, thirst, illness, and sleep.

But in Worldwake, full weather is not the next best move. The current repository already has concrete survival affordances that need deepening: food, water, basins, latrines, shelters, item decay. Exposure should come after those are richer, because exposure needs shelter/fire/clothing/route proof to avoid becoming another meter.

## **Minimal FOUNDATIONS-aligned exposure model**

When added, it should include:

* `PlaceExposureProfile`: cold/heat/wet/wind exposure pressure at a place or route edge.  
* `ShelterProtection`: protection against exposure, probably extending or relating to `SleepQualityProfile`.  
* `AgentExposureState`: accumulated exposure stress, not a global weather score.  
* `HeatSource`: concrete fire/campfire/stove consuming Firewood.  
* `ClothingProtection`: only if clothing is already meaningful as an item/facility carrier.  
* `ExposureWound`: cold/heat/wetness consequence, traceable like deprivation wounds.

Start with static authored exposure places and night/camp conditions. Do not implement fronts, seasons, storms, or climate unless a boundary process requires it.

## **Roadmap edit**

Add a Cluster 1 support/seam row:

**“Minimal Exposure and Shelter Consequence Carrier”**  
 Status: candidate after material degradation and scarcity-response proof.  
 Proof requirement: focused exposure branch plus 1440 CI scenario if landed.  
 Non-goal: full weather simulation.

---

# **12. Scarcity Response and Harsh-World Failure Proposal**

## **Requirements**

Scarcity response should emerge only from concrete stock, source failure, degradation, failed purchase/acquisition, and belief. No global scarcity event. No hidden rescue.

Agents should be able to:

* substitute;  
* refuse sale;  
* hoard;  
* borrow;  
* request aid;  
* ration;  
* steal;  
* flee;  
* abandon facilities;  
* die or collapse when lawful recovery fails.

## **Mechanics**

Start with survival-relevant S64 slice:

* Demand memory records stockout/refusal.  
* Seller refusal is an action or trade outcome, not hidden scoring.  
* Borrowing transfers goods and creates a debt record.  
* Aid transfers goods and creates gratitude/debt/record if social aftermath exists.  
* Ration orders are institutional records with priority, quantity, duration, and distribution action.  
* Hoarding is over-acquisition beyond immediate need, limited by carrying/storage, beliefs, and future-supply fear.  
* Survival theft is emitted only after lawful acquisition paths fail or are unavailable, subject to existing law/risk profiles.

S64’s direction is excellent, but it should be split. Its immediate Cluster 1 subset is concrete survival response; formal macro-rationing politics can wait.

## **How degradation becomes concrete state**

Internal degradation provides upstream pressure:

* Food cache spoils.  
* Water source depletes or becomes dirty.  
* Basin cannot wash.  
* Latrine blocks.  
* Shelter degrades.  
* Merchant stock decays or is hoarded.  
* Repeated failure creates memory and changed behavior.

Then scarcity response uses existing systems: trade, theft, social artifacts, records, evidence, candidate generation.

## **Collapse/death/theft/rationing/flight/abandonment**

A proper harsh scenario should read like this:

1. Water source depletes.  
2. Agent observes depletion locally.  
3. Agent tries remembered fallback.  
4. Fallback is distant/contested/dirty.  
5. Agent attempts trade.  
6. Seller refuses or stockout occurs.  
7. Agent requests aid or debt.  
8. Aid fails or is partial.  
9. Agent steals or rations or flees depending on profile and beliefs.  
10. If still unrecovered, dehydration wound/death occurs.  
11. Other agents observe theft/death/vacancy and respond.

That is harsh without being scripted.

## **Relationship to S60–S66**

* **S60:** reduce and partially absorb. Cluster 1 needs persistent shelter/facility condition and occupancy traces, not full ruins/dungeons/site hierarchy yet.  
* **S61:** defer. Predator ecology is not needed for current Cluster 1 completion.  
* **S62:** defer except for explicit boundary inflows later. Internal degradation should come first.  
* **S63:** leave for justice. Only touch if survival theft/warrants need minimal evidence seam.  
* **S64:** split and absorb the survival-relevant subset.  
* **S65:** split. Minimal aftermath from aid/debt/death may be needed; full grudge/kin/revenge later.  
* **S66:** reduce/defer. Add narrow facility vacancy/flight only after scarcity proof; full decline/reoccupation later.

---

# **13. Scenario Validation Plan**

## **Immediate proof correction**

1. Add S175 workflow ownership:  
   * `scenarios::survival_exhaustion_collapse::`  
   * `scenarios::survival_exhaustion_recovery::`  
2. Update scenario roadmap §5.19:  
   * Before workflow fix: “ignored focused tests present, not CI-owned.”  
   * After workflow fix: “CI-owned focused auxiliary proof, not collision-proven.”  
3. Keep generated coverage language structural-only.

## **New focused scenarios**

**`survival-water-source-depleted.ron`**  
 Agent reaches known water source; source is depleted; agent observes failure, updates belief, uses fallback or fails lawfully.

**`survival-dirty-water-tradeoff.ron`**  
 Dirty water reduces thirst but creates traceable consequence or is avoided if clean fallback known.

**`survival-food-spoilage-cache.ron`**  
 Remembered cache spoils; agent must choose spoiled food, alternative, trade, or theft.

**`survival-basin-dry-dirty.ron`**  
 Wash basin lacks clean water or is dirty; Wash fails/partially succeeds; refill/cleaning path proven.

**`survival-latrine-full.ron`**  
 Latrine hits critical fullness; relief path branches to cleanup, wait/queue, or wilderness relief; dirtiness aftermath proven.

**`survival-shelter-degraded.ron`**  
 Known shelter has lower condition/recovery or reduced capacity; rough sleep/rest-site fallback branch proven.

## **New 1440-tick CI-only collision scenarios**

**`survival-degrading-water-1440.ron`**  
 Multiple agents share a finite/regenerating water source. Depletion causes fallback travel, contention, trade/refusal, and at least one critical thirst window. Assertions prove source quantity changes, belief updates, fallback source provenance, and no omniscient target injection.

**`survival-food-spoilage-cache-1440.ron`**  
 Food cache spoils during survival loop. Agents consume, trade, preserve, substitute, or fail. Assertions prove exact lot lineage and spoilage consequences.

**`survival-sanitation-breakdown-1440.ron`**  
 Latrine/basin/place dirtiness degrades under multi-agent use. Agents queue, clean, refill, use wilderness relief, and suffer or recover from dirtiness pressure.

**`survival-rest-scarcity-collapse-1440.ron`**  
 Rest-site contention collides with travel, danger, obligations, and recovery. It must prove not only sleep happened, but why known shelter failed and why rough sleep/recovery/failure followed.

**`survival-scarcity-response-1440.ron`**  
 Stockout/degradation causes seller refusal, debt/aid/rationing, and possible survival theft. Assertions prove causal sequence.

**Later: `survival-exposure-cold-night-1440.ron`**  
 Only after minimal exposure exists.

Every 1440 scenario must be registered in `docs/scenario-roadmap.md` and run solely through `.github/workflows`, with deterministic replay and causal assertions.

---

# **14. Spec-Ready Requirements**

## **MUST**

* Degradation must be stored as concrete world state on item lots, resource sources, facilities, places, rest sites, or records.  
* Every self-care action must preserve player/AI symmetry.  
* Agents must use local observation or belief-backed memory, not live remote truth.  
* Every degradation-induced failure must emit traceable evidence or event state.  
* Food/water/sanitation/shelter degradation must have recovery paths.  
* Dirty/dry/full/depleted states must affect action legality or effect magnitude.  
* Scenario assertions must prove causal branch, not just survival/death.  
* Ignored long-running/focused tests must be workflow-owned if docs say CI-owned.

## **SHOULD**

* Use partial degradation states before binary failure.  
* Prefer existing carriers where adequate: `ResourceSource`, `ItemLot`, `PlaceDirtiness`, `WashBasinState`, `LatrineFullness`, `RestCapacity`, `SleepQualityProfile`.  
* Add source reliability memory after failed acquisition/observation.  
* Extend survival forensics to include degraded source/facility/rest-site summaries.  
* Provide profile-driven thresholds for tolerance, risk, substitution, hoarding, and willingness to use unsafe affordances.

## **MUST NOT**

* Add a global scarcity score, settlement health score, danger director, hidden rescue, or scripted collapse.  
* Add bathroom shame, etiquette, or hygiene politics.  
* Add disease ecology unless it becomes a concrete consequence carrier with source, trace, recovery, and proof.  
* Add full weather simulation as a prerequisite for Cluster 1.  
* Let the TUI leak omniscient hidden state to player POV.

## **Acceptance criteria**

A future implementation spec is acceptable only if it can produce at least one focused proof and one roadmap-registered collision proof plan for each major mechanic. “Code exists” is not enough.

---

# **15. Roadmap and Held-Spec Edit Recommendations**

## **`docs/gameplay-mechanic-deepening-roadmap.md`**

Add a new Cluster 1 frontier section:

**Material Degradation and Scarcity Carriers**

* Food spoilage / lot condition.  
* Water depletion and quality.  
* Basin/latrine degradation.  
* Shelter/rest-site condition.  
* Source reliability memory.  
* Long-running scarcity/degradation proof.

Mark S174/S175 as:

* landed focused coverage;  
* S175 workflow ownership pending until fixed;  
* not collision-proven.

Add “not now” notes:

* full weather;  
* predator ecology;  
* full settlement decline;  
* CLI polish.

## **`docs/scenario-roadmap.md`**

Update §5.19 to correct S175 workflow ownership. Add future rows:

* Cluster 1 material degradation focused proof.  
* Cluster 1 degradation 1440 collision proof.  
* Cluster 1 scarcity response 1440 proof.  
* Cluster 1 rest scarcity 1440 proof.  
* Later minimal exposure proof.

Every row should specify workflow ownership expectation.

## **Held spec edits**

**S60:** reduce. Keep persistent occupancy/site traces as later support; extract shelter/rest-site condition only if needed for Cluster 1.

**S61:** defer/archive from immediate Cluster 1. Predator ecology is not the next frontier.

**S62:** keep as later boundary support. Do not make S64 depend on it for first scarcity pass.

**S63:** leave alone, except note survival theft may later need minimal evidence/warrant seam.

**S64:** split. Absorb survival-relevant refusal, aid, debt, rationing, hoarding, substitution into Cluster 1 support. Defer macro institutional distribution.

**S65:** split. Minimal social aftermath for debt/aid/death can be support; full grudges/kin/revenge later.

**S66:** reduce. Extract narrow departure/facility vacancy only after scarcity response. Full settlement decline/reoccupation later.

---

# **16. Risks, Tradeoffs, and Open Questions**

## **Design risks**

The largest risk is turning rich survival into chores. Food spoilage, basin cleaning, latrine emptying, and shelter repair can become busywork if they do not force meaningful choices. The solution is to keep degradation sparse, consequence-rich, and profile-driven.

The second risk is hidden collapse. Harsh systems need recovery attempts and traces. Death is only satisfying if the player/author can reconstruct why recovery failed.

The third risk is overbroad ecology. Exposure, disease, predators, and weather can swallow Cluster 1. The best path is narrow carriers first.

## **Implementation unknowns a future spec must verify**

* Which degradation state can reuse existing components versus needs new components.  
* Whether `LotOperation::Spoiled` is already used anywhere beyond schema.  
* Whether current wash/latrine effects fully consume/update `WashBasinState` and `LatrineFullness` in all cases.  
* Whether candidate generation can rank dirty/unsafe/depleted affordances without new planner bloat.  
* How survival forensics should summarize source/facility condition without becoming authoritative.  
* Whether long-running CI runtime remains acceptable with several 1440 scenarios.

## **Open design choices**

* Should dirty water produce a wound immediately, a delayed condition, or only reduced utility at first?  
* Should spoiled food remain edible under desperation?  
* Should shelter degradation be a numeric condition, tags, or both?  
* Should rationing be institutional-only at first, or can household rationing exist as private behavior?  
* Should flight remove agents from simulation at boundary, or require an explicit off-map destination process?

These are not blockers; they are future spec decisions.

---

# **17. Final Recommendation**

The recommended next theme is:

**“Cluster 1 Material Degradation and Source Reliability”**

It should be one architectural proposal with multiple later implementation slices, not a monolithic implementation burst. The first slice should focus on **water source depletion/quality and washbasin/latrine functionality**, because the repository already has `ResourceSource`, `WashBasinState`, `LatrineFullness`, and item-decay maintenance hooks. That gives the highest leverage with the least new conceptual surface.

The best first implementation slice would be:

1. fix S175 workflow ownership;  
2. deepen water/basin/latrine degradation;  
3. extend forensics and belief surfaces for degraded affordances;  
4. add focused branch goldens;  
5. add one 1440 CI-owned `survival-degrading-water` collision scenario.

Do **not** start with full exposure, predators, disease, or settlement decline. Those will be more meaningful after internal degradation and scarcity response produce lawful pressure.

Cluster 1 is “done enough” to move to Cluster 2 only when:

* S172–S175 are workflow-owned and roadmap-honest;  
* food/water/wash/relief/rest have concrete degradation and recovery;  
* at least one harsh 1440 degradation/scarcity scenario is CI-owned;  
* scarcity can lawfully produce refusal, aid/debt/rationing or theft in proof;  
* collapse/death/flight/abandonment paths have traceable failed recovery opportunities;  
* generated coverage is never mistaken for behavioral proof;  
* player and AI remain symmetric;  
* future TUI views can explain survival decisions without omniscient rescue.

Right now, Cluster 1 is strong enough to justify the next deepening pass, but not strong enough to declare “done enough.”

