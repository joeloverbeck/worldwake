

# **First-Iteration Hostile AI Architecture Audit — Worldwake**

## **1. Executive Verdict**

**Verdict: keep the core architecture, but stop adding behavior until authority boundaries are tightened.** Worldwake’s live AI stack is not a doomed Frankenstein. It is a working hybrid with a mostly sound tactical GOAP core, a real belief-view wall, local affordance enumeration, explicit action definitions, and rich trace surfaces. But it is **becoming architecturally split-brained** around four seams:

1. **Belief-boundary enforcement is the highest-risk area.** The architecture intends belief-local planning, but some runtime belief accessors and snapshot admission paths are still too easy to misuse. The worst specific risk I found is `PerAgentBeliefView::effective_place()` falling back from “known entity” to current authoritative `world.effective_place(entity)` for non-self entities. That is exactly the kind of silent remote-truth leak FND-14A forbids if the entity is only known through stale memory, last-seen memory, or institutional mention. The FOUNDATIONS rule is explicit: delayed or off-place knowledge must be belief-backed, and authoritative non-co-located reads violate the design constitution.  
2. **HTN is useful but currently overnamed.** The live HTN layer is not a full executor or full ordered task-network planner. It is mostly **strategic-stage guidance**: methods are selected, subgoals are converted into location/acquisition stages, and the tactical planner still finds lawful actions. That is safer than an overpowered HTN, but it means parts of `MethodSchema` are currently decorative or under-enforced. Worse, `MethodPrecondition::AgentRole(_) => true`, and several `LocationKnown` variants return `false`, making some method declarations either too permissive or dead.  
3. **Goal/candidate/ranking responsibilities are too spread out.** `GoalSchema`, `GoalKindPlannerExt`, candidate emitters, ranking, agenda entries, opportunity compilation, blocker/discrepancy memory, and HTN methods all encode pieces of “why this goal exists, how it is pursued, when it is satisfied, how it is ranked, and what failure means.” Some of that is well-centralized, especially ranking’s total-order authority, but the overall semantic ownership is not clean enough for hundreds of agents and hundreds of locations.  
4. **The architecture has live fossil seams.** The clearest one is `GoalSchema.methods`: the schema contains a `methods` field, while the integration test requires all live dispatch declarations to keep that field empty because method assignment belongs to the method registry. That is not fatal today, but it is exactly the kind of “two live-looking authorities for one concept” that FND-28 warns against.

Severity ratings:

| Severity | Finding |
| ----- | ----- |
| **Critical** | Possible remote authoritative truth leak through belief-view location access; snapshot and planner code then amplify any leak because many strategic functions scan admitted snapshot entities. |
| **High** | HTN method declarations are richer than the live enforcement path; some preconditions are no-ops or unsupported. |
| **High** | Goal semantics are split across schema, `GoalKindPlannerExt`, candidate generation, ranking, HTN, and terminal/progress-barrier handling. |
| **High** | `GoalSchema.methods` is a fossil-looking authority field deliberately kept empty while `MethodRegistry` is live. |
| **Medium** | Candidate generation is side-effect-free in implementation mechanics, but semantically mixes candidate emission with discrepancy/violation/source-failure detection and pending memory writes. |
| **Medium** | Ranking is centralized but score-heavy; without portfolio slots and better explanation contracts, it can become abstract-score creep. |
| **Medium** | Diagnostics are broad but not yet sufficient for “why not?” questions across belief, method rejection, player action visibility, and aggregate scenario failure. |
| **Low / Medium** | Per-call HTN registry construction and shallow method traces are not currently dangerous, but they signal that HTN has not yet been made a stable first-class architecture layer. |

The recommended path is **Option B — Moderate Reshaping**: preserve GOAP, preserve HTN as lawful search-control, add an explicit BDI-shaped deliberation shell, collapse fossil authority seams, add static/type-level belief protections, and build golden scenarios before adding more AI behavior.

---

## **2. Repository Discovery Log**

### **Active docs and contracts inspected**

| File | Use in audit |
| ----- | ----- |
| `docs/FOUNDATIONS.md` | Treated as design constitution. FND-1, FND-7, FND-8, FND-14/14A, FND-19, FND-20, FND-21, FND-26, FND-28, FND-29/29A, and FND-31 are directly implicated. |
| `docs/planner-contracts.md` | Planner-facing contract for terminal surfacing, snapshot completeness, belief-backed travel cost, and traces. |
| `docs/spec-drafting-rules.md` | Confirms that planner-facing specs must state whether behavior is GOAP, HTN with fallback, or method-required, and that method-required needs a schema contract. |
| `AGENTS.md` | Confirms repository invariants: no `Player` type, belief-only planning, information locality, append-only event log, no compatibility fossil layers, and full AI-pipeline verification for validation/control changes. |
| `.claude/skills/goap-architecture-report/SKILL.md` | Used only as an audit checklist/report-generation guide, not as architecture authority. |

### **Active reports used**

| File | Use in audit |
| ----- | ----- |
| `reports/goap-architecture-report.md` | Used as a current report but cross-checked against code. It was useful for pipeline overview and known budget/diagnostic concerns. |
| `reports/ai-architecture-improvements.md` | Used as current prior analysis. It recommends a BDI shell, data-driven goal schemas, HTN as lawful search control, utility/portfolio triage, and stronger diagnostics. I agree with the broad direction but did not treat it as authoritative. |

### **Active code surfaces inspected**

| Area | Files / modules inspected |
| ----- | ----- |
| Core goal types | `crates/worldwake-core/src/goal.rs` |
| Candidate/agenda/ranking | `crates/worldwake-ai/src/candidate_generation.rs`, `agenda_types.rs`, `ranking.rs`, `goal_policy.rs`, `goal_schema.rs`, `goal_schema_registry.rs`, `goal_model.rs` |
| HTN | `crates/worldwake-ai/src/htn/mod.rs`, `method_schema.rs`, `methods.rs`, `registry.rs`, `selector.rs` |
| GOAP/strategic/tactical search | `crates/worldwake-ai/src/search/mod.rs`, `search/strategic.rs`, related search references from contracts |
| Planning snapshot/state | `crates/worldwake-ai/src/planning_snapshot.rs`, `planning_state.rs` surfaces via search and contract |
| Belief/perception surfaces | `crates/worldwake-sim/src/belief_view.rs`, `per_agent_belief_view.rs`, `affordance_query.rs` |
| Action definitions | `crates/worldwake-sim/src/action_def.rs`, relevant action surface references in `crates/worldwake-systems/src/*` search results |
| Tests | `crates/worldwake-ai/tests/integration/goal_schema_methods.rs`, plus active reports/contracts naming golden and trace expectations |

### **Archived files read**

**None.** Search results surfaced archive paths, but I did not open or rely on archived file contents. One active file, `AGENTS.md`, references `archive/specs/S154-test-binary-consolidation.md`, but that reference concerns cargo artifact hygiene/test-binary bloat, not this AI architecture audit, so it was not relevant and was not read.

### **Important areas not inspected sufficiently**

This is a first-iteration audit, not a full implementation review. I did not fully inspect every function body in:

* `agent_tick/planning.rs`  
* `decision_runtime.rs`  
* `decision_trace.rs`  
* `goal_switching.rs`  
* `agenda_manager.rs`  
* `frame_switch_policy.rs`  
* `plan_guard.rs`  
* `plan_guard_build.rs`  
* `plan_revalidation.rs`  
* `plan_repair.rs`  
* `failure_handling.rs`  
* `opportunity_compiler.rs`  
* every active `crates/worldwake-systems/src/*_actions.rs` action handler  
* all golden tests and scenario diagnostics

That uncertainty matters most for the sections on interruption, local repair, blocker churn, and trace sufficiency. The architectural verdict is still strong enough because the main seams are visible in the active contracts and central AI code.

---

## **3. Research Synthesis**

### **GOAP in games: keep it tactical, do not let it become the whole mind**

