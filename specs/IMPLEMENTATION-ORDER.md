# Implementation Order & Dependency Graph

## Completed Work

Phases 1–6 (E01–E22, FND-01, FND-02, S01–S58) completed.
See `archive/` for detailed completion records.

Completed Phase 7 specs:
- `S59: Expectation and Obligation Substrate` — archived at [archive/specs/S59-expectation-obligation-substrate.md](/home/joeloverbeck/projects/worldwake/archive/specs/S59-expectation-obligation-substrate.md). Time-bounded expectations, overdue detection, search/rescue actions, last-seen propagation. Golden coverage: Scenarios 120–125 in `golden_expectation.rs`.
- `S69: Goal Dispatch Consolidation` — completed in-place (adjunct infrastructure spec).
- `S70: Belief Store Query Encapsulation` — added 7 accessor/mutation methods to `AgentBeliefStore`, migrated ~24 direct field accesses in `perception.rs` to use the new API.
- `S75: Belief View Domain Decomposition` — archived at [archive/specs/S75-belief-view-domain-decomposition.md](/home/joeloverbeck/projects/worldwake/archive/specs/S75-belief-view-domain-decomposition.md). Decomposed `RuntimeBeliefView` into domain sub-traits, restructured `SnapshotEntity` into domain sub-structs, and replaced the old `GoalBeliefView` macro delegation path with bridge traits plus blanket impls while preserving the facade boundary.
- `S77: Belief Capacity Prioritization` — archived at [archive/specs/S77-belief-capacity-prioritization.md](/home/joeloverbeck/projects/worldwake/archive/specs/S77-belief-capacity-prioritization.md). Added `believed_kind` capture/preservation, tiered claim/entity eviction, removed the `SceneEvidence` gate on current-place observation, and corrected the downstream `e15` proof to the live contract.
- `S71: Event Log Delta Compaction` — replaced full `ComponentValue` snapshots in `ComponentDelta::Set` with compact `BeliefStoreDiff` via new `CompactSet` variant for `AgentBeliefStore` updates. Reduces per-event memory from ~300 KB to ~1-5 KB. Archived at [archive/specs/S71-event-log-delta-compaction.md](/home/joeloverbeck/projects/worldwake/archive/specs/S71-event-log-delta-compaction.md).
- `S72: Event Log Epoch Compaction` — periodic World checkpoints on `EventLog` with `state_deltas` stripping for bounded RAM. `CheckpointData`, `compaction_interval` on `EventLog`, `compact_event_log` SystemFn, `ScenarioDef.compaction_interval` (default: 50). Verification adapted for checkpoint-based reconstruction. Archived at [archive/specs/S72-event-log-epoch-compaction.md](/home/joeloverbeck/projects/worldwake/archive/specs/S72-event-log-epoch-compaction.md).
- `S80: Exploration Drive` — archived at [archive/specs/S80-exploration-drive.md](/home/joeloverbeck/projects/worldwake/archive/specs/S80-exploration-drive.md). Landed `ExplorationProfile`, live `ExploreLocation` goal generation/planning under self-care fallback rules, bounded consecutive exploration tracking, and dedicated golden coverage in `golden_exploration.rs` plus refreshed generated golden docs.
- `S84: ShareBelief Operator Fix` — archived at [archive/specs/S84-share-belief-operator-fix.md](/home/joeloverbeck/projects/worldwake/archive/specs/S84-share-belief-operator-fix.md). Fixed planning-snapshot evidence-listener place indexing for ShareBelief/Tell affordance search, confirmed root-omission diagnostics already owned the planner traceability slice, and validated the existing ShareBelief golden coverage.
- `S90: Mandatory Tactical Scoping` — archived at [archive/specs/S90-mandatory-tactical-scoping.md](/home/joeloverbeck/projects/worldwake/archive/specs/S90-mandatory-tactical-scoping.md). Removed the stale evidence-empty exploration suppression, corrected same-goal tactical explore classification/fail-fast boundaries, and added the D3 candidate-count safety valve plus focused threshold coverage.
- `S94: Commodity-Relevance Candidate Pruning` — archived at [archive/specs/S94-commodity-relevance-candidate-pruning.md](/home/joeloverbeck/projects/worldwake/archive/specs/S94-commodity-relevance-candidate-pruning.md). Added planner-level target-commodity extraction, root candidate pruning against the active commodity contract, `CommodityIrrelevant` decision-trace surfacing, and rewrote the budget-exhaustion golden family into zero-ignored honest post-filter regressions.
- `S96: Obligation Satiation` — archived at [archive/specs/S96-obligation-satiation.md](/home/joeloverbeck/projects/worldwake/archive/specs/S96-obligation-satiation.md). Landed `ObligationSatiationProfile` / `ObligationExecutionTracker`, runtime and scenario carriage, commit-time tracker updates for obligation actions, satiation-aware ranking dampening, and Scenario 144 golden coverage proving survival needs override saturated notice posting.
- `S98: Observer Affordance-Change Detection` — archived at [archive/specs/S98-observer-affordance-change-detection.md](/home/joeloverbeck/projects/worldwake/archive/specs/S98-observer-affordance-change-detection.md). Added observer-only affordance-set change detection between consecutive planning snapshots, Section 7 report emission for affordance appearance/disappearance, and focused observer-bin coverage for the helper and render path.
- `S99: Commodity Validation Helper Extraction` — archived at [archive/specs/S99-commodity-validation-helpers.md](/home/joeloverbeck/projects/worldwake/archive/specs/S99-commodity-validation-helpers.md). Consolidated duplicated `ensure_accessible_quantity` and `resolve_controlled_lots` helpers into shared internal `commodity_support` helpers in `worldwake-systems` without changing behavior.
- `S100: Tiered Belief Retention for Infrastructure Entities` — archived at [archive/specs/S100-belief-persistence-visited-places.md](/home/joeloverbeck/projects/worldwake/archive/specs/S100-belief-persistence-visited-places.md). Added `PerceptionProfile.infrastructure_retention_ticks`, tiered time-based retention for infrastructure entities/claims in `AgentBeliefStore`, aligned the dependent AI proof surfaces to the new lawful retention contract, and updated the authored CLI evaluation scenario profiles plus focused CLI load/spawn coverage.
- `S101: Activation-Based Belief Decay` — archived at [archive/specs/S101-activation-based-belief-decay.md](/home/joeloverbeck/projects/worldwake/archive/specs/S101-activation-based-belief-decay.md). Replaced the old hard-cap belief-retention path with activation-based retention over retained presentation history, landed confidence-threshold claim pruning and item-only need salience, and closed the remaining golden/doc fallout through the archived `S101ACTBASBEL-001` through `S101ACTBASBEL-005` ticket chain.
- `S102: Frontier-Aware Need-Driven Exploration` — archived at [archive/specs/S102-frontier-aware-exploration.md](/home/joeloverbeck/projects/worldwake/archive/specs/S102-frontier-aware-exploration.md). Landed the exploration follow-on for repeated acquisition budget exhaustion, multi-hop BFS frontier selection, exploration-chain belief reinforcement, per-agent exploration profile tuning, authored scenario support, and end-to-end golden coverage plus generated golden-doc refresh.
- `S105: Observation Salience Filtering` — archived at [archive/specs/S105-observation-salience-filtering.md](/home/joeloverbeck/projects/worldwake/archive/specs/S105-observation-salience-filtering.md). Landed `PerceptionProfile.observation_budget` with authored-input compatibility, deterministic same-place observation priority plus budget truncation in `collect_direct_local_observation_batch`, focused unit proof for non-place truncation / need-based non-Waste boosting, and Scenario 341 golden coverage for bounded waste visibility under reduced observation budgets.
- `S106: Ground Item Decay` — archived at [archive/specs/S106-ground-item-decay.md](/home/joeloverbeck/projects/worldwake/archive/specs/S106-ground-item-decay.md). Landed `GroundSince`, persisted `CommodityDecayMap`, `EventTag::ItemDecay`, `SystemId::ItemDecay`, live decay archival after `EvidenceDecay`, and Scenario 342 golden proof of bounded Waste steady state plus regenerated golden inventory/detail docs.
- `S107: Proactive Diversification Exploration` — archived at [archive/specs/S107-proactive-diversification.md](/home/joeloverbeck/projects/worldwake/archive/specs/S107-proactive-diversification.md). Landed `DiversificationProfile`, passive place-visit tracking, proactive `ExploreLocation` emission/ranking with need-slack and cooldown gating, authored scenario support, and golden exploration coverage plus refreshed generated golden docs.
- `S108: Per-Action Binding Strictness` — archived at [archive/specs/S108-per-action-binding-strictness.md](/home/joeloverbeck/projects/worldwake/archive/specs/S108-per-action-binding-strictness.md). Landed authoritative `BindingStrictness` on `ActionDef`, the dispatch-side strictness gate plus `ExactIdentityRequired` failure surface, the planner-contract correction that preserves same-target revalidation helpers, decision-trace strictness snapshots, and hybrid golden end-to-end proof for exact-identity refusal plus fungible fallback.
- `S120: Survival Critical-Window Forensics` — archived at [archive/specs/S120-survival-critical-window-forensics.md](/home/joeloverbeck/projects/worldwake/archive/specs/S120-survival-critical-window-forensics.md). Landed the shared runtime forensics model and extractor in `worldwake-ai`, golden-harness forensic helpers and focused proof, observer Section 9 rendering plus `--critical-window-top-n`, and the survival-debugging documentation updates.
- `S104: Survival Baseline Recovery` — archived at [archive/specs/S104-survival-baseline-recovery.md](/home/joeloverbeck/projects/worldwake/archive/specs/S104-survival-baseline-recovery.md). Landed the survival-baseline recovery slice: golden triage, TellProfile profile-gating cleanup, the authored `survival-baseline.ron` scenario, planner cleanup for the remaining survival-path `ProduceCommodity` budget exhaustion, and the Layer 0 survival golden proof. The later Layer 1–3 rebuild wave was intentionally not pursued.

