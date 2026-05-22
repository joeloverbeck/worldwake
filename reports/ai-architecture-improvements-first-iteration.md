# **Worldwake AI Architecture Improvement Proposal — Iteration 1**

## **1. Repository Grounding**

* **Repository:** `joeloverbeck/worldwake`  
* **Default branch:** `main`  
* **Current main ref / commit SHA:** `1281324690fc4c15b73aca8d8b762b9d515e20b7`  
* **SHA availability:** The Git tool exposed the current `main` SHA through a `main...main` compare. All material repository evidence below was still fetched directly from `ref=main`; search results were used only to locate files.

The project constitution is `docs/FOUNDATIONS.md`. It requires local causality, concrete state, lawful information carriers, belief/world separation, player/AI symmetry, resource-bounded practical reasoning, revisable intentions, concrete learning state, state-mediated systems, derived summaries as caches, and queryable causal history.

## **2. Freshness / Anti-Duplication Method**

I treated the live GitHub repository as the only source of truth. I used repository/code search to discover files and then fetched material files directly from `ref=main`, including the constitution, agent instructions, planner contracts, spec drafting rules, scenario roadmap, generated scenario coverage, the GOAP report skill, current active reports, AI modules, belief-view modules, affordance/action definitions, action handlers, CLI action surfaces, visualizer snapshot code, diagnostics, telemetry, opportunity compilation, HTN/search/revalidation/repair files, and inline focused tests.

I fetched the active fourth consolidation report only as a suspicion prompt, not as proof. Every reported defect below was checked against current `main` code. Archive search hits appeared during discovery, but I did not fetch or use archived files as evidence.

**Landed-fix checks and omissions:**

* I verified that the old broad `entity_kind` suspicion should not be reported as a simple current-truth leak: current `entity_kind` now distinguishes public topology, self/local/possessed authoritativeness, and belief/last-seen paths. The remaining issue is field-level source discipline, not `entity_kind` alone.  
* I did not report `resource_source`, `wash_basin_state`, or `has_production_job` as blanket current leaks; the current implementations are more local/belief-gated than the older suspicion implied.  
* I did not claim `listed_sale_lots_at` leaks remote listings. Current evidence shows it is local actor-place oriented. The current defect is seller/controller identity and other social/economic fields leaking through belief-known gates.  
* I did not claim the visualizer feeds normal AI/player state. I verified only that it builds an omniscient debug snapshot from `World`; the proposed fix is a static debug-only fence, not a gameplay-leak accusation.  
* I did not claim HTN methods are currently enforced hidden rails. Current code and tests keep method subgoals as `StageHint`; `RequiredActionLeaf` is not active.

## **3. Executive Verdict**

This first improvement iteration should be **belief-boundary hardening plus a moderate object-capability split**. It should not be a wholesale AI formalism rewrite.

The blunt truth: Worldwake’s current AI architecture is already structurally ambitious and mostly pointed in the right direction. It has GOAP-style action search, HTN stage guidance, concrete ranking/motive provenance, current-plan persistence, revalidation, repair, failure memory, discrepancy records, source reliability handling, decision traces, diagnostics, and performance telemetry. Throwing that away would be wasteful.

The serious architectural contradiction is narrower but deadly: **the contracts say planner/player-visible data must be source-scoped, while `PerAgentBeliefView` still carries ambient authoritative `&World` power and some belief-facing accessors still read live world truth after only weak “known entity” admission.** That is exactly the kind of leak `FOUNDATIONS.md` forbids.

**Do first:** make illegal information flow hard to express. Split authoritative world access from lawful character POV access; then require field-level provenance in planning snapshots and traces.

**Do not do yet:** no LLM runtime agents, no RL, no new gameplay mechanics, no global director, no scripted drama rails, no forced HTN action leaves, and no aggressive whole-crate rewrite until the moderate split proves insufficient.

**Preserve:** the current GOAP/HTN/utility/BDI-ish hybrid. It matches Worldwake better than pure behavior trees, pure HTN, pure GOAP, full POMDP planning, LLM agents, or RL. The hybrid needs sharper responsibility boundaries, not replacement.

## **4. Current Architecture Map**

### **Crate responsibilities**

`AGENTS.md` defines the crate split: `worldwake-core` owns ECS/data/event-log foundations; `worldwake-sim` owns simulation-facing definitions, affordances, belief views, scheduler surfaces, and action definitions; `worldwake-systems` owns authoritative action handlers; `worldwake-ai` owns candidate generation, ranking, planning, revalidation, repair, traces, and agent tick; `worldwake-cli` exposes the player/REPL surface. It also states the critical invariants: no player special casing, belief-only planning, information locality, deterministic ordering, append-only event log, state-mediated systems, no compatibility fossils, and the “Authoritative-to-AI Impact Rule.”

### **Belief and POV surfaces**

`worldwake-sim/src/belief_view.rs` has a strong substrate: `BeliefValue<T>`, `BeliefRead<T>`, confidence, source, acquired tick, status tags including stale/disputed/contradicted, and projection helpers. That is the right foundation.

The problem is `PerAgentBeliefView`. It stores `agent`, `current_tick`, `world: &World`, optional registry/runtime state, and a belief store. It implements runtime belief traits while retaining authoritative world access.

The worst verified defect is in `ControlBeliefView`: `believed_rights` and `can_control` require only self or a believed entity, then call authoritative `world.effective_rights` and `world.can_exercise_control`. That violates the planner contract’s explicit warning that rights/control need believed-rights/jurisdiction aspects rather than hidden authoritative fallback.

Other current-main leaks are more diffuse:

