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
- `S109: Typed Discrepancy Taxonomy and BlockerMemory Split` — archived at [archive/specs/S109-typed-discrepancy-taxonomy.md](/home/joeloverbeck/projects/worldwake/archive/specs/S109-typed-discrepancy-taxonomy.md). Replaced `BlockingFact::Unknown` / `AssumptionFailed` with typed `Discrepancy`, split the old blocker memory responsibilities into `DiscrepancyMemory` / `BlockerMemory` / `RepairMemory` / `LearnedOpportunityMemory`, migrated the planning trace to `discrepancy_trace`, removed `unknown_block_ticks`, and completed the scenario/save/generated-doc cleanup.
- `S111: Scenario Profile Homogeneity Lints` — archived at [archive/specs/S111-scenario-homogeneity-lints.md](/home/joeloverbeck/projects/worldwake/archive/specs/S111-scenario-homogeneity-lints.md). Landed scenario-load linting for `ProfileHomogeneity` and `UnreachableExplorationDrive`, `ScenarioDef.scenario_lint_overrides`, load-time enforcement plus CLI `--ignore-lints`, the `PlanningSnapshot` accessor-fence doctests, and the CI sweep over committed `scenarios/*.ron`.
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
- **S125**: ✅ COMPLETED — Institutional Treasuries and Bounty Funding — archived at [archive/specs/S125-institutional-treasuries-and-bounty-funding.md](/home/joeloverbeck/projects/worldwake/archive/specs/S125-institutional-treasuries-and-bounty-funding.md). Landed first-class office funds, bounty reward reservation, and scenario-authored institutional assets for the `survival-justice` bounty extension without personal-funds shortcuts or perception side effects.
  - depended on E17, S45, S51, S59 ✅; optional downstream relationship with S63 remains.

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

S123 (preference-ordering authority) was drafted post-S112-land from the portfolio-planning regression post-mortem. It was not in the original Phase 8 assessment; it enforces the structural invariant that `ranking::compare_ranked_goals` is the single authoritative preference ordering, preventing the class of bug that regressed `golden-survival / baseline` after S112. S123 is sequenced alongside the S122 adjunct as a Phase 8 critical-path closer rather than starting a new phase, because it is a narrow type-system refactor with no runtime-behaviour change expected.

### Dependency Graph

```text
S108 ✅ archived               S111 ✅ archived
   │
   └── S109 ✅ archived
         │
         ├── S110 ✅ archived (soft dep on S109)
         │       │
         │       ├── S112 ✅ archived (soft dep on S109)
         │       │       │
         │       │       └── S123 ✅ archived (hard dep on S112's public signatures)
         │       └── S113 ✅ archived (soft dep on S109, soft dep on S112)
         │
         └── S122 ✅ archived
```

### Active Execution Steps

**Wave 1** (parallel, no hard deps):
- **S108**: ✅ COMPLETED — archived at [archive/specs/S108-per-action-binding-strictness.md](/home/joeloverbeck/projects/worldwake/archive/specs/S108-per-action-binding-strictness.md). Landed authoritative `BindingStrictness` on `ActionDef`, the dispatch-side strictness gate plus `ExactIdentityRequired` failure surface, the planner-contract correction that preserves same-target revalidation helpers, decision-trace strictness snapshots, and hybrid golden end-to-end proof for exact-identity refusal plus fungible fallback.
- **S111**: ✅ COMPLETED — archived at [archive/specs/S111-scenario-homogeneity-lints.md](/home/joeloverbeck/projects/worldwake/archive/specs/S111-scenario-homogeneity-lints.md). Landed scenario-load-time `ProfileHomogeneity` and `UnreachableExplorationDrive` lints, `scenario_lint_overrides`, load-time enforcement plus CLI bypass warnings, the `PlanningSnapshot` accessor-only doctest regression, and the CI sweep over committed `scenarios/*.ron`.

**Wave 2** (after Wave 1):
- **S109**: ✅ COMPLETED — archived at [archive/specs/S109-typed-discrepancy-taxonomy.md](/home/joeloverbeck/projects/worldwake/archive/specs/S109-typed-discrepancy-taxonomy.md). Replaced `BlockingFact::Unknown`/`AssumptionFailed` with typed `Discrepancy`, split the old blocker-memory surface into `DiscrepancyMemory`/`BlockerMemory`/`RepairMemory`/`LearnedOpportunityMemory`, and completed the trace/save/scenario cleanup.
  - soft depends on S108 for `MatchOutcome::ExactIdentityRequired` → `Discrepancy::NoLegalBinding`
