# E20COMBEH-007: Golden tests — relief fallback (latrine preferred, wilderness, deprivation)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None — tests only
**Deps**: E20COMBEH-004 (wilderness relief action), E20COMBEH-005 (planner integration)

## Problem

The wilderness relief fallback feature needs golden E2E coverage to verify that: (a) the planner prefers latrines over wilderness relief when both are available, (b) wilderness relief works correctly as a fallback at outdoor places, and (c) deprivation accident still occurs when no relief option is available. These are spec-required golden tests T-LatrinePreferred, T-WildernessFallback, and T-DeprivationAccident.

## Assumption Reassessment (2026-03-30)

1. **Planner preference**: The planner explores plans in cost order. `toilet` at a latrine has no secondary dirtiness cost. `relieve_wilderness` increases dirtiness. However, the planner doesn't directly model dirtiness penalty as a cost — preference comes from travel distance. T-LatrinePreferred should set up a scenario where the agent is AT a latrine with high bladder, so `toilet` is zero-travel-cost and should be chosen over `relieve_wilderness` (even if outdoor tags are also present on the place or nearby).
2. **GoalKind::Relieve**: Live goal kind, maps to both `toilet` and `relieve_wilderness` via `PlannerOpKind::Relieve` (after E20COMBEH-005).
3. **Deprivation accident**: Handled by `needs_system` in `crates/worldwake-systems/src/needs.rs` when `DeprivationExposure` ticks exceed `bladder_accident_tolerance_ticks`. Creates waste and max dirtiness. This is existing E09 behavior.
4. **Isolation strategy**: T-LatrinePreferred: agent at place with both Latrine tag and an outdoor tag → planner should choose toilet. T-WildernessFallback: agent at outdoor-only place (no Latrine) → planner should choose relieve_wilderness. T-DeprivationAccident: agent at indoor-only place (no Latrine, no outdoor tag) → no relief action available → deprivation accident.
5. **PerceptionProfile**: Not needed for T-LatrinePreferred or T-WildernessFallback (testing AI decision, not observation). Not needed for T-DeprivationAccident (testing needs system consequence).

## Architecture Check

1. Three focused golden tests, each isolating one branch of the relief decision tree. Clear scenario isolation: place tags determine which actions are available, and the planner's search produces the expected choice.
2. No backward-compatibility shims. These are new tests.

## Verification Layers

1. Latrine preference → decision trace (plan search finds toilet, not relieve_wilderness) + action trace (toilet started/committed)
2. Wilderness fallback → decision trace (plan search finds relieve_wilderness) + action trace (relieve_wilderness committed) + authoritative world state (waste created, dirtiness increased)
3. Deprivation accident → authoritative world state (deprivation wound created, waste created, max dirtiness) + action trace (no relief action started)
4. These are golden E2E tests — they verify the full pipeline.

## What to Change

### 1. T-LatrinePreferred golden test

**Setup**: One agent with high bladder (above critical threshold) at a place tagged `Latrine` + `Village` (and optionally `Road` to make outdoor relief also available). Agent has `MetabolismProfile` with non-zero `wilderness_relief_dirtiness_penalty`.

**Assert**: Agent's plan selects `toilet` action. After execution: bladder = `Permille(0)`, dirtiness unchanged (no wilderness penalty), waste created at place.

### 2. T-WildernessFallback golden test

**Setup**: One agent with high bladder (above critical threshold) at a place tagged `Forest` only (no `Latrine` tag). No latrine reachable within planner search depth.

**Assert**: Agent's plan selects `relieve_wilderness`. After execution: bladder = `Permille(0)`, dirtiness increased by `wilderness_relief_dirtiness_penalty`, waste created at place.

### 3. T-DeprivationAccident golden test

**Setup**: One agent with high bladder at a place tagged `Inn` only (indoor, no outdoor tag, no `Latrine`). No reachable place with latrine or outdoor tag within planner budget. Agent's `bladder_accident_tolerance_ticks` set low so accident triggers quickly.