Completed adjunct specs:
- `S67: Golden E2E Gaps — S59` archived at [archive/specs/S67-golden-gaps-S59.md](/home/joeloverbeck/projects/worldwake/archive/specs/S67-golden-gaps-S59.md).
- `S68: Goal-Switch Contention Cleanup` archived at [archive/specs/S68-goal-switch-contention-cleanup.md](/home/joeloverbeck/projects/worldwake/archive/specs/S68-goal-switch-contention-cleanup.md). Golden E2E proof: Scenario 123 in `golden_production.rs`.
- `S73: Planning Snapshot Entity Relevance` archived at [archive/specs/S73-planning-snapshot-entity-relevance.md](/home/joeloverbeck/projects/worldwake/archive/specs/S73-planning-snapshot-entity-relevance.md). Added goal-aware snapshot filtering, per-place entity caps, and truthful soak telemetry/validation alignment for the planning-cost surface.
- `S74: Intention Commitment Under Needs Fluctuation` archived at [archive/specs/S74-intention-commitment-under-needs-fluctuation.md](/home/joeloverbeck/projects/worldwake/archive/specs/S74-intention-commitment-under-needs-fluctuation.md). Replaced the planning-path top-2 continuation heuristic with per-agent margin-based commitment, fixed the exposed same-goal merchant continuity regression, and corrected the soak baseline/spec validation handoff.
- `S76: Golden E2E Gaps — Simulation Observer Report` archived at [archive/specs/S76-golden-gaps-simulation-observer.md](/home/joeloverbeck/projects/worldwake/archive/specs/S76-golden-gaps-simulation-observer.md). Landed Scenarios 126–129 across `golden_simulation_gaps.rs`, `golden_perception_exposure.rs`, and `golden_reasoning_diversity.rs`, with the S76-D contract narrowed to the strongest live same-state `eat`/`drink` utility-divergence proof.
- `S78: Observer Failed-Plan Diagnostics` archived at [archive/specs/S78-observer-failed-plan-diagnostics.md](/home/joeloverbeck/projects/worldwake/archive/specs/S78-observer-failed-plan-diagnostics.md). Added failed-plan depth/candidate/location diagnostics, a bounded planning-time `TargetBeliefPresence` trace carrier, and live observer breakdown counts for missing target beliefs.
- `S81: Golden E2E Gaps — Simulation Remediation` archived at [archive/specs/S81-golden-gaps-simulation-remediation.md](/home/joeloverbeck/projects/worldwake/archive/specs/S81-golden-gaps-simulation-remediation.md). Landed the death traceability substrate (`DeathCause`, `DeadAt.cause`, `EventTag::Death`), authoritative need-based mortality and death-event tagging, and Scenarios 130–132 in `golden_simulation_gaps.rs`.
- `S85: Observer Behavioral Enrichment` — archived at [archive/specs/S85-observer-behavioral-enrichment.md](/home/joeloverbeck/projects/worldwake/archive/specs/S85-observer-behavioral-enrichment.md). Landed the observer-only enrichment wave: death tick/cause display, depth-0 frontier-exhaustion reason formatting, behavioral-transition need snapshots, post-travel/final affordance snapshots, and clearer unknown-location rendering for believed place entities.
- `S89: Universal Two-Phase Planning` — archived at [archive/specs/S89-universal-two-phase-planning.md](/home/joeloverbeck/projects/worldwake/archive/specs/S89-universal-two-phase-planning.md). Removed the old two-phase whitelist, landed `TravelToGoal` tactical scoping plus tactical-goal traceability for true strategic destination stages, added representative regression coverage, and narrowed exploration fallback into a separate bounded `Explore` barrier contract.