- **S110**: ✅ COMPLETED — archived at [archive/specs/S110-decision-history-events.md](/home/joeloverbeck/projects/worldwake/archive/specs/S110-decision-history-events.md). Landed the full authoritative decision-history event family across `worldwake-core`, `worldwake-ai`, `worldwake-sim`, and `worldwake-cli`: core payload/schema ownership, runtime emission at honest AI seams, save/load preservation, and observer Section 3 rendering with committed golden coverage.
  - soft depended on S109 for the `Discrepancy` payload carried by `BlockerRecorded`

**Wave 3** (after Wave 2):
- **S112**: ✅ COMPLETED — archived at [archive/specs/S112-portfolio-planning.md](/home/joeloverbeck/projects/worldwake/archive/specs/S112-portfolio-planning.md). Landed the portfolio substrate and probe integration ticket wave (`S112PORPLAN-002` through `S112PORPLAN-006`): deterministic survival / commitment / economic slot assembly, staged `PortfolioTrace`, feasibility-probe rejection surfacing through decision history, portfolio-led search ordering with `GoalPriorityClass` still gating slot winners, and the generated golden-doc-backed end-to-end proof of rejected `Sleep` and `ReportMissing` slots ahead of plausible `Bake Bread` production.
  - soft depended on S109 for probe reading of discrepancy/blocker memories
- **S113**: ✅ COMPLETED — archived at [archive/specs/S113-belief-envelope.md](/home/joeloverbeck/projects/worldwake/archive/specs/S113-belief-envelope.md). Landed the belief-envelope substrate and consumer wave across the archived `S113BELENV-001` through `S113BELENV-006` ticket chain: `BeliefValue<T>` / `BeliefSet<T>`, planner-facing target-location / entities-at / commodity-stock envelope reads, decision-history belief snapshots, live contradiction derivation from stored refutation provenance, and the scenario-backed stale-envelope golden proof.
  - soft depends on S109 for `BeliefStatus::Contradicted` alignment
  - soft depends on S112 for the still-deferred information slot consumer

**Wave 3 adjunct** (Phase 8 critical-path closer):
- **S122**: ✅ COMPLETED — archived at [archive/specs/S122-frame-assumption-commodity-availability.md](/home/joeloverbeck/projects/worldwake/archive/specs/S122-frame-assumption-commodity-availability.md). Landed the live `FrameAssumption::CommodityAvailableAt` path end to end: goal-derived assumption population, belief/co-location evaluation, failed-assumption trace payloads, survival-golden/falsification-harness proof, and the adjacent survival-path cleanup needed to make the S122 contradiction chain honest in long-run goldens.
  - hard depended on S109 (consumed the post-correction `record_assumption_failure` TTL/clearing semantics)
  - soft relationship with S110 remains historical only; S122 itself still did not add a new decision-history `EventTag`
  - soft relationship with S113 remains optional; the landed path uses the live belief surface without requiring the envelope layer first
- **S123**: ✅ COMPLETED — archived at [archive/specs/S123-preference-ordering-authority.md](/home/joeloverbeck/projects/worldwake/archive/specs/S123-preference-ordering-authority.md). Landed the final preference-ordering authority lock: `compare_ranked_goals` is now file-private, `RankingOutcome.ranked` is `pub(crate)`, `ranking.rs` carries compile-fail doctests for external visibility fences, and the new grep regression prevents any parallel `fn compare_ranked_goals` definition from reappearing under `worldwake-ai/src/`.
  - hard depends on S112 (post-S112 call-site surface is the migration target)
  - soft relationship with S74 (`explain_ranked_goal_order` stays the public "why X outranked Y" surface)
- **S124**: ✅ COMPLETED — archived at [archive/specs/S124-canonical-opportunity-expectation-failure.md](/home/joeloverbeck/projects/worldwake/archive/specs/S124-canonical-opportunity-expectation-failure.md). Landed the committed-source provenance follow-up wave across the archived `S124OPEXFAL-001` and `S124CANOPPEXP-001` through `S124CANOPPEXP-004` ticket chain: retained-plan concrete source preservation, normalized AI-layer expectation-failure incidents plus coalesced attribution, `SourceInvalidated` reconsideration routing, and decision-history `SourceExpectationFailure` surfacing through the canonical event payload family and observer rendering.
  - hard depends on S22 and S109
  - soft relationship with S110 and S112

### Phase 8 Gate

