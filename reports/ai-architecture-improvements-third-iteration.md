# **Worldwake AI Architecture Improvement Proposal — Next Iteration**

## **1. Repository Grounding**

**Repository:** `joeloverbeck/worldwake`  
 **Default branch:** `main`  
 **Current main SHA:** `de0992f351108ed9757a646add842c1bd8adf997`  
 **Manifest status:** usable as current-main tree inventory. The uploaded manifest states the current file inventory and includes the active docs, AI crate, sim/core/systems surfaces, generated golden docs, scenarios, and workflows.  
 **Does current main match user-supplied `de0992f`?** Yes. The Git app comparison showed live `main` identical to `de0992f351108ed9757a646add842c1bd8adf997`.  
 **Tool limitation:** the Git app exposed repository metadata, branch comparison, and targeted file fetches, but I did not rely on a Git-app recursive tree endpoint. After SHA verification, I used the uploaded manifest as the tree manifest, as requested.

The active implementation order confirms the post-S168 state: S165, S166, S167, and S168 are completed and archived; S60–S66 remain active gameplay/world-dynamics specs outside this AI-architecture iteration.

## **2. Freshness / Anti-Duplication Method**

I followed the requested pipeline: **repo metadata → current branch SHA → uploaded manifest → targeted current-main fetches → analysis**. I did not clone the repository, and I did not use GitHub code search or snippet-based repository search as evidence.

Material current-main files fetched directly from `de0992f351108ed9757a646add842c1bd8adf997` included:

Governance and contracts: `docs/FOUNDATIONS.md`, `AGENTS.md`, `CLAUDE.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/planner-contracts.md`, `docs/spec-drafting-rules.md`, `docs/scenario-roadmap.md`, generated scenario/golden inventories, current triage docs, and the current second-iteration AI architecture report.

Audit guidance: the GOAP architecture, architectural debt, golden gap, scenario analysis, and traceability-retrospective skills. Their guidance pushes toward live-code grounding, causal proof, and “do nothing” unless current evidence justifies a ticket.

AI surfaces: `lib.rs`, `agenda_manager.rs`, `candidate_generation.rs`, `agent_tick/*`, `decision_trace.rs`, `effect_schema_index.rs`, `failure_handling.rs`, `plan_repair.rs`, `partial_plan.rs`, `partial_plan_revalidation.rs`, `search/mod.rs`, `htn/*`, `opportunity_compiler/*`, and related planning code.

Belief/action/world surfaces: `belief_view.rs`, `per_agent_belief_view.rs`, `action_def.rs`, learned memories, route preference, testimony reliability, and epistemic/record/search actions.

I treated `archive/*` as historical by default and did not use archived files to override current-main code or active current-main docs. I also did not re-report S162–S168 as open work. The code now shows those waves have materially landed: belief-view locality gates are explicit, `InsertVerification` is not a dead enum by itself, opportunity compiler source fidelity is implemented, cognitive archetype proof exists in generated coverage, and partial-plan skeleton reuse is implemented as revalidated search seeding rather than forced continuation.

Rejected suspicion categories:

| Suspicion | Current-main result |
| ----- | ----- |
| S165 `InsertVerification` still dead | Rejected. It works when a lawful `RepairPlanCandidate` is supplied; the remaining seam is candidate-provider breadth. |
| S166 opportunity source status fake/static | Rejected. `Opportunity.source_belief` carries status and compiler tests cover real belief statuses. |
| S167 cognitive archetype proof absent | Rejected. Generated coverage now includes `cognitive-archetypes-divergence`; implementation order marks S167 completed. |
| S168 skeleton reuse is an unvalidated rail | Rejected. Skeleton steps are filtered, revalidated, and only used as search preferences. |
| HTN methods secretly authoritative | Rejected for current main. Current methods are all `StageHint`; planner contracts say `RequiredActionLeaf` needs future proof before authority. |

## **3. Executive Verdict**

The current AI architecture is **much closer to “good enough” than it was before S165–S168**, but it should not stop AI-architecture cycles yet.

The next wave should not be a radical redesign. It should preserve the current **GOAP / ranking / HTN-hint / BDI-ish intention / utility-profile hybrid** and add one missing capability layer: a **generalized lawful verification substrate**. After that, the highest leverage work is proof tightening: diagnostics and goldens must prove causal reason, not just plausible outcomes.

Bluntly:

**Do first:** unify lawful verification candidate production across `AskWitness`, `ConsultRecord`, `SearchPlace`, and future local inspection-style actions.

**Do second:** turn decision traces and scenario diagnostics into stronger proof surfaces.

**Do third:** normalize learning/habit/source-reliability lifecycle proof so FND-22A is mechanically checkable.

**Do not do:** replace the architecture with runtime LLM agents, RL, a global manager AI, a behavior-tree-only stack, or method-required HTN leaves without trace proof.

**Preserve the hybrid.** The current shape is constitutionally sound: belief-gated planning, local affordances, concrete action definitions, bounded search, revisable intentions, traceable decisions, and deterministic proof surfaces are all present. The remaining gaps are seams and proof contracts, not evidence that the whole architecture is wrong.

## **4. Current Architecture Map**

### **Crate responsibilities**

`worldwake-core` owns persistent world state, identity, beliefs, intentions, learned memory, route/testimony learning, event payloads, and world transactions. `worldwake-sim` owns action definitions, scheduling/runtime surfaces, belief views, affordance queries, save/load, and action traces. `worldwake-systems` owns authoritative action handlers and state mutations. `worldwake-ai` owns candidate generation, ranking, planning snapshots/state, GOAP search, HTN guidance, agenda/intention runtime, repair/revalidation, partial plans, traces, diagnostics, and AI runtime persistence. `worldwake-cli` and `worldwake-visualizer` are player/debug surfaces, with docs explicitly warning against privileged player/AI truth paths.

