# Implementation Order & Dependency Graph

## Completed Work

### Phase 1: World Legality (E01–E08) — COMPLETED
Established core and sim crates: ECS, topology graph, item/container model, conservation invariants, relation system, append-only event log with causal linking, transactional world mutations, canonical state hashing, action framework with preconditions, tick-driven scheduler, deterministic replay, and save/load persistence.

### Phase 2: Emergent Economy (E09–E13) — COMPLETED
Established systems and ai crates: homeostatic needs and deprivation wounds, resource regeneration and recipe-based crafting, merchant trade with valuation, combat with wound tracking, and pressure-based GOAP decision architecture with goal ranking, plan search, failure handling, and per-tick autonomous agent control.

### FND-01: Phase 1 Foundations Alignment — COMPLETED
Removed route scores, banned zero-tick actions, replaced load match-arms with physical profiles, renamed KnowledgeView→BeliefView, constrained loyalty mutations.

### FND-02: Phase 2 Foundations Alignment — COMPLETED
Aligned the E14/E15 specs with determinism and Section H requirements, preserved `SellCommodity` deferral until S04, wired treatment-oriented acquisition candidates, documented Phase 2 dampening, and added goal/event debuggability APIs.

### E21: CLI & Human Control — COMPLETED
Pulled forward from Phase 4 post-Phase 2 as the primary manual testing interface.

### E14: Perception & Belief System — COMPLETED
Established per-agent belief stores, passive/direct perception, social observation capture, `PerAgentBeliefView`, belief-only planner reads, public route/place structure reads, and belief-mediated remote facility/resource discovery.

### E15: Rumor, Witness & Discovery — COMPLETED
Established Tell-based social transmission, per-agent Tell profiles, discovery events for violated expectations, social observation capture for telling, and transaction-owned metadata finalization for transaction-built E15 event payloads.

### E15b: Social AI Goals + Golden E2E Tests — COMPLETED
Established GoalKind::ShareBelief, PlannerOpKind::Tell, social candidate generation with chain-length filtering, social_weight-driven ranking, 10 golden social E2E tests covering autonomous Tell, rumor relay, discovery correction, agent diversity, and the full information lifecycle. Added system-wide zero-motive filter in rank_candidates() and treatment_pain() helper for treatment acquisition scoring.

### E16: Offices, Succession & Factions — COMPLETED
Established offices and factions as first-class institutions, support declarations, courage-driven coercion handling, office actions, succession resolution, public-order aggregation, and political AI integration through the belief/runtime-view boundary. The force branch remains intentionally conservative; explicit contested control stays in active follow-up spec E16b.

### E16d: Political Planning Fix & Golden E2E Coverage — COMPLETED
Established planner semantics and verification coverage for political coalition-building and locality: `Bribe`/`Threaten` now participate in planning, office goldens cover claim/coalition/travel/suppression/eligibility/force/locality behavior, and the political golden coverage docs are up to date. The original “incumbent defense” sub-plan was corrected out of scope rather than forced into the support-law architecture.

### S01: Production Output Ownership Claims — COMPLETED
Established explicit ownership for production output: `ProductionOutputOwnershipPolicy` component with Actor/ProducerOwner/Unowned variants, `create_item_lot_with_owner()` atomic helper, `can_exercise_control()` extended with faction/office delegation, harvest and craft commit ownership resolution, `believed_owner_of()` belief query, ownership-gated pickup validation, and GOAP planner adaptations for actor-owned output.

### S02: Goal Decision Policy Unification — COMPLETED
Unified goal-family decision policy into a single `goal_policy.rs` module with `GoalFamilyPolicy`, `DecisionContext`, and `evaluate_suppression()`. Ranking and interrupts both consume the shared policy surface. Removed all four legacy functions (`is_suppressed`, `is_critical_survival_goal`, `is_reactive_goal`, `no_medium_or_above_self_care_or_danger`). All 16 goal families migrated.

### S03: Planner Target Identity & Affordance Binding — COMPLETED
Added `matches_binding()` to `GoalKindPlannerExt` with auxiliary-pass/terminal-check dispatch across all 17 `GoalKind` variants. Wired `.retain()` binding filter in search candidates. Added `BindingRejection` trace struct for debuggability. Exact-bound goals (`LootCorpse`, `EngageHostile`, `TreatWounds`, `ShareBelief`, `BuryCorpse`) now reject wrong-target affordances before successor construction; flexible goals remain unaffected.

### S07: Care Intent & Treatment Targeting — COMPLETED
Replaced split care model (`GoalKind::Heal` + `CommodityPurpose::Treatment`) with unified patient-anchored `GoalKind::TreatWounds { patient }`. Added `care_weight` to `UtilityProfile` for self/other ranking split. Made self-care lawful. Established `DirectObservation` gate for third-party care via `emit_care_goals()`. 12 golden tests prove self-care, third-party care, observation gate, and care goal invalidation.

### S08: Action Start Abort Resilience — COMPLETED
Established recoverable BestEffort start-abort handling, structured authoritative start-failure handoff into AI/runtime reconciliation, validation-only `start_heal()`, first-effect Medicine spending with explicit heal-local state, and golden care coverage for the real pre-start wound-disappearance race.