- [ ] All 7 specs reassessed (`/reassess-spec`) and ticket-decomposed
- [ ] Wave 1 specs implemented and passing golden E2E tests
- [ ] Wave 2 specs implemented and passing golden E2E tests
- [ ] Wave 3 specs implemented and passing golden E2E tests
- [x] Portfolio planning golden (infeasible top-N + feasible lower slot) passes
- [ ] Every `ActionDef` registration has explicit `BindingStrictness` (compile-time coverage)
- [x] No remaining `BlockingFact::Unknown` or `AssumptionFailed` variants (migrated to typed `Discrepancy`)
- [x] Authoritative decision history events surfaced in observer "Decision History" section for `survival-baseline.ron`
- [x] Belief envelope golden (stale belief surfaces via `status == Stale`) passes
- [x] S123 implemented: `OrderedRanked` migration complete; `compare_ranked_goals` file-private; compile-fail doctests and grep regression pass; no parallel comparator definition reachable in-crate
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [x] `cargo test --workspace` passing

- Note: Wave 1, Wave 2, Wave 3, and the S122-S124 adjunct follow-up chain are complete.

---

## Phase 9: Belief-First Continual Planning Structural

Follow-on to Phase 8. Deferred from the Phase 8 scope because these specs depend on the Phase 8 substrate (typed discrepancies, decision-history events, belief envelope) and are significantly larger refactors. Phase 9 specs should be reassessed just before implementation, since the architectural surface may drift after Phase 7 (S60–S66) and Phase 8 land.

### Dependency Graph

```text
S114 ✅ archived (hard deps on S109, S110, S113)
  │
  └── S115 ✅ archived (hard deps on S110, S112, S114; soft dep on S123)
```

### Active Execution Steps

**Completed in Wave 1**:
- **S114**: ✅ COMPLETED — archived at [archive/specs/S114-plan-step-guards.md](/home/joeloverbeck/projects/worldwake/archive/specs/S114-plan-step-guards.md). Landed the staged plan-step guard and expectation-monitoring wave across the S114 ticket family: `PlanGuard` / `PlanExpectation`, `ExpectationBasis::PlanStepCompletion`, widened `ExpectationMismatchPayload`, declarative `ActionDef` templates, AI-side mismatch emission, and the truthful paired proof surface for merchant-departure guard breach (golden branch-selection/local trade binding plus focused `agent_tick` mismatch/discrepancy proof).
- **S115**: ✅ COMPLETED — archived at [archive/specs/S115-agenda-manager.md](/home/joeloverbeck/projects/worldwake/archive/specs/S115-agenda-manager.md). Landed `AgendaProfile`, ai-runtime `AgendaState`, the `GoalOffer` / `AgendaEntry` rename, the `tick_agenda(...)` lifecycle manager, the D4A feasibility-rejection classifier, observer agenda-state rendering, and the later seller-return trade follow-on fixes needed to prove pending parking, revival, and eventual resumed purchase completion. The stronger `agenda_tick_system before candidate generation` plan was falsified on the live branch; the truthful shipped seam runs `tick_agenda(...)` after ranking while downstream planning still consumes the fresh ranked feed and executable commitment still finalizes at selected-plan adoption.

No active Phase 9 specs remain.

### Phase 9 Gate

- [x] Both specs reassessed post-Phase-7/8 and ticket-decomposed
- [x] S114 implemented: guard breaches short-circuit revalidation before affordance matching; `ExpectationMismatch` events appear in event log
- [x] S115 implemented: committed goals persist across ticks deterministically; pending/suspended surfaced in observer output
- [x] S115 D4A classifier lands: the S112 cargo-satisfaction assertion and `portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal` both pass via the lifecycle classifier, not via ad-hoc `build_candidate_plans` special cases
- [ ] Deterministic replay of scenarios produces identical agenda state transitions
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passing

---

## Developer Tooling

Non-phase-gated specs that add or extend developer-facing tooling. Tooling
specs are pure observer boundaries (FND-29 Debuggability): they read
authoritative simulation state but never mutate world meaning, and they
introduce no simulation components, actions, systems, or feedback loops. The
`docs/spec-drafting-rules.md` analyses that assume a simulation system
(FND-01 Section H, Permille-for-world-values, profile-driven parameters,
SystemFn integration, component registration) are explicitly opted out of in
each tooling spec, with reasoning.

### Adjunct Wave: Interactive Debugger

Derived from `/brainstorm` session (2026-04-24) that triaged desktop-app
technology options for Rust, validated that egui/eframe is the only mature
immediate-mode stack that matches the "no big opinionated framework"
constraint, and confirmed that a hand-rolled Fruchterman-Reingold layout
is cheaper than any available crate dep at current scenario scale (≤30
places). Lands a separate workspace crate so GUI transitive deps (winit,
wgpu, accesskit, swash) stay isolated from engine CI runs.

