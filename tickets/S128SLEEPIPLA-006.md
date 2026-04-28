# S128SLEEPIPLA-006: Scenario authoring for SleepQualityProfile

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No — scenario authoring layer only. Adds `PlaceDef.sleep_quality: Option<SleepQualityProfileDef>` and a new `SleepQualityProfileDef` type in the scenario module; modifies `spawn_place` to always apply `SleepQualityProfile` (universal-on-Place precedent).
**Deps**: archive/tickets/S128SLEEPIPLA-001.md

## Problem

archive/tickets/S128SLEEPIPLA-001.md added the `SleepQualityProfile` core type, default, schema registration, and save/load substrate, but the scenario layer still cannot author per-place quality or guarantee that every spawned place carries the component. Today, `PlaceDef` (`crates/worldwake-cli/src/scenario/types.rs:318-324`) carries only `name`, `tags`, and `visibility_profile: Option<PlaceVisibilityProfile>`. The `spawn_place` loop (`crates/worldwake-cli/src/scenario/mod.rs:273-278`) only conditionally applies `visibility_profile` — there is no precedent for unconditionally applying a place component with `unwrap_or_default()` (the agent universal pattern at `mod.rs:566-624` exists but no place equivalent). This ticket establishes the universal-on-Place precedent.

## Assumption Reassessment (2026-04-27)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-cli/src/scenario/types.rs:318-324` defines `PlaceDef` with three fields. The optional `visibility_profile: Option<PlaceVisibilityProfile>` (line 323) uses `#[serde(default)]` and is the closest existing optional-place-component precedent.
2. `crates/worldwake-cli/src/scenario/mod.rs:273-278` shows the existing place-component setup loop:
   ```rust
   for place_def in &def.places {
       let place_id = resolve_name(names, &place_def.name, "place visibility_profile")?;
       if let Some(profile) = &place_def.visibility_profile {
           txn.set_component_place_visibility_profile(place_id, profile.clone())?;
       }
   }
   ```
   This is conditional. The new pattern for `sleep_quality` is unconditional: always call `set_component_sleep_quality_profile` with `unwrap_or_default()`-resolved value.
3. Shared boundary under audit: the scenario load contract for places. The existing `if let Some(...)` pattern allows components to be absent; the new universal pattern guarantees presence. This is the first universal-on-Place wiring in the codebase — establishes a new precedent (per S128 spec D12 and `references/worldwake-validation-patterns.md` "New Component on EntityKind::Place" pattern).
4. No existing scenario authors `sleep_quality` (`grep "sleep_quality" scenarios/*.ron` returns zero matches per reassessment Step 2 sub-check). All existing scenarios load unchanged: places without `sleep_quality` get `SleepQualityProfile::default()` automatically via the new spawn loop.
5. `SleepQualityProfileDef` mirrors `SleepQualityProfile` but uses `u16` for `recovery_modifier` (permille value, RON-friendly) instead of `Permille`. Conversion must validate the value with `Permille::new(...)` and return a scenario-load error for values above `1000`; the corrected S128 spec no longer permits recovery amplification above the `Permille` range.
6. Information-path refactor (Rule 16): adds the `sleep_quality` authoring path. No old path exists. Canonical path is `PlaceDef.sleep_quality` → validated conversion to `SleepQualityProfile` → `set_component_sleep_quality_profile`. No alias path.

## Architecture Check

1. The unconditional spawn pattern matches the agent universal pattern (`metabolism_profile.unwrap_or_default()` at `mod.rs:576`), adapted for places. This honors S128 spec's "universal place component" intent literally — every place actually carries the component, so runtime reads can use `expect()` with no fallback branch.
2. The optional `PlaceDef.sleep_quality` field with `unwrap_or_default()` resolution preserves scenario brevity: authors only write `sleep_quality` when they want non-default modulation.
3. Establishing the universal-on-Place precedent now (rather than later) means future place components (e.g., S129 `PlaceDirtiness`) have a concrete pattern to follow. Per the audit Improvement #2 from the reassessment, this gap was flagged as needing a precedent — this ticket creates it.

## Verification Layers

1. Scenario load with `sleep_quality: Some(...)` produces a place with the authored profile → focused unit test in `crates/worldwake-cli/src/scenario/mod.rs` test module loading a minimal scenario with one authored place and asserting `txn.get_component_sleep_quality_profile(place)` returns the expected profile.
2. Scenario load with `sleep_quality` omitted produces a place with `SleepQualityProfile::default()` → focused unit test loading a place without the field and asserting `txn.get_component_sleep_quality_profile(place) == SleepQualityProfile::default()`.
3. Existing scenarios in `scenarios/*.ron` load without modification → existing scenario load tests pass; spot-check at least one canonical scenario (`survival-baseline.ron`) loads and every place has a queryable `SleepQualityProfile`.
4. Single-layer ticket: scenario authoring is a CLI-layer concern with no engine changes. Verification stays at focused unit tests for scenario load behavior; no decision-trace, action-trace, or golden E2E assertions are appropriate.

## What to Change

### 1. Add `SleepQualityProfileDef` type in `crates/worldwake-cli/src/scenario/types.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SleepQualityProfileDef {
    pub shelter: ShelterTag,
    pub ground_comfort: GroundComfortTag,
    pub recovery_modifier: u16, // permille value
}