The F.E.A.R.-style GOAP lesson is still relevant: GOAP works because goals and actions are modular, action preconditions/effects let the planner discover sequences at runtime, and local world knowledge can support reactive replanning. The important lesson is not “GOAP should own everything”; it is “runtime action sequencing beats hard-coded transition graphs when leaves are ordinary world actions.” F.E.A.R. used A* for action-sequence planning, not just pathfinding, and kept a very small execution FSM, with the planner choosing parameters and transitions.

**Implication for Worldwake:** the tactical GOAP core should stay. Worldwake’s `search/mod.rs` already has strategic guidance, tactical goals, candidate expansion, FF/landmark heuristics, beam truncation, terminal/barrier handling, and trace metadata. That is a real planner, not a toy behavior selector.

**What it does not imply:** GOAP should not own motive discovery, long-term intention commitment, social memory, or institutional task semantics. GOAP is the lawful action-sequence engine. It should not become the architecture’s entire psychology.

### **HTN planning: useful only when methods are real decompositions, not labels**

HTN planning decomposes abstract or compound tasks into subtasks until executable primitive tasks are reached. SHOP2 is a canonical HTN planner; its paper describes a system that won recognition in the 2002 International Planning Competition and emphasizes domain methods as the reason HTN can efficiently structure planning. General HTN descriptions make the same point: the solution is an executable sequence of primitive tasks produced by decomposing higher-level task networks.

**Implication for Worldwake:** HTN is justified when it encodes a reusable lawful pursuit pattern: investigation before enforcement, acquisition before craft, proof before bounty claim, coordination before dangerous confrontation. The current HTN module contains schemas for those patterns, including bounty, production, restock, investigation, and escort methods.

**What it does not imply:** a method name is not enough. If the live system converts subgoals mostly into strategic places and does not enforce leaves, artifacts, claims, or failure modes, then it is not full HTN authority. That can be fine, but the code and docs must say “strategic method guidance,” not pretend method leaves are binding.

### **BDI: the right deliberation wrapper, but not a magic import**

BDI separates beliefs, desires, and intentions; intention formation is about commitments that persist across time and constrain future deliberation. Rao and Georgeff’s work explicitly connects deliberation to the formation of intentions, which are plans of action an agent is committed to achieving. BDI software architecture also separates plan selection from execution of active plans, which is exactly the seam Worldwake needs.

**Implication for Worldwake:** Worldwake already has most BDI ingredients: belief views, candidate goals/desires, agenda state, committed/suspended entries, planning, revalidation, repair, and failure memory. `AgendaState` has committed, pending, and suspended entries; `AgendaEntry` stores offers, priority class, motive score, provenance, feasibility, and optional partial plan segment.

**What it does not imply:** adopting BDI jargon should not add a new framework layer that fights GOAP. The recommendation is a **BDI-shaped ownership boundary**, not a wholesale rewrite into a BDI engine.

### **Utility and motive scoring: useful for triage, dangerous as truth**

Utility scoring is appropriate for bounded local choice when every score is traceable to concrete agent-local state: needs, fear, obligations, memories, trust, learned blockers, opportunity evidence. It becomes dangerous when a derived score replaces the world fact it summarizes. Worldwake’s constitution permits local summaries and heuristics derived from accessible belief state, but it forbids abstract scores as authoritative truth.

**Implication for Worldwake:** ranking should remain centralized and explainable, but planning breadth should move toward portfolio slots rather than one global score soup. The current ranking module has a clear total-order authority, which is good, but it also has domain-heavy motive computation and several hard-coded tuning constants.

**What it does not imply:** do not replace concrete scarcity, danger, duty, or opportunity with utility numbers. Utility should decide which motive gets planning budget, not whether a fact exists.

### **Multi-agent coordination: use artifacts, queues, bids, and grants — not invisible locks**

The Contract Net Protocol is a classic multi-agent task allocation mechanism: a manager announces a task, contractors propose, and the manager awards the task. MAPF research similarly treats multi-agent movement as a space-time coordination problem, often with discrete move/wait actions and collision-free paths; practical systems emphasize scalable, bounded online coordination rather than perfect global optimization.

**Implication for Worldwake:** task allocation, queues, reservations, warrants, route claims, contracts, office assignments, and grants should be **world artifacts or world state**, not invisible planner privileges. Worldwake’s action definitions already include reservation requirements and contention-related surfaces; `affordance_query.rs` derives queue/grant/full/available contention status from belief/runtime views.

**What it does not imply:** do not introduce an omniscient town manager or global scheduler that tells NPCs what to do. Coordination must remain local, observable, and contestable.

### **Explainable planning: traces must answer contrastive “why not?” questions**

Explainable AI planning literature emphasizes that users need to understand what the planner was trying to achieve and why; contrastive work notes that people often ask “why this rather than that?”

**Implication for Worldwake:** traces must answer rejected candidates, omitted candidates, method rejection, fallback, stale belief use, missing knowledge, action legality, and player action visibility — not just “chosen plan was X.” Worldwake already has trace contracts for omitted operators, root candidates, route pruning, and same-goal selection provenance, but the HTN, belief-boundary, and player-facing “why not visible?” surfaces are still insufficient.

**What it does not imply:** do not bolt on a generic explanation layer after the fact. The explanation must be reconstructable from causal state, belief state, planner-local traces, and append-only history.

---

## **4. Current Architecture Map**

Worldwake currently has a layered hybrid architecture:

Perception / testimony / records / local observation  
   ↓  
Agent belief store + RuntimeBeliefView / GoalBeliefView  
   ↓  
PlanningSnapshot / PlanningState  
   ↓  
Opportunity compiler + candidate generation  
   ↓  
Ranking / suppression / motive scoring  
   ↓  
Agenda / intention / continuation / switching  
   ↓  
HTN method selection as strategic-stage guidance  
   ↓  
Strategic search: prerequisite/location stages  
   ↓  
Tactical GOAP search over ActionDef affordances  
   ↓  
Revalidation / guards / dispatch  
   ↓  
Action lifecycle / scheduler / contention / event log  
   ↓  
Perception and belief updates again

Layer-by-layer responsibility table:

| Layer | Current files/types | Reads | Writes | Authority owned | Risk |
| ----- | ----- | ----- | ----- | ----- | ----- |
| Perception and belief acquisition | `worldwake-systems/src/perception.rs`, epistemic/report/ask actions by search; `AgentBeliefStore`, `LastSeenMemory`, `ToldBeliefMemory` | Authoritative local observations, testimony, records, event aftermath | Belief stores, memories, records, social observations | Belief/memory world state | Not fully inspected; if perception omissions are not traceable, “why did agent not know?” fails. |
| Belief storage / belief view | `belief_view.rs`, `per_agent_belief_view.rs`, `RuntimeBeliefView`, `GoalBeliefView`, `BeliefRead`, `BeliefStatus` | `World`, `AgentBeliefStore`, same-tick local physical state | Read-only view outputs | Belief-facing access boundary | Highest risk. `effective_place()` can fall back to authoritative current location for known non-self entities. |
| FND-14A exception handling | `PerAgentBeliefView::has_authoritative_local_visibility`, local observation methods | Co-located physical state | Observed reads | Same-tick local physical observation | Conceptually right, but enforcement is by convention and tests, not by type system. |
| Motive / goal discovery | `candidate_generation.rs`, `goal_schema.rs`, `opportunity_compiler.rs`, `motive_source_mapping.rs` | GoalBeliefView, memories, opportunities, enterprise signals, discrepancies | `GoalOffer`, diagnostics, pending memory records | Desire/candidate discovery | Candidate generation is too broad; it emits goals and detects anomalies/violations/pending memory updates. |
| Candidate generation | `GoalOffer`, `GenerationContext`, `CandidateGenerationResult`, candidate extractors | Beliefs, local place, travel horizon, blocker/discrepancy memory, recipes, opportunities | Candidate list + diagnostics + pending records | Candidate existence and evidence traces | Risks becoming emitter sprawl and mixed read/write semantics. |
| Ranking / prioritization | `ranking.rs`, `GoalPolicy`, `UtilityProfile`, `DecisionContext` | Candidates, needs, danger, memories, trust/reliability, utility profile | Ordered `AgendaEntry`s, suppressed/damped/zero-motive diagnostics | Sole total order over agenda entries | Centralized, which is good; score semantics still too diffuse and tuning-heavy. |
| Agenda / intention persistence | `agenda_types.rs`, `agenda_manager`, `goal_switching`, `frame_switch_policy` | Ranked entries, current commitment, feasibility, partial plan | Committed/pending/suspended agenda entries | Commitment lifecycle | Good BDI ingredients, but ownership overlaps with plan state and repair memory. |
| HTN method selection | `htn/method_schema.rs`, `htn/methods.rs`, `htn/selector.rs`, `htn/registry.rs` | GoalOffer, profile, RuntimeBeliefView, motive refs, recipes | Selected method reference + trace | Method choice / strategic decomposition hint | Useful but under-enforced; role preconditions no-op, some location criteria impossible. |
| HTN decomposition | `search/strategic.rs` | Selected method subgoals, snapshot/planning state | Strategic stages | Search-control stages | Not full HTN leaves; `PerformAction` becomes location guidance, not enforced action sequence. |
| Strategic search | `search/strategic.rs` | Snapshot places/entities/costs, method stages, missing commodities | `StrategicPlan`, budget trace, method trace | Location/prerequisite itinerary | Scans snapshot entities; safe only if snapshot admission is perfect. |
| Tactical GOAP search | `search/mod.rs`, `search/candidates.rs`, `search/transition.rs`, landmarks/heuristics | PlanningState, ActionDef affordances, semantics, recipes, budgets | `PlannedPlan`, search trace | Lawful action-sequence planning | Strong core. Risk is semantic overload from goal/model/schema. |
| Action affordance enumeration | `affordance_query.rs`, `action_def.rs` | RuntimeBeliefView, ActionDefRegistry, ActionHandlerRegistry | `Affordance`s | Player/AI legal action surface | Good alignment with FND-8; must be the human UI action source too. |
| Action definitions | `ActionDef`, `Precondition`, `TargetSpec`, `ReservationReq`, guard/expectation templates | Static registry + runtime/belief view | Definitions; execution uses handlers | Preconditions/duration/cost/contention declarations | Strong. Should not be bypassed by HTN or UI. |
| Planning snapshot/state | `planning_snapshot.rs`, `planning_state.rs` | RuntimeBeliefView, evidence sets, travel horizon | Snapshot maps, route matrices, belief-backed state | Planner-local derived state | Necessary, but needs admission-source metadata and stronger static boundary. |
| Revalidation / guards | `plan_revalidation.rs`, `plan_guard.rs`, `plan_guard_build.rs`, action commit conditions | Current belief/runtime legality, plan assumptions | Valid/invalid plan status, guard traces | Pre-dispatch legality | Not fully inspected; should own “plan still legal?” not goal ranking. |
| Interruption / replanning / repair | `interrupts.rs`, `goal_switching.rs`, `plan_repair.rs`, `failure_handling.rs`, blockers/discrepancies | Current intention, failures, blockers, new evidence | blockers, repair memory, suspended/resumed intent | Failure response | Likely necessary; risk of local repair duplicating full planning. |
| Blocker/discrepancy memory | `BlockerMemory`, `DiscrepancyMemory`, `RepairMemory` | Failed plans, violated expectations, observations | Agent-local memory | Failure memory, not world truth | Must stay agent-local and evidence-backed; candidate generation currently records pending discrepancies. |
| Opportunity compilation | `opportunity_compiler.rs`, `LearnedOpportunityMemory` | Beliefs/memories/events | opportunities, learned bonuses | Derived opportunity cache | Risk of duplicating candidate generation unless schema-driven. |
| Contention/reservation/queues | `ReservationReq`, `ContentionGrant`, queues in runtime views/action systems | World queues/reservations/grants | queue/grant/reservation state | Scarce affordance arbitration | Good direction; intention must never silently reserve. |
| Dispatch / scheduler | `tick_step.rs`, `tick_action.rs`, action handlers | selected action, world legality | action instance, events, state changes | Authoritative world mutation | Must remain same for AI and human control. |
| Decision / causal traces | `decision_trace.rs`, search traces, candidate diagnostics, event log | decisions, omissions, causal events | traces, summaries | Debug/explanation | Rich, but still missing method rejection, belief-admission provenance, UI visibility explanations. |
| Human-control/player POV | `ControlSource`, CLI, affordance query | current controlled agent belief/action view | selected action | Choice source only | Future UI must not read world truth or debug traces outside debug mode. FOUNDATIONS is explicit: control source changes chooser, not laws. |

---

## **5. Hostile Failure Inventory**

### **1. Remote truth leak through `PerAgentBeliefView::effective_place`**

**Symptom:** For non-self entities, `effective_place()` returns belief-store `last_known_place`, but then falls back to `self.knows_entity(entity).then(|| self.world.effective_place(entity))`. Since `knows_entity()` includes cases beyond same-tick co-location, this can leak current authoritative location for entities known through stale or indirect paths.

**Why it matters:** Planning snapshots and HTN/strategic code rely heavily on `effective_place()`. One leak becomes route choice, method selection, candidate emission, or action visibility.

**FOUNDATIONS principle implicated:** FND-14, FND-14A, FND-15, FND-16.

**Likely downstream failure mode:** An agent last saw a bandit in the orchard, the bandit moves to the mill, and the planner “believes” the current mill location without any perception, testimony, trail, or record. The resulting behavior looks smart but is omniscient.

**Severity:** **Critical**

**Status:** Real issue or very high-confidence risk. Needs immediate focused test.

---

### **2. Snapshot admission lacks source metadata**

**Symptom:** The planner snapshot admits actor, evidence entities, places, co-located entities, remembered entities, and possession/container relations, but the admitted `entities` map does not appear to carry an explicit source such as `LocalSameTick`, `BeliefStore`, `LastSeen`, `GroundedEvidence`, `Topology`, or `HypotheticalEffect`. `collect_entities()` and downstream strategic code then treat admitted entities as a uniform set.

**Why it matters:** Strategic functions scan `state.snapshot().entities.keys()` for workstations, sellers, resources, and acquisition places. That is legal only if every field on every admitted entity is belief-correct.

**FOUNDATIONS principle implicated:** FND-14, FND-27, FND-29.

**Likely downstream failure mode:** A remote entity enters as “evidence carrier,” but later code reads unrelated authoritative fields from it because the snapshot no longer remembers why it was admitted.

**Severity:** **High**

**Status:** Real design gap.

---

### **3. HTN is not full HTN authority, but the schema reads like it is**

**Symptom:** Methods define preconditions, subgoals, expected artifacts, required claims, failure modes, explanation templates, motive bias, and budget hints. But strategic planning mostly converts subgoals to stages; `PerformAction` resolves to places, and tactical GOAP still chooses the actual action sequence.

**Why it matters:** The schema promises more semantic structure than the live system enforces. That creates false confidence.

**FOUNDATIONS principle implicated:** FND-20, FND-29, FND-31.

**Likely downstream failure mode:** A method appears to require proof before bounty claim, but flat tactical search or fallback may satisfy a related goal without the declared artifact/claim sequence. Or the trace says a method was selected, but the plan did not actually execute its leaves.

**Severity:** **High**

**Status:** Real issue.

---

### **4. HTN preconditions are partially fake**

**Symptom:** `MethodPrecondition::AgentRole(_) => true`. Several `LocationKnown` variants — witness, violation evidence, ledger — return `false` in selector precondition evaluation.

**Why it matters:** Role-specific methods are not role-specific. Some declared methods may be unreachable. This is not an HTN theory problem; it is a live semantic mismatch.

**FOUNDATIONS principle implicated:** FND-20, FND-22, FND-31.

**Likely downstream failure mode:** Group-hunt methods can be selected by non-hunters; on-scene investigation or office escort methods may never select despite apparently valid schemas.

**Severity:** **High**

**Status:** Real issue.

---

### **5. `GoalSchema.methods` is a fossil seam**

**Symptom:** `GoalSchema` has `methods: &'static [MethodSchemaId]`, but the integration test requires every live dispatch declaration to expose empty method anchors because method assignment belongs to the method registry.

