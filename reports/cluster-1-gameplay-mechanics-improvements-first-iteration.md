# **1. Executive Verdict**

**Cluster 1 is promising, long-running-scenario-backed, and absolutely not complete yet.** The current model has real substance: five homeostatic needs, AI goal generation, travel physiology, depletion/death, self-care actions, sleep episode state, waste/dirtiness consequences, survival-health contracts, and several ignored CI goldens. But it is **collision-incomplete** because the mechanics are not yet proven under the two pressures that make survival systems matter: **route/resource/facility discovery under bounded planning** and **interruption/recovery under competing action pressure**.

The first thing to deepen is **Wash as a lawful first-class survival action**. Right now Wash exists, is drive-generated, is action-backed, and is proven in baseline/drive-escalation contexts, but scattered and contested proof explicitly exempt Wash from budget-exhaustion checks, and contested omits Wash from the required survival self-care families. That is not a harmless testing detail; it is the exact seam where a meter risks becoming decorative instead of systemic. Wash must obey the same discovery, travel, planner budget, failure, contention, and trace rules as Eat/Drink/Sleep/Relieve. The active gameplay roadmap already calls this out as the first-class target.

The second thing to deepen is **interrupted self-care recovery**. The action engine already supports interruption/abort traces and reservation cleanup, but most Cluster 1 actions currently abort as no-ops. Sleep is the exception: it has durable `SleepEpisode` state and partial recovery. Eat, Drink, Toilet, Wilderness Relief, and Wash mostly remain atomic commit-or-nothing actions. That is acceptable only if the game can prove that interruption produces lawful retry/reprioritization rather than silent failure, hidden rescue, or permanent planner confusion.

What should **not** be deepened yet: social etiquette, privacy, bathroom politics, moral reactions to hygiene, disease ecology, full sanitation economy, complex shelter politics, trade redesign, theft redesign, pursuit redesign, justice redesign, or rescue scripting. The first pass should be brutal and simple: **occupy, release, abandon, interrupted cleanup, lawful retry, traceable failure**.

My strongest recommendation: name the next spec theme **“C1 Embodied Self-Care Collision Proof: Wash Planning + Interruptible Recovery.”** It should be one requirements spec with two implementation slices, not two unrelated specs, because Wash budget closure and interruption recovery both depend on the same simulation law: self-care is not a meter, it is an embodied time/route/resource/facility commitment.

---

# **2. Evidence Base**

## **Repository state used**

The live repository resolved to `joeloverbeck/worldwake`, default branch `main`. The current `main` SHA was **`a83cd87617a48e767c2bd53abd66117367cf4b6f`**, matching the SHA you intended for this pass. I used that SHA as the only authoritative repository ref.

I did not clone the repository and did not use GitHub code search snippets. I used repo metadata, the current branch SHA, the active repo manifest, and targeted exact-SHA file fetches.

## **Manifest status**

The uploaded manifest was present, but I treated it as low-trust inventory only. The active repo manifest at the verified SHA includes a path near the end that the uploaded manifest’s final section did not include, so I did **not** rely on the uploaded manifest as evidence.

## **Active repository files fetched**

Primary active docs:

* `docs/FOUNDATIONS.md`  
* `docs/scenario-roadmap.md`  
* `docs/gameplay-mechanic-deepening-roadmap.md`  
* `reports/manifest_2026-05-25.txt`

Key active workflows:

* `.github/workflows/golden-survival.yml`  
* `.github/workflows/golden-drive-escalation.yml`  
* `.github/workflows/golden-simulation-gaps.yml`  
* `.github/workflows/golden-planner-pathology.yml`

Key active scenarios and goldens:

* `scenarios/survival-baseline.ron`  
* `scenarios/survival-scattered.ron`  
* `scenarios/survival-contested.ron`  
* `scenarios/survival-drive-escalation.ron`  
* `crates/worldwake-ai/tests/scenarios/survival_baseline.rs`  
* `crates/worldwake-ai/tests/scenarios/survival_scattered.rs`  
* `crates/worldwake-ai/tests/scenarios/survival_contested.rs`  
* `crates/worldwake-ai/tests/scenarios/survival_drive_escalation.rs`  
* `crates/worldwake-ai/tests/scenarios/simulation_gaps.rs`  
* `crates/worldwake-ai/tests/integration/forensic_sleep_progress_barrier.rs`  
* `crates/worldwake-ai/tests/integration/forensic_wash_vs_water_competition.rs`

Key active implementation files:

* `crates/worldwake-core/src/needs.rs`  
* `crates/worldwake-core/src/sleep_episode.rs`  
* `crates/worldwake-core/src/goal.rs`  
* `crates/worldwake-systems/src/needs.rs`  
* `crates/worldwake-systems/src/needs_actions.rs`  
* `crates/worldwake-systems/src/sleep_synthesis.rs`  
* `crates/worldwake-systems/src/travel_actions.rs`  
* `crates/worldwake-systems/src/facility_queue.rs`  
* `crates/worldwake-systems/src/facility_queue_actions.rs`  
* `crates/worldwake-sim/src/start_gate.rs`  
* `crates/worldwake-sim/src/tick_action.rs`  
* `crates/worldwake-sim/src/interrupt_abort.rs`  
* `crates/worldwake-sim/src/action_termination.rs`  
* `crates/worldwake-sim/src/action_trace.rs`  
* `crates/worldwake-ai/src/goal_schema.rs`  
* `crates/worldwake-ai/src/goal_model.rs`  
* `crates/worldwake-ai/src/candidate_generation.rs`  
* `crates/worldwake-ai/src/planner_ops.rs`  
* `crates/worldwake-ai/src/search/transition.rs`

## **External sources consulted**

