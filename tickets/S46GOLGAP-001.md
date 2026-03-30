# S46GOLGAP-001: Implement golden_patrol_driven_crime_discovery (Scenario 57)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — test-only ticket
**Deps**: S46-golden-gaps-E19.md (spec), all E19/E17/E14/E13/E10 code already landed

## Problem

No golden test demonstrates the *design purpose* of guard patrol — that patrolling brings guards to locations where they discover crime through perception, triggering downstream investigation. Existing patrol scenarios (S52–S56) prove patrol mechanics in isolation. Existing theft-discovery scenarios (S36, S37) prove perception mismatch and investigation without patrol involvement. Scenario 57 fills this cross-system emergence gap.

## Assumption Reassessment (2026-03-30)

1. **PatrolRoute / PatrolProfile**: Exist at `crates/worldwake-core/src/patrol.rs`. `PatrolRoute { assigned_places, current_index }` and `PatrolProfile { base_dwell_ticks, dwell_vigilance_scale_ticks, vigilance, route_adaptation_sensitivity, patrol_motive_weight }`. Confirmed via existing golden_patrol.rs usage (lines 28–36, 38–53).
2. **ViolationMemory / ViolationKind**: Exist at `crates/worldwake-core/src/violation.rs`. `ViolationKind::EntityMissing { entity, expected_place }` and `ViolationKind::SuspectedTheft { theft, suspect }` are the relevant variants. Confirmed via golden_emergent.rs usage (lines ~4449, ~4743, ~5114).
3. **AgentBeliefStore / PerceptionProfile**: Exist at `crates/worldwake-core/src/belief.rs`. `seed_belief` and `seed_belief_from_world` harness helpers exist in `golden_harness/mod.rs` (lines 249–280).
4. **GoalKind::Patrol / GoalKind::InvestigateViolation**: Both exist at `crates/worldwake-core/src/goal.rs`. Patrol variant: `Patrol { place }`. InvestigateViolation variant: `InvestigateViolation { violation_id, place }`.
5. **Live GoalKinds under test**: `GoalKind::Patrol { place }` drives guard to the crime scene. `GoalKind::InvestigateViolation { violation_id, place }` is generated after perception detects EntityMissing. Both goal kinds are exercised by existing golden tests (S52–S56 for Patrol; S36/S37 for InvestigateViolation).
6. **Harness helpers**: `seed_agent`, `give_commodity`, `set_agent_perception_profile`, `seed_belief_from_world`, `new_txn`, `commit_txn` all exist in `golden_harness/mod.rs`. File-local helpers `set_patrol_state`, `default_perception_profile`, `patrol_profile`, `planning_trace_at` exist in `golden_patrol.rs`.
7. **Scenario isolation**: The guard must be the only AI-controlled agent running patrol+investigation goals. The thief's theft is executed manually via world transaction (relocate the bread lot) before the guard arrives — not via the thief's AI. This isolates the test to the patrol→perception→investigation chain without introducing competing AI behavior.
8. **Cross-system boundary**: Patrol (candidate generation, travel) → Perception (belief refresh, stale mismatch) → Crime/Justice (EntityMissing → InvestigateViolation → SuspectedTheft). Each boundary is verified at its own layer per verification layers below.
9. **No existing coverage**: No golden test combines patrol-driven travel with perception-triggered investigation. Searched `golden_patrol.rs` (scenarios 52–56, none chain into crime discovery), `golden_emergent.rs` (S36/S37, no patrol involvement), `golden_social.rs` (no patrol involvement). Gap is at golden/E2E layer.

## Architecture Check

1. This follows the established golden test pattern in `golden_patrol.rs`: file-local helpers, structured scenario comment block, decision/action trace assertions, and state assertions. No new helpers need to be exported from the harness module.
2. No backwards-compatibility shims. The test uses existing types and harness helpers directly.

## Verification Layers

1. Guard selects `Patrol` goal on opening tick → decision trace (`planning_trace_at`, `selection.selected_goal()`)
2. Guard travels to GeneralStore as patrol waypoint → action trace (`ActionTraceKind::Started { targets }` for patrol/travel actions)
3. After arrival, guard's belief store detects missing bread → authoritative world state (`get_component_agent_belief_store`)
4. Guard generates `InvestigateViolation` candidate → decision trace (`candidates.ranked` inspection)
5. Guard commits investigate action → action trace (`ActionTraceKind::Committed` for investigate action)
6. Guard's ViolationMemory contains SuspectedTheft evidence → authoritative world state (`get_component_violation_memory`)
7. Single-layer: conservation is inherently maintained because theft is a lawful relocation, not creation/destruction.