### E15c: Conversation Memory & Recipient Knowledge — COMPLETED
Replaced same-place Tell suppression with explicit conversation memory: `AgentBeliefStore` now tracks told/heard memory with retention-aware reads and deterministic eviction, Tell affordances and AI social candidate generation share listener-aware resend suppression, Tell commit records participant memory and heard dispositions, and golden/trace coverage proves unchanged-repeat suppression plus lawful re-tell after belief change or expiry.

### S14: Conversation Memory Emergence Golden E2E Suites — COMPLETED
Established cross-system golden proof that same-place office facts still require Tell before political planning appears, and that listener-aware resend suppression happens before tell-candidate truncation so untold office facts can still unlock downstream office behavior.

### S15: Start-Failure Emergence Golden E2E Suites — COMPLETED
Established production, trade, and political golden proof that the shared S08 `StartFailed` contract survives outside care: lawful authoritative start rejection now has end-to-end coverage across contested harvest, vanished local trade, and remote office-claim races, including next-tick AI reconciliation and deterministic replay.

### S16: S09 Golden Validation Suite — COMPLETED
Established the delivered S09 behavioral golden coverage and aligned the archival trail to the shipped ownership boundaries: `golden_defend_changed_conditions` now proves defend re-evaluation under changed conditions in `golden_combat.rs`, and `golden_spatial_multi_hop_plan` plus its deterministic replay companion prove default-budget VillageSquare hub reachability in `golden_ai_decisions.rs`. The later S16 follow-up work also strengthened the spatial decision-trace boundary with winning-search provenance and refactored the helper structure without duplicating the scenario.

### S11: Wound Lifecycle Audit — COMPLETED
Established the wound lifecycle corrections and hardening planned for Phase 3: deprivation harm now worsens a stable same-kind wound instead of duplicating entries, clotted-wound recovery gates are reflected in AI self-care ranking, wound progression/pruning contracts have focused regression coverage, and the golden/workspace verification boundary passed without requiring additional hash recapture.

### S17: Wound Lifecycle Golden E2E Suites — COMPLETED
Established the missing golden proof for the two S11 wound-lifecycle invariants: Scenario 29 in `golden_emergent.rs` now proves deprivation wound worsening consolidates through live needs dispatch without duplicating wounds, with a deterministic replay companion; Scenario 30 in `golden_combat.rs` now proves recovery-aware priority promotion makes the actor eat before wash so the recovery gate opens and wound severity decreases, also with deterministic replay coverage. Golden coverage docs and generated inventory were kept aligned at 133 `golden_*` tests.

### S12: Planner Prerequisite-Aware Search — COMPLETED
Established dynamic per-node prerequisite-aware place guidance for planner search, focused trace/budget coverage, and canonical remote-care plus remote-production golden proof. The final completion also aligned `ProduceCommodity` ranking and candidate generation with the planner architecture so remote self-consume crafting stays truthful as `ProduceCommodity` instead of falling back to stale acquire proxies.

### S18: Prerequisite-Aware Emergent Chain Goldens — COMPLETED
Established the missing Phase 3 golden proof for craft-restock and stale prerequisite-belief recovery, and closed the archival loop on the live planner contract after the stale `ProduceCommodity` narrative was corrected to the lawful `RestockCommodity` surface.

### S19: Institutional Record Consultation Golden E2E Suites — COMPLETED
Established the delivered office-record golden coverage and closed the documentation/archive loop around the live source-declared scenarios: Scenario 33 now proves remote record travel plus consultation plus political follow-through, Scenario 34 proves knowledge-asymmetry-driven office races, and the hand-maintained golden docs were aligned to the shipped scenario inventory instead of the older planned Scenario 32 narrative.

### S32: Crime Emergence Golden E2E Suites — COMPLETED
Established the delivered E17 crime/justice emergence coverage and closed the remaining documentation/archive loop around the live source-declared scenarios: Scenario 41 proves Fine infeasibility falls through to lawful exile based on local collectibility plus governed faction membership, Scenario 42 proves witness-count deterrence suppresses theft before action start, and Scenario 43 proves owner-local discovery and witness-report discovery converge on one accusation rather than duplicating institutional case state.

### E16b: Force Legitimacy & Jurisdiction Control — COMPLETED
Replaced the thin E16 force-succession shortcut with explicit jurisdiction-control state: public claims, contested control, uncontested hold periods, and eventual installation into office. Force succession now arises from concrete local state (claims, contests, hold durations) rather than a hidden timing heuristic, with proper information paths for coups and recognition.

### S16b (golden): Force-Legitimacy Emergence Golden E2E Suites — COMPLETED
3 cross-system golden tests in `golden_emergent.rs` proving E16b's force-legitimacy mechanisms participate in emergent multi-system chains: Suite 10 (controller departure enables rival claim), Suite 11 (force claim creates hostility witnessed and propagated), Suite 12 (contested force state propagates through belief system). All 6 tests (3 main + 3 replay) pass with deterministic replay.