---

## Phase 7: Consequence Carriers

Derived from external gameplay assessment (`brainstorming/prioritary-gameplay-systems.md`) validated against the actual codebase and `docs/FOUNDATIONS.md`. These specs add missing carriers of consequence that widen the causal graph — expectation tracking, persistent sites, predator ecology, boundary processes, contested justice, scarcity response, social aftermath, and settlement decline.

### Dependency Graph

```text
S59 ✅                    S60 (independent)     S62 (independent)     S69 ✅     S70 ✅     S75 ✅     S77 ✅ archived     S78 ✅ archived     S76 ✅ archived
     │                     │
     │                     ├── S61 (needs S60 for dens)
     ├── S63 (needs S59 ✅)│
     │                     │
S59 ─┤                     │
S63 ─┼── S65 (needs S59 ✅, S63)
     │
S62 ──── S64 (needs S62 for boundary pressure)
     │
S60 ─┤
S64 ─┼── S66 (needs S60, S64, S65)
S65 ─┘
```

### Active Execution Steps

**Wave 1** (parallel, no deps):
- **S59**: ✅ COMPLETED — Expectation and Obligation Substrate — time-bounded expectations, overdue detection, search/rescue actions, last-seen propagation. Golden coverage: Scenarios 120–125.
- **S60**: Persistent Site Occupancy — site profiles with sublocations, occupancy claims, site traces, BanditCamp migration
- **S62**: Boundary Processes and Remote Shocks — source regions, boundary channels, scheduled inflows, disruption mechanics
- **S69**: ✅ COMPLETED — Goal Dispatch Consolidation — consolidated GoalFamilyPolicy and progress barrier ops into GoalDispatchDeclaration; expanded GoalDispatchKey with payload-aware ShareBelief/PostNotice variants
- **S70**: Belief Store Query Encapsulation — add missing accessor/mutation methods to `AgentBeliefStore`, replace ~29 direct field accesses in `perception.rs` with API calls
- **S75**: ✅ COMPLETED — Belief View Domain Decomposition — archived at [archive/specs/S75-belief-view-domain-decomposition.md](/home/joeloverbeck/projects/worldwake/archive/specs/S75-belief-view-domain-decomposition.md). Decomposed the monolithic belief views into domain sub-traits, restructured `SnapshotEntity`, and completed the `GoalBeliefView` facade cleanup.

**Wave 2** (after Wave 1):
- **S61**: Predator Ecology and Dens — predator agents with territory, hunger-driven roaming, den habitation, carcass/track evidence
  - depends on S60 (dens use site occupancy model)
- **S63**: Contested Evidence and Warrants — warrants, detention, case records, alibi, evidence contest, wrongful-accusation correction
  - depends on S59 ✅ (dependency satisfied)

**Wave 3** (after Wave 2):
- **S64**: Scarcity Response — Debt, Rationing, and Substitution — borrowing/lending, ration orders, hoarding, sale refusal, substitute purchasing
  - depends on S62 (boundary shocks create the upstream shortage pressure)
- **S65**: Social Aftermath Memory — provenance-tracked grudges, gratitude, kin bonds, revenge, protection, favoritism
  - depends on S59 ✅ (rescue creates gratitude edges), S63 (wrongful accusation creates grudges)

**Wave 4** (after Wave 3):
- **S66**: Settlement Decline and Reoccupation — household departure, facility closure, building vacancy, squatter reoccupation, institutional degradation
  - depends on S60 (vacant buildings as occupyable sites), S64 (scarcity pressure drives departure), S65 (social bonds anchor or repel)

### Adjunct Wave: Simulation Observer Remediations

Derived from simulation observer report (`reports/simulation-remediation.md`) validated against
the actual codebase. S77 fixed the root cause (belief capacity eviction and current-place observation retention), S76 added golden coverage
for the observed pathologies, and S78 enhanced observer diagnostics.

```text
S77 ✅ archived ──┐
                  ├── S76 ✅ archived
S78 ✅ archived ──┘
```

**Completed adjunct wave**:
- **S77**: ✅ COMPLETED — Belief Capacity Prioritization — archived at [archive/specs/S77-belief-capacity-prioritization.md](/home/joeloverbeck/projects/worldwake/archive/specs/S77-belief-capacity-prioritization.md). Added `believed_kind` capture/preservation, tiered claim/entity eviction, removed the `SceneEvidence` gate on place observation, and corrected the downstream `e15` proof to the live contract.
- **S78**: ✅ COMPLETED — Observer Failed-Plan Diagnostics — archived at [archive/specs/S78-observer-failed-plan-diagnostics.md](/home/joeloverbeck/projects/worldwake/archive/specs/S78-observer-failed-plan-diagnostics.md). Added failed-plan depth/candidate/location diagnostics, bounded planning-time target-belief tracing, and observer frequency breakdowns.
- **S76**: ✅ COMPLETED — Golden Gaps — Simulation Observer Report — archived at [archive/specs/S76-golden-gaps-simulation-observer.md](/home/joeloverbeck/projects/worldwake/archive/specs/S76-golden-gaps-simulation-observer.md). Added the observer-gap golden suite for remote travel fallback, bounded idle under remote scarcity, perception retention for resource sources, and same-state utility-profile divergence.