* `facility_controller_at` uses believed-present candidates but then checks authoritative `world.can_exercise_control`; `seller_for_sale_lot` depends on that.  
* `direct_container`, `direct_possessor`, and `item_lot_commodity` read current world truth once `knows_entity` is true; `knows_entity` includes old beliefs, institutional claims, and last-seen records.  
* `stock_storage_policy`, mixed queue policy reads, merchandise profile, loyalty, office/reward, and faction policy accessors still expose current social/economic/political/faction facts under weak or partial gates.

### **Candidate generation and opportunity compilation**

Candidate generation is side-effect-free and trace-rich. It emits offers, evidence, suppression diagnostics, pending violations, pending discrepancies, source reliability failures, acquisition resets, reachable/filtered place counts, and candidate-source metadata. That is strong architecture.

The opportunity compiler is also on the right conceptual track: opportunities are anchored in `BeliefRef`, carry possible effects, information topics, believed legal status, social exposure, risks, and salience; it applies cognitive/perception caps and learned-memory damping. But because it consumes `RuntimeBeliefView`, it inherits view-lawfulness problems where the view leaks current truth.

### **Ranking, portfolio, and motive handling**

Ranking is not mere “score soup.” It preserves ranked/suppressed/damped/zero-motive categories and carries motive-source contributions, source reliability discounts, competition discounts, and source composites.

The portfolio layer uses concrete slots — survival, pain care, obligation duty, economic opportunity, social motive — and operating modes. It respects committed candidates and probes feasibility before slot selection. This is useful BDI-ish intention arbitration, provided the underlying candidate facts are lawful.

### **GOAP / tactical search**

Search builds `PlanningState` from a `PlanningSnapshot`, uses strategic planning when applicable, then tactical search with action generation, A*/heuristic guidance, typed barriers, beam/pruning controls, search budgets, and explicit terminal outcomes such as found, unsupported, budget exhausted, and frontier exhausted.

`PlanningState` itself is comparatively healthy: it stores snapshot-derived state, caches derived queries, does not need `World`, and implements control checks from snapshot fields. The weakness is upstream contamination of snapshot fields.

### **HTN / method guidance**

HTN method schemas have preconditions, subgoals, motive bias, and budget hints. Crucially, subgoals distinguish `StageHint` from `RequiredActionLeaf`; current registry tests assert current methods are only `StageHint` and no required leaves exist. This is good: HTN is guidance, not a hidden rail.

The selector evaluates methods under `RuntimeBeliefView`, so it also depends on lawful POV.

### **Planning snapshot/state**

`PlanningSnapshot` is the central planner-visible compression. It captures spatial, inventory, combat, social, economic, political, temporal, facility, and control fields. It has `AdmissionSource`, but that source is **entity-level**: self, local same-tick physical, grounded evidence, belief last seen, possession frontier, or public topology.

That is not enough. `build_snapshot_entity` stores many dynamic/social fields after calling belief-view accessors — `controllable_by_actor`, `stock_storage_policy`, `merchandise_profile`, `seller_for_sale_lot`, `office_data`, container/possessor, etc. — but the trace only says why the entity entered the snapshot, not why each field is lawful.

### **Revalidation, repair, and failure handling**

Plan revalidation checks guard invalidators, belief contradictions, resource access, route status, and current affordances. This is structurally correct, but `ResourceAccess` uses `view.can_control`, so it inherits the control leak.

Failure handling records blockers/discrepancies and clears or updates plan state. Plan repair has explicit repair attempts such as rebind, replace provider, insert verification, downgrade to typed barrier, and abandon; it carries evidence and budget. That is good, but it needs tests proving repair does not silently substitute remote truth.

### **Action affordance enumeration and dispatch**

`ActionDef` is strong: it includes actor constraints, target specs, preconditions, reservations, duration, costs, interruptibility, commit conditions, visibility, tags, payloads, handlers, binding strictness, guards, expectations, and effect schema.

`get_affordances` enumerates actions through `RuntimeBeliefView`; control and target-control preconditions use `view.can_control` and `view.believed_rights`. Thus affordance generation is architecturally symmetric but currently symmetrically vulnerable.

Authoritative dispatch is healthier. Trade/production handlers validate same-place constraints, listings, controller authority, quantities, exclusive facility grants, and effect application through authoritative transaction logic. This is the right separation: planner may be wrong, dispatch must be authoritative.

### **Player/AI symmetry**

The CLI action list and `do` path construct `PerAgentBeliefView::with_runtime_from_world` and call `get_affordances`, so the player POV inherits the same leaks as AI affordances. Target labels also mix live topology/current place and belief-derived labels.

This is bad in exactly the right way: the architecture is symmetric, but the shared surface is not lawful enough. Fixing one surface can fix both.

### **Visualizer / debug**

The visualizer builds an omniscient snapshot from `World`, including all places, agents, positions, needs, pressures, and derived pressure values. I did not verify that this feeds normal player/AI surfaces. The issue is weaker: omniscience is separated by crate/UI convention, not by an explicit `DebugOnly` capability type.

### **Traces, diagnostics, and proof surfaces**

Decision traces are already broad: affordances, candidates, planning attempts, selection, portfolio, execution, failures, discrepancies, exhaustion, frame transitions, cache counters, repair attempts, and ranked-goal provenance are all present.

Scenario diagnostics aggregate planning, repair, stale/contradicted belief actions, source reliability changes, route preference changes, false rumor propagation, coordination, performance, method usage, and cache metrics.

The missing proof surface is **field-source provenance**: current `SnapshotAdmissionTrace` records entity admission only.

Scenario coverage also shows proof gaps around cognitive archetypes and unmapped behavior-affecting scenario components such as portfolio weights, intention disposition, expectation store, last-seen memory, and social observations.

## **5. FOUNDATIONS Alignment Matrix**