```text
T01 ✅ archived (independent; no spec deps)
```

- **T01**: ✅ COMPLETED — Debug Visualizer — archived at
  [archive/specs/T01-debug-visualizer.md](../archive/specs/T01-debug-visualizer.md).
  Landed the new `worldwake-visualizer` crate: live
  stepper with play/pause/step/space/speed controls, force-directed place
  layout, agent rendering with transit-progress lerp, hover tooltips with
  zone-colored need bars, click-to-open tabbed modal (Overview / Needs /
  Beliefs / Inventory / Plan / Traces), local pointer-centered wheel zoom,
  and middle-drag pan. Observer-only boundary; reuses `worldwake-cli`'s
  scenario loader; adds no engine-side coupling.

---

## Phase 10: Survival Mechanic Depth

Derived from external gameplay assessment `reports/proposed-gameplay-mechanic-changes.md` (generated 2026-04-25 against `reports/scenario-narrative-survival-baseline-20260425-223423.md`), validated against the actual codebase post-S121/S122/S124/S125 and `docs/FOUNDATIONS.md`. Of 12 proposals, 6 accepted and scoped down into 6 specs (S126–S131). The remaining 6 proposals were rejected: PR-5 (UseClaim/FacilityQueue) is already addressed by `ContentionQueue` + `queue_for_facility_use`; PR-7 (broad perception-as-prediction-error) is largely addressed by S114/S122/S101/S105 with the narrow arrival-diff piece folded into S130; PR-9 (topology authoring) folds engine pieces into S128 + S129 with the per-place rebalance becoming a downstream scenario-authoring ticket; PR-10 (dampener checklist) is already addressed by `docs/spec-drafting-rules.md` Section H requirements; PR-11 (standalone partial-failure spec) folds into S127/S128/S129 as the granular-aftermath aspect of each; PR-12 (standalone event-tag spec) folds into S128/S129/S130 events per FND-30.

Phase 10 runs independently of Phase 7's pending consequence-carrier specs (S60–S66) — no hard dependency in either direction. The new specs deepen self-care mechanics that all current `survival-*.ron` scenarios exercise; they do not require persistent site occupancy, predator ecology, or boundary processes.

### Dependency Graph

```text
S126 ✅ archived
S127 ✅ archived
S128 ✅ archived
S131 ✅ archived
S133 ✅ archived
S130 ✅ archived (hard dep on S128 ✅; soft dep on S122 ✅)
S129 ✅ archived (soft dep on S128 ✅; hard deps on S106 ✅, S110 ✅)
```

All cross-spec dependencies among S126–S131 are **soft** (benefit but not blocking). Hard dependencies on S106, S110, S122 are already satisfied (those specs are completed).

### Archived Execution Steps

**Wave 1** (parallel, no hard deps among new specs):
- **S126**: ✅ COMPLETED — Need Projection and Plan Time-Budget Assumptions — archived at [archive/specs/S126-need-projection-time-budget.md](/home/joeloverbeck/projects/worldwake/archive/specs/S126-need-projection-time-budget.md). Landed `FrameAssumption::NeedSafeUntilTick`, `Discrepancy::NeedHorizonExceeded`, `HomeostaticNeeds::projected_tick_of`, keyed `MetabolismProfile::rate(need)` and `DriveThresholds::high(need)` accessors, the populate/evaluate/record chain through S109's typed-discrepancy path with `DiscrepancyClearing::TtlExpiry`, decision-trace surfacing, and end-to-end golden coverage in `golden_need_projection.rs` against the new auxiliary `scenarios/survival-need-projection.ron`. `SAVE_FORMAT_VERSION` bumped 47 → 48. The pre-existing `golden_goal_switching_during_multi_leg_travel` golden was rewritten under the new lawful horizon-aware contract during ticket -003.
- **S131**: ✅ COMPLETED (D4 rolled back — see S133) — Source Reliability Wait and Capacity Extension — archived at [archive/specs/S131-source-reliability-wait-capacity.md](/home/joeloverbeck/projects/worldwake/archive/specs/S131-source-reliability-wait-capacity.md). Landed `ReliabilityRecord` wait/capacity fields, `PreferenceProfile.wait_sensitivity_weight`, wait observation hooks for facility and resource-extraction grants, capacity observation from perception, composite source-reliability ranking, and five-scenario golden coverage in `golden_source_reliability.rs`. **Update 2026-05-03:** D4's motive-additive composite was rolled back on 2026-05-03 because it perturbed cross-category goal ranking and broke four pre-existing survival goldens; the data substrate (D1, D2, D3, D5, plus the data-path goldens 137 and 138) remains live. **Update 2026-05-04:** S133 reauthored the consumer as a same-commodity sub-rank tiebreaker and archived at [archive/specs/S133-source-composite-tiebreaker.md](/home/joeloverbeck/projects/worldwake/archive/specs/S133-source-composite-tiebreaker.md), with new D6 coverage in `golden_source_composite.rs` scenarios 375-380.

