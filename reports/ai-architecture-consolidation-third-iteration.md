# **Third-Iteration Hostile AI Architecture Audit — Worldwake**

I followed the uploaded mission brief as the governing prompt.

## **1. Executive Verdict**

**Verdict: Worldwake is cleaner than the second audit implied, but belief-backed simulation is still not proven safe.** The dangerous part is not the action dispatcher. The dangerous part is that `PerAgentBeliefView` is still a `World` wrapper with a very broad `RuntimeBeliefView`/`GoalBeliefView` surface, and several belief-looking methods still read current authoritative world facts for control, rights, institutional data, possession/container state, reservations, queues, extraction slots, social/legal facts, and debug/action explanation surfaces. `docs/FOUNDATIONS.md` makes belief/world separation and player/AI symmetry non-negotiable; the current code is only partly enforcing that boundary.

Severity ratings:

| Severity | Finding |
| ----- | ----- |
| **Critical** | FND-14/FND-14B is not type-level enforced. `PerAgentBeliefView` holds `&World`; many “belief” APIs perform method-local gates, and several gates are too coarse or absent. |
| **Critical** | Planning snapshots use **entity-level** `AdmissionSource`, but store many independently sourced fields. This cannot prove field-level lawful provenance. |
| **High** | Rights/control, office/record data, loyalty, reward encumbrance, stock-storage policy, extraction-slot claimability, reservation ranges, and some queue data can leak authoritative truth into AI or player-facing affordance/planning surfaces. |
| **High** | CLI action listing uses the same affordance path as AI, which is good, but CLI display/control helpers are omniscient meta/debug surfaces. Future normal player UI must not inherit them. |
| **Medium** | HTN has improved. It is now mostly honest method-guided strategic search, not fake enforced HTN. But calling it “HTN” remains slightly over-strong unless every method trace and fallback contract is explicit. |
| **Medium** | Candidate/ranking/portfolio systems are more consolidated than prior reports suggested, but candidate generation still bundles emission, diagnostics, omission classification, learned-memory scheduling, and discrepancy-adjacent behavior. |
| **Medium** | Repair is a controlled placeholder, not a finished local repair system. `InsertVerification` has no substrate, and `EventId(0)` causal placeholders weaken causal-history claims. |
| **Low** | The authoritative action runtime is relatively clean: affordance enumeration is belief-facing, start/commit are authoritative, and recoverable revalidation failures abort and replan rather than crash. |

The architecture should **not** receive more AI behavior until belief-source enforcement is consolidated. The right move is **moderate consolidation**, not a full rewrite.

---

## **2. Repository Discovery Log**

I used GitHub/code search and targeted file fetches only. I did **not** clone the repository. Archive files surfaced in search results, but I did not open, fetch, or rely on any archived file.

### **Active docs/contracts inspected**

| File | Used for |
| ----- | ----- |
| `docs/FOUNDATIONS.md` | Non-negotiable design constitution: local causality, belief/world separation, player symmetry, explicit action legality, no fossils, traces, validation. |
| `docs/planner-contracts.md` | Planner-visible authority contract, FND-14B, HTN method hint policy, belief-backed planning obligations. |
| `docs/spec-drafting-rules.md` | Source-class requirements for specs, belief-view accessors, HTN drafting rules. |
| `AGENTS.md` | Repository operating rules: no player type, `ControlSource`, no backward-compatibility fossils, AI impact rule. |
| `.claude/skills/goap-architecture-report/SKILL.md` | Reporting expectations for AI architecture review. |

### **Active reports inspected**

| File | Trust level |
| ----- | ----- |
| `reports/ai-architecture-consolidation-second-iteration.md` | Useful prior hypothesis, not authority. Several issues it named appear partly fixed in current code; several remain in new form. |

### **Active code inspected**

| Area | Files |
| ----- | ----- |
| Belief/runtime views | `crates/worldwake-sim/src/per_agent_belief_view.rs`, `crates/worldwake-sim/src/belief_view.rs` |
| Planning snapshot/state | `crates/worldwake-ai/src/planning_snapshot.rs`, `crates/worldwake-ai/src/planning_state.rs` |
| Candidate/ranking/portfolio | `candidate_generation.rs`, `goal_schema.rs`, `ranking.rs`, `agent_tick/portfolio.rs` |
| HTN/search | `htn/method_schema.rs`, `htn/methods.rs`, `htn/registry.rs`, `htn/selector.rs`, `search/strategic.rs` |
| Action legality/runtime | `action_handler.rs`, `tick_action.rs`, `tick_step.rs`, `plan_revalidation.rs` |
| Intentions/repair | `agent_tick/frame.rs`, `plan_repair.rs` |
| Player/CLI | `crates/worldwake-cli/src/handlers/actions.rs`, `handlers/control.rs`, `display.rs` |
| Diagnostics | `decision_trace.rs`, `action_trace.rs` |
| Core institutional data | `crates/worldwake-core/src/offices.rs` |

### **Active tests inspected**

| File | Notes |
| ----- | ----- |
| `crates/worldwake-ai/tests/scenarios/ai_decisions.rs` | Strong for goal invalidation, needs, exhaustion isolation, regeneration, interrupt behavior; weak for adversarial FND-14/player symmetry. |
| CLI action/control tests in `handlers/actions.rs` and `handlers/control.rs` | Basic behavior tests, not belief-boundary tests. |
| HTN registry tests | Strongly prove current methods are StageHint-only and reject required leaves for now. |

### **Areas not inspected sufficiently**

I did not fully inspect every action definition, every world component, every scenario helper, every system registration, or every golden test. I also did not run the test suite. The audit is still strong enough to identify release-blocking boundary issues because the dangerous seams are in central surfaces: `RuntimeBeliefView`, `PerAgentBeliefView`, `PlanningSnapshot`, `PlanningState`, CLI display/control, and traces.

---

## **3. Research Synthesis**

### **GOAP in games**

F.E.A.R.-style GOAP is commonly described as STRIPS-like goal/action planning where NPCs select goals and plan action sequences from preconditions/effects rather than hand-coded transitions. The important lesson for Worldwake is not “copy F.E.A.R.” It is that GOAP only earns its keep when action preconditions/effects remain the authority and sensors/beliefs feed the planner lawfully. Worldwake’s action runtime aligns with that idea; the belief-source boundary does not yet prove it.

What it implies: keep GOAP as the ordinary action-sequence planner.

What it does **not** imply: GOAP may not read `World` truth just because it is cheaper or more convenient.

Why it matters to live code: `get_affordances`, planning snapshots, and `PlanningState` are the Worldwake analog of the “sensor/world-knowledge” seam; that seam is currently too broad.

### **HTN / SHOP-style planning**

HTN planning decomposes compound tasks into simpler tasks until executable primitive tasks are obtained; SHOP2 is an HTN planning system known for ordered task decomposition and competition performance.

What it implies: HTN methods should be real decomposition authority only if method leaves are enforced primitive tasks and method preconditions are lawful.

What it does **not** imply: a list of stage hints is full HTN.