I used external design research for mechanics/dynamics/aesthetics, feedback, game feel, GOAP-style agent autonomy, and survival/simulation examples. The MDA paper defines mechanics as data/algorithm-level components, dynamics as runtime behavior over time, and aesthetics as the player-facing emotional response; it also emphasizes that small implementation decisions cascade into gameplay dynamics and player experience. The screenshot attempt for the MDA PDF failed because the PDF redirected to an unsafe HTTP URL, so I relied on the fetched parsed PDF text and cite that source explicitly.

I also consulted a game-feel survey that identifies feedback clarity and support for player intent as part of moment-to-moment game feel. For emergent AI, I used the F.E.A.R. GOAP example: its NPCs choose goals and compose plans from action preconditions/effects instead of following hard-coded behavior transitions. For survival/self-care examples, I used The Long Dark, RimWorld, Don’t Starve, Project Zomboid, Raft, and Abiotic Factor as comparative pressure patterns, not as mechanics to import wholesale.

---

# **3. Current Cluster 1 Model**

## **Foundations-aligned substrate already exists**

Worldwake’s active constitution is unusually explicit. It requires explainable emergence, no authored outcomes, no hidden quest logic, concrete state over abstract scores, stable identity for consequential objects, explicit source/sink/lineage, locality of knowledge, action preconditions/duration/cost/occupancy, deterministic replay, and validation that proves the authored causal reason rather than only a plausible end state.

Cluster 1 already has real authoritative state: `HomeostaticNeeds` contains Hunger, Thirst, Fatigue, Bladder, and Dirtiness; `MetabolismProfile` defines basal rates, sleep/rest efficiency, deprivation tolerances, toilet/wash duration, travel multipliers, and wilderness-relief dirtiness penalty; `DeprivationExposure` tracks ticks at critical exposure per need.

The needs system advances homeostatic pressure, applies action body costs, updates deprivation exposure, emits drive-escalation begin/end events, creates deprivation wounds, triggers bladder accidents, creates Waste, increases dirtiness/place dirtiness, and can mark death by need deprivation.

## **Current action families**

The active needs actions define:

* `eat` and `drink` as target-consumable actions with control and consumable-profile preconditions.  
* `sleep` as a durable sleep episode with start/tick/commit/abort behavior.  
* `toilet` as a latrine-place action that clears bladder and creates Waste.  
* `relieve_wilderness` as an outdoor-place action that clears bladder, creates Waste/evidence, and increases dirtiness/place dirtiness.  
* `wash` as a WashBasin-targeted action that requires clean basin water and reduces actor dirtiness while consuming clean water and increasing basin dirtiness.

Sleep is currently the strongest action in this cluster: it has `SleepEpisode` state with place, intended min/max ticks, accumulated recovery, recovery modifier, and wake conditions. Sleep tick reduces fatigue incrementally; abort ends the episode with `WakeReason::LocalDisturbance` and preserves accumulated recovery.

Travel is also substantive: it has adjacency preconditions, duration, in-transit state, body cost overrides from travel physiology, commit arrival, abort return-to-origin behavior, route experience, and movement trace evidence.

## **Current AI/planner model**

The goal model includes `ConsumeOwnedCommodity`, `AcquireCommodity(SelfConsume)`, `Sleep`, `Relieve`, `Wash`, and exploration hypotheses such as `MayContainLatrine`, `MayContainWashBasin`, and `MayContainSleepSite`.

The goal schema assigns self-care policy and `GoalPlanningBudget::SELF_CARE` to Consume, Acquire SelfConsume, Sleep, Relieve, Wash, and FreeCarryCapacity. It also declares Wash relevant ops as `[Wash, Travel]`, Relieve as `[Relieve, Travel]`, Sleep as `[Sleep]`, and Acquire as `[Travel, Trade, QueueForFacilityUse, Harvest, Craft, MoveCargo]`.

Candidate generation emits need candidates for consume, sleep, relieve, wash, and dirtiness-water acquisition, so Wash is not absent from the AI model.

Planner operation classification recognizes needs actions: `eat`/`drink` as Consume, `sleep` as Sleep, `toilet`/`relieve_wilderness` as Relieve, and `wash` as Wash. Wash may appear mid-plan and is not a materialization barrier.

The search transition builder estimates duration, applies effects hypothetically, accumulates total duration/search cost, and can produce progress/information/coordination terminals. That means the architecture is already capable of charging Wash planning cost in principle; the gap is in validation/proof and likely in target enumeration/discovery/travel search behavior.

## **Current proof strength**

The active scenario roadmap says scenario-backed goldens are the canonical proof surface and that a feature lands only when the golden proves the authored behavior and causal reason, not just a broad outcome.

Current Cluster 1 proof tiers:

| Surface | Current strength | Assessment |
| ----- | ----- | ----- |
| Needs state/metabolism/deprivation code | Implementation only + some tests | Real substrate. |
| Eat/Drink/Sleep/Relieve/Wash baseline | Long-running scenario-backed + roadmap-registered | Strong baseline, not collision proof. |
| Travel physiology under survival pressure | Long-running scenario-backed | Stronger for food/water/relief/sleep than Wash. |
| Wash under drive escalation | Long-running scenario-backed + roadmap-registered | Proves recurring Wash and no remote Wash omniscience in one branch. |
| Wash under budget exhaustion | Explicitly incomplete | Scattered/contested exempt it. |
| Facility queue contention | Scenario-backed in survival-trade for production/water harvesting | Not yet self-care facility contention. |
| Sleep partial progress | Implementation + regular forensic test | Promising, not self-care interruption recovery proof. |
| Collapse/death traceability | Scenario-backed simulation gap | Strong for hunger starvation, not repeated interrupted self-care. |