**Wave 2** (after Wave 1; soft deps consumable):
- **S127**: ✅ COMPLETED — Quantity-Aware Acquisition and Visible Source State — archived at [archive/specs/S127-quantity-aware-acquisition.md](/home/joeloverbeck/projects/worldwake/archive/specs/S127-quantity-aware-acquisition.md). Landed `AcquisitionQuantity { desired_min, desired_target, horizon_ticks }` on `AcquireCommodity`, `extraction_slots` + `extraction_duration_ticks` on `ResourceSource`, `LastHarvestTrace` ring buffer pruned by the existing `item_decay_system`, `ResourceExtractionQueues` per-slot contention substrate, `CommitTraceData::Harvest.partial_quantity` for partial-success harvest aftermath, candidate-generation quantity derivation from agent state, decision-trace surfacing of the quantity tuple, and five-scenario golden coverage in `golden_quantity_aware_acquisition.rs` (single-slot queue, multi-slot parallel grants, partial success, S126-driven target, FOUNDATIONS Section VI Scenario E). Final ticket S127QUAAWAACQ-010 also wired the AI's `BlockingFact::ReservationConflict` clearing baseline through new `extraction_slot_*` accessors so harvest-contention blockers clear structurally rather than by TTL.
- **S128**: ✅ COMPLETED — Sleep Episodes and Place-Quality Recovery — archived at [archive/specs/S128-sleep-episode-place-quality.md](/home/joeloverbeck/projects/worldwake/archive/specs/S128-sleep-episode-place-quality.md). Landed `SleepEpisode` runtime component, `WakeCondition` enum, universal `SleepQualityProfile` per-place, `EventTag::SleepEpisodeStarted` / `SleepEpisodeEnded`, episode-based sleep action handling, wake-on-S126 projection, per-place sleep candidate ranking, survival-baseline sleep-quality authoring, observer decision-history rendering, and six-scenario golden coverage in `golden_sleep_episode.rs`.
- **S130**: ✅ COMPLETED — Survey Records and Frontier Disconfirmation — archived at [archive/specs/S130-survey-records-frontier-disconfirmation.md](/home/joeloverbeck/projects/worldwake/archive/specs/S130-survey-records-frontier-disconfirmation.md). Landed `SurveyMemory` per-agent component, `HypothesisKind` on `ExploreLocation`, perception-time arrival evaluation, survey-memory decay, ranking damping for confirmed-empty places, `CandidateTrace.damped` diagnostics, and end-to-end golden coverage in `golden_exploration.rs`. Folds in PR-7's narrow arrival-diff piece.

**Wave 3** (after Wave 2; soft deps consumable):
- **S129**: ✅ COMPLETED — Place Dirtiness and Facility Wear — archived at [archive/specs/S129-place-dirtiness-facility-wear.md](/home/joeloverbeck/projects/worldwake/archive/specs/S129-place-dirtiness-facility-wear.md). Landed `PlaceDirtiness`, `LatrineFullness`, and `WashBasinState` components; `WasteCreated`/`WashFacilityUsed` event tags; relieve/toilet/wash handler extensions; basin refill and place dirtiness maintenance; partial-wash outcome; wash basin candidate/ranking integration; and eight-scenario golden coverage in `golden_place_dirtiness.rs`. `LatrineMaintained` remains deferred until a `clean_latrine` action exists.

### Follow-up Tickets (not specs)

