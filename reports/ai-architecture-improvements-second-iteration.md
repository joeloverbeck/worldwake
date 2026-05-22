# **Worldwake AI Architecture Improvement Proposal — Iteration 1, Redone Against Current `main`**

## **1. Repository Grounding**

* **Repository:** `joeloverbeck/worldwake`  
* **Default branch:** `main`  
* **Current `main` ref / commit SHA:** `1281324690fc4c15b73aca8d8b762b9d515e20b7`  
* **Commit identity:** merge of PR #125, “Implement s164 belief view kind source gate.”  
* **Important limitation:** the Git app exposed repo metadata, branch/commit SHA, commit fetch, blob/file fetches, and compare metadata, but I could not obtain a raw recursive tree manifest through the available Git tool surface. I did **not** use code search. I used the live SHA plus current-main authority docs and module roots as the path manifest, then fetched files directly from that SHA.

The most important current-main grounding fact is that `specs/IMPLEMENTATION-ORDER.md` says the prior belief-boundary consolidation findings were triaged and completed: S162 fixed source-gate hardening, S163 fixed CLI player POV, and S164 fixed the residual remote `entity_kind` / last-seen-kind leak. It also explicitly says the capability-trait split and per-field `SnapshotFieldSource` typing were rejected as re-litigation, not active work.

## **2. Freshness / Anti-Duplication Method**

I did not use code search snippets for this redo. The pipeline was:

1. **Repo metadata:** fetched repository metadata and confirmed default branch `main`.  
2. **Branch SHA:** confirmed `main` at `1281324690fc4c15b73aca8d8b762b9d515e20b7`.  
3. **Current-main authority docs:** fetched `specs/IMPLEMENTATION-ORDER.md`, `docs/triage/2026-05-22-ai-architecture-consolidation-fourth-iteration-triage.md`, `docs/FOUNDATIONS.md`, `AGENTS.md`, `docs/planner-contracts.md`, `docs/spec-drafting-rules.md`, `docs/scenario-roadmap.md`, and `docs/generated/scenario-coverage.md` from the exact SHA.  
4. **Targeted code fetches:** fetched the material modules directly from the exact SHA: `worldwake-ai/src/lib.rs`, `agent_tick/observation.rs`, `search/mod.rs`, `agenda_manager.rs`, `partial_plan.rs`, `plan_repair.rs`, `effect_schema_index.rs`, `htn/registry.rs`, `planner_duration_contract.rs`, `worldwake-sim/src/per_agent_belief_view.rs`, `worldwake-core/src/cognitive_profile.rs`, `worldwake-core/src/cognitive_archetype.rs`, and scenario loader/types files.

### **Anti-duplication findings**

The following items from my previous answer are **not actionable current-main recommendations** and are omitted as defects:

* **Do not propose a capability-trait split of `RuntimeBeliefView` / `PerAgentBeliefView`.** Current-main triage says that was rejected as Option-C churn; `worldwake-sim` is the lawful observation/dispatch layer allowed to hold `World`.  
* **Do not propose per-field `SnapshotFieldSource` typing as first-iteration work.** Current-main triage says `planning_snapshot.rs` has zero direct `world.` reads and snapshot lawfulness is enforced through the belief-view boundary once the view is lawful.  
* **Do not report `believed_rights` / `can_control` as a current defect.** Current-main triage says S162 deliberately permits the live authoritative read behind a self/belief-accessibility gate; current code matches that design.  
* **Do not report the remote `entity_kind` leak.** S164 landed it: remote kind now comes from stored belief or last-seen `observed_kind`, and bandit faction policy accessors are gated to known factions.

The proposals below are therefore deliberately aimed at **post-consolidation improvement**, not repeating consolidation tickets.

## **3. Executive Verdict**

Worldwake should **preserve the current GOAP / HTN StageHint / utility ranking / BDI-ish intention hybrid**. The architecture is no longer in the “fix the obvious belief leak first” state. The first improvement iteration should shift to **epistemic robustness, opportunity unification, intention continuity, concrete diversity/learning proof, and diagnostics/golden leverage**.

Bluntly: do **not** spend this iteration re-opening S162/S163/S164. The live repo has already made and documented those decisions. The best next work is to make agents better at acting under stale/false/partial belief without cheating, and to make those changes provable.

What should be done first:

1. Implement a real **epistemic verification repair path** instead of leaving `RepairKind::InsertVerification` as a permanently failing placeholder.  
2. Promote the **opportunity compiler** from narrow known-inventory support into the canonical typed substrate for opportunity-driven candidate emission.  
3. Turn **cognitive archetypes / learning / profile variation** into canonical scenario proof, because the code exists but generated scenario coverage still shows no active “Cognitive archetypes” feature.

What should explicitly not be done yet:

* no LLM runtime agents;  
* no RL;  
* no global director AI;  
* no per-field snapshot-source re-litigation;  
* no capability-trait split re-litigation;  
* no required HTN leaves until a method-required schema has proof;  
* no new gameplay mechanics as the main deliverable.

## **4. Current Architecture Map**

### **Current authority posture**

`AGENTS.md` still states the key invariants: no `Player` type, belief-only planning, information locality, state-mediated systems, append-only event log, determinism, no backward compatibility fossils, scenario profile completeness, and the authoritative-to-AI impact rule.

`docs/planner-contracts.md` now explicitly defines the live planner contract: root terminal surfacing, snapshot completeness, belief-backed travel costs, traceability for omitted operators, same-goal continuation provenance, and HTN method fallback/rejection. It also states that planner-visible fields are source-scoped through belief-view accessors, not per-field snapshot typing.

### **Belief view**

`PerAgentBeliefView` still holds `&World`, but current-main docs say this is accepted because it lives in the sim observation/dispatch layer. The view gates rights/control by self or believed-entity accessibility, and dispatch remains authoritative.

The S164 fix is present: remote `entity_kind` no longer falls back to live world truth; it uses belief-store `believed_kind` or last-seen `observed_kind`. Bandit faction policy accessors now require the faction to be one of the actor’s known bandit factions.

### **Candidate generation and opportunity compilation**

The read phase compiles opportunities, filters out the active goal, passes compiled opportunities into candidate generation, builds a perceived opportunity index only for candidates sourced from the compiler, and then ranks candidates with repair memory, learned opportunity memory, and testimony reliability.

The opportunity compiler currently emits opportunities mainly from known entity inventory, only if a `CommodityTransfer` producer exists, and assigns a compact belief reference. The compiled opportunity includes effects, information topics, required actions, believed legal status, social exposure, risks, and salience.

However, the current compiler is still narrow: it uses `required_actions: vec![PlannerOpKind::MoveCargo]` for known-inventory consumable opportunities and gives `source_belief` a hard-coded `BeliefStatusTag::Probable` rather than carrying richer belief status from the underlying claim/state.

### **Search and planning**

The planner has explicit `PlanSearchResult` outcomes: `Found`, `Unsupported`, `BudgetExhausted`, and `FrontierExhausted`. Search carries expansion summaries, budget accounting, travel-candidate caps, and strategic/tactical planning layers.

`PlannerDurationDependency` is now a live inventory over non-fixed planner durations and has a test that compares the inventory against the registered action definitions. This is exactly the kind of architectural contract the next iteration should imitate.

### **HTN**

The method registry contains 11 methods: bounty, production, restock, investigation, and escort patterns. Current tests assert all method subgoals are `StageHint`, and no method declares `RequiredActionLeaf`.

That is correct. HTN is currently search guidance and trace context, not an enforcement layer.

### **Repair, barriers, and partial plans**

`PlanRepair` is promising but incomplete. It defines repair order as `RebindTarget`, `ReplaceProvider`, `InsertVerification`, `DowngradeToTypedBarrier`, and `Abandon`. But `InsertVerification` currently always returns `RepairFailure::NoEpistemicSubstrate`.

`PartialPlanSegment` already supports suspended plans with completed prefixes, optional remaining skeletons, terminal barriers, resume/abandon conditions, causal links, and budget-exhaustion segments.

`AgendaManager` can resume partial plans and can spawn information-barrier companion `AskWitness` goals for suspended information barriers.

That means the next big improvement is not inventing new architecture. It is wiring these existing concepts into a more coherent epistemic repair and continuation system.

### **Agent diversity and learning**

Cognitive archetypes are implemented in core: there are archetype enums, profile templates, assignment policy/source, and `PersonalityAssignedPayload`.

Scenario spawn applies archetype deltas to perception, cognitive profile, portfolio weights, schema context, risk, epistemic disposition, testimony trust, and route preference, then records `PersonalityAssignedPayload`.

But generated scenario coverage still reports **Cognitive archetypes** absent in every scenario and flags portfolio weights, intention disposition, expectation store, last-seen memory, and social observations as unmapped feature fields.

### **Scenario proof**

The roadmap says scenario-backed goldens are canonical only when structural activation, behavioral proof, and proof of the authored causal reason all hold. It also says all main survival rows and final integration are landed, while broader report/witness remains structurally partial outside the landed branch.

This is the right proof philosophy. The next iteration should extend it to cognitive/learning and epistemic-repair behavior.

## **5. FOUNDATIONS Alignment Matrix**