### E17: Crime, Theft & Justice — COMPLETED
Established theft as an explicit ownership-bypassing action, typed suspected-theft evidence, accusation and verdict record entries in `CrimeRegister`, fine/exile punishment actions, crime/justice AI candidate generation and profile-driven ranking, and emergent golden proof for owner-local discovery, witnessed accusation chains, punishment traceability, and ownership-gated non-theft paths. The completed spec is archived at `archive/specs/E17-crime-theft-justice.md`.

### S20: AI Pipeline Structural Cleanup — COMPLETED
Split `agent_tick.rs` (~6124 lines) into `agent_tick/` directory with 7 sub-modules (mod, observation, candidates, active_action, planning, execution, journey) plus dedicated test module. Split `search.rs` (~5880 lines) into `search/` directory with 5 sub-modules (mod, frontier, heuristic, transition, candidates) plus dedicated test module. Pure refactoring with zero behavioral change — all 304 golden tests passed unchanged.

### S26: Planner Conformance Tests — COMPLETED
32 conformance tests comparing planner hypothetical transitions against authoritative handler outcomes for each action family: needs (eat, drink, sleep, relieve, wash), transport (pick_up, put_down), production (harvest, craft — no-op coverage gaps documented), movement (travel), economy (trade — no-op gap), combat (attack — no-op gap, loot, heal, bury), and political (declare_support, press_force_claim, queue_for_facility). One minor production accessor added (`PlanningState::homeostatic_needs_for`). 3 of 6 political tests deferred (consult_record, bribe, threaten).

### S21: Promote Causal Runtime State — COMPLETED
Promoted `ActiveGoal`, `JourneyCommitment`, and `FacilityQueueIntents` from ephemeral `AgentDecisionRuntime` fields to authoritative ECS components in `worldwake-core`. Relocated `JourneyCommitmentState` and `QueuedFacilityIntent` types to core. Refactored journey helpers to free functions with explicit component parameters. Bumped `SAVE_FORMAT_VERSION` from 4 to 5. Added save/load round-trip golden test verifying commitment preservation. Fixes FOUNDATIONS P11/P16/P19 violations where save/load lost journey commitments and active goals.

### S23: Refined Blocked Intents — COMPLETED
Refactored `BlockedIntentMemory` from `Vec<BlockedIntent>` to `BTreeMap<BlockerKey, BlockedIntent>` with compound keying (goal + place + target + action_def). Place-scoped blockers no longer suppress at candidate generation; they prune at plan search via `is_blocked_for_search()`. Unknown blockers use dedicated 5-tick TTL with `BlockerDiagnostic` context. `UnknownBlockerTrace` and `PlaceBlocker` filter reasons integrated into decision traces. `search_plan()` takes `&BlockedIntentMemory` parameter.

### S22: Generalized Intention Frames — COMPLETED
Replaced travel-specific `JourneyCommitment` + `TravelDispositionProfile` with domain-agnostic `IntentionFrame` + `IntentionDispositionProfile`. Added `IntentionDomain` (Travel, Care, Generic), `FrameAssumption` evaluation with critical/recoverable distinction, progress detection via `PlannerOpKind`, frame exhaustion → `BlockedIntent` integration with `PatienceExhausted` and `AssumptionFailed` blocking facts, `FrameTransitionTrace` in decision traces, save/load round-trip coverage, and workspace-wide verification. All 8 old journey types removed.

### S24: Typed Invalidation Domains — COMPLETED
Replaced `dirty: bool` + `Vec<DirtyReason>` dual-tracking with `DirtySet` (u16 bitflag newtype) across the entire AI pipeline. `DirtyReason` enum removed. 14 typed domains: 6 structural + 6 snapshot + 2 frame lifecycle. Trace output now shows specific domain names (e.g., `NEEDS|POSITION`) instead of opaque `SnapshotChanged`. `is_snapshot_only()` enables plan-continuation optimization when only snapshot domains changed.

### S25: Feasibility Sketching — COMPLETED
Added cheap feasibility pre-check (`FeasibilityHint`: Likely/Uncertain/Unlikely) to candidate ranking pipeline. Two-phase architecture: shared checks (exhausted frame, blocker memory) plus per-GoalKind dispatch table across all 17 variants. Feasibility reorders candidates within the same `GoalPriorityClass` — never excludes goals. Integrated into `process_agent()` after frame evaluation, decision traces show feasibility per candidate.

### S28: Knowledge-Path Traces — COMPLETED
Extended decision traces with belief provenance: `HomeostaticNeedId` in core, `institutional_belief_claims()` on `GoalBeliefView`, knowledge path types (`BeliefProvenance`, `BeliefAspect`, `InstitutionalBeliefProvenance`, `SelfKnowledgeProvenance`, `KnowledgePath`) in worldwake-ai, all 6 candidate families instrumented (needs, production, enterprise, combat, social, political), social/political emitters upgraded to `emit_candidate_with_trace()`, `dump_agent()` extended with per-candidate evidence/knowledge-path rendering. Zero-cost when tracing disabled.

### S27: Expectation-Violation Goals — COMPLETED
Established belief-vs-observation violation detection, per-agent violation memory/disposition components, exact-id `GoalKind::InvestigateViolation { violation_id, place }`, authoritative investigate execution with `WitnessedAbsence` aftermath, explicit unresolved/resolved incident lifecycle, and golden coverage for single-incident investigation, local depletion-to-`ShareBelief`, and same-place concurrent-violation identity.