The survival baseline scenario requires all five self-care families and includes food, water, latrine, washbasin, travel/exploration, and survival-health assertions. The scattered scenario also requires all five and intentionally separates food/water/wash affordances across places under travel pressure. The contested scenario, however, requires only Eat/Drink/Sleep/Relieve and its comments point at Wash preconditions and water possession as a tightening seam.

The drive-escalation scenario requires all five, starts with high dirtiness, co-locates a shared well and washbasin, proves repeated Wash, wilderness relief, and dirtiness escalation ending after Wash relief.

The core Wash budget gap is explicit: survival-scattered and survival-contested tests define Wash as a survival goal, then exclude Wash from budget-exhaustion checks, with comments saying Wash can exhaust budget before discovery of a WashBasin and that the issue is tracked as a travel-pruning/planning problem.

---

# **4. What Makes Cluster 1 Mechanically Full**

A “full” Cluster 1 mechanic is not five meters that occasionally trigger animations. It is a **situated embodied loop** where bodily pressure changes what routes, facilities, resources, risks, and social obligations matter.

The MDA lens is useful here because Worldwake’s player-facing experience must emerge from simulation mechanics, not from authored drama. MDA says mechanics generate runtime dynamics, which then create player experience; it also warns that small data/algorithm decisions can cascade upward into gameplay. For Worldwake, this means a Wash omission in a budget-exhaustion test is not a test quirk; it changes the dynamics of survival because it lets a bodily need escape the same pressure law as the rest of the loop.

A full Worldwake survival mechanic must satisfy seven criteria:

1. **Embodied authority.** The need is carried in authoritative world state, not an abstract survival score. Hunger, thirst, fatigue, bladder, dirtiness, waste, wounds, facility water, basin dirtiness, latrine fullness, place dirtiness, sleep episode state, and death state must all be real carriers of consequence.  
2. **Situated prioritization under imperfect knowledge.** Survival is compelling when the agent has to choose under uncertainty: eat now or travel to water, sleep here or risk the route, relieve outdoors and get dirty or wait for a latrine, wash now or save clean basin water. Games like The Long Dark and Project Zomboid make survival tense by combining body needs with route/environment/enemy pressure rather than treating needs as isolated UI bars.  
3. **Lawful acquisition/discovery chains.** Food, water, latrines, washbasins, sleep sites, and safe routes must be discovered through perception, beliefs, testimony, records, or local observation. No global map, no hidden target injection, no scenario rescue.  
4. **Time as the shared scarce currency.** Every self-care action competes with every other need and adjacent pressure. Don’t Starve, Raft, and The Long Dark all use time/resource pressure to make maintenance decisions consequential: hunger/thirst are not interesting by themselves, but become interesting when they consume time that could have been used for shelter, exploration, defense, or route planning.  
5. **Explicit contested affordances.** Shared washbasins, wells, latrines, beds, sleep spaces, and narrow routes cannot be “locked by intent.” They must be occupied, released, abandoned, or granted by explicit world processes. This is a direct Foundations requirement.  
6. **Failure leaves state.** Interruption cannot be an invisible reset. If sleep was interrupted, the accumulated recovery and wake reason must be visible. If Wash was interrupted, the basin must not remain ghost-occupied. If repeated interruptions prevent recovery, the path to collapse/death must be visible in unmet needs, action traces, failed starts/aborts, and decision traces.  
7. **Player/AI symmetry with bounded interface.** The human-controlled agent and AI-controlled agents must obey identical preconditions, costs, occupancy, interruption, and consequences. The UI may summarize lawful perception and memory; it must not expose omniscient facility/resource truth.

The useful comparative lesson from GOAP-style agents is not “copy F.E.A.R.” It is that emergent AI feels intelligent when agents compose actions from preconditions/effects and replan from changing world state instead of relying on hard-coded transitions. Worldwake’s self-care should work the same way: interruption recovery should be ordinary planning over changed state, not a special “rescue” script.

---

# **5. Gap Analysis**

## **5.1 Wash/travel planning under discovery and budget pressure**

**Evidence.** Wash is a real goal and real action: the schema gives Wash self-care budget and `[Wash, Travel]` ops, candidate generation emits Wash, the planner classifies `wash`, and the action requires a local WashBasin with clean water.

**Gap.** The long-running scattered and contested tests explicitly exclude Wash from budget-exhaustion assertions. The comments say Wash can exhaust budget before the agent discovers the WashBasin. That means Wash is currently not collision-proven under exactly the discovery/travel/budget condition that would make it meaningful.

**Design verdict.** The clean outcome is not to downgrade Wash into ignorable hygiene. The clean outcome is to make Wash a lawful self-care target with bounded discovery and travel planning equivalent to the other survival needs.

## **5.2 Self-care interruption and recovery**

**Evidence.** The action engine can interrupt or abort active actions, call `on_abort`, release reservations, emit `ActionInterrupted`/`ActionAborted`, and request replanning.

**Gap.** In Cluster 1, only sleep has meaningful partial progress and abort aftermath. Eat, Drink, Toilet, Wilderness Relief, and Wash all currently use `abort_noop`; Wash has commit-time partial relief only for insufficient clean water, not interruption progress.

**Design verdict.** Not every action needs partial progress. But every action needs an explicit interruption contract: what state changes before commit, what state changes at commit, what cleanup happens on abort, and what recovery planning can lawfully observe.

## **5.3 Facility contention for Wash/latrine/sleep**

**Evidence.** Facility queue machinery exists and is scenario-backed for survival-trade water harvesting. It supports queueing, grants, pruning invalid waiters, grant expiry, active exclusive action detection, and contention events.