**Why it matters:** It looks like schema owns method assignment, but code says registry owns it. That is exactly how fossil authority paths start.

**FOUNDATIONS principle implicated:** FND-28.

**Likely downstream failure mode:** Future work populates `GoalSchema.methods` and forgets registry, or vice versa, creating two incompatible method assignment surfaces.

**Severity:** **High**

**Status:** Real issue.

---

### **6. Candidate generation mixes desire emission with anomaly and memory-update detection**

**Symptom:** `CandidateGenerationResult` returns candidates and diagnostics, but also pending violations, pending discrepancies, source reliability failures, and acquisition-exhaustion resets. The file comments preserve read/write phase separation, but semantic responsibility is still mixed.

**Why it matters:** Candidate generation should answer “what might this agent want to do?” It should not quietly become the place where the world notices violations, discrepancies, and learned-source failures.

**FOUNDATIONS principle implicated:** FND-17, FND-18, FND-26, FND-29.

**Likely downstream failure mode:** A desire disappears because an anomaly detector changed memory in the same conceptual pass, or a discrepancy is recorded only if candidate generation happens to run that emitter.

**Severity:** **Medium / High**

**Status:** Real issue.

---

### **7. Goal satisfaction is split across too many mechanisms**

**Symptom:** Goal semantics live across `GoalKindPlannerExt::is_satisfied`, relevant ops, payload override, binding checks, progress barriers, synthesized root candidates, HTN methods, and tactical terminal kinds. The planner contract already has to explain exact-goal terminal surfacing separately because this boundary is easy to misdescribe.

**Why it matters:** A goal should have one schema-level statement of: what creates it, what satisfies it, what progress barriers count, what terminal actions are allowed, and what fallback is legal.

**FOUNDATIONS principle implicated:** FND-20, FND-28, FND-29.

**Likely downstream failure mode:** A goal becomes “satisfied” in one path but still pursuable in another; or a terminal action is synthesized without the same binding semantics as tactical search.

**Severity:** **High**

**Status:** Real issue.

---

### **8. Ranking is centralized but still vulnerable to abstract-score creep**

**Symptom:** Ranking has a strong total-order authority, which is good. But it uses utility profile, danger pressure, source reliability discounts, competition discounts, learned opportunity bonuses, repair memory bonuses, and hard-coded constants such as witness-gap weight and staleness normalization.

**Why it matters:** Scores are legal only as derived local reasoning. They become illegal if they start substituting for concrete hunger, danger, trust, obligation, scarcity, or evidence.

**FOUNDATIONS principle implicated:** FND-2, FND-3, FND-20, FND-22A.

**Likely downstream failure mode:** Agents behave plausibly in aggregate but no longer have explainable concrete reasons for choices.

**Severity:** **Medium**

**Status:** Real risk, not yet a proven bug.

---

### **9. HTN fallback policy is implicit**

**Symptom:** If no selected method produces stages, strategic search falls back to missing commodities and goal places. There is no visible method-required/fallback policy at the goal schema boundary.

**Why it matters:** FOUNDATIONS allows HTN but says method-required needs explicit schema contract and tests. The current system appears to treat fallback as generally legal.

**FOUNDATIONS principle implicated:** FND-20, FND-31.

**Likely downstream failure mode:** Future method-required behavior is added accidentally, but flat GOAP still bypasses it.

**Severity:** **High**

**Status:** Real issue.

---

### **10. Method traces are too shallow**

**Symptom:** `method_trace()` records selected method id, subgoals attempted as `Pending`, motive score, and no failure mode. It does not explain rejected methods, failed preconditions, why fallback happened, or whether method leaves were enforced.

**Why it matters:** For a hostile audit, “method selected” is not enough. The important questions are “why this method,” “why not the other method,” “which precondition failed,” and “did fallback happen?”

**FOUNDATIONS principle implicated:** FND-29, FND-31.

**Likely downstream failure mode:** HTN bugs look like GOAP bugs because the trace cannot distinguish method selection failure from tactical search failure.

**Severity:** **Medium / High**

**Status:** Real trace gap.

---

### **11. Player-POV symmetry is architecturally intended, but UI safety is not yet proven**

**Symptom:** FOUNDATIONS and `AGENTS.md` are explicit: no `Player` type; `ControlSource` changes chooser only; player-facing UI may show only what the current character can perceive, infer, remember, or obtain from records/testimony.

**Why it matters:** The future game constraint is not just philosophical. The human action UI must call the same belief/affordance surface as AI.

**FOUNDATIONS principle implicated:** FND-14, FND-19.

**Likely downstream failure mode:** A human-controlled character sees “go claim bounty” or “attack target at mill” because UI reads debug/world state, while AI would not know that.

**Severity:** **High**

**Status:** Suspected issue; UI code not sufficiently inspected.

---

### **12. `can_control` and action legality mix belief-facing and authoritative concerns**

**Symptom:** `ControlBeliefView::can_control()` uses authoritative `world.can_exercise_control()` after a local unowned-item shortcut, while `believed_rights()` has an explicit FND-14/FND-15 social-fact gate.

**Why it matters:** Dispatch must be authoritative. Planning/UI affordance visibility should be belief-local. These are not the same question: “can I legally control this?” vs “do I believe I can control this?” vs “will the world allow this action at commit?”

**FOUNDATIONS principle implicated:** FND-14A, FND-19, FND-24.

**Likely downstream failure mode:** Agent chooses an action because authoritative rights are true even though the agent has no belief path to those rights.

**Severity:** **Medium**

**Status:** Suspected issue; needs targeted tests.

---

### **13. Strategic search scans snapshot entity maps directly**

**Symptom:** Workstation, seller, resource-source, and acquisition-place functions scan `state.snapshot().entities.keys()`.

**Why it matters:** This is fine only if snapshot admission is airtight. Without admission metadata, these scans are brittle.

**FOUNDATIONS principle implicated:** FND-14, FND-27.

**Likely downstream failure mode:** A remote unobserved seller or resource source becomes a plan target because it was admitted for another reason.

**Severity:** **Medium / High**

**Status:** Real risk.

---

### **14. Diagnostics are rich but not yet complete**

**Symptom:** Planner contracts cover omitted operators, root candidate outcomes, route diagnostics, and same-goal branch attribution. Candidate generation has many diagnostics. Search has expansion summaries, root omissions, pruning traces, and cache counters.

**Why it matters:** The missing pieces are the exact ones needed for hostile architecture debugging: method rejection, fallback reason, belief-admission source, stale-belief use, “why did agent not know?”, and player action visibility.

**FOUNDATIONS principle implicated:** FND-29, FND-29A, FND-31.

**Likely downstream failure mode:** Runs look plausible, but developers cannot falsify whether behavior was emergent or leaked.

**Severity:** **Medium**

**Status:** Real gap.

---

### **15. False alarm: HTN is not currently bypassing action causality**

**Symptom:** The concern would be that HTN directly performs story beats or hidden success paths.

**Evidence:** Current strategic code selects a method, builds stages, then tactical search still uses ordinary planner candidates, successors, barriers, and `ActionDef` affordances.

**Why it matters:** Do not delete HTN just because it adds complexity. The dangerous version of HTN is not what this code currently does.

**FOUNDATIONS principle implicated:** FND-20.

**Severity:** **False alarm / Low**

**Status:** False alarm, with boundary caveats.

---

## **6. GOAP / HTN / BDI / Utility Responsibility Matrix**