## What to Change

### 1. Add Scenario 57 test function in `golden_patrol.rs`

Write `golden_patrol_driven_crime_discovery` test function with:

- **Topology**: Two places — VillageSquare and GeneralStore (both already exist as `PrototypePlace` variants).
- **Guard agent**: Placed at VillageSquare. Components: `PatrolRoute { assigned_places: [VillageSquare, GeneralStore], current_index: 0 }`, `PatrolProfile` with moderate patrol motive (e.g., 600), `PerceptionProfile` (via `default_perception_profile()`), empty `ViolationMemory`, empty `AgentBeliefStore` (set via `set_patrol_state`).
- **Bread lot**: Created at GeneralStore via `create_item_lot` + `set_ground_location`. Not possessed by anyone — just present at the place.
- **Seed guard's belief**: Use `seed_belief_from_world` to create a belief that the bread lot exists at GeneralStore (observed at Tick(0), source `DirectObservation`). This gives the guard a stale belief that bread is there.
- **Theft**: After guard's first patrol tick at VillageSquare, relocate the bread lot away from GeneralStore via a world transaction (e.g., move it to VillageSquare or remove its ground location). This simulates theft without involving AI.
- **Enable tracing**: `h.driver.enable_tracing()` and `h.enable_action_tracing()`.
- **Step loop**: Run enough ticks for the guard to: (a) complete patrol dwell at VillageSquare, (b) travel to GeneralStore, (c) have perception refresh detect the missing bread, (d) generate InvestigateViolation candidate, (e) commit investigate action.

**Assertions**:
- Opening tick: guard selects `GoalKind::Patrol { place: VillageSquare }` (decision trace).
- Action trace: patrol action started at VillageSquare, then travel to GeneralStore, then patrol started at GeneralStore.
- After arrival at GeneralStore: guard's `ViolationMemory` eventually contains a record with `ViolationKind::EntityMissing` for the bread lot (perception-driven mismatch).
- Decision trace: at some tick after arrival, guard generates `InvestigateViolation` candidate.
- Action trace: investigate action committed.
- Final state: guard's `ViolationMemory` contains a `ViolationKind::SuspectedTheft` record (investigate resolved the EntityMissing into typed evidence).

**Scenario comment block** following established format:
```
// Scenario 57: Patrol-Driven Crime Discovery Chain
// Systems: AI, Travel, Patrol, Perception, Crime/Justice
// GoalKinds: Patrol, InvestigateViolation
// ActionDomains: Travel, Generic
// Places: VillageSquare, GeneralStore
// Principles: 1, 7, 14, 17
```

## Files to Touch

- `crates/worldwake-ai/tests/golden_patrol.rs` (modify — add test function and any file-local helpers)

## Out of Scope

- Replay companion test (ticket S46GOLGAP-002).
- Doc regeneration (ticket S46GOLGAP-003).
- Any engine/production code changes — this is test-only.
- Adding new golden harness helpers to `golden_harness/mod.rs` (use existing helpers; if a new helper is truly needed, it should be file-local in `golden_patrol.rs`).
- Testing the thief's AI behavior or theft action handler — theft is simulated manually.
- Modifying existing scenarios S52–S56.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery` — new test passes.
2. `cargo test -p worldwake-ai --test golden_patrol` — all existing patrol golden tests (S52–S56) still pass.
3. `cargo clippy -p worldwake-ai` — no new warnings.

### Invariants

1. Guard discovers crime only through physical arrival at the crime scene via patrol travel — no remote awareness (FND-7).
2. Guard's investigation triggers from stale belief mismatch against observed state, not from authoritative world truth (FND-14, FND-17).
3. Conservation: bread lot is relocated, not destroyed — no conservation violation.
4. Existing patrol scenarios S52–S56 continue to pass unchanged.
5. Determinism: the test must produce the same outcome for a given seed (verified by replay companion in S46GOLGAP-002).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_patrol.rs::golden_patrol_driven_crime_discovery` — proves the cross-system patrol→perception→investigation chain end to end (Scenario 57).

### Commands

1. `cargo test -p worldwake-ai --test golden_patrol golden_patrol_driven_crime_discovery`
2. `cargo test -p worldwake-ai --test golden_patrol`
3. `cargo clippy -p worldwake-ai`