**Gap.** Current queue classification recognizes production Harvest/Craft, Corpse, and Care-style exclusivity, but not Wash, Toilet, Wilderness Relief, or Sleep. Needs actions also have no reservation requirements.

**Design verdict.** The queue system should be reused only if self-care facilities can be represented without lying about their domain. If not, the first pass should introduce a minimal self-care occupancy substrate rather than smuggling needs through production queue semantics.

## **5.4 Partial progress and abort aftermath**

**Evidence.** Sleep already accumulates recovery per tick and ends with a wake reason. Wash can partially reduce dirtiness at commit if clean basin water is insufficient. Toilet and wilderness relief have strong commit aftermath but no partial or abort aftermath. Eat/drink consume one unit at commit.

**Gap.** There is no general self-care “started use” state except sleep. If a Wash or Toilet action becomes occupant-exclusive before commit, abort must clean the occupancy. If no occupancy exists, contention remains imaginary.

**Design verdict.** First pass: keep Eat/Drink/Toilet/Wilderness Relief atomic unless a real state carrier is introduced. Sleep partial stays. Wash partial-on-commit stays. Do not invent partial bodily-progress math unless it leaves stable world state and trace.

## **5.5 Scarcity/degradation**

**Evidence.** Wash consumes basin clean water and increases basin dirtiness; latrine relief can increase latrine fullness and place dirtiness; wilderness relief creates waste and dirtiness; item decay and carried waste have separate scenario proof.

**Gap.** Scarcity is not yet collision-proven for Wash under travel/discovery/budget and facility contention. Wash currently has stateful water use, but scenario proof is weaker than the implementation.

## **5.6 Collapse/death traceability**

**Evidence.** The needs system can create deprivation wounds and death; the simulation-gaps golden proves sustained unmet hunger creates deprivation wounds, fatal wound load, `DeadAt` with need-deprivation cause, `EventTag::Death`, and no post-death actions.

**Gap.** That proof is not yet tied to repeated interrupted self-care recovery. A repeated-interruption death must show failed lawful recovery opportunities, not just a final death tag.

## **5.7 Player/AI symmetry and UI implications**

**Evidence.** Foundations require player/AI simulation symmetry and allow UI differences only in presentation of what the controlled agent can lawfully perceive, infer, remember, or access.

**Gap.** Human-facing UI must not display remote washbasin availability, queue state, or clean water unless the controlled agent has lawful knowledge. The drive-escalation belief-only Wash test already proves no remote Wash knowledge in one setup, and this should become a general player/AI contract.

## **5.8 Adjacent-cluster collision seams**

Adjacent pressure should be used only as validation pressure: obligation, pursuit, hostile presence, patrol duty, social interruption, or resource race can interrupt self-care. The scope must not expand into redesigning those systems.

---

# **6. Prioritized Proposal**

## **P0 — must fix now**

**P0.1 Close Wash budget/exhaustion gap.** Wash must be included in survival budget-exhaustion assertions in scattered/contested-like scenarios. If the current planner cannot reliably find Wash under lawful discovery/travel, the future spec must change the planner/belief/candidate path, not the definition of survival.

**P0.2 Define self-care interruption contracts by action family.** Every Cluster 1 action must declare start state, tick/partial behavior, commit behavior, abort cleanup, recovery visibility, and trace requirements.

**P0.3 Add minimal self-care occupancy for contested facilities.** WashBasin, latrine/toilet, and any scarce sleep surface must have explicit occupy/release/abandon/interrupted cleanup. Queue/grant/reservation should be reused only where current architecture can honestly model the facility.

**P0.4 Add scenario-backed proof for interrupted self-care recovery.** The proof must show lawful interruption, cleanup, replanning, eventual recovery or lawful degradation, and no hidden rescue.

**P0.5 Add trace assertions that prove the causal branch.** “Agent survived” is insufficient. Assertions must show the target action, interruption event, abort cleanup, recovery candidate, lawful knowledge source, action completion, need relief, and no budget escape.

## **P1 — should include if naturally required**

**P1.1 Generalize contention queue to Needs actions.** If Wash/latrine/sleep need waiting behavior, extend the existing queue model to classify Needs exclusive facility use instead of building a parallel queue.

**P1.2 Add profile-driven self-care patience thresholds.** Agents should not wait forever for a blocked basin/latrine/sleep surface when critical needs are worsening. Thresholds should come from profiles and action/need severity, not scenario scripts.

**P1.3 Add recovery memory for interrupted self-care.** A typed memory or trace-derived blocker can help avoid retrying a currently occupied/broken/dirty/dry target without global truth.

**P1.4 Improve player-facing self-care legibility.** Show lawful local/believed reasons: “needs wash,” “knows basin at Spring,” “basin occupied,” “interrupted by attacker,” “replanning to latrine,” “too tired to continue route.” Do not show hidden truth.

## **P2 — defer**

**P2.1 Disease, infection, odor, social shame, privacy, etiquette, bathroom politics.** These are not required for the first collision-proof pass.

**P2.2 Complex sanitation economy.** Waste/disposal/item decay already exist; do not turn this pass into a full sanitation/trash/maintenance sim.

**P2.3 Full shelter redesign.** Sleep surfaces may need occupancy; broad shelter safety/comfort ecology can wait.

**P2.4 Full adjacent cluster redesign.** Pursuit, obligation, trade, theft, justice, combat, and escort should be used only as pressure sources.

## **Suggested state/component classification**

