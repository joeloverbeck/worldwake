# S177WATSRCQUA-005: Quality-aware Drink (relief scaling + dirtiness penalty)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-systems/needs_actions` (`apply_consumable_effects` reads `ItemLot.quality` + `WaterToleranceProfile`; scales relief; raises dirtiness)
**Deps**: `archive/tickets/S177WATSRCQUA-002.md`, `archive/tickets/S177WATSRCQUA-003.md`

## Problem

The spec's D2 deliverable scales Drink's thirst relief by the lot's quality and raises dirtiness when the agent drinks lower-quality water. Before this ticket, `apply_consumable_effects` in `crates/worldwake-systems/src/needs_actions.rs` read only the commodity-intrinsic `thirst_relief_per_unit` from `CommodityConsumableProfile`; it applied unscaled thirst relief and never raised dirtiness on drink. Tickets 002 (`ItemLot.quality`) and 003 (`WaterToleranceProfile`) had already landed, so this ticket made the Drink commit handler consume both surfaces and give water quality behavioral consequences.

## Assumption Reassessment (2026-05-31)

1. `commit_drink` at `crates/worldwake-systems/src/needs_actions.rs` delegates to `apply_needs_effect_schema`, which routes to `apply_consumable_effects`. The actual relief computation lives in `apply_consumable_effects`.
2. Before this ticket, the relief computation was `needs.thirst.saturating_sub(profile.thirst_relief_per_unit)` where `profile` is `CommodityConsumableProfile` on the consumed commodity. No quality scaling existed, and dirtiness was read but never modified.
3. After ticket 002: `ItemLot.quality: Option<WaterQuality>` flows through extraction commit. The consumed lot (which the Drink action targets via its `ActionPayload`) carries the lot's quality.
4. After ticket 003: `WaterToleranceProfile` is universally seeded on every agent. The actor's tolerance is readable via `get_component_water_tolerance_profile(actor)` or the belief-view accessor.
5. Existing focused test `drink_consumes_one_unit_and_applies_consumable_effects` at `crates/worldwake-systems/src/needs_actions.rs` exercises the baseline relief computation. This ticket extended that path and added sibling tests covering quality-scaled relief and dirtiness writes.
6. Shared abstraction boundary: the `apply_consumable_effects` function's relief + dirtiness contract. New behavior is: for water lots with `Some(quality)`, relief is scaled by `tolerance.thirst_relief_factor(quality)` and dirtiness is raised by `tolerance.dirtiness_penalty(quality)`. For non-water or `quality: None`, behavior is unchanged.
7. Adjacent contradictions: none. This is a clean consumer extension — no production code outside `apply_consumable_effects` changes.

## Architecture Check

1. Quality-scaling at consumption (vs. quality-scaling at extraction) — placing the scaling logic in Drink commit means the same `ItemLot.quality` value can be traded, gifted, or relocated through inventory without changing semantics; the per-agent tolerance only applies at the consumption moment. This is FND-29A inspectable: "why did this agent get only 450‰ relief?" answers from `lot.quality + actor.tolerance.thirst_relief_factor(quality)` — the two factors are visible at the consumption site.
2. Dirtiness penalty (vs. a separate post-drink dirtiness rate) — the penalty is paired with the relief at the same site, so the two consequences of drinking lower-quality water travel together in code. FND-26 cohesion.
3. `tolerance.thirst_relief_factor(WaterQuality::Clean) == 1000` and `tolerance.dirtiness_penalty(Clean) == 0` means the new logic is behaviorally neutral for clean water — no regression for the proven baseline.

## Verified Layers

1. Drink commit with clean water: relief unchanged from baseline — proved by the modified `drink_consumes_one_unit_and_applies_consumable_effects` and `drink_water_clean_preserves_baseline_relief_no_dirtiness`.
2. Drink commit with muddy water + default tolerance: relief scaled to 450‰ of baseline; dirtiness raised by 200‰ — proved by `drink_water_muddy_scales_relief_and_raises_dirtiness`.
3. Drink commit with non-water lot: unchanged behavior — proved by `drink_non_water_lot_unchanged_behavior`.
4. Drink commit with custom tolerance override: relief uses overridden factor — proved by `drink_water_stale_with_hardy_tolerance_override`.

## Landed Changes

### 1. Modified `apply_consumable_effects`

`crates/worldwake-systems/src/needs_actions.rs` now reads the consumed `ItemLot`, applies water-quality scaling only when `lot.commodity == CommodityKind::Water && lot.quality.is_some()`, and otherwise preserves existing commodity-intrinsic behavior.

For water lots with `Some(quality)`, Drink reads the actor's universal `WaterToleranceProfile`, scales `CommodityConsumableProfile.thirst_relief_per_unit` with `scale_permille`, and adds the profile's quality-specific dirtiness penalty to `HomeostaticNeeds::dirtiness`.