| Responsibility | Should be owned by | Current owner | Problem? | Recommendation |
| ----- | ----- | ----- | ----- | ----- |
| Motive discovery | Motive/candidate extractor layer over belief/memory/evidence | `candidate_generation.rs`, opportunity compiler, schema extractors | Yes. Too broad and emitter-heavy. | Create explicit `MotiveRecord` / extractor registry; candidate generation should not own anomaly persistence. |
| Goal ranking | Ranking + portfolio triage | `ranking.rs`, `goal_policy.rs`, utility/profile state | Partly. Total order good; global score soup risk. | Keep ranking authority; add portfolio slots before expensive planning. |
| Intention persistence | BDI-shaped agenda/intention manager | `AgendaState`, goal switching, current plan, partial plans | Partly. Ingredients exist but ownership overlaps plan state. | Make agenda/intention explicitly own adopt/continue/suspend/abandon; plan is subordinate. |
| Method decomposition | HTN method registry + method selector | `htn/*`, `search/strategic.rs` | Yes. Schema richer than enforcement. | Keep HTN but rename/define as strategic method guidance unless leaves are enforced. |
| Action-sequence planning | Tactical GOAP | `search/mod.rs`, `search/transition.rs`, `ActionDef` affordances | Mostly clean. | Preserve. Do not move action legality into HTN or ranking. |
| Fallback planning | Goal schema fallback policy | Implicit in `search/strategic.rs` | Yes. Too implicit. | Add `MethodFallbackPolicy`: `AlwaysAllowed`, `AllowedWithTrace`, `ForbiddenBySchema`. |
| Failure attribution | Planner traces + method traces + blocker/discrepancy memory | Search traces, `failure_handling`, blockers, method trace | Partly. Method failure attribution weak. | Add method rejection/failure-mode trace and connect to blocker/discrepancy memory. |
| Repair | Repair manager under intention | `plan_repair`, `RepairMemory`, ranking bonus | Suspected overlap. | Repair may adjust current plan only if assumptions still hold; otherwise full replan. |
| Contention handling | World artifacts: queue/reservation/grant/contract | Action defs, runtime queues, affordance query | Mostly clean. | Preserve. Ensure intention never silently reserves. |
| Belief correction | Perception/testimony/evidence systems | Belief store, discrepancy/expectation systems, candidate generation pending records | Partly. Candidate generation too involved. | Move discrepancy/violation detection into belief/evidence update or dedicated anomaly pass. |
| Trace explanation | Causal history + belief traces + planner traces | Decision traces, planner contracts, event log | Partly. Missing why-not surfaces. | Add contrastive traces: rejected method, omitted candidate, stale belief, UI hidden action. |
| Player action visibility | Same belief/affordance surface as AI | Intended by FOUNDATIONS/AGENTS; CLI not fully inspected | Unproven. | Make player UI consume `RuntimeBeliefView` + `get_affordances_for_defs`, never raw world except debug. |

---

## **7. HTN Verdict**

### **Overall HTN verdict**

**HTN earns a place, but only as narrowed lawful search-control unless the implementation starts enforcing method leaves, artifacts, claims, and failure modes.**

The current layer solves a real problem plain GOAP does not solve cleanly: it can encode multi-stage pursuit patterns and restrict strategic search before tactical search explodes. That is aligned with FND-20. The problem is that the live code does not yet fully enforce the semantics that `MethodSchema` advertises.

### **HTN-covered goal/pattern classification**

| Method / pattern | Classification | Verdict |
| ----- | ----- | ----- |
| `fulfill_bounty_direct` | **HTN justified but boundary needs tightening** | The pattern is lawful: know bounty, know target, acquire weapon, travel, observe, attack, claim. But proof/claim requirements are not clearly enforced as method-required semantics. Flat GOAP fallback should remain legal for now. |
| `fulfill_bounty_investigation` | **HTN justified but boundary needs tightening** | Information gathering before action is exactly a good HTN use. Needs better witness/evidence location semantics and method failure attribution. |
| `fulfill_bounty_group_hunt` | **Insufficient evidence / boundary needs tightening** | It declares `AgentRole(Hunter)`, but role precondition currently always passes. It uses `DeclareSupport` as a placeholder for recruitment. Do not treat this as a real group-hunt method yet. |
| `produce_from_owned_stock` | **HTN optional; flat GOAP fallback should remain legal** | Crafting from owned inputs is probably short enough for GOAP. HTN can be a budget hint, not a semantic requirement. |
| `produce_with_gather` | **HTN justified; fallback legal** | Gathering input before crafting is a reusable pursuit pattern and can narrow search. |
| `produce_with_purchase` | **HTN justified; fallback legal** | Purchase-before-craft is a useful decomposition; fallback remains legal if GOAP can find trade/craft lawfully. |
| `restock_from_harvest` | **HTN justified but optional** | Good search-control for merchant/resource behavior. Not method-required. |
| `restock_from_market` | **HTN justified but optional** | Good search-control. Not method-required. |
| `investigate_on_scene` | **HTN justified but currently suspect** | On-scene evidence inspection is a good pattern, but selector returns false for `LocationKnown(ViolationEvidence)` variants, so the method may be dead. |
| `investigate_by_witness` | **HTN justified and should remain** | Witness-before-investigation is a strong lawful pattern. Needs better trace and fallback policy. |
| `investigate_by_ledger` | **HTN justified and should remain** | Ledger/office investigation is a good institutional method. Needs belief-backed office/record access tests. |
| `escort_to_home` | **HTN justified but boundary needs tightening** | Escort destination semantics matter. If home is required by goal schema, enforce it; otherwise fallback legal. |
| `escort_to_office` | **Insufficient evidence / likely dead until `LocationKnown(Ledger)` works** | Current selector returns false for ledger location criteria. Do not rely on this method until fixed. |

### **Method-required recommendations**

**No current HTN-covered goal should be marked method-required yet.**

That is a hostile conclusion: if the code does not enforce method leaves and artifacts, no method deserves method-required authority. A future method-required goal is valid only when all four conditions hold:

1. **Flat fallback is semantically invalid.** Example: “claim bounty reward” may be method-required only if the schema proves that proof acquisition, issuer/office access, and reward claim are part of the goal’s meaning, not merely one route to it.  
2. **The schema contract states why.** It must name required artifacts, claims, invalidators, fallback policy, and terminal operators.  
3. **Golden tests prove bypass is impossible.** Tests must construct a case where flat GOAP could find a tempting shortcut and assert that it is rejected with a method-required trace.  
4. **The trace explains the constraint.** The agent trace must say: selected method X because preconditions Y; flat fallback forbidden because schema contract Z; failed because artifact/claim/precondition W.

Until then, all HTN methods should be treated as **lawful search-control with fallback**, not semantic authority.

---

## **8. Belief Boundary and Player-POV Audit**

### **Direct world-read risks**

The biggest concrete risk is `PerAgentBeliefView::effective_place()` for non-self entities. The current implementation returns belief-store `last_known_place`, but can then fall back to current authoritative `world.effective_place(entity)` whenever `knows_entity(entity)` is true. Because `knows_entity()` includes last-seen memory and institutional belief subjects, this can turn stale knowledge into live location truth.

This is not a theoretical nit. The planner contract says remote entities must enter snapshots through remembered beliefs or explicit grounded evidence, not raw remote truth.

### **Remote truth admission risks**

`PlanningSnapshot` is the right idea, but it needs source tagging. The planner should know whether an entity/place/field is present because of:

* same-tick co-location  
* actor self-authoritative state  
* belief store claim  
* last-seen memory  
* grounded evidence carrier  
* topology/public structure  
* hypothetical planner effect

Without that source, functions scanning `snapshot.entities` cannot prove they are not using a field admitted for the wrong reason.

### **Social/institutional knowledge risks**

The architecture mostly understands this. `has_authoritative_local_visibility()` explicitly says it must not gate social/relational facts; `believed_rights()` says effective rights are social/jurisdictional and require an explicit belief gate.

The residual risk is `can_control()`: dispatch needs authoritative legality, but planning/UI need belief-visible legality. Keep those surfaces separate:

believed_can_attempt_control(actor, entity) -> belief/UI/planning affordance  
authoritative_can_exercise_control(actor, entity) -> dispatch/commit validation

### **UI/player-facing risks**

The future UI must not ask “what is true?” It must ask:

What can the controlled character perceive?  
What does the controlled character remember?  
What records/testimony can the character consult?  
What affordances are visible through the same view the AI uses?  
What action will the authoritative dispatcher accept if attempted?

The normal player UI must not read raw `World` for:

* remote entity location  
* remote inventory  
* ownership/effective rights  
* bounty validity unless learned through a record/testimony  
* office holder/support claims unless believed  
* route danger unless learned  
* hidden candidates omitted by belief gaps  
* debug traces from other agents