Why it matters to live code: Worldwake’s current HTN layer is method-guided strategic search: all current subgoals are `StageHint`, registry tests forbid `RequiredActionLeaf`, and strategic search falls back to generic stages. That is fine, but it must be named and documented honestly.

### **BDI and intention management**

BDI separates beliefs, desires, and intentions; its practical value is balancing deliberation with execution of currently active plans. It also does not automatically enforce private beliefs or correct communication.

What it implies: Worldwake’s `IntentionFrame` direction is correct: intentions should persist, suspend, resume, and abandon under explicit conditions.

What it does **not** imply: an intention gets privileged future access to resources or hidden reservations.

Why it matters to live code: `agent_tick/frame.rs` has the right shape, but it still depends on broad `RuntimeBeliefView` access and uses placeholder causal IDs in some memory records.

### **Utility AI / ranking**

Utility systems score candidate actions or behaviors from current inputs. They are useful for arbitration, but score functions can turn into “score soup” if they replace concrete state, source provenance, and action legality.

What it implies: Worldwake can use motive scores, priority classes, source reliability, and portfolio weights as arbitration.

What it does **not** imply: a high score can substitute for lawful knowledge, preconditions, or concrete resources.

Why it matters to live code: `ranking.rs` has a single ordering contract and provenance, which is good; the remaining risk is that candidate generation/ranking becomes the semantic owner of facts that should be owned by belief, world state, or actions.

### **Multi-agent coordination, reservations, queues**

Contract-net-like protocols allocate tasks through explicit proposal/award messages, and MAPF research emphasizes that multi-agent plans need explicit collision/contention assumptions and constraints.

What it implies: queues, grants, reservations, and contention should be explicit world state with explicit carriers of knowledge.

What it does **not** imply: agents may inspect remote reservation tables because the table exists.

Why it matters to live code: queue/grant methods are partly local-gated, but extraction claimability and reservation ranges/conflicts still read authoritative world state in belief-facing surfaces.

### **Explainable planning**

Explainable planning research emphasizes model-based explanations and contrastive “why this rather than that?” questions.

What it implies: Worldwake traces should answer why selected, why rejected, why unknown, why illegal, why fallback, and why repair.

What it does **not** imply: a trace summary is enough if it cannot prove lawful input provenance.

Why it matters to live code: decision traces are rich, but snapshot admission is entity-level and HTN subgoals are mostly pending hints; they do not yet prove field-level FND-14 compliance.

### **Partial observability, false belief, epistemic planning**

Epistemic planning explicitly models knowledge/belief and information flow; POMDP framing maps policies over histories or belief states rather than underlying hidden states.

What it implies: Worldwake’s false belief, stale belief, contradictions, testimony, and records must be first-class.

What it does **not** imply: Worldwake needs full epistemic modal planning now.

Why it matters to live code: the current architecture has belief stores and confidence, but several social/legal/economic helpers still collapse “known entity” into “current authoritative fields.”

### **Capability/type-level enforcement**

Capability-based security bundles an unforgeable reference with explicit rights; object-capability design removes ambient authority and makes authority transfer explicit.

What it implies: do not give AI-facing modules ambient `&World`. Give them only source-scoped capabilities: local physical observation, self-authoritative state, belief memory, public topology, record-consult result, debug-only truth.

What it does **not** imply: import a security framework wholesale.

Why it matters to live code: `PerAgentBeliefView` currently wraps `&World`, so the compiler cannot distinguish lawful belief reads from accidental truth reads.

---

## **4. Current Architecture Map**

| Layer | Current files/types | Reads | Writes | Authority owned | Risk |
| ----- | ----- | ----- | ----- | ----- | ----- |
| Perception/belief acquisition | `PerAgentBeliefView`, belief stores, perception/institutional traces | `World`, belief store, local visibility, event-derived stores | Belief stores through action/perception systems | Belief and observed views, not world truth | **High**: view holds `&World`; method gates vary. |
| Runtime/goal belief view | `RuntimeBeliefView`, `GoalBeliefView`, trait stack | Very broad accessor surface | None directly | Supposed to expose lawful agent-local state | **Critical**: too broad; no source-typed returns for many dangerous facts. |
| Local physical observation | `ObservedRead`, `LocalPhysicalObservationView` | Same-place physical world facts | None | Same-tick co-located physical observation | Mostly clean. |
| Source/admission typing | `AdmissionSource`, `BeliefRead`, `ObservedRead` | Belief/source metadata | Snapshot admission traces | Entity admission and some value source tagging | **Insufficient**: entity-level snapshot admission is not field-level proof. |
| Planning snapshot | `PlanningSnapshot`, `SnapshotEntity` | `RuntimeBeliefView` | Planner-local snapshot | Derived planner-local data | **Critical**: many fields stored without field-specific source. |
| Planning state | `PlanningState` | Snapshot, planner-local overrides/caches | Planner-local deltas/caches | Planner-local derived state | Safe only if snapshot is safe. |
| Motive/goal discovery | `candidate_generation.rs`, `GoalOffer`, extractors | `GoalBeliefView`, memories | Candidate list, diagnostics, pending learned-memory writes | Desire/candidate proposal | Medium: side effects are deferred, but responsibilities are crowded. |
| Ranking/utility | `ranking.rs`, `AgendaEntry`, `OrderedRanked` | Candidates, belief view, profiles, memories | Ordered ranked goals | Preference ordering | Mostly consolidated; still score-heavy. |
| Portfolio triage | `agent_tick/portfolio.rs` | Ordered ranked goals, weights, operating mode | Slot map | Optional triage layer | Medium: staged/dead-code smell; not yet clear authority. |
| Agenda/intention | `IntentionFrame`, `agent_tick/frame.rs` | View, current plan, frames, memories | Frame state, blockers/discrepancies | Stable revisable intent | Good direction; causal placeholders hurt proof. |
| HTN method selection | `htn/selector.rs`, `MethodSchema` | `RuntimeBeliefView`, method registry | Selected/rejected method trace | Strategic search guidance | Mostly clean but not enforcement HTN. |
| HTN decomposition | `htn/methods.rs`, `search/strategic.rs` | Selected method subgoals | Strategic stages | Stage hints | Honest if documented as hints. |
| GOAP/action search | `search/strategic.rs`, tactical planners | `PlanningState`, affordances | `PlannedPlan` | Action-sequence search | Lawful only if snapshot/source path is lawful. |
| Revalidation/guards | `plan_revalidation.rs` | `RuntimeBeliefView`, affordance defs | Plan continuation/replacement decision | Belief-side pre-start validation | Medium: `can_control` guard inherits control leaks. |
| Repair | `plan_repair.rs` | Broken causal links, discrepancies, candidates | Repaired plan or typed barrier | Local repair attempt | Incomplete; not dangerous if treated as staged. |
| Action affordances | `ActionHandler`, `get_affordances` | `RuntimeBeliefView` | Affordance list | Belief-facing action menu | Good seam; view lawfulness is bottleneck. |
| Authoritative dispatch | `tick_step.rs`, `tick_action.rs`, `WorldTxn` | `World`, action defs, handlers | World mutations/event log | Live world authority | Clean relative to rest. |
| Player action surface | CLI `actions`, `do` | `PerAgentBeliefView`, affordances | Input queue | Human control over same actions | Good core, weak POV display. |
| Debug/control/display | CLI `switch`, `display.rs`, traces | `World` | Control source, stdout, traces | Meta/debug/observer | Must be isolated from normal play. |
| Decision/action traces | `decision_trace.rs`, `action_trace.rs` | Planner data, sometimes authoritative `World` | Append-only trace sinks | Diagnostics | Rich but not proof-grade for field source. |

