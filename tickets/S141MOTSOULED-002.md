# S141MOTSOULED-002: `UtilityProfile` motive-class weight fields

**Status**: PENDING
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
2. `SAVE_FORMAT_VERSION` is currently 77 at `crates/worldwake-sim/src/save_load.rs:6`. This ticket bumps it to 78 to signal the S141 schema era. 003 (adds `RankedGoalSummary.motive_source_contributions`) and 005 (adds `GoalCommittedPayload.decisive_motive_sources`) deliberately do NOT bump further — their new fields use `#[serde(default)]` (defaulting to `Vec::new()`) to keep mid-migration saves loadable under version 78.
3. Shared abstraction boundary: `UtilityProfile` is the per-agent profile authoring surface read by `ranking.rs` via `RankingContext.utility`. Its construction shape is the contract under audit. Per Step 2 sub-check (d), 67 struct-literal construction sites enumerate every field with no `..Default::default()` spread syntax, so adding 5 required fields touches every site. The 10 `scenarios/*.ron` files that author `utility_profile:` continue to load without changes because the new fields have `#[serde(default = "...")]`.

## Architecture Check

1. Per-`MotiveSource`-class weights on `UtilityProfile` (per-agent state) deliver FND-22 (concrete per-agent variation) — two agents with identical world state but different `greed_weight` rank the same `Greed(opportunity)` motive differently. This is preferred over a global tuning constant on the comparator.
2. `#[serde(default = "...")]` per field preserves backward-compatible loading of pre-S141 saves while the version bump signals forward-incompatibility to old loaders. No backward-compatibility shim is added to live authoritative paths; the serde defaults are confined to the deserialization boundary (FND-28-compliant).
3. The 5 new defaults (3 at `pm(500)` balanced, 2 at `pm(400)` for shame/revenge) bias against runaway feedback loops on the violence axis without introducing an artificial cap (FND-11) — the dampener is per-agent profile authoring, not a numeric clamp.

## Verification Layers

1. UtilityProfile shape and Default → focused unit tests in `crates/worldwake-core/src/utility_profile.rs#[cfg(test)]` (expanded existing default-value assertion at line 95; new test asserting each of the 5 new fields' Default value matches the spec's table).
2. Save-load round-trip with version bump → focused integration test in `crates/worldwake-sim/src/save_load.rs#[cfg(test)]` (round-trip a `UtilityProfile` snapshot through version 78; assert the 5 new fields persist correctly).
3. Backward-compatible deserialization → focused test loading a version-77 byte stream with 78-aware code, asserting the 5 new fields populate from `#[serde(default)]` and the rest match.
4. Scenario authoring → existing scenario suite passes without `utility_profile:` block edits (proof that `#[serde(default)]` covers the optional-author case).
5. Generated documentation → `docs/profiles/all-profiles.md` regenerated via `python3 scripts/profile_docs.py --write`; the new fields appear in the rendered UtilityProfile section.

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

### 4. Update every `UtilityProfile { ... }` literal construction site

Per Step 2 sub-check (d), 67 sites across the workspace enumerate all fields by name with no `..Default::default()` spread escape. Each must add the 5 new fields with explicit values (or with the same default helpers). The per-file site counts to update:

- `crates/worldwake-ai/src/ranking.rs` (14 sites)
- `crates/worldwake-ai/tests/golden_travel_physiology.rs` (12 sites)
- `crates/worldwake-ai/src/candidate_generation.rs` (10 sites)
- `crates/worldwake-ai/tests/golden_harness/soak_world.rs` (9 sites)
- `crates/worldwake-ai/tests/golden_simulation_gaps.rs` (8 sites)
- `crates/worldwake-ai/tests/golden_ai_decisions.rs` (8 sites)
- `crates/worldwake-ai/tests/golden_exploration.rs` (5 sites)
- `crates/worldwake-core/src/utility_profile.rs` (4 sites — `Default` + tests)
- `crates/worldwake-ai/tests/golden_planner_pathology.rs` (3 sites)
- `crates/worldwake-ai/tests/golden_offices.rs` (3 sites)
- Remaining smaller sites in `crates/worldwake-core/src/test_utils.rs` (1 — `sample_utility_profile`), `crates/worldwake-core/src/belief.rs`, `crates/worldwake-sim/src/per_agent_belief_view.rs`, `crates/worldwake-systems/src/office_actions.rs`, `crates/worldwake-ai/src/goal_explanation.rs`, `crates/worldwake-ai/src/agent_tick/tests.rs`, and several `crates/worldwake-ai/tests/golden_*.rs` files (`golden_merchant_selling.rs`, `golden_source_composite.rs`, `golden_source_reliability.rs`, `golden_place_dirtiness.rs`, `golden_sleep_episode.rs`, `golden_item_decay.rs`, `golden_survival_drive_escalation.rs`, `golden_harness/mod.rs`) — together comprising the remaining ~14 sites of the 67 total.

