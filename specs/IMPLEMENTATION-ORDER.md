# Implementation Order & Dependency Graph

## Completed Work

Phases 1–6 (E01–E22, FND-01, FND-02, S01–S58) completed.
See `archive/` for detailed completion records.

Completed adjunct specs:
- `S67: Golden E2E Gaps — S59 Expectation and Obligation Substrate` archived at [archive/specs/S67-golden-gaps-S59.md](/home/joeloverbeck/projects/worldwake/archive/specs/S67-golden-gaps-S59.md).
- `S67` adds golden coverage for the currently implemented `S59` overdue-search and report-missing chains, but it does not imply that parent spec `S59` is complete. `S59` remains active until its remaining proposed behavior is fully implemented.
- `S68: Goal-Switch Contention Cleanup` archived at [archive/specs/S68-goal-switch-contention-cleanup.md](/home/joeloverbeck/projects/worldwake/archive/specs/S68-goal-switch-contention-cleanup.md).
- `S68` fixes stale `ContentionIntents` on all plan-abandonment paths (goal switch, lost plan, assumption failure, patience exhaustion, plan terminals, failure handling). Golden E2E proof: Scenario 123 in `golden_production.rs`.

---

## Phase 7: Consequence Carriers

Derived from external gameplay assessment (`brainstorming/prioritary-gameplay-systems.md`) validated against the actual codebase and `docs/FOUNDATIONS.md`. These specs add missing carriers of consequence that widen the causal graph — expectation tracking, persistent sites, predator ecology, boundary processes, contested justice, scarcity response, social aftermath, and settlement decline.

### Dependency Graph

```text
S59 (independent)     S60 (independent)     S62 (independent)     S69 (independent)
     │                     │
     │                     ├── S61 (needs S60 for dens)
     ├── S63 (needs S59)   │
     │                     │
S59 ─┤                     │
S63 ─┼── S65 (needs S59, S63)
     │
S62 ──── S64 (needs S62 for boundary pressure)
     │
S60 ─┤
S64 ─┼── S66 (needs S60, S64, S65)
S65 ─┘
```

### Active Execution Steps

**Wave 1** (parallel, no deps):
- **S59**: Expectation and Obligation Substrate — time-bounded expectations, overdue detection, search/rescue actions, last-seen propagation
  - current status note: the `S67` follow-on golden-gap spec is complete for the implemented overdue-search and report-missing slices, but `S59` itself still remains active because at least one proposed goal/action path is not fully live yet
- **S60**: Persistent Site Occupancy — site profiles with sublocations, occupancy claims, site traces, BanditCamp migration
- **S62**: Boundary Processes and Remote Shocks — source regions, boundary channels, scheduled inflows, disruption mechanics
- **S69**: Goal Dispatch Consolidation — ✅ COMPLETED — consolidated GoalFamilyPolicy and progress barrier ops into GoalDispatchDeclaration; expanded GoalDispatchKey with payload-aware ShareBelief/PostNotice variants (base GoalPriorityClass excluded — runtime-dependent)

**Wave 2** (after Wave 1):
- **S61**: Predator Ecology and Dens — predator agents with territory, hunger-driven roaming, den habitation, carcass/track evidence
  - depends on S60 (dens use site occupancy model)
- **S63**: Contested Evidence and Warrants — warrants, detention, case records, alibi, evidence contest, wrongful-accusation correction
  - depends on S59 (overdue expectations feed into suspicion/accusation)

**Wave 3** (after Wave 2):
- **S64**: Scarcity Response — Debt, Rationing, and Substitution — borrowing/lending, ration orders, hoarding, sale refusal, substitute purchasing
  - depends on S62 (boundary shocks create the upstream shortage pressure)
- **S65**: Social Aftermath Memory — provenance-tracked grudges, gratitude, kin bonds, revenge, protection, favoritism
  - depends on S59 (rescue creates gratitude edges), S63 (wrongful accusation creates grudges)

**Wave 4** (after Wave 3):
- **S66**: Settlement Decline and Reoccupation — household departure, facility closure, building vacancy, squatter reoccupation, institutional degradation
  - depends on S60 (vacant buildings as occupyable sites), S64 (scarcity pressure drives departure), S65 (social bonds anchor or repel)

### Phase 7 Gate

- [ ] All 8 specs reassessed (`/reassess-spec`) and ticket-decomposed
- [ ] Wave 1 specs implemented and passing golden E2E tests
- [ ] Wave 2 specs implemented and passing golden E2E tests
- [ ] Wave 3 specs implemented and passing golden E2E tests
- [ ] Wave 4 specs implemented and passing golden E2E tests
- Note: completed golden-gap follow-on specs such as `S67` count as coverage progress for implemented slices, but they do not satisfy a parent wave gate while the parent feature spec itself is still active.
- [ ] Canonical regression A (beast starvation → bounty → hunt) fully producible
- [ ] Canonical regression G (false rumor → wrongful accusation → correction) fully producible
- [ ] Canonical regression H (remote shock → local shortage → adaptation) fully producible
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test --workspace` passing
- [ ] Golden E2E coverage for each new spec's core behavior
