# S177WATSRCQUA-006: Basin refill quality preference + `WashBasinState.dirty_water_refill_penalty`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core/place_dirtiness` (field addition to `WashBasinState`), `worldwake-systems/item_decay` (`first_colocated_water_source` rewritten from first-match to cleanest-preference; `next_wash_basin_refill` raises basin dirtiness on muddy/stale refill), `worldwake-sim/save_load` (SAVE_FORMAT_VERSION bump), downstream `WashBasinState` clone/fixture fallout across sim, AI, CLI, and systems
**Deps**: `archive/tickets/S177WATSRCQUA-001.md`

## Problem

Before this ticket, the spec's D6 deliverable required basin refill to prefer cleanest available water and raise the basin's `dirtiness_level` when refilling from muddy/stale water, coupling to S176's wash-effectiveness gate so dirty-water refill creates real maintenance pressure. The live `first_colocated_water_source` at intake returned the first water source it found via `query_resource_source().find(...)` with no quality preference. That left basin refill indifferent to water quality and prevented the basin-side quality coupling from materializing.

## Assumption Reassessment (2026-05-31)

1. At intake, `first_colocated_water_source` in `crates/worldwake-systems/src/item_decay.rs` performed a first-match scan filtered by `state.commodity == CommodityKind::Water && state.available_quantity.0 > 0 && resource_place(...) == Some(place)`. The landed cleanest-preference rewrite iterates all matching sources, sorts by `quality` ascending (Clean < Stale < Muddy per `WaterQuality::Ord`), and tie-breaks by `EntityId` ascending for determinism.
2. `WashBasinState` at `crates/worldwake-core/src/place_dirtiness.rs:45-55` carries `clean_water_units, max_clean_water, refill_per_tick, units_per_full_wash, dirtiness_level, dirtiness_per_use, max_effective_dirtiness`. Derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` (per reassessment Agent 1 report). New `dirty_water_refill_penalty: BTreeMap<WaterQuality, Permille>` with `#[serde(default)]` is derive-compatible.
3. `next_wash_basin_refill` in `crates/worldwake-systems/src/item_decay.rs` returns `Option<(WashBasinState, EntityId, ResourceSource)>`. Before this ticket it transferred water unconditionally; the landed behavior reads the source's `quality`, looks up the basin's `dirty_water_refill_penalty[quality]`, and raises `basin_state.dirtiness_level` by that amount (saturating).
4. Existing tests at `crates/worldwake-systems/src/item_decay.rs:838-934` exercise basin refill: `wash_basin_refills_from_colocated_water_source` (line 838), `wash_basin_refill_capped_at_max_clean_water` (line 884), `wash_basin_refill_capped_at_source_available_quantity` (line 930). These must be updated or sibling tests added to cover quality-aware behavior. The tests today use a `seed_water_source` helper at line 583 that produces a `ResourceSource { commodity: Water, available_quantity, ... }` with no quality — these will produce `quality: None` after ticket 001 lands.
5. WashBasinState construction sites: relatively few (mainly in `place_dirtiness.rs` and test code). Spread-syntax usage should be checked during implementation; the field needs a sensible `Default` value (empty `BTreeMap` is meaningless — better to provide a per-quality default with Clean→0, Stale→small, Muddy→larger).
6. Shared abstraction boundary: `WashBasinState`'s serialized shape + `first_colocated_water_source`'s contract. The contract change is observable (cleanest-preference selection vs. first-match), so the existing tests that asserted "refills from this source" were kept equivalent where they had one water source, and new tests assert the specific source chosen when several qualities coexist.
7. SAVE_FORMAT_VERSION cascade: this ticket bumps 114→115 (the final bump in the S177 cascade).
8. Coupling to S176: S176 (archived) wires `WashBasinState.dirtiness_level` to wash effectiveness via `max_effective_dirtiness`. This ticket's dirty-water-refill writes to `dirtiness_level` flow through S176's gate naturally — no S176-side change needed.
9. Adjacent contradictions: the `first_colocated_water_source` rewrite changes selection behavior for scenarios where multiple water sources of differing quality coexist at a place. Such scenarios don't exist today (no quality field), so no existing test regression — but document the behavioral expansion in the test plan.
10. Determinism check: `BTreeMap<WaterQuality, Permille>` iteration order is deterministic. The cleanest-preference sort uses `WaterQuality::Ord` (Clean < Stale < Muddy is the natural lexicographic order) plus `EntityId` tie-break — fully deterministic.