---

## **5. Belief-Backed Simulation Verdict**

**No: Worldwake does not yet prove `docs/FOUNDATIONS.md` belief/world separation.** It partially enforces it through conventions, method-local gates, tests, and docs. It does not enforce it strongly at the type or architecture level.

Clean surfaces:

* Action dispatch and commit are authoritative and separated from affordance planning.  
* Local physical observation helpers are explicitly marked and mostly same-place gated.  
* Economic sale listing/seller access appears improved: listing/seller helpers are local-only.  
* HTN current methods are StageHint-only, not covert required leaves.

Dangerous surfaces:

* `GoalBeliefView`/`RuntimeBeliefView` are too broad and contain social/legal/economic/contention methods without field-level source typing.  
* `PerAgentBeliefView` still has live-world reads for control, rights, office/record data, queue/reservation/extraction surfaces, some social/institutional fields, and some policy/profile fields.  
* `PlanningSnapshot` stores many fields from those helpers and can amplify a single bad read into all search branches.

### **Belief View Source Table**

| Method / Surface | Source class it should use | Current source in code | Lawful? | Risk | Required change/test |
| ----- | ----- | ----- | ----- | ----- | ----- |
| `effective_place` | Self-authoritative; local physical; possessed physical; belief/last-seen for remote | Self live, local/possessed live, else belief/last_seen | Mostly | Medium | Add stale/false target-location tests. |
| `entity_kind` | Public topology for places; local physical or belief for entities | Places public; otherwise `knows_entity` then current world kind | Ambiguous | Medium | Remote known corpse/type-change test; kind must come from belief unless local. |
| `entities_at(place)` | Local physical if actor at place; belief memory otherwise | Local live, else belief store | Yes | Low | Preserve. |
| Ownership/holder/possessor/container | Self/local/possessed physical; explicit belief/testimony/record otherwise | `direct_container`/`direct_possessor` can read current world for known entities; snapshot filters some remote cases | Not proven | High | Field-source wrapper plus remote moved-container false-belief test. |
| Rights/control | Self-authoritative only for actor; explicit believed rights/record/local exercise observation | `believed_rights`/`can_control` gate then call `world.effective_rights`/`world.can_exercise_control`; owner gate reads truth | No | **Critical** | Replace with `ControlBeliefRead`; tests for remote ownership/control changes unknown to actor. |
| `has_control` / control source | Self/currently controlled surface only; otherwise belief/testimony | Reads `control_source` for any known entity | No for ordinary POV | High | Restrict to self/debug; player switch symmetry test. |
| Faction/office/jurisdiction/support/loyalty | Institutional beliefs, records, testimony | Factions partly belief; office data and loyalty read live once known | Not proven | High | Replace `office_data`/`loyalty_to` with institutional belief reads. |
| `record_data` / `office_data` | Consulted record memory or local record observation; not current record truth | Current world record/office data if entity known | No | **Critical** | Record-snapshot stored in belief; test changed record not known until consulted. |
| Sale listing/seller/stock | Local market observation; belief/testimony/record for remote | Listing/seller local-only; stock via belief/local; `merchandise_profile` may leak remote seller profile | Mostly | Medium | Preserve local listing; remove remote profile leak. |
| Commodity quantity | Self/direct possession/local physical; belief inventory remote | Mostly follows that | Mostly | Low/Medium | Add remote false-stock test. |
| Resource source/workstation/job | Local physical or believed state | Source/workstation/job mostly local/belief; `stock_storage_policy` leaks current policy if known facility | Mixed | Medium | Policy must be belief/record/local only. |
| Queues/reservations/grants/contention | Local observation, own ticket, explicit record/testimony; authoritative only at dispatch | Some local-gated; extraction claimability, queue join/patience, reservation conflicts/ranges read live | No | **Critical** | Remove from belief view or source-tag; adversarial remote queue/reservation tests. |
| Route danger/threat/travel cost | Public topology plus known/stale route memories and threat records | Topology public; route preferences/beliefs used; incomplete inspection of threat compiler | Unclear | Medium | Route-danger stale/false-warning tests. |
| Belief confidence/stale/contradiction | Belief store only | `BeliefRead`, `BeliefValue`, confidence policy | Yes | Low | Preserve. |
| Last-seen/testimony/records | Belief memory, testimony, consulted record artifacts | Belief stores exist; record data live leak undermines record path | Mixed | High | Separate `KnownRecordSnapshot` from current `RecordData`. |
| Direct physical observation helpers | Same-tick local physical only | `ObservedRead` and local visibility helpers | Mostly yes | Low | Preserve; forbid social/legal reads under this umbrella. |

### **Planning Snapshot Source Table**

| Snapshot field | Source class required | Current admission/source handling | Lawful? | Risk | Required change/test |
| ----- | ----- | ----- | ----- | ----- | ----- |
| `SnapshotEntity.admission` | Entity admission only; not enough for fields | Single `AdmissionSource` per entity | Insufficient | **Critical** | Add field-level source table. |
| `kind`, `alive`, `effective_place` | Self/local/belief per field | Uses view; belief-backed branch narrows some fields | Mostly | Medium | Field provenance trace. |
| `direct_container`, `direct_possessor` | Local/self/possessed or explicit belief | Remote belief-backed case partly suppressed; other admissions still depend on view | Not proven | High | Snapshot field source + moved-container test. |
| `record_data`, `office_data` | Consulted/remembered record, not current truth | Stored from view, which reads current world once known | No | **Critical** | Store only `BelievedRecordData`/`BelievedOfficeData`. |
| `controllable_by_actor`, `has_control` | Believed rights/control or self-authoritative | Stored from view methods that read current control/right truth | No | High | Remove from snapshot or source-tag. |
| Queues/reservations/extraction | Local/own ticket/belief only | Snapshot stores reservation ranges and queue data from view | No | **Critical** | Do not snapshot remote authoritative contention state. |
| Sale listings/seller | Local or believed market listing | Local-only helpers reduce risk | Mostly | Low/Medium | Add remote stale listing test. |
| Distance matrix | Public topology/perceived route costs | Snapshot builds matrices; Floyd-Warshall comment assumes small snapshots | Lawful but scaling risk | Medium | Cap/trace matrix size; route-source tests. |
| Snapshot admission trace | Per-field proof | Entity-level `SnapshotAdmissionTrace` only | No | High | Trace `SnapshotFieldAdmissionTrace`. |
| PlanningState caches | Pure memoization of lawful snapshot | Declared pure memoization | Lawful if snapshot lawful | Medium | Add invariant test that cache never calls live world. |