- **S133**: ✅ COMPLETED — Source Composite Tiebreaker for Same-Commodity Acquisition — archived at [archive/specs/S133-source-composite-tiebreaker.md](/home/joeloverbeck/projects/worldwake/archive/specs/S133-source-composite-tiebreaker.md). Reauthored S131's D4 wait/capacity consumer as a same-commodity sub-rank tiebreaker after the original motive-additive composite was rolled back on 2026-05-03 (it perturbed cross-category goal ranking — `survival_drive_escalation_lands_row_four`, `survival_offices_proves_force_law_uptake`, `survival_preferences_keeps_proactive_diversification_alive_under_survival`, `survival_tell_lands_row_five` regressed). S131's data substrate (perception capacity write, queue wait observation, `PreferenceProfile.wait_sensitivity_weight`) remains live; S133 landed `SourceCompositeRank` with permille-scale trust/wait/capacity factors that operate only inside the sort-comparator tiebreaker between same-`(commodity, purpose)` sibling opportunities, never on `motive_score`. Added `PreferenceProfile.capacity_observation_weight` and golden coverage in `golden_source_composite.rs` scenarios 375-380, including the post-review Scenario 380 metadata correction. FND-26 / FND-27 / FND-28 alignment.

- **Survival-baseline contention authoring** (post-S127/S131): enable `ContentionPolicy::FirstArrival` on the wells/orchard in `scenarios/survival-baseline.ron` so the existing `queue_for_facility_use` substrate is exercised under realistic pressure. Authoring task, not a spec.
- **Additional sleep-quality rebalance** (post-S128): `scenarios/survival-baseline.ron` now has the S128 four-place `SleepQualityProfile` authoring. Extend the bounded `0..=1000` sleep-quality profiles to other survival scenarios when those scenarios need place-specific sleep-site differentiation.
- **Place dirtiness authoring** (post-S129): author `PlaceDirtiness.decay_per_tick` per place to differentiate accumulation/recovery rates per topology character.

### Adjunct Wave: S129 Post-Remediation Architectural Audit

Derived from the 2026-05-01 `/brainstorm` triage of the four CIREM tickets (`S129CIREM-001..004`) that remediated post-S129 CI failures. The triage examined whether those fixes were workarounds masking deeper architectural issues. Verdict: the four tickets fixed real root causes (no weight-knob hacks, no contract relaxations, no force-X-when-Y carve-outs), but three of them revealed deeper substrate patterns the per-ticket scope did not generalize. This wave addresses those patterns. Source: `docs/triage/2026-05-01-s129cirem-followup-triage.md`.

```text
S132 ✅ archived
BELASPCOV-001 ticket (completed — claim-aspect coverage audit)
INFRARET-001 ticket (completed — infrastructure retention generalized)
RELIEFACT-001 ticket (completed — per-need relief-actionability predicate)
LOCROOT-001 ticket (completed — direct-root synthesis locality audit)
```

**Wave** (parallel):

- **S132**: ✅ COMPLETED — Frontier-Exhaustion Strategy as Goal-Kind Property — archived at [archive/specs/S132-frontier-exhaustion-strategy.md](/home/joeloverbeck/projects/worldwake/archive/specs/S132-frontier-exhaustion-strategy.md). Replaced the per-`GoalKind` allow-list in `frontier_exhaustion_entry` with a goal-kind-property dispatch (`PermanentUntilInvalidator | CooldownRetry`). Closes the pattern of S129CIREM-002 / S129CIREM-004 each adding a one-off variant to the switch. FND-21 / FND-22A alignment.
- **BELASPCOV-001** (ticket): `BelievedEntityState ↔ EntityBeliefAspect` claim-aspect coverage audit — produced `docs/audits/2026-05-01-believed-entity-state-claim-coverage.md`, found no mutable belief-content gaps, and created no per-gap secondary tickets. CIREM-003's `WashBasinState` claim-aspect addition was ad hoc; this ticket verified the current summary fields before the next chronic-stall failure mode.
- **INFRARET-001** (ticket): COMPLETED — generalized `state_salience_boost` from two hardcoded shape pairs to a direct-observation plus need-pressure predicate over current opportunity aspects. Soft dependency on BELASPCOV-001 was satisfied; the completed audit found no additional aspects to fold in.
- **RELIEFACT-001** (ticket): COMPLETED — extracted per-need relief-actionability predicate from `emit_exploration_candidates` (replaced the inline dirtiness `if` branch added by CIREM-002 with a per-need dispatch).
- **LOCROOT-001** (ticket): COMPLETED — audited direct-root synthesis (`GoalOffer::synthesized_root_candidate_targets`) for arms whose action precondition is `EntityAtActorPlace` / `ActorPlace`, produced `docs/audits/2026-05-01-direct-root-synthesis-locality.md`, and gated co-location-bearing synthesized direct roots so remote targets compose through travel before local action.

### Phase 10 Gate

