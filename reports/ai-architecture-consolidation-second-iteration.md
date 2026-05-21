# **Second-Iteration Hostile AI Architecture Audit — Worldwake**

Mission source followed: uploaded second-iteration audit prompt.

## **1. Executive Verdict**

**Verdict: Worldwake is cleaner than the first iteration, but it is still not safe to add broad new AI behavior. The architecture now has real consolidation surfaces — `GoalSchema`, portfolio triage, agenda state, `MethodRegistry`, `AdmissionSource`, richer traces, belief-wall golden tests — but the most important boundary is still porous in the live `PerAgentBeliefView`.**

The main problem is not an AI planner directly grabbing `World` in `worldwake-ai`. I did **not** find that as the dominant pattern. The main problem is worse because it is quieter: **the official belief-facing view still exposes some current authoritative world facts after a weak “known/accessed” gate.** That means both AI and player-facing action surfaces can inherit the same leak, because the CLI correctly uses `PerAgentBeliefView` plus `get_affordances()` for the human action menu.

The architecture should **not** be thrown away. The tactical GOAP/action-definition core is worth keeping. The HTN layer is also worth keeping, but only under a narrower contract: it is currently **method-guided strategic planning**, not a fully enforced ordered task-network executor. `MethodSchema` names and subgoals imply more authority than the live code enforces.

Severity ratings:

| Severity | Finding |
| ----- | ----- |
| **Critical** | Belief-view-mediated current-truth leaks: remote sale listings/seller stock, current container/possessor, social/control rights, temporal queues/reservations, production jobs, and some load/capacity reads. |
| **Critical** | Player-POV symmetry inherits the same leaks because the normal CLI action menu uses the same `PerAgentBeliefView`/`get_affordances()` route. The UI surface is architecturally aligned, but only as safe as the belief view. |
| **High** | Planning snapshot admission is a good design, but entity-level `AdmissionSource` is not strong enough when field reads can still fall back to current world state. |
| **High** | HTN is useful but overnamed. Current methods select strategic stages and trace provenance; they do not enforce every declared leaf/subgoal as an ordered task list. |
| **High** | Candidate generation is partly schema-driven but still has a live `LEGACY_EXTRACTOR_ORDER` seam and an out-of-band blocked-self-care fallback. |
| **Medium** | Ranking/portfolio is much better consolidated, but still risks score soup if scores are allowed to stand in for concrete state. |
| **Medium** | Repair/replan is real and useful, but it is becoming a second planning policy surface unless its authority is narrowed. |
| **Medium** | Traces are rich, but some of them still describe planner activity rather than proving lawful causal/belief paths. HTN subgoals can remain `Pending`; snapshot admission is per-entity, not per-field. |

**Recommendation: Option B — Moderate Consolidation.** Preserve the GOAP/action-definition core, preserve HTN as lawful method-guided strategic planning, preserve agenda/portfolio, but harden the belief boundary before adding behavior. The next implementation cycle should be mostly boundary repair, trace proof, and golden scenario coverage.

---

## **2. Repository Discovery Log**

### **Active docs/contracts inspected**

| File | Role in audit |
| ----- | ----- |
| `docs/FOUNDATIONS.md` | Treated as the non-negotiable design constitution. It explicitly requires local causality, belief/world separation, first-class ignorance/stale/false belief, agent symmetry, action legality, no fossil layers, causal history, and validation. |
| `docs/planner-contracts.md` | Current planner authority contract. It states the planner must not read authoritative runtime state directly, remote entity visibility must be belief/evidence-backed, same-tick co-location is physical-only, and HTN fallback is legal unless a method-required contract proves otherwise. |
| `docs/spec-drafting-rules.md` | Confirms future specs must state information paths, HTN fallback policy, stored-vs-derived state, and scenario-definable profile surfaces. |
| `AGENTS.md` | Confirms no `Player` type, `ControlSource` only, belief-only planning, append-only event log, no compatibility fossil layers, and full AI-pipeline validation after control/validation changes. |
| `.claude/skills/goap-architecture-report/SKILL.md` | Used only as a process/checklist artifact, not architecture authority. |

### **Active reports used**

| File | Use |
| ----- | ----- |
| `archive/reports/goap-architecture-report-2026-05-21-exploited.md` | Used for pipeline overview, budgets, snapshot costs, trace gaps, and prior diagnostic concerns; cross-checked against current code. |
| `archive/reports/ai-architecture-improvements.md` | Used as current prior analysis for BDI shell, schema-driven goals, HTN as lawful search control, portfolio triage, and stronger diagnostics. |
| `archive/reports/ai-architecture-consolidation-first-iteration.md` | Used only as first-iteration context to identify what changed and what still persists. It already called out remote truth leakage, overnamed HTN, scattered goal semantics, and fossil seams. |

### **Active code files inspected**

| Area | Files inspected |
| ----- | ----- |
| Core goal identity | `crates/worldwake-core/src/goal.rs` |
| Goal schema/model | `crates/worldwake-ai/src/goal_model.rs`, `goal_schema.rs` |
| Candidate generation | `crates/worldwake-ai/src/candidate_generation.rs` |
| Agent schema context | `crates/worldwake-core/src/agent_schema_context_profile.rs` |
| Ranking/portfolio | `crates/worldwake-ai/src/ranking.rs`, `agent_tick/portfolio.rs`, `agent_tick/planning.rs` |
| Agenda/intention/repair | `agenda_manager.rs`, `agent_tick/frame.rs`, `plan_revalidation.rs`, `failure_handling.rs`, `agent_tick/planning.rs` |
| HTN | `htn/method_schema.rs`, `methods.rs`, `registry.rs`, `selector.rs`, `search/strategic.rs` |
| Planning snapshot | `planning_snapshot.rs` |
| Belief and affordance surfaces | `belief_view.rs`, `per_agent_belief_view.rs`, `affordance_query.rs`, `action_handler.rs` |
| Player/CLI control | `crates/worldwake-cli/src/handlers/actions.rs`, `handlers/control.rs` |
| Diagnostics/tests | `decision_trace.rs`, `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` |

### **Archived files read**

**None.** Search results surfaced archive paths, but I did not open or rely on archived material. The audit stayed on active non-archive code/docs/reports.

### **Important areas not inspected sufficiently**

I did not exhaustively inspect every action handler in `crates/worldwake-systems/src/`, every visualizer/debugger surface, or every scenario test. The conclusions about the belief-view leaks are still strong because the risky reads are in the central `PerAgentBeliefView`, and both planner and player-facing affordance surfaces depend on that view.

---

## **3. Research Synthesis**

### **GOAP in games**

The F.E.A.R.-style GOAP lesson is still useful: planning over action preconditions/effects lets NPCs discover action sequences at runtime instead of hard-coding behavior transitions, but it also depends on local knowledge/sensors and fast replanning rather than omniscient truth.

**Implication for Worldwake:** keep GOAP as the lawful action-sequence engine. Worldwake’s action definitions, preconditions, reservations, target specs, payloads, and dispatch checks are the right substrate.

**What it does not imply:** GOAP should not own motive discovery, belief correction, intention persistence, institutional law, or task allocation. Those need separate stateful architecture.