### **Player/UI Truth-Leak Table**

| Surface | Reads through | Normal play or debug? | Could leak truth? | Required separation/test |
| ----- | ----- | ----- | ----- | ----- |
| CLI `actions` | `PerAgentBeliefView` + `get_affordances` | Intended player-facing prototype | Yes, through belief-view leaks | Add AI/player affordance symmetry tests under false belief, stale owner, remote queue. |
| CLI `do` | Last affordance, strict request | Normal prototype | Low | Preserve strict mode; stale menu revalidation test. |
| CLI hidden-action filtering | Registry name filter | UI convenience | Medium | Treat as presentation only; action law remains registry/affordance. |
| CLI `switch` | Direct world name/location/control mutation | Debug/meta | Yes | Mark explicitly not normal POV; future player switch must use POV-safe resolver. |
| `display.rs` names/location/deltas | Direct `World` | Debug/observer | Yes | Create `pov_display.rs` using `RuntimeBeliefView`; forbid normal UI import of `display.rs`. |
| `ActionTraceSink` legality traces | Authoritative `World` | Debug/golden | Yes | Never expose as normal character knowledge. |
| `DecisionTraceSink` | Planner traces/snapshot admissions | Debug/golden, maybe future UI | Yes if surfaced raw | Add `DebugOnlyTrace` capability; redacted POV trace view. |
| Future UI action menus | Should read same lawful affordance surface as AI | Normal play | Yes unless hardened | Golden player/AI symmetry suite. |

---

## **6. Hidden Authority Leak Inventory**

| Leak | Symptom | Evidence | Type | Why it matters | FOUNDATIONS implicated | Failure mode | Severity | Confidence | Test |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| Control/right truth through belief view | `can_control`/`believed_rights` call live world control/right resolution after weak gates | `ControlBeliefView` implementation uses `world.effective_rights` and `world.can_exercise_control` | Direct | Agents can act on legal authority they did not learn | FND-14, FND-14A, FND-19 | Remote ownership/control update changes AI/UI actions instantly | Critical | High | Actor believes old owner; world owner changes remotely; actor must not see new control action until carrier. |
| Current record/office truth | `record_data`/`office_data` expose current authoritative data for known records/offices | Per-agent view returns current world components; `OfficeData` contains jurisdiction/succession/vacancy | Direct/social | Legal/institutional facts become omniscient once entity known | FND-14, FND-17, FND-19 | Player sees office vacancy or record change without consulting record/testimony | Critical | High | Change crime register remotely; no consult/tell; candidate/affordance must not change. |
| Extraction slot claimability | `actor_can_claim_extraction_slot` reads live extraction queues without local/belief gate | Queue/extraction methods in `per_agent_belief_view.rs` | Direct/contention | Remote resource contention becomes known | FND-14, FND-7, FND-19 | Agent avoids/chooses resource because queue truth changed remotely | Critical | High | Remote agent fills extraction slot; distant actor’s plan/rank/UI must remain based on stale belief. |
| Reservation ranges/conflicts | Reservation helpers expose live ranges/conflicts | `reservation_conflicts`, `reservation_ranges` read live | Direct/contention | Reservations are authoritative dispatch facts, not planner knowledge unless local/own ticket | FND-7, FND-14 | Planner avoids a collision it cannot know | Critical | High | Remote reservation created; actor action menu must not hide candidate unless notified. |
| Snapshot field laundering | Entity-level admission lets many fields ride on one source | `SnapshotEntity` single `AdmissionSource`, many fields | Snapshot-mediated | One lawful entity admission can launder unlawful field values | FND-14B, FND-27 | Search branches rely on current record/control/queue facts | Critical | High | Snapshot trace must prove per-field sources; current trace cannot. |
| Direct container/possessor current state | Known remote entity can trigger current container/possessor reads | Per-agent view methods; snapshot partly filters belief-backed remote | Direct/snapshot | Custody/possession is dynamic and must travel through observation/testimony | FND-5, FND-14 | Agent tracks stolen/moved item without seeing it | High | Medium/High | Move item between containers remotely; no observation; plan must remain stale. |
| Loyalty/reward/social truth | Social/institutional current facts leak through helper methods | `loyalty_to`, `visible_reward_encumbrance`, factions/policies | Direct/social | Social facts are not physical co-location facts | FND-14A, FND-17 | Political/economic goals shift from hidden loyalty/bounty state | High | Medium | Remote loyalty/support change; actor cannot know absent testimony/record. |
| Debug legality truth | Action trace computes authoritative fine failure facts and quantities | `derive_start_failure_legality_trace` reads `World` | Trace-mediated | Safe only if debug/golden; lethal if normal UI uses it | FND-19, FND-29 | Player learns accused holdings/current place from failed fine trace | High | High | Normal UI redaction test: no authoritative quantity/place in player-visible trace. |
| CLI display omniscience | Display names/location resolve through `World` | `display.rs`, `control.rs` | UI-mediated | Future player POV can inherit debug truth | FND-19 | Switch menu/location display leaks remote location | High | High | POV switch/display test must use believed names/locations only. |
| `has_control` for known entity | Control source read exposed via belief view | `has_control` reads `AgentData.control_source` | Direct/UI | Control source is meta unless explicitly perceived | FND-19 | Player can infer which NPC is human/AI or controlled | Medium/High | High | Remote control-source change invisible to character. |
| Merchandise/stock-storage policy | Known facility/seller exposes current profile/policy | `merchandise_profile`, `stock_storage_policy` | Direct/economic | Merchant policies are not magically known | FND-14, FND-17 | Agent optimizes around merchant internal policy | Medium | Medium | Remote merchant policy changes; no testimony; no planning effect. |
| Method precondition inheritance | HTN preconditions use belief view, so leaks propagate into method selection | `htn/selector.rs` calls `record_data`, sale/source helpers | Indirect | HTN can amplify hidden truth into strategic branch choice | FND-14, FND-20 | Method chosen/rejected by current record/control truth | High | Medium | Record changed remotely; HTN method trace must not change. |
| PlanningState cache trust | Cache is pure but caches leaked snapshot data | PlanningState cache invariant says pure memoization | Cache-mediated | Cache makes bad facts stable and hard to detect | FND-27 | Debug trace says cache hit, not unlawful source | Medium | High | Cache provenance invariant: every cached value carries field source. |

---

## **7. Hostile Failure Inventory**

