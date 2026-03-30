# S46: Patrol-Driven Crime Discovery — Golden E2E Coverage

**Status**: ✅ COMPLETED

## Summary

One golden E2E suite covering the cross-system emergence chain where guard patrol drives crime discovery. Existing patrol scenarios (52-56) prove patrol mechanics in isolation: route cycling, interruption handling, belief-driven urgency, route adaptation, and locality enforcement. No existing golden test demonstrates the *design purpose* of guard patrol — that patrolling brings guards to locations where they discover crime through perception, triggering downstream investigation.

## Motivation

E19 implemented guard patrol with route adaptation and belief-driven urgency. E17 implemented crime discovery through expectation-violation perception. These two systems are designed to work together: guards patrol to find crime. But no golden test proves this cross-system chain end to end.

| Behavior | Systems Involved | Why It Matters |
|----------|-----------------|----------------|
| Patrol drives guard to crime scene | Patrol candidate gen, AI plan search, Travel | Patrol is the causal mechanism that brings the guard to the right location |
| Perception refresh detects missing goods | Perception, Belief Store | Guard's stale belief about goods at patrol waypoint creates expectation mismatch |
| Mismatch triggers investigation | AI candidate gen (InvestigateViolation), Generic actions | EntityMissing discovery leads to typed SuspectedTheft evidence |
| Full chain requires all systems | Patrol + Travel + Perception + Crime/Justice + AI | Removing any single system breaks the chain — true emergence |

## Crate

`worldwake-ai` (golden tests in `crates/worldwake-ai/tests/`)

## Dependencies

All completed:
- E19 (guard patrol — `PatrolRoute`, `PatrolProfile`, patrol action, route adaptation)
- E17 (crime/justice — `ViolationMemory`, `EntityMissing`, `InvestigateViolation`, `SuspectedTheft`)
- E14/E15 (beliefs, perception — `AgentBeliefStore`, perception refresh, stale belief detection)
- E13 (decision architecture — GOAP planner, candidate generation, goal switching)
- E10 (production — resource sources, item lots for the goods that get stolen)

## Foundational Alignment

| Principle | How This Suite Validates Compliance |
|-----------|-------------------------------------|
| FND-1 (Maximal Emergence) | Crime discovery arises from patrol + perception + belief mismatch, not scripted detection triggers |
| FND-7 (Locality) | Guard discovers crime only by physically arriving at the location through patrol travel — no remote awareness |
| FND-14 (World State != Belief State) | Guard's investigation triggers from stale belief mismatch against observed state, not from world truth |
| FND-17 (Surprise from Violated Expectation) | Guard expected goods to be present at patrol waypoint; perceives their absence; anomaly drives investigation |
| FND-20 (Resource-Bounded Reasoning) | Guard's patrol goal competes with other goals; investigation goal emerges from perception, not from omniscient crime awareness |
| FND-26 (Systems Through State) | Patrol, perception, and investigation interact through shared state (beliefs, ViolationMemory) rather than direct system calls |

## Deduplication Analysis

| Existing Coverage | What It Proves | Why S46 Is Different |
|-------------------|---------------|---------------------|
| S37 (Theft → owner discovers) | Owner returns to stash, perceives missing items via stale belief | Travel is incidental (owner goes for other reasons); S46 uses patrol as the causal driver |
| S36 (Entity missing → investigation) | Seeds the EntityMissing mismatch directly | No patrol involvement; mismatch is injected, not discovered through patrol travel |
| S52-56 (Patrol mechanics) | Route cycling, interruption, urgency, adaptation, locality | Never chain into crime discovery; patrol operates in isolation |
| S35 (Same-place concurrent violations) | Multiple violations at one place stay distinct | No patrol involvement; violations are seeded directly |

## Deliverables

### Suite 1: Patrol-Driven Crime Discovery Chain (Scenario 57)

**File**: `crates/worldwake-ai/tests/golden_patrol.rs`

**Tests**: `golden_patrol_driven_crime_discovery` + `golden_patrol_driven_crime_discovery_replays_deterministically`

#### Setup
- Two-place topology: VillageSquare and GeneralStore (connected by travel edge)
- One guard agent at VillageSquare with:
  - `PatrolRoute { assigned_places: [VillageSquare, GeneralStore], current_index: 0 }`
  - `PatrolProfile` with moderate motive weight
  - `PerceptionProfile` for observation and belief refresh
  - `ViolationMemory` (empty initially)
  - Prior belief that bread exists at GeneralStore (seeded into `AgentBeliefStore` as a place-commodity belief from earlier observation)
