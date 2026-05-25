# **Worldwake AI Architecture Stop-Condition Audit — Post-S169/S170**

This audit follows the uploaded mission prompt and supplied manifest.

## **1. Repository Grounding**

**Repository:** `joeloverbeck/worldwake`  
 **Default branch:** `main`  
 **Current main SHA:** `e0ec83d47621227bac0d76d5f44ee192a86ec369`  
 **Manifest status:** usable as the current tree inventory. I verified live repo/default-branch metadata first, then fetched `reports/manifest_2026-05-25.txt` from the exact current SHA as the live manifest counterpart.  
 **Whether current main matches user-supplied `e0ec83d`:** yes.  
 **Whether S169/S170 are completed on current main:** yes. `specs/IMPLEMENTATION-ORDER.md` marks S169 and S170 completed/archived and says no active AI-architecture specs remain; the current merge commit is also the S170 merge.  
 **Tool limitations:** I did not clone the repository and did not execute the test suite locally. I used GitHub repo metadata, exact-SHA targeted file fetches, generated current-main docs, and external research for comparison. I did not use GitHub code-search snippets as evidence.

## **2. Freshness / Anti-Duplication Method**

I followed the requested chain: **repo metadata → current `main` SHA → tree manifest → targeted exact-SHA fetches → analysis**. The uploaded manifest was used only after live repo/default-branch verification. The current-main manifest was also fetched from `e0ec83d47621227bac0d76d5f44ee192a86ec369`.

I fetched the constitution and governance files from current main: `docs/FOUNDATIONS.md`, `AGENTS.md`, `CLAUDE.md`, `specs/IMPLEMENTATION-ORDER.md`, `docs/planner-contracts.md`, `docs/spec-drafting-rules.md`, `docs/scenario-roadmap.md`, generated golden/coverage docs, the latest triage, and current reports as historical context only. `FOUNDATIONS.md` is explicit about local causality, belief/world separation, local information carriers, concrete state, bounded reasoning, revisable intentions, learned-state provenance, causal history, and validation/falsification.

I fetched the S169/S170 live implementation surfaces: `verification_provider/**`, `plan_repair.rs`, `plan_revalidation.rs`, `partial_plan.rs`, `partial_plan_revalidation.rs`, `agent_tick/learned_state_observation.rs`, learned opportunity memory, route preference, testimony reliability, discrepancy, blocker memory, and decision payloads. I also fetched representative AI, sim, core, systems, CLI, diagnostic, generated-golden, workflow, and verification-script surfaces. The archive was treated as historical by default. The S169/S170 specs and tickets were only useful as intent; the audit conclusions below are based on current-main code/docs.

Rejected suspicion categories because current main already handles them:

| Suspicion | Current-main result |
| ----- | ----- |
| “Verification is AskWitness-only.” | Rejected at the repair/revalidation seam. A three-provider registry exists for `AskWitness`, `ConsultRecord`, and `SearchPlace`. |
| “S170 still lacks basic learned-state provenance.” | Rejected. Learned opportunity, route preference, discrepancy, blocker, and testimony reliability surfaces now carry concrete source/tick/event-style provenance. |
| “HTN methods have become hidden authority.” | Rejected. Live methods are `StageHint`; registry tests reject current `RequiredActionLeaf` usage. |
| “Partial-plan skeleton reuse is a rail.” | Rejected. Skeletons are filtered and revalidated; unsupported, stale, contradicted, or unknown predicates fail reuse. |
| “Scenario warnings prove architecture debt.” | Rejected. The roadmap explicitly separates structural activation from behavioral causal proof and treats some warning fields as support/editorial fields pending classification. |

## **3. Executive Stop-Condition Verdict**

**Yes: AI architecture cycles can honestly stop now.**

Not because the architecture is perfect. It is not. It still has proof-light seams: goal-level verification companion polymorphism is not generalized, learned-update-to-later-decision causality is indirect rather than a single joinable edge, and scenario-coverage warnings need classification. But none of those are proven architecture blockers on current main.

