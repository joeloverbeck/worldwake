# S177WATSRCQUA-005: Quality-aware Drink (relief scaling + dirtiness penalty)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-systems/needs_actions` (`apply_consumable_effects` reads `ItemLot.quality` + `WaterToleranceProfile`; scales relief; raises dirtiness)
**Deps**: `archive/tickets/S177WATSRCQUA-002.md`, `archive/tickets/S177WATSRCQUA-003.md`

## Problem

The spec's D2 deliverable scales Drink's thirst relief by the lot's quality and raises dirtiness when the agent drinks lower-quality water. Today `apply_consumable_effects` in `crates/worldwake-systems/src/needs_actions.rs:1123-1154` reads the commodity-intrinsic `thirst_relief_per_unit` from `CommodityConsumableProfile` and applies it unscaled; there is no dirtiness write on drink. After tickets 002 (ItemLot.quality) and 003 (WaterToleranceProfile) land, this ticket makes the Drink commit handler read both surfaces and apply quality-aware effects. Without it, the new `ItemLot.quality` field has no behavioral consequence on Drink — quality remains inert.

## Assumption Reassessment (2026-05-31)

1. `commit_drink` at `crates/worldwake-systems/src/needs_actions.rs:1112-1121` delegates to `apply_needs_effect_schema`, which routes to `apply_consumable_effects` at lines 1123-1154. The actual relief computation lives in `apply_consumable_effects`.
2. Today's relief computation: `needs.thirst.saturating_sub(profile.thirst_relief_per_unit)` where `profile` is `CommodityConsumableProfile` on the consumed commodity. No quality scaling. Dirtiness is read but never modified.
3. After ticket 002: `ItemLot.quality: Option<WaterQuality>` flows through extraction commit. The consumed lot (which the Drink action targets via its `ActionPayload`) carries the lot's quality.
4. After ticket 003: `WaterToleranceProfile` is universally seeded on every agent. The actor's tolerance is readable via `get_component_water_tolerance_profile(actor)` or the belief-view accessor.
5. Existing focused test `drink_consumes_one_unit_and_applies_consumable_effects` at `crates/worldwake-systems/src/needs_actions.rs:1968` exercises the current relief computation. This test must be extended (or sibling tests added) to cover quality-scaled relief and dirtiness writes.
6. Shared abstraction boundary: the `apply_consumable_effects` function's relief + dirtiness contract. New behavior is: for water lots with `Some(quality)`, relief is scaled by `tolerance.thirst_relief_factor(quality)` and dirtiness is raised by `tolerance.dirtiness_penalty(quality)`. For non-water or `quality: None`, behavior is unchanged.
7. Adjacent contradictions: none. This is a clean consumer extension — no production code outside `apply_consumable_effects` changes.

## Architecture Check

1. Quality-scaling at consumption (vs. quality-scaling at extraction) — placing the scaling logic in Drink commit means the same `ItemLot.quality` value can be traded, gifted, or relocated through inventory without changing semantics; the per-agent tolerance only applies at the consumption moment. This is FND-29A inspectable: "why did this agent get only 450‰ relief?" answers from `lot.quality + actor.tolerance.thirst_relief_factor(quality)` — the two factors are visible at the consumption site.
2. Dirtiness penalty (vs. a separate post-drink dirtiness rate) — the penalty is paired with the relief at the same site, so the two consequences of drinking lower-quality water travel together in code. FND-26 cohesion.
3. `tolerance.thirst_relief_factor(WaterQuality::Clean) == 1000` and `tolerance.dirtiness_penalty(Clean) == 0` means the new logic is behaviorally neutral for clean water — no regression for the proven baseline.

## Verification Layers

1. Drink commit with clean water: relief unchanged from baseline — modified existing test asserts.
2. Drink commit with muddy water + default tolerance: relief scaled to 450‰ of baseline; dirtiness raised by 200‰ — new focused tests.
3. Drink commit with non-water lot (apple, etc.): unchanged behavior — new focused test for regression coverage.
4. Drink commit with custom tolerance override: relief uses overridden factor — new focused test exercising FND-22 diversity.

## What to Change

### 1. Modify `apply_consumable_effects`

`crates/worldwake-systems/src/needs_actions.rs:1123-1154` — extend the relief computation to read `lot.quality` and `actor.water_tolerance_profile`:

```rust
fn apply_consumable_effects(/* … existing params … */) -> Result<(), ActionError> {
    let needs = actor_needs(txn, actor)?;
    let lot = /* … existing lot lookup … */;
    let profile = /* … existing CommodityConsumableProfile lookup … */;

    // NEW: read quality from lot and tolerance from actor
    let (relief_factor, dirtiness_penalty) = match lot.quality {
        Some(quality) => {
            let tolerance = txn
                .get_component_water_tolerance_profile(actor)
                .expect("universal-profile contract: every agent has WaterToleranceProfile");
            (
                tolerance.thirst_relief_factor(quality),
                tolerance.dirtiness_penalty(quality),
            )
        }
        None => (Permille::new(1000).unwrap(), Permille::new(0).unwrap()),
    };

    let scaled_thirst_relief = scale_permille(profile.thirst_relief_per_unit, relief_factor);

    let next = HomeostaticNeeds::new(
        needs.hunger,
        needs.thirst.saturating_sub(scaled_thirst_relief),
        needs.fatigue,
        needs.bladder.saturating_add(profile.bladder_fill_per_unit),
        needs.dirtiness.saturating_add(dirtiness_penalty),
    );
    consume_one_unit(txn, target)?;
    set_actor_needs(txn, actor, next)
}
```

Implement `scale_permille(value, factor)` as a multiplication of `Permille` values producing `Permille` (e.g., `value.value() * factor.value() / 1000`). Verify whether such a helper already exists in `worldwake-core::numerics` — if so, reuse; if not, add it there.

### 2. Update existing `drink_consumes_one_unit_and_applies_consumable_effects` test

The test currently exercises a baseline (likely `Clean` or `None` quality) lot. Verify the existing test's lot setup produces a `quality: None` lot (non-water commodity, or pre-ticket-002 fixture) and add explicit assertions that confirm relief is unchanged at baseline. If the test seeds a water lot, add explicit `quality: Some(Clean)` so the test exercises the explicit-clean path through the new code.

### 3. Add new focused tests

- `drink_water_muddy_scales_relief_and_raises_dirtiness` — actor with default tolerance drinks `ItemLot { commodity: Water, quality: Some(Muddy), … }`; relief is 450‰ of `profile.thirst_relief_per_unit`; dirtiness rises by 200‰.
- `drink_water_clean_preserves_baseline_relief_no_dirtiness` — Clean quality leaves relief unchanged and dirtiness unchanged.
- `drink_non_water_lot_unchanged_behavior` — apple lot consumed; relief unchanged, dirtiness unchanged.
- `drink_water_stale_with_hardy_tolerance_override` — actor with overridden tolerance (`thirst_relief_factor[Stale] = 900`, `dirtiness_penalty[Stale] = 20`) drinks Stale water; relief is 900‰ of baseline; dirtiness rises by 20‰. Proves FND-22 diversity.

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify — `apply_consumable_effects` extension; new tests in test module)
- `crates/worldwake-core/src/numerics.rs` (modify only if `scale_permille` helper doesn't already exist — verify via grep before adding)

## Out of Scope

- ItemLot.quality propagation — owned by ticket 002.
- WaterToleranceProfile component — owned by ticket 003.
- Quality observation belief updates — owned by ticket 004.
- Basin refill behavior — owned by ticket 006.
- Drink action precondition changes — no preconditions are added; quality is a utility axis, not a gate (per spec D2 and Authoritative-to-AI Impact Analysis).
- Trade/exchange path quality propagation — the lot carries quality through inventory; no Drink-side change needed.

## Acceptance Criteria

### Tests That Must Pass

1. New: `drink_water_muddy_scales_relief_and_raises_dirtiness`.
2. New: `drink_water_clean_preserves_baseline_relief_no_dirtiness`.
3. New: `drink_non_water_lot_unchanged_behavior`.
4. New: `drink_water_stale_with_hardy_tolerance_override`.
5. Modified: `drink_consumes_one_unit_and_applies_consumable_effects` at `crates/worldwake-systems/src/needs_actions.rs:1968` — updated to assert baseline-clean path through the new code.
6. Existing: `cargo test --workspace` passes — Drink behavior on existing baseline scenarios (no quality) is preserved.

### Invariants

1. Drink relief = `commodity.thirst_relief_per_unit * tolerance.thirst_relief_factor(quality) / 1000` for water lots with `Some(quality)`; unscaled for non-water or `quality: None`.
2. Drink dirtiness write = `tolerance.dirtiness_penalty(quality)` for water lots with `Some(quality)`; zero (no write) for non-water or `quality: None`.
3. The Drink action precondition surface is unchanged — quality is consumable for all variants, not gated.
4. No new system commands another — Drink reads `ItemLot.quality`, reads `WaterToleranceProfile`, writes `HomeostaticNeeds`. All state-mediated (FND-26).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` (test module extension) — 4 new focused tests covering quality-scaled Drink, plus update of existing `drink_consumes_one_unit_and_applies_consumable_effects`.

### Commands

1. `cargo test -p worldwake-systems drink_water` — targeted Drink tests.
2. `cargo test -p worldwake-systems drink_consumes_one_unit` — verify modified existing test passes.
3. `./scripts/verify.sh` — full workspace.