| Suggested type | Classification | Recommendation |
| ----- | ----- | ----- |
| `SelfCareOccupancy` / generalized `OccupiedBy` | Authoritative world state | P0 if facility contention is added. |
| `SelfCareUseKind` enum: Wash, LatrineRelief, SleepSurface | Authoritative support type | P0 if occupancy is generalized. |
| `WashSessionProgress` | Authoritative world state | P1 only if duration-based partial Wash is adopted. |
| `SleepSurface` / `SleepSlot` | Authoritative world state | P1 if scarce sleep surfaces matter; otherwise use place-level sleep quality. |
| `SelfCareAffordanceView` | Derived read model | P0/P1 as planner/UI convenience only; never truth authority. |
| `SelfCareInterrupted` / `SelfCareRecoveryTrace` | Event/trace only | P0. Useful proof surface. |
| “Survival score” | Not recommended | Violates concrete-state preference. |
| Hidden “rescue agent” / hidden target injection | Must not exist | Violates Foundations. |

---

# **7. Target 1 Proposal — Wash Under Discovery and Budget Pressure**

## **Requirements**

Wash MUST become a first-class survival need under the same planning, travel, discovery, and budget accounting as Eat/Drink/Sleep/Relieve.

Concretely:

* Wash MUST remain `GoalPlanningBudget::SELF_CARE`.  
* Wash MUST remain relevant to `Wash` and `Travel`.  
* Wash MUST be included in budget-exhaustion assertions.  
* Wash MUST NOT be carved out of scattered/contested survival budget tests.  
* Wash target enumeration MUST use lawful beliefs, local observation, or discovered affordances only.  
* Wash MUST NOT use global facility truth unless the agent is in a debug/test-only omniscient mode that is explicitly outside normal AI/player planning.  
* Wash planning MUST handle at least:  
  * known local WashBasin;  
  * known remote WashBasin with route;  
  * no known WashBasin but plausible exploration hypothesis;  
  * known WashBasin with insufficient clean water;  
  * known WashBasin occupied/contended;  
  * route too expensive or planner budget exhausted.

## **Mechanics**

Current Wash commit mechanics are sound enough to keep: local target, WashBasin tag, clean water requirement, duration from metabolism profile, clean water consumption, dirtiness reduction, basin dirtiness increase, partial commit relief if insufficient clean water.

The missing mechanics are not in commit; they are in **pre-commit search and collision**:

1. **Belief-backed target discovery.** A wash candidate can be generated only from a known/believed WashBasin or same-tick local observation. If no known WashBasin exists, the agent may generate exploration toward a place with a lawful `MayContainWashBasin` hypothesis.  
2. **Travel-to-Wash must be charged.** Travel steps toward a WashBasin must consume the same search and duration budget as other self-care travel. Wash cannot be evaluated “out of band.”  
3. **Wash water availability must be represented as an expectation, not omniscience.** If the basin is believed to have clean water, the plan may proceed; if start/commit disproves it, the failure must generate a discrepancy/blocker/recovery trace.  
4. **Dry/dirty/occupied basin recovery.** If the basin is unusable, the agent may:  
   * select another known basin;  
   * acquire/harvest water if current architecture supports filling/using basin water lawfully;  
   * explore for another wash affordance;  
   * defer Wash if another need is more critical;  
   * suffer rising dirtiness if recovery fails.

## **Planner/belief implications**

The evidence says the architecture already sees Wash as a self-care goal and relevant to Travel. The future spec must audit the exact `emit_wash_goal` target enumeration helper and its interaction with evidence places, travel horizon, and blocked self-care exploration. I did not fetch every helper body, so the exact code path remains an implementation verification item.

Wash should use the same epistemic standard proven by the drive-escalation belief-only regression: remote WashBasin truth must not produce Wash plans if the agent lacks lawful belief.

## **Facility/cleanliness/contention implications**

WashBasin already carries clean/dirty water state. It needs explicit contention if multiple agents can use it. The minimum model is:

* occupy basin at action start or grant;  
* release on commit;  
* release on abort/interruption;  
* abandon/timeout cleanup if actor dies/leaves/place invalidates;  
* trace occupant and reason.

Do not introduce social norms. The first Wash contention proof is mechanical: two dirty agents, one basin, one gets it, the other waits/replans/uses alternate route or suffers delay.

## **Trace/event implications**

A successful Wash-under-pressure proof must expose:

* selected Wash goal with Drive provenance;  
* candidate evidence from lawful belief/local observation;  
* planned route or local action;  
* planner budget result including Wash;  
* action start for travel and/or wash;  
* basin occupancy/grant if contested;  
* commit event with water consumed and dirtiness delta;  
* no hidden rescue or global target injection.

## **Scenario/golden proof**

Add or modify a scattered/contested-like scenario so that:

* WashBasin is not co-located with all other resources.  
* At least one agent must travel or discover the basin.  
* Planner budget is tight enough that an omitted Wash check would be caught.  
* Survival-health contract includes Wash.  
* Assertion fails if Wash is excluded from budget checks.  
* Assertion proves either:  
  * Wash found and completed within budget from lawful belief/discovery; or  
  * Wash budget exhaustion is traceable and recovery/exploration follows lawfully.

## **Alternatives rejected**

**Reject “Wash is hygiene, not survival.”** Existing repo evidence treats Dirtiness as a homeostatic need, drive-escalated, action-backed, and required in multiple survival contracts.

**Reject “leave Wash excluded until later.”** That preserves the known seam and makes Cluster 1 look complete while one of its five needs bypasses the hardest proof.

**Reject “seed all agents with WashBasin knowledge.”** That would hide the problem. Discovery pressure is the point.

---

# **8. Target 2 Proposal — Interruption and Recovery**

## **General interruption requirements**

Every self-care action MUST declare:

* authoritative state before start;  
* state written at start, if any;  
* tick/partial effects;  
* commit effects;  
* abort/interruption effects;  
* cleanup of occupancy/reservation/session state;  
* recovery-visible facts;  
* event/action/decision traces;  
* whether repeated disruption can lead to lawful collapse.

