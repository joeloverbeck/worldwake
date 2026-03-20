# S16S09GOLVAL-002: Golden — Defend Re-Evaluation Under Changed Conditions

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - dead hostiles no longer project combat pressure through belief/planning hostility queries
**Deps**: archive/tickets/completed/S16S09GOLVAL-001.md (shared helpers in harness)

## Problem

The existing defend golden proved defend lifecycle only: defend commits, the agent re-enters the decision pipeline, and the agent later does something. It did not prove the changed-conditions boundary that matters for Principle 19: when the threat dies during the defend window, the defender must stop selecting the combat branch and re-evaluate into a lawful non-combat goal.

## Assumption Reassessment (2026-03-20)

1. `golden_defend_replans_after_finite_stance_expires` exists at `crates/worldwake-ai/tests/golden_combat.rs` and still only proves lifecycle, not changed-conditions goal selection.
2. No pre-existing golden covered "attacker dies during defend stance -> next selected goal is non-combat."
3. The active spec reference is `specs/S16-s09-golden-validation.md`.
4. The current eating goal is `GoalKind::ConsumeOwnedCommodity { commodity: ... }`, not `ConsumeCommodity`.
5. `ReduceDanger` is pressure-derived through hostility surfaces such as `visible_hostiles_for`, `hostile_targets_of`, and `current_attackers_of`; it is not a flat "hostile exists" predicate.
6. The original ticket assumption that this could remain tests-only was wrong. The new golden exposed a real architectural mismatch: dead hostiles still leaked through runtime belief and planning-state hostility queries, so combat pressure could remain artificially alive after the attacker died.
7. The correct architectural fix is at the hostility-query layer, not a `ReduceDanger` special case and not a post-defend fallback heuristic. Dead believed entities should stop contributing to combat affordances and danger pressure everywhere those shared query surfaces are used.
8. `no_recovery_combat_profile()` and `stable_wound_list()` already existed in the golden harness from `archive/tickets/completed/S16S09GOLVAL-001.md`.
9. The clean verification boundary for this ticket is changed-conditions replanning: once defend has resolved and the attacker is dead, the first post-resolution selected goal must no longer be `ReduceDanger`.
10. The original proposal to also assert downstream eat/heal execution was broader than necessary for this ticket. That behavior remains useful, but the key invariant here is selection-boundary correctness after conditions change.

## Architecture Check

1. The resulting architecture is better than the pre-fix state. Before the fix, dead hostiles remained visible/actionable in shared hostility queries, which is the wrong abstraction: it let stale combat pressure survive past authoritative death.
2. Fixing `per_agent_belief_view` and `planning_state` is cleaner and more extensible than patching `derive_danger_pressure()` or hard-coding a defend-specific replan rule. All current and future danger/hostility consumers now inherit the correct dead-target filtering automatically.
3. The golden belongs next to the existing defend lifecycle golden, but it should remain a separate test. Lifecycle and changed-conditions re-evaluation are distinct contracts and fail for different reasons.
4. No backwards-compatibility shims, aliases, or dual paths were introduced.

## Verification Layers

1. Seeded defend resolves on schedule -> action trace / authoritative action lifecycle
2. Attacker dies during the defend window -> authoritative world state
3. First post-resolution selected goal is not `ReduceDanger` -> decision trace
4. Dead hostiles are excluded from runtime belief hostility queries -> focused unit test
5. Dead hostiles are excluded from planning snapshot hostility queries -> focused unit test
6. Deterministic replay holds across two identical seeded runs -> world hash + event log hash

## What Changed

### 1. Golden coverage

Added a new defend changed-conditions scenario in `crates/worldwake-ai/tests/golden_combat.rs` by reusing the live-combat scaffold rather than building a separate VillageSquare setup from scratch. The scenario keeps the defender in a seeded finite defend stance while the attacker is made fragile enough to die during that window and unable to kill the defender first.

Assertions now prove:

1. the seeded defend action resolves,
2. the attacker dies during that window,
3. the first post-resolution selected goal is no longer `ReduceDanger`, and
4. the scenario replays deterministically.

### 2. Engine/runtime fix

Corrected dead-target filtering in:

1. `crates/worldwake-sim/src/per_agent_belief_view.rs`
2. `crates/worldwake-ai/src/planning_state.rs`

This removes dead hostiles from shared hostility/query surfaces instead of special-casing a single planner branch.

### 3. Focused regression coverage

Added focused tests for both runtime belief state and planning snapshot state so the dead-hostile invariant is pinned below the golden layer.

## Files Touched

- `crates/worldwake-ai/tests/golden_combat.rs`
- `crates/worldwake-sim/src/per_agent_belief_view.rs`
- `crates/worldwake-ai/src/planning_state.rs`
- `tickets/S16S09GOLVAL-002.md`

## Out of Scope

- Rewriting defend/combat architecture beyond the dead-hostility leak
- Adding fallback heuristics or branch-specific aliases for `ReduceDanger`
- Modifying unrelated existing goldens
- Asserting the exact downstream winner between eating and self-treatment in this ticket
- Multi-agent divergence coverage from `S16S09GOLVAL-003`

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_defend_changed_conditions -- --exact`
2. `cargo test -p worldwake-ai golden_defend_changed_conditions_replays_deterministically -- --exact`
3. `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::visible_hostiles_exclude_dead_believed_targets -- --exact`
4. `cargo test -p worldwake-ai --lib planning_state::tests::dead_hostiles_are_not_visible_or_actionable_in_snapshot_state -- --exact`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace`
8. `scripts/verify.sh`

### Invariants

1. Append-only event log is never mutated
2. Conservation invariants hold
3. Determinism holds for identical seeds
4. Existing defend lifecycle coverage still passes unchanged
5. Dead entities no longer remain visible/actionable through shared hostility-query surfaces

## Test Plan

### New/Modified Tests

1. `golden_defend_changed_conditions` in `crates/worldwake-ai/tests/golden_combat.rs`
2. `golden_defend_changed_conditions_replays_deterministically` in `crates/worldwake-ai/tests/golden_combat.rs`
3. `visible_hostiles_exclude_dead_believed_targets` in `crates/worldwake-sim/src/per_agent_belief_view.rs`
4. `dead_hostiles_are_not_visible_or_actionable_in_snapshot_state` in `crates/worldwake-ai/src/planning_state.rs`

### Commands Run

1. `cargo test -p worldwake-ai golden_defend_changed_conditions -- --exact`
2. `cargo test -p worldwake-ai golden_defend_changed_conditions_replays_deterministically -- --exact`
3. `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::visible_hostiles_exclude_dead_believed_targets -- --exact`
4. `cargo test -p worldwake-ai --lib planning_state::tests::dead_hostiles_are_not_visible_or_actionable_in_snapshot_state -- --exact`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace`
8. `scripts/verify.sh`

## Outcome

Completed on 2026-03-20.

What actually changed vs. originally planned:

1. Added the new changed-conditions golden and deterministic replay companion.
2. Added focused regression tests at the runtime belief and planning snapshot layers.
3. Fixed a real engine bug the golden exposed: dead hostiles were still visible/actionable through shared hostility-query surfaces and could keep `ReduceDanger` alive after the attacker died.
4. Narrowed the golden's main behavioral assertion to the clean boundary that matters here: after defend resolves and the attacker is dead, the first selected follow-up goal is non-combat. The original proposal's downstream eat/heal execution assertion was broader than needed for this ticket and is not the core invariant.
