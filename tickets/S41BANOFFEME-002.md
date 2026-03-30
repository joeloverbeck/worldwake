# S41BANOFFEME-002: Suite 1 — Pressure-Driven Raid Emergence (Scenario 47)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: S41BANOFFEME-001 (spec reassessment — corrections to `enterprise_weight` and setup)

## Problem

`GoalKind::RaidTarget` has zero golden E2E coverage. No golden test proves that bandits initiate raids against co-located non-faction targets under hunger pressure, execute combat through the standard attack action, and loot the defeated target. This is the core offensive behavior of the E18 bandit dynamics system.

## Assumption Reassessment (2026-03-30)

1. **`GoalKind::RaidTarget { target }`** — confirmed live at `crates/worldwake-core/src/goal.rs:134`. Candidate generation via `emit_raid_target_goals()` at `crates/worldwake-ai/src/candidate_generation.rs:1432–1471`.
2. **RaidTarget candidate emission** requires: (a) agent is member of a bandit faction, (b) target is co-located, (c) target is not in the same faction, (d) `danger_pressure < thresholds.danger.high()`. Confirmed via `local_raid_targets()` usage.
3. **RaidTarget ranking** — priority class `Medium` (ranking.rs:344), motive score = `enterprise_weight` (ranking.rs:518). Bandits need non-zero `enterprise_weight` for the raid goal to have a meaningful motive. The existing `bandit_utility_profile()` in T22 sets `enterprise_weight: pm(0)`. **This test must use a custom profile with `enterprise_weight >= pm(300)`** to ensure raid competes with hunger goals.
4. **`GoalKind::LootCorpse { corpse }`** — priority class `Low` (ranking.rs:378–384). Loot is generated as an opportunistic goal after combat kills the target. Suppressed under High stress (goal_policy.rs:162–163).
5. **Attack action** — `"attack"` action definition confirmed in action registries. Combat handler in `crates/worldwake-systems/src/combat.rs`.
6. **Loot action** — `"loot"` action definition confirmed. Handler in `crates/worldwake-systems/src/combat.rs`.
7. **`verify_live_lot_conservation()`** — confirmed at `crates/worldwake-core/src/conservation.rs`, used in multiple golden tests.
8. **Decision traces** — `DecisionOutcome::Planning` contains `candidates.generated` and `selection.selected_goal`. Action traces track `ActionTraceKind::Committed`.
9. **Scenario isolation**: The test places a `ResourceSource` (OrchardRow with Apple) at BanditCamp to verify bandits prefer raiding when a co-located target exists vs. harvesting when alone. Pre-arrival ticks should show non-raid goal selections (harvest/consume). The test intentionally excludes medicine, healers, and other agents to isolate the raid chain.
10. **Existing helpers**: `build_custom_harness()`, `seed_agent_with_recipes()`, `default_perception_profile()`, `bandit_profile()`, `connect()`, `stable_wound_list()`, `set_control_source()`, `add_hostility()` — all confirmed in `golden_t22_bandit_camp_destruction.rs` and `golden_harness/mod.rs`.
11. **No adjacent contradictions** exposed — this test adds coverage for an existing, untested code path.

## Architecture Check

1. Extends `golden_t22_bandit_camp_destruction.rs` — co-locates all bandit golden tests. Reuses existing topology helpers and profile constructors.
2. No new abstractions. Setup is a function returning scenario IDs; the test body is a linear tick loop with accumulating boolean flags (same pattern as existing `run_t22_scenario`).
3. No backwards-compatibility shims.

## Verification Layers

1. Candidate generation emits `RaidTarget` → decision trace: `candidates.generated` contains `RaidTarget { target: traveler }`
2. Goal selection picks `RaidTarget` over alternatives → decision trace: `selection.selected_goal` matches `RaidTarget`
3. Attack action lifecycle → action trace: `ActionTraceKind::Committed` for `"attack"` by bandit targeting traveler
4. Loot action lifecycle → action trace: `ActionTraceKind::Committed` for `"loot"` by bandit
5. Conservation → `verify_live_lot_conservation()` on authoritative world state post-raid
6. No pre-arrival raid → decision traces in pre-arrival ticks show non-`RaidTarget` selections (harvest, consume)
7. Deterministic replay → `hash_world()` + `hash_event_log()` match across two runs with same seed