### Adjunct Wave: Simulation Remediation Fixes

Derived from simulation remediation report (`reports/simulation-remediation.md`) validated against
the actual codebase and `docs/FOUNDATIONS.md`. The archived S79 spec fixed the harvest-to-consume affordance
chain gap, the archived S81 spec landed the remediation golden coverage plus death traceability substrate/runtime,
and the archived S80 spec closed the exploration-drive follow-up for agents trapped by geographic ignorance.

```text
S79 ✅ archived
S81 ✅ archived
S80 ✅ archived
```

- **S79**: ✅ COMPLETED — Resource-Source Consumption Affordances — archived at `archive/specs/S79-resource-source-consumption-affordances.md`
- **S81**: ✅ COMPLETED — Golden Gaps — Simulation Remediation — archived at [archive/specs/S81-golden-gaps-simulation-remediation.md](/home/joeloverbeck/projects/worldwake/archive/specs/S81-golden-gaps-simulation-remediation.md). Landed GT-1 multi-agent convergence, GT-2 death traceability, and GT-3 colocated harvest-to-consume proof, plus the supporting death traceability substrate/runtime slices.
- **S80**: ✅ COMPLETED — Exploration Drive — archived at [archive/specs/S80-exploration-drive.md](/home/joeloverbeck/projects/worldwake/archive/specs/S80-exploration-drive.md). Landed exploration pressure from unmet needs + limited geographic beliefs, per-agent `ExplorationProfile`, self-care fallback dispatch wiring, and exploration-specific golden coverage.

### Adjunct Wave: Simulation Remediation Phase 2

Derived from re-run of simulation observer report (`reports/simulation-remediation.md`) validated against
the actual codebase and `docs/FOUNDATIONS.md` post-S78/S79/S80/S81. Addresses the remaining waste
accumulation dampener gap, ShareBelief frontier-exhaustion, and observer diagnostic enrichments.

```text
S82 ✅ archived
S83 ✅ archived
S84 ✅ archived
S85 ✅ archived
```

**Wave**:
- **S82**: ✅ COMPLETED — Waste Disposal and Inventory Management — archived at [archive/specs/S82-waste-disposal-inventory-management.md](/home/joeloverbeck/projects/worldwake/archive/specs/S82-waste-disposal-inventory-management.md). Landed `drop_item`, `FreeCarryCapacity`, `DisposalProfile`, planner/candidate/ranking integration, CLI disposal-profile override support, and disposal-cycle golden coverage with deterministic replay.
- **S83**: ✅ COMPLETED — Belief-Informed Candidate Pruning — archived at [archive/specs/S83-belief-informed-candidate-pruning.md](/home/joeloverbeck/projects/worldwake/archive/specs/S83-belief-informed-candidate-pruning.md). Landed belief-gated acquisition-place filtering and candidate-trace counters for reachable-vs-filtered places. The temporary `speculative_acquisition` profile support introduced there was later removed by `SPECACQRMV-001` because it violated the live belief-grounded planning contract.
- **S84**: ✅ COMPLETED — ShareBelief Operator Fix — archived at [archive/specs/S84-share-belief-operator-fix.md](/home/joeloverbeck/projects/worldwake/archive/specs/S84-share-belief-operator-fix.md). Landed the snapshot place-indexing fix for evidence listeners, then closed the proposed pruning/diagnostic/golden follow-up slices against the stronger live planner and golden contracts.
- **S85**: ✅ COMPLETED — Observer Behavioral Enrichment — archived at [archive/specs/S85-observer-behavioral-enrichment.md](/home/joeloverbeck/projects/worldwake/archive/specs/S85-observer-behavioral-enrichment.md). Landed the observer-only enrichment wave: death tick/cause display, depth-0 frontier-exhaustion reason formatting, behavioral-transition need snapshots, post-travel/final affordance snapshots, and clearer unknown-location rendering for believed place entities.

### Adjunct Wave: Simulation Remediation Phase 3

Derived from 2026-04-10 post-S85 simulation observer re-run and deep research into GOAP scaling
(LAMA planner, LGOAP, Fast Downward deferred evaluation) validated against
the actual codebase and `docs/FOUNDATIONS.md`. S83's belief-gated acquisition filtering was insufficient —
1400–2600 candidates still exhaust the planner budget at depth 0, blocking all multi-location plans.
Root cause: flat A* forward search with 1400+ branching factor is architecturally unsound.

```text
S88 ✅ archived (independent, supersedes S86 + S87)
S90 ✅ archived (depends on S88 + S89)
```

**Wave**:
- **S88**: ✅ COMPLETED — Two-Phase Landmark-Guided Planning — archived at [archive/specs/S88-two-phase-landmark-planning.md](/home/joeloverbeck/projects/worldwake/archive/specs/S88-two-phase-landmark-planning.md). Landed the belief-backed strategic planner, landmark-guided tactical planning substrate, dual-frontier search, live two-phase integration for remote `TreatWounds` and `ProduceCommodity`, decision-trace enrichment, and golden coverage for two-phase planner behavior and landmark-depth diversity. Superseded S86 (heuristic candidate scoring) and S87 (observer diagnostic gaps; observer tooling may be reconsidered as a future spec).
- **S90**: ✅ COMPLETED — Mandatory Tactical Scoping — archived at [archive/specs/S90-mandatory-tactical-scoping.md](/home/joeloverbeck/projects/worldwake/archive/specs/S90-mandatory-tactical-scoping.md). Landed evidence-directed tactical exploration, barrier-required vs fallback exploration classification, bounded fail-fast for missing tactical goals on the barrier-required slice, and the per-agent candidate-count safety valve plus focused threshold proof.

### Completed Adjunct: Planner Pathology Fixes