The engine already supports interruption, abort, action traces, reservation release, and replanning. The gap is action-family semantics.

## **Eat**

**Current model.** Eat commits atomically: consumes one unit, reduces hunger, and applies bladder fill. Abort is no-op.

**Requirement.** Eat MUST remain no-partial unless the item/portion system supports partial consumption with source/sink lineage. Interrupted Eat before commit MUST consume nothing and relieve nothing. Recovery MUST revalidate target control and quantity; if the item is gone or no longer controlled, the agent MUST replan toward another consumption/acquisition path.

**Trace proof.** ActionStarted `eat`, ActionInterrupted/Aborted, no item quantity delta, later ActionCommitted `eat` or lawful failure/acquisition.

## **Drink**

Same as Eat. Interrupted Drink before commit consumes nothing and relieves nothing. Recovery revalidates water possession/control and replans if water is gone.

## **Sleep**

**Current model.** Sleep already has start state, tick recovery, accumulated recovery, abort ending with LocalDisturbance, and commit ending with accumulated recovery.

**Requirement.** Preserve and deepen, do not replace. Interrupted sleep MUST preserve accumulated recovery, end the episode with a reason, release any sleep surface occupancy, and make the agent re-evaluate whether to resume sleep or address a now-more-critical need.

**Trace proof.** SleepEpisodeStarted, several fatigue reductions, interruption, SleepEpisodeEnded with LocalDisturbance and accumulated recovery, later sleep resumption or different need priority justified by current state.

## **Toilet / Latrine relief**

**Current model.** Toilet requires actor at a latrine-tagged place, clears bladder, creates Waste, updates latrine fullness, and can increase place dirtiness. Abort is no-op.

**Requirement.** First pass: no partial bodily progress. Interrupted before commit clears nothing and creates no Waste. If latrine occupancy is added, start MUST occupy and abort MUST release. Recovery MUST choose same latrine if still available, another known latrine, wilderness relief if legal and pressure is high, or suffer rising bladder.

**Tasteful handling.** Avoid lurid detail. Mechanically, this is “relief action interrupted; bladder unchanged; facility released; agent replans.”

## **Wilderness relief**

**Current model.** Requires outdoor relief tags, creates Waste/evidence, clears bladder, increases actor dirtiness and place dirtiness. Abort is no-op.

**Requirement.** First pass: no partial. Interrupted before commit clears nothing and creates no Waste. Because wilderness relief is less facility-bound, occupancy is optional unless wilderness relief spots become explicit scarce affordances. Recovery can retry wilderness relief, travel to latrine, or suffer bladder consequences.

## **Wash**

**Current model.** Wash requires local WashBasin with clean water, reduces dirtiness, consumes clean water, dirties basin, and can be partial at commit if insufficient clean water. Abort is no-op.

**Requirement.** First pass: preserve commit-time partial Wash, but do not add duration-based partial progress unless a `WashSessionProgress` state is introduced. Interrupted Wash before commit should normally consume no water and reduce no dirtiness. If the basin is occupied at start, abort MUST release occupancy. Recovery MUST replan from observed current basin state: same basin, another basin, water acquisition/refill if lawful, exploration, or defer under more critical need.

**Why not duration-partial Wash now?** Partial Wash can be interesting, but without a durable session/progress carrier it becomes invisible arithmetic. Foundations prefer state carriers of consequence, not decorative realism.

## **Repeated interruption leading to collapse/death**

Repeated severe disruption MAY lawfully lead to collapse or death, but only if:

* the unmet need remains authoritative and rising;  
* recovery opportunities were generated and failed lawfully;  
* interruptions are ordinary world events, not test scripts;  
* event log/action trace/decision trace show the chain;  
* death/collapse state records cause and prior deprivation exposure.

The existing death-traceability golden proves deprivation death can be durable and traceable for hunger; this needs a new collision proof where repeated interruption is part of the causal chain.

---

# **9. Minimal Facility Contention Model**

The first pass should model **mechanical exclusivity**, not social behavior.

## **Required operations**

For WashBasin, latrine/toilet facility, and scarce sleep surface:

* **Occupy:** actor begins using the affordance.  
* **Release:** action commits normally.  
* **Abandon:** actor leaves/dies/invalidates use.  
* **Interrupted cleanup:** action aborts/interruption releases occupancy and records reason.  
* **Queue/grant/reservation:** only if existing architecture can model it honestly.

## **Reuse existing queue when appropriate**

The existing queue/grant system is valuable: it already has grants, expiry, pruning, active exclusive action checks, wait observations, and contention events.

But it currently recognizes production Harvest/Craft-style workstation exclusivity and other non-needs kinds, not self-care Needs actions. Therefore:

* Reuse it if `promotable_contention_kind` / exclusive classification can be generalized to Needs without mislabeling.  
* Do not fake Wash as production just to get a queue.  
* Do not create implicit planner locks. Intent is not entitlement; only explicit occupancy/grant/reservation counts.

## **When not to queue**

Do not queue wilderness relief unless the map models specific scarce relief affordances. A wide outdoor area should not require a bathroom line. If wilderness relief becomes location-sensitive for privacy/social reasons later, that belongs to a later cluster.

## **Sleep surfaces**

If sleep remains place-level, contention can be place capacity or omitted in the first pass. If sleep surfaces matter downstream, they need identity and occupancy. Do not invent “bed politics”; just prove two tired agents cannot occupy one cot without explicit resolution.

---

# **10. Scenario Validation Plan**

Scenario proof should move Cluster 1 toward collision-proven coverage. The order of proof strength remains:

1. Implementation only.  
2. Regular golden coverage.  
3. Long-running scenario-backed coverage.  
4. Registered roadmap coverage.  
5. Collision-proven coverage.