### **Belief view surfaces**

`PerAgentBeliefView` is the key boundary. Same-tick co-located physical observation is allowed for directly perceivable physical properties, but the code explicitly says that helper must not gate social/relational knowledge such as ownership, rights, or institutional claims. `believed_rights` and `can_control` require self or an explicit believed entity gate.

`BeliefValue` carries confidence, acquired tick, claimed event tick, and status. Belief status includes `Certain`, `Probable`, `Stale`, `Disputed`, and `Contradicted`, and claim projection can demote/refute/dispute claims rather than silently collapsing to true/false.

### **Candidate generation**

Candidate generation is a large explicit extractor registry, not a random bag of emitters. It has 21 named extractors, including needs, production, bounty, social, ask witness, patrol, political, search, report found, escort, exploration, expectation violation, opportunity compiler, and blocked self-care exploration. Diagnostics record offers, suppressions, omissions, evidence, sources, extractor sources, blocked desires, and belief-filter counts.

The important post-S168 state is that candidate generation and opportunity compilation now interact through typed opportunities and source attribution, not stale snippets or parallel untraceable routes. The `OpportunityCompiler` extractor only emits if no existing emitter produced that same goal, and redundant compiler candidates are removed when an emitter already owns the same goal.

### **Opportunity compiler**

Compiled opportunities carry the opportunity key, perceived tick, source belief, possible effects, possible information, required actions, legal status, social exposure, risks, and salience. The compiler derives source-belief status from live belief claims and uses the effect-schema index to populate required actions.

The compiler is real and useful, but still narrow: its main candidate-consumer path is `AcquireCommodity`. This is a consolidation seam, not dead code.

### **Ranking, portfolio, and motive slots**

The read phase ranks candidates using utility, memories, testimony reliability, repair memory, learned opportunity memory, and source reliability failures. Planning then builds a portfolio, applies feasibility probes, derives an operating mode, and uses portfolio weights mostly for cap/probe/diagnostic discipline. Crucially, final search order still follows ranked candidate order, not arbitrary slot category priority.

This is not pure “score soup.” Numeric scores exist, but they are attached to motive sources, source reliability, feasibility, and traceable provenance. The proof gap is not the existence of scores; it is ensuring every score-affecting discount remains explainable.

### **GOAP tactical search and strategic search**

The search layer distinguishes unsupported, found, budget-exhausted, and frontier-exhausted outcomes. It uses strategic planning metadata, tactical goals, relevant action definitions, root/expansion candidate traces, commodity relevance filters, travel pruning, travel caps, landmarks, FF-style heuristic support, beam pruning, and budget limits.

The architecture is still fundamentally GOAP-like: goals and actions are represented by preconditions/effects and searched through bounded state-space planning. It is not scripted story logic.

### **HTN / method guidance**

HTN schemas exist as method declarations with belief preconditions, subgoals, motive bias, and budget hints. Current method builders map subgoals to `StageHint`. `RequiredActionLeaf` is declared but not used by live methods. The planner contract explicitly says current HTN methods are guidance, not authority, unless future proof establishes method-required leaves.

This is the right boundary for now.

### **Planning snapshot/state**

Planning snapshots are built from actor belief view, candidate evidence entities/places, blocked facility uses, route preferences, relevant op kinds, and a travel horizon. Snapshot admission traces record why an entity was admitted. Cache counters track snapshot and planning-state cache behavior.

This aligns strongly with FND-12: performance compresses computation, not causality.

### **Plan guards, revalidation, repair**

Actions define preconditions, duration, costs, reservations, commit conditions, visibility, binding strictness, guard templates, expectation templates, and effect schema.

Plan repair now has typed repair attempts: rebind target, replace provider, insert verification, downgrade to typed barrier, abandon. `InsertVerification` is functional when the current context supplies a matching lawful repair candidate; otherwise it fails as `NoEpistemicSubstrate`.

That means S165 landed, but the next seam is clear: lawful verification candidates are still too ad hoc.

### **Partial plans / skeleton reuse**

Partial-plan segments preserve terminal barrier type, barrier fact, resume/abandon conditions, causal links, and optional remaining skeleton. Skeleton filtering excludes dangerous or too-specific steps, and skeleton revalidation rejects stale, contradicted, unknown, or unsupported belief predicates.

Search uses skeletons only as preferred successor hints; it does not force the old tail.

### **Action affordance and dispatch**

Action definitions encode legality in concrete fields: actor constraints, target specs, preconditions, reservations, durations, costs, interruptibility, commit conditions, visibility, payload, binding strictness, guard/expectation templates, and effect schema.

`AskWitness`, `ConsultRecord`, and `SearchPlace` are real lawful information-gathering actions with same-place visibility, payload validators, duration, and authoritative validation.

### **Learning, diversity, and habits**

Learning state exists and is concrete. `LearnedOpportunityMemory` records opportunity, observed tick, expiry tick, and observed-at place, with capacity enforcement. `RoutePreference` records safe/dangerous traversals, ticks, a dangerous traversal event, and decays toward neutral. `TestimonyReliability` records confirmations/refutations/stale/contradictions by source/topic and stores a provenance event ring.

This is promising, but uneven. Testimony reliability has stronger provenance than learned opportunities; route preference has event provenance for dangerous traversals but not safe traversals.

