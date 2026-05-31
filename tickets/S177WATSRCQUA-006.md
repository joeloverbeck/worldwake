# S177WATSRCQUA-006: Basin refill quality preference + `WashBasinState.dirty_water_refill_penalty`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core/place_dirtiness` (field addition to `WashBasinState`), `worldwake-systems/item_decay` (`first_colocated_water_source` rewritten from first-match to cleanest-preference; `next_wash_basin_refill` raises basin dirtiness on muddy refill), `worldwake-sim/save_load` (SAVE_FORMAT_VERSION bump)
**Deps**: S177WATSRCQUA-001

## Problem

The spec's D6 deliverable makes basin refill prefer cleanest available water and raises the basin's `dirtiness_level` when refilling from muddy/stale water — coupling to S176's wash-effectiveness gate so dirty-water refill creates real maintenance pressure. Today `first_colocated_water_source` at `crates/worldwake-systems/src/item_decay.rs:214-223` returns the first water source it finds via `query_resource_source().find(...)` with no quality preference. Without this ticket, basin refill is indifferent to water quality and the basin-side quality coupling (a key emergent loop in the spec's target patterns) does not materialize.

## Assumption Reassessment (2026-05-31)

1. `first_colocated_water_source` at `crates/worldwake-systems/src/item_decay.rs:214-223` performs a first-match scan filtered by `state.commodity == CommodityKind::Water && state.available_quantity.0 > 0 && resource_place(...) == Some(place)`. The fn returns `Option<(EntityId, &ResourceSource)>`. The "cleanest-preference" rewrite iterates all matching sources, sorts by `quality` ascending (Clean < Stale < Muddy per `WaterQuality::Ord`), and tie-breaks by `EntityId` ascending for determinism.
2. `WashBasinState` at `crates/worldwake-core/src/place_dirtiness.rs:45-55` carries `clean_water_units, max_clean_water, refill_per_tick, units_per_full_wash, dirtiness_level, dirtiness_per_use, max_effective_dirtiness`. Derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` (per reassessment Agent 1 report). New `dirty_water_refill_penalty: BTreeMap<WaterQuality, Permille>` with `#[serde(default)]` is derive-compatible.
3. `next_wash_basin_refill` at `crates/worldwake-systems/src/item_decay.rs:184-212` returns `Option<(WashBasinState, EntityId, ResourceSource)>`. Today it transfers water unconditionally; the new behavior reads the source's `quality` (now present after ticket 001), looks up the basin's `dirty_water_refill_penalty[quality]`, and raises `basin_state.dirtiness_level` by that amount (saturating).
4. Existing tests at `crates/worldwake-systems/src/item_decay.rs:838-934` exercise basin refill: `wash_basin_refills_from_colocated_water_source` (line 838), `wash_basin_refill_capped_at_max_clean_water` (line 884), `wash_basin_refill_capped_at_source_available_quantity` (line 930). These must be updated or sibling tests added to cover quality-aware behavior. The tests today use a `seed_water_source` helper at line 583 that produces a `ResourceSource { commodity: Water, available_quantity, ... }` with no quality — these will produce `quality: None` after ticket 001 lands.
5. WashBasinState construction sites: relatively few (mainly in `place_dirtiness.rs` and test code). Spread-syntax usage should be checked during implementation; the field needs a sensible `Default` value (empty `BTreeMap` is meaningless — better to provide a per-quality default with Clean→0, Stale→small, Muddy→larger).
6. Shared abstraction boundary: `WashBasinState`'s serialized shape + `first_colocated_water_source`'s contract. The contract change is observable (cleanest-preference selection vs. first-match), so the existing tests that assert "refills from this source" need to assert the specific source chosen — verify the existing test setup has only one water source at the place (which makes first-match and cleanest-preference equivalent for those tests).
7. SAVE_FORMAT_VERSION cascade: this ticket bumps 114→115 (the final bump in the S177 cascade).
8. Coupling to S176: S176 (archived) wires `WashBasinState.dirtiness_level` to wash effectiveness via `max_effective_dirtiness`. This ticket's dirty-water-refill writes to `dirtiness_level` flow through S176's gate naturally — no S176-side change needed.
9. Adjacent contradictions: the `first_colocated_water_source` rewrite changes selection behavior for scenarios where multiple water sources of differing quality coexist at a place. Such scenarios don't exist today (no quality field), so no existing test regression — but document the behavioral expansion in the test plan.
10. Determinism check: `BTreeMap<WaterQuality, Permille>` iteration order is deterministic. The cleanest-preference sort uses `WaterQuality::Ord` (Clean < Stale < Muddy is the natural lexicographic order) plus `EntityId` tie-break — fully deterministic.