| Area | Current alignment | Improvement need |
| ----- | ----- | ----- |
| FND-1 local causality | Stronger after S162/S164. Live docs say previous leaks were fixed or dismissed as lawful. | Do not re-open stale boundary work; focus on belief-driven behavior under uncertainty. |
| FND-3 concrete state | Strong. Opportunities, memories, profiles, archetypes, blockers, discrepancies, and partial plans are concrete state. | Opportunity compiler should carry richer concrete belief/provenance status. |
| FND-7 locality | Stronger after S164 observed-kind carrier. | Verification/repair should acquire missing knowledge by local carriers, not generic replanning. |
| FND-8 action cost/occupancy | Strong via action definitions and duration contracts. | Verification actions must remain ordinary affordances with duration/cost. |
| FND-12 performance compression | Good scaffolding via budgets, caps, telemetry, and duration contracts. | Add causal-equivalence/performance gates for opportunity compiler expansion and partial-plan reuse. |
| FND-14/14A/14B belief/world | Current-main says consolidation is complete enough to leave re-litigation behind. | Maintain regression gates; do not propose rejected per-field/capability split. |
| FND-15 knowledge carriers | Strong substrate. Last-seen `observed_kind` is now a carrier. | Verification repair should explicitly create or seek carriers. |
| FND-16 stale/false/contradictory belief | Substrate exists through beliefs, discrepancies, repair, and barriers. | `InsertVerification` is the obvious missing execution path. |
| FND-17 expectation violation | Strong. Mismatches, blockers, discrepancies, source failures exist. | Use them to generate verification/continuation rather than always downgrade or replan. |
| FND-18 records/evidence | Strong. Memories, events, records, traces exist. | Opportunity compiler and repair should preserve belief status and source identity more faithfully. |
| FND-19 player/AI symmetry | S163 is completed and archived per active implementation order. | Add future UI proof only when a real player UI exists. |
| FND-20 bounded reasoning | Strong. GOAP, budgets, caps, partial plans, and HTN hints exist. | Use partial-plan skeletons to preserve useful bounded work after barriers. |
| FND-21 revisable intentions | Strong but underexploited. Partial plans and agenda resume exist. | Make barrier/verification continuation the first-class next improvement. |
| FND-22/22A diversity/learning | Code exists; proof is weaker. | Promote archetype/learning proof into canonical scenario coverage. |
| FND-26 state-mediated systems | Strong. Current architecture avoids direct system-to-system commands. | Keep opportunity/repair changes state-mediated. |
| FND-27 caches not truth | Stronger after live triage; snapshot-through-view invariant is accepted. | Do not add new cached opportunity truth without source/provenance. |
| FND-28 no fossils | Good. S164 bumped save format rather than preserving compatibility shim per commit evidence. | Avoid resurrecting rejected old proposals. |
| FND-29/29A debuggability | Strong traces and scenario diagnostics exist. | Add trace proof for verification repair, opportunity compiler source use, and archetype-driven divergence. |

## **6. Research Synthesis**

GOAP remains the right tactical core. The F.E.A.R. example is still instructive: GOAP lets agents select goals and compute action dependencies at runtime instead of hard-coding transitions; the useful lesson for Worldwake is compositional action search over local knowledge, not omniscient optimality.

BDI remains the right intention lens. The BDI model separates plan selection from execution of active plans, and Rao/Georgeff’s framing treats intentions as plans the agent is committed to achieving. That maps well to Worldwake’s agenda, current plan, partial plan, and revalidation machinery.

HTN remains useful as method guidance, not as proof of legality. HTN research shows decomposition and preferences can guide planning, but Worldwake’s current StageHint-only stance is correct until ordinary affordance/action proof exists for required leaves.

Utility AI is useful only while traceable. Utility systems score actions or behaviors from current context, but Worldwake must keep motive contributions, source reliability, concrete risk, and belief provenance visible so “utility” does not become abstract score soup.

Behavior trees should not replace the planner. BTs are modular and reactive, but the survey literature frames them mainly as behavior-organization structures, not as a solution to partial-observation provenance, source-carrying knowledge, or deliberative repair.

POMDPs are useful as a caution, not a replacement. POMDPs model partial observability, but exact synthesis is generally computationally intractable; Worldwake should borrow belief-state discipline while keeping deterministic, bounded, inspectable planning.

PROV/data-lineage ideas remain relevant for the opportunity compiler and repair traces. PROV emphasizes entities, activities, agents, derivations, usage, and attribution as a way to assess reliability and trustworthiness; Worldwake should apply that pattern to opportunity and repair provenance, not necessarily to every snapshot field.

Object-capability ideas are useful only as a future UI/debug discipline, not as a current-main replacement proposal. The repo has explicitly rejected a capability-trait split for current belief views, so the relevant lesson is narrower: avoid ambient authority in future player/debug tools.

## **7. Ranked Architecture Proposals**

### **Proposal 1 — Epistemic Verification Repair**

