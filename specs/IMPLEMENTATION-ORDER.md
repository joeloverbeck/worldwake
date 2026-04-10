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

Completed adjunct specs:
- `S67: Golden E2E Gaps — S59` archived at [archive/specs/S67-golden-gaps-S59.md](/home/joeloverbeck/projects/worldwake/archive/specs/S67-golden-gaps-S59.md).
- `S68: Goal-Switch Contention Cleanup` archived at [archive/specs/S68-goal-switch-contention-cleanup.md](/home/joeloverbeck/projects/worldwake/archive/specs/S68-goal-switch-contention-cleanup.md). Golden E2E proof: Scenario 123 in `golden_production.rs`.
- `S73: Planning Snapshot Entity Relevance` archived at [archive/specs/S73-planning-snapshot-entity-relevance.md](/home/joeloverbeck/projects/worldwake/archive/specs/S73-planning-snapshot-entity-relevance.md). Added goal-aware snapshot filtering, per-place entity caps, and truthful soak telemetry/validation alignment for the planning-cost surface.
- `S74: Intention Commitment Under Needs Fluctuation` archived at [archive/specs/S74-intention-commitment-under-needs-fluctuation.md](/home/joeloverbeck/projects/worldwake/archive/specs/S74-intention-commitment-under-needs-fluctuation.md). Replaced the planning-path top-2 continuation heuristic with per-agent margin-based commitment, fixed the exposed same-goal merchant continuity regression, and corrected the soak baseline/spec validation handoff.
- `S76: Golden E2E Gaps — Simulation Observer Report` archived at [archive/specs/S76-golden-gaps-simulation-observer.md](/home/joeloverbeck/projects/worldwake/archive/specs/S76-golden-gaps-simulation-observer.md). Landed Scenarios 126–129 across `golden_simulation_gaps.rs`, `golden_perception_exposure.rs`, and `golden_reasoning_diversity.rs`, with the S76-D contract narrowed to the strongest live same-state `eat`/`drink` utility-divergence proof.
- `S78: Observer Failed-Plan Diagnostics` archived at [archive/specs/S78-observer-failed-plan-diagnostics.md](/home/joeloverbeck/projects/worldwake/archive/specs/S78-observer-failed-plan-diagnostics.md). Added failed-plan depth/candidate/location diagnostics, a bounded planning-time `TargetBeliefPresence` trace carrier, and live observer breakdown counts for missing target beliefs.
- `S81: Golden E2E Gaps — Simulation Remediation` archived at [archive/specs/S81-golden-gaps-simulation-remediation.md](/home/joeloverbeck/projects/worldwake/archive/specs/S81-golden-gaps-simulation-remediation.md). Landed the death traceability substrate (`DeathCause`, `DeadAt.cause`, `EventTag::Death`), authoritative need-based mortality and death-event tagging, and Scenarios 130–132 in `golden_simulation_gaps.rs`.

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