### **Traces, diagnostics, goldens, CI proof**

Decision traces now include compiled opportunities, compiler load, snapshot admissions, cache counters, repair attempts, partial-plan resume traces, and causal-link cap hits.

Generated proof inventories are substantial: 59 scenario source files, 292 golden tests, 224 scenario blocks, and 220/224 annotated blocks in the golden coverage matrix.

The weak point is not absence of proof infrastructure. The weak point is that structural coverage and plausible behavior can still outrun causal proof.

## **5. FOUNDATIONS Alignment Matrix**

| Area | Alignment | Current-main evidence | Verdict |
| ----- | ----- | ----- | ----- |
| FND-1 local causality | Strong | Actions, belief views, local physical observation gates, same-place epistemic actions. | Preserve. |
| FND-3 concrete state over abstract scores | Mostly strong | Ranking uses motive sources, source reliability, repair memory, learned memory, testimony reliability. | Needs proof that score changes are trace-backed. |
| FND-7 locality | Strong | `AskWitness`, `ConsultRecord`, `SearchPlace` require local/actor-place legality. | Extend, do not bypass. |
| FND-8 action preconditions/duration/cost/occupancy | Strong | `ActionDef` contains preconditions, duration, reservations, body/attention costs, interruptibility, commit conditions. | Good. |
| FND-9 scheduling/tie-breaking | Moderate/strong | Ranking/search order deterministic; generated inventories emphasize deterministic proof. | Keep deterministic tie-break tests. |
| FND-12 performance compression without causal cheating | Strong | Snapshot/cache counters, planning-state counters, caps, budget traces. | Needs performance equivalence gates when optimization changes. |
| FND-14/14A/14B belief/world separation | Strong | Social/rights knowledge blocked from local physical exception. | Do not reopen unless regression found. |
| FND-15 knowledge carriers | Strong but incomplete | Witness/record/search actions exist; social and institutional facts require carriers. | Broaden verification carriers. |
| FND-16 ignorance/stale/false/contradiction | Strong | Belief statuses and skeleton revalidation reject stale/contradicted/unknown. | Good. |
| FND-17 expectation violation | Moderate/strong | Expectation mismatch, search-place overdue expectation, source failure incidents. | Needs generalized repair linkage. |
| FND-18 records/evidence | Moderate/strong | `ConsultRecord` exists; opportunity source belief and traces exist. | Make record consultation a first-class verification provider. |
| FND-19 agent/player symmetry | Strong in contract | No privileged `Player`; same action definitions; CLI warns against debug truth. | No new issue verified. |
| FND-20 bounded reasoning | Strong | Portfolio caps, feasibility probes, GOAP budgets, HTN hints, search caps. | Preserve. |
| FND-21 revisable intentions | Strong | Agenda, suspension/resume, partial-plan skeleton reuse, patience, revalidation. | Strengthen non-AskWitness verification. |
| FND-22/22A diversity/learning | Moderate | Concrete learned memory, route preference, testimony reliability exist. | Needs uniform lifecycle/provenance contract. |
| FND-26 state-mediated systems | Strong | Planner operates through state, action defs, handlers, beliefs, event payloads. | Good. |
| FND-27 derived summaries are caches | Mostly strong | Snapshot/cache counters and generated docs are derived proof aids. | CI should guard stale generated docs. |
| FND-28 no fossils | Moderate | Live order archives completed specs; contracts forbid stale paths. | Watch `RequiredActionLeaf` as future hook. |
| FND-29/29A debuggability/history | Strong but not complete | Decision traces include repair, resume, source, snapshot, cache. | Convert traces into stronger assertions. |
| FND-31 validation/falsification | Moderate/strong | Large golden inventory; roadmap requires structural + behavior + causal proof. | Tighten causal proof gates. |

## **6. Dead / Half-Finished Architecture Audit**

### **Item A — `RequiredActionLeaf` exists but no live method uses it**

**Evidence:** `MethodSubgoalAuthority` declares `StageHint` and `RequiredActionLeaf`, but current method construction maps every method subgoal through `MethodSubgoal::stage_hint`. Planner contracts say all current methods are stage hints and that required-action authority needs stronger proof before being trusted.

**Why it matters:** It is an attractive footgun. A future spec could flip a method leaf to “required” without proving the planner actually selected, skipped, failed, or lawfully substituted the leaf.

**Classification:** valid future hook, not harmful dead code today.

**Recommendation:** quarantine with a static lint or registry validation: no `RequiredActionLeaf` allowed in live methods unless a method-required proof suite is present.

**Proof required:** one method-required golden with selected/skipped/failed leaf traces, one unit test verifying fallback legality, one negative test proving illegal method authority fails.

### **Item B — Verification repair is implemented but candidate-provider breadth is narrow**

**Evidence:** `InsertVerification` now succeeds when a matching `RepairPlanCandidate` exists and fails as `NoEpistemicSubstrate` otherwise. Current agenda information-barrier companion spawning is `AskWitness`-centric and entity-belief-centric. Existing lawful verification actions include `AskWitness`, `ConsultRecord`, and `SearchPlace`, but only `AskWitness` is visibly wired into the current companion repair path.

**Why it matters:** S165 solved the enum/repair hole, but the next constitutional need is broader lawful verification. Worldwake has real carriers for records and search expectations; the AI architecture should be able to ask for those carriers without bespoke one-off paths.

**Classification:** half-finished architecture seam, harmful only when stale/contradicted institutional/search facts need repair.

**Recommendation:** finish as generalized verification candidate production, not as another hard-coded `AskWitness` patch.