- Bread item lot at GeneralStore (matches the guard's prior belief)
- One thief agent who steals the bread while guard is at VillageSquare (theft executed before guard patrols to GeneralStore)

#### Chain
1. Guard selects `Patrol { place: VillageSquare }` on opening tick, completes dwell
2. Guard's plan includes travel to GeneralStore (next waypoint)
3. Meanwhile, thief steals bread from GeneralStore (bread lot removed or relocated)
4. Guard arrives at GeneralStore via patrol travel
5. Perception refresh detects mismatch: guard believed bread was here, but it's gone
6. `EntityMissing` discovery triggers `InvestigateViolation` candidate
7. Guard selects investigation goal, executes `investigate` action
8. Investigation produces `SuspectedTheft` evidence in guard's `ViolationMemory`

#### Assertions
- Guard's first goal is `Patrol` (confirmed via decision trace)
- Guard travels to GeneralStore as part of patrol plan
- After arrival, guard's belief store reflects the missing bread (perception refresh)
- Guard generates `InvestigateViolation` candidate (confirmed via decision trace)
- Guard's `ViolationMemory` contains a `SuspectedTheft` record after investigation completes
- Replay companion confirms deterministic reproduction

#### GoalKinds Exercised
- `Patrol { place }` — drives guard to crime scene
- `InvestigateViolation` — triggered by perception mismatch

#### ActionDomains Exercised
- Generic (patrol dwell, investigate)
- Travel (patrol-driven travel to waypoint)

#### Systems Exercised
- Patrol (candidate generation, route progression)
- Travel (waypoint-to-waypoint movement)
- Perception (belief refresh, stale belief mismatch detection)
- AI (candidate generation, plan search, goal switching from patrol to investigation)
- Crime/Justice (EntityMissing discovery, SuspectedTheft evidence creation)

#### Foundation Principles
- 1 (Maximal Emergence)
- 7 (Locality)
- 14 (World State != Belief State)
- 17 (Surprise from Violated Expectation)

## Ticket Breakdown

### S46PATCRIMDIS-001: Implement Scenario 57 primary test

**Scope**: Write `golden_patrol_driven_crime_discovery` in `golden_patrol.rs`.

**Deliverables**:
- Test function with full setup (guard, thief, topology, beliefs)
- Decision trace assertions for patrol selection and investigation goal switch
- Action trace assertions for patrol commit, travel, investigate
- ViolationMemory assertion for SuspectedTheft evidence
- Structured scenario header comment with metadata annotations

**Acceptance**: `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery` passes.

### S46PATCRIMDIS-002: Implement Scenario 57 replay companion

**Scope**: Write `golden_patrol_driven_crime_discovery_replays_deterministically` in `golden_patrol.rs`.

**Deliverables**:
- Extract shared setup into a helper function returning `(StateHash, StateHash)`
- Run twice with same seed, assert world hash and event log hash match
- Follows existing replay companion pattern (see `run_patrol_cycle` in same file)

**Acceptance**: `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery_replays_deterministically` passes.

### S46PATCRIMDIS-003: Regenerate coverage docs

**Scope**: Run `python3 scripts/golden_inventory.py --write --check-docs` after implementation.

**Deliverables**:
- Updated `docs/generated/golden-e2e-inventory.md`
- Updated `docs/generated/golden-scenario-map.md`
- Updated `docs/generated/golden-coverage-matrix.md`
- Scenario 57 appears in all three generated files

**Acceptance**: Script exits cleanly with no errors.

## Verification

- `cargo test -p worldwake-ai --test golden_patrol` — all patrol goldens pass (existing + new)
- `python3 scripts/golden_inventory.py --write --check-docs` — no duplicate IDs, no missing annotations
- Conservation invariant holds (theft is a lawful transfer, not creation/destruction)
- Replay companion confirms determinism

## Outcome

- **Completion date**: 2026-03-30
- **What changed**: All three tickets (S46GOLGAP-001, S46GOLGAP-002, S46GOLGAP-003) implemented. Scenario 57 (patrol-driven crime discovery) added to `golden_patrol.rs` with primary test + replay companion + coverage docs regenerated.
- **Deviations**: None. Implementation matched spec precisely.
- **Verification**: All 23 patrol golden tests pass. Coverage docs regenerated cleanly (72 scenario blocks across 12 files).