### **HTN / SHOP-style planning**

SHOP2 is a canonical HTN planner; the cited paper describes HTN planning’s strength as domain method decomposition, including temporal and metric planning features.

**Implication for Worldwake:** HTN is justified only where methods encode reusable lawful pursuit patterns that genuinely constrain search. Production/restock/investigation/escort patterns plausibly qualify.

**What it does not imply:** a method schema with subgoals is not automatically enforced HTN. If subgoals are converted into strategic stages and GOAP still chooses actual leaves, the live layer is method-guided strategic search, not full HTN execution. That is acceptable, but it must be named and traced honestly.

### **BDI and intention management**

Rao and Georgeff frame deliberation as leading to intentions: plans of action an agent is committed to achieving.

**Implication for Worldwake:** the agenda/intention architecture is the right shape. `AgendaState` committed/pending/suspended entries, continuation margins, partial plans, revalidation, and blockers are Worldwake’s BDI-like shell.

**What it does not imply:** do not import BDI terminology as decorative naming. The shell should own commitment lifecycle only; it should not duplicate GOAP, HTN, or action legality.

### **Utility / motive scoring**

Utility AI is a common game-AI behavior-selection technique that scores possible actions/behaviors from current inputs.

**Implication for Worldwake:** utility-like ranking is useful for bounded planning triage, especially with hundreds of agents. Portfolio slots are a good direction.

**What it does not imply:** a score must never replace concrete state. FND-3 is clear: concrete state beats abstract scores. Worldwake should use motive scores to allocate planning budget, not to decide whether a seller exists, whether a contract is valid, or whether an agent “knows” something.

### **Multi-agent coordination**

Contract Net is a classic task-allocation protocol where tasks are proposed and contractors bid/accept/reject; it is relevant because it makes coordination messages explicit rather than magical. Multi-agent pathfinding research treats movement as conflict-free space-time coordination, and practical approaches acknowledge scalability and partial/decentralized information limits.

**Implication for Worldwake:** coordination should be world artifacts: declarations, queues, grants, contracts, reservations, claims, visible assignments, notices. The current queue/reservation/grant surfaces are conceptually right.

**What it does not imply:** do not add a privileged global coordinator. Any coordination surface must be observable, contestable, and persisted as world/belief state.

### **Explainable planning**

Explainable planning research emphasizes communicating the foundations of planner behavior, not just dumping a chosen plan.

**Implication for Worldwake:** traces must answer contrastive questions: why this goal, why not that candidate, why fallback, why method rejected, why stale belief, why unknown, why action invisible, why contention resolved this way.

**What it does not imply:** a pretty trace is not proof. A trace that lacks source/admission/belief provenance can make omniscience look causal.

### **Epistemic planning and partial observability**

Epistemic planning is explicitly about planning with distributed knowledge and capabilities; DEL-based work starts from classical planning and adds knowledge/perspective because ordinary STRIPS-like state is insufficient for multi-agent knowledge. Cooperative epistemic planning extends this toward decentralized coordination under distributed knowledge.

**Implication for Worldwake:** FND-14 and FND-14A are not optional polish. False belief, stale belief, ignorance, testimony, records, and social carriers are the core of the future game.

**What it does not imply:** Worldwake does not need a formal epistemic planner now. It needs source-typed belief boundaries and scenario tests that prove the simpler architecture preserves ignorance.

---

## **4. Current Architecture Map**

Current live decision pipeline:

World events / local perception / testimony / records  
 -> AgentBeliefStore / memories / social observations  
 -> PerAgentBeliefView / RuntimeBeliefView / GoalBeliefView  
 -> candidate extractors + opportunity compiler  
 -> ranking + suppression + damping + portfolio slots  
 -> agenda/intention continuation/suspension/adoption  
 -> PlanningSnapshot with AdmissionSource  
 -> HTN method selection as strategic guidance  
 -> strategic search stages  
 -> tactical GOAP search over ActionDef affordances  
 -> revalidation / guards / repair / failure classification  
 -> scheduler / action lifecycle / authoritative dispatch  
 -> event log / traces / belief updates

| Layer | Current files/types | Reads | Writes | Authority owned | Risk |
| ----- | ----- | ----- | ----- | ----- | ----- |
| Perception and belief acquisition | Belief stores, local observations, testimony/records via systems; `AgentBeliefStore` | World events, local observations, testimony, records | Beliefs, memories, records | Agent-local knowledge | Not fully audited; must remain the only path for remote/social knowledge. |
| Belief view wall | `belief_view.rs`, `per_agent_belief_view.rs`, `RuntimeBeliefView`, `GoalBeliefView` | `World`, scheduler runtime, belief stores | Read-only surface | AI/player-facing knowledge boundary | **Critical leak surface**: several methods read current world facts after weak gates. |
| FND-14A local physical exception | `PerAgentBeliefView` local observation helpers | Same-tick co-located physical state | Observed physical values | Local physical perception | Good concept, but social/institutional reads still need hard separation. |
| Goal schema | `goal_schema.rs`, `GoalDispatchKey`, `GoalSchema` | Static declarations | Static policy tables | Goal dispatch metadata | Useful, but `relevant_ops_authority()` is explicitly hint-only. |
| Candidate generation | `candidate_generation.rs`, extractors, `GoalOffer` | Belief view, memories, recipes, opportunities | Candidates plus diagnostics/pending records | Desire/opportunity discovery | Half-consolidated: schema extractor declarations still flow through `LEGACY_EXTRACTOR_ORDER`. |
| Opportunity compilation | `opportunity_compiler`, `PerceivedOpportunityIndex` | Learned opportunity memories | Candidate opportunities | Derived opportunity recall | Risks duplicating candidate generation. |
| Ranking | `ranking.rs`, `GoalPolicy`, motive sources | Candidates, needs, memories, trust, competition | Ordered agenda entries, suppressed/damped diagnostics | Sole total-order preference | Better centralized; still score-heavy. |
| Portfolio triage | `agent_tick/portfolio.rs` | Ranked entries, feasibility probe, operating mode | Portfolio slots, search cap influence | Planning-budget triage | Good for scaling; fixed slot taxonomy can become brittle. |
| Agenda/intention | `agenda_manager.rs`, `IntentionFrame`, `AgendaState` | ranked goals, current plan, failures | committed/pending/suspended entries | Commitment lifecycle | Stronger BDI shell; still overlaps with repair and plan state. |
| Planning snapshot | `planning_snapshot.rs` | Belief view, evidence sets, travel horizon | Snapshot maps, costs, admissions | Planner-local derived state | Good admission model, but per-entity source is not enough if fields leak. |
| HTN selection | `htn/selector.rs`, `MethodRegistry` | Goal offer, belief view, profile disabled methods, recipes | selected/rejected method trace | Method choice | Narrowly useful; not full action authority. |
| HTN decomposition | `search/strategic.rs` | selected method subgoals | Strategic stages | Search guidance | Subgoals are mostly stage hints; leaves not strictly enforced. |
| Strategic search | `search/strategic.rs` | snapshot state, stages, route costs | strategic plan/stages | High-level itinerary | Amplifies snapshot mistakes. |
| Tactical GOAP | search modules, planner ops | `PlanningState`, action defs, semantics | `PlannedPlan` | Action-sequence planning | Strong core; must remain under action legality. |
| Affordance enumeration | `affordance_query.rs` | `RuntimeBeliefView`, action defs/handlers | `Affordance`s | AI/player visible actions | Correct authority shape; leaks if belief view leaks. |
| Revalidation/guards | `plan_revalidation.rs` | current belief view, affordance query, guards | valid/invalid outcome | “Can continue?” | Good use of affordances; exact-target synthetic validation needs tests. |
| Failure handling | `failure_handling.rs` | failed step, belief view, execution failure | blockers, discrepancies, dirty replan | Failure attribution | Useful, but some classifiers read sale/job/queue state through same risky view. |
| Action dispatch | `action_handler.rs`, scheduler | authoritative world state | action instances/world mutations | Commit authority | Correct place for truth. |
| Human control | `cli/handlers/actions.rs`, `control.rs`, `ControlSource` | controlled entity, belief affordances | input queue / control source | chooser only | Good symmetry path; switch command uses world truth for meta-selection, which is acceptable only as debug/meta UI. |
| Decision traces | `decision_trace.rs` | candidates/plans/snapshots/methods | per-agent traces | Debug read-model | Rich but not sufficient as proof of causal/belief legality. |