## **Scenario A — Wash budget closure in scattered survival**

**Shape.** Modify or add a `survival-scattered-wash-budget` scenario derived from `survival-scattered.ron`.

**Pressure.**

* Food, water, latrine, and washbasin separated.  
* WashBasin not initially local.  
* Agent has high dirtiness and incomplete route/resource knowledge.  
* Travel costs matter.  
* Planner budget is bounded.

**Assertions.**

* Wash is included in survival budget-exhaustion checks.  
* No Wash-specific test exemption.  
* If Wash plan is found: trace shows lawful evidence, route, action start, commit, dirtiness reduction.  
* If budget exhausts: trace shows Wash budget exhausted, recovery/exploration follows, and scenario expectation is explicitly authored as lawful failure/recovery.  
* Survival-health contract includes Wash.

## **Scenario B — Wash under contested basin/resource pressure**

**Shape.** Extend contested-like scenario or add a sibling.

**Pressure.**

* Two or more dirty agents.  
* One WashBasin with finite clean water.  
* One alternate basin or water refill path optional.  
* Shared route choke or shared well.

**Assertions.**

* Only one actor occupies basin at a time.  
* Losing actor waits/replans lawfully.  
* Basin clean water and dirtiness deltas are traceable.  
* Wash no longer bypasses planner budget checks.

## **Scenario C — Interrupted self-care recovery**

**Shape.** One or two agents; ordinary adjacent pressure interrupts self-care. Use minimal adjacent trigger such as hostile danger, urgent obligation, or local disturbance. Do not script rescue.

**Branches to prove.**

* Eat interrupted before commit, no consumption, later retry/acquire.  
* Drink interrupted before commit, no water consumed, later retry/acquire.  
* Sleep interrupted after partial recovery, episode ends with recovery preserved, later resume or reprioritize.  
* Toilet interrupted before commit, no Waste/no bladder relief, facility released, later relief.  
* Wilderness relief interrupted before commit, no Waste/no relief, later relief or latrine.  
* Wash interrupted before commit, basin released, later Wash completed or lawful failure.

**Assertions.**

Not just “agent survived.” The trace must show exact action family, interruption reason, cleanup, recovery planning, and final consequence.

## **Scenario D — Repeated interruption can lawfully collapse**

**Shape.** A harsher scenario, likely ignored CI-only.

**Pressure.**

* Repeated interruptions prevent one critical need from being satisfied.  
* There are lawful recovery opportunities.  
* Eventually deprivation wounds/collapse/death occur if recovery fails.

**Assertions.**

* Deprivation exposure increases.  
* Recovery attempts are visible.  
* Failed actions are traceable.  
* Death/collapse cause is tied to the unmet need.  
* No actions start after death.  
* Deterministic replay/state hash remains stable.

## **Scenario E — Player/AI symmetry smoke proof**

**Shape.** Same scenario with controlled human-agent command and autonomous AI command path.

**Assertions.**

* Same preconditions and legality.  
* Same failure if commanded to use unknown/remote/occupied facility.  
* UI shows only lawful belief/local observation.

---

# **11. Spec-Ready Requirements**

## **11.1 Global Cluster 1 requirements**

**MUST**

* Cluster 1 MUST treat Hunger, Thirst, Fatigue, Bladder, and Dirtiness as one embodied survival loop.  
* Every self-care action MUST have explicit preconditions, duration, cost, interruption behavior, commit behavior, abort behavior, and aftermath.  
* Every consequential facility/item/waste/wound/death/consequence MUST have stable identity or explicit event lineage where downstream systems care.  
* Planning inputs MUST be belief-backed, local, self-knowledge, or lawful public/artifact knowledge.  
* AI and player-controlled agents MUST obey identical simulation laws.  
* Scenario assertions MUST prove causal branches, not merely broad survival.

**SHOULD**

* Profile thresholds should govern urgency, patience, recovery retry, and acceptable waiting.  
* Trace surfaces should expose action family, need state, target facility, route evidence, occupancy state, interruption reason, cleanup, recovery, and outcome.  
* UI should summarize lawful reasons after the fact: “why did this agent not wash?” should be answerable.

**MUST NOT**

* No hidden rescue.  
* No global facility/resource truth as planning knowledge.  
* No scenario-specific target injection.  
* No abstract survival score as authority.  
* No implicit planner locks.  
* No social etiquette/privacy systems in this pass unless required by active mechanics.

## **11.2 Wash requirements**

**MUST**

* Wash MUST be included in budget-exhaustion checks for survival goals.  
* Wash MUST use self-care planning budget.  
* Wash MUST be planned through lawful local/belief-backed affordances.  
* Wash MUST charge travel search/duration cost.  
* Wash MUST fail visibly if target basin is gone, dry, occupied, unreachable, or belief-disconfirmed.  
* Wash interruption MUST release any occupied basin.  
* Wash commit MUST emit/retain water consumed, dirtiness delta, basin delta, and partial flag.

**Acceptance criteria**

* A scattered-like golden fails if Wash is omitted from budget-exhaustion checks.  
* At least one agent completes Wash after remote discovery/travel without global knowledge.  
* At least one trace shows Wash candidate evidence source.  
* A belief-only regression proves no Wash plan appears for unknown remote WashBasin.  
* A contested Wash proof shows basin occupancy/release and no simultaneous use.

**Likely systems/files affected by a future implementation spec**

* `crates/worldwake-ai/src/candidate_generation.rs`  
* `crates/worldwake-ai/src/goal_schema.rs`  
* `crates/worldwake-ai/src/goal_model.rs`  
* `crates/worldwake-ai/src/search/transition.rs`  
* `crates/worldwake-systems/src/needs_actions.rs`  
* `crates/worldwake-systems/src/facility_queue.rs`  
* `crates/worldwake-systems/src/facility_queue_actions.rs`  
* survival scenario tests and scenario RON files

