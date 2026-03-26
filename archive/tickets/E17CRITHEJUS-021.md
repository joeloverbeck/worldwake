# E17CRITHEJUS-021: Replace placeholder crime ranking with profile-driven motive scoring

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` ranking and shared AI belief/runtime profile surface
**Deps**: `specs/E17-crime-theft-justice.md`, `archive/tickets/E17CRITHEJUS-010.md`, `tickets/E17CRITHEJUS-011.md`

## Problem

The live AI can now generate theft goals, and justice candidate generation is tracked separately, but crime and justice ranking still uses a placeholder path: `GoalKind::StealItem`, `GoalKind::Accuse`, and `GoalKind::PunishAccused` all rank at `GoalPriorityClass::Low` with constant motive score `1` in `crates/worldwake-ai/src/ranking.rs`. That contradicts E17's profile-driven crime architecture and collapses agent diversity for theft and justice choices.

## Assumption Reassessment (2026-03-26)

1. Live ranking arithmetic currently hardcodes crime and justice goals to `GoalPriorityClass::Low` and motive `1` in [`crates/worldwake-ai/src/ranking.rs`](../crates/worldwake-ai/src/ranking.rs). The placeholder-focused regression was `ranking::tests::deferred_crime_and_justice_goals_rank_low_with_minimal_motive`, which this ticket replaces with profile-driven assertions.
2. The active E17 spec still says crime motive weights must come from concrete agent profiles: theft from `TheftDispositionProfile.theft_motive_weight` with witness penalty, justice from `JusticeDispositionProfile.accusation_motive_weight` in [`specs/E17-crime-theft-justice.md`](../specs/E17-crime-theft-justice.md).
3. Shared abstraction boundary under audit: `worldwake_ai::ranking::{priority_class,motive_score}` consuming `worldwake_sim::GoalBeliefView` and emitting ranked `StealItem` / `Accuse` / `PunishAccused` candidates.
4. This is an AI ranking ticket, not candidate generation and not `agent_tick`. The live `GoalKind` surfaces under test are `StealItem`, `Accuse`, and `PunishAccused`; the exact ranking surface is `rank_candidates()`.
5. Ordering dependency is only partially real here. Crime goals already share the same live priority class (`Low`), so cross-family divergence within that band depends on motive score and fallback ordering. But theft motive is actor-profile plus actor-local witness pressure, not target-specific arithmetic, so this ticket cannot and should not claim to reorder same-actor same-place theft targets relative to each other.
6. `JusticeDispositionProfile` already exists as a live core component in [`crates/worldwake-core/src/crime.rs`](../crates/worldwake-core/src/crime.rs) and world schema registration, but the AI-facing read surfaces still do not expose it. `GoalBeliefView` lacks a `justice_disposition_profile()` accessor, and the mirrored runtime/snapshot profile surfaces currently only carry `trade_disposition_profile`, `theft_disposition_profile`, and `violation_disposition_profile`.
7. `E17CRITHEJUS-011` is about justice candidate admission, not ranking. It should not be stretched to carry crime-goal scoring logic because that would duplicate arithmetic across candidate generation and ranking.
8. The current theft candidate emitter already uses witness count as an emission gate. This ticket should not remove that gate. Ranking should use the same concrete substrate to score remaining theft candidates, not invent a second unrelated theft heuristic.
9. No current active ticket owns this weakness. `E17CRITHEJUS-012` and `E17CRITHEJUS-013` are golden proofs; `E17CRITHEJUS-014` is final verification. None of them should silently normalize the placeholder constant-motive path.
10. If justice candidates still do not exist when this ticket is implemented, ranking coverage for `Accuse`/`PunishAccused` can remain focused/unit-level in `ranking.rs` until `E17CRITHEJUS-011` lands. The architecture gap is still valid independently, and the ranking tests must therefore seed justice profile data directly through test doubles instead of waiting on candidate-generation coverage.
11. Adjacent contradiction: the spec text still says crime/justice suppression should begin at `GoalPriorityClass::Medium`, while live `goal_policy.rs` suppresses them at `High`. That is a separate policy mismatch, not the same problem as constant motive ranking, and should not be folded into this ticket without explicit scope expansion.
12. Concrete arithmetic to preserve:
   - Theft motive should remain `max(0, theft_motive_weight - witness_risk_penalty * observed_non_self_agent_count)` using local observed agents at the actor's place.
   - Justice motive should come from `JusticeDispositionProfile.accusation_motive_weight`.
   - Priority class remains `Low` unless the project owner changes the crime priority model separately.

## Architecture Check

1. The clean architecture is to keep crime-goal existence in candidate generation and crime-goal desirability in ranking. That preserves the existing AI separation of concerns instead of smuggling motive arithmetic back into `GroundedGoal` or duplicating it inside candidate emitters.
2. Extending the shared self-authoritative profile surface with `JusticeDispositionProfile` is cleaner than letting ranking reach into broader runtime/world APIs. Crime-goal ranking should stay on the same belief-facing boundary as other motive logic, and the runtime/snapshot mirrors should stay structurally aligned rather than making justice profile a one-off exception.
3. Using the same concrete, local witness substrate for both theft admission and theft ranking aligns with `docs/FOUNDATIONS.md`: no abstract drama dials, no ungrounded crime scoring, and no hidden global knowledge path.
4. No backwards-compatibility aliasing or parallel scoring paths should be introduced. Replace the placeholder constant-motive branch directly.

## Verification Layers

1. Theft ranked motive reflects theft profile and observed witness penalty -> focused `ranking.rs` unit tests
2. Justice ranked motive reflects `JusticeDispositionProfile.accusation_motive_weight` -> focused `ranking.rs` unit tests
3. Theft witness pressure lowers or zeros the actor's theft motive using concrete local state rather than a placeholder constant -> focused `ranking.rs` unit tests
4. Crime ranking still stays within the intended low-priority family -> focused `ranking.rs` unit tests
5. AI crate regression safety after ranking changes -> `cargo test -p worldwake-ai`
6. Lint cleanliness for touched AI/belief-view surfaces -> `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## What to Change