Debug/replay/authoring tools can read truth, but they must be type- or mode-separated from normal play.

### **Suggested static/type-level protections**

Add explicit wrappers:

pub enum PlannerReadSource {  
   SelfAuthoritative,  
   LocalSameTickPhysical,  
   BeliefStoreClaim,  
   LastSeenMemory,  
   Testimony,  
   Record,  
   GroundedEvidenceCarrier,  
   PublicTopology,  
   HypotheticalPlannerEffect,  
}

pub struct PlannerVisible<T> {  
   pub value: T,  
   pub source: PlannerReadSource,  
   pub acquired_tick: Option<Tick>,  
   pub claimed_event_tick: Option<Tick>,  
   pub confidence: Option<Permille>,  
}

Then split view traits:

trait LocalPhysicalObservationView { ... }       // same-tick co-located physical only  
trait AgentBeliefReadView { ... }                // belief/memory/testimony/records  
trait PublicStructureView { ... }                // topology, public immutable structure  
trait DispatchAuthorityView { ... }              // commit-time truth, not planning/UI

And add a hard architectural rule:

worldwake-ai must never receive &World.  
worldwake-ai may receive only RuntimeBeliefView, GoalBeliefView, PlanningSnapshot, and explicit debug-only trace readers.

### **Test scenarios proving player/AI symmetry**

1. **Stale target location:** AI and human-controlled versions of the same character both last saw a target at A. Target moves to B. Neither may see B as a possible target location until new evidence arrives.  
2. **Unknown ownership:** Character stands next to a chest. UI may show physical chest actions but not owner/effective-right facts unless belief-backed.  
3. **Bounty board visibility:** Character who has not seen or heard the bounty cannot see bounty actions. Character who read the board can.  
4. **Control-source swap:** Switch `ControlSource::Ai` to `Human` mid-intention. Available actions and legality remain identical.  
5. **Debug separation:** Debug overlay can show truth, but normal action UI cannot consume the debug source.

---

## **9. Consolidation or Redesign Options**

### **Option A — Conservative Consolidation**

Minimal changes: document boundaries, add tests, add traces, and avoid broad code restructuring.

**Benefits**

* Low churn.  
* Preserves working behavior.  
* Good near-term safety if the team wants to keep shipping small features.

**Risks**

* Does not remove responsibility overlap.  
* Belief-boundary safety remains mostly convention/test-driven.  
* HTN remains semantically ambiguous.

**Migration cost**

Low to medium.

**FOUNDATIONS alignment**

Improves FND-29 and FND-31, but only partially addresses FND-14/FND-28.

**Test impact**

Adds focused and golden tests. Minimal expected code breakage.

**Choose when**

You want one stabilization sprint before larger AI refactoring.

---

### **Option B — Moderate Reshaping**

Keep the tactical GOAP planner and current action surface. Keep HTN, but narrow its authority. Introduce explicit BDI ownership boundaries, source-tagged planner visibility, schema-owned fallback policy, and portfolio triage.

**Benefits**

* Fixes the actual architecture seams without rewriting the planner.  
* Makes HTN honest.  
* Makes belief leaks easier to catch.  
* Scales better toward hundreds of agents.  
* Aligns with the current active improvement report without treating it as gospel.

**Risks**

* Requires touching central files.  
* Some tests will expose previously hidden omniscience or method assumptions.  
* Requires discipline to avoid “new layer, same ambiguity.”

**Migration cost**

Medium.

**FOUNDATIONS alignment**

Strong. Directly addresses FND-14, FND-20, FND-21, FND-28, FND-29, FND-31.

**Test impact**

Moderate. Expect changes to AI golden traces, not necessarily world outcomes.

**Choose when**

This is my recommendation.

---

### **Option C — Aggressive Redesign**

Rebuild the AI stack around a schema-first BDI/HTN architecture: motives, goals, method policies, satisfaction predicates, and diagnostics become data-driven schema objects; GOAP becomes only a tactical executor.

**Benefits**

* Cleanest conceptual model.  
* Best long-term extensibility.  
* Reduces emitter sprawl and semantic duplication.

**Risks**

* High churn.  
* Easy to destroy working behavior.  
* Requires a strong migration plan and many golden scenarios.  
* Premature if the immediate belief-boundary bug is unfixed.

**Migration cost**

High.

**FOUNDATIONS alignment**

Potentially excellent, but only if done carefully. A rushed rewrite would violate the anti-workaround spirit of FOUNDATIONS.

**Test impact**

Large. Many golden tests must be rewritten or re-baselined.

**Choose when**

After Option B reveals that current candidate/ranking/schema seams cannot be stabilized.

---

## **10. Recommended Architecture**

### **Preserve**

* Tactical GOAP search over ordinary `ActionDef` affordances.  
* `RuntimeBeliefView` / `GoalBeliefView` as the planner-facing boundary.  
* `PlanningSnapshot` as the planner-local state object.  
* Explicit `ActionDef` preconditions, duration, costs, reservations, interruptibility, guard and expectation templates.  
* Centralized ranking total-order authority.  
* Agenda/intention concepts.  
* HTN methods as lawful strategic search-control.  
* Existing trace infrastructure.

### **Remove or collapse**

* `GoalSchema.methods` as a live-looking but deliberately empty authority field.  
* Fake HTN preconditions: no-op `AgentRole`, permanently false `LocationKnown` variants.  
* Any planner or UI path that can get authoritative remote entity state through belief-view fallback.  
* Any future candidate emitter that records memory/discrepancy/violation state as a side effect of desire emission.

### **Consolidate**

* Goal semantics into `GoalSchema`: creation extractors, satisfaction predicate, progress barriers, terminal operators, fallback policy, method policy, invalidators, trace contract.  
* Candidate generation into extractor families rather than an ever-growing emitter pile.  
* Ranking into portfolio triage plus total order inside slots.  
* HTN method selection into traceable method choice with rejection reasons.

### **Redesign**

* Planner-visible source tagging.  
* Belief/dispatch split for control/legal affordances.  
* Method fallback policy.  
* Aggregate diagnostics dashboard.

### **Defer**

* Full HTN execution with enforced ordered leaves.  
* Large-scale multi-agent route reservation systems.  
* LLM or generative planning integration.  
* Learning beyond current memory/reliability/opportunity mechanisms.

### **Do not touch yet**

* Tactical search internals unless golden tests reveal concrete search bugs.  
* Action definition format except to add source/trace hooks if needed.  
* Scheduler/action lifecycle unless player-symmetry tests fail there.

Target decision pipeline:

World events / perception / testimony / records  
   ↓  
Agent-local belief store + memory + evidence provenance  
   ↓  
Motive extractor registry  
   ↓  
MotiveRecord set  
   ↓  
Portfolio triage:  
   active intention / survival / danger / duty / economy / social / exploration / local opportunity  
   ↓  
Agenda / intention manager:  
   continue / adopt / suspend / abandon  
   ↓  
GoalSchema:  
   satisfaction, invalidators, terminal ops, fallback policy, method policy  
   ↓  
HTN method selector:  
   reusable pursuit pattern, rejection trace, fallback policy  
   ↓  
Strategic stage planner:  
   belief-visible places/prerequisites only  
   ↓  
Tactical GOAP:  
   ordinary ActionDef affordances, no privileged HTN leaves  
   ↓  
Revalidation:  
   guards, assumptions, contention, current legality  
   ↓  
Dispatch:  
   authoritative action lifecycle, queues, reservations, events  
   ↓  
Append-only causal history + belief update  
---

## **11. File-by-File Proposal**