---

## **5. Delta Since First Iteration**

I did not assume merged work. I inferred current shape from `main`.

| Change observed | Intended fix | Does it fix it? | New risk | Recommendation |
| ----- | ----- | ----- | ----- | ----- |
| `PerAgentBeliefView::effective_place` appears corrected for remote pursuit; golden test proves stale last-seen target is used instead of live remote place. | Fix first-iteration remote location leak | Partially yes | Other view methods still leak non-location facts | Preserve, then extend same discipline to every remote/social/economic/temporal field. |
| `AdmissionSource` exists in `PlanningSnapshot`. | Explain why entities enter snapshots | Partially yes | Per-entity source is weaker than per-field source | Preserve and extend to per-field provenance. |
| `GoalSchema` centralizes dispatch metadata, budgets, candidate extractors. | Reduce scattered goal semantics | Partially yes | `relevant_ops_authority()` is hint-only; extractor execution still uses legacy global order | Preserve but make schema the actual extractor wiring authority. |
| Candidate extractors are typed by `CandidateExtractorId`. | Reduce emitter sprawl | Partially yes | `LEGACY_EXTRACTOR_ORDER` still owns ordering; blocked-self-care fallback bypasses extractor chain | Remove fossil seam. |
| Portfolio slots added. | Prevent top-K score collapse | Mostly yes | Fixed slots can become another hard-coded taxonomy | Preserve; make slot assignment declarative and trace decisive slot reasons. |
| Agenda state expanded. | Stable intentions without rails | Better | Agenda can become a second candidate-generation source through companion goals | Preserve but formalize agenda authority. |
| HTN `MethodRegistry` owns method assignment. | Remove schema-vs-registry split | Mostly yes | Method schemas still overpromise enforcement | Preserve, narrow contract. |
| Method traces added. | Explain selected/rejected/fallback methods | Partially | Subgoals can remain `Pending`, so trace can be decorative | Add enforced/stage-hint distinction. |
| Golden belief-wall tests exist. | Prove FND-14A for theft, remote pursuit, control swap | Yes for covered cases | Coverage is too narrow: economic/social/queue/containment leaks remain | Extend with adversarial leak suite. |
| CLI human action menu uses belief affordances. | Player/AI symmetry | Architecturally yes | Inherits belief-view leaks | Preserve; add UI leak tests. |

---

## **6. Hidden Authority Leak Inventory**

This is the most important section.

| Leak | Symptom | Evidence | Type | Why it matters | FOUNDATIONS implicated | Failure mode | Severity | Confidence | Test that proves/falsifies |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| Remote sale listing / seller stock truth | `listed_sale_lots_at`, `has_sale_listing`, `seller_for_sale_lot` can expose current sale state after belief-based entity admission. | `PerAgentBeliefView` economic methods read current sale listing/seller/stock-like facts; planning snapshot copies economic fields. | Indirect, belief-view-mediated, snapshot-mediated | Seller stock is explicitly a prompt concern and a classic remote-truth leak. | FND-7, FND-14, FND-14A, FND-16, FND-27 | Agent “knows” a remote seller delisted/restocked without observation/testimony/record. | **Critical** | High | Agent has stale belief that remote market sells grain. Seller delists unseen. Candidate generation/planning must still use stale belief or unknown, not current delist. |
| Current container/possessor leak | `direct_container`/`direct_possessor` return current world relation for `knows_entity`, which can include stale belief. | Inventory section reads current container/possessor under broad knowledge gate. | Indirect, belief-view-mediated | Current possession/custody is social/physical state that must travel by observation or testimony. | FND-4, FND-7, FND-14, FND-16 | Item moved from chest to thief remotely; agent instantly targets thief/container. | **Critical** | High | Agent last saw item in chest. Move item unseen. Planner/UI must not reveal new possessor/container. |
| Social/control rights leak | `believed_rights`/`can_control` gate accessibility, then consult current `world.effective_rights` / `world.can_exercise_control`. | Control belief view does this after access checks. | Indirect social authority leak | FND-14A says social/ownership/rights/jurisdiction always need belief entries, even co-located. | FND-14A, FND-18, FND-19 | Player/AI sees a control/steal/claim affordance appear/disappear because rights changed remotely. | **Critical** | Medium-high | Transfer ownership/rights without informing actor. Affordance set must not change until belief/record/testimony is acquired. |
| Temporal queue/reservation leak | queue/grant/reservation methods can expose live contention state without proving local observation. | Temporal methods in `PerAgentBeliefView` expose reservation ranges, queue positions, grants; affordance/failure logic uses contention surfaces. | Indirect, action-surface-mediated | Queue/grant status is world state; remote knowledge should require local observation, record, or testimony. | FND-8, FND-9, FND-14, FND-19 | Agent avoids/chooses remote facility because queue changed unseen. | **High** | Medium-high | Remote workstation reservation changes unseen. Planner must not know unless actor is present, has a record, or receives testimony. |
| Remote production job leak | `has_production_job` returns current job state; precondition `TargetLacksProductionJob` and failure classification use it. | `has_production_job` exposed through view; affordance query uses `TargetLacksProductionJob`; failure handling checks it. | Indirect, precondition-mediated | Production activity should be observed or recorded, not globally sensed. | FND-7, FND-14, FND-16 | Agent knows a remote workstation became busy/free. | **High** | High | Start/finish remote production job unseen. Candidate/planner must remain stale/unknown. |
| Load/carry capacity leak | `carry_capacity` and `load_of_entity` can return current values for remote known entities. | Inventory view exposes load/capacity reads. | Indirect | Less severe than location/rights, but still remote physical current-state leakage. | FND-7, FND-14, FND-16 | Agent infers remote cargo changes or encumbrance without observation. | Medium | Medium | Remote target’s load changes unseen; planner must not adjust route/trade/escort assumptions. |
| Evidence entity full-field amplification | `GroundedEvidence`/evidence entities enter snapshot, then many fields are built through view reads; if view leaks, snapshot freezes leak into planning state. | Snapshot entity build uses belief-backed values for some fields but still queries view for many other fields; evidence entities can bypass belief-backed field path. | Snapshot-mediated | Evidence that entity exists/was mentioned is not evidence of every current field. | FND-14, FND-18, FND-27, FND-29 | Rumor about seller imports current stock, current queue, current possessor. | **High** | Medium-high | Candidate evidence includes remote entity from testimony. Snapshot must include only testified/believed fields, not all current fields. |
| Debug/trace-to-decision leak | No direct leak found, but traces contain snapshot admissions and rich planner-local data. Need prove traces never feed normal decision/UI. | Decision trace model is rich and intended diagnostic; no evidence of it driving decisions in inspected code. | Suspected trace-mediated | Trace data can become a backdoor UI if reused by normal player interface. | FND-19, FND-29, FND-29A | Player opens “normal” view that includes omniscient trace facts. | Medium | Low-medium | Normal player UI must not read `AgentDecisionTrace` except as debug/replay mode with explicit separation. |
| Player action surface inherits leaks | CLI uses `PerAgentBeliefView` and `get_affordances()` for action listing; that is architecturally right but shares leaks. | `handle_actions` builds `PerAgentBeliefView` and calls `get_affordances`; action request is strict. | UI-mediated | Future player POV requires no extra omniscience. | FND-19 | Player sees remote/social truth via action menu. | **Critical** | High for inheritance, medium for exact downstream cases | For each leak above, run identical AI-vs-Human control-source action-menu fingerprint. |