- [x] All 6 specs reassessed (`/reassess-spec`) and ticket-decomposed
- [x] Wave 1 specs implemented and passing golden E2E tests
- [x] Wave 2 specs implemented and passing golden E2E tests
- [x] Wave 3 specs implemented and passing golden E2E tests
- [ ] Authored survival-baseline scenario rebalance landed (sleep_quality, place_dirtiness)
- [ ] Survival-baseline narrative report shows: (a) measurable variation in sleep-site preference across agents, (b) `desired_target > 1` acquisition under projected horizons, (c) at least one `SurveyRecord` damping observable in decision trace, (d) one `SleepEpisodeStarted` per sleep adoption (no `sleep → sleep` artifact)
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [x] `cargo test --workspace` passing
- [x] Golden E2E coverage for each new spec's core behavior

---

## Phase 11: Belief-First Continual Planning Architectural

Derived from external AI architecture assessment (`brainstorming/worldwake_ai_architecture_review.md`, regenerated 2026-05-04 post-S133) validated against the actual codebase and `docs/FOUNDATIONS.md`. The assessment post-dates ALL completed specs through S133. Of ~33 distinct proposals across the assessment's 4 sections + roadmap, 9 accepted (S134–S142), 4 folded into accepted specs, 20 rejected.

Phase 11 is the third layer of the Belief-First Continual Planning arc: Phase 8 (Foundation — typed discrepancies, decision-history events, belief envelope, portfolio planning), Phase 9 (Structural — plan-step guards, agenda manager), Phase 11 (Architectural — canonical effect schemas, opportunity compiler, plan causal links and repair, motive sources, multi-axis artifact lifecycle, contention inspectability, epistemic sensing).

Key triage decisions:
- **Stale claims rejected**: PR-1 (ranking.rs compile errors) is stale — S133 (commits c01e1027, 5089337b, 27c36047 on 2026-05-04) cleanly wired `capacity_observation_weight` and `source_composite`. PR-5 (replace `max_candidates_to_plan = 2` with portfolio) already addressed by S112. PR-16 (scenario homogeneity validation) already addressed by S111.
- **Already drafted**: PR-25 (BoundaryRegion/BoundaryProcess) already drafted as S62 in Phase 7. PR-27 (jurisdiction/delegation) folds into existing drafted S63.
- **Folded into accepted specs**: PR-7 (relevant_ops as hint) → S138; PR-13 (richer travel pruning) → S138; PR-15 (typed blocker clearing) → S137; PR-20 (richer interrupt-layer opportunism) → S138.
- **Deferred to Phase 12**: PR-9 (HTN-style methods) — productive only after S134 (effect schemas), S138 (opportunity compiler), and S141 (motive sources) land. PR-23 (InstitutionalTask ledger) — defer until office-driven scenarios are exercised. PR-26 (full evidence-carrier set) — defer until combat scenarios stress-test the gap. PR-29 (recurring routines) — defer until Phase 7's S60+ specs exercise patrol/office scenarios. PR-32 (additional architecture lints) — handled per emerging invariant as tickets rather than a standalone spec.
- **Rejected as YAGNI**: PR-6 (causal-role goal-interruption profile — refinement absent specific failure scenario), PR-14 (cross-tick search continuation — S132 + S102 cover graceful degradation), PR-19 (in-world deliberation attention), PR-21 (concrete habit/trust learning state — S131/S130/S107 cover today's needs), PR-28 (long actions partial state — beyond S127's harvest/wash), PR-30 (live metrics dashboard — observer.rs covers most), PR-31 (adversarial fuzzers), PR-33 (LLM guidance — not a spec).

Phase 11 runs independently of Phase 7's pending consequence-carrier specs (S60–S66) — no hard dependency in either direction. Most Phase 11 specs touch the AI planner substrate, the event log, or the artifact-lifecycle layer; Phase 7 adds new ECS consequence carriers.

### Dependency Graph

```text
S134 (completed, archived)    S135 (completed, archived)    S140 (independent)            S142 (independent)
   │                                                                                            
   └── S138 (hard dep on S134)
         │
         ├── S137 (soft deps on S134, S136, S138)
         │     │
         │     └── S139 (soft deps on S137, S138)
         │
         └── (S139 also soft deps on S138)

S136 (completed, archived)
   │
   └── S141 (soft dep on S136)
```

### Active Execution Steps

