# S08ACTSTAABORES-002: Align Heal Medicine Consumption With First Treatment Effect

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` heal action lifecycle and care integration coverage
**Deps**: `specs/S08-action-start-abort-resilience.md`

## Problem

`start_heal()` currently destroys one unit of Medicine before any wound mutation occurs, while `tick_heal()` owns the real treatment effect and `commit_heal()` is empty. If the target loses its wounds before the first treatment tick, the action aborts after start and Medicine is lost without any treatment effect. That violates conservation and the current gradual-healing action semantics.

## Assumption Reassessment (2026-03-19)

1. In `crates/worldwake-systems/src/combat.rs`, `start_heal()` still validates context and immediately calls `consume_one_unit_of_commodity(..., CommodityKind::Medicine)`.
2. In the same file, `tick_heal()` still performs the actual wound mutation by calling `apply_treatment(...)` and writing the resulting `WoundList`; `commit_heal()` remains a no-op.
3. Existing focused coverage in `combat::tests::heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity`, `combat::tests::heal_removes_fully_healed_wounds`, and `combat::tests::self_treatment_succeeds_when_actor_has_wounds_and_medicine` proves the heal lifecycle already exists, but the assertions are currently aligned with the old "consume at start" behavior.
4. Existing integration coverage in `crates/worldwake-systems/tests/e09_needs_integration.rs::scheduler_driven_care_actions_apply_effects_and_preserve_conservation` already proves scheduler-driven care crosses crate boundaries and must continue to pass after the lifecycle adjustment.
5. This ticket is authoritative combat/action behavior, not AI candidate-generation logic. AI pipeline verification is intentionally separated into `S08ACTSTAABORES-003`.
6. `specs/S08-action-start-abort-resilience.md` explicitly preserves gradual healing and rejects a pure "move Medicine to `commit_heal()`" fix because treatment is incremental in the current architecture.
7. No mismatch found between the spec and the current authoritative heal implementation; the drift is in the old fix recommendation, not in the identified bug.

## Architecture Check

1. Making `start_heal()` validation-only and spending Medicine in the same transaction as the first successful wound mutation is cleaner than moving all effect to `commit_heal()`, because it preserves the existing finite-duration incremental treatment model and keeps conserved input destruction aligned with the first irreversible domain effect.
2. No backwards-compatibility shims or alternate heal code paths are introduced; the ticket should update the canonical heal lifecycle in place.

## Verification Layers

1. Heal start does not destroy Medicine before treatment begins -> focused runtime test in `crates/worldwake-systems/src/combat.rs`
2. First successful treatment tick spends exactly one Medicine and mutates wounds in the same action tick -> focused runtime test in `crates/worldwake-systems/src/combat.rs`
3. If wounds disappear before the first treatment tick, heal aborts without spending Medicine -> focused runtime test in `crates/worldwake-systems/src/combat.rs`
4. Scheduler-driven care still preserves conservation and applies treatment effects -> integration test in `crates/worldwake-systems/tests/e09_needs_integration.rs`
5. Later wound state should not be used as a proxy for the spend boundary alone; the focused lifecycle tests must assert the pre-first-tick, first-effect, and post-abort stages directly.

## What to Change

### 1. Make heal start validation-only

Refactor `start_heal()` in `crates/worldwake-systems/src/combat.rs` so it validates actor, target, co-location, liveness, and wound presence, but does not consume Medicine or apply any irreversible state change.

### 2. Add explicit first-effect spend behavior

Refactor the heal lifecycle so the first successful treatment-effect tick both:
- consumes exactly one unit of Medicine, and
- applies the first `WoundList` mutation in the same transaction.

The implementation may use narrow action-local state if needed, but it must keep the first-effect boundary explicit in code and tests.

### 3. Preserve gradual healing semantics

Do not collapse heal into an end-of-action state flip. Subsequent heal ticks must continue to reduce bleeding/severity incrementally after the initial Medicine spend, and `commit_heal()` should remain free of domain work unless the implementation needs a narrowly justified cleanup step.

### 4. Update focused and integration coverage

Revise existing combat tests and add regressions for:
- no Medicine loss before the first effective treatment tick,
- no Medicine loss when wounds vanish before that first tick,
- exact-once Medicine spend on the first successful treatment tick,
- continued gradual wound reduction and eventual completion.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify if the existing integration assertions need to pin the new spend boundary)

## Out of Scope

- `worldwake-sim` BestEffort start-failure classification changes
- `worldwake-ai` blocker derivation, plan-failure handling, or golden-harness behavior
- Redesigning `DurationExpr::TargetTreatment`
- Adding a reservation system or pre-consumption hold for Medicine
- Converting heal into a commit-only or single-tick action

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-systems combat::tests::heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity`
2. A focused `combat` regression proving wounds disappearing before the first treatment tick do not consume Medicine
3. A focused `combat` regression proving Medicine is consumed exactly once on the first successful treatment tick
4. `cargo test -p worldwake-systems combat::tests::heal_removes_fully_healed_wounds`
5. `cargo test -p worldwake-systems combat::tests::self_treatment_succeeds_when_actor_has_wounds_and_medicine`
6. `cargo test -p worldwake-systems scheduler_driven_care_actions_apply_effects_and_preserve_conservation`

### Invariants

1. Medicine, as a conserved commodity, is never destroyed before heal produces its first real treatment effect.
2. Heal remains a multi-tick gradual treatment action rather than a commit-only state flip.
3. If heal aborts before the first successful treatment effect, the actor still retains the Medicine that was required to start.
4. Once at least one treatment effect has occurred, the single Medicine unit remains lawfully spent even if the action aborts later.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — update the existing heal lifecycle test to assert no spend at start and exact-once spend on first effect.
2. `crates/worldwake-systems/src/combat.rs` — add a regression for target wound disappearance between start and first treatment tick.
3. `crates/worldwake-systems/src/combat.rs` — keep full-heal and self-heal focused coverage aligned with the new lifecycle boundary.
4. `crates/worldwake-systems/tests/e09_needs_integration.rs` — preserve scheduler-driven conservation and care-effect integration if the current assertions need tightening.

### Commands

1. `cargo test -p worldwake-systems combat::tests::heal_lifecycle_consumes_medicine_and_reduces_bleeding_and_severity`
2. `cargo test -p worldwake-systems combat::tests::heal_removes_fully_healed_wounds`
3. `cargo test -p worldwake-systems scheduler_driven_care_actions_apply_effects_and_preserve_conservation`
4. `cargo test -p worldwake-systems`