For non-water lots and water lots with `quality: None`, Drink continues using unscaled `thirst_relief_per_unit` and adds no dirtiness penalty.

### 2. Added `scale_permille`

`crates/worldwake-core/src/numerics.rs` now owns `scale_permille(value, factor) -> Permille`, exported from `worldwake-core`, so the relief scaling is reusable and covered beside the `Permille` type.

### 3. Updated focused tests

`drink_consumes_one_unit_and_applies_consumable_effects` now exercises explicit clean water and asserts dirtiness remains unchanged.

Added focused coverage:

- `drink_water_muddy_scales_relief_and_raises_dirtiness`
- `drink_water_clean_preserves_baseline_relief_no_dirtiness`
- `drink_non_water_lot_unchanged_behavior`
- `drink_water_stale_with_hardy_tolerance_override`

## Landed Files

- `crates/worldwake-systems/src/needs_actions.rs` (modified — `apply_consumable_effects` extension; focused tests in the test module)
- `crates/worldwake-core/src/numerics.rs` (modified — added `scale_permille` helper and unit test)
- `crates/worldwake-core/src/lib.rs` (modified — re-exported `scale_permille`)

## Out of Scope

- ItemLot.quality propagation — owned by ticket 002.
- WaterToleranceProfile component — owned by ticket 003.
- Quality observation belief updates — owned by ticket 004.
- Basin refill behavior — owned by ticket 006.
- Drink action precondition changes — no preconditions are added; quality is a utility axis, not a gate (per spec D2 and Authoritative-to-AI Impact Analysis).
- Trade/exchange path quality propagation — the lot carries quality through inventory; no Drink-side change needed.

## Acceptance Result

### Tests Passed

1. Added: `drink_water_muddy_scales_relief_and_raises_dirtiness`.
2. Added: `drink_water_clean_preserves_baseline_relief_no_dirtiness`.
3. Added: `drink_non_water_lot_unchanged_behavior`.
4. Added: `drink_water_stale_with_hardy_tolerance_override`.
5. Modified: `drink_consumes_one_unit_and_applies_consumable_effects` — updated to assert baseline-clean path and unchanged dirtiness.
6. Existing workspace tests passed with `cargo test --workspace --quiet`.

### Invariants

1. Drink relief = `commodity.thirst_relief_per_unit * tolerance.thirst_relief_factor(quality) / 1000` for water lots with `Some(quality)`; unscaled for non-water lots or `quality: None`.
2. Drink dirtiness write = `tolerance.dirtiness_penalty(quality)` for water lots with `Some(quality)`; zero for non-water lots or `quality: None`.
3. The Drink action precondition surface is unchanged — quality is consumable for all variants, not gated.
4. No new system commands another — Drink reads `ItemLot.quality`, reads `WaterToleranceProfile`, writes `HomeostaticNeeds`. All state-mediated (FND-26).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` (test module extension) — 4 added focused tests covering quality-scaled Drink, plus update of existing `drink_consumes_one_unit_and_applies_consumable_effects`.
2. `crates/worldwake-core/src/numerics.rs` — added `scale_permille_multiplies_by_permille_factor`.

### Verification Commands

1. `cargo test -p worldwake-systems drink_ -- --nocapture` — passed targeted Drink-family tests, including the modified baseline and added quality tests.
2. `cargo test -p worldwake-core scale_permille` — passed focused helper coverage.
3. `cargo test -p worldwake-systems` — passed affected crate suite.
4. `cargo test --workspace --quiet` — passed workspace test gate.

## Outcome

Completed on 2026-05-31.

- Drink now treats `ItemLot.quality` as a concrete consequence carrier for water: muddy/stale/clean quality affects thirst relief and dirtiness through the consuming actor's `WaterToleranceProfile`.
- Non-water lots and water lots without quality remain behaviorally unchanged.
- `scale_permille` landed in `worldwake-core::numerics` and is re-exported from `worldwake-core` for the Drink implementation and future bounded numeric scaling.

## Deviations

- The non-water regression proof uses the shared `apply_consumable_effects` seam directly because non-water consumables do not necessarily appear as Drink affordances. This is the exact consumer contract changed by this ticket.
- `./scripts/verify.sh` was not run for this per-ticket closeout. The `implement-spec-tickets` harness owns the final pre-push verification gate after the full S177 ticket family lands; this ticket ran the affected focused tests, affected crate suite, and `cargo test --workspace --quiet`.

## Verification Result

- Passed `cargo test -p worldwake-systems drink_ -- --nocapture`.
- Passed `cargo test -p worldwake-core scale_permille`.
- Passed `cargo test -p worldwake-systems`.
- Passed `cargo test --workspace --quiet`.
- Waived `./scripts/verify.sh` because the harness reserves the full pre-push gate for final S177 branch completion.