Derived from the archived planner-pathology golden proof surface in [archive/specs/S91-planner-pathology-golden-tests.md](/home/joeloverbeck/projects/worldwake/archive/specs/S91-planner-pathology-golden-tests.md), validated against the `cli-evaluation.ron` observer substrate and `docs/FOUNDATIONS.md`.

```text
S91 ✅ archived
  └── S92 ✅ archived
  └── S93 ✅ completed
        └── S94 ✅ completed
```

- **S91**: ✅ COMPLETED — archived at [archive/specs/S91-planner-pathology-golden-tests.md](/home/joeloverbeck/projects/worldwake/archive/specs/S91-planner-pathology-golden-tests.md). Landed the planner-pathology golden proof surface in `golden_planner_pathology.rs`; Scenario 142 now proves the remote-water acquisition failure is fixed, Scenario 143 now proves the `FreeCarryCapacity` zero-step loop is fixed, and the third intended Guard Theron pathology was closed as a stale non-reproducible observer claim on the current branch.
- **S92**: ✅ COMPLETED — [archive/specs/S92-free-carry-capacity-zero-step-loop-fix.md](/home/joeloverbeck/projects/worldwake/archive/specs/S92-free-carry-capacity-zero-step-loop-fix.md). Fixes the `FreeCarryCapacity -> GoalSatisfied[steps=0]` contract mismatch and flips Scenario 143 from bug reproduction to fix proof.
- **S93**: ✅ COMPLETED — [archive/specs/S93-budget-exhaustion-snapshot-goldens.md](/home/joeloverbeck/projects/worldwake/archive/specs/S93-budget-exhaustion-snapshot-goldens.md). Added the dedicated budget-exhaustion snapshot golden suite in `golden_budget_exhaustion_snapshots.rs`, including the four remaining `AcquireCommodity` observer reproductions, the two `TreatWounds` observer reproductions, and ignored phase-2 `Found` follow-ups. The early Thornwall water cases replay `cli-evaluation.ron` to the observer ticks under seed `7777`; the remaining cases use focused file-local reconstruction on the shared snapshot harness.
- **S94**: ✅ COMPLETED — [archive/specs/S94-commodity-relevance-candidate-pruning.md](/home/joeloverbeck/projects/worldwake/archive/specs/S94-commodity-relevance-candidate-pruning.md). Added `GoalKindPlannerExt::target_commodity()`, commodity-relevance pruning at the root candidate stage with active tactical commodity handling, `CommodityIrrelevant` trace recording, and the zero-ignored post-filter rewrite of `golden_budget_exhaustion_snapshots.rs`.

### Adjunct Wave: GOAP Heuristic Upgrade

Derived from external GOAP architecture assessment (`brainstorming/goap-upgrades.md`) validated against
the actual codebase and `docs/FOUNDATIONS.md`. Of 7 proposals, 6 were rejected as already addressed
(S74/S88/S89/S39/S91-S94) or YAGNI. S95 is the one accepted upgrade: FF-style relaxed-plan heuristic
for better tactical search guidance.

```text
S95 ✅ archived
```

**Wave**:
- **S95**: ✅ COMPLETED — archived at [archive/specs/S95-relaxed-plan-heuristic.md](/home/joeloverbeck/projects/worldwake/archive/specs/S95-relaxed-plan-heuristic.md). Landed the FF-style relaxed-plan heuristic substrate (`compute_ff_heuristic` / `RelaxedPlanResult`), per-agent `use_ff_heuristic` toggle, live tactical-search `h_ff` and helpful-action integration, decision-trace enrichment, and truthful golden E2E proof at the remote-care planning trace boundary.

### Adjunct Wave: Simulation Remediation Phase 4

Derived from 2026-04-12 post-S95 simulation observer re-run (`reports/simulation-remediation.md`) validated
against the actual codebase and `docs/FOUNDATIONS.md`. Of 11 proposals (4 golden tests, 2 spec changes,
5 tickets), 2 accepted, 2 scoped down, 8 rejected as already addressed by S76-S95 or deferred.
```text
S96 ✅ archived
S97 ✅ archived
S98 ✅ archived
```

**Wave** (parallel, no deps):
- **S96**: ✅ COMPLETED — archived at [archive/specs/S96-obligation-satiation.md](/home/joeloverbeck/projects/worldwake/archive/specs/S96-obligation-satiation.md). Landed `ObligationSatiationProfile` / `ObligationExecutionTracker`, runtime and scenario carriage, obligation-action tracker updates, satiation-aware ranking, and Scenario 144 golden proof that survival needs override saturated notice posting.
- **S97**: ✅ COMPLETED — archived at [archive/specs/S97-post-notice-artifact-ttl.md](/home/joeloverbeck/projects/worldwake/archive/specs/S97-post-notice-artifact-ttl.md). Landed `ArtifactPostingProfile` as a universal agent component, planner-visible profile carriage, runtime candidate-generation TTL provisioning for `PostBounty` / `PostNotice`, CLI scenario support, and the `golden_integration.rs` bounded-population expiry proof with regenerated golden docs.
- **S98**: ✅ COMPLETED — archived at [archive/specs/S98-observer-affordance-change-detection.md](/home/joeloverbeck/projects/worldwake/archive/specs/S98-observer-affordance-change-detection.md). Landed observer-only affordance-set change detection between planning snapshots, Section 7 report emission for affordance appearance/disappearance, and focused observer-bin proof of both helper behavior and final report output.

### Adjunct Wave: Architectural Debt Remediation

Derived from architectural debt analysis (`reports/architectural-debt-2026-04-12-golden_resilience.md`) validated against
the actual codebase and `docs/FOUNDATIONS.md`. Single confirmed finding: commodity validation helper duplication
across 4 action handler modules in `worldwake-systems`.

```text
S99 ✅ archived
```

**Completed wave**:
- **S99**: ✅ COMPLETED — archived at [archive/specs/S99-commodity-validation-helpers.md](/home/joeloverbeck/projects/worldwake/archive/specs/S99-commodity-validation-helpers.md). Consolidated duplicated `ensure_accessible_quantity` (4 copies) and `resolve_controlled_lots` (3 copies) into shared internal `commodity_support` helpers in `worldwake-systems` while preserving behavior and caller-specific underflow messaging.