| Area | Alignment | Gap / risk |
| ----- | ----- | ----- |
| FND-1 local causality | Partial | Systems/action dispatch preserve concrete validation, but planner/player-visible accessors can see remote current truth through `PerAgentBeliefView`. |
| FND-3 concrete state over abstract scores | Strong | Ranking/portfolio/opportunity compiler carry concrete motive sources, beliefs, risks, legal status, memory damping, and slots. Keep this. |
| FND-7 locality | Partial | Local physical observation is recognized, but social/economic/control facts still leak through weak known-entity gates. |
| FND-8 action preconditions/duration/cost/occupancy | Strong | `ActionDef` includes preconditions, duration, costs, reservations, interruptibility, commit conditions, expectations, and effect schema. |
| FND-9 scheduling/tie-breaking | Mostly aligned | Deterministic ordering appears throughout ranking/search/portfolio and core guidance; no major issue found in this pass. |
| FND-12 performance may compress, never cheat causality | Partial | Snapshot/caches/telemetry are good, but snapshot compression currently loses field source lawfulness. |
| FND-14 / 14A / 14B belief/world separation | Failing in key places | The docs are clear; the code still gives a belief view ambient `&World` and authoritative control/social reads. |
| FND-15 knowledge carriers | Partial | Belief refs, testimony, expectation stores, and opportunity source beliefs exist; dynamic snapshot fields lack per-field carrier references. |
| FND-16 ignorance/stale/false/contradiction | Good substrate, weak enforcement | Status tags, discrepancy memory, revalidation, and diagnostics exist; illegal live reads can erase ignorance before those systems matter. |
| FND-17 expectation violation | Strong | Expectation mismatch, decisive evidence, replan triggers, source reliability failures, and diagnostics are present. |
| FND-18 records/evidence/causal history | Strong but incomplete | Event log, decision traces, repair traces, belief refs, and diagnostics exist; add field-source traces. |
| FND-19 player/AI symmetry | Structurally strong, currently leaky | CLI and AI share affordance generation, but the shared belief view leaks. |
| FND-20 bounded practical reasoning | Strong | Budgets, caps, frontier exhaustion, beam truncation, opportunity caps, and telemetry exist. |
| FND-21 intentions as revisable commitments | Strong | Current plan continuation, revalidation, invalidation, repair, and replanning are explicit. |
| FND-22 / 22A diversity and learning | Partial | Concrete profile/memory/state exists, but generated scenario coverage shows cognitive archetype proof gaps and unmapped components. |
| FND-26 systems interact through state | Strong | Action systems validate through world transaction/effects; AI does not directly command world mutations. |
| FND-27 summaries are caches, not truth | Partial | `PlanningState` says caches are derived and invalidated; snapshot admission can still reify unproven fields as planner truth. |
| FND-28 no compatibility fossils | No major defect found | Current contracts explicitly forbid fossils; this proposal avoids compatibility shims that preserve contradiction. |
| FND-29 / 29A debuggability and causal history | Strong foundation | Decision traces and diagnostics are strong; field-level provenance is the missing high-value extension. |

## **6. Research Synthesis**

GOAP is still a good fit for Worldwake. The main lesson from F.E.A.R.-style GOAP is not “use a planner everywhere”; it is that goals and actions with preconditions/effects reduce hand-authored transition rails and let the agent compose behavior at runtime. That aligns with Worldwake’s anti-script constitution. The caveat is crucial: GOAP must search over the agent’s lawful knowledge, not the authoritative world.

HTN planning is useful as **method guidance**, not as hidden enforcement. SHOP2’s HTN planning success came from task/method decomposition plus temporal/metric planning support, which maps well to Worldwake’s current `StageHint` model. The repo is right to avoid `RequiredActionLeaf` until it can prove leaf legality through ordinary action search and trace evidence.

BDI is a good conceptual lens for Worldwake’s agenda/current-plan/frame architecture. BDI separates belief, desire/goal selection, and intention execution; Rao and Georgeff describe deliberation forming intentions as plans an agent is committed to achieving. Worldwake already has the better practical version: commitments are revisable, revalidated, and repairable. Classical BDI also has a known learning gap, so Worldwake should not import BDI wholesale; it should add concrete learning state under FND-22A.

Utility AI is valuable for ranking only when the score remains explainable. Game utility systems score possible behaviors from current context, but Worldwake must keep motive-source contributions, source reliability, concrete risks, and belief provenance visible; otherwise utility turns into a naked probability dial.

Behavior trees should not replace the current planner. BTs are modular and reactive, and research treats them as efficient structures for switching among tasks, but they do not by themselves solve provenance, belief lawfulness, causal history, or deliberative planning under stale/false belief. They may be useful as a local reactive execution layer later, not as the central architecture.

POMDP research is useful as inspiration for belief-state discipline, not as a full formalism to adopt. POMDP solving is intractable in general, and exact optimal policies can be too large; Worldwake needs deterministic, inspectable, resource-bounded planning, not stochastic policy optimization.

W3C PROV is directly relevant. PROV treats provenance as information about entities, activities, and people involved in producing data, useful for judging quality, reliability, or trustworthiness; it distinguishes entities, activities, agents, derivations, attribution, and usage. Worldwake should not import PROV wholesale, but it should adopt the pattern: every planner-visible dynamic field needs a source/provenance record with derivation lineage.

Object-capability design is the right security analogy. Capabilities combine an unforgeable reference with the authority to perform operations, and the model supports least privilege/privilege separation. Worldwake’s current `PerAgentBeliefView` is effectively an ambient-authority object: it can name world entities and ask authoritative questions. The fix is capability-shaped: planner/player code should receive only lawful POV capabilities.

Event sourcing and deterministic replay support the repo’s debugging goals. Fowler’s event-sourcing pattern captures all state changes as events, allowing reconstruction of past states and audit/debug replay; deterministic simulation research similarly treats determinism as critical for repeatable, trustworthy debugging. Worldwake already has an event log and traces; field-source provenance would complete the causal explanation loop.