| Smell | Evidence | Why it matters | FOUNDATIONS implicated | Downstream failure | Severity | Status |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| Broad `RuntimeBeliefView` god trait | Trait composes many domains and is passed to snapshots, affordances, revalidation, duration, repair | Too many surfaces can access too much | FND-14, FND-28 | Future helper accidentally reads truth | Critical | Real issue |
| Entity-level source admission | `SnapshotEntity.admission` only per entity | Different fields require different carriers | FND-14B, FND-27 | False proof of belief locality | Critical | Real issue |
| “Known entity” treated as permission to current fields | Several helpers gate on `knows_entity` then read live components | Knowing an entity exists is not knowing all current facts | FND-14A | Remote fact updates leak | High | Real issue |
| HTN label overclaim | All subgoals StageHint; fallback generic search legal | This is not enforced decomposition | FND-20 | Designers may add fake methods | Medium | Real but manageable |
| Repair staged but named complete | `InsertVerification` returns no substrate; tests say future ticket wires replacement | Could mask planning failure | FND-21, FND-31 | Agents “repair” by typed barrier/abandon only | Medium | Real issue |
| Causal placeholder IDs | `EventId(0)` used in blockers/discrepancies | Violates append-only causal proof ambition | FND-29 | Trace cannot prove source event | Medium | Real issue |
| Candidate generation overloaded | Extractors, omissions, diagnostics, pending memory writes in one pass | Ownership muddiness grows with features | FND-28 | Emitters become hidden sensors | Medium | Real issue |
| Portfolio staged/dead-code smell | `#![allow(dead_code)]` in portfolio | Could become fossil or parallel ranking authority | FND-28 | Two triage systems fight | Medium | Suspected issue |
| CLI display used as normal POV risk | `display.rs` reads world directly | Future UI likely copies it | FND-19 | Player omniscience | High | Real issue |
| Trace as explanation not proof | Decision/action traces rich but not field-source complete | Explanation can launder truth | FND-29, FND-31 | Golden tests pass with illegal knowledge | High | Real issue |
| Action dispatch split | Handler types separate belief affordances from authoritative commit | Correct seam | FND-7, FND-14 | None | Low | Already-clean surface |
| StageHint HTN registry tests | Tests forbid required leaves | Prevents fake enforcement | FND-20 | None | Low | Already-clean surface |
| Economic sale listing local gate | Listing/seller helpers local-only | Fixes prior obvious leak | FND-14A | None if preserved | Low | Already-clean surface |

---

## **8. GOAP / HTN / BDI / Utility Responsibility Matrix**

| Responsibility | Should be owned by | Current owner | Problem? | Recommendation |
| ----- | ----- | ----- | ----- | ----- |
| Motive discovery | Candidate extractors over lawful beliefs | `candidate_generation.rs` | Crowded but workable | Keep; split diagnostics/memory scheduling later. |
| Candidate generation | Goal schemas + extractors | `GoalSchema`, extractor registry | Improved | Preserve schema-driven order; forbid hidden world reads. |
| Goal ranking | Ranking module only | `ranking.rs` | Mostly clean | Keep single total order. |
| Portfolio/slot triage | Optional BDI deliberation shell | `agent_tick/portfolio.rs` | Staged/dead-code smell | Either wire deliberately or remove before expansion. |
| Intention persistence | BDI-like frame layer | `IntentionFrame`, `frame.rs` | Good shape | Keep; add real causal IDs and stronger assumption provenance. |
| Method selection | HTN stage-guidance layer | `htn/selector.rs` | Depends on belief-view safety | Keep but call it method-guided search. |
| Method decomposition | HTN registry/methods | StageHint-only methods | Not full HTN | Keep StageHint until enforcement contract exists. |
| Action-sequence planning | GOAP/search | `search/strategic.rs`, tactical search | Correct owner | Preserve. |
| Fallback planning | GOAP/search policy | Strategic search | Mostly correct | Trace why fallback legal. |
| Failure attribution | Action runtime + AI failure handling | `tick_action`, scheduler, repair/discrepancy | Mixed | Add field-source and event-causal links. |
| Repair | Local repair layer | `plan_repair.rs` | Incomplete | Keep narrow; do not pretend complete. |
| Contention handling | World action/scheduler authority; belief only through carriers | Mixed: sim queues + belief helpers | Leaks | Remove remote authoritative reads. |
| Belief correction | Perception/testimony/records | Belief store/actions | Mostly | Keep; record current snapshot only on lawful consult. |
| Source/admission control | Belief/snapshot boundary | `AdmissionSource`, `BeliefRead`, `ObservedRead` | Too coarse | Add field-level source. |
| Player action visibility | Same affordance surface as AI | CLI `actions` | Good core | Harden view; separate debug display. |
| Trace explanation | Decision/action trace | `decision_trace.rs`, `action_trace.rs` | Rich but not proof-grade | Add contrastive and field-source traces. |
| Test/golden validation | Golden harness/scenario tests | Some strong, many missing | Insufficient for FND-14 | Add adversarial belief-wall suite. |

---

## **9. HTN Verdict**

**HTN earns its place only as method-guided strategic search right now.** It does not yet earn method-required authority.

Current live facts:

* `MethodSubgoalAuthority` has `StageHint` and `RequiredActionLeaf`, but registry tests assert every current method subgoal is `StageHint`; required leaves are not used.  
* Methods are selected through preconditions evaluated over `RuntimeBeliefView`.  
* Strategic search converts method subgoals into stages and falls back to generic stages when no method applies or stages are unavailable.

| Method / family | Classification | Live problem solved | Preconditions belief-local? | Subgoals enforced? | Fallback legal? | Missing tests |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| `fulfill_bounty_direct` | HTN justified but boundary needs tightening | Prioritizes direct pursuit when target last-seen/known | Depends on bounty/record/source safety | Advisory StageHint | Yes | Stale bounty record; false last-seen target; no direct record consultation. |
| `fulfill_bounty_investigation` | HTN justified but boundary needs tightening | Adds information-gathering route for bounty | Depends on witness/record belief path | Advisory | Yes | Witness testimony vs current record truth. |
| `fulfill_bounty_support_declared_direct` | HTN optional; flat GOAP legal | Biases pursuit when support declared | Social support source must be explicit | Advisory | Yes | Support declaration false/stale tests. |
| `produce_from_owned_stock` | HTN optional; maybe unnecessary | Narrows production search from owned inputs | Mostly self/local inventory | Advisory | Yes | Owned stock moved/stolen mid-plan. |
| `produce_gather_inputs` | HTN justified as strategic search narrowing | Gather before craft | Resource source belief/locality critical | Advisory | Yes | Remote depleted source remains stale. |
| `produce_purchase_inputs` | HTN justified but boundary needs tightening | Buy before craft | Seller/listing locality mostly improved | Advisory | Yes | Remote market listing stale/false. |
| `restock_by_harvest` | HTN justified | Merchant restock from source | Source/job/queue leak risk | Advisory | Yes | Queue/extraction-slot false belief. |
| `restock_by_market_purchase` | HTN justified | Merchant restock through market | Listing local, seller profile risk | Advisory | Yes | Remote seller profile not known. |
| `investigate_by_witness` | HTN justified | Encodes social evidence path | Witness beliefs must be explicit | Advisory | Yes | Witness lies/contradictions/stale testimony. |
| `investigate_by_ledger` | HTN justified but boundary needs tightening | Consult records as evidence | `record_data` current truth leak | Advisory | Yes | Ledger changed after last consult. |
| `escort_to_home` | Insufficient evidence / optional | Travel/escort pattern | Target home/safety belief source unclear | Advisory | Yes | Ward location false belief; escort destination stale. |