### Adjunct Wave: Survival Baseline Recovery

Derived from starvation diagnostic (`reports/needs-starvation-diagnostic.md`) which revealed that agents
could not satisfy basic needs in realistic 1440-tick simulations despite passing 349 golden tests.
Root cause: golden tests pre-wired success conditions in sterile environments, never testing whether agents
could bootstrap survival from realistic starting conditions.

**Resolved prerequisite for Phase 7 consequence-carrier specs (S60–S66)**: the survival baseline was proven
before adding more gameplay complexity.

```text
S103 ✅ archived
S104 ✅ archived
```

**Completed wave**:
- **S103**: ✅ COMPLETED — Belief Claim Deduplication and Amortized Pruning — performance fix for the S101 belief system.
- **S104**: ✅ COMPLETED — archived at [archive/specs/S104-survival-baseline-recovery.md](/home/joeloverbeck/projects/worldwake/archive/specs/S104-survival-baseline-recovery.md). Landed golden triage, profile-gating cleanup, the authored survival baseline scenario, survival-path planner cleanup, and the Layer 0 survival regression proof; the draft Layer 1–3 rebuild was intentionally not pursued.

### Adjunct Wave: Perception and Item Lifecycle

Derived from healthy scenario analysis of `survival-baseline.ron` (seed 104004, 1440 ticks, 0 deaths)
revealing LOW-severity architectural gaps in perception budget waste on low-value entities, unbounded
item accumulation, and belief store pollution. External research (aura-nimbus interest management,
DF/RimWorld decay systems, ACT-R activation models) validated both gaps as genuine architectural
deficiencies that would worsen in longer runs or denser scenarios.

```text
S105 ✅ archived
S106 ✅ archived
```

**Wave**:
- **S105**: ✅ COMPLETED — archived at [archive/specs/S105-observation-salience-filtering.md](/home/joeloverbeck/projects/worldwake/archive/specs/S105-observation-salience-filtering.md). Landed `PerceptionProfile.observation_budget`, omitted-field authored-input compatibility, deterministic observation priority plus truncation in `collect_direct_local_observation_batch`, focused unit coverage for same-place budget behavior, and Scenario 341 golden proof of bounded waste retention under reduced observation budgets.
- **S106**: ✅ COMPLETED — archived at [archive/specs/S106-ground-item-decay.md](/home/joeloverbeck/projects/worldwake/archive/specs/S106-ground-item-decay.md). Landed `GroundSince`, persisted `CommodityDecayMap`, `EventTag::ItemDecay`, `SystemId::ItemDecay`, live `item_decay_system`, and Scenario 342 golden proof of bounded Waste steady state with regenerated golden inventory/detail docs.

### Adjunct Wave: Proactive Diversification

Derived from healthy scenario analysis of `survival-scattered.ron` (seed 205005, 1440 ticks, 0 deaths)
revealing that all agents converge on a single 2-location corridor, ignoring reachable alternative
resources. External research (optimal foraging theory, intrinsic motivation, UCB exploration,
ant colony scouting) validated proactive diversification as a well-established mechanism for
population-level resource resilience through per-agent curiosity variation.

```text
S107 ✅ archived
```

**Completed wave**:
- **S107**: ✅ COMPLETED — archived at [archive/specs/S107-proactive-diversification.md](/home/joeloverbeck/projects/worldwake/archive/specs/S107-proactive-diversification.md). Landed `DiversificationProfile`, passive place-visit tracking in `AgentBeliefStore`, proactive `ExploreLocation` emission/ranking with need-slack and cooldown gating, authored scenario support, and `golden_exploration.rs` coverage for proactive discovery, need-pressure veto, and cooldown spacing.

### Adjunct Wave: Survival Baseline Under Contention

Derived from `/brainstorm` session (2026-04-17, `docs/plans/2026-04-17-survival-contested-scenario-design.md`) that triaged whether the two existing survival scenarios (`survival-baseline.ron`, `survival-scattered.ron`) cover the baseline appropriately before layering gameplay-feature scenarios. Finding: neither scenario exercises resource contention, dynamic depletion, or realistic rivalry among more than 3 agents; closing that gap before Phase 7 consequence-carrier feature work preserves the "prove baseline before features" discipline that `/brainstorm` context notes was violated prior to the S104 recovery.

Constraint: the new scenario uses only the profile set already exercised by the baseline / scattered scenarios — no `DiversificationProfile` or other opt-in feature profiles — so the proof is genuinely about baseline survival under contention, not about whether a specific feature works.