---

## **7. Hostile Failure Inventory**

| Smell | Evidence | Why it matters | FOUNDATIONS implicated | Downstream failure | Severity | Status |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| `LEGACY_EXTRACTOR_ORDER` still controls extractor execution | Candidate generation builds ordered extractors from schemas, then sorts by legacy order. | Schema consolidation is incomplete. | FND-28 | New schema declaration does not actually own generation order/authority. | High | Real |
| Blocked self-care fallback bypasses extractor chain | `emit_exploration_candidates_for_blocked_self_care` runs after extractor loop. | Hidden candidate source outside schema. | FND-20, FND-28 | Future behavior added to fallback path without diagnostics/schema. | High | Real |
| Candidate generation mixes emission with anomaly/source-failure/pending memory detection | Result includes candidates plus pending violations/discrepancies/source reliability failures/acquisition resets. | Read phase is side-effect-free mechanically, but responsibility is too broad. | FND-18, FND-26, FND-29 | Candidate pass becomes hidden perception/interpreter. | Medium | Real |
| HTN group hunt is fake or premature | `fulfill_bounty_group_hunt` uses support declaration/social signal but no real recruit/coordination leaf. | Method name promises group hunt that live actions do not enforce. | FND-20, FND-31 | Agents appear to coordinate but only attack after a symbolic declaration. | High | Real |
| HTN subgoals overpromise | `SubgoalTemplate` includes `PerformAction`, `ResolveCoordination`, etc.; strategic planner maps them to stages rather than strict execution. | Method schemas richer than enforcement. | FND-20, FND-29 | Trace says method selected but actual plan violates intended sequence. | High | Real |
| `GoalSchema.relevant_ops_authority()` says hint-only | Tests assert hint-only authority. | The name “authority” is misleading. | FND-28 | Engineers treat relevant ops as legality. | Medium | Real |
| `GoalKey` normalizes acquisition quantity away | `GoalKey::from` drops `AcquireCommodity.quantity` identity. | May be intended, but concrete quantity matters for plans. | FND-3 | Same goal identity collides across different concrete quantities. | Medium | Real but maybe intentional |
| Ranking is centralized but dense | Ranking combines motive, provenance, reliability, competition, feasibility, source composite, etc. | Centralization is good; explanatory burden rises. | FND-3, FND-29 | Score soup masks concrete reasons. | Medium | Real |
| Portfolio search order follows global ranking, not slot category | Planning code says portfolio is trace/budget protection; search order remains ranking order. | Portfolio is less authoritative than it appears. | FND-20 | Emergency slot may not control planning as expected unless weights do it. | Medium | Real |
| `ActionDefId(u32::MAX)` placeholder in escort payload | Goal payload override uses placeholder action ID for intended heal action. | Sentinel values are fossil seeds. | FND-28 | A later layer trusts placeholder as real ID. | Medium | Real |
| Repair can duplicate planning semantics | Planning has accepted repair classification, route signatures, counterparty rebinding, pending repair resume. | Local repair must be narrower than full replanning. | FND-21, FND-29 | Repair hides root failure or silently changes goal semantics. | Medium | Suspected/real |
| Snapshot uses all-pairs shortest paths | Reports note Floyd-Warshall-style cost computation and per-tick snapshots. | Hundreds of locations/agents will hurt. | FND-20 | Planning cost explodes before behavior becomes interesting. | Medium | Real |
| Traces can be decorative | HTN trace has `SubgoalAttemptOutcome::{Pending,Succeeded,Failed}` but selected-stage construction marks subgoals pending. | It explains intended method, not whether method actually constrained behavior. | FND-29, FND-31 | False confidence in causality. | Medium | Real |

---

## **8. GOAP / HTN / BDI / Utility Responsibility Matrix**

| Responsibility | Should be owned by | Current owner | Problem? | Recommendation |
| ----- | ----- | ----- | ----- | ----- |
| Motive discovery | Candidate extractors + memories + profiles | Candidate generation, opportunity compiler, agenda companions | Yes, spread | Make extractor declarations schema-owned; split observation interpretation from motive emission. |
| Candidate generation | Goal schema + extractor registry | `candidate_generation.rs` with legacy order | Yes | Remove `LEGACY_EXTRACTOR_ORDER`; no out-of-band emitters. |
| Goal ranking | Ranking module | `ranking.rs` | Mostly no | Preserve sole total-order authority; require concrete source contributions. |
| Portfolio/slot triage | Portfolio module | `agent_tick/portfolio.rs` plus planning cap | Partly | Preserve, but make slot role explicit: budget triage, not hidden ranking override. |
| Intention persistence | Agenda/IntentionFrame | `AgendaState`, `IntentionFrame`, planning continuation | Mostly no | Preserve; document lifecycle contract. |
| Method selection | HTN registry/selector | `MethodRegistry`, `selector.rs` | Mostly no | Preserve; selector must remain belief-view-only. |
| Method decomposition | HTN strategic guidance | `search/strategic.rs` | Yes, naming | Rename/contract as stage guidance unless leaves enforced. |
| Action-sequence planning | Tactical GOAP | search modules/action defs | No | Preserve. |
| Fallback planning | Strategic search policy | `search/strategic.rs` | Mostly no | Keep fallback legal by default; trace why legal. |
| Failure attribution | Failure handler + discrepancy/blocker memory | `failure_handling.rs` | Partly | Preserve, but classify from belief/state sources; avoid hidden truth reads. |
| Repair | Narrow local repair layer | `plan_repair`, `agent_tick/planning.rs` | Suspected | Allow only bounded rebinding/resume; full planning remains planner-owned. |
| Contention handling | Action/scheduler/world artifacts | action defs, queues, reservations, failure classification | Mostly no | Preserve; ensure remote knowledge is belief-gated. |
| Belief correction | Perception/testimony/records | Candidate gen pending records + systems | Yes | Move anomaly detection out of candidate emission into observation interpretation. |
| Player action visibility | Same affordance path as AI | CLI actions use `PerAgentBeliefView` + `get_affordances()` | Shape good, data risky | Preserve route; fix belief view. |
| Trace explanation | Decision trace + event log + belief provenance | `decision_trace.rs`, event log | Partly | Add contrastive missing-knowledge and per-field admission traces. |