### S31: Goal-Aware Exhaustion Invalidation — COMPLETED
Replaced coarse exhaustion invalidation with goal-aware conditions plus baseline snapshots, removed the `EXHAUSTION_SKIP_TTL` retry gate in favor of condition-based cache clearing, separated budget-exhaustion retry from invalidation semantics, and closed the loop with golden validation for needs-driven recovery, unrelated commodity non-invalidation, and deterministic save/load persistence of the exhaustion runtime state. The completed spec is archived at `archive/specs/S31-goal-aware-exhaustion-invalidation.md`.

### S13: Political Emergence Golden E2E Suites — COMPLETED
Established cross-system political emergence golden coverage: Scenario 44 proves utility-weight-driven care-vs-enterprise priority ordering under wounds, Scenario 45 proves combat death cascading into force-law vacancy and succession through shared state, and Scenario 46 proves autonomous Tell transferring institutional office beliefs to unlock political planning at a remote jurisdiction. All 7 golden tests (3 main + 2 variant + 3 replay) in `golden_emergent.rs` with structured `// Scenario 44/45/46:` source annotations. The completed spec is archived at `archive/specs/S13-political-emergence-golden-suites.md`.

### S33: Opportunity-Scoped Goal Identity — COMPLETED
Established desire-level `GoalKey` vs opportunity-level `OpportunityKey` separation across the full AI pipeline: per-opportunity `GroundedGoal` emission, post-emission blocker filtering with desire-level blocked diagnostics, opportunity-keyed exhaustion/runtime persistence, ranked same-goal sibling admission without desire-level collapse, `PlannedPlan.opportunity`, and golden proof for blocked-source and exhausted-opportunity source switching. The completed spec is archived at `archive/specs/S33-opportunity-scoped-goal-identity.md`.

All completed specs are archived under `archive/specs/`.

---

## Dependency Graph

```text
Phase 1-2 + FND-01 + FND-02 + E21 + E14 + E15 + E15b + E15c + E16 + E16b + E16c + E16d + S01 + S02 + S03 + S07 + S08 + S14: COMPLETED

S09 ✅ (design fix to defend action duration completed)
S11 ✅ (wound lifecycle audit completed)
S11 ──→ S17 ✅ (wound lifecycle golden E2E coverage completed)
S12 ✅ (planner prerequisite-aware search heuristic completed)
E16c ──→ S13 ✅ (political emergence golden coverage completed)
E16b, E16c ──→ S16b-golden ✅ (force-legitimacy emergence golden suites completed)
E16c ──→ S19 ✅ (institutional record consultation golden coverage completed)
S15 ✅ (cross-system start-failure emergence golden coverage)
S16 ✅ (S09 behavioral golden validation coverage)
E15 ──→ E15b (social AI goals need Tell mechanics + belief system) ✅
E15, E15b ──→ E15c ✅ (conversation memory and recipient knowledge completed)
E15c, E16d ──→ S14 ✅ (cross-system golden proof for same-place Tell and listener-aware pre-truncation)
S01 ──→ ✅ COMPLETED (production output ownership claims)
S02 ──→ ✅ COMPLETED (goal decision policy unification)
E16 ──→ E16c (institutional beliefs need offices/factions/support substrate)
E16c ──→ E16b ✅ (force legitimacy needs institutional records and belief propagation)
E16c ✅ ──→ E17 ✅ (justice records and institutional knowledge now reuse the same record/belief architecture)
E16 ──→ E16b ✅ (explicit force legitimacy needs offices, factions, and succession substrate)
E15 ✅, S03 ✅ ──→ E17 ✅ (crime closeout shipped on top of discovery, ownership claims, and planner binding)
E16 ──→ E18 (bandits need faction system)
E16 ──→ E19 (guards need public order)
E16b ✅ ──→ E19 (guards need contested-office control state)
E16c ──→ E19 (guards need institutional belief/record pathways)
S01 ✅, S03 ✅, E16c ✅ ──→ E17 ✅ (crime closeout shipped with ownership claims, planner binding, and record architecture)
S27 ✅ ──→ E17 ✅ (crime discovery now builds on the delivered expectation-violation theft-detection path)
E17 ✅ ──→ E19 (guard crime response depends on the shipped theft/justice substrate)
S02 ✅, E16 ──→ E18, E20
S02 ✅, E16, E16b ✅, E16c ──→ E19
E16c ──→ S05 (institutional stock ledgers should reuse record architecture)
S04 ──→ S05 (stock storage needs selling + ownership)
S04 ──→ S06 (opportunity valuation needs market presence)
S10 (no unmet deps — E11 trade + E14 perception both completed; can be scheduled anytime)
S10 ──→ S06 (opportunity valuation benefits from variable pricing)
E14 provides the prerequisite belief boundary for E15, ~~E15c~~, E16, E16c, ~~S01~~, ~~S02~~, ~~S03~~, S04, ~~S07~~, and S10.
S31 ✅, S23 ✅, S22 ✅ ──→ S33 ✅ (opportunity-scoped identity completed)
S33 ✅ ──→ S36 ✅ (declarative registration delivered on top of final goal identity shape)
S33 ✅ ──→ S37 ✅ (cooldown exhaustion shipped on opportunity-scoped identity)
S33 ✅ ──→ S39 (side-benefit scoring needs opportunity awareness)
S35 ✅ ──→ S38 (learned preferences needs activity observation for source reliability)
S27 ✅ ──→ S34 ✅ (epistemic actions extend violation detection)
S34 ✅, S35 ✅ (independent, can parallel with S33)
E18, E19, E20 ──→ E22 (integration tests need everything)

S20 ✅ (structural cleanup completed — groundwork for S21–S28)
S26 ✅ (planner conformance tests completed — 32 tests across all action families)
S20 ✅ ──→ S21 ✅ (promote causal runtime state — completed)
S20 ✅ ──→ S23 ✅ (refined blocked intents — completed)
S21 ✅ ──→ S22 ✅ (generalized intention frames — completed)
S21 ✅ ──→ S24 ✅ (typed invalidation domains — completed)
S21 ✅ ──→ S28 ✅ (knowledge-path traces — completed)
S20 ✅, S23 ✅ ──→ S25 ✅ (feasibility sketching — completed)
S22 ✅, S23 ✅ ──→ S27 ✅ (expectation-violation goals completed)
```

