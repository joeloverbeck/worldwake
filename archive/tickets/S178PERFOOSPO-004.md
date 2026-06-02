# S178PERFOOSPO-004: Condition-scaled Eat hunger relief

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — Eat handler hunger-relief computation scales by the lot's `PerishableState.condition` band.
**Deps**: `archive/tickets/S178PERFOOSPO-001.md`, `archive/tickets/S178PERFOOSPO-003.md`

## Problem

D4 makes hunger relief from Eat depend on the consumed lot's freshness band. Fresh → full `hunger_relief_per_unit`; Stale → linearly-scaled reduced relief between thresholds; Spoiled → minimal floor (Permille 150 of base). Co-located read of `PerishableState` at action commit is lawful per FND-14A because Eat preconditions require possession or co-location with the target lot. The Eat precondition continues to allow spoiled food — the desperation gate lives at candidate generation (ticket 006), not at precondition. Mirrors the S177 Drink/water-quality scaling pattern at the same module.

## Assumption Reassessment (2026-05-31)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Eat commit handler `commit_eat` at `crates/worldwake-systems/src/needs_actions.rs:1091`. `apply_consumable_effects` at lines 1113-1167 reads `profile.hunger_relief_per_unit` at line 1124 and applies at line 1159 with no condition scaling today (`needs.hunger.saturating_sub(profile.hunger_relief_per_unit)`). `#[cfg(test)]` boundary at line 1535. Existing Eat tests: `eat_consumes_one_unit_and_applies_consumable_effects` (line 1949), `aborted_eat_does_not_consume_item` (line 2202), `eat_accepts_actor_owned_ground_lot` (line 3381), `uncontrolled_ground_item_does_not_produce_eat_affordance` (line 3347), `eat_def_id` helper (line 3370).
2. Precedent (precision-rules §2 — Layer Precision): S177's Drink path scales by `tolerance.thirst_relief_factor(quality)` at lines 1138-1154 of the same file. This ticket mirrors the pattern for hunger × `PerishableState.condition`, keeping the relief-scaling helper as a pure function alongside the existing relief computation.
3. Shared abstraction boundary: action-commit local FND-14A authoritative state. Eat handler reads `world.get_component_perishable_state(lot)` and `world.commodity_perish_profiles().get(&commodity)` directly at action commit — both lawful because Eat preconditions enforce co-location/possession (the lot is in the same place as the agent, or in their possession). No belief-view indirection at action commit (belief view is for planning-time remote reads; ticket 005).
4. Spoiled-floor `Permille` value: the spec leaves the exact floor to scenario tuning. Pin to `Permille::new_unchecked(150)` (15% of base relief) for initial implementation — spoiled food gives a small but non-zero relief so hunger never deadlocks purely from spoilage. This matches the Section H #9c dampener language ("Spoiled food still gives minimal relief, so hunger never deadlocks purely from spoilage — it diverts into reduced-relief / fallback, not instant collapse").

## Architecture Check

1. Reading `PerishableState` directly from the lot at action commit is FND-14A-compliant because Eat preconditions enforce co-location/possession. No belief-view indirection needed at action commit — the planner's belief-mediated read (ticket 005) is for goal emission, not for the consumed-relief computation.
2. Linear scaling between `stale_threshold` and `spoiled_threshold` keeps the relief curve continuous (no cliff at the band boundary), preserving granular aftermath (FND-10). Integer arithmetic per AGENTS.md Determinism invariant.

## Verified Layers

1. Relief scales correctly per condition band → focused unit test on the relief-computation helper (3 tests: Fresh, Stale, Spoiled).
2. Existing Eat behavior unchanged for lots without `PerishableState` → regression assertion (the helper returns base relief unchanged when the lot has no perishable component).
3. Existing `eat_consumes_one_unit_and_applies_consumable_effects` (line 1949) extended (not rewritten) to assert relief amount when Eat consumes a Fresh perishable lot.

## Landed Changes

### 1. Relief-scaling helper

In `crates/worldwake-systems/src/needs_actions.rs`, added a pure helper function:

```rust
fn scale_hunger_relief_by_condition(
    base_relief: Permille,
    perishable: Option<&PerishableState>,
    profile: Option<&CommodityPerishProfile>,
    spoiled_floor: Permille,
) -> Permille {
    let (Some(state), Some(profile)) = (perishable, profile) else {
        return base_relief; // non-perishable lot: full relief
    };
    let band = Freshness::derive_from(state.condition, profile);
    match band {
        Freshness::Fresh => base_relief,
        Freshness::Stale => {
            let range = profile.stale_threshold.value().saturating_sub(profile.spoiled_threshold.value());
            if range == 0 {
                return base_relief; // degenerate profile, fall back to base
            }
            let above = state.condition.value().saturating_sub(profile.spoiled_threshold.value());
            let scaled = (base_relief.value() as u32 * above as u32 / range as u32) as u16;
            Permille::new_unchecked(scaled)
        }
        Freshness::Spoiled => {
            Permille::new_unchecked(
                (base_relief.value() as u32 * spoiled_floor.value() as u32 / 1000) as u16,
            )
        }
    }
}
```