**Rank:** 1  
 **Verdict:** Adopt.  
 **Problem verified on current main:** `RepairKind::InsertVerification` is present in the repair attempt order but always fails with `NoEpistemicSubstrate`.  
 **Current-main evidence:** `PlanRepairContext` already carries broken causal links, preserved prefix, reusable suffix, replacement candidates, new evidence, and discrepancy entry; partial plans and agenda companions already support information barriers and resumption.  
 **Research support:** BDI-style architectures separate deliberation from execution and treat intentions as persistent but revisable commitments; verification repair is exactly the missing bridge between stale belief and intention continuity.  
 **FOUNDATIONS alignment:** FND-15, FND-16, FND-17, FND-20, FND-21, FND-29.  
 **Design:** Make `InsertVerification` produce an ordinary lawful verification subplan when the broken link is a stale/contradicted/missing belief and a verification affordance exists. Examples: ask witness for entity belief, consult record, search place, inspect local source, or travel to observe. If no verification affordance exists, downgrade to the existing typed barrier path.  
 **Affected files / crates:** `worldwake-ai/src/plan_repair.rs`, `partial_plan.rs`, `agenda_manager.rs`, `plan_revalidation.rs`, `decision_trace.rs`, candidate generation for verification candidates, relevant system actions only as existing affordance consumers.  
 **Migration strategy:** Start with `BeliefStale`, `BeliefContradicted`, and `MissingObservation`. Do not add new gameplay actions. Use existing `AskWitness`, `ConsultRecord`, `SearchPlace`, and travel/observe affordances.  
 **Proof strategy:** Focused repair tests plus one golden: an agent acts on stale belief, detects mismatch, inserts verification, acquires lawful evidence, resumes or abandons with trace.  
 **Risks:** Verification can thrash if it loops; use existing repair memory, ask-witness memory, backoff, and partial-plan patience.  
 **What this makes impossible:** A stale-belief failure collapsing directly into generic replanning when a lawful verification path exists.  
 **What this preserves:** Existing GOAP, barriers, partial plan, and agenda architecture.  
 **What this replaces or removes:** The placeholder `InsertVerification => NoEpistemicSubstrate` path.  
 **Why now:** This is the highest-leverage belief-boundary improvement that is not already implemented.

### **Proposal 2 — Opportunity Compiler 2.0 as Canonical Typed Opportunity Substrate**

**Rank:** 2  
 **Verdict:** Adopt in modified form.  
 **Problem verified on current main:** The opportunity compiler exists but is narrow: it compiles known-entity inventory opportunities, requires only a `CommodityTransfer` producer, emits `MoveCargo`, and hard-codes source belief status as `Probable`.  
 **Current-main evidence:** Read phase already compiles opportunities and passes them into candidate generation; `EffectSchemaIndex` already maps action effect schemas to effect fact keys.  
 **Research support:** GOAP works best when actions/effects are explicit and composable; PROV-like provenance helps preserve why an opportunity is trusted.  
 **FOUNDATIONS alignment:** FND-3, FND-15, FND-16, FND-20, FND-27, FND-29.  
 **Design:** Make `Opportunity` the canonical typed bridge from belief/evidence to candidate generation for acquisition/economic/social opportunities. It should carry the real belief status/freshness where available, derive possible actions from `EffectSchemaIndex`, distinguish transfer/consume/produce/harvest/trade/ask/consult/search information outcomes, and avoid parallel emitter logic for the same opportunity family.  
 **Affected files / crates:** `opportunity_compiler/*`, `candidate_generation.rs`, `effect_schema_index.rs`, `decision_trace.rs`, `scenario_diagnostics`, `ranking.rs`.  
 **Migration strategy:** Do not rewrite all emitters at once. Start with `AcquireCommodity(SelfConsume)` and `RestockCommodity`, because they already straddle known inventory, trade, harvest, and source reliability.  
 **Proof strategy:** For each migrated family, assert candidate parity with old emitters, then delete duplicate emitter branches only when the compiler trace fully explains the candidate and omitted alternatives.  
 **Risks:** Overgeneralizing too early. Keep it incremental and trace-first.  
 **What this makes impossible:** Candidate generation and opportunity compilation drifting into two separate source-contract systems.  
 **What this preserves:** Current candidate/ranking/search behavior while moving source attribution earlier and making it more explicit.  
 **What this replaces or removes:** Duplicate opportunity-specific logic once parity is proven.  
 **Why now:** The compiler already exists; its current narrowness is the next obvious architecture leverage point.

### **Proposal 3 — Partial-Plan Skeletons and Barrier-Resumable Intentions**

