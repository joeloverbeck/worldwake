# S08ACTSTAABORES-002: Align Heal Medicine Consumption With First Treatment Effect

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` action-local tick-state support and recoverable tick-abort handling, `worldwake-systems` heal action lifecycle, and care integration coverage
**Deps**: `specs/S08-action-start-abort-resilience.md`

## Problem

`start_heal()` currently destroys one unit of Medicine before any wound mutation occurs, while `tick_heal()` owns the real treatment effect and `commit_heal()` is empty. If the target loses its wounds before the first treatment tick, the action aborts after start and Medicine is lost without any treatment effect. That violates conservation and the current gradual-healing action semantics.

## Assumption Reassessment (2026-03-19)

1. In `crates/worldwake-systems/src/combat.rs`, `start_heal()` still validates context and immediately calls `consume_one_unit_of_commodity(..., CommodityKind::Medicine)`.
2. In the same file, `tick_heal()` still performs the actual wound mutation by calling `apply_treatment(...)` and writing the resulting `WoundList`; `commit_heal()` remains a no-op.
3. Existing focused coverage in `combat::tests::heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity`, `combat::tests::heal_removes_fully_healed_wounds`, and `combat::tests::self_treatment_succeeds_when_actor_has_wounds_and_medicine` proves the heal lifecycle already exists, but the assertions are currently aligned with the old "consume at start" behavior.
4. Existing integration coverage in `crates/worldwake-systems/tests/e09_needs_integration.rs::scheduler_driven_care_actions_apply_effects_and_preserve_conservation` already proves scheduler-driven care crosses crate boundaries and must continue to pass after the lifecycle adjustment.
5. `crates/worldwake-sim/src/tick_step.rs` already contains `tick_step::tests::best_effort_request_records_abort_requested_start_failure_without_failing_tick` and `tick_step::tests::strict_request_propagates_abort_requested_start_failure`, so the BestEffort start-abort crash is no longer part of this ticket's live scope.
6. `specs/S08-action-start-abort-resilience.md` explicitly preserves gradual healing and rejects a pure "move Medicine to \`commit_heal()\`" fix because treatment is incremental in the current architecture.
7. Current action-framework state is set only by `on_start`: `crates/worldwake-sim/src/action_handler.rs::ActionTickFn` takes `&ActionInstance`, and `crates/worldwake-sim/src/tick_action.rs` does not provide a path for `on_tick` to persist updated `local_state`. Without a small framework change, a "first successful treatment tick" spend boundary would have to depend on incidental scheduler timing rather than explicit action state.
8. `crates/worldwake-sim/src/tick_step.rs` processes newly started actions in the same scheduler tick, so there is no scheduler-level inter-tick gap between heal start and the first treatment tick. The remaining gap exists on the lower-level direct lifecycle surface (`start_action()` followed by `tick_action()`), and `tick_action()` currently propagates `ActionError::AbortRequested(_)` from `on_tick()` as a hard error rather than a lawful abort.
9. Mismatch found and corrected: the original ticket scoped work to `worldwake-systems` only, but the clean durable implementation requires narrow `worldwake-sim` changes so heal can track whether Medicine has already been spent and so mid-action handler-requested aborts can terminate cleanly instead of surfacing as raw engine errors on the direct lifecycle surface. AI pipeline verification remains intentionally separated into `S08ACTSTAABORES-003`.

## Architecture Check

1. Making `start_heal()` validation-only and spending Medicine in the same transaction as the first successful wound mutation is cleaner than moving all effect to `commit_heal()`, because it preserves the existing finite-duration incremental treatment model and keeps conserved input destruction aligned with the first irreversible domain effect.
2. Extending the action framework so `on_tick` can update action-local state is cleaner than inferring "first tick" from scheduler timing or wound deltas. The framework already permits action-local state (`ActionState`); this ticket only completes that model so multi-tick actions can express explicit lifecycle boundaries.
3. Treating `AbortRequested` from `on_tick()` as a normal action abort is cleaner than forcing handlers to encode lawful mid-action invalidation as internal errors. That keeps start, tick, and commit phases architecturally consistent and makes multi-tick actions more extensible.
4. No backwards-compatibility shims or alternate heal code paths are introduced; the ticket should update the canonical heal lifecycle and shared action substrate in place.

## Verification Layers

1. Tick handlers can persist action-local state across active ticks -> focused runtime test in `crates/worldwake-sim/src/tick_action.rs`
2. `on_tick()` handler-requested aborts terminate the action cleanly instead of surfacing as raw errors -> focused runtime test in `crates/worldwake-sim/src/tick_action.rs`
3. Heal start does not destroy Medicine before treatment begins -> focused runtime test in `crates/worldwake-systems/src/combat.rs`
4. First successful treatment tick spends exactly one Medicine and mutates wounds in the same action tick -> focused runtime test in `crates/worldwake-systems/src/combat.rs`
5. If wounds disappear before the first treatment tick on the direct lifecycle surface, heal aborts without spending Medicine -> focused runtime test in `crates/worldwake-systems/src/combat.rs`
6. Scheduler-driven care still preserves conservation and applies treatment effects -> integration test in `crates/worldwake-systems/tests/e09_needs_integration.rs`
7. Later wound state should not be used as a proxy for the spend boundary alone; the focused lifecycle tests must assert the pre-first-tick, first-effect, and post-abort stages directly.

## What to Change

### 1. Make heal start validation-only

Refactor `start_heal()` in `crates/worldwake-systems/src/combat.rs` so it validates actor, target, co-location, liveness, and wound presence, but does not consume Medicine or apply any irreversible state change.

### 2. Add explicit action-local first-effect state

Introduce the minimal `worldwake-sim` support needed for a multi-tick action to update its own `local_state` during `on_tick()`. Keep the change narrow and generic so it supports explicit lifecycle tracking without adding hidden timing contracts or domain-specific scheduler hooks.

### 3. Make tick-phase handler aborts recoverable

Update `worldwake-sim` tick execution so `ActionError::AbortRequested(_)` returned from `on_tick()` follows the same abort/finalization path used by handler-requested aborts at commit, rather than escaping as a raw engine error.

### 4. Add explicit first-effect spend behavior

Refactor the heal lifecycle so the first successful treatment-effect tick both:
- consumes exactly one unit of Medicine, and
- applies the first `WoundList` mutation in the same transaction.

The implementation may use narrow action-local state if needed, but it must keep the first-effect boundary explicit in code and tests.

### 5. Preserve gradual healing semantics

Do not collapse heal into an end-of-action state flip. Subsequent heal ticks must continue to reduce bleeding/severity incrementally after the initial Medicine spend, and `commit_heal()` should remain free of domain work unless the implementation needs a narrowly justified cleanup step.

### 6. Update focused and integration coverage

Revise existing combat tests and add regressions for:
- no Medicine loss before the first effective treatment tick,
- no Medicine loss when wounds vanish before that first tick,
- exact-once Medicine spend on the first successful treatment tick,
- continued gradual wound reduction and eventual completion.

## Files to Touch

- `crates/worldwake-sim/src/action_handler.rs` (modify)
- `crates/worldwake-sim/src/action_state.rs` (modify)
- `crates/worldwake-sim/src/tick_action.rs` (modify)
- `crates/worldwake-systems/src/combat.rs` (modify)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify if the existing integration assertions need to pin the new spend boundary)

## Out of Scope

- `worldwake-ai` blocker derivation, plan-failure handling, or golden-harness behavior
- Redesigning `DurationExpr::TargetTreatment`
- Adding a reservation system or pre-consumption hold for Medicine
- Converting heal into a commit-only or single-tick action

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-sim tick_action::tests::tick_action_persists_updated_local_state_from_handler`
2. `cargo test -p worldwake-sim tick_action::tests::tick_action_converts_handler_requested_tick_abort_into_action_abort`
3. `cargo test -p worldwake-systems combat::tests::heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity`
4. A focused `combat` regression proving wounds disappearing before the first treatment tick do not consume Medicine and do not surface as a raw engine error
5. A focused `combat` regression proving Medicine is consumed exactly once on the first successful treatment tick
6. `cargo test -p worldwake-systems combat::tests::heal_removes_fully_healed_wounds`
7. `cargo test -p worldwake-systems combat::tests::self_treatment_succeeds_when_actor_has_wounds_and_medicine`
8. `cargo test -p worldwake-systems scheduler_driven_care_actions_apply_effects_and_preserve_conservation`

