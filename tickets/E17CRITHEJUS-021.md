# E17CRITHEJUS-021: Replace placeholder crime ranking with profile-driven motive scoring

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` ranking and AI belief-view surface
**Deps**: `specs/E17-crime-theft-justice.md`, `archive/tickets/E17CRITHEJUS-010.md`, `tickets/E17CRITHEJUS-011.md`

## Problem

The live AI can now generate theft goals, and justice candidate generation is tracked separately, but crime and justice ranking still uses a placeholder path: `GoalKind::StealItem`, `GoalKind::Accuse`, and `GoalKind::PunishAccused` all rank at `GoalPriorityClass::Low` with constant motive score `1` in `crates/worldwake-ai/src/ranking.rs`. That contradicts E17's profile-driven crime architecture and collapses agent diversity for theft and justice choices.

## Assumption Reassessment (2026-03-26)

1. Live ranking arithmetic currently hardcodes crime and justice goals to `GoalPriorityClass::Low` and motive `1` in [`crates/worldwake-ai/src/ranking.rs`](../crates/worldwake-ai/src/ranking.rs). The focused regression proving that placeholder contract is `ranking::tests::deferred_crime_and_justice_goals_rank_low_with_minimal_motive`.
2. The active E17 spec still says crime motive weights must come from concrete agent profiles: theft from `TheftDispositionProfile.theft_motive_weight` with witness penalty, justice from `JusticeDispositionProfile.accusation_motive_weight` in [`specs/E17-crime-theft-justice.md`](../specs/E17-crime-theft-justice.md).
3. Shared abstraction boundary under audit: `worldwake_ai::ranking::{priority_class,motive_score}` consuming `worldwake_sim::GoalBeliefView` and emitting ranked `StealItem` / `Accuse` / `PunishAccused` candidates.
4. This is an AI ranking ticket, not candidate generation and not `agent_tick`. The live `GoalKind` surfaces under test are `StealItem`, `Accuse`, and `PunishAccused`; the exact ranking surface is `rank_candidates()`.
5. Ordering dependency is real here. Crime goals already share the same live priority class (`Low`), so current divergence within that band depends entirely on motive score and fallback ordering. With constant motive `1`, the engine loses the spec-required profile-driven diversity and tie-breaks mostly by stable kind/entity ordering.
6. The live `GoalBeliefView` exposes `theft_disposition_profile` after `E17CRITHEJUS-010`, but it does not currently expose `JusticeDispositionProfile`. This ticket will likely need that surface added to keep ranking belief-facing and not runtime-coupled.
7. `E17CRITHEJUS-011` is about justice candidate admission, not ranking. It should not be stretched to carry crime-goal scoring logic because that would duplicate arithmetic across candidate generation and ranking.
8. The current theft candidate emitter already uses witness count as an emission gate. This ticket should not remove that gate. Ranking should use the same concrete substrate to score remaining theft candidates, not invent a second unrelated theft heuristic.
9. No current active ticket owns this weakness. `E17CRITHEJUS-012` and `E17CRITHEJUS-013` are golden proofs; `E17CRITHEJUS-014` is final verification. None of them should silently normalize the placeholder constant-motive path.
10. If justice candidates still do not exist when this ticket is implemented, ranking coverage for `Accuse`/`PunishAccused` can remain focused/unit-level in `ranking.rs` until `E17CRITHEJUS-011` lands. The architecture gap is still valid independently.
11. Adjacent contradiction: the spec text still says crime/justice suppression should begin at `GoalPriorityClass::Medium`, while live `goal_policy.rs` suppresses them at `High`. That is a separate policy mismatch, not the same problem as constant motive ranking, and should not be folded into this ticket without explicit scope expansion.
12. Concrete arithmetic to preserve:
   - Theft motive should remain `max(0, theft_motive_weight - witness_risk_penalty * observed_non_self_agent_count)` using local observed agents at the actor's place.
   - Justice motive should come from `JusticeDispositionProfile.accusation_motive_weight`.
   - Priority class remains `Low` unless the project owner changes the crime priority model separately.

## Architecture Check

1. The clean architecture is to keep crime-goal existence in candidate generation and crime-goal desirability in ranking. That preserves the existing AI separation of concerns instead of smuggling motive arithmetic back into `GroundedGoal` or duplicating it inside candidate emitters.
2. Extending `GoalBeliefView` with `JusticeDispositionProfile` is cleaner than letting ranking reach into broader runtime/world APIs. Crime-goal ranking should stay on the same belief-facing boundary as other motive logic.
3. Using the same concrete, local witness substrate for both theft admission and theft ranking aligns with `docs/FOUNDATIONS.md`: no abstract drama dials, no ungrounded crime scoring, and no hidden global knowledge path.
4. No backwards-compatibility aliasing or parallel scoring paths should be introduced. Replace the placeholder constant-motive branch directly.

## Verification Layers

1. Theft ranked motive reflects theft profile and observed witness penalty -> focused `ranking.rs` unit tests
2. Justice ranked motive reflects `JusticeDispositionProfile.accusation_motive_weight` -> focused `ranking.rs` unit tests
3. Same-priority crime goal ordering changes with concrete profile math rather than stable fallback ordering -> focused `ranking.rs` unit tests
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

### 2. Expose justice profile on the AI belief-view boundary

- Add `justice_disposition_profile()` to `GoalBeliefView`
- Forward it through the `impl_goal_belief_view!` macro
- Implement it on the live per-agent/runtime belief view and any local test doubles that ranking coverage touches

### 3. Strengthen focused ranking coverage

- Add tests that prove:
  - higher theft motive profile outranks lower theft motive profile within the same priority class
  - witness-heavy theft opportunities rank below witness-light theft opportunities when all else is equal
  - justice goals derive motive from justice profile instead of the placeholder constant
  - low-priority crime goals remain low-priority even after motive becomes profile-driven

## Files to Touch

- `tickets/E17CRITHEJUS-021.md` (new)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
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
2. `crates/worldwake-ai/src/ranking.rs` — add ordering tests showing witness-penalized theft loses to cleaner theft opportunities within the same priority class

### Commands

1. `cargo test -p worldwake-ai deferred_crime_and_justice_goals_rank_low_with_minimal_motive`
2. `cargo test -p worldwake-ai ranking::tests`
3. `cargo test -p worldwake-ai`
4. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