| File / Module | Proposed change | Confidence | Reason | Acceptance criteria |
| ----- | ----- | ----- | ----- | ----- |
| `crates/worldwake-sim/src/per_agent_belief_view.rs` | Fix non-self `effective_place()` so authoritative location is returned only for self, same-tick local physical visibility, or direct possession/control cases where the actor can physically observe the object. Otherwise return belief/last-seen location only. | High | Current fallback risks remote truth leak. | Stale-location golden fails before fix and passes after. No remote moved target is revealed without evidence. |
| `crates/worldwake-ai/src/planning_snapshot.rs` | Add admission source metadata for each entity/place/field. Expose safe iterators such as `visible_entities_by_source()` and `entities_admitted_for(predicate)`. | High | Snapshot is the amplifier of belief leaks. | Tests assert source for local, belief, last-seen, evidence, topology, and hypothetical entries. |
| `crates/worldwake-ai/src/search/strategic.rs` | Rename/trace HTN as strategic guidance unless leaves are enforced. Add fallback policy and rejected-method trace. Avoid rebuilding method registry per call. | High | Current method semantics exceed enforcement. | Trace explains selected/rejected methods and fallback. |
| `crates/worldwake-ai/src/htn/selector.rs` | Implement or remove `AgentRole`; implement `LocationKnown` for witness/evidence/ledger or delete methods that depend on unsupported criteria. | High | Fake preconditions are worse than absent preconditions. | No method precondition may be unconditional unless explicitly declared `Always`. |
| `crates/worldwake-ai/src/htn/method_schema.rs` | Add `fallback_policy`, `enforcement_level`, and `leaf_policy`: `GuidanceOnly`, `PreferredLeaves`, `RequiredLeaves`. | Medium | Clarifies current vs future HTN authority. | Method trace includes enforcement level and fallback policy. |
| `crates/worldwake-ai/src/goal_schema.rs` | Remove `methods` or replace with explicit method-policy metadata. Do not keep empty method anchors. | High | Current field is a fossil seam. | Integration test changes from “methods empty” to “schema declares method authority policy.” |
| `crates/worldwake-ai/tests/integration/goal_schema_methods.rs` | Replace empty-method-anchor test with method-authority consistency tests. | High | The current test preserves the fossil. | Test proves exactly one method-assignment authority. |
| `crates/worldwake-ai/src/candidate_generation.rs` | Split candidate emission from anomaly/discrepancy/violation detection. Candidate generation may return observations-to-record, but those should be typed and owned by a separate anomaly pass. | Medium | Current side-effect-free mechanics still mix semantic ownership. | Candidate pass can be run without producing new memory writes except explicit typed observations. |
| `crates/worldwake-ai/src/ranking.rs` | Keep total-order authority, but introduce portfolio selection before planning. Move constants into profiles/schema or trace them as named policy constants. | Medium | Avoids score soup and top-K brittleness. | Trace says which portfolio slot admitted each planned candidate. |
| `crates/worldwake-ai/src/goal_model.rs` | Move terminal binding/payload/satisfaction declarations toward `GoalSchema`. Keep compatibility only during migration. | Medium | Current `GoalKindPlannerExt` is too semantically central. | Each goal has one schema-owned satisfaction/terminal contract. |
| `docs/planner-contracts.md` | Add planner-visible admission contract and method fallback contract. | High | Contracts should make current boundary explicit. | New tickets must cite source admission and fallback policy. |
| `docs/spec-drafting-rules.md` | Strengthen HTN method drafting checklist. | High | Prevents fake methods and method-required bypasses. | Any HTN spec must name fallback, leaves, belief sources, tests, traces. |
| `crates/worldwake-ai/src/decision_trace.rs` | Add method rejection/fallback traces, snapshot admission trace, stale-belief trace, player action visibility trace. | Medium | Needed for FND-29/31. | Golden trace assertions answer why/why-not questions. |
| `crates/worldwake-cli/src/*` | Ensure player UI uses the same belief/affordance surface as AI and cannot read raw world truth outside debug. | Medium | Future player POV depends on this. | Control-source swap test shows identical action set. |

### **Candidate replacement: `PerAgentBeliefView::effective_place`**

This is high-confidence as a direction, but it should be tested before being treated as final patch text.

fn effective_place(&self, entity: EntityId) -> Option<EntityId> {  
   if entity == self.agent {  
       return self.world.effective_place(entity);  
   }

   // Same-tick local physical observation is FND-14A-compliant.  
   if self.has_authoritative_local_visibility(entity) {  
       return self.world.effective_place(entity);  
   }

   // Direct possession is physically local to the actor's controlled body/inventory.  
   // This is not a general "known entity" fallback.  
   if self.world.possessor_of(entity) == Some(self.agent) {  
       return self.world.effective_place(entity);  
   }

   // Belief-store location is the normal non-local path.  
   if let Some(place) = self  
       .believed_entity(entity)  
       .and_then(|state| state.last_known_place)  
   {  
       return Some(place);  
   }

   // Last-seen memory is stale belief, not current truth.  
   self.world  
       .get_component_last_seen_memory(self.agent)  
       .and_then(|memory| memory.records.get(&entity).map(|record| record.place))  
}

Acceptance test:

Given:  
- Agent A last saw Target T at Place P1.  
- T moves to Place P2.  
- A receives no new observation, testimony, trace, record, or report.  
Then:  
- PerAgentBeliefView(A).effective_place(T) returns P1 or stale-belief equivalent, never P2.  
- PlanningSnapshot for A may include T at P1 only if belief/last-seen admission is legal.  
- Strategic planning may not target P2.

### **Exact replacement text candidate for `docs/planner-contracts.md`**

Add under “Entity admission and the belief barrier”:

### Planner-visible admission source

Every entity, place, and non-self field admitted into `PlanningSnapshot` must have an explicit planner-visible admission source.

Allowed admission sources are:

- `SelfAuthoritative`: actor-owned body/internal state.  
- `LocalSameTickPhysical`: directly perceivable co-located physical fact under FND-14A.  
- `BeliefStoreClaim`: explicit agent belief claim with provenance/confidence/freshness.  
- `LastSeenMemory`: remembered location or state, never current remote truth.  
- `TestimonyOrRecord`: information carried by testimony, notice, ledger, report, or other artifact.  
- `GroundedEvidenceCarrier`: an entity admitted because the current goal/candidate explicitly carries it as evidence.  
- `PublicTopology`: place graph/topology visibility, not remote entity visibility.  
- `HypotheticalPlannerEffect`: state produced only inside planner-local successor simulation.

Admission of an entity for one reason does not authorize unrelated authoritative fields on that entity.  
For example, admitting a remote violation record as evidence does not authorize reading the current  
location, inventory, owner, rights, or live occupants of unrelated remote entities.

Planner code that scans `snapshot.entities` must either restrict the scan to fields legal for that  
entity's admission source or use an iterator that enforces that restriction.

### **Exact replacement text candidate for `docs/spec-drafting-rules.md`**

Add to planner-formalism analysis:

For each HTN method, the spec must declare:

1. the reusable pursuit pattern it encodes;  
2. why plain GOAP is insufficient or less reliable for this pattern;  
3. whether the method is `GuidanceOnly`, `PreferredLeaves`, or `RequiredLeaves`;  
4. whether flat GOAP fallback is allowed, forbidden, or allowed only after traced method failure;  
5. every belief, record, testimony, local observation, or public-structure fact the method may read;  
6. every artifact, claim, queue position, reservation, or proof the method requires;  
7. every failure mode the method must attribute;  
8. golden tests proving method selection, method rejection, fallback behavior, and trace explanation.

A method-required goal is invalid unless the schema proves that fallback would satisfy the wrong  
semantic condition, not merely that fallback is less efficient.  
---

## **12. Golden Scenario and Evaluation Matrix**