No method should become method-required yet. A method-required policy would need:

1. `RequiredActionLeaf` enforced by strategic/tactical search.  
2. A schema contract saying flat GOAP is semantically invalid.  
3. A trace that records rejected methods, selected method, enforced leaves, fallback forbidden reason, and failed leaf.  
4. Golden tests proving flat fallback would violate causality or legality.

---

## **10. Player-POV and UI Symmetry Audit**

The core direction is good: the CLI action menu calls `get_affordances()` through `PerAgentBeliefView`, stores the resulting affordances, and `do` enqueues a strict request from that menu. That is the right skeleton for AI/player symmetry.

The current player-facing prototype is **not yet safe** because the belief view itself is not safe. If `PerAgentBeliefView` leaks remote control, record, queue, or reservation truth, the player menu leaks it too.

Specific risks:

* `handle_actions` is only as lawful as `PerAgentBeliefView`.  
* `handle_switch` is a meta operation that resolves names, checks aliveness, mutates control source, and prints current location through direct `World` access. That is fine for debug, not for normal POV.  
* `display.rs` resolves entity names, locations, deltas, item lots, resource sources, and kinds through direct `World`. This must not be imported into future normal play UI.  
* Action traces can contain authoritative quantities and legal facts. Good for golden tests, unsafe for player-visible feedback.

Required protections:

1. Add `DebugWorldView`/`ObserverUi` capability for omniscient CLI/debug surfaces.  
2. Add `CharacterPovView` for normal UI, backed by the same lawful belief/action surface as AI.  
3. Forbid normal UI modules from importing `worldwake_core::World` directly.  
4. Add tests where AI and human control produce identical affordances under stale ownership, false seller stock, hidden office changes, remote reservations, remote queue changes, and stale target location.

---

## **11. Intentions, Repair, and Replanning Audit**

`IntentionFrame` is the right direction. It models commitment without turning commitment into entitlement. Frames can suspend, resume, exhaust, clear plans, record blockers/discrepancies, and avoid thrashing. The assumptions system includes route existence, target alive, commodity availability, critical threat, and projected need safety.

Clean parts:

* A blocked travel step increments stalled ticks and clears current plan/materialization bindings/facility intents rather than silently forcing success.  
* Commodity availability assessment is local physical when co-located and otherwise uses the agent belief store.  
* Assumption evaluation distinguishes critical failure, recoverable suspension, and deferred unknowns.

Problems:

* Frames mostly appear travel-shaped: `update_frame_for_adopted_plan` creates frames only when the selected plan has a terminal travel destination. That may be acceptable now, but it means “intention stability” is not uniformly implemented for all goal families.  
* Several discrepancy/blocker records use `EventId(0)`. That is not append-only causal history; it is a placeholder.  
* Repair is not finished. `InsertVerification` returns `NoEpistemicSubstrate`, and tests explicitly describe staged repair search awaiting future wiring.

Verdict: stable intentions are **conceptually clean but incomplete**. Do not expand repair until causal links and field-source provenance are real.

---

## **12. Candidate, Ranking, Portfolio, and Scaling Audit**

Candidate generation is better than the prior report suggested. The extractor order is explicit, schema-driven candidate extractor selection exists, and the old “legacy order” smell is reduced.

Remaining risks:

| Area | Assessment |
| ----- | ----- |
| Candidate fan-out | Many extractors are centralized; this is acceptable now but will get expensive with hundreds of agents/locations. |
| Extractor ownership | Candidate generation still mixes opportunity emission, omission diagnostics, violation detection, learned opportunity behavior, and pending memory writes. |
| Ranking authority | `ranking.rs` has one authoritative total order and read-only `OrderedRanked`; good. |
| Utility/motive score risk | Scores are useful, but every score must be traceable to concrete state/belief. |
| Portfolio slots | The slot model is promising but smells staged because the module is `allow(dead_code)`. Decide whether it is real or remove it. |
| Top-K planning | Strategic search narrows candidate planning; good, but trace budget exhaustion must remain explicit. |
| Snapshot/path costs | Snapshot distance matrix uses Floyd-Warshall and assumes small snapshots; fine for now, risky if snapshots grow. |
| Trace volume | Rich trace data is useful but will explode at hundreds of agents unless sampled, aggregated, and queryable. |
| Learned opportunity behavior | Useful, but must not become a hidden candidate authority or magical memory correction. |

Near-term path: harden belief sources first, then split candidate generation into `emit`, `diagnose omissions`, `schedule memory writes`, and `compile opportunities`.

Long-term path: event/delta-driven candidate invalidation, per-agent dirty sets, per-place indices, bounded trace retention, and source-scoped snapshot caches.

---

## **13. Test Validity and Golden Coverage Audit**

| Test / Area | Valid under FOUNDATIONS? | What it proves | What it fails to prove | Recommendation |
| ----- | ----- | ----- | ----- | ----- |
| `golden_goal_invalidation_by_another_agent` | Yes | Need-driven acquisition, conservation, alternate goal after resource mismatch | Not belief locality; Bob is seeded with broad world beliefs later in other tests | Keep. |
| Frontier exhaustion isolation | Yes | Unrelated commodity change does not clear unrelated exhaustion | Not hidden authority; Bob is explicitly seeded with world beliefs | Keep, but add belief-source assertions. |
| Priority interrupt | Yes | Need escalation and interruption work | Does not test false belief or player symmetry | Keep. |
| Local depleted source regeneration | Yes | Local observation/regeneration/failure-memory behavior | Not remote source ignorance | Keep and add remote variant. |
| CLI `actions` tests | Weak but valid | Menu populates and stores affordances | Does not prove lawful POV | Expand heavily. |
| CLI `switch` tests | Valid as debug/meta | Control transfer preserves world state | Could normalize omniscient switch/display as normal play | Mark debug-only; add normal POV tests separately. |
| HTN registry tests | Strong | Current methods are StageHint-only; no required leaves | Does not prove method preconditions lawful | Keep; add belief-leak method tests. |
| Action runtime tests | Strong for dispatch | Recoverable revalidation path and action lifecycle | Not planner-source lawfulness | Keep. |
| Snapshot admission tests | Insufficient evidence found | Entity admission may be traced | Not field-level source proof | Add required field-source tests. |
| Player/AI symmetry tests | Missing | Nothing | Critical future constraint untested | Add before UI expansion. |
| Queue/reservation FND-14 tests | Missing | Nothing | Critical leak class untested | Add immediately. |
| Record/office stale/false tests | Missing | Nothing | Critical legal/social leak class untested | Add immediately. |

Tests that encode omniscience were not conclusively found in the inspected files, but several current tests seed broad world beliefs for convenience. That is acceptable only when the test name and assertions are not claiming ignorance/stale-belief behavior.