---

## Active Execution Steps

### Phase 3: Information & Politics

**Step 9**: COMPLETED
- **E14**: Perception & Belief System
  - Replaced `OmniscientBeliefView`
  - Established the belief/evidence inputs later social systems use for loyalty/support modeling
  - Satisfied FND-01 Section B deferred information pipeline requirements

**Step 10** (parallel after completed E14/E15):
- **E15b**: Social AI Goals — ✅ COMPLETED
- **E16**: Offices, Succession & Factions — ✅ COMPLETED
- **S01**: Production Output Ownership Claims — ✅ COMPLETED
- **S02**: Goal Decision Policy Unification — ✅ COMPLETED
- **S03**: Planner Target Identity & Affordance Binding — ✅ COMPLETED
- **S07**: Care Intent & Treatment Targeting — ✅ COMPLETED

**Step 11** (parallel):
- **E15c**: Conversation Memory & Recipient Knowledge — ✅ COMPLETED
  - established explicit conversation memory and lawful resend suppression
- **S08**: Action Start Abort Resilience — ✅ COMPLETED
  - recoverable start-failure classification, structured AI/runtime failure handoff, first-effect Medicine spending, and care golden regression are in place
- **S09**: Indefinite Action Re-Evaluation — ✅ COMPLETED
  - removed indefinite action duration paths; defend now uses profile-driven finite `ActorDefendStance` duration and re-enters normal replanning after commit
- **S11**: Wound Lifecycle Audit — ✅ COMPLETED
  - delivered deprivation-wound identity preservation, recovery-aware self-care ranking, wound hardening coverage, and closeout verification
- **S12**: Planner Prerequisite-Aware Search — ✅ COMPLETED
  - dynamic combined-place search guidance, focused trace/budget coverage, and canonical remote-care plus remote-production goldens are in place
- **S15**: Start-Failure Emergence Golden E2E Suites — ✅ COMPLETED
  - production/trade/politics goldens now prove `StartFailed` plus next-tick AI recovery outside the care domain
- **S16**: S09 Golden Validation Suite — ✅ COMPLETED
  - shipped defend changed-conditions and VillageSquare spatial-validation goldens at their canonical ownership boundaries, with deterministic replay and decision-trace-first route proof
- **S17**: Wound Lifecycle Golden E2E Suites — ✅ COMPLETED
  - Scenario 29 in `golden_emergent.rs` proves deprivation worsening consolidates without duplicating wounds, and Scenario 30 in `golden_combat.rs` proves recovery-aware need promotion opens the wound-recovery gate; both shipped with deterministic replay companions and aligned golden docs/inventory
- **S18**: Prerequisite-Aware Emergent Chain Goldens — ✅ COMPLETED
  - craft-restock and stale prerequisite-belief recovery goldens are shipped, and the stale planner-surface narrative was corrected to the live `RestockCommodity` contract before archival
- **S19**: Institutional Record Consultation Golden E2E Suites — ✅ COMPLETED
  - shipped office-record golden proof for the live Scenario 33 remote-record route and Scenario 34 knowledge-asymmetry race, then aligned the human-maintained golden docs to the generated/source-declared inventory instead of the older planned Scenario 32 draft narrative
- **S32**: Crime Emergence Golden E2E Suites — ✅ COMPLETED
  - shipped Scenario 41 exile fallback, Scenario 42 witness deterrence, and Scenario 43 dual-discovery convergence in `golden_emergent.rs`, then aligned the dashboard/generated inventory/archive trail to the live source-declared scenarios

**Step 12**:
- **E16c**: Institutional Beliefs & Record Consultation
  - needs E14, E15, E16
  - ✅ completed: AI no longer reads live office/faction/support helper seams; political goldens now rely on institutional beliefs or consultation-owned setup