impl SleepQualityProfileDef {
    fn into_profile(self) -> Result<SleepQualityProfile, String> {
        Ok(SleepQualityProfile {
            shelter: self.shelter,
            ground_comfort: self.ground_comfort,
            recovery_modifier: Permille::new(self.recovery_modifier)
                .ok_or_else(|| format!("sleep_quality.recovery_modifier {} exceeds 1000", self.recovery_modifier))?,
        })
    }
}
```

`ShelterTag` and `GroundComfortTag` are re-exported from `worldwake-core`; import them at the top of `types.rs`. `SleepQualityProfile` and `Permille` are similarly imported.

### 2. Add `sleep_quality` field to `PlaceDef`

```rust
pub struct PlaceDef {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<PlaceTag>,
    #[serde(default)]
    pub visibility_profile: Option<PlaceVisibilityProfile>,
    #[serde(default)]
    pub sleep_quality: Option<SleepQualityProfileDef>,
}
```

### 3. Modify `spawn_place` loop in `crates/worldwake-cli/src/scenario/mod.rs`

Replace the existing optional pattern with two adjacent loops (or merge into one — preserve the existing visibility_profile loop's behavior unchanged, then add the sleep_quality loop):

```rust
for place_def in &def.places {
    let place_id = resolve_name(names, &place_def.name, "place sleep_quality")?;
    let profile: SleepQualityProfile = place_def
        .sleep_quality
        .as_ref()
        .cloned()
        .map(SleepQualityProfileDef::into_profile)
        .transpose()?
        .unwrap_or_default();
    txn.set_component_sleep_quality_profile(place_id, profile)?;
}
```

Place this immediately after the existing `visibility_profile` loop. The result: every place loaded from any scenario carries a `SleepQualityProfile` component, defaulting to `(Open, Earth, 1000)` when not authored.

### 4. (Optional follow-up) Author per-place `SleepQualityProfile` in `survival-baseline.ron`

S128 Motivating Evidence calls out Hillside Shelter, Riverside Camp, Forest Clearing, Fertile Fields. The spec D3 example numbers are:

- Hillside Shelter: `(Shelter, Soft, 1000)` — best / unpenalized
- Riverside Camp: `(Roofed, Earth, 900)`
- Forest Clearing: `(PartialCover, Earth, 800)`
- Fertile Fields: `(Open, Earth, 700)` — worst

Authoring these is technically out of scope per spec D12 ("Survival-baseline rebalance is a follow-up ticket"), but this ticket may include the four-place authoring as a courtesy update if the implementer determines it does not block other in-flight work. If authored, S128SLEEPIPLA-007's golden test 5 (site preference) becomes directly executable. **Recommendation**: author the four places in this ticket, since the work is mechanical and unblocks downstream test coverage. Flag clearly in the implementation summary if deferred.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify — add `SleepQualityProfileDef` type with checked conversion helper, add `sleep_quality` field to `PlaceDef`)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify — add unconditional `spawn_place` loop for `SleepQualityProfile`)
- `Likely: scenarios/survival-baseline.ron` (modify if authoring the four-place rebalance per Section 4 above)

## Out of Scope

- New universal place component types beyond `SleepQualityProfile` — out of scope; this ticket establishes the precedent only
- Authoring `sleep_quality` in scenarios beyond `survival-baseline.ron` (if even that one is included) — defer to follow-up scenario-tuning tickets
- Runtime reads of the universal place component — handled by S128SLEEPIPLA-004 (handler) and S128SLEEPIPLA-005 (candidate emitter)
- Changing the existing `PlaceVisibilityProfile` to a universal pattern — out of scope; visibility_profile remains optional per its existing contract

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-cli scenario sleep_quality_authored` (new) — minimal scenario with `sleep_quality: Some(...)` → place carries the authored profile.
2. `cargo test -p worldwake-cli scenario sleep_quality_default_when_omitted` (new) — minimal scenario without `sleep_quality` → place carries `SleepQualityProfile::default()`.
3. `cargo test -p worldwake-cli scenario sleep_quality_rejects_out_of_range_recovery_modifier` (new) — authored `recovery_modifier > 1000` fails scenario loading with a clear error.
4. `cargo test -p worldwake-cli` — existing scenario tests pass; specifically scenarios loading `survival-baseline.ron` succeed.
5. `cargo test -p worldwake-systems` — existing tests pass (sleep behavior unchanged at this ticket; consumers land in -004 / -005).
6. Existing suite: `cargo test --workspace`.

### Invariants

1. Every place created via `spawn_place` (or the equivalent scenario loader) carries a `SleepQualityProfile` component.
2. `txn.get_component_sleep_quality_profile(place)` returns the authored profile when `sleep_quality: Some(...)` is present with a valid `recovery_modifier`, and `SleepQualityProfile::default()` otherwise.
3. Existing scenarios in `scenarios/*.ron` load without modification — none currently use `sleep_quality`, and the deserializer's `#[serde(default)]` makes it optional.
4. `SleepQualityProfileDef.recovery_modifier: u16` converts to `SleepQualityProfile.recovery_modifier: Permille` via checked `Permille::new`; values above `1000` fail scenario loading with a clear error instead of silently creating an invalid profile.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/mod.rs` test module (modify — add `sleep_quality_authored_place_carries_profile`, `sleep_quality_omitted_place_carries_default`, and `sleep_quality_rejects_out_of_range_recovery_modifier`).
2. `crates/worldwake-cli/src/scenario/types.rs` test module (modify if a test module exists, otherwise skip — add a focused conversion test for `SleepQualityProfileDef → SleepQualityProfile`, including rejection of `recovery_modifier > 1000`).

### Commands

1. `cargo test -p worldwake-cli scenario`
2. `cargo test -p worldwake-cli` (full crate)
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `./scripts/verify.sh`