---

## **14. Consolidation or Redesign Options**

### **Option A — Conservative Hardening**

Minimal changes: add tests, docs, assertions, and source traces.

Benefits: low migration cost; preserves current architecture.

Risks: does not eliminate ambient `&World` authority; future leaks remain likely.

Migration cost: low.

FOUNDATIONS alignment: partial.

Choose this only if the next milestone is very small and no new AI behavior is added.

### **Option B — Moderate Consolidation**

Keep GOAP, current StageHint HTN, candidate/ranking structure, action runtime, and intention frames. Replace broad source conventions with field-level source types and split dangerous belief surfaces.

Benefits: fixes the real boundary without rewriting everything.

Risks: medium refactor cost; snapshot/action tests must be updated.

Migration cost: medium.

FOUNDATIONS alignment: strong.

**Recommended.**

### **Option C — Aggressive Redesign**

Replace `RuntimeBeliefView` with capability-typed modules, rebuild snapshot representation, redesign candidate generation as event-driven BDI deliberation, and remove all staged repair/portfolio seams until rebuilt.

Benefits: strongest long-term architecture.

Risks: high churn; likely delays simulation expansion; may destroy working improvements.

Migration cost: high.

FOUNDATIONS alignment: strongest if executed well.

Choose only if field-source hardening reveals pervasive leaks that cannot be isolated.

---

## **15. Recommended Architecture**

Preserve:

* Authoritative action runtime and `WorldTxn` dispatch.  
* GOAP/action-sequence planning.  
* Current HTN layer as StageHint method-guided search.  
* Candidate/ranking single-order contract.  
* Intention frames.  
* Decision/action traces, but with stronger provenance.

Remove or redesign:

* Broad `RuntimeBeliefView` as the default API for every planning surface.  
* Live-world social/legal/control/queue/reservation methods on belief views.  
* Entity-level-only planning snapshot admission.  
* Normal UI dependence on `display.rs`.

Defer:

* Method-required HTN.  
* Advanced repair insertion.  
* Large-scale optimization beyond source-scoped caches.

Target pipeline:

World truth  
 ↓ only through lawful carriers  
Perception / testimony / record consultation / self-state / local physical observation  
 ↓ source-tagged writes  
Agent belief + memory + records  
 ↓ field-source-limited views  
Candidate emission  
 ↓ concrete GoalOffer + evidence paths  
Ranking / portfolio deliberation  
 ↓ chosen intention  
StageHint HTN method guidance  
 ↓ ordinary GOAP/action search over field-sourced PlanningSnapshot  
Plan + guards + causal links  
 ↓ strict affordance/start validation  
Authoritative action runtime / scheduler / WorldTxn  
 ↓ event log + lawful perception updates  
Belief and memory changes  
---

## **16. File-by-File Proposal**

| File / Module | Proposed change | Confidence | Reason | Acceptance criteria |
| ----- | ----- | ----- | ----- | ----- |
| `crates/worldwake-sim/src/belief_view.rs` | Split broad traits into source-specific capability traits: `SelfStateView`, `LocalPhysicalView`, `BeliefMemoryView`, `InstitutionalBeliefView`, `ContentionBeliefView`, `DebugWorldView`. | High | `RuntimeBeliefView` is too broad. | AI/planner code cannot call control/record/queue methods unless capability type permits. |
| `per_agent_belief_view.rs` | Replace live social/legal/control reads with `BeliefRead`/`ObservedRead`/`InstitutionalRead` return types. | High | Current methods leak truth. | Remote changes do not affect view unless actor observed/consulted/heard. |
| `per_agent_belief_view.rs` | Remove or local-gate `actor_can_claim_extraction_slot`, `has_extraction_queues`, `reservation_conflicts`, `reservation_ranges`, queue join/patience helpers. | High | Current contention reads are authoritative. | New tests prove remote queue/reservation changes are invisible. |
| `planning_snapshot.rs` | Add field-level source provenance. | High | Entity admission is insufficient. | Every `SnapshotEntity` field has a `SnapshotFieldSource`; trace can report it. |
| `planning_snapshot.rs` | Stop copying `record_data`, `office_data`, `controllable_by_actor`, reservation ranges, and queue fields unless field source is lawful. | High | Snapshot currently launders view leaks. | Snapshot construction fails/lints if dangerous field lacks source. |
| `planning_state.rs` | Preserve cache but attach provenance or prove values derived solely from snapshot fields. | Medium | Cache itself is fine but hides leaks. | Cache trace includes field source or verified derived marker. |
| `htn/selector.rs` | Require method precondition traces to include source class for every fact read. | Medium | Method choice can leak via preconditions. | Rejected/selected method trace says belief/local/record/debug source. |
| `htn/method_schema.rs` | Rename docs: “current methods are strategic StageHint methods, not enforced HTN leaves.” | High | Prevents future misuse. | Docs/tests reject method-required additions without contract. |
| `search/strategic.rs` | Trace fallback legality: no method, method failed, selected method produced no stages, fallback allowed by policy. | Medium | Current fallback is mostly okay but needs proof. | Golden tests assert fallback reason. |
| `action_trace.rs` | Mark authoritative legality traces debug-only and add redacted POV serializer. | High | Trace contains authoritative facts. | Normal UI cannot access raw trace facts. |
| `worldwake-cli/src/display.rs` | Add module-level warning: observer/debug only; normal POV must not import. Create `pov_display.rs`. | High | Current display is omniscient. | Compile-time or lint guard for normal UI module. |
| `worldwake-cli/src/handlers/control.rs` | Mark `switch`/`observe` as meta-control, not character action. | High | It mutates hidden control source and prints truth. | Normal game UI has separate POV-safe switching. |
| `agent_tick/frame.rs` | Replace `EventId(0)` placeholders with real causal event IDs or explicit `NoSourceEventYet` type. | High | Placeholder breaks causal history. | No blocker/discrepancy has fake event ID. |
| `plan_repair.rs` | Keep repair narrow; do not advertise verification insertion until substrate exists. | High | Current `InsertVerification` is a stub. | Trace says unavailable, not silently repaired. |
| `docs/planner-contracts.md` | Add field-level planner visibility contract. | High | Current docs already point that way. | All planner-visible fields declare source class. |

Candidate replacement text for `docs/planner-contracts.md`:

### Field-level planner visibility

Entity admission is never sufficient to admit all fields on that entity.

Every planner-visible field MUST carry one of these source classes:

- `SelfAuthoritative`  
- `LocalSameTickPhysical`  
- `BeliefMemory`  
- `Testimony`  
- `ConsultedRecordSnapshot`  
- `PublicTopology`  
- `PlannerDerived`  
- `AuthoritativeDispatchOnly`  
- `DebugOnly`

A `PlanningSnapshot` may include an entity because the entity is locally visible, remembered, possessed, or topologically public. That admission does not by itself authorize reading current ownership, rights, office data, record contents, queue state, reservations, grants, control source, loyalty, faction policy, seller stock, production jobs, or social/legal claims. Those fields must be absent unless their own source class is lawful for the actor at the snapshot tick.  
---