---

## **9. HTN Verdict**

**HTN should remain, but only as a narrow method-guided strategic search layer until the code enforces method leaves.** Do not expand HTN features. Do not mark current methods method-required. No current method family has enough live enforcement proof to justify forbidding flat GOAP fallback.

Current registry has 11 methods across `FulfillBounty`, `ProduceCommodity`, `RestockCommodity`, `InvestigateViolation`, and `EscortToSafety`.

| Method family | Classification | Live problem solved | Preconditions real? | Subgoals enforced? | Leaves ordinary affordances? | Fallback legal? | Missing tests |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| `fulfill_bounty_direct` | HTN justified but boundary needs tightening | Structures arm/travel/observe/attack/claim pattern | Mostly, but needs belief-source proof | Advisory/stage-guided | Intended yes | Yes | Stale target, missing weapon, bounty issuer unavailable, claim impossible. |
| `fulfill_bounty_investigation` | HTN optional; fallback legal | Gives lawful search/inquiry path when target location unknown | Partly | Advisory | Ask/inspect leaves are ordinary if action-backed | Yes | Method rejection when no witness/record; trace fallback reason. |
| `fulfill_bounty_group_hunt` | Fake HTN / misleading method surface | Intended coordination | Weak | Not enforced | No real recruit/coordination leaf | Yes | Should be removed/renamed until group coordination artifacts exist. |
| `ProduceCommodity` methods | HTN justified but boundary needs tightening | Acquisition/craft/return staging | Mostly | Stage-guided | Craft/harvest/trade ordinary | Yes | Competing workstation, stale resource, remote job leak, recipe mismatch. |
| `RestockCommodity` methods | HTN optional; flat fallback legal | Restock via purchase/production | Mostly | Stage-guided | Ordinary actions | Yes | Seller delists unseen, stock storage unavailable, market staffing mismatch. |
| `InvestigateViolation` by witness/ledger | HTN justified but boundary needs tightening | Witness/record-first investigation path | Needs stronger social/record source proof | Advisory | Ask/consult/investigate ordinary | Yes | Unknown witness, stale ledger, violation expired without belief update. |
| `EscortToSafety` | HTN optional; boundary needs tightening | Travel/escort/home pattern | Partly | Advisory | Ordinary if escort action-backed | Yes | Destination unknown, route blocked, ward incapacitated, placeholder payload misuse. |

### **Specific HTN failures**

`MethodSchema` declares `SubgoalTemplate::PerformAction`, `ResolveCoordination`, `ReturnTo`, motive bias, budget hints, and failure modes. That reads like HTN authority. But `search/strategic.rs` converts selected methods into strategic stages and falls back to flat stage building if no method or no stages are produced.

The method trace can report selected/rejected methods, fallback reasons, and subgoal attempts, but subgoal outcomes are not proof of enforced leaves.

### **HTN contract I recommend**

Current `MethodSchema` should gain an explicit enforcement mode:

pub enum MethodSubgoalAuthority {  
   /// Subgoal contributes strategic destinations, prerequisite commodities, or trace context.  
   StageHint,

   /// Subgoal must correspond to at least one ordinary ActionDef-backed planned step,  
   /// and the trace must prove selected/rejected/failure status.  
   RequiredActionLeaf,  
}

But do **not** rush to implement full required leaves. First rename/contract the current layer honestly:

Current HTN authority:  
- may select a method from belief-visible preconditions;  
- may bias strategic stages and budget;  
- may explain why a method was selected/rejected;  
- may not bypass ActionDef affordances;  
- may not declare flat GOAP semantically invalid unless a method-required contract and golden test exist;  
- may not claim subgoals were enforced unless the selected plan proves corresponding ordinary action leaves.

### **Method-required verdict**

**No current HTN-covered goal should be method-required yet.** The burden of proof is not met.

A future method-required declaration must prove:

1. Flat fallback would be semantically illegal, not merely less efficient.  
2. The method has required leaves mapped to ordinary `ActionDef` affordances.  
3. The plan trace proves each required leaf was satisfied, skipped lawfully, or failed.  
4. A golden scenario fails if flat fallback bypasses the method.  
5. Failure produces belief/memory/world state, not silent retry.

---

## **10. Belief Boundary and Player-POV Audit**

### **Direct world-read risks**

The strongest design win is that `worldwake-ai` mostly talks through `RuntimeBeliefView`, planning snapshots, and affordances rather than directly reading `World`. That is the right shape.

The strongest design failure is that `RuntimeBeliefView` implementation itself still contains current-world reads that are not safe for remote/social facts. `PerAgentBeliefView` has the correct conceptual split — local physical observations, belief reads, social/political/economic views — but the implementation sometimes equates “known entity” with “allowed to read current state.”

### **Remote truth admission risks**

Snapshot collection is better than before: `AdmissionSource` distinguishes self, local same-tick physical, grounded evidence, belief last-seen, possession containment frontier, and public topology.

But the snapshot still builds many fields by calling the belief view. If the belief view returns current truth for sale listings, container/possessor, production jobs, queues, rights, or load, the snapshot will faithfully freeze leaked truth.

### **Social/institutional knowledge risks**

The current tests are strong for one important case: co-located physical chest/facility is visible, but owner/holder/jurisdiction/office-holder remain unknown, and theft candidate/action is suppressed without explicit owner belief.

That does not prove seller stock, office control, rights transfer, queue grants, or record accessibility. FND-14A explicitly says social/relational facts require belief entries even when co-located.

### **Planning snapshot/source admission risks**

Entity-level admission is not enough. The same entity can have:

* locally observed physical kind,  
* stale believed location,  
* unknown owner,  
* current authoritative job state,  
* unknown sale listing,  
* record-known office affiliation.

A single `AdmissionSource` cannot safely describe all fields. Snapshot fields need source tags.

### **Cache/read-model risks**

Reports already note cache/snapshot scaling and memoization risks. FND-27 says caches are never truth. Caches must store source provenance or they will become indistinguishable from truth.

### **Debug/trace/visualizer risks**

Decision traces are rich and explicitly diagnostic. I did not find evidence that traces feed normal decisions. The risk is future UI: if trace data is reused for a player-facing “why” view, it can leak snapshot admissions or rejected alternatives the character did not lawfully know.

