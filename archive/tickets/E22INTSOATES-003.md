# E22INTSOATES-003: T28 — Pursuit Across Information Boundary

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E22INTSOATES-001

## Problem

Existing pursuit goldens (`golden_pursuit.rs`, Scenarios 68–70) test basic 2-3 place pursuit with fresh beliefs. T28 tests a longer pursuit where information staleness causes honest failure, exercises the violation → replan cycle, and verifies information delay is physical — not omniscient correction.

## Assumption Reassessment (2026-03-31)

1. `PursuitProfile` exists with `min_location_confidence` and `max_pursuit_travel_ticks` fields — confirmed in `crates/worldwake-core/src/pursuit.rs`.
2. `ViolationKind::EntityMissing` exists — confirmed in `crates/worldwake-core/src/violation.rs`.
3. `ViolationMemory` component exists with `record()` and `is_recorded()` APIs — confirmed.
4. `GoalKind::EngageHostile` exists — confirmed in `crates/worldwake-core/src/goal.rs`.
5. `BanditCamp`, `BanditFactionPolicy` exist — confirmed.
6. `golden_pursuit.rs` has 3 scenarios (68–70) covering remote pursuit, loot-after-kill, belief-staleness recovery — confirmed. T28 adds: longer chain (4 places), explicit `ViolationKind::EntityMissing` assertion, pursuit budget enforcement.
7. `ActionDomain::Epistemic` exists — confirmed in `ActionDomain` enum.
8. Decision trace can show goal switching and replan — confirmed via `DecisionTraceSink`.
9. T28 isolates pursuit from trade/political/crime systems. Setup uses only bandit + target in a linear 4-place topology.
10. No adjacent contradictions.
11. T28 tick budget is ≤ 50 ticks. 4 places with 3-tick travel edges. Target moves before bandit arrives.

## Architecture Check

1. T28 exercises existing pursuit + violation systems through the standard action framework. No new action types or goal kinds needed. The test proves the existing information-boundary behavior works across a longer chain.
2. No backwards-compatibility shims introduced.

## Verification Layers

1. Bandit perception of target → decision trace (initial `EngageHostile` candidate generated)
2. Target departure before bandit arrival → authoritative world state (target placement at Village, not Crossroads, when bandit arrives)
3. Violation detection → authoritative component state (`ViolationKind::EntityMissing` in bandit's `ViolationMemory`)
4. Pursuit bounded by profile → decision trace or action trace (bandit does not pursue beyond `max_pursuit_travel_ticks`)
5. No teleportation → event-log delta (all movement through `TravelEdge` traversal)
6. Belief-only planning (Principle 14) → decision trace (bandit acts on `BelievedEntityState`, not world state)
7. Cross-domain coverage → event-log scan (≥ 2 `ActionDomain` values — Travel + Generic; relaxed from ≥ 3 since combat doesn't occur in a pursuit failure scenario)
8. Determinism → state hash comparison across 2 seeds

## What to Change

### 1. Add T28 scenario to `crates/worldwake-ai/tests/golden_integration.rs`

- Build 4-place linear topology: Hideout → Crossroads → Village → Sanctuary (3-tick edges)
- Bandit at Hideout with `PursuitProfile { min_location_confidence: Permille(600), max_pursuit_travel_ticks: NonZeroU32(8) }`, `CombatProfile`, `BanditCamp` membership
- Target at Crossroads with `CommodityKind::Gold`, `ControlSource::Ai`, beliefs/goals seeded to travel toward Village
- Bandit perceives target at Crossroads
- Target travels to Village before bandit arrives at Crossroads
- Bandit arrives at Crossroads → target absent → `ViolationKind::EntityMissing`
- Bandit replans based on stale belief and pursuit confidence
- Verify all acceptance criteria from spec
- `fn run_t28_pursuit_information_boundary(seed: Seed) -> (StateHash, StateHash)`
- Two `#[test]` functions

## Files to Touch

- `crates/worldwake-ai/tests/golden_integration.rs` (modify)

## Out of Scope

- Changes to pursuit system, violation system, or perception
- Modifying existing `golden_pursuit.rs` scenarios
- Any engine code changes

## Acceptance Criteria

### Tests That Must Pass

1. `t28_pursuit_information_boundary_seed_1` — bandit pursues, finds empty location, records violation, pursuit bounded
2. `t28_pursuit_information_boundary_seed_2` — determinism verification
3. Bandit does NOT teleport to target at any tick
4. `ViolationKind::EntityMissing` appears in bandit's `ViolationMemory` after arrival at empty Crossroads
5. Pursuit bounded by `PursuitProfile.max_pursuit_travel_ticks`
6. Event log crosses ≥ 2 `ActionDomain` values (Travel + Generic). Relaxed from original ≥ 3: combat doesn't occur in a pursuit failure scenario (target escapes), and `investigate` uses `Generic` domain, not `Epistemic`.
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Information delay is physical: bandit acts on belief state, not world state (Principle 14)
2. All agent movement through physical `TravelEdge` traversal — no teleportation
3. Violation memory correctly records the information-boundary failure

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_integration.rs::t28_pursuit_information_boundary_seed_1` — proves pursuit failure from stale beliefs
2. `crates/worldwake-ai/tests/golden_integration.rs::t28_pursuit_information_boundary_seed_2` — determinism

### Commands

1. `cargo test -p worldwake-ai --test golden_integration -- t28`
2. `cargo test --workspace`

## Outcome

- **Completion date**: 2026-03-31
- **What changed**: Added T28 scenario (`t28_pursuit_information_boundary_seed_1`, `t28_pursuit_information_boundary_seed_2`) to `crates/worldwake-ai/tests/golden_integration.rs`. Tests a 4-place linear pursuit where stale beliefs cause the bandit to arrive at an empty location, recording `ViolationKind::EntityMissing`, with pursuit bounded by `PursuitProfile.max_pursuit_travel_ticks`.
- **Deviations from original plan**: None
- **Verification**: `cargo test -p worldwake-ai --test golden_integration -- t28` passes; `cargo test --workspace` passes