## What to Change

### 1. Add Suite 1 topology builder

In `golden_t22_bandit_camp_destruction.rs`, add `build_s47_topology()` returning a 3-place topology: BanditCamp, RoadJunction, SafeVillage. New `const` entity IDs for these places (non-overlapping with T22 IDs).

### 2. Add Suite 1 setup function

`seed_s47_scenario(h: &mut GoldenHarness) -> S47Ids` — creates:
- 3 bandits at BanditCamp with `BanditFactionPolicy`, `CombatProfile` (moderate), `HomeostaticNeeds` (hunger >= 700), `MetabolismProfile` (non-zero hunger_rate), `PerceptionProfile`, `UtilityProfile` (enterprise_weight >= pm(300), danger_weight >= pm(700), courage <= pm(200))
- Active `BanditCamp` component with faction-owned supply container (1 Bread — insufficient for 3 agents)
- `ResourceSource` at BanditCamp (OrchardRow, Apple) for harvest alternative
- 1 non-faction traveler at RoadJunction (will travel to BanditCamp) with Apple x4, weak `CombatProfile`, `PerceptionProfile`

### 3. Add `run_s47_scenario(seed: Seed)` function

Linear tick loop (up to ~100 ticks):
1. Pre-arrival phase: step ticks, verify no RaidTarget selection via decision traces
2. Traveler arrives at BanditCamp (or is placed there)
3. Post-arrival: accumulate flags for raid candidate emission, raid selection, attack committed, loot committed
4. Assert all flags, assert conservation, return state hashes

### 4. Add two test functions

- `golden_pressure_driven_raid_emergence` — calls `run_s47_scenario(Seed([47; 32]))`
- `golden_pressure_driven_raid_emergence_replays_deterministically` — calls twice, asserts hash equality

## Files to Touch

- `crates/worldwake-ai/tests/golden_t22_bandit_camp_destruction.rs` (modify — add Suite 1 tests)

## Out of Scope

- Changes to `worldwake-core`, `worldwake-sim`, or `worldwake-systems` crate code
- Modifying existing T22 test functions or setup
- Suite 2 (S41BANOFFEME-003) and Suite 3 (S41BANOFFEME-004) tests
- Golden inventory script updates (S41BANOFFEME-005)
- Belief propagation, Tell actions, merchant rerouting (Suite 2 concerns)
- Wound dampening mechanism (Suite 3 concern)

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_pressure_driven_raid_emergence -- --exact` — main test passes
2. `cargo test -p worldwake-ai golden_pressure_driven_raid_emergence_replays_deterministically -- --exact` — replay test passes
3. `cargo test -p worldwake-ai` — all existing golden tests still pass (no regressions)

### Invariants

1. `verify_live_lot_conservation()` holds after the raid-loot chain — total Apple + Bread quantity is conserved.
2. No RaidTarget selection occurs before traveler is co-located with bandits (emergence, not scripting).
3. Attack and loot actions commit through the standard action lifecycle (ActionTraceKind::Committed), not through any special bandit-specific path.
4. Deterministic replay produces identical `StateHash` for world and event log.

## Test Plan

### New/Modified Tests

1. `golden_pressure_driven_raid_emergence` — proves hunger-driven raid candidate emission, selection, combat execution, and post-combat looting as an emergent chain
2. `golden_pressure_driven_raid_emergence_replays_deterministically` — proves deterministic replay invariant for Suite 1

### Commands

1. `cargo test -p worldwake-ai golden_pressure_driven_raid_emergence` — targeted suite
2. `cargo test -p worldwake-ai` — full AI crate regression
3. `cargo clippy --workspace` ��� no warnings
