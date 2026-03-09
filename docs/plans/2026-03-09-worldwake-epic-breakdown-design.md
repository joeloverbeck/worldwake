# Worldwake Epic Breakdown Design Document

**Date**: 2026-03-09
**Status**: Approved
**Scope**: Break the brainstorming spec into implementable epics

## Decisions

### Language & Platform
- **Language**: Rust
- **UI**: CLI/text only (prototype)
- **External deps**: Minimal — `serde`, `bincode`, `rand_chacha`, standard crates only
- **No external ECS**: Custom entity store in `worldwake-core`

### Architecture
- 5-crate workspace: core, sim, systems, ai, cli
- Entity-component store with typed `HashMap` storage
- Append-only event log as causal source of truth
- Tick-based scheduler with deterministic RNG

### Agent AI
- Hand-coded utility scoring + GOAP-style planner
- Deterministic (given same beliefs + same RNG state)
- Single hierarchy: pressures → utility → goal → plan → execute
- Belief-only planning (no omniscience)

### Epic Granularity
- 22 fine-grained epics
- System-per-epic with foundation-first ordering
- 4 phases with strict gates between them

## Rationale

### Why 22 epics instead of fewer?
Each epic delivers a testable, self-contained system. Fine granularity means:
- Clear dependency tracking
- Parallelizable implementation (e.g., E09-E12 in parallel)
- Phase gates catch violations early
- Each epic can be reviewed independently

### Why custom ECS?
The spec requires specific invariants (conservation, unique placement, reservation exclusivity) that are easier to enforce with a known data model than with a generic ECS framework. The world is small enough that performance of a `HashMap`-based store is adequate.

### Why GOAP over behavior trees?
GOAP operates on abstract state and produces action sequences, which maps cleanly to the spec's action framework (10 properties per action, precondition gating, commit validation). Behavior trees would require more manual authoring of transitions.

### Why phases with gates?
The spec explicitly states (section 12): "Do not start Phase 3 or 4 until Phase 1 determinism and conservation tests are green." Gates ensure foundational invariants hold before building higher-level systems.

## Epic Map

| Epic | Name | Crate | Phase |
|------|------|-------|-------|
| E01 | Project Scaffold & Core Types | core | 1 |
| E02 | World Topology | core | 1 |
| E03 | Entity Store & Components | core | 1 |
| E04 | Items & Containers | core | 1 |
| E05 | Relations & Ownership | core | 1 |
| E06 | Event Log & Causality | sim | 1 |
| E07 | Action Framework | sim | 1 |
| E08 | Time, Scheduler, Replay | sim | 1 |
| E09 | Needs & Metabolism | systems | 2 |
| E10 | Production & Transport | systems | 2 |
| E11 | Trade & Economy | systems | 2 |
| E12 | Combat & Health | systems | 2 |
| E13 | Decision Architecture | ai | 2 |
| E14 | Perception & Beliefs | systems | 3 |
| E15 | Rumor, Witness, Discovery | systems | 3 |
| E16 | Offices, Succession, Factions | systems | 3 |
| E17 | Crime, Theft, Justice | systems | 3 |
| E18 | Bandit Dynamics | systems | 4 |
| E19 | Guard & Patrol | systems | 4 |
| E20 | Companion Behaviors | systems | 4 |
| E21 | CLI & Human Control | cli | 4 |
| E22 | Integration & Soak Tests | workspace | 4 |

## Test Coverage Map

| Test | Epic(s) | Invariant |
|------|---------|-----------|
| T01 UniqueLocation | E05 | 9.4 |
| T02 Conservation | E04 | 9.5 |
| T03 NoNegativeInventory | E04 | 9.6 |
| T04 ReservationLock | E05 | 9.8 |
| T05 PreconditionGate | E07 | 9.9 |
| T06 CommitValidation | E07 | 9.9 |
| T07 EventProvenance | E06 | 9.3 |
| T08 ReplayDeterminism | E08 | 9.2 |
| T09 SaveLoadRoundTrip | E08 | 9.19 |
| T10 BeliefIsolation | E14 | 9.11 |
| T11 OfficeUniqueness | E16 | 9.13 |
| T12 NoPlayerBranching | E13, E21 | 9.12 |
| T13 ContainmentAcyclic | E04 | 9.18 |
| T14 DeadAgentsInactive | E12 | 9.14 |
| T15 NeedProgression | E09 | 9.16 |
| T20-T27 Scenarios | E22 | Multiple |
| T30-T32 Soak/Regression | E22 | All |

## Risk Assessment

1. **GOAP planner complexity**: May need simplification for v0. Mitigation: start with simple goals (eat, drink) and expand.
2. **Belief system overhead**: Maintaining per-agent beliefs adds memory. Mitigation: small world (25-40 agents) keeps this manageable.
3. **Determinism**: Any floating-point comparison or HashMap iteration order could break determinism. Mitigation: use `BTreeMap` where order matters, avoid float equality.
4. **Soak test stability**: T30 (100 seeds, 7 days) may reveal rare edge cases. Mitigation: extensive property testing in earlier epics.