**Completed Wave 1 substrate**:
- **S134**: Canonical Effect Schema for ActionDef — completed and archived at `archive/specs/S134-canonical-effect-schema.md`. Planner forward model now uses simulator action schemas via declarative `EffectSchema` per `ActionDef`; `apply_hypothetical_transition` parallel dispatch was removed.
- **S135**: Planner Snapshot Perception Budget and Observation Omission — completed and archived at `archive/specs/S135-planner-perception-budget.md`. Removed the planner-side `max_snapshot_entities_per_place` cap, added typed per-agent omission records and observer rendering, surfaced omission attribution through decision traces and hypothetical revalidation, and landed scenarios 381-383 in `golden_perception_omission.rs`.
- **S136**: Always-On Decision Event Payload Extension — completed and archived at `archive/specs/S136-decision-event-payload-extension.md`. Implemented across the archived `S136DECEVEPAY-001` through `S136DECEVEPAY-007` ticket chain: always-on decision payloads now carry rejected-goal comparison dimensions, failure-path decisive refs, frame assumptions with step provenance, observer single-line summaries, replay/save-load coverage, and generated golden payload-shape coverage plus deterministic payload-size soak enforcement.

**Remaining Wave 1** (parallel, no hard deps):
- **S140**: Multi-Axis Artifact Lifecycle — five-axis lifecycle (existence/visibility/legal_effect/credibility/actionability) replacing single-enum `ArtifactState`. FND-25A direct compliance.
- **S142**: Contention Event Inspectability — `EventTag::ContentionResolved` with typed `ContentionResolutionRule` and per-claimant outcome.

**Wave 2** (after Wave 1):
- **S138**: Affordance-to-Opportunity Compiler with Effect-Schema Indexing — bottom-up opportunity compiler driven by `EffectSchemaIndex`; `relevant_ops` becomes hint, effect-schema becomes authority. Folds in PR-7, PR-13, PR-20.
  - hard dependency S134 is satisfied (for `EffectSchemaIndex`)
- **S141**: Motive Source Ledger and Desire Tokens — `MotiveSource` enum on `GoalOffer`; `motive_score` becomes derived view over per-agent state references. FND-3 direct compliance.
  - S136 soft dependency is satisfied (for `decisive_motive_sources` payload integration)

**Wave 3** (after Wave 2):
- **S137**: Plan Causal Links and Localized Repair Search — extend `PlanGuard` with `CausalLink` provenance; `PlanRepairContext` localized repair before full replan; per-`Discrepancy` `ClearingCondition`. Folds in PR-15.
  - S134 and S136 soft dependencies are satisfied (repair benefits from queryable effect-schema; repair-applied payload integration can reuse S136 decisive-ref families); still soft depends on S138 (rebind-target consumes opportunity output)
- **S139**: Epistemic Sensing Subgoals — Ask Witness and Inspect Container — discrete `GoalKind::AskWitness` and `GoalKind::InspectContainer` with `EpistemicProfile`. Required for FOUNDATIONS Scenarios C and G end-to-end.
  - soft depends on S137 (`RepairKind::InsertVerification` splices these), S138 (opportunity compiler emits witness/container anchors)

### Phase 11 Gate

- [ ] All 9 specs reassessed (`/reassess-spec`) and ticket-decomposed
- [ ] Wave 1 specs implemented and passing golden E2E tests
- [ ] Wave 2 specs implemented and passing golden E2E tests
- [ ] Wave 3 specs implemented and passing golden E2E tests
- [x] `apply_hypothetical_transition` removed; conformance test reshaped to coverage assertions over `EffectSchema`
- [x] `max_snapshot_entities_per_place` removed; planner snapshot reads from `observation_budget`-truncated belief observations only
- [x] S136 always-on decision payload extension landed: rejected-goal dimensions, failure-path `decisive_*` refs, frame assumptions with step provenance, observer summaries, replay/save-load coverage, golden payload-shape coverage, and payload-size soak enforcement
- [ ] Plan repair golden shows ≥30% reduction in full-replan triggers vs pre-S137 baseline on `survival-baseline.ron`
- [ ] Opportunity compiler golden proves desperate-agent steal opportunity emission and salience-weighted ranking
- [ ] Multi-axis artifact lifecycle proves FOUNDATIONS Scenario G (false rumor → wrongful accusation → contested evidence → exoneration) end-to-end
- [ ] Motive-source parity regression: every existing 1440-tick survival golden produces identical `motive_score` values pre/post-S141
- [ ] Contention-resolution events emit at every grant-issuance site under `survival-contested.ron`
- [ ] FOUNDATIONS Scenario C (stored gold robbery): owner-believes-gold-present → `InspectContainer` → mismatch → robbery report producible end-to-end
- [ ] FOUNDATIONS Scenario G (false rumor → wrongful accusation → correction): `AskWitness` chain across two contradicting witnesses producible end-to-end
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passing
- [ ] Golden E2E coverage for each new spec's core behavior