### **UI/player-facing risks**

The normal CLI action menu is architecturally aligned: it gets the controlled entity, builds `PerAgentBeliefView`, calls `get_affordances()`, filters internal actions, and queues strict action requests. Control switching changes `ControlSource` via transaction and `ControllerState`, not world laws.

That is good. But it means the player UI is only as lawful as `PerAgentBeliefView`.

### **Static/type-level protections**

Add source-typed view APIs:

pub enum FieldSource {  
   SelfAuthoritative,  
   LocalPhysicalSameTick,  
   DirectPossessionSameTick,  
   Belief {  
       acquired_tick: Tick,  
       claimed_event_tick: Option<Tick>,  
       confidence: Permille,  
       source: PerceptionSource,  
   },  
   GroundedEvidence {  
       evidence_entity: EntityId,  
       aspect: EvidenceAspect,  
   },  
   PublicTopology,  
}

pub struct Sourced<T> {  
   pub value: T,  
   pub source: FieldSource,  
}

Then replace risky methods with source-aware variants:

fn believed_sale_listing(&self, agent: EntityId, lot: EntityId) -> BeliefValue<Option<SaleListingBelief>>;  
fn observed_local_sale_listing(&self, actor: EntityId, lot: EntityId) -> Option<Sourced<SaleListingState>>;  
fn believed_container_of(&self, agent: EntityId, entity: EntityId) -> BeliefValue<Option<EntityId>>;  
fn observed_local_container_of(&self, actor: EntityId, entity: EntityId) -> Option<Sourced<Option<EntityId>>>;  
fn believed_production_job(&self, agent: EntityId, facility: EntityId) -> BeliefValue<Option<ProductionJobBelief>>;

The dispatch/commit path can still read truth. The AI/player planning path cannot.

---

## **11. Intentions, Repair, and Replanning Audit**

### **Current commitment lifecycle**

Worldwake now has a credible BDI-shaped shell. `AgendaState` manages committed, pending, suspended entries; agenda ticking can revive, merge, rank, commit, and demote entries. `IntentionFrame` update code creates/refreshes/suspends frames and clears frames when plans are lost or patience is exhausted.

This is directionally correct.

### **Switch margins / interruption**

Planning continuation uses current-vs-top ranked motive, priority class, and planning switch margin to decide continuation vs replanning. That is a good anti-thrash shape.

The risk is that motive score becomes too dominant. If concrete goal failure, stale belief, or action illegality is summarized into a number too early, the switch margin hides why a plan should change.

### **Plan revalidation**

Revalidation correctly re-enters the affordance path: it checks guards, resolves targets, tests affordance matches, and validates payload overrides/target-specific steps through `RuntimeBeliefView` and action handlers.

This is good. It keeps action legality centralized.

The danger is that revalidation inherits the same belief-view leaks. `RequiredFact::ResourceAccess` checks `view.can_control`, which is currently a social/control leak candidate.

### **Local repair vs full replan**

The planning module classifies accepted repair by same goal, commodity equivalence, counterparty rebinding, opportunity anchor rebinding, or route signature changes. This is useful, but it is close to becoming a second planner.

Repair should be allowed only when:

1. same intention remains,  
2. same goal semantics remain,  
3. substitute binding is local/belief-lawful,  
4. repair budget is smaller than planning budget,  
5. trace records rejected repair kinds,  
6. repair writes blocker/discrepancy state if it fails.

Anything else is full replanning.

### **Failure produces state/memory**

Failure handling clears current plan/frame, classifies blocker/discrepancy, records it, and sets dirty replan. That is the right shape. It does not silently retry by default.

But some classifier logic again uses risky view methods: seller stock, production jobs, reservation ranges, queues. Fixing the belief view will fix much of this.

### **Verdict on intentions**

**Stable but patch-accumulated.** The architecture is not rails. Intentions are revisable and interruptible. But the logic is distributed across agenda, frame, planning, revalidation, repair, failure handling, blocker memory, discrepancy memory, and expectation stores. That is a lot of places to reason about one lifecycle.

Near-term goal: document one canonical state machine for commitment lifecycle and make traces prove every transition.

---

## **12. Consolidation or Redesign Options**

### **Option A — Conservative Hardening**

Minimal changes.

**Benefits**

* Keeps current architecture intact.  
* Adds tests for known leak families.  
* Low migration cost.

**Risks**

* Leaves `PerAgentBeliefView` as a convention-heavy wall.  
* Does not eliminate legacy extractor/fallback seams.  
* HTN remains overnamed.

**Migration cost**

Low to medium.

**Alignment with FOUNDATIONS**

Improves FND-31 validation but does not fully satisfy FND-14/FND-28.

**When to choose**

Only if you need a short stabilization pass before a bigger consolidation.

### **Option B — Moderate Consolidation**

Keep architecture, reorganize ownership boundaries.

**Benefits**

* Fixes the highest-risk belief leaks.  
* Preserves GOAP/action legality.  
* Keeps HTN but narrows authority.  
* Removes candidate-generation fossil seams.  
* Makes player POV safer immediately.

**Risks**

* Requires touching central belief-view APIs.  
* Some tests will need substantial rewrites.  
* May reveal more stale assumptions in current behavior.

**Migration cost**

Medium.

**Alignment with FOUNDATIONS**

Best near-term fit for FND-14, FND-14A, FND-19, FND-20, FND-28, FND-29, FND-31.

**When to choose**

Choose this now.

### **Option C — Aggressive Redesign**

Rebuild around source-typed epistemic state and per-field sourced snapshots.

**Benefits**

* Strongest safety.  
* Makes illegal truth reads difficult by construction.  
* Future player POV becomes robust.

**Risks**

* High migration cost.  
* Likely breaks much current planner code.  
* Could stall behavior work for too long.

**Migration cost**

High.

**Alignment with FOUNDATIONS**

Excellent, but heavy.

**When to choose**

Choose only if Option B tests reveal pervasive leaks that cannot be localized.

---

## **13. Recommended Architecture**

Choose **Option B — Moderate Consolidation**.

### **Preserve**

* Tactical GOAP over ordinary `ActionDef` affordances.  
* `PerAgentBeliefView` as the single AI/player view wall, but harden it.  
* `PlanningSnapshot`, but add field-source provenance.  
* Agenda/IntentionFrame shell.  
* Portfolio triage.  
* HTN `MethodRegistry`, but narrow its authority.  
* Decision traces and event log.

### **Remove or collapse**

* `LEGACY_EXTRACTOR_ORDER` as a live authority name.  
* Blocked-self-care fallback outside the extractor registry.  
* Fake group-hunt HTN method, or rename it to a support-declaration/direct-pursuit method.  
* Any sentinel placeholder payload values that can be mistaken for real IDs.  
* Any method schema fields that are not enforced or traced.

### **Consolidate**

* Candidate emission under schema/extractor registry.  
* Observation/anomaly/discrepancy detection outside candidate emission.  
* Repair authority under a narrow “same intention, local rebinding” contract.  
* Player action visibility under the same belief-affordance path, with explicit debug separation.

### **Target pipeline**