### **Item C — Opportunity compiler and candidate emitters are parallel but disciplined**

**Evidence:** `OpportunityCompiler` produces source-rich opportunities, but candidate generation still has many emitter extractors. Compiler candidates are currently mostly acquisition-oriented and are removed when a same-goal emitter already exists.

**Why it matters:** This can become a duplicated authority path if the compiler and emitters disagree about source fidelity, legal status, or required actions.

**Classification:** valid architecture seam, not currently harmful.

**Recommendation:** do not delete either side. Add a convergence contract: for any goal family the compiler covers, candidate evidence must preserve compiler source belief, effect facts, required actions, and suppression reason.

### **Item D — Structural scenario coverage warnings remain partly unresolved**

**Evidence:** generated scenario coverage still warns about unmapped fields such as `portfolio_weights_profile`, `expectation_store`, `last_seen_memory`, `social_observations`, `intention_disposition`, and `risk_weight_profile`; the roadmap says some are intentionally support fields or pending promote-or-classify decisions.

**Why it matters:** Warnings are useful only if they distinguish “unproved feature” from “intentional support field.”

**Classification:** proof-hygiene gap, not automatically architecture debt.

**Recommendation:** classify each warning as canonical feature, support field, fixture-only field, or obsolete field. Only feature rows need new goldens.

### **Item E — Pending discrepancy provenance can be weak**

**Evidence:** `apply_pending_discrepancies` records discrepancy entries with `source_event: None`.

**Why it matters:** FND-29 wants queryable causal history. A discrepancy without a source event can still be valid, but it is weaker as proof.

**Classification:** traceability weakness.

**Recommendation:** require pending discrepancy producers to supply a source event or explicit “no event because read-phase inference” reason.

## **7. Missing-Capability Analysis**

### **Capability 1 — Generalized lawful verification substrate**

**What exists:** lawful information actions (`AskWitness`, `ConsultRecord`, `SearchPlace`), repair candidates, belief statuses, partial-plan revalidation, and AskWitness-based information barrier companions.

**What cannot yet be expressed cleanly:** “This stale institutional fact can be lawfully checked by consulting this colocated record,” or “this overdue expectation can be checked by searching this place,” as first-class repair/resumption candidates with traceable provider, expected evidence, and rejection reasons.

**Needed now?** Yes. This is the highest-leverage post-S165 seam.

**AI architecture or gameplay mechanics?** AI architecture. It uses already-existing actions and belief contracts.

**Extend or redesign?** Extend current architecture with a new verification-provider layer.

### **Capability 2 — Diagnostics as proof, not merely description**

**What exists:** rich decision traces, generated scenario inventory, golden matrix, and roadmap proof criteria.

**What cannot yet be trusted enough:** scenario success may still be “looked plausible” unless tests assert the causal trace branch: candidate source, omission reason, selected method hint, repair attempt, partial resume, source failure, and lawful action visibility.

**Needed now?** Yes, because AI architecture cycles should end only when proof gates are robust.

**Extend or redesign?** Extend diagnostics harnesses and CI checks.

### **Capability 3 — Uniform learned-state lifecycle contract**

**What exists:** learned opportunity memory, route preferences, testimony reliability, and cognitive profile diversity.

**What cannot yet be uniformly inspected:** every learned/habit/preference update should expose origin, scope, overwrite policy, decay/expiry, save/load semantics, and decision effect.

**Needed now?** Medium-high. It is a stop-condition blocker for FND-22A if left inconsistent.

**Extend or redesign?** Extend current learned-state components and traces.

### **Capability 4 — Candidate/opportunity convergence contract**

**What exists:** source-rich compiler, extractor registry, candidate sources, and duplicate suppression.

**What cannot yet be guaranteed:** for compiler-covered domains, the selected candidate always carries the compiler’s source status, required action evidence, legal/risk status, and suppression history.

**Needed now?** Medium. It is less urgent than verification, because current code is disciplined.

**Extend or redesign?** Targeted hardening, not redesign.

## **8. Research Synthesis**

GOAP remains a good fit. F.E.A.R.’s GOAP design is historically notable because NPCs selected goals and planned action sequences from preconditions/effects instead of hard-coded behavior transitions; the lesson for Worldwake is to keep action-level planning and avoid reverting to authored scripts.

HTN research supports the current use of methods as domain guidance, but not as magic authority. SHOP2-style HTN planning uses methods to decompose tasks into subtasks and can improve performance by injecting domain knowledge; that maps well to Worldwake’s `StageHint` approach. The same literature also shows that primitive executable actions and method applicability must be grounded in current state, which argues against flipping `RequiredActionLeaf` without proof.

BDI research supports Worldwake’s current “intentions as commitments” direction. Rao and Georgeff frame deliberation as forming intentions, meaning plans of action the agent is committed to achieving; this supports agenda/intention persistence with explicit reconsideration rather than constant myopic replanning.

POMDP research is useful as inspiration but wrong as a wholesale replacement. POMDPs model partial observability, but exact solutions are difficult and belief-state policies introduce probabilistic machinery that would conflict with Worldwake’s deterministic, queryable, source-backed design. A better fit is practical deterministic belief-state planning with explicit observation actions and replanning, similar to work that uses deterministic cost-sensitive planning in belief space and reuses unexecuted plan structure after observations.

W3C PROV is directly relevant to planner-visible provenance. PROV’s core idea is that trust depends on entities, activities, agents, derivations, usage, generation, and responsibility; Worldwake should not import the standard wholesale, but its conceptual shape matches FND-18/FND-29 and supports a typed provenance discipline for learned memory, verification candidates, and diagnostics.