The next work should be **not** another AI-architecture wave, **not** a subsystem redesign, and definitely **not** a radical redesign. The next work is a transition into gameplay-mechanics readiness with hard guardrails and some proof-hygiene housekeeping.

The current GOAP / HTN-hint / utility-ranking / BDI-ish intention hybrid should be preserved. It is constitutionally aligned: belief-gated planning, local lawful action carriers, bounded search, revisable intentions, concrete learning state, deterministic traces, action affordance symmetry, and golden proof surfaces are all present.

What must not be done: no runtime LLM agents, no RL training loop, no global omniscient manager AI, no authored drama/quest rails, no method-required HTN authority without proof, no broad diagnostics-as-CI-gate redesign, and no re-opening S165–S170 as if they had not landed.

## **4. Current Architecture Map**

**World/core/sim/systems/AI/CLI responsibilities.** `worldwake-core` owns persistent world state, identities, beliefs, learned memory, route/testimony/discrepancy/blocker stores, intentions, decision event payloads, verification types, event log structures, and world transactions. `WorldTxn` stages state changes, records deltas/evidence/tags/decision payloads, and commits append-only events. `worldwake-sim` owns belief views, action definitions, action validation, affordance enumeration, scheduling, traces, save/load, and request resolution. `worldwake-systems` owns authoritative action handlers and perception/world mutation systems. `worldwake-ai` owns candidate generation, ranking, planning snapshots/state, GOAP search, HTN hints, plan repair/revalidation, partial plans, learned-state observation, traces, and diagnostics. `worldwake-cli` exposes player/debug interfaces; action menus use the same affordance machinery, while inspection is explicitly debug/observer truth, not normal POV-safe UI.

**Belief view surfaces.** `BeliefValue` carries confidence, acquired tick, claimed event tick, and status; statuses include certain, probable, stale, disputed, and contradicted. `PerAgentBeliefView` allows same-tick local physical observation but explicitly blocks that exception from serving as social/relational/institutional knowledge.

**Candidate generation.** Candidate generation is a typed extractor pipeline, not an omniscient manager. It uses source-sensitive goal schemas, opportunity compiler output, evidence traces, omission traces, blocker/discrepancy suppression, and ranking provenance.

**Opportunity compiler.** Opportunities carry source belief, possible effects, possible information, required actions, legal status, social exposure, risks, and salience. The compiler derives candidate actions from the effect schema index and tests source-belief statuses rather than treating compiled opportunities as truth.

**Ranking and portfolio.** Ranking is deterministic and source-aware. It includes learned opportunity bonus, source reliability discounting, testimony/source-composite context, route preference, repair memory, and competition pressure. The total order is deliberately encapsulated.

**GOAP tactical search and strategic search.** Search remains a bounded, explainable GOAP-style planner over lawful actions and planner-visible state. Planning contracts require exact terminal surfacing, belief-only inputs, source-scoped planner fields, duration/cost discipline, and no hidden world fallback.

**HTN/method guidance.** HTN is guidance, not authority. Current methods are `StageHint`; `RequiredActionLeaf` remains a guarded future hook. The registry has explicit negative tests preventing accidental current usage.

**Planning snapshot/state.** Snapshot admission sources distinguish self-authoritative, local same-tick physical, grounded evidence, belief last-seen, possession containment frontier, and public topology. Cache counters and private travel/distance state support performance without hidden truth access.

**Plan guards, revalidation, and repair.** Action definitions carry constraints, targets, preconditions, reservations, duration, cost, interruptibility, commit conditions, visibility, binding strictness, guards, expectations, and effect schemas. Start-gate and action validation re-check legality authoritatively before execution. Plan revalidation catches stale/contradicted/low-confidence beliefs and action/payload/affordance mismatches.

**S169 verification provider registry.** The registry now supports `AskWitness`, `ConsultRecord`, and `SearchPlace`, with deterministic provider ordering, provider kind, target, repair candidate, and rejection reasons. Provider implementations enforce locality and payload validity: witnesses must be local agents, records must be local record entities, and search-place candidates are actor-place overdue-expectation checks.