World events  
 -> perception/testimony/records produce sourced belief/memory records  
 -> source-typed PerAgentBeliefView  
 -> schema-owned candidate extractors  
 -> ranking + portfolio budget triage  
 -> agenda/intention continuation or adoption  
 -> per-field sourced PlanningSnapshot  
 -> optional method-guided strategic stages  
 -> tactical GOAP over ActionDef affordances  
 -> revalidation through same affordance path  
 -> authoritative dispatch/commit  
 -> event log + belief updates + contrastive traces  
---

## **14. File-by-File Proposal**

| File / Module | Proposed change | Confidence | Reason | Acceptance criteria |
| ----- | ----- | ----- | ----- | ----- |
| `crates/worldwake-sim/src/per_agent_belief_view.rs` | Replace current remote sale listing, seller, container, possessor, production job, queue/reservation, rights/control, load reads with local-observation or belief-backed variants. | High | Central leak surface. | Remote unseen changes do not alter AI candidates/plans/player affordances. |
| `crates/worldwake-sim/src/belief_view.rs` | Split broad `RuntimeBeliefView` into source-class traits or return `BeliefValue/Sourced<T>` for risky fields. | High | Current trait has no static boundary proof. | Compile-time distinction between local physical, belief, topology, and dispatch truth. |
| `crates/worldwake-ai/src/planning_snapshot.rs` | Move from entity-level `AdmissionSource` to per-field source provenance for risky fields. | High | Entity admission does not prove field legality. | Snapshot trace can say why each economic/social/temporal field is known. |
| `crates/worldwake-ai/src/candidate_generation.rs` | Remove `LEGACY_EXTRACTOR_ORDER`; make extractor order a canonical registry constant not named legacy, and ensure every out-of-band emitter is an extractor. | High | Fossil seam. | No candidate can be emitted outside declared extractor/source path. |
| `crates/worldwake-ai/src/goal_schema.rs` | Treat `candidate_extractors` as actual wiring authority; keep `relevant_ops` explicitly hint-only. | High | Schema consolidation only half-landed. | Tests fail if extractor emits undeclared goal family. |
| `crates/worldwake-ai/src/htn/method_schema.rs` | Add explicit `MethodSubgoalAuthority` or rename subgoals as stage hints. | Medium-high | Current method leaves overpromise. | Trace distinguishes stage hint from required leaf. |
| `crates/worldwake-ai/src/htn/methods.rs` | Remove/rename `fulfill_bounty_group_hunt` until real coordination artifacts exist. | High | It is misleading. | No method claims group hunt without recruit/contract/support/grant mechanism. |
| `crates/worldwake-ai/src/search/strategic.rs` | Trace method fallback legality and selected-method stage conversion explicitly. | Medium-high | Current trace can imply more than code enforces. | For each method attempt: selected/rejected/fallback/stage-hint list. |
| `crates/worldwake-ai/src/decision_trace.rs` | Add per-field admission/knowledge-path trace for snapshot fields and player action omission reasons. | Medium | Needed for contrastive diagnostics. | Trace answers “why did I not see this action?” and “why was this remote fact known?” |
| `crates/worldwake-cli/src/handlers/actions.rs` | Preserve current belief-affordance path; add tests for economic/social leak cases. | High | Shape is right. | AI/Human control-source swap preserves lawful affordance set for every leak scenario. |
| `crates/worldwake-ai/src/agent_tick/planning.rs` | Keep portfolio; trace whether candidate skipped by cap, probe, exhaustion, or different-goal stop. | Medium | Scaling/diagnostics. | Top-K omissions are traceable. |
| `crates/worldwake-ai/src/failure_handling.rs` | Ensure classifiers use only lawful local/belief-backed state; no remote current seller/job/queue reads. | High | Failure memory can become hidden sensing. | Failure classification cannot reveal unseen current facts. |
| `docs/planner-contracts.md` | Add “per-field source provenance” and “method-guided HTN” contract language. | High | Current contract good but not specific enough. | New specs/tests cite this as authority. |
| `docs/spec-drafting-rules.md` | Add rule: every new belief-view method declares source class and stale/unknown behavior. | High | Prevents future leaks. | Spec lint rejects source-ambiguous planner reads. |

Candidate replacement contract text for `docs/planner-contracts.md`:

### Planner-visible fields are source-scoped

A snapshot entity admission source does not authorize every field on that entity.  
Every planner-visible field whose value can vary outside same-tick local physical  
observation must carry one of:

- self-authoritative source,  
- same-tick local physical source,  
- direct-possession physical source,  
- explicit belief/memory source with acquired tick and confidence,  
- explicit record/testimony/evidence source,  
- public topology source.

Remote social, economic, ownership, control, queue, reservation, production-job,  
container, possessor, and stock/listing facts must not fall back to current  
authoritative world state merely because the entity is known.

Candidate replacement contract text for HTN:

### HTN method authority

The current HTN layer is method-guided strategic planning unless a method  
explicitly declares required leaves and tests prove enforcement.

A selected method may:  
- bias strategic stages,  
- narrow search,  
- contribute trace provenance,  
- set planning-budget hints.

A selected method may not:  
- bypass ActionDef affordances,  
- bypass dispatch/commit validation,  
- claim subgoals were enforced unless selected plan steps prove corresponding  
 ordinary action leaves,  
- forbid flat GOAP fallback without a method-required schema contract and  
 golden tests.  
---

## **15. Golden Scenario and Evaluation Matrix**