Behavior trees are useful for reactive control, but not a replacement. They are modular and reactive, and can be blended with planning, but a BT-only architecture would lose Worldwake’s current strengths: belief-backed planning snapshots, causal-link repair, GOAP search, source fidelity, and event-log proof.

Utility AI is useful only when scores are grounded. Game AI utility systems are often effective for scoring choices, but Worldwake’s constitution rejects naked probability dials and abstract score soup. The current ranking layer should stay, but every utility/ranking effect must stay tied to motive source, belief source, learned state, or lawful boundary artifact.

## **9. Ranked Architecture Proposals**

### **Proposal 1 — Generalized Lawful Verification Substrate**

**Rank:** 1  
 **Verdict:** Adopt.

**Problem verified on current main:** Verification repair now works only when a lawful replacement candidate is already supplied. Existing lawful information actions are broader than the current AskWitness/entity-belief repair path.

**Current-main evidence:** `InsertVerification` requires `RepairPlanCandidate`; `AskWitness`, `ConsultRecord`, and `SearchPlace` are real lawful actions; agenda information-barrier companions are AskWitness/entity-belief centric.

**Research support:** BDI and belief-space planning both support explicit information acquisition and revisable commitments; PROV supports source/activity/entity provenance for trust.

**FOUNDATIONS alignment:** FND-14/15/16/17/18/21/29/31.

**Design:** Add a `VerificationNeed` and `VerificationCandidateProvider` layer. Inputs should be belief-backed needs: missing/stale/contradicted entity location, institutional claim unknown/stale/conflicted, overdue expectation, source depleted locally, source absent locally, route/resource access unknown. Outputs should be typed `RepairPlanCandidate` / information-barrier companion candidates with:

* action op kind and action def;  
* lawful local target/provider;  
* expected observation or claim update;  
* source belief or record reference;  
* rejection reason if no lawful provider exists;  
* trace payload linking need → candidate → selected/rejected repair.

Initial providers:

| Provider | Needs served |
| ----- | ----- |
| `AskWitness` | entity belief, resource whereabouts, witness-known topics |
| `ConsultRecord` | institutional claim, office holder, verdict/bounty/record-backed fact |
| `SearchPlace` | overdue expectation, missing subject at expected place |
| direct same-tick local observation | only physical facts allowed by FND-14A |
| future inspection provider | only after an action exists and is fetched/proved |

**Affected files / crates:** `worldwake-ai/src/plan_repair.rs`, `agenda_manager.rs`, `candidate_generation.rs`, `decision_trace.rs`, `partial_plan.rs`, `partial_plan_revalidation.rs`, `planner_ops.rs`, tests under `worldwake-ai/tests/scenarios/*`; action providers in `worldwake-systems` should remain authoritative and unchanged unless a missing payload/validator is discovered.

**Migration strategy:** Start with provider registry producing candidates but do not change ranking. Wire provider output into existing `replacement_candidates` and information-barrier companion creation. Then add trace assertions. Only after proof, allow more goal families.

**Proof strategy:** Unit tests per provider; integration tests for `InsertVerification` using `ConsultRecord` and `SearchPlace`; golden scenario where stale institutional belief is repaired by record consultation; negative tests proving remote truth is not inserted; trace tests proving rejected verification providers are visible.

**Risks:** Fan-out, hidden omniscience, accidental social-fact same-tick leakage.

**What this makes impossible:** silent remote correction of stale facts.

**What this preserves:** existing GOAP search, action legality, belief view boundaries, S165 repair structure.

**What this replaces/removes:** ad hoc AskWitness-only verification companion logic should become a provider instance.

**Why now:** S165 fixed the repair enum. The next bottleneck is lawful substrate breadth.

---

### **Proposal 2 — Diagnostics-as-Proof Golden Contract**

**Rank:** 2  
 **Verdict:** Adopt.

**Problem verified on current main:** The proof infrastructure is rich, but generated coverage is partly structural and the golden matrix is not fully annotated. Roadmap criteria require causal proof, not just structural activation.

**Current-main evidence:** Decision traces expose compiled opportunities, snapshot admissions, repair attempts, partial-plan resumes, cache counters, and causal-link cap hits.

**Research support:** PROV’s trustworthiness model reinforces that provenance is not decoration; it is part of assessing reliability.

**FOUNDATIONS alignment:** FND-29/29A/31, plus FND-27.

**Design:** Add a “causal proof contract” layer for canonical AI goldens. A canonical AI scenario should assert at least one critical trace reason:

* candidate present/absent due to source;  
* candidate suppressed due to a named gate;  
* selected plan search provenance;  
* method hint selected/rejected/fallback;  
* verification provider selected/rejected;  
* partial-plan resume reused/fell back with per-step verdicts;  
* source reliability failure detected;  
* no forbidden player/debug/omniscient access.

Generated docs should distinguish structural activation from causal proof.

**Affected files / crates:** `docs/generated/*`, `scripts/golden_inventory.py`, `crates/worldwake-ai/tests/golden_harness/*`, scenario diagnostics harnesses, roadmap docs.

**Migration strategy:** Do not impose a global threshold immediately. Start with high-risk scenarios: plan repair, opportunity compiler, partial-plan terminals, survival-ask-consult, cognitive-archetypes-divergence, source-reliability, route-preferences.

**Proof strategy:** CI should fail for unannotated canonical AI scenario blocks and for missing trace-contract assertions in newly added AI architecture goldens.

**Risks:** brittle tests if trace shape overfits implementation internals.