**Partial plans / skeleton reuse.** Partial-plan segments preserve completed prefix, remaining skeleton, terminal barrier, resume/abandon conditions, and causal links. Skeleton reuse is revalidated and treated as search seeding, not forced continuation.

**Player/AI symmetry.** CLI action listing uses `get_affordances()` through `PerAgentBeliefView`, the same affordance path AI uses. Debug inspection is explicitly marked as authoritative/debug-only and not normal POV-safe UI.

**Learning/diversity/habits after S170.** Learned opportunity memory, route preference, testimony reliability, discrepancy memory, and blocker memory now carry concrete source/event/tick/scope-style information.

**Traces/diagnostics/goldens/workflows.** Decision traces include repair attempts, provider/rejection traces, partial-plan resume traces, snapshot admissions, ranking provenance, omitted/suppressed candidates, opportunity compiler load, and cache counters. Generated proof docs report 59 golden scenario files, 292 `golden_*` tests, 224 scenario blocks, and a roadmap rule that a feature lands only when structural activation, behavioral proof, and causal reason all hold.

## **5. FOUNDATIONS Stop-Condition Matrix**

| Principle | Classification | Audit result |
| ----- | ----- | ----- |
| FND-1 local causality | Mechanically enforced | World mutations go through actions/systems/world transactions; no global story manager surface was found. |
| FND-3 concrete state over abstract scores | Mostly enforced | Ranking uses scores, but ties them to motive sources, learned memory, reliability, route preference, testimony, blockers, and traces. |
| FND-7 locality | Mechanically enforced for inspected AI seams | Verification providers and epistemic actions are local/same-place or actor-place lawful carriers. |
| FND-8 preconditions/duration/cost/occupancy | Mechanically enforced | `ActionDef`, start-gate, validation, reservation, duration, cost, and commit conditions are concrete. |
| FND-9 scheduling/tie-breaking | Mostly enforced | `tick_step` gives explicit tick order; ranking/search order are deterministic; request trace records resolution order. |
| FND-12 performance compression, not causality | Mostly enforced but proof-light | Snapshot/cache counters and private state access preserve causality, but future optimizations need equivalence tests. |
| FND-14/14A/14B belief/world separation | Mechanically enforced | Per-agent belief view distinguishes physical same-tick exception from social/institutional facts; static debug-view check exists. |
| FND-15 knowledge carriers | Mostly enforced | Ask, tell, consult record, search, perception, testimony, and institutional belief carriers exist. |
| FND-16 ignorance/stale/false/contradiction | Mostly enforced | Belief statuses and revalidation distinguish stale, contradicted, disputed, unknown, and low-confidence states. |
| FND-17 expectation violation | Mostly enforced | Search-place verification and expectation mismatch payloads exist; proof is adequate, not perfect. |
| FND-18 records/evidence | Mostly enforced | Record consultation and evidence/event payloads are concrete; records are lawful carriers. |
| FND-19 player/AI symmetry | Mostly enforced | Human action menu uses same affordance path; debug inspection is explicitly segregated. |
| FND-20 bounded reasoning | Mechanically enforced | Planning budgets, caps, beam/candidate counts, HTN hints, and diagnostics exist. |
| FND-21 revisable intentions | Mostly enforced | Revalidation, repair, partial plans, resume/abandon rules, and traces exist. |
| FND-22/22A diversity and learning | Mostly enforced but proof-light | S170 gives origin/scope/tick/event-style provenance; later decision-effect traceability is indirect. |
| FND-26 systems via state | Mechanically enforced | Systems mutate world state and event logs; AI consumes state/beliefs/traces, not privileged commands. |
| FND-27 summaries are caches | Mostly enforced | Generated docs, diagnostics, snapshots, and caches are derived surfaces with counters/checks. |
| FND-28 no compatibility fossils | Mostly enforced | No live AI authority path depends on archived specs; `RequiredActionLeaf` is guarded dormant hook, not live authority. |
| FND-29/29A debuggability/history | Mostly enforced | Event log verification, decision traces, action traces, request traces, and diagnostics exist. |
| FND-31 validation/falsification | Mostly enforced but proof-light | Golden inventory and roadmap are strong; some warnings need classification. |

