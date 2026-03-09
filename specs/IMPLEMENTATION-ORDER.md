# Implementation Order & Dependency Graph

## Dependency Graph

```
E01 ──→ E02 (topology needs IDs/types)
E01 ──→ E03 (entity store needs IDs/types)
E03 ──→ E04 (items need entity store)
E03 ──→ E05 (relations need entity store)
E04 ──→ E06 (events track item changes)
E05 ──→ E06 (events track relation changes)
E06 ──→ E07 (actions emit events)
E07 ──→ E08 (scheduler drives actions, replay needs events)

--- Phase 1 gate: E08 determinism + conservation tests green ---

E08 ──→ E09 (needs use tick-driven progression)
E08 ──→ E10 (production uses scheduler)
E08 ──→ E11 (trade uses actions)
E08 ──→ E12 (combat uses actions)
E09,E10,E11,E12 ──→ E13 (AI needs all systems to plan over)

--- Phase 2 gate: agents autonomously survive ---

E13 ──→ E14 (beliefs needed for agent planning)
E14 ──→ E15 (rumors build on perception)
E14 ──→ E16 (succession needs beliefs/loyalty)
E15 ──→ E17 (crime needs discovery)

--- Phase 3 gate: information propagates, offices transfer ---

E16 ──→ E18 (bandits need faction system)
E16 ──→ E19 (guards need public order)
E13 ──→ E20 (companions use decision arch)
E13 ──→ E21 (CLI uses affordance query)
E18,E19,E20,E21 ──→ E22 (integration tests need everything)
```

## Execution Steps (with parallelism)

### Step 1
- **E01**: Project Scaffold & Core Types

### Step 2 (parallel after E01)
- **E02**: World Topology
- **E03**: Entity Store & Components

### Step 3 (parallel, both need E03)
- **E04**: Items & Containers
- **E05**: Relations & Ownership Semantics

### Step 4 (needs E04 + E05)
- **E06**: Event Log & Causality

### Step 5 (needs E06)
- **E07**: Action Framework

### Step 6 (needs E07)
- **E08**: Time, Scheduler, Replay & Save/Load

### Phase 1 Gate
Before proceeding, ALL must pass:
- T01: Unique location
- T02: Conservation
- T03: No negative inventory
- T04: Reservation lock
- T05: Precondition gate
- T06: Commit validation
- T07: Event provenance
- T08: Replay determinism
- T09: Save/load round-trip
- T13: Containment acyclic

### Step 7 (parallel after E08)
- **E09**: Needs & Metabolism
- **E10**: Production & Transport
- **E11**: Trade & Economy
- **E12**: Combat & Health

### Step 8 (needs E09-E12)
- **E13**: Agent Decision Architecture

### Phase 2 Gate
Before proceeding:
- Agents autonomously eat, drink, sleep, trade
- Merchants restock physically
- Basic survival loop runs 24+ hours without deadlock
- T12: No player branching
- T14: Dead agents inactive
- T15: Need progression

### Step 9 (needs E13)
- **E14**: Perception & Belief System

### Step 10 (parallel after E14)
- **E15**: Rumor, Witness & Discovery
- **E16**: Offices, Succession & Factions

### Step 11 (needs E15)
- **E17**: Crime, Theft & Justice

### Phase 3 Gate
Before proceeding:
- Information propagates through explicit channels
- Offices transfer through succession
- T10: Belief isolation
- T11: Office uniqueness
- T25: Unseen crime discovery

### Step 12 (parallel)
- **E18**: Bandit Camp Dynamics (needs E16)
- **E19**: Guard & Patrol Adaptation (needs E16)
- **E20**: Companion Behaviors (needs E13)
- **E21**: CLI & Human Control (needs E13)

### Step 13 (needs E18-E21)
- **E22**: Scenario Integration & Soak Tests

### Final Acceptance
- All T20-T32 pass
- 100-seed soak test with zero invariant violations
- Replay consistency verified
- Causal depth >= 4 across >= 3 subsystems for all 4 exemplar scenarios

## Crate Dependency Graph

```
worldwake-core: (no internal deps)
worldwake-sim: depends on worldwake-core
worldwake-systems: depends on worldwake-core, worldwake-sim
worldwake-ai: depends on worldwake-core, worldwake-sim, worldwake-systems
worldwake-cli: depends on worldwake-core, worldwake-sim, worldwake-systems, worldwake-ai
```

## Phase Summary

| Phase | Epics | Goal | Gate Test |
|-------|-------|------|-----------|
| 1: World Legality | E01-E08 | Deterministic world with conservation | T01-T09, T13 |
| 2: Survival & Logistics | E09-E13 | Agents autonomously survive | T12, T14, T15 |
| 3: Information & Politics | E14-E17 | Information propagates, offices transfer | T10, T11, T25 |
| 4: Adaptation & CLI | E18-E22 | Full integration, all scenarios | T20-T32 |