**What this makes impossible:** goldens that pass merely because “someone survived” or behavior looked plausible.

**What this preserves:** current golden inventory, scenario roadmap, generated docs.

**What this replaces/removes:** structural coverage as a proxy for causal proof.

**Why now:** This is the difference between stopping AI architecture cycles honestly and carrying invisible debt into gameplay cycles.

---

### **Proposal 3 — Learned-State Lifecycle and Provenance Contract**

**Rank:** 3  
 **Verdict:** Adopt in modified form.

**Problem verified on current main:** Learned-state components exist but provenance/lifecycle richness is uneven. Testimony reliability has source/topic/event provenance; learned opportunity memory has tick/place/expiry but no event provenance; route preference has dangerous traversal event but safe traversal lacks matching event.

**Current-main evidence:** Generated scenario coverage still warns about profile/support fields, and roadmap says some warnings are intentional support fields pending classification.

**Research support:** BDI by itself does not solve learning; the architecture must provide concrete learning mechanisms. PROV suggests storing derivation/activity/source metadata for trust.

**FOUNDATIONS alignment:** FND-22/22A, FND-18, FND-29, FND-31.

**Design:** Define a small learned-state contract:

LearnedStateUpdate {

 subject_key,

 source_scope,

 update_kind,

 observed_tick,

 source_event_or_reason,

 decay_or_expiry,

 overwrite_policy,

 decision_effect_trace,

}

Do not force all components into one generic store. Keep concrete domain types, but require every learned/habit/preference field to answer the same questions.

**Affected files / crates:** `learned_opportunity_memory.rs`, `route_preference.rs`, `testimony_reliability.rs`, `agent_tick/learned_state_observation.rs`, `decision_trace.rs`, generated coverage docs.

**Migration strategy:** Start with trace/event provenance, not behavior change. Then classify scenario-coverage warnings.

**Proof strategy:** unit tests for event provenance and decay/expiry; save/load round-trip tests; golden trace assertion proving a learned value changed a future decision.

**Risks:** over-generalizing into abstract learning sludge.

**What this makes impossible:** preference changes that cannot be explained by concrete experience.

**What this preserves:** current concrete learning components.

**What this replaces/removes:** inconsistent hidden update semantics.

**Why now:** Agent diversity and learning are the last major AI-readiness area that could otherwise keep returning as architecture debt.

---

### **Proposal 4 — Candidate / Opportunity Convergence Contract**

**Rank:** 4  
 **Verdict:** Adopt narrowly.

**Problem verified on current main:** Candidate emitters and opportunity compiler coexist. They are disciplined today, but their authority boundary is not fully formalized.

**Current-main evidence:** Opportunity compiler carries source belief, effect facts, required actions, risks, legal/social status, and salience. Candidate generation records sources and removes redundant compiler candidates when emitters already own the same goal.

**Research support:** GOAP designs benefit from action/effect reuse, but source-local knowledge must remain explicit.

**FOUNDATIONS alignment:** FND-3, FND-12, FND-15, FND-27, FND-29.

**Design:** For every compiler-covered goal family:

* emitted candidate must carry compiler source belief when compiler is source;  
* duplicate suppression must trace why emitter wins;  
* required actions must derive from effect schema;  
* source status must propagate to ranking/traces;  
* compiler load must remain bounded and deterministic.

**Affected files / crates:** `opportunity_compiler/*`, `candidate_generation.rs`, `ranking.rs`, `decision_trace.rs`, `effect_schema_index.rs`.

**Migration strategy:** Keep `AcquireCommodity` as the pilot. Do not generalize to all goals until parity tests are clean.

**Proof strategy:** candidate/emitter parity tests, duplicate suppression trace tests, source-status ranking tests, effect-schema required-action tests.

**Risks:** premature unification could break working emitters.

**What this makes impossible:** two untraceable authorities for the same opportunity.

**What this preserves:** current extractor registry and compiler.

**What this replaces/removes:** implicit duplicate suppression.

**Why now:** This seam will get harder as the compiler covers more goal families.

---

### **Proposal 5 — HTN Authority Honesty Gate**

**Rank:** 5  
 **Verdict:** Defer implementation; adopt lint/proof gate.

**Problem verified on current main:** `RequiredActionLeaf` is declared but unused; current methods are stage hints.

**Current-main evidence:** Planner contracts explicitly say current HTN methods are hints and future required leaves need proof.

**Research support:** HTN methods are powerful domain knowledge, but they are authoritative decompositions only when primitive actions and applicability are grounded in current state.

**FOUNDATIONS alignment:** FND-20, FND-21, FND-29, FND-31.

**Design:** Add a registry validation that fails if any live method uses `RequiredActionLeaf` without a companion proof flag or test fixture. Keep all methods as `StageHint` for now.

**Affected files / crates:** `htn/*`, `planner-contracts.md`, HTN integration tests.

**Migration strategy:** lint first; no behavior change.

**Proof strategy:** static registry test and one negative fixture.

**Risks:** none significant.

**What this makes impossible:** accidental method-authority drift.

**What this preserves:** current HTN guidance role.

**What this replaces/removes:** ambiguous future interpretation of `RequiredActionLeaf`.

**Why now:** Cheap guardrail before someone uses the enum incorrectly.

## **10. Alternatives Considered**