## **6. Landed Work Regression Audit**

| Wave | Current main now does | Gap closed? | Remaining seam | Blocks stop? |
| ----- | ----- | ----- | ----- | ----- |
| S165 — Epistemic Verification Repair | Plan repair can insert verification and traces repair attempts/failures. | Yes | Superseded by S169 provider breadth. | No |
| S166 — Opportunity Compiler Source Fidelity | Opportunities carry source belief, effect facts, required actions, legal/risk/social fields; tests cover real statuses. | Yes | Future goal-family expansion must preserve source fidelity. | No |
| S167 — Cognitive Archetype Behavioral Proof | Golden inventory includes cognitive archetype tests and divergence scenarios. | Yes | None material. | No |
| S168 — Partial-Plan Skeleton Reuse | Skeleton reuse is filtered/revalidated and traceable. | Yes | Stuck/starvation watchlist remains ordinary runtime risk. | No |
| S169 — Generalized Lawful Verification Substrate | Three-provider registry exists with deterministic ordering and provider rejection kinds; payload validators and local rejection tests exist. | Yes at repair seam | Goal-level agenda-companion polymorphism remains not generalized. | No |
| S170 — Learned-State Provenance Hardening | Learned opportunity, route preference, discrepancy, blocker, testimony, and save format carry provenance meaning. | Yes | Direct “learned update X affected decision Y” is indirect through traces/context. | No |

**S169 specific verdict:** closed. The old “AskWitness-only repair seam” is no longer true. The remaining goal-level companion polymorphism issue is real but not proven harmful enough to justify another architecture wave. Current `GoalKind::AskWitness` still specializes entity-belief barriers, while `ConsultRecord` and `SearchPlace` are present in concrete goal/action contexts rather than a generic verification goal.

**S170 specific verdict:** closed enough. The auditor can answer “what changed this preference/memory?” for the inspected stores through source/event/tick/scope fields. The auditor can usually answer “how did it affect later decisions?” through decision traces and ranking context, but not with a single normalized causal edge. That is a future nicety, not a blocker.

## **7. Dead / Half-Finished Architecture Audit**

| Item | Evidence | Classification | Action | Blocks stop? |
| ----- | ----- | ----- | ----- | ----- |
| `RequiredActionLeaf` | Declared but current registry tests ensure no live method uses it. | Valid dormant hook | Leave guarded; require proof before use. | No |
| Goal-level generic verification companion | `GoalKind::AskWitness` remains entity-belief-centric; concrete `ConsultRecord`/`SearchPlace` paths exist elsewhere. | Deferred seam | Leave until a current scenario proves blockage. | No |
| Scenario coverage warnings | Generated coverage warns about support fields; roadmap says some are support/editorial until promoted. | Proof hygiene | Classify warnings; do not call architecture debt. | No |
| Opportunity compiler/emitter duality | Compiler and emitters coexist; compiler output is source-rich and deduped. | Valid parallel path | Preserve; add parity tests only when a concrete mismatch appears. | No |
| Debug/observer truth surfaces | Observer and inspect are explicitly tooling/debug; AI has static debug-view exclusion. | Valid tooling | Keep segregated. | No |
| Current reports in `reports/` | Older report inside current main is pre-S169/S170 and self-identifies older SHA. | Historical context | Do not use as live evidence unless reverified. | No |

No live dead-code or half-finished architecture was strong enough to justify another AI-architecture spec.

## **8. Missing-Capability Analysis**

| Capability | Current architecture provides | Cannot yet express cleanly | Needed now? | AI architecture or gameplay? | Blocks gameplay cycles? |
| ----- | ----- | ----- | ----- | ----- | ----- |
| Generic verification goal | Three-provider repair registry; concrete ask/record/search actions. | A single agenda-level “verify this belief/claim/expectation by any lawful carrier.” | No | AI architecture | No |
| Learned-update decision-effect edge | Stores provenance; ranking/decision traces expose learned context. | One normalized edge from learned update event to later selected/suppressed decision. | No | AI diagnostics/provenance | No |
| Full diagnostics-as-CI gate | Scenario diagnostics and long workflow lanes exist. | A broad pass/fail architecture dashboard. | No | Proof infrastructure | No |
| Candidate/opportunity unification | Source-rich compiler and traced emitters. | One universal emitter/compiler architecture. | No | AI architecture | No |
| Method-required HTN | StageHint methods, registry validation. | Authoritative method leaves. | No | AI architecture | No |
| Strong stuck-window formalism | Revalidation, partial plans, traces, diagnostics. | Universal starvation proof across all future gameplay. | No | Mixed; mostly gameplay/proof | No |