## Architecture Check

1. Cleanest-preference selection (vs. first-match-with-quality-ignore) is the FND-1 emergence-aligned choice — when only muddy water is available, the basin refill draws from it (correct fallback behavior); when clean water is available, the basin prefers it (correct optimization). The selection's determinism (sort then tie-break) is the FND-9 scheduling-invariant choice.
2. `dirty_water_refill_penalty` on `WashBasinState` (vs. on `ResourceSource` or as a global constant) is the FND-26 state-cohesion choice — the basin owns its own dirtying mechanics, since different basin types could plausibly have different sensitivity (a fine porcelain basin dirties faster than a stone trough). FND-22 also benefits: a scenario author could author per-basin diversity.
3. `BTreeMap<WaterQuality, Permille>` (vs. paired fields per variant) follows the determinism invariant and matches the `WaterToleranceProfile` pattern from ticket 003 for ecosystem consistency.

## Verified Layers

1. `first_colocated_water_source` cleanest-preference selection: focused unit test seeds 3 water sources at one place (Clean, Stale, Muddy), confirms the fn returns the Clean source.
2. `first_colocated_water_source` tie-break by entity id: focused test seeds 2 Clean sources, confirms deterministic selection by `EntityId` ordering.
3. `first_colocated_water_source` skips depleted sources: focused test seeds Clean-depleted + Muddy-available, confirms Muddy is selected (depletion takes precedence over quality).
4. `next_wash_basin_refill` raises dirtiness on muddy refill: focused test refills basin from `quality: Some(Muddy)` source, asserts `basin.dirtiness_level` rises by `dirty_water_refill_penalty[Muddy]`.
5. `next_wash_basin_refill` does not raise dirtiness for clean refill: focused test with `quality: Some(Clean)` source asserts `dirtiness_level` unchanged.
6. S176 coupling: with raised `dirtiness_level`, the wash-effectiveness gate produces lower relief — this is owned by S176's tests, but a focused integration test here can confirm the chained behavior (refill from muddy, then attempt wash, observe lower relief).
7. SAVE_FORMAT_VERSION migration test confirms 114→115.

## Landed Changes

### 1. Added `dirty_water_refill_penalty` to `WashBasinState`

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

The `Default` impl and explicit construction sites now include the default penalty map.

### 2. Rewrote `first_colocated_water_source`

`crates/worldwake-systems/src/item_decay.rs`:

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

The function name stayed unchanged because the call surface is internal and the behavior is covered by focused tests.

### 3. Modified `next_wash_basin_refill` to raise dirtiness on muddy/stale refill

`crates/worldwake-systems/src/item_decay.rs`: after the transfer amount is computed and before returning, `basin_state.dirtiness_level` is raised by the per-quality penalty:

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

The penalty is applied once per refill operation (not per-unit transferred), matching the existing per-tick semantics.

### 4. Updated existing tests + added new tests

- Modified `wash_basin_refills_from_colocated_water_source` to seed source with `quality: Some(Clean)` and assert no dirtiness change.
- Added `basin_refill_prefers_clean_over_muddy_when_both_available`.
- Added `basin_refill_raises_dirtiness_on_muddy_water`.
- Added `basin_refill_preserves_dirtiness_on_clean_water`.
- Added `basin_refill_falls_back_to_muddy_when_clean_depleted`.

### 5. Bumped `SAVE_FORMAT_VERSION`

`crates/worldwake-sim/src/save_load.rs` now stores `SAVE_FORMAT_VERSION` as `115`.

## Landed Files