| Alternative | Benefits | Costs / risks | Migration burden | Proof burden | Recommendation |
| ----- | ----- | ----- | ----- | ----- | ----- |
| Current architecture with targeted hardening | Low disruption; preserves S165–S168; ideal for HTN lint, trace gates, candidate/opportunity contracts | May leave verification too narrow | Low | Medium | Necessary but insufficient. |
| Current hybrid plus new capability layer | Adds generalized lawful verification without rewriting GOAP/ranking/HTN | Must prevent provider fan-out and omniscience | Medium | High | Best option. |
| Moderate subsystem redesign | Candidate/opportunity unification or repair/revalidation unification could simplify long-term authority | High risk of breaking working current architecture | High | Very high | Defer until provider layer exposes real pressure. |
| Radical redesign | Theoretically cleaner if current stack were unsalvageable | Would throw away belief gates, traces, goldens, deterministic planning, and landed repair/skeleton work | Extreme | Extreme | Reject. |

The recommendation is **current hybrid plus one focused new capability layer**, backed by targeted hardening and stronger proof gates.

## **11. Proposed Next Iteration Scope**

First implementation tranche:

1. **Spec A — Verification Candidate Provider Registry**  
   * Define `VerificationNeed`.  
   * Define `VerificationCandidateProvider`.  
   * Implement providers for `AskWitness`, `ConsultRecord`, and `SearchPlace`.  
   * Emit traceable rejection reasons.  
2. **Spec B — Repair / Partial-Plan Verification Integration**  
   * Feed provider candidates into `PlanRepairContext.replacement_candidates`.  
   * Replace hard-coded AskWitness information-barrier companion logic with provider-backed companion generation.  
   * Preserve current S165 behavior as one provider case.  
3. **Spec C — Verification Proof Goldens**  
   * Add one record-consult repair scenario.  
   * Add one search-place expectation repair scenario.  
   * Add negative tests for remote truth and unavailable providers.  
   * Assert repair attempts, verification anchors, selected providers, and rejection reasons.  
4. **Spec D — HTN RequiredActionLeaf Guard**  
   * Add registry validation that current live methods remain `StageHint`.  
   * Document the proof required before using required leaves.

Defer:

* gameplay specs S60–S66;  
* full candidate/opportunity unification;  
* learned-state normalization beyond initial provenance audit;  
* any radical planner rewrite.

Acceptance criteria:

* no belief/world separation regression;  
* no remote truth repair;  
* at least two verification providers beyond AskWitness are proven;  
* decision traces expose selected and rejected verification candidates;  
* existing S165–S168 goldens remain green;  
* generated docs updated.

Minimum test/golden/static gates:

* unit tests per provider;  
* integration tests for `InsertVerification` with provider candidates;  
* golden E2E for consult-record and search-place repair;  
* negative omniscience tests;  
* static HTN method-authority test;  
* save/load test if new runtime state is added.

  ## **12. Proof Matrix**

| Invariant | Test type | Files/modules | Expected trace/provenance | Golden? | Static gate? | Failure smell |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| Belief/world separation remains intact after S162–S164 | unit + integration | `per_agent_belief_view.rs`, belief tests | local physical source vs belief-store source | yes for cross-system | yes | remote social fact appears by co-location |
| Repair/revalidation/verification does not silently correct remote truth | unit + golden | `plan_repair.rs`, provider registry | selected/rejected provider, lawful local target | yes | no | belief becomes correct without action/carrier |
| Partial-plan skeleton reuse never becomes a rail | unit + trace test | `partial_plan.rs`, `partial_plan_revalidation.rs`, `search/mod.rs` | per-step verdicts, seeded ops, fallback reason | yes | no | stale skeleton step forced into plan |
| Suspended intentions do not starve urgent needs | golden + diagnostics | `agenda_manager.rs`, `goal_switching.rs` | suspension/resume/abandon trace, urgent candidate ranking | yes | no | idle/stuck window under survival pressure |
| Candidate/opportunity generation remains source-faithful | unit + trace | `candidate_generation.rs`, `opportunity_compiler/*` | candidate source, source belief status, duplicate suppression reason | yes for behavior | yes for registry | compiler/emitter disagree silently |
| HTN methods remain lawful StageHints unless proof exists | registry test | `htn/*` | method authority audit | no | yes | live RequiredActionLeaf without proof |
| Learning/habit/source reliability changes have origin/scope/decay | unit + save/load | learned memory, route preference, testimony reliability | update event/ref, scope key, expiry/decay | yes for decision effect | yes optional | preference changes with no cause |
| Ranking/portfolio stay concrete-state reasoning | unit + trace | `ranking.rs`, `agent_tick/planning.rs` | motive source contributions, discounts, feasibility | no/yes | no | bare score changes no provenance |
| Diagnostics prove causal reason | golden harness | `decision_trace.rs`, generated docs | selected candidate, suppression, repair, resume, source failure | yes | yes for docs | “survived” with no causal assertion |
| Performance optimizations preserve causal equivalence | perf + replay | snapshots, planning state, search | cache counters + same selected action | no | no | cache hit changes behavior |
| Save/load/replay preserve AI runtime meaning | integration | save/load runtime, agent tick | same frame/plan/memory/learning state after load | yes | no | loaded AI forgets intention or learned state |

  ## **13. Invalid Tests / Dangerous Existing Expectations**

I did not verify a current-main test that is outright invalid and should be deleted. The dangerous expectations are more subtle:

### **Structural coverage treated as causal proof**

**Evidence:** generated coverage is structural and still warns on several fields; roadmap says canonical proof requires structural activation, behavioral proof, and causal reason proof.

**Conflict:** structural activation alone does not satisfy FND-31.

**Action:** strengthen, not delete. Generated docs should label structural-only coverage clearly.