**Step 13**:
- **E16b**: Force Legitimacy & Jurisdiction Control — ✅ COMPLETED
  - replaced thin force-succession shortcut with explicit jurisdiction-control state (claims, contests, hold durations, installation)
- **E17**: Crime, Theft & Justice — ✅ COMPLETED
  - explicit theft, crime evidence, accusation/punishment records, and E17 golden closeout are shipped; archived spec: `archive/specs/E17-crime-theft-justice.md`; Phase 3 itself remains open for non-E17 items such as `S13`, `T10`, `T11`, and `T25`
- **S13**: Political Emergence Golden E2E Suites — ✅ COMPLETED
  - 7 golden tests (Scenarios 44/45/46) proving combat-death succession, Tell-driven office claims, and care-vs-enterprise priority ordering
  - archived spec: `archive/specs/S13-political-emergence-golden-suites.md`
- **S16b-golden**: Force-Legitimacy Emergence Golden E2E Suites — ✅ COMPLETED
  - 3 golden suites proving force-legitimacy participates in emergent multi-system chains (controller departure, hostility propagation, contested state propagation)

#### Phase 3 Gate
- [ ] `OmniscientBeliefView` fully replaced — no code path uses it
- [ ] Information propagates through explicit channels (witnesses, rumors, records)
- [ ] Offices transfer through succession
- [x] Redundant Tell suppression uses explicit conversation memory rather than same-place listener-knowledge shortcuts
- [x] Political planning gives explicit `Bribe`/`Threaten` outcomes instead of falling through unchanged planning state
- [x] Political golden coverage proves claim, coalition, threat, travel, eligibility, suppression, force-succession, and locality scenarios
- [x] Institutional facts propagate through records and consultation rather than live helper shortcuts
- [x] Force succession uses explicit contest/control state rather than presence-only installation
- [ ] All FND-02 tickets verified closed
- [ ] T10: Belief isolation — agent does not react to unseen theft, death, or camp migration
- [ ] T11: Office uniqueness
- [ ] T25: Unseen crime discovery

---

### Phase 3+: AI Architecture Overhaul

**Step 13.5 Wave 0** (parallel, no deps) — ✅ COMPLETED:
- **S20**: AI Pipeline Structural Cleanup — ✅ COMPLETED
  - split `agent_tick.rs` and `search.rs` into modular sub-directories with dedicated test modules
- **S26**: Planner Conformance Tests — ✅ COMPLETED
  - 32 conformance tests comparing planner transitions against handler outcomes for all action families
  - one minor production accessor (`PlanningState::homeostatic_needs_for`)

**Step 13.5 Wave 1** (parallel, after S20):
- **S21**: Promote Causal Runtime State — ✅ COMPLETED
  - promoted `ActiveGoal`, `JourneyCommitment`, `FacilityQueueIntents` to authoritative ECS components
  - fixed FOUNDATIONS P11/P16/P19 violations (save/load now preserves commitments)
  - `SAVE_FORMAT_VERSION` bumped to 5; save/load golden test verifies commitment preservation
- **S23**: Refined Blocked Intents — ✅ COMPLETED
  - compound-keyed blocker records (goal + place + target + action_def), place-scoped search pruning, 5-tick Unknown TTL with diagnostics
  - `BlockedIntentMemory` refactored from `Vec` to `BTreeMap<BlockerKey, BlockedIntent>`; `search_plan()` gains `&BlockedIntentMemory` for place-scoped candidate pruning

**Step 13.5 Wave 2** (parallel, after S21):
- **S22**: Generalized Intention Frames — ✅ COMPLETED
  - replaced travel-specific journey commitment with domain-agnostic IntentionFrame + IntentionDispositionProfile
  - removed all 8 old journey types; added assumption evaluation, progress detection, exhaustion→BlockedIntent, frame traces, save/load coverage
- **S24**: Typed Invalidation Domains — ✅ COMPLETED
  - replaced `dirty: bool` + `Vec<DirtyReason>` with `DirtySet` bitflag newtype for typed replan triggers
- **S25**: Feasibility Sketching — ✅ COMPLETED
  - cheap feasibility pre-check before GOAP search budget allocation
  - two-phase architecture (shared checks + per-GoalKind dispatch), integrated into ranking and decision traces
- **S28**: Knowledge-Path Traces — ✅ COMPLETED
  - extended decision traces with belief provenance across all 6 candidate families
  - `dump_agent()` renders per-candidate evidence, feasibility, and knowledge paths

**Step 13.5 Wave 3** — ✅ COMPLETED:
- **S27**: Expectation-Violation Goals — ✅ COMPLETED
  - belief-vs-observation mismatch now produces concrete recorded violation incidents
  - shipped exact-id `InvestigateViolation` goals plus unresolved/resolved incident lifecycle
  - golden coverage proves single-incident investigation, local depletion reporting, and same-place sibling isolation

**Step 13.5 Wave 4** (parallel, golden-perf-derived) — ✅ COMPLETED:
- **S29**: Planning State Structural Sharing — ✅ COMPLETED
  - replaced clone-per-expansion `PlanningState` / step history with `SharedMap` / `SharedSet` / `SharedVec` copy-on-write sharing
  - benchmark closeout confirmed a strong default-budget gain, with mixed higher-budget end-to-end results depending on scenario shape
  - pure Principle 11 optimization; behavioral verification stayed green
