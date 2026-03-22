# S09INDACTREEVA-004: Reassess and archive `Indefinite` removal ticket against delivered finite-duration architecture

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Docs/ticket alignment only — production finite-duration architecture already exists
**Deps**: `specs/S09-indefinite-action-re-evaluation.md`, archived `S09INDACTREEVA-002`

## Problem

This ticket's original implementation scope is stale. The codebase already removed `DurationExpr::Indefinite` and `ActionDuration::Indefinite`, switched defend to `DurationExpr::ActorDefendStance`, and simplified `ActionDuration` to a finite-only transparent newtype.

The useful work here is to correct the ticket to match the actual architecture, verify the delivered behavior and focused coverage, fix the one remaining stale doc reference, and archive the ticket with an accurate outcome.

## Assumption Reassessment (2026-03-20)

1. The ticket's core assumption is wrong: the architectural removal is already done in live code, not pending.
2. `crates/worldwake-sim/src/action_semantics.rs` already has the intended post-S09 shape:
   - `DurationExpr::ActorDefendStance` exists.
   - `DurationExpr::Indefinite` does not exist.
   - `resolve_for()` already reads `CombatProfile.defend_stance_ticks`.
3. `crates/worldwake-sim/src/action_duration.rs` is already the cleaner end-state architecture: `ActionDuration` is a transparent `u32` newtype with `new()`, `ticks()`, and finite `advance()` semantics only. The ticket's older "single-variant enum" fallback is now obsolete.
4. `crates/worldwake-sim/src/belief_view.rs` already estimates `DurationExpr::ActorDefendStance` from the actor's believed `CombatProfile`.
5. `crates/worldwake-systems/src/combat.rs` already registers defend with `DurationExpr::ActorDefendStance`; defend is not using `Indefinite` anywhere in production code.
6. `crates/worldwake-ai/src/search.rs` already consumes finite duration directly through `duration.ticks()`; the older `Indefinite` special-case branch is gone.
7. `rg -n "Indefinite" crates` returns no hits. The only live stale reference found during reassessment was in `CLAUDE.md`.
8. The ticket's original test-gap claim is also stale. Exact existing focused coverage already proves the key invariants:
   - `crates/worldwake-sim/src/action_semantics.rs`: `duration_expr_resolves_trade_and_combat_driven_ticks_from_authoritative_state`
   - `crates/worldwake-sim/src/start_gate.rs`: `start_action_supports_dynamic_defend_duration_when_no_reservations_are_needed`
   - `crates/worldwake-sim/src/per_agent_belief_view.rs`: `estimate_duration_uses_actor_defend_stance_ticks_from_combat_profile`
   - `crates/worldwake-systems/src/combat.rs`: `register_defend_action_creates_profile_driven_public_defend_definition`, `defend_affordance_starts_with_finite_profile_duration_and_commits`
   - `crates/worldwake-ai/src/search.rs`: `build_successor_estimates_defend_ticks_from_combat_profile`

## Architecture Check

1. The current architecture is better than the one this ticket originally described. The finite-only `ActionDuration` newtype is cleaner than either the old two-variant enum or the ticket's interim "single-variant enum" idea. It removes dead branching, avoids future aliasing pressure, and makes the invariant explicit in the type itself.
2. `DurationExpr::ActorDefendStance` remains the right abstraction. It preserves profile-driven per-agent variation while keeping defend duration resolved through the same runtime/belief seam used by other dynamic duration expressions.
3. No further production rewrite is beneficial here. Reopening the implementation would add churn without improving extensibility or robustness.
4. The only justified cleanup was doc alignment: remove the stale `Finite or Indefinite` wording from `CLAUDE.md` so repository guidance matches live authority paths.

## Verification Layers

1. Finite-only duration type contract -> focused unit coverage in `crates/worldwake-sim/src/action_duration.rs`
2. Authoritative defend duration resolution from `CombatProfile.defend_stance_ticks` -> focused unit coverage in `crates/worldwake-sim/src/action_semantics.rs`
3. Action-start runtime duration materialization -> focused runtime coverage in `crates/worldwake-sim/src/start_gate.rs`
4. Belief-side defend duration estimation -> focused runtime-belief coverage in `crates/worldwake-sim/src/per_agent_belief_view.rs`
5. Defend action registration and finite lifecycle behavior -> focused combat coverage in `crates/worldwake-systems/src/combat.rs`
6. Planner successor cost uses finite defend duration -> focused planner coverage in `crates/worldwake-ai/src/search.rs`
7. Repository-wide regression safety -> `cargo test --workspace` and `cargo clippy --workspace`