**Rank:** 3  
 **Verdict:** Adopt.  
 **Problem verified on current main:** `PartialPlanSegment` supports `remaining_skeleton`, completed prefixes, terminal barriers, resume/abandon conditions, and causal links, but the budget-exhaustion constructor writes `remaining_skeleton: None`; resumption currently re-enters the agenda rather than preserving a richer plan skeleton.  
 **Current-main evidence:** Agenda resumption and information-barrier companions already exist. The architecture has the pieces but not the mature “resume this suspended pursuit with remembered skeleton” behavior.  
 **Research support:** BDI-style persistence is about commitment over time, not rerunning deliberation from scratch every time an assumption changes.  
 **FOUNDATIONS alignment:** FND-20, FND-21, FND-27, FND-29.  
 **Design:** When a plan reaches an information/resource/jurisdiction/coordination barrier, preserve the useful prefix and a compact remaining skeleton. Resume should validate the skeleton against fresh lawful beliefs before rebuilding full tactical details.  
 **Affected files / crates:** `partial_plan.rs`, `agenda_manager.rs`, `agent_tick/planning.rs`, `plan_repair.rs`, `decision_trace.rs`.  
 **Migration strategy:** Start with information barriers and search-budget barriers. Do not preserve skeletons for volatile combat or target identity bindings until trace proof is strong.  
 **Proof strategy:** Golden: an agent suspends on missing information, asks/consults/searches, resumes the same high-level pursuit instead of losing the intention or reselecting a rival.  
 **Risks:** Fossilized skeletons can become stale. Require resume conditions, max attempts, and revalidation.  
 **What this makes impossible:** Treating every barrier as a full intention reset when the architecture has enough state to continue.  
 **What this preserves:** Current agenda and ranking authority.  
 **What this replaces or removes:** Some wasteful full replans after typed barriers.  
 **Why now:** It improves bounded reasoning without adding new mechanics.

### **Proposal 4 — Cognitive Archetype and Learning Proof Lane**

**Rank:** 4  
 **Verdict:** Adopt.  
 **Problem verified on current main:** Cognitive archetype components/templates and scenario spawning exist, but generated scenario coverage shows “Cognitive archetypes” absent across all scenarios. It also flags behavior-affecting authored fields such as portfolio weights, intention disposition, expectation store, last-seen memory, and social observations as unmapped feature rows/support fields.  
 **Current-main evidence:** Archetype templates affect cognitive, perception, portfolio, schema context, risk, epistemic, testimony, and route preference profiles; scenario spawning records assignment payloads.  
 **Research support:** BDI by itself does not guarantee meaningful diversity or learning; those need concrete state and update paths.  
 **FOUNDATIONS alignment:** FND-22/22A, FND-20, FND-29, FND-31.  
 **Design:** Promote archetypes and concrete learning/habit state into canonical proof surfaces. The first tranche should not add new mechanics; it should prove existing archetype deltas materially change choices and traces.  
 **Affected files / crates:** scenario coverage generator, roadmap, scenario RONs, `scenario_diagnostics`, `cognitive_archetype.rs`, `scenario/mod.rs`, AI goldens.  
 **Migration strategy:** Add one canonical cognitive-archetype scenario or extend `final-integration` with explicit archetype activation. Then add focused tests showing resolved profile hash and chosen behavior differ for two agents with same role/beliefs but different archetypes.  
 **Proof strategy:** Golden must assert both behavior divergence and trace explanation: motive source, profile delta, selected plan, and rejected alternative.  
 **Risks:** Archetypes becoming abstract “personality flavor.” Keep them grounded in concrete profile deltas and assignment events.  
 **What this makes impossible:** Claiming FND-22 is satisfied only because profile types exist.  
 **What this preserves:** Existing archetype templates and scenario assignment system.  
 **What this replaces or removes:** Coverage blind spot around cognitive archetypes.  
 **Why now:** It is the cleanest post-consolidation AI improvement with a direct current-main proof gap.

### **Proposal 5 — Source-Reliability and Learning Trace Tightening**

**Rank:** 5  
 **Verdict:** Adopt.  
 **Problem verified on current main:** The opportunity compiler and ranking use learned opportunity memory and source reliability, but opportunity `source_belief` status is currently compact and hard-coded in the compiler, and scenario diagnostics aggregate reliability changes at a broad topic level.  
 **Current-main evidence:** Read phase applies pending source reliability failures after candidate generation, and ranking consumes repair/learned memory/testimony reliability.  
 **Research support:** Provenance/lineage systems are valuable because they expose origins, transformations, and trustworthiness of data.  
 **FOUNDATIONS alignment:** FND-15, FND-16, FND-17, FND-18, FND-22A, FND-29.  
 **Design:** Strengthen traceability for learned updates: source reliability changes, learned opportunity damping, route preference updates, and blocker/repair memory should include origin event, source, scope, expiry/decay, and affected candidate keys in decision traces.  
 **Affected files / crates:** `ranking.rs`, `opportunity_compiler`, `decision_trace.rs`, `scenario_diagnostics`, `experience_recording.rs`, source reliability modules.  
 **Migration strategy:** Start with acquisition source reliability because the existing pipeline already produces pending failures and discounts.  
 **Proof strategy:** Scenario where two agents receive different source histories and diverge; trace explains learned damping and source reliability discount.  
 **Risks:** Trace bloat. Use compact IDs and detail-on-demand.  
 **What this makes impossible:** “The agent learned” without inspectable update path.  
 **What this preserves:** Current learning substrates.  
 **What this replaces or removes:** Broad aggregate-only explanations for learned behavior changes.  
 **Why now:** It turns FND-22A from implementation presence into debuggable architecture.