**No missing AI capability is proven as a blocker by current-main evidence.**

## **9. Proof and Diagnostics Readiness**

The proof surfaces are good enough to stop architecture cycles, but they are not a license to stop testing. Current proof includes unit/integration/golden tests, generated scenario details, generated golden inventory, a scenario roadmap, decision traces, action traces, request-resolution traces, scenario diagnostics, save/load versioning, and dedicated long-running workflow lanes.

**Are current goldens proving causal reason or merely plausible outcome?** Mixed, but trending strong enough. The roadmap explicitly says structural activation is not behavioral proof and that a scenario lands only when the golden proves the authored causal reason, not merely a rival lawful branch. The generated index includes cross-system chains and “Proves” fields, but some generated details still summarize scenario behavior rather than asserting every trace edge.

**Are scenario-coverage warnings meaningful blockers?** Not currently. They are classification warnings until promoted into canonical feature rows. The roadmap explicitly calls out fields such as `intention_disposition`, `expectation_store`, `last_seen_memory`, and `social_observations` as unresolved classification/support-field issues rather than automatic unlanded gameplay features.

**Should diagnostics-as-CI-gate remain deferred?** Yes. Diagnostic workflows exist for scenario diagnostics and planner pathology; `scripts/verify.sh` already gates formatting, workspace tests, static checks, clippy, and scenario coverage. A broad diagnostics CI gate would be architecture churn without a current-main defect.

**Is proof strong enough to stop architecture cycles?** Yes. It is not perfect, but it is now sufficient for a transition. The remaining proof work is classification/maintenance, not architecture design.

## **10. Research Synthesis**

GOAP remains a good fit. The F.E.A.R. lineage used STRIPS-like preconditions/effects so NPCs could pick goals and plan action sequences instead of relying on hand-authored state transitions; that maps directly to Worldwake’s desire to avoid hidden rails and derive behavior from local state/action legality. The lesson is to keep the planner action model explicit and debuggable, not to replace it with scripts.

HTN planning is useful when domain knowledge decomposes tasks into subtasks, but HTN can easily become stronger than ordinary action search because methods encode procedural knowledge and can generate executable primitive sequences. Worldwake’s StageHint-only HTN stance is therefore right: HTN should guide search until a stronger proof strategy justifies method-required leaves.

BDI architectures separate belief, desire/goal selection, and intention execution. Their useful lesson is commitment with reconsideration: intentions should persist long enough to avoid thrashing but be dropped when impossible, stale, or overridden by stronger needs. Worldwake’s agenda, intention frames, revalidation, abandonment, and partial-plan reuse are aligned with this model; pure BDI does not supply enough planning/provenance machinery by itself.

POMDP/belief-space literature confirms the value of explicit belief state and sensing actions under partial observability, but full optimal POMDP solving is not the right target here. Worldwake’s deterministic, explainable, belief-backed replanning and lawful verification actions are the practical constitutional subset.

Behavior trees are strong for modular reactive control, but adopting a BT-first redesign would risk turning Worldwake’s local-causality planning into designer-authored control flow. BT ideas are useful for reactive interruption and execution policy; they should not replace the current causal planner.

W3C PROV reinforces the value of concrete provenance: entities, activities, agents, derivation, generation, usage, and responsibility are the right conceptual vocabulary for “what changed this belief/memory and why should I trust it?” S170’s event/source/tick/scope fields are directionally correct; a future unified learned-effect edge would be a trace polish, not a redesign.

Explainable goal-driven-agent research emphasizes exposing perception, beliefs, goals, plans, and decisions to humans; Worldwake’s traces and generated golden details already implement that shape better than a black-box learning architecture would.