- `crates/worldwake-core/src/place_dirtiness.rs` (added `dirty_water_refill_penalty`; default helper; bincode/default/accessor tests)
- `crates/worldwake-systems/src/item_decay.rs` (cleanest source selection; dirty-water basin penalty; focused refill tests)
- `crates/worldwake-sim/src/save_load.rs` (SAVE_FORMAT_VERSION 114→115; save-version test updates)
- `crates/worldwake-core/src/belief.rs`, `crates/worldwake-core/src/delta.rs`, `crates/worldwake-sim/src/affordance_query.rs`, `crates/worldwake-sim/src/belief_view.rs`, `crates/worldwake-sim/src/per_agent_belief_view.rs`, `crates/worldwake-ai/src/*`, `crates/worldwake-cli/src/*`, and scenario/golden fixtures (shared `WashBasinState` no-longer-`Copy` and constructor fallout)
- `crates/worldwake-systems/src/needs_actions.rs` (clone/fixture fallout for the no-longer-`Copy` basin state)

## Out of Scope

- S176's wash-effectiveness gate logic — already in place; this ticket's dirtiness writes flow through it automatically.
- Author-side scenario tuning of `dirty_water_refill_penalty` per-basin — out of scope; default penalty applies. Tickets 009-010 may author scenario-specific penalties if needed.
- Drink action reading basin quality — Drink does not interact with `WashBasinState` (that's a different action surface).
- Renaming `first_colocated_water_source` workspace-wide — only rename if grep confirms ≤2 call sites; otherwise keep the name with documentation that selection is now cleanest-preference.

## Acceptance Result

### Tests Passed

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

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/place_dirtiness.rs` (test module extension) — `WashBasinState` roundtrip with `dirty_water_refill_penalty`.
2. `crates/worldwake-systems/src/item_decay.rs` (test module extension) — 5 new focused tests covering cleanest-preference + dirty-water penalty + fallback; 3 modified existing tests.

### Verification Commands

1. Passed `cargo test -p worldwake-systems basin_refill` — targeted refill, cleanest-selection, dirty-water penalty, fallback, and tie-break tests.
2. Passed `cargo test -p worldwake-core wash_basin` — core wash-basin state/belief/accessor focused coverage.
3. Passed `cargo test -p worldwake-sim save` — save-version and full non-default save roundtrip coverage.
4. Passed `cargo test --workspace --no-run` — shared constructor and all-target compile fallout sweep.
5. Passed `cargo clippy --workspace --all-targets -- -D warnings` — CI-shaped all-target lint gate.
6. Passed `cargo test --workspace` — full workspace tests after final source/lint adjustments.
7. Waived `./scripts/verify.sh` for this per-ticket closeout because the harness final branch phase still owns the full pre-push gate; this ticket ran the live wrapper's substantive cargo test and CI-shaped clippy gates directly.

See Merge-Order Constraints in Step 6 summary — final SAVE_FORMAT_VERSION bump in the S177 cascade (114→115).

## Outcome

Completed on 2026-05-31.

- `WashBasinState` now stores a deterministic per-quality `dirty_water_refill_penalty` map with defaults of Clean 0‰, Stale 20‰, and Muddy 80‰.
- Basin maintenance now chooses the cleanest available colocated water source, skips depleted cleaner sources, tie-breaks deterministically by `EntityId`, and raises basin `dirtiness_level` by the selected source quality's penalty.
- `SAVE_FORMAT_VERSION` is now 115 for the persisted basin-state shape.
- Because `WashBasinState` now owns a `BTreeMap`, it is no longer `Copy`; downstream belief views, AI fixtures, CLI fixtures, and systems tests were updated to clone or inherit defaults explicitly.

## Deviations

- The internal function name `first_colocated_water_source` was kept; the landed contract is enforced by focused tests rather than a rename.
- No S176 wash-effectiveness code changed. The dirty-water refill writes to the existing `dirtiness_level` carrier, so S176's existing wash gate consumes the result through the established state path.

## Verification Result

- Passed `cargo test -p worldwake-systems basin_refill`
- Passed `cargo test -p worldwake-core wash_basin`
- Passed `cargo test -p worldwake-sim save`
- Passed `cargo test --workspace --no-run`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`
- Waived `./scripts/verify.sh` for this per-ticket closeout because the harness final branch phase still owns the full pre-push gate.