- **S30**: AI Runtime Save/Load Parity — ✅ COMPLETED
  - persisted AI runtime state now survives save/load without breaking determinism coverage
  - the delivered architecture keeps `worldwake-sim` save-only and makes validated restore an AI-owned boundary
  - archived spec: `archive/specs/S30-ai-runtime-save-load-parity.md`
- **S31**: Goal-Aware Exhaustion Invalidation — ✅ COMPLETED
  - per-goal invalidation conditions and baseline snapshots now replace the coarse dirty-bit reset path
  - eliminates unrelated commodity over-invalidation while restoring needs-driven goal re-entry when the relevant baseline changes
  - removes `EXHAUSTION_SKIP_TTL` in favor of condition-based invalidation, while preserving explicit budget-exhaustion retry semantics
  - archived spec: `archive/specs/S31-goal-aware-exhaustion-invalidation.md`

Dependency graph:
```
S30 ✅ ──→ S31 ✅
```

**Step 13.5 Wave 5** (AI architecture review-derived):
- **S33**: Opportunity-Scoped Goal Identity — ✅ COMPLETED
  - separated desire-level identity (`GoalKey`) from opportunity-level identity (`OpportunityKey`) across candidate generation, blocker/exhaustion scope, plan binding, ranked admission, and persisted runtime state
  - archived spec: `archive/specs/S33-opportunity-scoped-goal-identity.md`
  - exhaustion and blocking scoped per-opportunity; IntentionFrame persists on desire
  - unblocks S36, S37, S39
- **S34**: General Epistemic Actions — ✅ COMPLETED
  - ask_witness action + grounded-goal stale-evidence barrier contract
  - enables canonical Scenario D (rumor → travel → empty source → replan)
  - archived spec: `archive/specs/S34-general-epistemic-actions.md`
- **S35**: Observable Activity Signals — ✅ COMPLETED
  - perception records co-located agents' active actions as `BelievedActivity`
  - ranking discounts contested resources based on observed competition
  - archived spec: `archive/specs/S35-observable-activity-signals.md`
- **S36**: Declarative Goal Registration — ✅ COMPLETED
  - `GoalDispatchKey` and `GoalDispatchDeclaration` now centralize AI dispatch identity and declaration-owned metadata/strategy routing
  - exhaustive declaration dispatch plus focused tests now guard completeness without parallel shadow identities
  - archived spec: `archive/specs/S36-declarative-goal-registration.md`
- **S37**: Cooldown-Based Exhaustion — ✅ COMPLETED
  - replaced budget-halving backoff with cooldown-based retry at full search depth
  - persisted cooldown retry state across AI runtime save/load with explicit current-format version gating in `worldwake-sim`
  - archived spec: `archive/specs/S37-cooldown-based-exhaustion.md`

Dependency graph:
```
S31 ✅ ──→ S33 ✅
S23 ✅ ──→ S33 ✅
S22 ✅ ──→ S33 ✅
S33 ✅ ──→ S36 ✅
S33 ✅ ──→ S37 ✅
S33 ✅ ──→ S39
S34 ✅ (independent)
S35 ✅ (independent)
S35 ✅ ──→ S38
```

---

### Phase 4: Adaptation & Integration

**Step 14** (parallel):
- **E18**: Bandit Camp Dynamics
  - needs ~~S02~~, E16
- **E19**: Guard & Patrol Adaptation
  - needs ~~S02~~, E16, E16b, E16c, E17
- **E20**: Companion Behaviors
  - needs ~~S02~~ (all deps met)

**Step 15** (needs E18–E20):
- **E22**: Scenario Integration & Soak Tests

#### Phase 4 Gate
- [ ] All T20–T32 pass
- [ ] 100-seed soak test with zero invariant violations
- [ ] Replay consistency verified
- [ ] Causal depth ≥ 4 across ≥ 3 subsystems for all 4 exemplar scenarios

---

### Phase 4+: Economy Deepening & AI Preferences

**Step 16** (parallel after E22):
- **S04**: Merchant Selling Market Presence (needs E14)
- **S05**: Merchant Stock Storage & Stalls (needs S04, S01, E16c)
- **S06**: Commodity Opportunity Valuation (needs S04, benefits from S10)
- **S10**: Bilateral Trade Negotiation (all deps met — E11, E14 completed; can be scheduled earlier)
- **S38**: Learned Route and Source Preferences (needs S35, S33)
  - per-agent `RouteExperience` and `SourceReliability` components from action outcomes
  - ranking adjustments for route danger and source failure history
  - `PreferenceProfile` provides per-agent diversity
- **S39**: Limited Side-Benefit Plan Scoring (needs S33)
  - post-search side-benefit detection at plan destinations
  - tie-breaking bonus for plans that also satisfy secondary goals
  - `side_benefit_weight` on `UtilityProfile` for per-agent diversity