## What Changed

### 1. Corrected scope

This ticket now reflects the real state of the codebase: the S09 finite-duration architecture was already delivered before this pass.

### 2. Removed stale documentation

`CLAUDE.md` now describes `ActionDuration` as always finite.

### 3. Re-verified the delivered architecture

Ran focused tests for authoritative resolution, belief estimation, runtime start behavior, combat lifecycle behavior, planner costing, plus workspace tests and clippy.

## Files Touched

- `tickets/S09INDACTREEVA-004.md`
- `CLAUDE.md`

## Out of Scope

- Re-implementing `ActorDefendStance`
- Reintroducing or emulating `Indefinite`
- Refactoring `ActionDuration` beyond its current finite-only newtype
- Additional planner/combat/CLI code changes without a new regression

## Acceptance Criteria

1. The ticket no longer claims missing production work that is already present.
2. Repository docs no longer describe `ActionDuration` as finite-or-indefinite.
3. Focused defend-duration verification continues to pass across authoritative, belief, runtime, combat, and planner layers.
4. `cargo test --workspace` passes.
5. `cargo clippy --workspace` passes.

## Test Plan

## New/Modified Tests

None. Reassessment found that the previously claimed gap was already covered by focused existing tests.

## Short Rationale For Each

None required. Existing focused tests already cover the finite defend-duration invariant at the authoritative, belief, runtime, combat, and planner layers.

## Verification Commands

1. `cargo test -p worldwake-sim action_duration -- --nocapture`
2. `cargo test -p worldwake-sim action_semantics::tests::duration_expr_resolves_trade_and_combat_driven_ticks_from_authoritative_state -- --nocapture`
3. `cargo test -p worldwake-sim start_gate::tests::start_action_supports_dynamic_defend_duration_when_no_reservations_are_needed -- --nocapture`
4. `cargo test -p worldwake-sim per_agent_belief_view::tests::estimate_duration_uses_actor_defend_stance_ticks_from_combat_profile -- --nocapture`
5. `cargo test -p worldwake-systems combat::tests::register_defend_action_creates_profile_driven_public_defend_definition -- --nocapture`
6. `cargo test -p worldwake-systems combat::tests::defend_affordance_starts_with_finite_profile_duration_and_commits -- --nocapture`
7. `cargo test -p worldwake-ai search::tests::build_successor_estimates_defend_ticks_from_combat_profile -- --nocapture`
8. `cargo test --workspace`
9. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-20
- What actually changed:
  - Reassessed the ticket against the live S09 architecture and corrected its assumptions and scope.
  - Removed the last stale `ActionDuration` doc wording from `CLAUDE.md`.
- Deviations from original plan:
  - Did not modify production simulation, AI, combat, or CLI code because the finite-duration architecture had already been implemented.
  - Did not add tests because the focused coverage the ticket claimed was missing already existed.
- Verification results:
  - `cargo test -p worldwake-sim action_duration -- --nocapture`
  - `cargo test -p worldwake-sim action_semantics::tests::duration_expr_resolves_trade_and_combat_driven_ticks_from_authoritative_state -- --nocapture`
  - `cargo test -p worldwake-sim start_gate::tests::start_action_supports_dynamic_defend_duration_when_no_reservations_are_needed -- --nocapture`
  - `cargo test -p worldwake-sim per_agent_belief_view::tests::estimate_duration_uses_actor_defend_stance_ticks_from_combat_profile -- --nocapture`
  - `cargo test -p worldwake-systems combat::tests::register_defend_action_creates_profile_driven_public_defend_definition -- --nocapture`
  - `cargo test -p worldwake-systems combat::tests::defend_affordance_starts_with_finite_profile_duration_and_commits -- --nocapture`
  - `cargo test -p worldwake-ai search::tests::build_successor_estimates_defend_ticks_from_combat_profile -- --nocapture`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