## **17. Golden Scenario and Evaluation Matrix**

| Scenario / Metric | Purpose | Systems exercised | Required assertions | Failure smell |
| ----- | ----- | ----- | ----- | ----- |
| Remote owner changes unseen | Hidden authority leak | Belief view, snapshot, affordance, UI | No new control/right action appears until carrier | Rights truth leak |
| Remote office vacancy changes unseen | Institutional belief | Office data, HTN political candidates | Claim/support candidates unchanged until record/testimony | Office truth leak |
| Record entry changed after last consult | Record memory | Consult record, candidate gen, traces | Actor uses stale consulted snapshot | Current record leak |
| Remote extraction slot filled | Contention | Queue/grant/extraction helpers | Distant actor still believes old claimability | Queue truth leak |
| Remote reservation created | Scheduler/planner | Reservations, affordance revalidation | Planner may attempt; authoritative start may fail | Planner omniscience |
| Item moved between containers unseen | Custody/possession | Container/possessor snapshot | Actor belief remains stale | Container truth leak |
| Seller delists remote stock | Economy | Sale listing, seller stock, ranking | No remote ranking/action change absent market observation | Stock truth leak |
| Merchant policy changes unseen | Economy/profile | Merchandise/stock policy | No planning change absent testimony/record | Profile leak |
| False target last-seen pursuit | Belief/HTN/GOAP | Pursuit, route, combat | Agent travels to believed place; discovers contradiction locally | Target-location truth leak |
| Contradictory testimony | Belief confidence | Testimony, ranking, trace | Candidate damped/omitted by contradiction, not truth | Belief collapse |
| Stale route danger | Route danger | Route prefs/threat records | Agent can choose bad route from stale belief | Route omniscience |
| Player/AI affordance parity | Symmetry | CLI/future UI, AI driver | Same character, same beliefs => same actions | Player/AI split |
| Debug trace redaction | UI separation | Action/decision trace | Normal UI cannot see authoritative hidden facts | Trace-mediated leak |
| HTN record-precondition false belief | HTN | Method selector, record data | Method selected/rejected from belief snapshot only | Method truth leak |
| HTN fallback legality | HTN/GOAP | Method trace/search | Trace says why fallback allowed | Fake HTN |
| Repair unavailable verification | Repair | Plan repair trace | `InsertVerification` reports no substrate; no fake repair | Silent repair |
| Frame causal source | Intentions | Frame/blocker/discrepancy | No `EventId(0)` placeholder | Fake causal history |
| Snapshot field-source audit | Snapshot | Planning snapshot | Every field has lawful source | Entity-source laundering |
| Large-world soak | Scaling | Candidate/ranking/snapshot/traces | Per-agent timings, snapshot size, trace bytes bounded | Scaling collapse |

---

## **18. Research-Backed Design Rules For Future AI Work**

* A goal deserves HTN only when it encodes a reusable pursuit pattern that narrows search without bypassing ordinary action affordances.  
* Flat GOAP is enough when the desired behavior can be expressed through ordinary preconditions/effects and goal conditions.  
* Method-required behavior is justified only when flat fallback would violate semantics, and only with enforced leaves, tests, and fallback-forbidden traces.  
* A motive becomes an intention when it survives ranking, feasibility, interruption margins, and can be represented as a revisable frame with explicit assumptions.  
* A blocker is belief memory when it reflects the agent’s experienced failure; it is world state only when it is a concrete physical/social object in the world.  
* Repair is allowed when it preserves a causally valid prefix and has lawful new evidence. Otherwise full replan or typed barrier.  
* A cache is legal only when it is source-scoped, invalidated by the relevant source, and cannot read `World` after construction.  
* A derived score becomes dangerous when it hides which concrete belief, record, testimony, or local observation caused it.  
* Player-facing UI leaks omniscience when it uses direct `World` names, locations, office data, record data, queue state, or trace facts not known to the character.  
* A trace is sufficient only when it can answer “why this,” “why not that,” and “what lawful source made this fact planner-visible.”  
* A belief-view method is lawful only when its source class is explicit and narrower than its name.  
* A source/admission tag is too coarse if two fields on the same entity can lawfully differ in visibility.  
* A test should be deleted or rewritten if it asserts behavior that requires remote truth without observation, testimony, record consultation, or self-authority.

---

## **19. Open Questions and Uncertainties**

| Uncertainty | Why it matters | Evidence needed |
| ----- | ----- | ----- |
| Full action-definition coverage | Some affordance handlers may contain additional hidden truth reads | Inspect all registered handlers and payload validators. |
| Full perception system behavior | Belief updates may already mitigate or worsen some leaks | Inspect perception systems, testimony actions, record consultation actions. |
| Route danger source path | I inspected route/topology surfaces but not every threat-warning path | Inspect `route_threat`, patrol/threat warning emitters, route preference writes. |
| Full test suite | I inspected representative golden/CLI/HTN tests, not all tests | Full targeted search/fetch of all `tests/` and `#[cfg(test)]` modules. |
| Portfolio live usage | The module is staged-looking; unclear how central it is in final selection | Inspect full `agent_tick/mod.rs` and planning selection path. |
| Future UI plans | Current CLI is prototype/debug mixed | Inspect active UI specs if any exist outside fetched files. |
| Whether `office_data` is intended as public topology | Foundations says social/legal facts are not physical facts; but office metadata may be partly public | Decide per field: title/seat maybe public, holder/vacancy/jurisdiction/legal effect needs carrier. |
| Whether control source is simulation-visible | `ControlSource` is meta-control, not character knowledge | Add explicit docs/tests. |
| Whether `DebugWorldView` is fully isolated | Trait has compile-fail guards, but raw `World` imports can bypass it | Module-boundary/lint inspection. |
| Exact migration cost for field sources | Snapshot and tests may need broad updates | Prototype `SnapshotFieldSource` on three high-risk fields first. |

---

## **20. Fourth-Iteration Prompt Suggestions**

The fourth audit should not re-ask whether HTN is “good.” It should audit whether the accepted changes actually proved FND-14.

Suggested fourth-iteration focus:

1. Prove or falsify field-level source enforcement in `PlanningSnapshot`.  
2. Audit every `RuntimeBeliefView` method for source class and remove ambient authority.  
3. Run adversarial golden tests for owner/control, record/office, reservation/queue, container/possessor, seller stock, and route-danger false beliefs.  
4. Verify normal player UI cannot import omniscient `display.rs`, raw `World`, or debug trace data.  
5. Re-audit HTN after source hardening: method selection must trace lawful inputs, fallback legality, and rejected methods.  
6. Re-audit repair only after causal links stop using placeholder event IDs.  
7. Measure snapshot size, trace volume, and candidate fan-out on a larger synthetic world with at least dozens of agents and locations.

The next release-blocking standard should be simple: **no AI decision, player action menu, or normal UI surface may change because of a remote authoritative fact unless a lawful carrier changed the character’s belief.**