For mechanical sweep correctness, prefer adding all 5 new fields by name (matching the existing site convention of explicit per-field listing) rather than introducing `..Default::default()` spread for the first time — spread-syntax introduction would diverge from this codebase's established field-enumeration convention.

### 5. Regenerate `docs/profiles/all-profiles.md`

Run `python3 scripts/profile_docs.py --write` after the field additions. Confirm the generated diff includes the 5 new UtilityProfile rows.

## Files to Touch

Core / sim:
- `crates/worldwake-core/src/utility_profile.rs` (modify — fields, Default, helpers, tests)
- `crates/worldwake-core/src/test_utils.rs` (modify — `sample_utility_profile`)
- `crates/worldwake-core/src/belief.rs` (modify — construction sites)
- `crates/worldwake-sim/src/save_load.rs` (modify — `SAVE_FORMAT_VERSION` 77→78)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-systems/src/office_actions.rs` (modify)

AI runtime construction sites:
- `crates/worldwake-ai/src/ranking.rs`
- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/goal_explanation.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`

AI test construction sites:
- `crates/worldwake-ai/tests/golden_travel_physiology.rs`
- `crates/worldwake-ai/tests/golden_harness/soak_world.rs`
- `crates/worldwake-ai/tests/golden_harness/mod.rs`
- `crates/worldwake-ai/tests/golden_simulation_gaps.rs`
- `crates/worldwake-ai/tests/golden_ai_decisions.rs`
- `crates/worldwake-ai/tests/golden_exploration.rs`
- `crates/worldwake-ai/tests/golden_planner_pathology.rs`
- `crates/worldwake-ai/tests/golden_offices.rs`
- `crates/worldwake-ai/tests/golden_merchant_selling.rs`
- `crates/worldwake-ai/tests/golden_source_composite.rs`
- `crates/worldwake-ai/tests/golden_source_reliability.rs`
- `crates/worldwake-ai/tests/golden_place_dirtiness.rs`
- `crates/worldwake-ai/tests/golden_sleep_episode.rs`
- `crates/worldwake-ai/tests/golden_item_decay.rs`
- `crates/worldwake-ai/tests/golden_survival_drive_escalation.rs`

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

1. UtilityProfile shape: `assert_eq!(UtilityProfile::default().office_duty_weight, Permille::new_unchecked(500))` and 4 sibling assertions for the other new fields.
2. Bincode round-trip: `UtilityProfile::default()` survives bincode encode/decode unchanged across all 20 fields.
3. Cross-version deserialization: a version-77 byte stream of a 15-field `UtilityProfile` deserializes successfully into the 20-field struct, with the 5 new fields populated from `#[serde(default)]` helpers.
4. Existing suite: `cargo test --workspace` (every construction site must compile after the field additions; existing goldens unaffected by the field additions because no consumer reads the new weights yet).

### Invariants

1. `Default for UtilityProfile` and `#[serde(default = "default_X_weight")]` produce identical values for every new field — divergence would mean two construction paths yield different per-agent state from the same authoritative type, which would violate FND-3.
2. `SAVE_FORMAT_VERSION` is the single source of truth for save-format era; downstream tickets 003 and 005 do NOT bump the version further.
3. No `..Default::default()` spread syntax is introduced at any of the 67 sites — preserves the codebase's existing convention of explicit field enumeration.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs#[cfg(test)]` — extend the existing default-value assertion at line 95 to cover all 20 fields (15 existing + 5 new); add a cross-version-deserialization test asserting a version-77 byte stream loads with the new fields defaulted.
2. `crates/worldwake-sim/src/save_load.rs#[cfg(test)]` — round-trip an event log containing a `UtilityProfile` component at the new version 78; assert the byte stream round-trips byte-identically.

### Commands

1. `cargo test -p worldwake-core utility_profile`
2. `cargo test -p worldwake-sim save_load`
3. `cargo test --workspace` (full sweep proves all 67 construction sites compile)
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `python3 scripts/profile_docs.py --write` (regenerates `docs/profiles/all-profiles.md`; manually review the diff to confirm the 5 new fields appear)

Merge note: Bumps `SAVE_FORMAT_VERSION` 77→78 to signal the S141 schema era. Tickets 003 (adds field to `RankedGoalSummary`) and 005 (adds field to `GoalCommittedPayload`) deliberately do NOT bump further — their new fields use `#[serde(default)]` (defaulting to empty `Vec`) so mid-migration saves load correctly under version 78. Land 002 before any sibling S141 ticket that touches serialized state.
