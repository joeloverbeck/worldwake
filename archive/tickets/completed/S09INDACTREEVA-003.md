# S09INDACTREEVA-003: Switch defend action definition to `ActorDefendStance`

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None — implementation was already present; ticket scope corrected to verification and archival
**Deps**: `specs/S09-indefinite-action-re-evaluation.md`

## Problem

This ticket originally assumed the defend action still used an indefinite runtime duration and needed a small implementation pass to switch it to `DurationExpr::ActorDefendStance`. The current codebase no longer matches that premise. Reassessing the current architecture shows the defend duration migration is already implemented across core, sim, systems, and AI coverage. The correct work for this ticket is to update its assumptions and archive it without redoing delivered engine changes.

## Assumption Reassessment (2026-03-20)

1. `defend_action_def()` in [crates/worldwake-systems/src/combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs#L384) already uses `duration: DurationExpr::ActorDefendStance`; the ticket’s claim that it still used `DurationExpr::Indefinite` was stale.
2. The focused systems assertion already exists in [crates/worldwake-systems/src/combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs#L1607) via `register_defend_action_creates_profile_driven_public_defend_definition`, which asserts `defend.duration == DurationExpr::ActorDefendStance`.
3. The focused runtime lifecycle assertion already exists in [crates/worldwake-systems/src/combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs#L2842) via `defend_affordance_starts_with_finite_profile_duration_and_commits`, which proves the active defend action resolves to `ActionDuration::new(10)`, counts down, commits, and clears stance.
4. The sim-layer duration resolution already exists in [crates/worldwake-sim/src/action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs#L89) and [crates/worldwake-sim/src/belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/belief_view.rs#L678): `DurationExpr::ActorDefendStance` resolves from `CombatProfile.defend_stance_ticks` in both authoritative and belief-estimation paths.
5. `CombatProfile.defend_stance_ticks` already exists in [crates/worldwake-core/src/combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/combat.rs#L8), so the cross-crate data contract this ticket wanted is already established.
6. The broader architecture has progressed beyond the original ticket scope: `ActionDuration` is already a finite-only newtype in [crates/worldwake-sim/src/action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs#L1), and planner cost handling in [crates/worldwake-ai/src/search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs#L360) already consumes `duration.ticks()` with no `Indefinite` special-casing.
7. Existing focused sim coverage already proves the defend duration resolution boundary at both layers:
   - [crates/worldwake-sim/src/per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs#L1494) verifies belief-side duration estimation from `defend_stance_ticks`.
   - [crates/worldwake-sim/src/start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs#L551) verifies authoritative start-time resolution for dynamic defend duration.
8. Existing golden E2E coverage already proves the re-evaluation behavior the ticket deferred to a later step: [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs#L974) contains `golden_defend_replans_after_finite_stance_expires`, which uses action traces and decision traces to prove the agent re-enters the decision pipeline after a seeded finite defend commit.
9. This is not a stale-request, start-failure, control-handoff, or political closure ticket, so those extra boundary sections are not applicable here.
10. Mismatch and correction: the ticket was stale and materially understated delivered architecture and coverage. Scope is corrected from “implement defend duration migration” to “verify existing implementation, document the stronger current architecture, and archive the ticket.”

## Architecture Check

1. The current architecture is better than the original ticket proposed. It did not stop at swapping one systems field; it removed the indefinite-duration architecture entirely from the active runtime shape by making [ActionDuration](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs#L1) finite-only and by resolving defend duration through `CombatProfile.defend_stance_ticks` in both authoritative and belief-estimation paths.
2. That design is cleaner and more extensible because it preserves Principle 8 at the type boundary, keeps per-agent variation in concrete profile state, and avoids planner/runtime branches for a special “forever” action case.
3. No backwards-compatibility aliases or shims are involved. The old indefinite path is gone from the active architecture rather than being preserved behind compatibility logic.

## Verification Layers

1. Defend action definition uses `DurationExpr::ActorDefendStance` -> focused systems unit coverage in [crates/worldwake-systems/src/combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs#L1607)
2. Belief-side duration estimation uses `CombatProfile.defend_stance_ticks` -> focused sim runtime coverage in [crates/worldwake-sim/src/per_agent_belief_view.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/per_agent_belief_view.rs#L1494)
3. Authoritative action start resolves defend to a finite runtime duration -> focused sim runtime coverage in [crates/worldwake-sim/src/start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs#L551)
4. Active defend lifecycle counts down, commits, and clears stance -> focused systems runtime coverage in [crates/worldwake-systems/src/combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/combat.rs#L2842)
5. AI re-enters planning after finite defend expiration -> golden E2E coverage with action-trace and decision-trace assertions in [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs#L974)
6. Additional layer mapping is complete here; later effects are not being used as a proxy for earlier boundaries because the repo already has direct proof surfaces at definition, belief estimation, authoritative start, runtime lifecycle, and golden replan levels.

## What to Change

### 1. Correct the ticket

Rewrite the ticket to reflect the real codebase state:

- defend already uses `DurationExpr::ActorDefendStance`
- defend already resolves to finite profile-driven duration
- existing tests already cover focused resolution, authoritative lifecycle, and AI re-planning

### 2. Archive the ticket

Since the implementation is already delivered and verified, archive the ticket with an Outcome section describing what actually changed versus the original plan.

## Files to Touch

- `tickets/S09INDACTREEVA-003.md` (modify, then archive)

## Out of Scope

- Any new engine or test changes for defend duration
- Repeating already-delivered `DurationExpr` / `ActionDuration` cleanup work
- Refactoring unrelated combat, planner, or CLI code

## Acceptance Criteria

### Tests That Must Pass

1. Existing focused test: `cargo test -p worldwake-sim estimate_duration_uses_actor_defend_stance_ticks_from_combat_profile`
2. Existing focused test: `cargo test -p worldwake-sim start_action_supports_dynamic_defend_duration_when_no_reservations_are_needed`
3. Existing focused test: `cargo test -p worldwake-systems register_defend_action_creates_profile_driven_public_defend_definition`
4. Existing focused test: `cargo test -p worldwake-systems defend_affordance_starts_with_finite_profile_duration_and_commits`
5. Existing focused/golden test: `cargo test -p worldwake-ai golden_defend_replans_after_finite_stance_expires`
6. Existing suite: `cargo test -p worldwake-systems`
7. Existing suite: `cargo test --workspace`
8. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Defend duration remains profile-driven via `CombatProfile.defend_stance_ticks`
2. Defend remains finite at active runtime and therefore naturally re-enters the decision cycle on commit
3. The architecture continues to avoid indefinite action-duration branches and compatibility shims

## Test Plan

### New/Modified Tests

1. [crates/worldwake-sim/src/action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs#L51) — modified `action_duration_roundtrips_through_bincode` to remove a single-element loop that violated `clippy::single-element-loop`; no behavior changed, but this was required for `cargo clippy --workspace --all-targets -- -D warnings` to pass.
2. [crates/worldwake-ai/tests/golden_combat.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_combat.rs#L974) — added `#[allow(clippy::too_many_lines)]` to `golden_defend_replans_after_finite_stance_expires` because the existing golden scenario is intentionally dense and was one line over the configured pedantic threshold; this kept the proven test shape intact while satisfying the repo lint gate.

### Commands

1. `cargo test -p worldwake-sim estimate_duration_uses_actor_defend_stance_ticks_from_combat_profile`
2. `cargo test -p worldwake-sim start_action_supports_dynamic_defend_duration_when_no_reservations_are_needed`
3. `cargo test -p worldwake-systems register_defend_action_creates_profile_driven_public_defend_definition`
4. `cargo test -p worldwake-systems defend_affordance_starts_with_finite_profile_duration_and_commits`
5. `cargo test -p worldwake-ai golden_defend_replans_after_finite_stance_expires`
6. `cargo test -p worldwake-systems`
7. `cargo test --workspace`
8. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completion date**: 2026-03-20
- **What actually changed**: No defend-duration engine changes were required. The ticket was corrected to match the already-delivered defend-duration architecture, and two unrelated test-only lint cleanups were applied so the requested repo-wide `clippy` gate would pass: one in `action_duration_roundtrips_through_bincode`, and one item-level allow on the existing golden defend replan test.
- **Deviations from original plan**: The original plan assumed a pending one-file systems change and deferred AI/golden verification. In reality, the codebase had already completed the broader architecture: `CombatProfile.defend_stance_ticks`, `DurationExpr::ActorDefendStance`, finite-only `ActionDuration`, start-time/belief-time resolution, focused lifecycle tests, and golden AI re-planning coverage.
- **Verification results**: Focused sim and systems defend-duration tests passed during reassessment; broader `cargo test -p worldwake-systems`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and the targeted golden AI defend replan test were run for final confirmation.
