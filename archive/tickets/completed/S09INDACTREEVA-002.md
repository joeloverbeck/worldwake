# S09INDACTREEVA-002: Validate and harden `DurationExpr::ActorDefendStance` duration resolution coverage

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Tests only — production `ActorDefendStance` wiring already exists
**Deps**: S09INDACTREEVA-001 (`defend_stance_ticks` on `CombatProfile`)

## Problem

This ticket no longer represents missing production implementation. The current codebase already contains the architectural change it originally proposed: `CombatProfile.defend_stance_ticks`, `DurationExpr::ActorDefendStance`, authoritative duration resolution, belief-side duration estimation, and defend action usage are all present.

The remaining useful work is narrower: verify that the delivered architecture is correct, close the focused belief-side coverage gap if needed, and archive this ticket with an accurate outcome.

## Assumption Reassessment (2026-03-20)

1. The original ticket assumptions are stale. `DurationExpr` already has 8 variants in `crates/worldwake-sim/src/action_semantics.rs`, and `ActorDefendStance` is already one of them. `Indefinite` is already gone from the live enum.
2. `DurationExpr::resolve_for()` already resolves `ActorDefendStance` from authoritative `CombatProfile.defend_stance_ticks` in `crates/worldwake-sim/src/action_semantics.rs`.
3. `estimate_duration_from_beliefs()` already resolves `ActorDefendStance` from `view.combat_profile(actor)` in `crates/worldwake-sim/src/belief_view.rs`.
4. `ActionDuration` has already been simplified to the finite-only transparent newtype in `crates/worldwake-sim/src/action_duration.rs`. This is cleaner than the ticket’s older “coexist temporarily with `Indefinite`” assumption and better matches Foundation Principle 8.
5. The defend action already uses `DurationExpr::ActorDefendStance` in `crates/worldwake-systems/src/combat.rs`, and the registration test already asserts that public contract.
6. Focused authoritative coverage already exists:
   - `crates/worldwake-sim/src/action_semantics.rs`: fixed-ticks, roundtrip, resolve success, resolve failure
   - `crates/worldwake-sim/src/start_gate.rs`: active instance duration resolves to `ActionDuration::new(10)` at action start
   - `crates/worldwake-systems/src/combat.rs`: defend affordance starts with finite profile duration and commits
7. The meaningful remaining gap is focused belief-side coverage for `ActorDefendStance`. The generic runtime-helper test in `crates/worldwake-sim/src/per_agent_belief_view.rs` exercises duration estimation infrastructure, but it does not assert the defend-stance-specific estimate directly.

## Architecture Check

1. The live architecture is stronger than the original ticket described. Removing indefinite durations from the live authority path and using a finite-only `ActionDuration` type is the cleaner design: no aliasing, no compatibility branch, no stale special-casing.
2. `ActorDefendStance` is still the right abstraction. It keeps defend duration profile-driven and resolved at action start/belief estimation time, matching the existing dynamic-duration model used by other duration expressions.
3. The only remaining work worth doing in this ticket is test hardening. Reopening production code here would be churn, not architectural improvement.
4. Longer-term ideal architecture: dynamic duration expressions are currently well-factored. If anything evolves later, it should be around concentrated test helpers for duration-expression coverage, not new runtime compatibility layers.

## Verification Layers

1. `DurationExpr` variant coverage and `fixed_ticks()` contract -> focused unit coverage in `crates/worldwake-sim/src/action_semantics.rs`
2. Authoritative resolution from `CombatProfile.defend_stance_ticks` -> focused unit coverage in `crates/worldwake-sim/src/action_semantics.rs`
3. Action-start runtime duration materialization -> focused runtime coverage in `crates/worldwake-sim/src/start_gate.rs`
4. Defend action definition uses `ActorDefendStance` and active defend instances stay finite -> focused combat coverage in `crates/worldwake-systems/src/combat.rs`
5. Belief-side duration estimation for defend -> focused runtime-belief coverage in `crates/worldwake-sim/src/per_agent_belief_view.rs`

## What to Change

### 1. Correct the ticket scope

Keep this ticket aligned with the current codebase instead of restating already-delivered production work.

### 2. Add focused belief-side coverage if still missing

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, add an explicit regression test that `estimate_duration_from_beliefs()` / `view.estimate_duration()` returns `ActionDuration::new(defend_stance_ticks)` for `DurationExpr::ActorDefendStance`.

### 3. Re-verify the delivered implementation

Run the relevant focused suites, then workspace tests and clippy before archiving the ticket.

## Files to Touch

- `tickets/S09INDACTREEVA-002.md` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify only if direct `ActorDefendStance` belief coverage is still missing)

## Out of Scope

- Re-implementing `ActorDefendStance` in production code
- Reintroducing `Indefinite` as a temporary compatibility phase
- Further refactoring of `ActionDuration`
- Changes to planner, scheduler, CLI, or defend behavior beyond verification-driven fixes

## Acceptance Criteria

1. The ticket accurately reflects the current architecture and narrowed scope.
2. Focused tests explicitly prove belief-side estimation for `ActorDefendStance`.
3. Relevant targeted suites pass.
4. `cargo test --workspace` passes.
5. `cargo clippy --workspace` passes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — add a focused `ActorDefendStance` duration-estimation assertion if no direct one exists.

### Commands

1. `cargo test -p worldwake-sim action_semantics::tests::duration_expr_resolves_trade_and_combat_driven_ticks_from_authoritative_state`
2. `cargo test -p worldwake-sim start_gate::tests::start_action_supports_dynamic_defend_duration_when_no_reservations_are_needed`
3. `cargo test -p worldwake-systems combat::tests::defend_affordance_starts_with_finite_profile_duration_and_commits`
4. `cargo test -p worldwake-sim`
5. `cargo test -p worldwake-systems`
6. `cargo test --workspace`
7. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-20
- What actually changed:
  - Reassessed the ticket against the live codebase and corrected its scope. The production architecture originally described by this ticket was already implemented before this pass.
  - Added one focused regression test in `crates/worldwake-sim/src/per_agent_belief_view.rs` proving belief-side duration estimation returns `ActionDuration::new(defend_stance_ticks)` for `DurationExpr::ActorDefendStance`.
- Deviations from original plan:
  - Did not modify production `DurationExpr`, `ActionDuration`, `belief_view`, or defend-action wiring because those changes were already present.
  - Narrowed the ticket to verification hardening and archival instead of redoing already-delivered implementation.
- Verification results:
  - `cargo test -p worldwake-sim per_agent_belief_view::tests::estimate_duration_uses_actor_defend_stance_ticks_from_combat_profile`
  - `cargo test -p worldwake-sim action_semantics::tests::duration_expr_resolves_trade_and_combat_driven_ticks_from_authoritative_state`
  - `cargo test -p worldwake-sim start_gate::tests::start_action_supports_dynamic_defend_duration_when_no_reservations_are_needed`
  - `cargo test -p worldwake-systems combat::tests::defend_affordance_starts_with_finite_profile_duration_and_commits`
  - `cargo test -p worldwake-sim`
  - `cargo test -p worldwake-systems`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