### **Proposal 6 — Diagnostics as CI-Gated Architecture Audits**

**Rank:** 6  
 **Verdict:** Adopt.  
 **Problem verified on current main:** `ScenarioDiagnosticsReport` exists and aggregates many useful metrics, but current proof philosophy is still primarily golden-specific; diagnostics are not yet clearly a CI gate for architecture drift.  
 **Current-main evidence:** Diagnostics track archetypes, goal pressure, planning, method usage, repair, stale/contradicted belief actions, source reliability changes, route preference changes, coordination, and performance.  
 **Research support:** Deterministic/plausibility claims are weaker than auditable provenance and replayable traces; PROV/event-lineage ideas support explicit audit surfaces.  
 **FOUNDATIONS alignment:** FND-29, FND-31, FND-12.  
 **Design:** Add architecture-level diagnostic gates for long-run sweeps: maximum unexplained budget exhaustion rate, method fallback rate per goal family, repair success/failure distribution, false-rumor propagation count, source reliability change count, route preference changes, and cognitive archetype distribution.  
 **Affected files / crates:** `scenario_diagnostics`, test harnesses, `docs/scenario-roadmap.md`, generated docs.  
 **Migration strategy:** Start as non-failing reports, then promote stable invariants to CI gates.  
 **Proof strategy:** Long-run seed sweep plus replay/save-load equivalence for one canonical scenario.  
 **Risks:** Overfitting thresholds. Use bands and qualitative “failure smell” gates first.  
 **What this makes impossible:** A feature “passing” one golden while wrecking systemic AI behavior.  
 **What this preserves:** Current golden philosophy.  
 **What this replaces or removes:** Sole reliance on single-scenario assertions for cross-system behavior.  
 **Why now:** The architecture now has enough diagnostics to make them authoritative.

### **Proposal 7 — HTN Method Maturity Without Rails**

**Rank:** 7  
 **Verdict:** Defer required leaves; adopt trace maturity.  
 **Problem verified on current main:** All methods are StageHints, and tests explicitly forbid `RequiredActionLeaf`.  
 **Current-main evidence:** Planner contracts say fallback remains legal unless a future method-required schema contract proves flat fallback semantically invalid.  
 **Research support:** HTN preference/decomposition can guide search, but using methods as hard rails requires soundness proof.  
 **FOUNDATIONS alignment:** FND-20, FND-21, FND-29, FND-31.  
 **Design:** Do not add required leaves yet. Instead, improve method traces so subgoals record which strategic stages they actually generated, whether they survived to tactical search, and why fallback occurred.  
 **Affected files / crates:** `htn/selector.rs`, `search/strategic.rs`, `decision_trace.rs`, `scenario_diagnostics`.  
 **Migration strategy:** Trace-only first. Required leaves only after one method-required goal has a schema contract and proof.  
 **Proof strategy:** Focused tests for selected method, rejected method, fallback reason, and stage-to-search mapping.  
 **Risks:** Turning StageHints into de facto scripts. Keep action legality in GOAP/search.  
 **What this makes impossible:** Claiming HTN caused behavior without evidence of how it shaped search.  
 **What this preserves:** Current StageHint design.  
 **What this replaces or removes:** Ambiguous method traces that show selected subgoals but not their actual search effect.  
 **Why now:** It improves explainability without altering planner authority.

### **Proposal 8 — Performance/Causal-Equivalence Guardrails for Expanded AI**

