# S141MOTSOULED-002: `UtilityProfile` motive-class weight fields

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `worldwake-core::UtilityProfile` field extension; `SAVE_FORMAT_VERSION` bump
**Deps**: spec S141 deliverable D4 (independent of 001 — UtilityProfile changes don't reference `MotiveSource`)

## Problem

S141's `motive_score` body refactor (owned by 004) reads per-motive-class weights from `UtilityProfile`. The 7 kept `MotiveSource` variants need 5 additional `Permille` fields on `UtilityProfile` — the existing per-need weights cover `NeedPressure`, the existing `pain_weight` covers `Pain`, and the new fields cover `OfficeDuty`, `Loyalty`, `Greed`, `Shame`, `Revenge`. Without these fields, 004's per-variant scoring helpers cannot read agent-specific weights, and FND-22 (agent diversity through concrete variation) is unsatisfied for the new motive classes.

This ticket is independent of 001 because the 5 new fields are forward references for 004 — they exist as unread defaults until 004 lands. The transient state is not dead code in a live authority path (motive_score still reads via the existing `match goal_kind` body until 004 flips it).

## Assumption Reassessment (2026-05-12)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `UtilityProfile` currently has 15 fields and a `Default` impl at `crates/worldwake-core/src/utility_profile.rs:8,41-65` (per S141 reassessment). The existing `pain_weight: Permille` at line 20 stays — the spec already accounts for it as the `Pain` variant's weight. Existing focused tests at `crates/worldwake-core/src/utility_profile.rs#[cfg(test)]` (e.g., the default roundtrip assertion at line 95) need new-field updates. No runtime trace or golden test references the 5 new field names yet; they are net-new.
2. `SAVE_FORMAT_VERSION` was 77 at `crates/worldwake-sim/src/save_load.rs:6`. This ticket bumps it to 78 to signal the S141 schema era. 003 (adds `RankedGoalSummary.motive_source_contributions`) and 005 (adds `GoalCommittedPayload.decisive_motive_sources`) deliberately do NOT bump further. Their new fields should use `#[serde(default)]` for omitted-field serde at the payload/current-format boundary, but full pre-bump save files with header version 77 remain rejected by the loader after this ticket.
3. Shared abstraction boundary: `UtilityProfile` is the per-agent profile authoring surface read by `ranking.rs` via `RankingContext.utility`. Its construction shape is the contract under audit. Live reassessment found the exhaustive constructor fallout was smaller than the draft's 67-site estimate because many `UtilityProfile` literals already use `..UtilityProfile::default()` and inherit the new fields. The full manual literals that compiled as incomplete were updated explicitly. Authored scenario files that omit the new fields continue to load because the new fields have `#[serde(default = "...")]`.

## Architecture Check

1. Per-`MotiveSource`-class weights on `UtilityProfile` (per-agent state) deliver FND-22 (concrete per-agent variation) — two agents with identical world state but different `greed_weight` rank the same `Greed(opportunity)` motive differently. This is preferred over a global tuning constant on the comparator.
2. `#[serde(default = "...")]` per field preserves omitted-field deserialization for `UtilityProfile` payloads while the save-format version bump keeps full older saves rejected by the loader. No backward-compatibility shim is added to live authoritative paths; the serde defaults are confined to the deserialization boundary (FND-28-compliant).
3. The 5 new defaults (3 at `pm(500)` balanced, 2 at `pm(400)` for shame/revenge) bias against runaway feedback loops on the violence axis without introducing an artificial cap (FND-11) — the dampener is per-agent profile authoring, not a numeric clamp.

## Verification Layers

1. UtilityProfile shape and Default -> focused unit tests in `crates/worldwake-core/src/utility_profile.rs#[cfg(test)]` (expanded existing default-value assertion; added serde/default test asserts each of the 5 added fields' Default value matches the spec's table).
2. Save-load round-trip with version bump -> focused integration test in `crates/worldwake-sim/src/save_load.rs#[cfg(test)]` (round-trip a `UtilityProfile` snapshot through version 78; assert the 5 added fields persist correctly).
3. Omitted-field deserialization -> focused test loading a 15-field RON `UtilityProfile` payload with 78-aware code, asserting the 5 added fields populate from `#[serde(default)]` and the rest match. Full version-77 save files are still rejected by `load_from_bytes`.
4. Scenario authoring → existing scenario suite passes without `utility_profile:` block edits (proof that `#[serde(default)]` covers the optional-author case).
5. Generated documentation -> `docs/profiles/all-profiles.md` regenerated via `python3 scripts/profile_docs.py --write`; the added fields appear in the rendered UtilityProfile section.

## What to Change

### 1. Extend `UtilityProfile` struct

At `crates/worldwake-core/src/utility_profile.rs:8` add 5 fields after the existing field block, each with its own `#[serde(default = "default_<name>")]` annotation:

```rust
#[serde(default = "default_office_duty_weight")]
pub office_duty_weight: Permille,
#[serde(default = "default_loyalty_weight")]
pub loyalty_weight: Permille,
#[serde(default = "default_greed_weight")]
pub greed_weight: Permille,
#[serde(default = "default_shame_weight")]
pub shame_weight: Permille,
#[serde(default = "default_revenge_weight")]
pub revenge_weight: Permille,
```

Add the corresponding 5 helper functions at module scope:

```rust
fn default_office_duty_weight() -> Permille { Permille::new_unchecked(500) }
fn default_loyalty_weight()      -> Permille { Permille::new_unchecked(500) }
fn default_greed_weight()        -> Permille { Permille::new_unchecked(500) }
fn default_shame_weight()        -> Permille { Permille::new_unchecked(400) }
fn default_revenge_weight()      -> Permille { Permille::new_unchecked(400) }
```

Per the spec D4 defaults table: 3 balanced (`pm(500)`) and 2 slight-downweight (`pm(400)` for shame/revenge to dampen feedback loops).

### 2. Extend `Default for UtilityProfile`

At `crates/worldwake-core/src/utility_profile.rs:41-65` add the 5 new fields to the existing `Default` body using the same helper functions (so `Default` and `#[serde(default)]` agree).

### 3. Bump `SAVE_FORMAT_VERSION`

At `crates/worldwake-sim/src/save_load.rs:6` change `pub const SAVE_FORMAT_VERSION: u32 = 77;` to `78`. The on-disk loader at line 129 already routes `SAVE_FORMAT_VERSION => load_current_format(payload)`; no further matching arm is needed because pre-S141 saves are rejected by version mismatch (this is the FND-28-compliant pattern — no legacy load path).

### 4. Update exhaustive `UtilityProfile { ... }` literal construction sites

Compile fallout found full manual literals only in:

- `crates/worldwake-ai/src/goal_explanation.rs`
- `crates/worldwake-ai/src/ranking.rs`
- `crates/worldwake-ai/tests/golden_planner_pathology.rs`
- `crates/worldwake-core/src/test_utils.rs`
- `crates/worldwake-core/src/utility_profile.rs`
- `crates/worldwake-sim/src/save_load.rs`

Those sites now add all 5 new fields by name. Existing `..UtilityProfile::default()` literals were left as-is and inherit the new default values; no new spread syntax was introduced.

### 5. Regenerate `docs/profiles/all-profiles.md`

Run `python3 scripts/profile_docs.py --write` after the field additions. Confirm the generated diff includes the 5 new UtilityProfile rows.

## Files to Touch

Core / sim:
- `crates/worldwake-core/src/utility_profile.rs` (modify — fields, Default, helpers, tests)
- `crates/worldwake-core/src/test_utils.rs` (modify — `sample_utility_profile`)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` 77→78)

AI runtime construction sites:
- `crates/worldwake-ai/src/ranking.rs`
- `crates/worldwake-ai/src/goal_explanation.rs`

AI test construction sites:
- `crates/worldwake-ai/tests/golden_planner_pathology.rs`

Generated:
- `docs/profiles/all-profiles.md` (regenerated)

## Out of Scope

- Reading the 5 new fields from `motive_score` — owned by 004 (the field values exist as defaults but no scoring helper reads them yet).
- `ProfileHomogeneity` lint extension to detect cloned values across the new fields — owned by 007 (validation deliverable).
- Any scenario `.ron` file edits to author non-default values for the new weights — optional and scenario-author-driven; not load-bearing for compilation because `#[serde(default)]` covers absence.
- The `MotiveSource` enum and `MotiveSourceRef` carrier — owned by `archive/tickets/S141MOTSOULED-001.md`.
- Save-format bumps for the other S141 fields (`RankedGoalSummary` in 003, `GoalCommittedPayload` in 005) — they ride the 77→78 bump set here via `#[serde(default)]`.

## Acceptance Criteria

### Tests That Must Pass

1. UtilityProfile shape: `assert_eq!(UtilityProfile::default().office_duty_weight, Permille::new_unchecked(500))` and 4 sibling assertions for the other added fields.
2. Bincode round-trip: `UtilityProfile::default()` survives bincode encode/decode unchanged across all 20 fields.
3. Omitted-field deserialization: a 15-field `UtilityProfile` payload deserializes successfully into the 20-field struct, with the 5 added fields populated from `#[serde(default)]` helpers. Full save files with version 77 are rejected after the version bump.
4. Existing suite: `cargo test --workspace` (every construction site must compile after the field additions; existing goldens unaffected by the field additions because no consumer reads the added weights yet).

### Invariants

1. `Default for UtilityProfile` and `#[serde(default = "default_X_weight")]` produce identical values for every added field; divergence would mean two construction paths yield different per-agent state from the same authoritative type, which would violate FND-3.
2. `SAVE_FORMAT_VERSION` is the single source of truth for save-format era; downstream tickets 003 and 005 do NOT bump the version further.
3. No added `..Default::default()` spread syntax was introduced; existing spread literals inherit the added fields through `Default`.

## Test Plan

### Added/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs#[cfg(test)]` — extended the existing default-value assertion to cover all 20 fields (15 existing + 5 added); added an omitted-field serde test asserting a 15-field `UtilityProfile` RON payload loads with the added fields defaulted.
2. `crates/worldwake-sim/src/save_load.rs#[cfg(test)]` — round-trip an event log containing a `UtilityProfile` component at version 78; assert the byte stream round-trips byte-identically.

### Commands

1. `cargo test -p worldwake-core utility_profile`
2. `cargo test -p worldwake-sim save_load`
3. `cargo test --workspace` (full sweep proves all construction sites compile)
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `python3 scripts/profile_docs.py --write` (regenerates `docs/profiles/all-profiles.md`; manually review the diff to confirm the 5 added fields appear)

## Outcome

Completed on 2026-05-12.

- Added `office_duty_weight`, `loyalty_weight`, `greed_weight`, `shame_weight`, and `revenge_weight` to `UtilityProfile` with per-field serde defaults and matching `Default` values.
- Bumped `SAVE_FORMAT_VERSION` from 77 to 78 and extended the save-load round-trip fixture to persist non-default motive-class weights.
- Updated exhaustive `UtilityProfile` literals surfaced by all-target compile fallout and regenerated `docs/profiles/all-profiles.md`.
- Left `motive_score` and runtime use of the new fields to S141MOTSOULED-004, as planned.

## Deviations

- The draft's "67 exhaustive literals" count was stale. Existing default-spread literals inherited the new fields; only compiler-exposed full literals were patched explicitly.
- The drafted "version-77 byte stream loads under version 78" wording was narrowed. This ticket proves omitted-field serde for bare/current-format `UtilityProfile` payloads, while full save files with header version 77 remain rejected after the version bump.
- `python3 scripts/profile_docs.py --write` completed and regenerated the UtilityProfile rows, while also reporting 15 pre-existing documentation-gap warnings for unrelated profile docs.

## Verification Result

- Passed `python3 scripts/profile_docs.py --write`
- Passed `cargo fmt --all`
- Passed `cargo test --workspace --no-run`
- Passed `cargo test -p worldwake-core utility_profile`
- Passed `cargo test -p worldwake-sim save_load`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`

Merge note: Bumps `SAVE_FORMAT_VERSION` 77→78 to signal the S141 schema era. Tickets 003 (adds field to `RankedGoalSummary`) and 005 (adds field to `GoalCommittedPayload`) deliberately do NOT bump further. Their new fields use `#[serde(default)]` for omitted-field payload/current-format deserialization; full pre-78 save files remain rejected by the save header version gate.