| Scenario / Metric | Purpose | Systems exercised | Required assertions | Failure smell |
| ----- | ----- | ----- | ----- | ----- |
| Stale target location | Prove no remote truth leak | Belief view, snapshot, strategic planner | Agent plans from last-known place or asks/searches; never targets current unseen place | Omniscient pursuit |
| Local co-location physical observation | Preserve FND-14A exception | PerAgentBeliefView, affordances | Co-located item quantity/workstation/source visible same tick | Overcorrected belief gate blocks lawful perception |
| Unknown ownership beside chest | Social facts require belief | Belief view, affordance UI | Physical chest visible; owner/right facts unknown unless believed | Co-location reveals ownership |
| Bounty direct method vs flat fallback | Verify HTN guidance and fallback | HTN, strategic, tactical GOAP | Method selected when bounty/target known; fallback trace if method unusable | Silent fallback or hidden method authority |
| Investigation on-scene method | Detect dead method preconditions | HTN selector | `LocationKnown(ViolationEvidence)` either works or method rejected with trace | Method never selects despite evidence |
| Hunter role method | Verify role gating | HTN selector, agent profiles | Non-hunter cannot select group hunt if role required | `AgentRole(_) => true` persists |
| Produce with owned inputs | Ensure flat GOAP remains legal | HTN, GOAP | HTN may guide, but flat craft path still legal | Method over-constrains simple craft |
| Method-required future fixture | Prove burden of proof | Goal schema, HTN, tactical planner | Flat fallback forbidden only with explicit schema contract and trace | Shortcut bypasses required artifact |
| Goal ranking portfolio | Avoid score soup | Candidate gen, ranking, agenda | Candidate admitted by slot; trace shows slot and score | Weak opportunity drowns survival/duty |
| Candidate omitted due to ignorance | Explain why no candidate | Candidate gen, belief trace | Trace says missing belief/evidence, not just absent candidate | Source diving required |
| Player-control swap | Prove agent symmetry | ControlSource, UI, affordances, dispatch | Same available action set before/after AI ↔ human swap | UI sees more than AI |
| Debug truth separation | Prevent UI leak | CLI/UI/debug trace | Debug can show truth only in debug mode; normal UI cannot use it | Debug overlay affects actions |
| Queue contention | Preserve explicit contention | Affordance query, action lifecycle, scheduler | Queue/grant/full status explains action availability | Intent silently reserves facility |
| Reservation conflict | Verify explicit reservation | ActionDef, scheduler, planner | Conflict resolved by reservation/grant/tie rule | Tick order decides silently |
| Interruption by new danger | Stable but revisable intentions | Agenda, interruption, revalidation | Agent suspends/replans with trace; no hidden reservation | Rails or thrashing |
| Blocker churn | Prevent local repair loops | Failure handling, blocker memory, repair | Repeated failed repair escalates to full replan/suspension | Infinite repair retry |
| Discrepancy from violated expectation | Absence via expectation | Expectation, discrepancy memory, candidate gen | Missing item detected only if prior expectation existed | Omniscient missing-object detection |
| Stale belief correction | Belief revision | Perception, belief store, ranking | New evidence supersedes stale belief with provenance | Old belief silently overwritten without trace |
| Trace why method lost | HTN explainability | HTN selector, decision trace | Rejected methods list failed preconditions | Only winner traced |
| Trace why action legal | Player/AI affordance explainability | Affordance query, ActionDef | UI/action trace names preconditions, targets, contention status | “Action unavailable” with no reason |
| 100-agent soak | Early scale signal | Candidate gen, ranking, snapshots, traces | Candidate count, snapshot size, expansions, beam truncation, trace bytes recorded | Budget exhaustion hidden |
| 500-location topology sample | Snapshot/topology scaling | Snapshot, route costs | Travel horizon limits places; no all-world entity scan | Snapshot grows with world |
| Trace-volume cap | Prevent debug runaway | Decision trace aggregation | Trace bytes per tick bounded and sampled | Debugging changes sim cost unboundedly |

---

## **13. Research-Backed Design Rules For Future AI Work**

1. **A goal deserves HTN only when it has a reusable lawful pursuit pattern.** “Travel, inspect evidence, ask witness, adjudicate” can be HTN. “Make this story beat happen” cannot.  
2. **Flat GOAP is enough for short direct chains.** If ordinary affordance search can solve it within budget and failure attribution is generic, do not add a method.  
3. **Method-required is rare.** It is valid only when flat fallback would satisfy the wrong semantic condition, not merely when HTN is more efficient.  
4. **HTN leaves must be ordinary world affordances.** No method may create success outside `ActionDef`, preconditions, duration, cost, contention, and dispatch.  
5. **A motive becomes an intention only when the agent commits under explicit assumptions.** A high score is not an intention. A plan is not an intention. A world reservation is not created by intention.  
6. **Repair is allowed only while the intention’s semantic assumptions still hold.** If the target is gone, the belief is contradicted, the queue is lost, or the goal is stale, repair must yield to replan/suspend/abandon.  
7. **A blocker is agent-local memory unless it is a world artifact.** “I failed to buy bread at this shop” is memory. “This shop is legally closed” is world/institutional state. Do not collapse them.  
8. **A cache is legal only if deleting it and recomputing preserves world meaning.** Route danger cache is fine; route danger truth is not.  
9. **A derived score becomes dangerous when it hides its source facts.** Ranking must trace concrete needs, obligations, evidence, trust, and memories.  
10. **Candidate generation should not discover world truth by wanting things.** If candidate generation detects discrepancies, those observations need a named anomaly/evidence path.  
11. **Player-facing UI is an AI decision surface.** It must use the same belief and affordance gates as AI. Debug truth is a separate tool.  
12. **Every “why?” trace needs a “why not?” sibling.** Chosen goal, rejected goal, selected method, rejected method, visible action, hidden action, known fact, unknown fact.  
13. **Do not call strategic-stage guidance “full HTN.”** Use honest names. If leaves are not enforced, say so.  
14. **Do not add another live authority field for an existing concept.** If registry owns methods, schema must not look like it owns methods unless the ownership is intentionally changed.  
15. **Belief locality must be type-protected eventually.** Review and tests are not enough for a system that wants hundreds of agents and player POV switching.

---

## **14. Open Questions and Uncertainties**

1. **How much of `IntentionFrame` is currently live versus adjacent to `AgendaState`?** I inspected agenda types but not the full intention-frame lifecycle. The second audit should trace adopt/continue/suspend/abandon end to end.  
2. **Is `can_control()` actually used for player-visible affordances, planner-visible affordances, dispatch validation, or all three?** If all three, it needs splitting. If dispatch only, the risk is lower.  
3. **Are candidate-generation pending records already applied in a clean write phase with strong trace separation?** The comments say yes mechanically; the semantic ownership still smells mixed.  
4. **How many HTN methods are actually selected in golden tests?** The selector suggests some may be dead. This needs coverage, not speculation.  
5. **Does `LastSeenMemory` currently also produce `BelievedEntityState` before `effective_place()` is called?** `known_entity_beliefs()` synthesizes beliefs from last-seen records, but `effective_place()`’s direct fallback still looks dangerous. The exact runtime behavior should be verified with a focused failing test.  
6. **How much of the player UI already uses `get_affordances`?** I did not fully inspect CLI/UI.  
7. **Are aggregate diagnostics already present under `scenario_diagnostics.rs` or reports not fully inspected?** I found rich local traces, but not enough evidence of a scenario-level dashboard.  
8. **Does plan repair duplicate full planning or only perform local lawful substitution?** Needs a dedicated second-pass audit.  
9. **Are action guard/expectation templates consistently populated for all important actions?** `ActionDef` supports them; coverage may lag.  
10. **Does current route danger use only belief-backed threat state in every path?** Planner contracts say yes; I did not exhaustively inspect every route-threat call.

---

## **15. Next-Iteration Prompt Suggestions**

For the second iteration, focus on one of these, in order:

1. **Belief-boundary proof audit.** Trace every `RuntimeBeliefView` method used by planning and UI. Build a table of self-authoritative, local physical, belief-backed, public topology, and dispatch-only reads. Add failing tests for stale-location, ownership, office-holder, route-danger, and bounty-record knowledge.  
2. **HTN enforcement audit.** For each method, run or inspect tests proving selection, rejection, fallback, and resulting tactical action sequence. Decide whether the layer is renamed to strategic method guidance or upgraded toward enforced leaves.  
3. **Intention/repair audit.** Trace committed agenda entries, active plans, plan repair, blockers, discrepancies, and revalidation from adoption through failure. Identify whether intentions are stable commitments or accumulated patches.  
4. **Player-POV action surface audit.** Verify that the human-controlled character sees exactly the same lawful action surface an AI-controlled version would see, with debug truth fully separated.  
5. **Candidate/ranking scalability audit.** Measure candidate fan-out, snapshot size, search expansions, beam pruning, trace volume, and blocker churn across synthetic 50/100/250-agent scenarios.