**Rank:** 8  
 **Verdict:** Adopt as supporting work.  
 **Problem verified on current main:** Planner has budgets and caps; opportunity/compiler/repair/partial-plan expansion will increase trace and search complexity.  
 **Current-main evidence:** `CognitiveProfile` already controls candidate caps, travel caps, node expansions, repair budget fraction, learned memory TTLs, and causal-link caps.  
 **Research support:** POMDP literature is a useful warning: belief-state planning can become computationally intractable, so Worldwake needs deterministic bounded approximations rather than unbounded optimality.  
 **FOUNDATIONS alignment:** FND-12, FND-20, FND-27, FND-31.  
 **Design:** For every proposal above, add performance counters and causal-equivalence checks before broad rollout: compiler opportunity count, compiler truncation, repair insertions, verification loops, partial-plan resumes, trace size, and outcome equivalence under save/load/replay.  
 **Affected files / crates:** `perf_telemetry.rs`, `scenario_diagnostics`, `opportunity_compiler`, `plan_repair`, `partial_plan`.  
 **Migration strategy:** Instrument first, optimize second.  
 **Proof strategy:** Soak before/after with same seeds; behavior changes must be explained by new lawful reasoning, not cache drift.  
 **Risks:** Metrics without action. Tie metrics to failure smells.  
 **What this makes impossible:** Letting “better AI” become unbounded planner fan-out.  
 **What this preserves:** Current bounded practical reasoning.  
 **What this replaces or removes:** Unmeasured expansion of search/trace surfaces.  
 **Why now:** Every worthwhile improvement above risks fan-out.

## **8. Alternatives Considered**

| Alternative | Benefits | Costs / risks | Migration burden | Proof burden | Recommendation |
| ----- | ----- | ----- | ----- | ----- | ----- |
| Conservative targeted hardening | Low churn | Stale: S162/S163/S164 already did the hardening that survived triage | Low | Low | Reject as the main iteration. Keep regression tests only. |
| Moderate object-capability split | Would make information authority mechanically clearer | Current-main explicitly rejected capability-trait split as re-litigation / Option-C churn | High | High | Do not re-propose now. Revisit only after a new verified leak survives triage. |
| Aggressive AI/world boundary redesign | Strongest possible static separation | Contradicts current-main triage; likely disrupts lawful sim-layer observation architecture | Very high | Very high | Reject for iteration one. |
| Larger formalism shift | Could simplify if current hybrid were failing | Current hybrid is not failing; it has rich contracts, traces, budgets, and proof surfaces | Very high | Very high | Reject. |
| Current hybrid + epistemic/learning/opportunity improvements | Builds on live strengths; avoids stale re-litigation | Requires careful proofs to avoid trace bloat and fan-out | Medium | Medium-high | Adopt. |

## **9. Proposed First Iteration Scope**

### **First implementation tranche**

1. **Repair verification MVP**  
   * Implement `RepairKind::InsertVerification` for stale/contradicted/missing belief cases.  
   * Use only existing lawful affordances.  
   * Add repair traces showing verification candidate chosen or rejected.  
2. **Opportunity compiler acquisition migration**  
   * Make compiled acquisition opportunities carry richer source/belief status.  
   * Derive available action families through `EffectSchemaIndex`.  
   * Prove candidate parity for `AcquireCommodity(SelfConsume)` before deleting duplicate emitter logic.  
3. **Cognitive archetype proof**  
   * Promote “Cognitive archetypes” into generated scenario coverage or a canonical proof-support row.  
   * Add one golden proving two archetypes produce different decisions under the same local facts and explain the difference in traces.  
4. **Diagnostics gate draft**  
   * Add a non-failing diagnostic report for verification repair, opportunity compiler usage, partial-plan resume, and archetype distribution.  
   * Convert only stable invariants to failing gates later.

### **Defer**

* Required HTN leaves.  
* Full partial-plan skeleton preservation for every barrier type.  
* Any new player UI capability layer.  
* Any capability-trait split or per-field source typing.  
* New gameplay features.

### **Acceptance criteria**

* `InsertVerification` can produce at least one lawful verification subplan and no longer always fails.  
* A stale/false belief golden shows verification or typed barrier behavior for the right causal reason.  
* Opportunity compiler traces explain at least one acquisition candidate end-to-end.  
* Cognitive archetype behavior is structurally active and behaviorally proven in at least one scenario/golden.  
* Diagnostics expose repair, compiler, and archetype metrics.  
* No current S162/S163/S164 regression gate breaks.

## **10. Proof Matrix**