## **7. Ranked Architecture Proposals**

### **Proposal 1 — Source-Scoped Character POV Capability Boundary**

**Rank:** 1  
 **Verdict:** Adopt.  
 **Problem verified on current main:** `PerAgentBeliefView` stores authoritative `&World`; belief-facing rights/control methods call authoritative `effective_rights` and `can_exercise_control` for merely believed entities; other social/economic/faction fields leak through weak gates.  
 **Current-main evidence:** `ControlBeliefView`, `facility_controller_at`, `seller_for_sale_lot`, container/possessor, stock policy, queue policy, office/reward, loyalty, faction policy, and merchandise accessors all show the issue in current fetched files.  
 **Research support:** Object-capability and ambient-authority models support making invalid authority unrepresentable instead of relying on discipline.  
 **FOUNDATIONS alignment:** Directly addresses FND-14/14A/14B, FND-15, FND-16, FND-19, FND-26, FND-27.  
 **Design:** Introduce a lawful `CharacterPovView` / `PlannerPovView` capability family that exposes only source-scoped accessors: self-state, same-tick local physical observation, direct possession, belief/memory/evidence, and public topology. Social facts — rights, ownership, jurisdiction, controller, seller, faction, office, reward, policy, obligation — must come from explicit belief/evidence/institutional carriers or be absent/unknown. Authoritative `World` remains available only to orchestration, perception construction, debug, tests, and action dispatch validation.  
 **Affected files / crates:** `worldwake-core` belief/source types; `worldwake-sim` belief views and affordance query; `worldwake-ai` runtime view construction, snapshots, planning, revalidation; `worldwake-cli` action listing; `worldwake-visualizer` debug fences; `worldwake-systems` dispatch stays authoritative.  
 **Migration strategy:** First replace `ControlBeliefView` implementation with belief-backed rights/control reads. Then split high-risk accessors into physical vs social/dynamic source classes. Then move AI/CLI to `CharacterPovView`. Finally add static gates.  
 **Proof strategy:** Unit tests for every high-risk accessor; integration tests for snapshots and affordances; CLI/AI symmetry tests; dispatch rejection tests for false beliefs; compile/static gate forbidding planner/player-facing code from calling authoritative control APIs.  
 **Risks:** Medium churn; temporary test breakage where tests expected omniscient convenience; source types may initially feel verbose.  
 **What this makes impossible:** Planner/player code accidentally learning remote current rights/control/social/economic facts by naming an entity.  
 **What this preserves:** Current GOAP/HTN/ranking/revalidation/repair architecture.  
 **What this replaces or removes:** Ambient authoritative `&World` access from belief/player/planner-facing APIs; weak `knows_entity => live field` patterns.  
 **Why now:** Every other architecture improvement depends on lawful inputs.

### **Proposal 2 — Per-Field Provenance in Planning Snapshots and Decision Traces**

**Rank:** 2  
 **Verdict:** Adopt.  
 **Problem verified on current main:** `PlanningSnapshot` records entity-level `AdmissionSource`, while dynamic/social fields are stored as plain values. `SnapshotAdmissionTrace` likewise records entity admission only.  
 **Current-main evidence:** `build_snapshot_entity` stores `stock_storage_policy`, `controllable_by_actor`, `seller_for_sale_lot`, office data, container/possessor, and other fields through view accessors without per-field provenance.  
 **Research support:** PROV’s entity/activity/agent/derivation model is a good pattern for trustable field lineage.  
 **FOUNDATIONS alignment:** FND-4, FND-14B, FND-15, FND-16, FND-18, FND-27, FND-29.  
 **Design:** Replace high-risk dynamic fields with `PlannerField<T>` or `FieldRead<T>`: `{ value, source_class, provenance_ref, observed_tick, status, confidence, derivation }`. Entity admission remains useful but cannot justify any dynamic field. A field may be absent/unknown even if the entity is known.  
 **Affected files / crates:** `planning_snapshot.rs`, `planning_state.rs`, `decision_trace.rs`, `scenario_diagnostics`, `candidate_generation`, `affordance_query`, CLI labels.  
 **Migration strategy:** Start with control/rights/container/possessor/seller/stock/queue/faction/office fields. After proofs, expand to all dynamic fields. Keep public topology and same-tick local physical fields simpler but still tagged.  
 **Proof strategy:** Snapshot invariant test: every dynamic planner-visible field has provenance or is absent. Decision-trace assertions must show field source for selected candidates, denied candidates, and action visibility.  
 **Risks:** Trace size and ergonomics. Mitigate with compact provenance IDs and debug expansion.  
 **What this makes impossible:** Treating `BeliefLastSeen` or entity admission as permission to read current dynamic truth.  
 **What this preserves:** Snapshot compression and planning-state cache design.  
 **What this replaces or removes:** Entity-level admission as a surrogate for field lawfulness.  
 **Why now:** It turns belief-boundary safety from code review into testable evidence.

### **Proposal 3 — Unified Player/AI POV Affordance Surface**