## **11.3 Interruption/recovery requirements**

**MUST**

* Interrupted self-care MUST produce an action trace event.  
* Abort cleanup MUST release occupancy/reservations/session state.  
* Recovery MUST arise from ordinary planning.  
* Eat/Drink MUST NOT partially consume unless item portions support it cleanly.  
* Sleep MUST preserve partial recovery.  
* Toilet/Wilderness Relief MUST either be atomic no-partial before commit or introduce explicit partial state.  
* Wash MUST either be atomic no-partial before commit or introduce explicit `WashSessionProgress`.  
* Repeated severe interruption MAY lead to collapse/death only through traceable unmet needs and failed lawful recovery.

**Acceptance criteria**

* Interrupted Sleep proof shows accumulated recovery and wake reason.  
* Interrupted Wash proof shows basin released and later reacquired/alternate selected.  
* Interrupted Toilet proof shows no bladder relief/no Waste before commit and later relief/recovery.  
* Interrupted Eat/Drink proof shows no item consumption before commit and later retry/acquisition.  
* Repeated interruption proof shows deprivation path and no post-death actions if death occurs.

## **11.4 Facility contention requirements**

**MUST**

* Shared WashBasin use MUST be mutually exclusive.  
* Latrine/toilet use MUST be mutually exclusive where a concrete facility is modeled.  
* Sleep surfaces MUST be mutually exclusive if surfaces are modeled as scarce entities.  
* Occupancy MUST be authoritative state or an explicit existing queue/grant/reservation state.  
* Abandon/interruption cleanup MUST be deterministic and traceable.

**SHOULD**

* Reuse existing queue/grant machinery where it can classify Needs actions honestly.  
* Add a minimal self-care occupancy model if queue generalization is too large.

**MUST NOT**

* Do not use planner intent as lock.  
* Do not add etiquette/privacy/social reactions in this pass.

## **11.5 Profile-driven parameter guidance**

Thresholds should come from scenario/profile data, not hard-coded scenario rescue:

* self-care patience before abandoning occupied facility;  
* maximum retry count or cooldown for a disconfirmed facility;  
* critical need urgency margin for interrupting non-critical actions;  
* minimum sleep ticks before voluntary wake if recovery is partial;  
* Wash clean-water minimum and partial relief curve;  
* travel horizon and prerequisite-location caps for self-care goals.

## **11.6 Non-goals**

* No implementation PR.  
* No implementation tickets.  
* No broad economy/trade/theft/justice/combat redesign.  
* No disease/odor/social shame.  
* No omniscient UI.  
* No rescue scripts.

---

# **12. Risks, Tradeoffs, and Open Questions**

## **Design risks**

**Risk: Wash becomes busywork.** If Wash is just another bar to clear, it will feel like chores. It becomes compelling only when dirtiness interacts with relief, water, basin state, travel, facility contention, time pressure, and recovery from interruption.

**Risk: interruption becomes frustration.** Interruption should usually be survivable friction. The design must make recovery legible and fair: what happened, what state changed, what can the agent do now?

**Risk: over-modeling bodily functions.** Toilet/wilderness relief should remain tasteful and mechanical. Waste, dirtiness, place dirtiness, latrine fullness, and recovery are enough.

**Risk: queue generalization gets too big.** Existing queue machinery is attractive, but forcing self-care through production semantics would be worse than a minimal occupancy state.

## **Implementation unknowns to verify before coding**

* Exact Wash target enumeration helper and how it handles evidence places, beliefs, travel horizon, and blocked self-care exploration.  
* Whether WashBasin clean-water belief is available in planner snapshots or only authoritative action preconditions.  
* Whether existing effect schemas can model self-care occupancy as hypothetical effects without destabilizing planning.  
* Whether `QueueForFacilityUse` should become relevant/progress-barrier for Wash/Relieve/Sleep or whether occupancy is action-start-only.  
* Whether player UI already has enough belief-view boundary enforcement for self-care facilities.  
* Whether sleep surfaces exist implicitly, as place quality, or need explicit identity.

## **Evidence uncertainty**

I fetched the active core files needed for this proposal, but I did not inspect every candidate-generation helper body. The evidence is sufficient to identify the Wash budget proof gap and action interruption asymmetry; a future implementation spec must audit exact helper-level behavior before code changes.

---

# **13. Final Recommendation**

The next spec/theme should be:

**C1 Embodied Self-Care Collision Proof: Wash Planning + Interruptible Recovery**

It should be **one spec**, not multiple disconnected specs, because the same architectural principle binds the work: self-care must be a lawful embodied action loop under bounded knowledge, time, facility contention, interruption, and traceable consequence.

The best first implementation slice is:

**Slice 1: Wash budget closure and proof.**  
 Remove the Wash carve-out from survival budget-exhaustion validation, then make the smallest lawful planner/belief/scenario changes necessary for Wash to pass as a first-class survival goal under scattered/contested-like discovery and travel pressure.

The second slice is:

**Slice 2: self-care interruption recovery and minimal occupancy.**  
 Define action-family interruption semantics, add occupancy/release/abandon cleanup for self-care facilities where needed, and prove interrupted Eat/Drink/Sleep/Relieve/Wilderness Relief/Wash recovery with trace assertions.

Do not let this drift into hygiene flavor. The thing that will make Cluster 1 feel complete is not more meters; it is **situated prioritization under imperfect knowledge**, with every survival outcome explainable after the fact through world state, beliefs, actions, interruptions, recovery attempts, and consequences.