#### Final Acceptance
- All Phase 4 gate criteria plus:
- [ ] Merchants autonomously sell at markets with stock storage
- [ ] Commodity opportunity valuation drives trade route decisions
- [ ] Economy sustains 100+ tick soak without conservation violations

---

## Active Spec Inventory

All specs in `specs/` must appear exactly once in this order. Completed/archived specs live in `archive/specs/`.

E17 is intentionally absent from the table below because its completed spec now lives in `archive/specs/E17-crime-theft-justice.md`.

| Spec | Phase | Step | Dependencies |
|------|-------|------|-------------|
| ~~`S13-political-emergence-golden-suites.md`~~ | 3 | 13 | ✅ COMPLETED |
| ~~`S20-structural-cleanup.md`~~ | 3+ | 13.5 W0 | ✅ COMPLETED |
| ~~`S26-planner-conformance-tests.md`~~ | 3+ | 13.5 W0 | ✅ COMPLETED |
| ~~`S21-promote-causal-runtime-state.md`~~ | 3+ | 13.5 W1 | ✅ COMPLETED |
| ~~`S23-refined-blocked-intents.md`~~ | 3+ | 13.5 W1 | ✅ COMPLETED |
| ~~`S22-generalized-intention-frames.md`~~ | 3+ | 13.5 W2 | ✅ COMPLETED |
| ~~`S24-typed-invalidation-domains.md`~~ | 3+ | 13.5 W2 | ✅ COMPLETED |
| ~~`S25-feasibility-sketching.md`~~ | 3+ | 13.5 W2 | ✅ COMPLETED |
| ~~`S28-knowledge-path-traces.md`~~ | 3+ | 13.5 W2 | ✅ COMPLETED |
| ~~`S29-planning-state-structural-sharing.md`~~ | 3+ | 13.5 W4 | ✅ COMPLETED |
| `E18-bandit-dynamics.md` | 4 | 14 | E16, S02 |
| `E19-guard-patrol.md` | 4 | 14 | E16, E16b, E16c, ~~S02~~, E17 |
| `E20-companion-behaviors.md` | 4 | 14 | S02 |
| `E22-integration-soak-tests.md` | 4 | 15 | E18, E19, E20 |
| `S04-merchant-selling-market-presence.md` | 4+ | 16 | E14 |
| `S05-merchant-stock-storage-and-stalls.md` | 4+ | 16 | S04, S01, E16c |
| `S06-commodity-opportunity-valuation.md` | 4+ | 16 | S04 |
| `S10-bilateral-trade-negotiation.md` | 4+ | 16 | E11, E14 (all met) |
| ~~`S33-opportunity-scoped-goal-identity.md`~~ | 3+ | 13.5 W5 | ✅ COMPLETED |
| ~~`S34-general-epistemic-actions.md`~~ | 3+ | 13.5 W5 | ✅ COMPLETED |
| ~~`S35-observable-activity-signals.md`~~ | 3+ | 13.5 W5 | ✅ COMPLETED |
| ~~`S36-declarative-goal-registration.md`~~ | 3+ | 13.5 W5 | ✅ COMPLETED |
| `S38-learned-route-source-preferences.md` | 4+ | 16 | S35, S33 |
| `S39-limited-side-benefit-plan-scoring.md` | 4+ | 16 | S33 |

## Crate Dependency Graph

```text
worldwake-core:    (no internal deps)
worldwake-sim:     depends on worldwake-core
worldwake-systems: depends on worldwake-core, worldwake-sim
worldwake-ai:      depends on worldwake-core, worldwake-sim, worldwake-systems
worldwake-cli:     depends on worldwake-core, worldwake-sim, worldwake-systems, worldwake-ai
```

## Phase Summary

| Phase | Specs | Goal | Status |
|-------|-------|------|--------|
| 1: World Legality | E01–E08 | Deterministic world with conservation | ✅ COMPLETED |
| FND-01 | FND01-001–005 | Phase 1 foundations alignment | ✅ COMPLETED |
| 2: Emergent Economy | E09–E13 | Agents autonomously survive | ✅ COMPLETED |
| E21 | E21 | CLI & human control | ✅ COMPLETED |
| FND-02 | FND02-001–006 | Phase 2 foundations alignment | ✅ COMPLETED |
| 3: Information & Politics | E14–E17, E15b, E15c, E16b, E16c, S01–S03, S07–S09, S11–S19, S32, S16b-golden | Information propagates, offices transfer | IN PROGRESS (E14, E15b, E15c, E16, E16b, E16c, E16d, E17, S01, S02, S03, S07, S08, S09, S11, S12, S13, S14, S15, S16, S17, S18, S19, S32, S16b-golden complete; gate items `T10`/`T11`/`T25` remain open) |
| 3+: AI Architecture Overhaul | S20–S37 | Honest causal state, general intentions, refined diagnostics, planning performance, opportunity identity, epistemic actions, observable activity, declarative registration, cooldown exhaustion | ✅ COMPLETED |
| 4: Adaptation & Integration | E18–E20, E22 | Full integration, all scenarios | PENDING |
| 4+: Economy & AI Preferences | S04–S06, S10, S38–S39 | Merchant economy depth, learned preferences, side-benefit scoring | PENDING |