### 1. Replace crime placeholder motive scoring in `ranking.rs`

- Update `motive_score()` so:
  - `GoalKind::StealItem { .. }` uses `TheftDispositionProfile` and local observed witness count at the actor's current place
  - `GoalKind::Accuse { .. }` and `GoalKind::PunishAccused { .. }` use `JusticeDispositionProfile.accusation_motive_weight`
- Keep `GoalPriorityClass::Low` for the crime/justice family unless the owner explicitly wants a policy change.
- Remove or rewrite the focused test that currently locks in the placeholder `1` motive contract.

### 2. Expose justice profile on the AI belief/runtime boundary

- Add `justice_disposition_profile()` to `GoalBeliefView`
- Add the corresponding accessor to the underlying runtime-facing profile surface that `impl_goal_belief_view!` delegates through, keeping snapshot-backed planning state structurally aligned with the live per-agent view
- Forward it through the `impl_goal_belief_view!` macro
- Implement it on the live per-agent/runtime belief view, snapshot/planning-state mirrors if required by the trait surface, and any local test doubles that ranking coverage touches

### 3. Strengthen focused ranking coverage

- Add tests that prove:
  - theft motive equals `theft_motive_weight - witness_risk_penalty * observed_non_self_agent_count` when positive
  - theft motive drops out of ranking when local witness pressure reduces that arithmetic to zero
  - justice goals derive motive from justice profile instead of the placeholder constant
  - low-priority crime goals remain low-priority even after motive becomes profile-driven

## Files to Touch

- `tickets/E17CRITHEJUS-021.md` (modify)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify if runtime profile surface expands)
- `crates/worldwake-ai/src/planning_state.rs` (modify if runtime profile surface expands)
- Any ranking test doubles that implement `GoalBeliefView` (modify as needed)

## Out of Scope

- Justice candidate admission (`E17CRITHEJUS-011`)
- Golden crime scenarios (`E17CRITHEJUS-012`, `E17CRITHEJUS-013`)
- Crime suppression-threshold policy changes in `goal_policy.rs`
- Reworking `GoalPriorityClass` for crime/justice away from `Low`
- New social-evidence transport paths

## Acceptance Criteria

### Tests That Must Pass

1. Theft ranking uses `TheftDispositionProfile` and observed witness penalty instead of constant motive `1`
2. Justice ranking uses `JusticeDispositionProfile.accusation_motive_weight` instead of constant motive `1`
3. Crime/justice goals remain `GoalPriorityClass::Low`
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Crime and justice ordering is driven by concrete local/profile state, not placeholder constants
2. Ranking remains belief-facing and does not widen into direct authoritative-world queries
3. Theft witness deterrence stays grounded in local co-presence, not global head counts or abstract stealth scores

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/ranking.rs` — replace the placeholder crime-ranking test with profile-driven theft/justice motive coverage
2. `crates/worldwake-ai/src/ranking.rs` — add focused tests showing local witness pressure reduces or suppresses theft motive without changing the crime priority family
3. `crates/worldwake-ai/src/planning_state.rs` — extend the existing snapshot/runtime profile roundtrip test to cover `JusticeDispositionProfile` on the mirrored runtime surface

### Commands

1. `cargo test -p worldwake-ai ranking::tests::crime_goals_use_profile_driven_motive_scores`
2. `cargo test -p worldwake-ai ranking::tests`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`

## Outcome

- Completed: 2026-03-26
- Actual changes:
  - Replaced the placeholder constant-motive crime ranking branch in [`crates/worldwake-ai/src/ranking.rs`](../crates/worldwake-ai/src/ranking.rs) with profile-driven theft and justice motive helpers
  - Added `justice_disposition_profile()` to the shared AI belief/runtime surface in [`crates/worldwake-sim/src/belief_view.rs`](../crates/worldwake-sim/src/belief_view.rs) and implemented it in [`crates/worldwake-sim/src/per_agent_belief_view.rs`](../crates/worldwake-sim/src/per_agent_belief_view.rs)
  - Kept snapshot-backed planning state structurally aligned by threading justice profile through [`crates/worldwake-ai/src/planning_snapshot.rs`](../crates/worldwake-ai/src/planning_snapshot.rs) and [`crates/worldwake-ai/src/planning_state.rs`](../crates/worldwake-ai/src/planning_state.rs)
  - Replaced the placeholder ranking regression with profile-driven ranking tests and extended snapshot/runtime profile coverage
- Deviations from original plan:
  - Reassessment showed theft witness pressure is actor-local, not target-specific, so the ticket was corrected to test motive reduction/suppression rather than impossible within-pass theft-target reordering
  - No priority-family or suppression-threshold policy changes were made; crime/justice goals remain `Low` and still use the existing `goal_policy.rs` suppression threshold
- Verification results:
  - `cargo test -p worldwake-ai crime_goals_use_profile_driven_motive_scores`
  - `cargo test -p worldwake-ai theft_goal_is_zero_motive_when_witness_penalty_cancels_profile_weight`
  - `cargo test -p worldwake-ai planning_state_matches_runtime_duration_estimation_for_dynamic_duration_contract`
  - `cargo test -p worldwake-ai ranking::tests`
  - `cargo test -p worldwake-ai`
  - `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