| Invariant | Test type | Files/modules | Expected trace/provenance | Golden needed | Static gate needed | Failure smell |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| S162/S164 belief boundary remains intact | Regression | `per_agent_belief_view`, S162/S164 goldens | Existing belief-wall traces remain unchanged | Yes | No | Reappearance of remote-kind/current-truth leak |
| `InsertVerification` does not always fail | Unit | `plan_repair.rs` | `RepairAttemptTrace` shows verification candidate selected or rejected by explicit reason | No | No | `NoEpistemicSubstrate` for every case |
| Verification uses ordinary lawful affordances | Integration | repair + search + affordances | selected `AskWitness` / `ConsultRecord` / `SearchPlace` / travel observe step | Yes | No | Verification appears without action/precondition/duration |
| Verification does not erase stale/false belief magically | Golden | repair, belief store, revalidation | discrepancy remains until evidence carrier updates belief | Yes | No | Belief becomes true with no carrier |
| Partial-plan barrier resumes preserve commitment | Golden | `partial_plan`, `agenda_manager` | segment id, resume condition, resumed agenda entry | Yes | No | Agent drops goal despite satisfied resume condition |
| Opportunity compiler candidate parity | Unit/integration | `opportunity_compiler`, `candidate_generation` | compiler source attribution, old/new candidate match | No initially | No | Candidate disappears without trace |
| Opportunity compiler carries truthful belief status | Unit | `opportunity_compiler` | source belief status reflects stale/probable/contradicted where available | No | No | All compiled opportunities are `Probable` |
| Effect index remains aligned to action defs | Unit | `effect_schema_index` | effect facts map to expected action defs | No | No | Compiler assumes action family not in effect schema |
| Archetype assignment is concrete and traceable | Unit + scenario | scenario spawn, core archetype | `PersonalityAssignedPayload`, resolved profile hash | Yes | No | Behavior differs with no profile/event cause |
| Same role, same belief, different archetype can diverge | Golden | scenario + AI trace | motive/profile deltas explain different selected plans | Yes | No | Archetype exists but does not affect traceable choice |
| Diagnostics report systemic AI health | Soak/report | `scenario_diagnostics` | repair, compiler, archetype, budget, stale-belief metrics | No initially | No | Golden passes but diagnostics show pathological drift |
| HTN remains StageHint-only unless proof added | Unit | `htn/registry`, planner contracts | all current subgoals StageHint | No | Existing test | Required leaf appears without schema proof |
| Expanded compiler/repair preserves bounded reasoning | Soak/perf | `perf_telemetry`, diagnostics | capped opportunity count, repair loop count, trace size | No | No | New path causes unbounded fan-out |

## **11. Invalid Tests / Dangerous Existing Expectations**

I did not verify any current-main test that should be deleted as invalid.

The dangerous expectations are documentation/proof gaps, not bad tests:

* **Stale expectation to avoid:** treating S162/S163/S164 issues as still open. Current-main says they are completed or deliberately rejected.  
* **Proof gap:** generated scenario coverage still shows “Cognitive archetypes” absent, despite archetype implementation and scenario assignment code existing.  
* **Architecture gap:** `InsertVerification` exists as a repair kind but always fails, so any test assuming verification repair exists would be premature.  
* **Trace gap:** opportunity compiler source belief is too compact for strong FND-15/FND-16 proof, especially because current compiled opportunities hard-code belief status as probable.

## **12. Implementation Risk and Migration Order**

1. **Start with tests for `InsertVerification`.** Prove the current always-fail behavior, then implement one narrow verification path.  
2. **Wire verification through existing actions.** Do not add mechanics; use current `AskWitness`, `ConsultRecord`, `SearchPlace`, and travel/observe paths.  
3. **Add repair traces before broadening behavior.** Make the explanation surface land with the behavior.  
4. **Improve opportunity compiler source fidelity.** Carry richer source/belief status before migrating more candidate families.  
5. **Migrate one acquisition family to compiler-backed emission.** Assert parity with existing candidate generation before removing duplicate branches.  
6. **Add cognitive archetype proof.** Use existing archetype assignment and profile deltas; do not add new personality systems.  
7. **Promote diagnostics to non-failing reports.** Stabilize thresholds before CI gating.  
8. **Only then consider richer partial-plan skeleton reuse.** Start with information barriers and budget barriers; avoid volatile combat paths.  
9. **Defer HTN required leaves.** Keep StageHints until trace maturity proves a method-required contract is worthwhile.

## **13. Final Recommendation**

Preserve the current hybrid architecture. The repo’s live state has already completed the belief-boundary consolidation work that survived triage, and it explicitly rejects reopening the big capability-split/per-field-source proposals. The next high-leverage architecture cycle should be about **acting better under lawful uncertainty**.

The best first tranche is:

1. **Implement epistemic verification repair.**  
2. **Promote the opportunity compiler into a richer typed candidate substrate.**  
3. **Make cognitive archetypes and learning behavior canonically proven, not merely implemented.**

Top 3 risks:

1. Reopening stale consolidation decisions instead of improving the current architecture.  
2. Expanding opportunity compilation or verification into planner fan-out.  
3. Letting archetypes become abstract flavor instead of concrete, traceable state.

Top 3 proof gates:

1. A stale/false-belief golden where verification repair happens for the right reason.  
2. Opportunity compiler parity and source-status tests for one acquisition family.  
3. Cognitive archetype golden proving traceable divergence between same-role agents.

Top 3 things not to do:

1. Do not propose the rejected capability-trait split or per-field `SnapshotFieldSource` typing again.  
2. Do not add runtime LLM/RL/global director systems.  
3. Do not add new gameplay mechanics when the real improvement is AI architecture proof and repair discipline.