## Architecture Check

1. Cleanest-preference selection (vs. first-match-with-quality-ignore) is the FND-1 emergence-aligned choice — when only muddy water is available, the basin refill draws from it (correct fallback behavior); when clean water is available, the basin prefers it (correct optimization). The selection's determinism (sort then tie-break) is the FND-9 scheduling-invariant choice.
2. `dirty_water_refill_penalty` on `WashBasinState` (vs. on `ResourceSource` or as a global constant) is the FND-26 state-cohesion choice — the basin owns its own dirtying mechanics, since different basin types could plausibly have different sensitivity (a fine porcelain basin dirties faster than a stone trough). FND-22 also benefits: a scenario author could author per-basin diversity.
3. `BTreeMap<WaterQuality, Permille>` (vs. paired fields per variant) follows the determinism invariant and matches the `WaterToleranceProfile` pattern from ticket 003 for ecosystem consistency.

## Verification Layers

1. `first_colocated_water_source` cleanest-preference selection: focused unit test seeds 3 water sources at one place (Clean, Stale, Muddy), confirms the fn returns the Clean source.
2. `first_colocated_water_source` tie-break by entity id: focused test seeds 2 Clean sources, confirms deterministic selection by `EntityId` ordering.
3. `first_colocated_water_source` skips depleted sources: focused test seeds Clean-depleted + Muddy-available, confirms Muddy is selected (depletion takes precedence over quality).
4. `next_wash_basin_refill` raises dirtiness on muddy refill: focused test refills basin from `quality: Some(Muddy)` source, asserts `basin.dirtiness_level` rises by `dirty_water_refill_penalty[Muddy]`.
5. `next_wash_basin_refill` does not raise dirtiness for clean refill: focused test with `quality: Some(Clean)` source asserts `dirtiness_level` unchanged.
6. S176 coupling: with raised `dirtiness_level`, the wash-effectiveness gate produces lower relief — this is owned by S176's tests, but a focused integration test here can confirm the chained behavior (refill from muddy, then attempt wash, observe lower relief).
7. SAVE_FORMAT_VERSION migration test confirms 114→115.

## What to Change

### 1. Add `dirty_water_refill_penalty` to `WashBasinState`

`crates/worldwake-core/src/place_dirtiness.rs:45-55`:

```rust
pub struct WashBasinState {
    pub clean_water_units: u16,
    pub max_clean_water: u16,
    pub refill_per_tick: u16,
    pub units_per_full_wash: u16,
    pub dirtiness_level: Permille,
    pub dirtiness_per_use: Permille,
    pub max_effective_dirtiness: Permille,
    #[serde(default = "default_dirty_water_refill_penalty")]
    pub dirty_water_refill_penalty: BTreeMap<WaterQuality, Permille>,
}

fn default_dirty_water_refill_penalty() -> BTreeMap<WaterQuality, Permille> {
    BTreeMap::from([
        (WaterQuality::Clean, Permille::new(0).unwrap()),
        (WaterQuality::Stale, Permille::new(20).unwrap()),
        (WaterQuality::Muddy, Permille::new(80).unwrap()),
    ])
}
```

Update the `Default` impl for `WashBasinState` (if one exists) and any explicit construction sites.

### 2. Rewrite `first_colocated_water_source`

`crates/worldwake-systems/src/item_decay.rs:214-223`:

```rust
fn first_colocated_water_source(
    world: &World,
    place: EntityId,
) -> Option<(EntityId, &ResourceSource)> {
    world
        .query_resource_source()
        .filter(|(source_id, state)| {
            state.commodity == CommodityKind::Water
                && state.available_quantity.0 > 0
                && resource_place(world, *source_id) == Some(place)
        })
        .min_by(|(a_id, a_state), (b_id, b_state)| {
            // Cleanest preference: None sorts as "unknown" — equivalent to Clean for selection.
            // Among Some variants, Clean < Stale < Muddy per WaterQuality::Ord.
            let a_q = a_state.quality.unwrap_or(WaterQuality::Clean);
            let b_q = b_state.quality.unwrap_or(WaterQuality::Clean);
            a_q.cmp(&b_q).then_with(|| a_id.cmp(b_id))
        })
}
```

Rename the function to reflect its new contract (e.g., `cleanest_colocated_water_source`) — verify the rename's blast radius via grep before deciding whether to rename or keep the old name. If renamed, update all call sites.

### 3. Modify `next_wash_basin_refill` to raise dirtiness on muddy/stale refill