**Rank:** 3  
 **Verdict:** Adopt.  
 **Problem verified on current main:** CLI `actions` and `do` paths create `PerAgentBeliefView::with_runtime_from_world` and call `get_affordances`; `get_affordances` uses the same control/right accessors as AI.  
 **Current-main evidence:** Player and AI are symmetric, but around a leaky view. Target labels also mix live and belief-derived information.  
 **Research support:** GOAP/action-selection systems are only as fair as their sensors/knowledge cache; Worldwake needs identical lawful POV for human and AI control.  
 **FOUNDATIONS alignment:** FND-14B, FND-19, FND-29.  
 **Design:** `get_affordances` should accept the same lawful `CharacterPovView` for AI and player. CLI action labels should be generated by POV-safe labelers: public topology names, local physical labels, or belief-backed names. Add explicit debug commands for omniscient listings; do not conflate them with normal `actions`.  
 **Affected files / crates:** `worldwake-sim/src/affordance_query.rs`, `worldwake-cli/src/repl.rs`, `worldwake-cli/src/handlers/actions.rs`, `worldwake-ai/src/agent_tick/**`, visualizer debug surfaces.  
 **Migration strategy:** Build adapter first; move CLI and AI to it; then delete old leaky calls.  
 **Proof strategy:** For a given character/state, CLI action list and AI affordance list must match. Remote right changes must not appear in either until a lawful carrier arrives.  
 **Risks:** Some CLI convenience labels may disappear until belief carriers exist. That is a good failure.  
 **What this makes impossible:** Players seeing actions or labels the character cannot lawfully know while AI remains constrained, or vice versa.  
 **What this preserves:** The no-`Player` architecture and `ControlSource` symmetry.  
 **What this replaces or removes:** Separate informal assumptions for player menus vs AI affordances.  
 **Why now:** It is the most visible proof that belief-boundary fixes are real.

### **Proposal 4 — False/Stale/Contradicted Belief Discipline for Plan/Replan/Repair**

**Rank:** 4  
 **Verdict:** Adopt.  
 **Problem verified on current main:** Revalidation, repair, and failure handling are structurally strong, but some guard checks depend on leaky view accessors such as `can_control`.  
 **Current-main evidence:** Plan repair has evidence/budget/discrepancy mechanisms, and failure handling records blockers/discrepancies, but no verified proof yet that repair cannot silently correct remote truth once the view leaks.  
 **Research support:** BDI’s useful idea is persistent but revisable intention; Worldwake should use that while keeping belief fallibility explicit.  
 **FOUNDATIONS alignment:** FND-16, FND-17, FND-18, FND-21, FND-29.  
 **Design:** A stale/false belief may generate a plan. A contradictory belief may suppress or route to verification. Authoritative dispatch may reject a plan. That rejection must produce discrepancy/evidence/source-reliability updates, not magical snapshot correction. Repair may use only newly lawful evidence or downgrade to typed barrier/verification.  
 **Affected files / crates:** `plan_revalidation.rs`, `failure_handling.rs`, `plan_repair.rs`, `agent_tick/observation.rs`, `decision_trace.rs`, `scenario_diagnostics`.  
 **Migration strategy:** After Proposal 1, add false-belief canonical tests before modifying repair behavior. Then add trace assertions for each repair path.  
 **Proof strategy:** Golden scenarios for false control belief, stale stock belief, stolen/moved item, contradicted target location, and missing seller/controller.  
 **Risks:** Behavior may look “dumber” because agents remain wrong longer. That is correct Worldwake behavior.  
 **What this makes impossible:** Replan/repair using omniscient truth to hide an illegal premise.  
 **What this preserves:** Existing revalidation/repair machinery.  
 **What this replaces or removes:** Any implicit “correct the plan because current world says so” path.  
 **Why now:** Once POV is lawful, stale/false belief behavior becomes a first-class product feature.

### **Proposal 5 — Concrete Agent Diversity, Learning, Habits, and Doctrine State**

**Rank:** 5  
 **Verdict:** Adopt in modified form.  
 **Problem verified on current main:** There is already concrete learning/profile substrate — opportunity memory, testimony reliability, risk/law/perception/cognitive profiles, portfolio weights — but scenario coverage flags cognitive archetypes as absent and several behavior-affecting scenario components as unmapped.  
 **Current-main evidence:** Ranking consumes repair memory, learned opportunity memory, testimony reliability, source reliability, and concrete motive provenance. Scenario diagnostics track source reliability changes, route preference changes, false rumors, stale/contradicted belief actions, and archetypes.  
 **Research support:** Classical BDI does not itself solve learning, so concrete stored learning state must be added explicitly rather than assumed.  
 **FOUNDATIONS alignment:** FND-3, FND-16, FND-18, FND-20, FND-22/22A, FND-29.  
 **Design:** Define a small set of inspectable learning/habit state types: source reliability by topic/source, route preference memory, blocked-intent memory, risk/fear/courage modifiers, habit reinforcement counters, preference shifts, and institution-level doctrine. Every update needs origin, scope, update rule, decay/overwrite rule, and explanation hook.  
 **Affected files / crates:** `worldwake-core` profile/memory components; `worldwake-ai` ranking/candidate/diagnostics; scenario schema and coverage; golden tests.  
 **Migration strategy:** Do not invent new gameplay. First make existing profile/memory components scenario-definable and covered. Then add one or two proof-backed habit/source-reliability paths.  
 **Proof strategy:** Scenario coverage must map cognitive archetypes and portfolio weights. Golden tests must prove two agents with different concrete state make different decisions and traces explain why.  
 **Risks:** Easy to create abstract personality knobs. Reject those. Every change must have stored causal origin.  
 **What this makes impossible:** Hidden global adaptation or unexplained personality drift.  
 **What this preserves:** Current utility/ranking/portfolio architecture.  
 **What this replaces or removes:** Unmapped profile components and ad hoc behavior variation.  
 **Why now:** Belief-boundary safety protects what agents know; diversity/learning explains why lawful agents still differ.

### **Proposal 6 — Field-Source, Affordance-Legality, and Causal Explanation Trace Upgrade**