## **11. Alternatives Considered**

| Alternative | Benefits | Costs/risks | Migration burden | Recommendation |
| ----- | ----- | ----- | ----- | ----- |
| 1. Stop AI architecture cycles now | Avoids churn; preserves stable architecture; lets gameplay-mechanics work begin under guardrails. | Carries proof-light seams. | Low. | **Adopt.** |
| 2. One more targeted hardening wave | Could unify generic verification companions or learned-effect edges. | No proven blocker; likely invents work. | Medium. | Reject for now. |
| 3. Focused subsystem redesign | Could redesign agenda verification, diagnostics, or compiler/emitter boundary. | Current subsystems are functioning and traceable. | High. | Reject. |
| 4. Radical redesign | Theoretical chance to simplify. | Would endanger belief-only planning, local causality, determinism, and existing proof. | Extreme. | Strongly reject. |

Recommendation: **stop AI-architecture cycles now.**

## **12. Ranked Proposals, Only If Needed**

**No new AI-architecture proposals are warranted by current-main evidence.**

Watchlist items are not proposals: goal-level verification companion polymorphism, learned-update decision-effect traceability, and scenario-warning classification should be revisited only if gameplay work exposes a concrete failure.

## **13. Proposed Next Iteration Scope**

Because no AI-architecture proposals are warranted, the next iteration should be a **transition/gate cycle into non-AI gameplay-mechanics work**, not an implementation wave for S60–S66 and not a disguised architecture wave.

Do first:

1. Freeze the AI-architecture improvement queue.  
2. Preserve `scripts/verify.sh` and existing golden workflow lanes as the minimum gate.  
3. Classify current scenario-coverage warning fields as canonical feature, support field, fixture-only field, or obsolete mapping.  
4. Draft the next gameplay-mechanics prompt with explicit AI guardrails: belief-backed planner inputs, no hidden truth paths, no rails, action-level legality, causal golden assertions, S169/S170 proof preservation.

Defer:

* Generic verification goal.  
* Unified learned-effect causal edge.  
* Diagnostics-as-CI-gate.  
* Candidate/opportunity unification.  
* Method-required HTN.

Do not touch:

* Runtime LLM agents.  
* RL training.  
* Global manager AI.  
* Hidden quest/drama pacing.  
* S165–S170 reimplementation.  
* S60–S66 in this audit cycle.

Acceptance criteria for the transition: the first gameplay-mechanics cycle must not weaken belief/world separation, local carrier discipline, action legality, traceability, save/load meaning, or deterministic replay.

## **14. Proof Matrix**

