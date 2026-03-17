# S07CARINTANDTRETAR-003: Remove SelfTargetActionKind::Heal and self-target prohibition in combat

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — action validation in worldwake-sim and worldwake-systems
**Deps**: S07CARINTANDTRETAR-001 (TreatWounds type must exist for test assertions)

## Problem

The current `validate_heal_context()` in `combat.rs` (lines 790-796) forbids self-targeting for heal actions. This prevents a wounded agent from treating itself, which breaks the unified self-care/third-party-care model. Additionally, `SelfTargetActionKind::Heal` in `action_handler.rs` (line 252) is the corresponding enum variant that must be removed.

## Assumption Reassessment (2026-03-17)

1. `validate_heal_context()` at `combat.rs:782` checks `target == instance.actor` and returns `SelfTargetForbidden` — confirmed
2. `SelfTargetActionKind` enum has two variants: `Attack` and `Heal` at `action_handler.rs:250-253` — confirmed
3. The `SelfTargetForbidden` error variant references `SelfTargetActionKind` at `action_handler.rs:215-218` — remains valid for `Attack`
4. `validate_heal_context()` also checks: entity kind, alive status, not incapacitated, has wounds — these must remain
5. Test `abort_reason_helpers_preserve_structured_semantics_and_optional_detail` at `action_handler.rs:556-571` uses `SelfTargetActionKind::Heal` in an assertion — must be updated

## Architecture Check

1. Removing only the self-target check (not the entire validation function) preserves all other constraints: medicine required, co-location, actor alive/not incapacitated, target has wounds.
2. `SelfTargetActionKind` becomes a single-variant enum with just `Attack`. This is intentional — attacks should never self-target.
3. No shim — the old variant is simply removed.

## What to Change

### 1. Remove self-target check in `validate_heal_context()`

Delete lines 790-796 in `combat.rs`:
```rust
if target == instance.actor {
    return Err(ActionError::AbortRequested(
        ActionAbortRequestReason::SelfTargetForbidden {
            actor: instance.actor,
            action: SelfTargetActionKind::Heal,
        },
    ));
}
```

### 2. Remove `SelfTargetActionKind::Heal` variant

In `action_handler.rs`, remove the `Heal` variant from `SelfTargetActionKind` enum, leaving single-variant:
```rust
pub enum SelfTargetActionKind {
    Attack,
}
```

### 3. Update test in `action_handler.rs`

The test `abort_reason_helpers_preserve_structured_semantics_and_optional_detail` has an assertion using `SelfTargetActionKind::Heal`. Replace with `SelfTargetActionKind::Attack` or remove that specific assertion.

## Files to Touch

- `crates/worldwake-systems/src/combat.rs` (modify — remove self-target check)
- `crates/worldwake-sim/src/action_handler.rs` (modify — remove Heal variant, update test)

## Out of Scope

- Changing any other validation logic in `validate_heal_context()` (co-location, alive, medicine, wounds)
- Changing how `SelfTargetActionKind::Attack` is used
- Treatment action handler changes beyond the validation removal
- AI or planner changes
- Golden tests (separate ticket)

## Acceptance Criteria

### Tests That Must Pass

1. Self-treatment is lawful when actor has wounds + Medicine + same place + is alive + not incapacitated — new test
2. Self-treatment still requires Medicine (fails without it) — new test
3. Self-treatment still requires co-location (if target is somehow different place) — existing constraint
4. Self-treatment still requires target has wounds — existing constraint
5. Attack self-target remains forbidden (`SelfTargetActionKind::Attack` still exists) — existing test
6. Existing suite: `cargo test -p worldwake-systems` and `cargo test -p worldwake-sim`

### Invariants

1. `SelfTargetActionKind` has exactly 1 variant (`Attack`)
2. `validate_heal_context()` still enforces: target exists, target is Agent, target is alive, target not incapacitated, target has wounds
3. Self-healing is permitted when all non-self-target constraints pass
4. Self-attacking remains forbidden

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/combat.rs` — new test: self-treatment succeeds when all constraints met
2. `crates/worldwake-systems/src/combat.rs` — new test: self-treatment fails without medicine (proves other constraints still enforced)
3. `crates/worldwake-sim/src/action_handler.rs` — modify `abort_reason_helpers_preserve_structured_semantics_and_optional_detail` to use `Attack` instead of `Heal`

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-sim`
3. `cargo clippy -p worldwake-systems -p worldwake-sim`