| Scenario / Metric | Purpose | Systems exercised | Required assertions | Failure smell |
| ----- | ----- | ----- | ----- | ----- |
| Remote seller delists unseen | Hidden economic truth leak | belief view, candidate generation, trade planning, player actions | Agent/player keeps stale belief or unknown; no instant delist knowledge | Remote stock omniscience |
| Remote seller restocks unseen | Hidden opportunity leak | opportunity compiler, ranking, GOAP | No new acquire candidate unless testimony/record/local observation | Magical market knowledge |
| Item moved between containers unseen | Container/possessor leak | inventory belief, snapshot, theft/loot/trade | Planner uses last-known container or unknown | Current possessor leak |
| Rights transfer unseen | Social/control leak | `can_control`, affordances, UI | Action visibility unchanged until rights belief/record acquired | Player menu changes magically |
| Remote workstation job starts unseen | Production job leak | production/restock, `TargetLacksProductionJob`, failure handling | Planner does not know job busy/free remotely | Remote facility omniscience |
| Remote queue/grant changes unseen | Temporal contention leak | reservations, queues, affordance contention | Planner cannot optimize against unseen queue | Invisible scheduling oracle |
| Evidence entity limited fields | Snapshot source test | candidate evidence, snapshot admission | Snapshot imports only evidence-backed aspects | Evidence becomes truth portal |
| AI/Human control swap economic leak | Player symmetry | CLI actions, affordances, control source | AI and Human see identical lawful menu, with no remote truth | Player-only or shared omniscience |
| Debug trace separation | UI safety | traces, visualizer, CLI | Normal UI cannot read decision trace facts | Debug becomes gameplay oracle |
| HTN direct bounty stale target | Method boundary | HTN selector, strategic stages, GOAP | Method uses believed place, not current target place | HTN leaks target location |
| HTN group hunt method | Fake method detection | HTN registry/methods/traces | Either removed/renamed or proves coordination artifact | Story beat disguised as method |
| HTN fallback explanation | Fallback contract | selector, strategic search, trace | Trace says no viable method/method no stages/fallback legal | Silent flat fallback |
| Method-required negative test | Prevent premature method-required | HTN/GOAP | No method-required current goals | Overconstraint/churn |
| Same-goal sibling planning | Intention stability | ranking, portfolio, planning cap | Trace says why same-goal continuation stopped | Hidden top-K truncation |
| Repair rebind seller | Repair vs replan | failure handling, repair, agenda | Repair only same intention and lawful known substitute | Repair as second planner |
| Failed repair parking | Failure memory | agenda, pending repair, blocker memory | Goal parked with revival trigger, not silent retry | Dead-end loop |
| Blocker TTL/clearing | Memory churn | blockers/discrepancies, candidate filtering | Blocker clears only by lawful observed condition or TTL | Permanent ghost blocker |
| Contention local queue | Reservation legality | scheduler, queues, affordances | Local observed queue affects action; remote unseen queue does not | Mixed local/remote truth |
| Candidate extractor declaration | Fossil removal | schema, candidate generation | Every emitted candidate source is declared | Hidden emitter |
| Portfolio cap stress | Scaling | ranking, portfolio, planning | Trace reports cap-hit and omitted candidates | Invisible budget starvation |
| 100 agents / 100 places soak | Architectural scaling | snapshots, planning, traces | Bounded snapshot count, trace volume, planning time | Floyd-Warshall/per-agent explosion |
| Trace contrastive audit | Diagnostics | decision_trace, event log | Can answer why selected/rejected/unknown/fallback/legal | Decorative explanation |

---

## **16. Research-Backed Design Rules For Future AI Work**

1. **A goal deserves HTN only when it encodes a reusable lawful pursuit pattern that flat GOAP handles poorly.** “Attack target” usually does not need HTN. “Investigate violation through witness/ledger before enforcement” might.  
2. **Flat GOAP is enough when ordinary action preconditions/effects express the pursuit cleanly.** Do not add method schemas because a behavior feels narratively important.  
3. **Method-required is justified only when flat fallback would be semantically illegal.** Inefficient, less elegant, or less story-like is not enough.  
4. **A motive becomes an intention only after ranking, feasibility, portfolio budget admission, and commitment lifecycle checks.** Candidate emission is not commitment.  
5. **A blocker is belief memory when it describes what this agent learned from failure.** It is world state only if it is an external artifact like a queue, reservation, locked door, record, notice, contract, or physical obstruction.  
6. **Repair is allowed only for bounded rebinding under the same intention.** If the goal semantics change, run full replanning.  
7. **A cache is legal only if it stores its source class and invalidation condition.** A cache without provenance becomes truth.  
8. **A derived score becomes dangerous when it answers a factual question.** Scores can decide planning order; they cannot decide whether a seller exists or whether an agent knows a law.  
9. **Player-facing UI leaks omniscience whenever it displays facts/actions not available through the controlled character’s belief/action surface.** Debug mode must be explicit and separate.  
10. **A trace is sufficient only if it is contrastive and source-backed.** “Selected X” is decorative. “Selected X because A/B/C; rejected Y because no belief/legal action; fallback because method precondition failed; unknown because no carrier” is useful.  
11. **Every new `RuntimeBeliefView` method must declare source semantics before implementation.** The declaration should say: self-authoritative, same-tick local physical, direct possession, belief-backed, public topology, record/testimony-backed, or dispatch-only truth.  
12. **HTN leaves are not enforced unless the selected `PlannedPlan` proves corresponding ordinary `ActionDef` steps.**

---

## **17. Open Questions and Uncertainties**

| Uncertainty | Why it matters | Evidence needed |
| ----- | ----- | ----- |
| How many action handlers call risky belief-view methods indirectly? | Hidden leaks may occur outside inspected central files. | Targeted grep/fetch of all handlers using sale listing, production job, control, queue, reservation, container/possessor. |
| Whether visualizer/debugger data can be opened in normal play UI | Trace/debug leak risk. | Inspect visualizer/debugger modules and normal UI command routing. |
| Whether `GroundedEvidence` intentionally means “current physical inspection” in some cases | It may be lawful for local evidence, unlawful for rumor/record evidence. | Audit every source of `evidence_entities` in `GoalOffer`. |
| Whether `GoalKey` quantity normalization is always safe | Could conflate distinct acquisition intentions. | Tests with same commodity, different desired quantity/horizon/source. |
| Whether portfolio fixed slots are enough | Fixed slots may collapse diverse motives. | Soak tests with mixed survival, duty, economic, social, danger, justice goals. |
| Whether repair route-signature classification is semantically correct | Could misclassify provider/route substitutions. | Golden repair traces with same seller/different route, different seller/same route, same goal/different commodity. |
| Whether `ViolationId -> EntityId` mapping in HTN selector is contractual | Looks suspicious without context. | Inspect violation/evidence representation; add explicit type if valid. |
| Whether `ActionDefId(u32::MAX)` placeholder is unreachable | Sentinel values rot. | Add test that no plan/action trace/dispatch sees placeholder ID. |
| Whether existing belief-wall tests cover all FND-14A classes | They cover theft/social authority and remote pursuit; not economic/queue/records. | Add matrix in section 15. |

---

## **18. Third-Iteration Prompt Suggestions**

The third audit should focus on **proof**, not architecture aesthetics.

Suggested third-iteration prompt:

# Mission: Third-Iteration Belief-Proof and Player-POV Audit — Worldwake

Audit whether the second-iteration changes actually proved belief-local AI/player symmetry.

Highest priorities:

1. Verify every `RuntimeBeliefView` method has source-class semantics.  
2. Search for all current-world reads reachable from AI planning, candidate generation,  
  ranking, HTN selection, revalidation, repair, failure handling, and player action UI.  
3. Prove remote economic, social, temporal, containment, rights, and production facts  
  do not leak.  
4. Verify `PlanningSnapshot` carries per-field source provenance for risky fields.  
5. Verify HTN methods are either honestly stage hints or enforced required leaves.  
6. Verify no method is marked method-required without a schema/test contract.  
7. Verify candidate generation has no fossil emitters outside schema/extractor authority.  
8. Verify player control uses the same lawful action surface as AI.  
9. Verify debug/trace/visualizer data cannot influence normal play.  
10. Run golden adversarial scenarios for stale belief, false belief, contradiction,  
   unknown facts, remote unseen changes, and control-source swaps.

Deliver:  
- hidden authority leak diff since second iteration,  
- per-view-method source table,  
- per-snapshot-field source table,  
- HTN method enforcement table,  
- player-POV leak matrix,  
- failing/added golden tests,  
- remaining uncertainty list.

The strongest next milestone is not “more interesting NPC behavior.” It is **making ignorance durable**. Until remote economic/social/temporal truth leaks are dead, more behavior will just make omniscience harder to diagnose.