**Rank:** 6  
 **Verdict:** Adopt.  
 **Problem verified on current main:** Decision traces are broad but do not prove dynamic field provenance or normal-vs-debug action visibility.  
 **Current-main evidence:** Scenario diagnostics already aggregate rich planning/belief/performance metrics; extend them rather than replacing them.  
 **Research support:** Event sourcing and replay/debug practices support storing enough causal history to answer “how did we get here?”  
 **FOUNDATIONS alignment:** FND-17, FND-18, FND-27, FND-29/29A.  
 **Design:** Add `FieldSourceTrace`, `AffordanceLegalityTrace`, `PlayerActionVisibilityTrace`, `SourceInvalidationTrace`, `RepairEvidenceTrace`, and `CandidateAbsenceTrace`. These should be compact in normal runs and expandable in debug/golden runs.  
 **Affected files / crates:** `decision_trace.rs`, `scenario_diagnostics`, `affordance_query`, `planning_snapshot`, CLI, visualizer.  
 **Migration strategy:** Attach traces to the high-risk fields first. Then expose query helpers and golden assertions.  
 **Proof strategy:** Goldens assert not just selected action, but absence/suppression source, field provenance, and denial reason.  
 **Risks:** Trace bloat. Mitigate with IDs, sampling, and debug-level expansion.  
 **What this makes impossible:** A behavior change with no explainable lawful premise.  
 **What this preserves:** Existing decision trace shape and diagnostics aggregator.  
 **What this replaces or removes:** Entity-only admission as sufficient explanation.  
 **Why now:** Once field provenance exists, traces become the enforcement/reporting layer.

### **Proposal 7 — Performance and Scaling Guards That Preserve Causal Equivalence**

**Rank:** 7  
 **Verdict:** Adopt with guardrails.  
 **Problem verified on current main:** Performance telemetry exists, but provenance/snapshot hardening will add churn and fan-out risk.  
 **Current-main evidence:** Search already has budgets, beam truncation, frontier/budget exhaustion, cache counters, and diagnostics; opportunity compiler has caps/flooring/damping.  
 **Research support:** Deterministic simulation research emphasizes repeatable outputs from the same initial conditions/event history; performance compression must not change causal behavior.  
 **FOUNDATIONS alignment:** FND-12, FND-20, FND-27, FND-29.  
 **Design:** Add provenance-aware cache metrics, snapshot field counts, trace-size budgets, source-cache invalidation counters, and causal-equivalence tests comparing uncompressed vs cached snapshot construction.  
 **Affected files / crates:** `perf_telemetry.rs`, `planning_snapshot`, `planning_state`, `scenario_diagnostics`, soak/golden harness.  
 **Migration strategy:** Instrument before optimizing. Then compact provenance IDs and cache only derived lawful reads.  
 **Proof strategy:** Soak tests must prove no unacceptable planning-time regression and no behavior divergence except where old behavior depended on illegal information.  
 **Risks:** Performance panic leading to causal shortcuts. Ban that explicitly.  
 **What this makes impossible:** “Optimization” that reintroduces omniscient shortcuts.  
 **What this preserves:** Current budgeted practical reasoning.  
 **What this replaces or removes:** Unmeasured snapshot/provenance growth.  
 **Why now:** Boundary hardening will otherwise get blamed for performance regressions without data.

### **Proposal 8 — Formalism Responsibility Boundaries: Keep the Hybrid, Clarify the Jobs**

**Rank:** 8  
 **Verdict:** Adopt as architecture clarification; reject formalism replacement.  
 **Problem verified on current main:** The hybrid is already present and useful, but responsibility boundaries should be documented and tested: ranking chooses intention pressure, HTN gives lawful stage hints, GOAP/search proves action sequences, revalidation/repair handles execution drift, dispatch validates authority.  
 **Current-main evidence:** HTN current tests keep subgoals as `StageHint`; search and action affordance layers remain the action proof layer.  
 **Research support:** GOAP, HTN, BDI, utility, and BT research each solve different parts; none alone fits Worldwake’s constitutional constraints as well as the current hybrid.  
 **FOUNDATIONS alignment:** FND-1, FND-3, FND-8, FND-20, FND-21, FND-26.  
 **Design:** Write a live planner formalism contract: utility/ranking cannot invent facts; HTN cannot enforce leaves without proof; GOAP/search cannot query authoritative world; repair cannot correct hidden truth; dispatch remains authoritative; diagnostics must show the chain.  
 **Affected files / crates:** `docs/planner-contracts.md`, `docs/spec-drafting-rules.md`, `worldwake-ai` module docs/tests.  
 **Migration strategy:** Update docs/contracts after Proposals 1–3 produce concrete types, not before.  
 **Proof strategy:** Static gate for `RequiredActionLeaf` use; trace assertion that every selected HTN method is a hint unless leaf proof exists.  
 **Risks:** Over-documenting without enforcement. Tie docs to tests/static gates.  
 **What this makes impossible:** Formalism drift where HTN/ranking/repair become privileged command channels.  
 **What this preserves:** Current architecture’s best qualities.  
 **What this replaces or removes:** Ambiguous “HTN selected it, therefore it is lawful” thinking.  
 **Why now:** The architecture is good enough to preserve, but only with hard boundaries.

## **8. Alternatives Considered**

| Alternative | Benefits | Costs / risks | Proof burden | Recommendation |
| ----- | ----- | ----- | ----- | ----- |
| Current architecture with targeted hardening | Lowest churn; keeps existing modules; easy to fix obvious accessors | Not enough. Ambient `&World` remains easy to misuse; weak gates can recur | Unit tests plus grep gates, but fragile | **Reject as sufficient.** Use only as first patch inside the moderate split. |
| Moderate object-capability split | Makes illegal flows harder to express; preserves current GOAP/HTN/ranking/repair; fixes AI and player surfaces together | Medium refactor; source wrappers/provenance ergonomics | Unit, integration, golden, trace, static gates | **Adopt. This is the best iteration-one path.** |
| Aggressive AI/world boundary redesign | Strongest long-term separation; `worldwake-ai` cannot receive `World` except allowlisted orchestration/debug/test boundaries | High churn; risks breaking useful trace/orchestration code; may stall feature work | Compile-fail gates, whole-pipeline migration, broad goldens | **Defer.** Make it the fallback if moderate gates fail or leaks recur. |
| Larger formalism shift | Could simplify one layer if the current hybrid were incoherent | Current hybrid is not the problem; pure BT/HTN/GOAP/POMDP/BDI would lose strengths or add wrong assumptions | Very high; would need to re-prove everything | **Reject for iteration one.** Preserve hybrid; clarify responsibilities. |