### **HTN `RequiredActionLeaf` treated as live authority**

**Evidence:** enum exists, but methods use `StageHint`; contracts say future required leaves need proof.

**Conflict:** tests that expect method-required execution today would encode architecture drift.

**Action:** static guard; no current method-required tests except negative validation.

### **Verification repair tests that only prove AskWitness**

**Evidence:** current lawful epistemic actions include `AskWitness`, `ConsultRecord`, and `SearchPlace`; repair candidate insertion is generic once candidates exist.

**Conflict:** AskWitness-only tests can pass while record/search verification remains architecturally absent.

**Action:** add sibling provider tests; do not delete AskWitness tests.

### **Discrepancy proof without source event**

**Evidence:** pending discrepancies can be recorded with `source_event: None`.

**Conflict:** causal history can be too weak for FND-29.

**Action:** narrow or strengthen expectations: either assert a source event or assert an explicit no-source reason.

## **14. Stop Condition For AI Architecture Cycles**

The team can stop AI-architecture improvement cycles only when these are true:

### **Required invariants**

* AI never reads remote authoritative world truth except through belief-backed views or lawful boundary artifacts.  
* Same-tick local observation is limited to physical facts, never social/institutional facts.  
* Candidate generation, opportunity compilation, ranking, planning, repair, revalidation, and partial-plan resume all carry source/provenance.  
* HTN methods remain hints unless method-required proof is mechanically present.  
* Repair and verification never create truth; they schedule lawful actions or expose explicit failure.  
* Partial plans preserve intentions without rails.  
* Learning/habit/preference updates have concrete origin, scope, expiry/decay, overwrite, save/load, and trace proof.  
* Performance caches and snapshots preserve causal equivalence.

  ### **Required proof surfaces**

* Unit tests for belief gates, provider legality, source statuses, learned-state decay, and route/testimony updates.  
* Integration tests for planning snapshots, affordance legality, repair/revalidation, save/load, and replay.  
* Golden E2E tests for verification repair, partial-plan resume, opportunity source fidelity, cognitive diversity, source reliability, and diagnostics causality.  
* Static gates for HTN authority, extractor registry completeness, generated docs freshness, and no debug-view access in AI.  
* Decision-trace assertions for selected/suppressed candidates, repair attempts, partial resume decisions, source failures, and fallback reasons.

  ### **Acceptable remaining imperfections**

* Some support fields can remain scenario-unmapped if explicitly classified as support/cache/fixture-only.  
* HTN can remain `StageHint` only.  
* Opportunity compiler can remain narrow if its covered families have source-fidelity proof.  
* Utility/ranking can remain numeric if every score path has concrete provenance.

  ### **Unacceptable remaining risks**

* Any duplicate authority path that can override belief state.  
* Any repair path that corrects truth without action.  
* Any generated golden that passes on plausible outcome without causal assertion.  
* Any learned preference that changes future behavior without inspectable origin.  
* Any cache optimization that changes selected actions.

  ### **Would current main meet the stop condition?**

No. It is close, but the generalized verification substrate and diagnostics-as-proof contract are not complete enough yet.

## **15. Implementation Risk and Migration Order**

1. **Add provider types without behavior change.** Define `VerificationNeed`, `VerificationCandidate`, and provider rejection trace. Compile only.  
2. **Implement AskWitness as provider parity.** Prove it reproduces S165 behavior.  
3. **Add ConsultRecord provider.** Target institutional claim/record-backed needs.  
4. **Add SearchPlace provider.** Target overdue expectation/search needs.  
5. **Wire providers into repair.** Keep existing downgrade/abandon behavior if no provider is lawful.  
6. **Wire providers into partial-plan information barriers.** Replace hard-coded entity-belief witness companion only after parity tests pass.  
7. **Add causal proof goldens.** Prefer two narrow scenarios over one massive scenario.  
8. **Add HTN authority static gate.**  
9. **Classify generated coverage warnings.**  
10. **Re-run generated inventories and verify docs.**

This sequence avoids breaking all planning at once, keeps S165–S168 meaningful, and gives a clear stopping point: verification providers proven, traces strengthened, no radical redesign.

## **16. Final Recommendation**

Preserve the architecture. Do not redesign the planner. The current hybrid is the right shape: GOAP search for lawful action sequences, ranking for bounded practical choice, HTN as honest guidance, BDI-ish frames for revisable commitments, and concrete belief/source state for constitutional grounding.

The next iteration should be a focused AI architecture wave: **generalized lawful verification plus proof tightening**. That is the fastest path toward honestly ending AI-architecture cycles and moving to gameplay-mechanics improvement later.

### **Top 3 actions**

1. Build the generalized lawful verification provider registry.  
2. Add causal trace/golden assertions for verification, repair, partial-plan resume, and source failure.  
3. Add learned-state lifecycle/provenance contracts after verification lands.

   ### **Top 3 risks**

1. Accidentally introducing remote truth through verification providers.  
2. Letting structural scenario coverage masquerade as causal proof.  
3. Prematurely turning HTN methods into authority before trace proof exists.

   ### **Top 3 proof gates**

1. Negative omniscience tests for every verification provider.  
2. Golden traces proving selected/rejected verification candidates and repair outcomes.  
3. Static guard preventing live `RequiredActionLeaf` without method-required proof.

   ### **Top 3 things not to do**

1. Do not open S60–S66 in this cycle.  
2. Do not replace the hybrid with LLM agents, RL, global manager AI, or behavior-tree-only control.  
3. Do not “fix” verification by directly correcting beliefs; schedule lawful actions or expose failure.  
1. 

