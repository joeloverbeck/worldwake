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
cannot satisfy basic needs in realistic 1440-tick simulations despite passing 349 golden tests.
Root cause: golden tests pre-wire success conditions in sterile environments, never testing whether agents
can bootstrap survival from realistic starting conditions.

**Prerequisite for all Phase 7 consequence-carrier specs (S60–S66)**: survival baseline must be proven
before adding more gameplay complexity.

```text
S103 (independent, parallel)
S104 (blocks S60–S66)
```

**Wave**:
- **S103**: Belief Claim Deduplication and Amortized Pruning — performance fix for S101 belief system (independent, can proceed in parallel with S104)
- **S104**: Survival Baseline Recovery — golden test triage, profile-gating cleanup, survival baseline scenario, golden test rebuild from survival-capable agents

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