| Invariant | Current proof status | Test type | Files/modules | Expected trace/provenance | Golden needed? | Static gate? | Failure smell | Blocks stop? |
| ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- | ----- |
| Belief/world separation remains intact | Strong | Static + golden + unit | `per_agent_belief_view`, `planning_snapshot`, `check_no_debug_view_in_ai.sh` | Admission source, belief status | No | Yes | AI reads debug/authoritative remote truth | Yes if broken |
| S169 repair does not silently correct remote truth | Strong | Unit + repair trace | `verification_provider/**`, `plan_repair` | Provider kind, local target, rejection reason | No | No | Repair inserts remote truth action | Yes if broken |
| S169 provider choice/rejection traceable | Strong | Unit + decision trace | `decision_trace`, `plan_repair` | `verification_provider`, `verification_rejections` | No | No | Repair succeeds without provider trace | Maybe |
| Goal-level barriers do not collapse to AskWitness | Proof-light but acceptable | Scenario when needed | `goal_model`, `goal_schema` | Companion goal kind and lawful carrier | Maybe later | No | Institutional/search fact can only ask witness | No today |
| Partial-plan skeleton never becomes rail | Strong | Unit + golden | `partial_plan*` | Resume decision, seeded ops, per-step verdict | No | No | Old skeleton executes despite stale/unknown belief | Yes if broken |
| Suspended intentions do not starve urgent needs | Moderate | Golden/diagnostic | `agenda_manager`, `frame_switch_policy`, diagnostics | Suspension/resume/abandon reason | Maybe | No | Long idle under critical need | No today |
| Candidate/opportunity generation source-faithful | Strong | Unit + golden | `candidate_generation`, `opportunity_compiler` | Source belief, required actions, suppression reason | No | No | Compiler emits truth-backed candidate | Yes if broken |
| HTN methods remain StageHint | Strong | Registry/static-style test | `htn/*` | Method trace selected/fallback | No | Yes | Required leaf appears silently | No today |
| Learning changes have origin/scope/decay | Strong enough | Unit + save/load | learned memory files | source/event/tick/scope/expiry | No | No | memory changes with no source | Maybe |
| Learned updates affect decisions traceably | Moderate | Trace assertion | `ranking`, `decision_trace` | learned context in ranked/selected candidate | Maybe later | No | decision changes with no visible learned context | No today |
| Ranking remains concrete-state reasoning | Strong enough | Unit + trace | `ranking`, `decision_payload` | motive sources, reliability, route/testimony | No | No | opaque aggregate score only | Maybe |
| Diagnostics prove causal reason when relied on | Moderate | Golden diagnostics | `scenario_diagnostics` | metric tied to traces/events | No broad gate | No | dashboard passes while causal branch absent | No today |
| Performance preserves causal equivalence | Moderate | Replay/perf | snapshots/cache/diagnostics | cache hit/miss/invalidation counters | When optimizing | No | cache hides truth-path difference | Maybe |
| Save/load/replay preserve AI meaning | Strong enough | Unit + replay | `save_load`, runtime | save version, runtime payload, provenance stores | No | No | learned state lost across load | Maybe |
| Player/AI/debug boundaries separated | Strong | Static + CLI tests | CLI handlers, observer, debug script | affordance provenance vs debug-only truth | No | Yes | player action uses omniscient inspect path | Yes if broken |

## **15. Invalid Tests / Dangerous Existing Expectations**

No current-main test was verified as invalid.

Dangerous-but-not-invalid expectations:

| Surface | Evidence | Risk | Action | Blocks stop? |
| ----- | ----- | ----- | ----- | ----- |
| Scenario coverage warnings | Generated warnings and roadmap classification language. | Treating support fields as unlanded features, or ignoring a real feature gap. | Classify warnings. | No |
| Older current reports | Current report file is pre-S169/S170 and names an older SHA. | Re-proposing landed work from stale report language. | Use only as historical context. | No |
| Plan-repair generated docs still emphasize AskWitness-era scenarios | Plan-repair details include S165 AskWitness proof; S169 provider proof lives more in current provider/repair code/tests. | Mistaking lack of generated doc prominence for lack of implementation. | Keep provider-specific tests; add golden only if gameplay exposes need. | No |
| Debug inspection reads truth | CLI inspect explicitly says debug/observer only, not POV-safe normal UI. | Accidentally using inspect output as player/AI contract. | Keep segregation. | No |

## **16. Stop Condition For AI Architecture Cycles**

A strict stop condition should require:

1. Current main verified against live branch SHA.  
2. No active AI-architecture specs in `specs/IMPLEMENTATION-ORDER.md`.  
3. Belief/world separation mechanically guarded.  
4. Planner-visible data is belief-backed, local physical same-tick, or lawful boundary artifact.  
5. Actions remain concrete: preconditions, duration, cost, occupancy/reservation, interruption, commit validation.  
6. GOAP search remains bounded and traceable.  
7. HTN remains hint-only unless method authority is separately proven.  
8. Repair/revalidation cannot silently correct remote truth.  
9. Verification provider choice/rejection is trace-visible.  
10. Learned state has origin/scope/tick/event/decay or explicit read-phase source.  
11. Learned state is preserved through save/load/replay.  
12. Decision traces expose candidate generation, suppression, ranking, selected plan, repair, fallback, partial-plan resume, and source failures.  
13. Debug/observer surfaces are segregated from AI/player normal action paths.  
14. Generated goldens/scenario docs are not treated as truth unless backed by current-main code/tests.  
15. No dormant hook is a live authority path without proof.

