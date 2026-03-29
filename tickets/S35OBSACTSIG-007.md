# S35OBSACTSIG-007: Golden test — competition-aware harvest avoidance

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None — test only
**Deps**: S35OBSACTSIG-001 through S35OBSACTSIG-006 (all behavioral tickets)

## Problem

The full observable-activity pipeline needs an end-to-end golden test proving that agents actually change behavior based on observed competition. The spec requires: "Two agents at same harvest source — second agent (high `activity_awareness_weight`) discounts occupied source and picks alternative; first agent (low awareness) would not be deterred." Plus a deterministic replay companion.

## Assumption Reassessment (2026-03-29)

1. Golden tests live in `crates/worldwake-ai/tests/` as integration tests. Existing golden tests (e.g., `golden_harvest_*.rs`, `golden_trade_*.rs`) provide the pattern.
2. Golden tests use `GoldenTestHarness` (or equivalent) which wraps `SimulationState` + `AgentTickDriver`.
3. `enable_tracing()` on the driver enables decision traces for debugging.
4. `enable_action_tracing()` on the harness enables action lifecycle traces.
5. Agents require `PerceptionProfile` to observe entities and activities.
6. The test scenario needs: two places each with a resource source, two agents with different `activity_awareness_weight`, one agent already harvesting at source A, second agent choosing between source A (occupied) and source B (unoccupied).
7. Deterministic replay is verified via `replay_and_verify()` — standard golden test pattern.
8. After all S35 tickets are complete, `perception_system()` will observe active actions, `rank_candidates()` will apply competition discount, and agents will prefer uncontested resources.
9. Any future perception-layer cleanup that unifies passive/entity and active/activity direct-observation bookkeeping is not required to prove the end-to-end invariant in this ticket. This golden should verify behavior through the public pipeline, not absorb an internal refactor.

## Architecture Check

1. A dedicated golden test file isolates the competition-avoidance scenario cleanly.
2. Two resource sources at two places ensures the agent has a genuine alternative — not testing suppression, but redirection.
3. Different `activity_awareness_weight` values demonstrate P20 (agent diversity).
4. Replay companion ensures determinism (P11 — performance may compress computation, never causality).
5. This ticket should remain proof-only. If the scenario exposes awkward perception internals, that follow-up belongs in a separate perception ticket rather than being smuggled into the golden.

## Verification Layers

1. Agent with high awareness avoids occupied source -> golden test assertion (agent's chosen goal/action targets source B)
2. Agent with low awareness does NOT avoid occupied source -> golden test assertion (or separate setup showing no avoidance)
3. Competition discount visible in decision trace -> trace assertion
4. Deterministic replay produces same outcome -> replay companion
5. Multi-layer: perception -> belief -> ranking -> plan -> action execution

## What to Change

### 1. Create golden test file

Create `crates/worldwake-ai/tests/golden_competition_avoidance.rs` (or add scenario to existing golden test file per project conventions).

### 2. Test scenario setup

- **World**: Two places (Orchard A, Orchard B) connected to a central place. Each has a resource source (e.g., apples).
- **Agent Alpha**: At Orchard A, already harvesting. `activity_awareness_weight: Permille(200)` (default — moderate awareness).
- **Agent Beta**: At central place, needs to harvest. `activity_awareness_weight: Permille(500)` (high awareness). Has recipes and tools for harvesting.
- **Agent Gamma** (optional control): Same setup as Beta but with `activity_awareness_weight: Permille(0)` to verify no avoidance.
- All agents have `PerceptionProfile` with adequate `observation_fidelity`.

### 3. Test assertions

- After sufficient ticks for Beta to observe Alpha's activity and plan:
  - Beta's chosen goal targets Orchard B (the unoccupied source), not Orchard A.
  - Decision trace shows `CompetitionDiscount` applied to the Orchard A opportunity.
- If Gamma is included: Gamma may still choose Orchard A despite Alpha's presence (no discount with weight 0).
- Replay: `replay_and_verify()` produces identical outcome.

### 4. Update golden test inventory

Run `python3 scripts/golden_inventory.py --write --check-docs` to regenerate test inventories.

## Files to Touch

- `crates/worldwake-ai/tests/golden_competition_avoidance.rs` (new)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-map.md` (regenerated)

## Out of Scope

- Any production code changes — this is test-only
- Any perception-pipeline refactor that unifies passive/entity and active/activity direct-observation bookkeeping
- Trade competition golden test (could be a follow-up)
- Multi-agent competition beyond 2-3 agents
- Performance benchmarking

## Acceptance Criteria

### Tests That Must Pass

1. `golden_competition_avoidance` test passes: high-awareness agent avoids occupied resource.
2. Decision trace shows `CompetitionDiscount` on the occupied-source goal for the high-awareness agent.
3. Deterministic replay produces identical outcome.
4. All existing golden tests still pass: `cargo test -p worldwake-ai`

### Invariants

1. Agent behavior change is driven by beliefs (observed activity), not authoritative state (P12).
2. The occupied source is still available — the agent chose to avoid it, not forced (discount, not suppression).
3. Deterministic replay reproduces the same avoidance decision given the same seed.
4. Perception requires `PerceptionProfile` — agents without profiles do not observe activity.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_competition_avoidance.rs` — full E2E golden test with decision trace assertions and replay companion.

### Commands

1. `cargo test -p worldwake-ai -- golden_competition_avoidance`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