### Invariants

1. Medicine, as a conserved commodity, is never destroyed before heal produces its first real treatment effect.
2. Heal remains a multi-tick gradual treatment action rather than a commit-only state flip.
3. If heal aborts before the first successful treatment effect, the actor still retains the Medicine that was required to start.
4. Once at least one treatment effect has occurred, the single Medicine unit remains lawfully spent even if the action aborts later.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/tick_action.rs` — add a focused runtime test proving an `on_tick` handler can persist updated action-local state across active ticks.
2. `crates/worldwake-sim/src/tick_action.rs` — add a focused runtime test proving handler-requested tick aborts become clean action aborts.
3. `crates/worldwake-systems/src/combat.rs` — update the existing heal lifecycle test to assert no spend at start and exact-once spend on first effect.
4. `crates/worldwake-systems/src/combat.rs` — add a regression for target wound disappearance between start and first treatment tick.
5. `crates/worldwake-systems/src/combat.rs` — keep full-heal and self-heal focused coverage aligned with the new lifecycle boundary.
6. `crates/worldwake-systems/tests/e09_needs_integration.rs` — preserve scheduler-driven conservation and care-effect integration if the current assertions need tightening.

### Commands

1. `cargo test -p worldwake-sim tick_action::tests::tick_action_persists_updated_local_state_from_handler`
2. `cargo test -p worldwake-sim tick_action::tests::tick_action_converts_handler_requested_tick_abort_into_action_abort`
3. `cargo test -p worldwake-systems combat::tests::heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity`
4. `cargo test -p worldwake-systems combat::tests::heal_removes_fully_healed_wounds`
5. `cargo test -p worldwake-systems scheduler_driven_care_actions_apply_effects_and_preserve_conservation`
6. `cargo test -p worldwake-systems`

## Outcome

- Completed: 2026-03-19
- Actual changes:
  - Added shared `worldwake-sim` support for tick handlers to persist action-local state by passing `&mut ActionInstance` into `ActionTickFn`.
  - Added `ActionState::Heal { medicine_spent }` and used it to make `start_heal()` validation-only while spending Medicine exactly once on the first successful treatment tick.
  - Made `tick_action()` treat handler-requested `AbortRequested` errors during `on_tick()` as normal action aborts, so direct lifecycle calls no longer surface the heal invalidation case as a raw engine error.
  - Updated focused heal lifecycle tests and added regressions for first-effect spending and wound disappearance before first treatment.
- Deviations from original plan:
  - Scope widened from `worldwake-systems` only to include narrow `worldwake-sim` substrate changes. This was required to avoid encoding the heal spend boundary as an incidental scheduler-timing heuristic.
  - The ticket now also fixes graceful `on_tick()` abort handling because the new direct lifecycle regression exposed that the original architecture treated lawful mid-action invalidation as a hard error.
- Verification results:
  - `cargo test -p worldwake-sim` passed.
  - `cargo test -p worldwake-systems` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