**Assert**: No relief action started (plan search finds nothing). Bladder continues rising. After tolerance exceeded: deprivation accident event, waste created, dirtiness at maximum.

## Files to Touch

- `crates/worldwake-ai/src/golden_tests/` (new test file or addition to existing golden test module)

## Out of Scope

- Travel physiology golden tests (E20COMBEH-006)
- Witness/social golden tests (E20COMBEH-008)
- Unit tests for individual components (covered in E20COMBEH-001 through E20COMBEH-005)
- Changes to production code (this ticket is tests only)
- Planner cost modeling of dirtiness penalty (out of scope for this epic)

## Acceptance Criteria

### Tests That Must Pass

1. `golden_latrine_preferred` — agent at latrine chooses toilet over relieve_wilderness
2. `golden_wilderness_fallback` — agent at outdoor place with no latrine uses relieve_wilderness, dirtiness increases, waste created
3. `golden_deprivation_accident` — agent with no relief option suffers accident after tolerance exceeded
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Waste conservation: every relief event (toilet, wilderness, accident) creates exactly one Waste entity
2. Bladder always goes to exactly `Permille(0)` after successful relief
3. Dirtiness penalty only applied for wilderness relief, not toilet
4. Agent symmetry: no agent type exempt from any behavior

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/golden_tests/` — `golden_latrine_preferred` — planner preference verification
2. `crates/worldwake-ai/src/golden_tests/` — `golden_wilderness_fallback` — fallback action + effects
3. `crates/worldwake-ai/src/golden_tests/` — `golden_deprivation_accident` — no-option deprivation

### Commands

1. `cargo test -p worldwake-ai golden_latrine`
2. `cargo test -p worldwake-ai golden_wilderness`
3. `cargo test -p worldwake-ai golden_deprivation`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-30
- What changed:
  - Added three golden tests to `crates/worldwake-ai/tests/golden_travel_physiology.rs` (Scenarios 5, 6, 7):
    - `golden_latrine_preferred` — agent at PublicLatrine with critical bladder commits `toilet`, not `relieve_wilderness`; bladder resets, dirtiness unchanged, waste created
    - `golden_wilderness_fallback` — agent at ForestPath (outdoor, no latrine) commits `relieve_wilderness`; bladder resets, dirtiness increases by `wilderness_relief_dirtiness_penalty`, waste created
    - `golden_deprivation_accident` — agent at CommonHouse (indoor, no latrine, no outdoor tags) with `bladder_accident_tolerance_ticks=1`; no relief action commits, needs system fires deprivation accident, bladder resets, dirtiness spikes, waste created
  - Added constants `COMMON_HOUSE` and `FOREST_PATH` to the test file
- Deviations from original plan:
  - Ticket assumed `bladder_accident_tolerance_ticks=nz(3)` was sufficient for deprivation isolation. Actual tick ordering (actions process before systems) means the agent can travel away from CommonHouse before the needs system fires. Changed to `nz(1)` and checked waste at agent's effective_place rather than hardcoded CommonHouse.
  - Latrine preference and wilderness fallback tests use a break-at-commit loop instead of fixed tick count, because needs values are transient (basal drift re-escalates after relief commit).
  - `PerceptionProfile` not needed, as stated in ticket assumption 5 — confirmed correct.
  - Filed follow-up `tickets/GOLDOC-001.md` to document the ordering trap and transient-state assertion pattern in `docs/golden-e2e-testing.md`.
- Verification results:
  - `cargo test -p worldwake-ai --test golden_travel_physiology golden_latrine_preferred` passed
  - `cargo test -p worldwake-ai --test golden_travel_physiology golden_wilderness_fallback` passed
  - `cargo test -p worldwake-ai --test golden_travel_physiology golden_deprivation_accident` passed
  - `cargo test -p worldwake-ai` passed (all 888 tests)
  - `cargo test --workspace` passed
  - `cargo clippy -p worldwake-ai` passed (no warnings)