- **survival-contested** (scenario + goldens): ✅ LANDED — `scenarios/survival-contested.ron` (seed 306006, 1440 ticks, 4 AI agents, 8 places with chokepoint hub, two capacity-4 wells and a single co-located wash basin). `golden_survival_contested.rs` Scenarios 158–163 prove: all 4 agents alive; both water sources drawn during the run; both camp sides reach food under chokepoint pressure; all 5 self-care families committed; no non-Wash survival-goal budget exhaustion; no stuck idle windows ≥ 40 ticks. `MAX_CRITICAL_RUN_TICKS` landed at 400 (matching scattered — see the golden's comment for the wash-starvation dynamic under contention, which is a legitimate observation for gameplay-feature specs to consider).

### Adjunct Wave: Contested Scenario Remediation

Derived from `/scenario-analysis` of `survival-contested.ron` (seed 306006, 1440 ticks) and the follow-up `/brainstorm` triage (2026-04-17). The scenario surfaced a structural wash-starvation equilibrium: all 4 agents had dirtiness >= 750 permille for 703–901 ticks because dirtiness_weight (625) loses motive scoring to hunger_weight (700–750) and the existing `MAX_CRITICAL_RUN_TICKS=400` golden tolerance knowingly encodes this deferred architectural concern. The brainstorm triage also identified four detection gaps in the observer anomaly suite (geographic convergence, maintenance-cycle starvation, recipe monoculture, sub-threshold acute spikes) and one precision gap in the `STUCK_AGENT` detector. Solutions split between one engine change (drive escalation) and two observer-only refinements.

```text
S116 ✅ archived
S117 ✅ archived
S118 ✅ archived (observer, independent)
```

**Wave** (parallel, no deps among S116/S117/S118):

- **S116**: ✅ COMPLETED — archived at [archive/specs/S116-drive-escalation-sustained-critical.md](/home/joeloverbeck/projects/worldwake/archive/specs/S116-drive-escalation-sustained-critical.md). Landed `DriveEscalationProfile`, extended `DeprivationExposure`, escalation transition event tagging from `needs_system`, motive-score multiplier integration in AI ranking, dedicated drive-escalation golden coverage, and the contested authored survival-bound tightening from 400 to 300.
- **S117**: ✅ COMPLETED — archived at [archive/specs/S117-convergence-maintenance-observer-smells.md](/home/joeloverbeck/projects/worldwake/archive/specs/S117-convergence-maintenance-observer-smells.md). Landed the four observer-side anomaly detectors (`GEOGRAPHIC_CONVERGENCE`, `MAINTENANCE_STARVATION`, `RECIPE_MONOCULTURE`, `ACUTE_NEED_SPIKE`), Section 2 maintenance-rate and recipe-usage tables, the compiled-binary golden suite plus fixtures, and the downstream scenario-analysis doc graduation; the live baseline contract keeps `GEOGRAPHIC_CONVERGENCE` absent on `survival-baseline.ron` while treating the broader smell surface as diagnostic rather than a health-oracle failure.
- **S118**: ✅ COMPLETED — archived at [archive/specs/S118-stuck-agent-detector-active-frame-exclusion.md](/home/joeloverbeck/projects/worldwake/archive/specs/S118-stuck-agent-detector-active-frame-exclusion.md). Landed the observer-only active-frame exclusion for `STUCK_AGENT`, the wash+travel regression, the genuine-idle and `StartFailed` guardrail proofs, and the downstream scenario-analysis caveat simplification; eliminates the 26-tick wash+travel false positive observed on Agent C.

Follow-up tickets (not specs): scenario weight rebalance for `survival-contested.ron`, belief-retention investigation for landmark facilities under memory pressure (pending mid-run belief snapshot capability).

### Adjunct Wave: Survival Stability Hardening

Derived from the 2026-04-18 S116 implementation retrospective after tickets `S116DRIESCSUS-011` and `S116DRIESCSUS-012` exposed 2 distinct stability gaps on the live branch. First, survival goldens could drift away from authored scenario truth by hardcoding health-envelope constants outside the scenario's own `DriveThresholds` and intended invariants. Second, once a long-run authored-critical failure appeared, explaining it required ad hoc ignored reproducers instead of a stable causal read-model. The scenarios were not proven broadly unstable; the missing substrate was canonical proof carriage plus long-run survival forensics.

```text
S119 (independent)
S120 ✅ archived (independent)
S121 (soft depends on S119)
```

**Wave** (parallel, no hard deps):

- **S119**: ✅ COMPLETED — archived at [archive/specs/S119-authored-survival-health-contracts.md](/home/joeloverbeck/projects/worldwake/archive/specs/S119-authored-survival-health-contracts.md). Landed `SurvivalHealthContractDef` and `NeedsActionFamily` on `ScenarioDef`, shared survival-golden harness helpers (`expect_survival_health_contract`, `assert_authored_critical_runs`, `assert_required_self_care_families`, `assert_no_stuck_idle_windows`), authored `survival_health_contract` sections in the three survival scenarios, the contract-presence guard plus dedicated regression test, and the `docs/golden-e2e-testing.md` update. Post-retrofit fallout absorbed by `S119AUTHSURVHC-002` (scattered hunger bound raised to 550) and `S121` (per-need `critical_run_limits` extension).
- **S120**: ✅ COMPLETED — archived at [archive/specs/S120-survival-critical-window-forensics.md](/home/joeloverbeck/projects/worldwake/archive/specs/S120-survival-critical-window-forensics.md). Landed the deterministic derived report surface in `worldwake-ai`, shared golden-harness helpers plus focused proof, observer Section 9 rendering and CLI control, and the canonical debugging/doc guidance for survival critical-window analysis.
- **S121**: ✅ COMPLETED — archived at [archive/specs/S121-per-need-survival-health-contracts.md](/home/joeloverbeck/projects/worldwake/archive/specs/S121-per-need-survival-health-contracts.md). Extended `SurvivalHealthContractDef` with optional per-need `critical_run_limits` (`SurvivalCriticalRunLimitsDef`), taught the shared survival-golden harness to honor per-need overrides via `assert_authored_critical_runs_with_overrides` and `SurvivalCriticalRunLimitOverrides`, retrofitted `scenarios/survival-contested.ron` with a dirtiness-specific override so contested no longer over-asserts unlike self-care families, and documented the richer contract in `docs/golden-e2e-testing.md`. Delivered by `S121PERNEEDSHC-001`.

### Phase 7 Gate

- [ ] All 9 specs reassessed (`/reassess-spec`) and ticket-decomposed
- [ ] Wave 1 specs implemented and passing golden E2E tests
- [ ] Wave 2 specs implemented and passing golden E2E tests
- [ ] Wave 3 specs implemented and passing golden E2E tests
- [ ] Wave 4 specs implemented and passing golden E2E tests
- Note: S59 and S69 from Wave 1 are complete. Wave 1 gate requires S60, S62, and S70 to also be implemented.
- [ ] Canonical regression A (beast starvation → bounty → hunt) fully producible
- [ ] Canonical regression G (false rumor → wrongful accusation → correction) fully producible
- [ ] Canonical regression H (remote shock → local shortage → adaptation) fully producible
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passing
- [ ] Golden E2E coverage for each new spec's core behavior

---

## Phase 8: Belief-First Continual Planning Foundation

Derived from external AI architecture assessment (`brainstorming/worldwake_ai_architecture_review.md`, regenerated 2026-04-17) validated against the actual codebase and `docs/FOUNDATIONS.md`. The assessment post-dates all completed specs through S107. Of ~40+ proposals across 7 sections, 6 accepted and 4 scoped down into 8 specs (6 in Phase 8, 2 deferred to Phase 9). The remaining proposals were rejected as already addressed (unified payload-override validator, emitter-based generation, `shortest_travel_ticks` accessor fence, `SceneEvidence` carriers, `ContentionQueue`/`ContentionGrant`, `ArtifactState`, `ExplorationProfile`/`DiversificationProfile`), as overlapping with drafted Phase 7 specs S60–S66 (evidence extension, route condition state, institution throughput, commitment aftermath), or as YAGNI without concrete scenarios.

Phase 8 runs in parallel with the remaining Phase 7 feature work — Phase 8 touches the AI planner substrate and the event-log schema, while Phase 7 adds new ECS consequence carriers. No direct dependency in either direction.

The travel-fence epistemic regression audit (originally PR-1.11 in the assessment) is folded into S111 as a ticket, not a standalone spec.

### Dependency Graph

```text
S108 (independent)              S111 (independent)
   │
   └── S109 (soft dep on S108)
         │
         └── S110 (soft dep on S109)
                    │
                    ├── S112 (soft dep on S109)
                    └── S113 (soft dep on S109)
```

### Active Execution Steps

**Wave 1** (parallel, no hard deps):
- **S108**: ✅ COMPLETED — archived at [archive/specs/S108-per-action-binding-strictness.md](/home/joeloverbeck/projects/worldwake/archive/specs/S108-per-action-binding-strictness.md). Landed authoritative `BindingStrictness` on `ActionDef`, the dispatch-side strictness gate plus `ExactIdentityRequired` failure surface, the planner-contract correction that preserves same-target revalidation helpers, decision-trace strictness snapshots, and hybrid golden end-to-end proof for exact-identity refusal plus fungible fallback.
- **S111**: Scenario Profile Homogeneity Lints — scenario-load-time lints for profile homogeneity, proactive-exploration-without-curiosity, archetype inheritance; includes the `PlanningSnapshot` accessor-only architecture regression test (PR-1.11 travel-fence audit).

**Wave 2** (after Wave 1):
- **S109**: Typed Discrepancy Taxonomy and BlockedIntentMemory Split — replace `BlockingFact::Unknown`/`AssumptionFailed` with `Discrepancy` enum; split `BlockedIntentMemory` into `DiscrepancyMemory`/`BlockerMemory`/`RepairMemory`/`LearnedOpportunityMemory`; per-class TTL.
  - soft depends on S108 for `MatchOutcome::ExactIdentityRequired` → `Discrepancy::NoLegalBinding`
- **S110**: Authoritative Decision History Events — new `EventTag` variants (`GoalOffered`, `GoalCommitted`, `PlanAdopted`, `PlanInvalidated`, `ExpectationMismatch`, `RepairApplied`, `BlockerRecorded`, `ReplanTriggered`); bounded rejected-alternative summaries.
  - soft depends on S109 for `Discrepancy` payload on `BlockerRecorded`

**Wave 3** (after Wave 2):
- **S112**: Portfolio Planning with Feasibility Probes — replace flat top-N selection with 4-slot portfolio (survival / commitment / economic / information) gated by cheap feasibility probes; low-confidence agents activate the information slot.
  - soft depends on S109 for probe reading of discrepancy/blocker memories
- **S113**: Planner-Facing Belief Envelope — `BeliefValue<T>` / `BeliefSet<T>` wrappers on target-presence / believed-location / believed-stock queries; surface confidence, freshness, status (Certain/Probable/Stale/Disputed/Contradicted), and alternatives.
  - soft depends on S109 for `BeliefStatus::Contradicted` alignment

### Phase 8 Gate

- [ ] All 6 specs reassessed (`/reassess-spec`) and ticket-decomposed
- [ ] Wave 1 specs implemented and passing golden E2E tests
- [ ] Wave 2 specs implemented and passing golden E2E tests
- [ ] Wave 3 specs implemented and passing golden E2E tests
- [ ] Every `ActionDef` registration has explicit `BindingStrictness` (compile-time coverage)
- [ ] No remaining `BlockingFact::Unknown` or `AssumptionFailed` variants (migrated to typed `Discrepancy`)
- [ ] Authoritative decision history events surfaced in observer "Decision History" section for `survival-baseline.ron` and `survival-contested.ron`
- [ ] Portfolio planning golden (infeasible top-N + feasible lower slot) passes
- [ ] Belief envelope golden (stale belief surfaces via `status == Stale`) passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passing

- Note: S108 is complete and archived. The remaining Wave 1 gate item is S111.

---

## Phase 9: Belief-First Continual Planning Structural

Follow-on to Phase 8. Deferred from the Phase 8 scope because these specs depend on the Phase 8 substrate (typed discrepancies, decision-history events, belief envelope) and are significantly larger refactors. Phase 9 specs should be reassessed just before implementation, since the architectural surface may drift after Phase 7 (S60–S66) and Phase 8 land.

### Dependency Graph

```text
S114 (hard deps on S109, S110, S113)
  │
  └── S115 (hard deps on S110, S112, S114)
```

### Active Execution Steps

**Wave 1**:
- **S114**: Plan Step Guards and Expectation Monitoring — annotate `PlannedStep` with `PlanGuard` (required facts, min confidence, invalidators) and `PlanExpectation` (immediate / state / informed / regression); new `expectation_monitor_system` SystemFn; guard breaches emit `ExpectationMismatch` events and typed discrepancies.

**Wave 2** (after Wave 1):
- **S115**: Agenda Manager with Goal Lifecycle — `AgendaState` with `committed`/`pending`/`suspended` entries, each with origin, freshness, revival trigger, kill condition; rename `GroundedGoal` → `GoalOffer`, `RankedGoal` → `AgendaEntry`; new `agenda_tick_system` SystemFn; margin-based commit (S74) reads the agenda manager.

### Phase 9 Gate

- [ ] Both specs reassessed post-Phase-7/8 and ticket-decomposed
- [ ] S114 implemented: guard breaches short-circuit revalidation before affordance matching; `ExpectationMismatch` events appear in event log
- [ ] S115 implemented: committed goals persist across ticks deterministically; pending/suspended surfaced in observer output
- [ ] Deterministic replay of scenarios produces identical agenda state transitions
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passing