## **9. Proposed First Iteration Scope**

### **Do first**

1. **Define source-scoped POV types.** Add or refactor toward `CharacterPovView` / `PlannerPovView` with explicit source classes matching planner contracts: self, same-tick local physical, possession, belief/memory/evidence, public topology.  
2. **Replace rights/control accessors.** `believed_rights` and `can_control` must stop calling authoritative `world.effective_rights` / `world.can_exercise_control` from planner/player-facing views.  
3. **Add field provenance for high-risk snapshot fields.** Start with control, owner, container, possessor, seller/controller, stock, queue, faction, office/reward, merchandise, production/facility policy.  
4. **Move CLI and AI affordances to the same lawful POV.** Normal action lists become character-POV; debug/visualizer gets explicit debug-only capability.  
5. **Create the proof suite before expanding features.**

### **Defer**

* Full aggressive `worldwake-ai` no-`World` redesign.  
* New learning mechanics beyond making existing profile/memory paths concrete and covered.  
* Required HTN action leaves.  
* Behavior tree execution layer.  
* Major performance rewrites until provenance telemetry exists.

### **Do not touch**

* No new gameplay mechanics as a substitute for architecture proof.  
* No LLM/RL/runtime manager.  
* No hidden compatibility shim that keeps old authoritative view behavior alive.  
* No tests that expect remote omniscient truth unless a lawful carrier is present.

### **Likely ticket/spec breakdown**

1. `POV-001`: Source-scoped POV type and source class taxonomy.  
2. `POV-002`: Belief-backed control/rights/social fact accessors.  
3. `POV-003`: Replace weak known-entity live dynamic fields.  
4. `SNAP-001`: `PlannerField<T>` and high-risk field provenance.  
5. `AFF-001`: AI/CLI affordance unification.  
6. `TRACE-001`: Field-source and affordance-legality traces.  
7. `PROOF-001`: False/stale/contradicted belief goldens.  
8. `STATIC-001`: CI/static gates for forbidden authority access.  
9. `COVER-001`: Scenario coverage updates for cognitive/profile state.

### **Acceptance criteria**

* Remote authoritative control/right changes do not alter AI or player action visibility until a lawful belief/evidence carrier arrives.  
* Every high-risk planner snapshot field is either absent/unknown or has field provenance.  
* CLI and AI affordance surfaces match under identical character beliefs.  
* False/stale beliefs can cause attempted plans; dispatch can reject them; the trace records discrepancy and source update.  
* Static gates fail if planner/player-facing code calls authoritative `World` control/right APIs.  
* Scenario coverage no longer flags key cognitive/profile proof gaps introduced by this iteration.

## **10. Proof Matrix**

| Invariant | Test type | Files/modules | Expected trace/provenance | Golden? | Static gate? | Failure smell |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| Remote control right changes do not affect planning/player visibility until carrier arrives | Unit + integration | `per_agent_belief_view`, `affordance_query`, CLI, planning snapshot | `can_control = Unknown/Absent`, no control field provenance | Yes | Yes | Action appears because world right changed |
| False control belief can lead to attempted plan and authoritative rejection | Integration + golden | AI planning, systems dispatch, failure handling | belief ref, dispatch denial, discrepancy, source reliability update | Yes | No | Planner silently corrects control truth |
| Remote item moved/stolen does not update custody belief magically | Unit + golden | container/possessor accessors, snapshot, repair | last-seen provenance remains stale; no current possessor unless observed | Yes | Yes | Snapshot shows new possessor without carrier |
| Seller/controller/stock/queue/faction/office facts require lawful source | Unit matrix | belief view accessors, snapshot fields | field-specific source or absent | Yes for seller/stock | Yes | Belief-known entity exposes current social/economic fact |
| Same-tick local physical observation exposes physical facts but not ownership/rights/social facts | Unit + integration | POV view, affordance query | local physical source only for physical fields; social fields unknown unless carrier | Yes | Yes | Co-location reveals ownership/control |
| `BeliefLastSeen` does not expose current remote dynamic fields | Unit | snapshot admission, belief view | `BeliefLastSeen` entity admission; dynamic fields stale/absent with belief ref | Yes | Yes | Last-seen entity gets current stock/container/control |
| CLI/player action list and AI affordance surface match | Integration | CLI actions, AI tick, `get_affordances` | same affordance legality trace | Yes | No | Player sees action AI cannot or vice versa |
| Debug/visualizer omniscience cannot feed normal surfaces | Static + integration | visualizer snapshot, CLI, sim POV | `DebugOmniscientView` marker; no normal POV conversion | No | Yes | Debug snapshot implements runtime belief trait |
| Every dynamic planning snapshot field has source/provenance or is absent | Snapshot invariant test | `planning_snapshot`, `planning_state`, trace | `PlannerField<T>` with source class/ref | No | Yes | Plain dynamic `Option<T>` reappears |
| HTN methods remain lawful `StageHint` unless enforced leaves have proof | Unit + trace | HTN registry/selector/search | method trace marks hint; required leaf has action proof | No | Yes | HTN bypasses search/affordance legality |
| Repair/replan does not silently correct remote truth | Golden + trace assertion | revalidation, repair, failure handling | repair evidence source; absent new carrier => typed barrier/verification | Yes | No | Repair binds to current truth without observation |
| Learned habits/preferences/source reliability updates have origin/scope/decay | Unit + scenario | core memory/profile, ranking, diagnostics | update event with origin/scope/tick/decay | Yes | No | Preference changes with no recorded cause |
| Candidate/ranking/portfolio avoid abstract score soup | Trace assertion | candidate generation, ranking, portfolio | motive source contributions, source reliability, concrete risks | No | No | Selected goal has score but no concrete drivers |
| Performance optimizations preserve causal equivalence | Soak + property/golden | snapshot/state caches, perf telemetry | cache counters plus same selected plans under same lawful inputs | No | No | Cache changes behavior or hides missing provenance |