Acceptable remaining imperfections:

* Goal-level verification companion polymorphism is not generic.  
* Learned-update-to-later-decision causality is indirect.  
* Scenario coverage warning fields need classification.  
* Diagnostics are not a universal CI gate.  
* Some future gameplay mechanics may force new proofs.

Unacceptable remaining risks:

* Any remote truth read in AI planning.  
* Any untraceable provider repair.  
* Any action/HTN method that bypasses start-gate legality.  
* Any learned memory mutation with no source/tick/scope.  
* Any gameplay golden that passes by plausible outcome instead of causal reason.

**Does current main satisfy the stop condition now?** Yes, close enough to stop AI-architecture cycles. The remaining gaps are not strong enough to justify another AI-architecture iteration.

## **17. Gameplay-Mechanics Readiness Gate**

**Is AI architecture ready enough for later gameplay-mechanics improvement cycles?** Yes.

AI-architecture blockers that must be resolved first: **none proven**.

AI-architecture risks that can be carried safely:

* Generic verification goal not yet present.  
* Learned-effect traceability is indirect.  
* Scenario-warning classification remains incomplete.  
* HTN `RequiredActionLeaf` remains dormant.  
* Diagnostics are not broad CI gates.

Guardrails for a future gameplay-mechanics prompt:

* New mechanics must be implemented as world state, actions, records, relations, obligations, artifacts, boundaries, or systems—not AI shortcuts.  
* Planner-visible inputs must be belief-backed, local same-tick physical observations, or lawful boundary artifacts.  
* Social/institutional facts must never use the local physical exception.  
* New learned state must include origin, scope, tick/event or explicit read-phase inference, decay/overwrite semantics, save/load preservation, and decision-trace visibility.  
* New actions must expose preconditions, duration, cost, occupancy/reservation, interruption, visibility, payload validation, and effect schema.  
* Goldens must assert causal reasons or named lawful alternatives.  
* HTN methods stay `StageHint` unless a dedicated method-authority proof suite lands first.  
* Debug/observer affordances must not become player/AI normal surfaces.

## **18. Implementation Risk and Migration Order**

No AI-architecture migration is recommended.

Transition order:

1. Mark AI-architecture improvement cycles stopped for current main.  
2. Run the existing local verification lane before opening gameplay work: `cargo fmt`, workspace tests, static checks, clippy, and scenario coverage via `scripts/verify.sh`. I did not run this locally.  
3. Classify scenario-coverage warning fields as support/canonical/fixture/obsolete.  
4. Start a separate gameplay-mechanics scoping cycle with the guardrails above.  
5. During gameplay work, reopen AI architecture only if a concrete current-main failure appears: illegal truth path, untraceable verification, learned-state mutation without provenance, planner-visible cache becoming authority, or a golden that can only pass through hidden causality.

## **19. Final Recommendation**

**Stop AI architecture cycles.** Current main is aligned well enough with `docs/FOUNDATIONS.md` to move toward gameplay-mechanics work under strict guardrails.

Top 3 actions:

1. Close the AI-architecture loop now.  
2. Prepare the next gameplay-mechanics prompt with explicit belief/locality/action/provenance proof gates.  
3. Classify scenario-coverage warnings as hygiene, not architecture proposals.

Top 3 risks:

1. Reopening architecture because “more traceability would be nice” rather than because current main fails.  
2. Gameplay work accidentally introducing hidden truth paths or authored outcome rails.  
3. Turning HTN or diagnostics into authority without proof.

Top 3 proof gates:

1. S169 provider locality, provider rejection, and repair trace.  
2. Belief/world/debug boundary static and golden proof.  
3. S170 learned-state provenance plus save/load/replay preservation.

Top 3 things not to do:

1. Do not add LLM/RL/global-manager/story-rail systems.  
2. Do not promote `RequiredActionLeaf` without a dedicated proof suite.  
3. Do not re-propose S165–S170 or open S60–S66 from this audit.

**One-sentence verdict:** continue gameplay-mechanics preparation; do not continue AI-architecture improvement cycles unless future current-main evidence proves a real constitutional regression.