`crates/worldwake-systems/src/item_decay.rs:184-212`: after the transfer amount is computed and before returning, raise `basin_state.dirtiness_level` by the per-quality penalty:

```rust
if let Some(quality) = source.quality {
    let penalty = basin_state
        .dirty_water_refill_penalty
        .get(&quality)
        .copied()
        .unwrap_or(Permille::new(0).unwrap());
    basin_state.dirtiness_level = basin_state.dirtiness_level.saturating_add(penalty);
}
```

The penalty is applied once per refill operation (not per-unit transferred) — matching the existing per-tick semantics. If per-unit-scaled penalty is desired, document the rationale.

### 4. Update existing tests + add new tests

- Modify `wash_basin_refills_from_colocated_water_source` (line 838) to seed source with `quality: Some(Clean)` and assert no dirtiness change. Or split into a sibling test if the existing assertion can stand.
- Add `basin_refill_prefers_clean_over_muddy_when_both_available`.
- Add `basin_refill_raises_dirtiness_on_muddy_water`.
- Add `basin_refill_preserves_dirtiness_on_clean_water`.
- Add `basin_refill_falls_back_to_muddy_when_clean_depleted`.

### 5. Bump `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs:7`: change `114` to `115`.

## Files to Touch

- `crates/worldwake-core/src/place_dirtiness.rs` (modify — add `dirty_water_refill_penalty` field to `WashBasinState`; default helper; test extension)
- `crates/worldwake-systems/src/item_decay.rs` (modify — rewrite `first_colocated_water_source` to cleanest-preference; extend `next_wash_basin_refill` with quality penalty write; modify existing tests; add new tests)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump `SAVE_FORMAT_VERSION` 114→115)

## Out of Scope

- S176's wash-effectiveness gate logic — already in place; this ticket's dirtiness writes flow through it automatically.
- Author-side scenario tuning of `dirty_water_refill_penalty` per-basin — out of scope; default penalty applies. Tickets 009-010 may author scenario-specific penalties if needed.
- Drink action reading basin quality — Drink does not interact with `WashBasinState` (that's a different action surface).
- Renaming `first_colocated_water_source` workspace-wide — only rename if grep confirms ≤2 call sites; otherwise keep the name with documentation that selection is now cleanest-preference.

## Acceptance Criteria

### Tests That Must Pass

1. New: `basin_refill_prefers_clean_over_muddy_when_both_available` — assertion that selection picks Clean.
2. New: `basin_refill_raises_dirtiness_on_muddy_water` — `dirtiness_level` rises by `dirty_water_refill_penalty[Muddy]`.
3. New: `basin_refill_preserves_dirtiness_on_clean_water` — `dirtiness_level` unchanged.
4. New: `basin_refill_falls_back_to_muddy_when_clean_depleted` — Clean source with 0 quantity is skipped; Muddy source selected.
5. New: `first_colocated_water_source_tie_breaks_by_entity_id` — two Clean sources, deterministic selection.
6. Modified: `wash_basin_refills_from_colocated_water_source` (line 838) — updated to seed Clean source; baseline assertion preserved.
7. Modified: `wash_basin_refill_capped_at_max_clean_water` (line 884) — verified that the cap logic is unchanged by the quality additions.
8. Modified: `wash_basin_refill_capped_at_source_available_quantity` (line 930) — verified that source-quantity cap is unchanged.
9. Existing: `cargo test --workspace` passes.

### Invariants

1. `first_colocated_water_source` always returns the cleanest-available water source at the place (Clean < Stale < Muddy), tie-broken deterministically by `EntityId`.
2. Basin refill from `quality: Some(quality)` raises `dirtiness_level` by `dirty_water_refill_penalty[quality]` (saturating). For `quality: None` or non-water sources, dirtiness is unchanged.
3. `BTreeMap<WaterQuality, Permille>` iteration is deterministic.
4. `SAVE_FORMAT_VERSION` is now 115.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/place_dirtiness.rs` (test module extension) — `WashBasinState` roundtrip with `dirty_water_refill_penalty`.
2. `crates/worldwake-systems/src/item_decay.rs` (test module extension) — 5 new focused tests covering cleanest-preference + dirty-water penalty + fallback; 3 modified existing tests.

### Commands

1. `cargo test -p worldwake-systems first_colocated_water_source` — targeted selection tests.
2. `cargo test -p worldwake-systems basin_refill` — targeted refill tests.
3. `cargo test -p worldwake-systems wash_basin_refill` — modified existing tests.
4. `./scripts/verify.sh` — full workspace.

See Merge-Order Constraints in Step 6 summary — final SAVE_FORMAT_VERSION bump in the S177 cascade (114→115).