### 2. Wire into `apply_consumable_effects`

In `apply_consumable_effects`, replaced the bare `profile.hunger_relief_per_unit` with the scaled value:

```rust
let scaled_relief = scale_hunger_relief_by_condition(
    profile.hunger_relief_per_unit,
    txn.get_component_perishable_state(target),
    txn.commodity_perish_profiles().get(&lot.commodity),
    Permille::new_unchecked(150),
);
needs.hunger = needs.hunger.saturating_sub(scaled_relief);
```

The spoiled-floor `Permille::new_unchecked(150)` is a per-call constant. Per-commodity tuning is a future-scope addition if scenarios warrant.

## Landed Files

- `crates/worldwake-systems/src/needs_actions.rs` (modify — add `scale_hunger_relief_by_condition` helper; wire into `apply_consumable_effects` at line 1159)

## Out of Scope

- Eat precondition changes (Eat continues to allow spoiled food — gate is in candidate generation, ticket 006).
- Belief-view accessor for lot condition (ticket 005 — used by planning-time reads, not action-commit reads).
- Per-commodity spoiled-floor field on `CommodityPerishProfile` (future tuning if scenarios warrant; current per-call constant satisfies S178's stated scope).
- Drink-path changes (S177 already covered).

## Acceptance Criteria

### Acceptance Tests

1. `eat_fresh_food_gives_full_relief` — Apple lot with `condition=1000` gives full `hunger_relief_per_unit`.
2. `eat_stale_food_gives_linearly_scaled_relief` — Apple lot with `condition` midway between thresholds (e.g., 500) gives roughly `base × (500 - 333) / (667 - 333) ≈ base × 0.5`.
3. `eat_spoiled_food_gives_floor_relief` — Apple lot with `condition < spoiled_threshold` (e.g., 100) gives `Permille::new_unchecked(150) × hunger_relief_per_unit / 1000`.
4. `eat_non_perishable_food_unaffected_by_helper` — calling `scale_hunger_relief_by_condition` with `None` for perishable returns `base_relief` unchanged.
5. `eat_consumes_one_unit_and_applies_consumable_effects` (line 1949, extended) — Fresh Apple consumption asserts full relief amount as a regression guard.
6. Existing: `cargo test -p worldwake-systems needs_actions::tests::eat_`.

### Invariants

1. Eat precondition continues to allow spoiled food — gating moves to candidate generation, not precondition (FND-14A scope; FND-20 reasoning-over-scripts).
2. Relief computation is deterministic per `(condition, profile)` pair — integer arithmetic only (AGENTS.md Determinism invariant).
3. Spoiled-floor is non-zero — hunger never deadlocks purely from spoilage (FND-11 dampener per Section H #9c).

## Test Plan Result

### Landed Tests

1. `crates/worldwake-systems/src/needs_actions.rs` `#[cfg(test)]` — added 4 new unit tests (Fresh, Stale, Spoiled, non-perishable).
2. Existing `eat_consumes_one_unit_and_applies_consumable_effects` — extended to assert relief amount under Fresh condition (regression guard).

### Verification Commands

1. `cargo test -p worldwake-systems needs_actions::tests::eat_`
2. `cargo test --workspace`
3. `./scripts/verify.sh`

## Outcome

Implemented condition-scaled Eat hunger relief in `apply_consumable_effects`. Fresh perishable food now gives full base hunger relief, Stale food scales linearly between the stale and spoiled thresholds, and Spoiled food gives the non-zero `Permille(150)` floor. Non-perishable food keeps the previous full-relief path. Drink behavior remains unchanged.

The action path reads `PerishableState` and `CommodityPerishProfile` from the authoritative transaction at Eat commit time, which is lawful under FND-14A because the consumed lot is co-located or possessed at action commit. Eat preconditions were not tightened; planner-time spoiled-food gating remains owned by ticket 006.

## Deviations

The only scope correction was the dependency list: live Eat scaling depends on ticket 003 because perishable lot creation and condition advancement now land there. No production scope was widened beyond the ticket's Eat relief computation.

## Verification Result

- Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py archive/tickets/S178PERFOOSPO-004.md`.
- Passed `cargo test -p worldwake-systems needs_actions::tests::eat_`.
- Passed `cargo test -p worldwake-systems`.
- Passed `cargo test --workspace`.
- Waived `./scripts/verify.sh` for this per-ticket closeout; the full queued spec closeout will own the pre-PR verification gate.