## **11. Invalid Tests / Dangerous Existing Expectations**

I did not verify a current-main focused test that is outright invalid due to omniscient expectations. The dangerous current-main expectations I did verify are architectural/code expectations, not necessarily test assertions.

| Current evidence | Conflict | Action |
| ----- | ----- | ----- |
| `PlanningSnapshot` and `SnapshotAdmissionTrace` record entity-level admission, while docs say entity admission is not field admission. | Encourages reviewers/tests to treat “entity known” as “field lawful.” | Replace with per-field provenance tests. |
| CLI action/label path uses `PerAgentBeliefView::with_runtime_from_world` and labels from mixed live/belief data. | Normal player POV can inherit AI belief leaks. | Rewrite tests around shared lawful `CharacterPovView`. |
| Visualizer builds omniscient world snapshot without an explicit debug-only type boundary. | Debug omniscience is separated by convention, not mechanically. | Add debug capability marker/static gate. |
| Scenario coverage flags cognitive archetypes absent and multiple behavior-affecting scenario fields unmapped. | Learning/diversity state may exist without canonical proof. | Add coverage mappings and proof scenarios. |
| HTN registry tests correctly assert StageHint-only methods. | Not invalid, but dangerous if reports/docs imply HTN is enforcement. | Keep the test; add a static gate for future `RequiredActionLeaf`. |

## **12. Implementation Risk and Migration Order**

1. **Patch the worst leak first:** change `ControlBeliefView` so planner/player-facing `believed_rights` and `can_control` cannot call authoritative world rights/control. This will break fewer things than a full POV rewrite and immediately removes the highest-risk contradiction.  
2. **Introduce source-scoped accessors for social/dynamic facts:** seller/controller, stock policy, queue policy, faction/office/reward, container/possessor, merchandise, production/facility policy.  
3. **Add provenance wrappers to high-risk snapshot fields:** do not attempt all fields at once. Start where current leaks are verified.  
4. **Move affordance generation to lawful POV:** AI and CLI together. This preserves player/AI symmetry and avoids split behavior.  
5. **Add trace proof:** field-source, affordance legality, denial, replan/repair evidence.  
6. **Add static gates:** forbid planner/player-facing modules from calling authoritative rights/control APIs; forbid debug snapshots from implementing normal belief traits; require provenance for dynamic snapshot fields.  
7. **Add goldens:** false control belief, stale custody, stock/seller carrier, local physical vs social fact, CLI/AI symmetry.  
8. **Only then expand learning/diversity:** map existing components in scenario coverage, then add concrete habit/source-reliability proof.  
9. **Stop after iteration one when the boundary is mechanically safe:** do not drift into new mechanics or formalism rewrite.

This order removes the most dangerous authority leak first, keeps goldens meaningful, preserves decision trace debuggability, and avoids a hidden compatibility layer.

## **13. Final Recommendation**

Adopt the **moderate object-capability split** now. Preserve the current GOAP/HTN/utility/BDI-ish hybrid. Do not replace it.

The current architecture is not fundamentally wrong. Its decision cycle is unusually well-instrumented: candidates, motive provenance, portfolio slots, strategic/tactical search, HTN hints, plan continuation, revalidation, repair, failure memory, discrepancy handling, source reliability, traces, diagnostics, and telemetry are all worth keeping. The defect is that the most important boundary — lawful planner/player-visible knowledge — is still too easy to violate because the belief view carries authoritative world power.

Aggressive AI/world boundary redesign should remain the long-term pressure valve, not iteration-one default. If the moderate split cannot prevent recurring leaks, then move to the aggressive rule: `worldwake-ai` cannot receive authoritative `World` except in tightly allowlisted orchestration/debug/test boundaries. But do not pay that migration cost before proving the smaller split insufficient.

### **Top 3 actions**

1. Remove authoritative rights/control access from planner/player-facing belief views.  
2. Add per-field provenance for high-risk planning snapshot fields.  
3. Unify CLI/player and AI affordances through the same lawful `CharacterPovView`.

### **Top 3 risks**

1. Tests or goldens that secretly expected omniscient convenience will fail.  
2. Provenance wrappers can bloat traces/snapshots if not compacted.  
3. Developers may try to preserve old behavior through compatibility shims; that would keep the contradiction alive.

### **Top 3 proof gates**

1. Static gate: planner/player-facing code cannot call authoritative `effective_rights` / `can_exercise_control`.  
2. Snapshot invariant: every dynamic planner-visible field has source/provenance or is absent.  
3. Golden: false/stale belief causes lawful attempted plan, authoritative rejection or verification, and recorded discrepancy — never silent remote truth correction.

### **Top 3 things not to do**

1. Do not replace the current hybrid with LLM agents, RL, pure behavior trees, pure HTN, or full POMDP planning.  
2. Do not add new gameplay content to paper over architecture proof gaps.  
3. Do not allow “temporary” compatibility paths that keep ambient world authority inside normal POV surfaces.

