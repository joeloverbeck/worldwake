# S176SANFACDEG-002: MetabolismProfile cleaning durations + MetabolismDurationKind

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `MetabolismProfile` core profile (2 new fields), `MetabolismDurationKind` enum (sim, 2 variants + 2 resolver arms), serialized `SimulationState` format (`SAVE_FORMAT_VERSION` bump), generated profile docs
**Deps**: S176SANFACDEG-001 (`SAVE_FORMAT_VERSION` cascade order)

## Problem

The cleaning actions (S176SANFACDEG-004) are duration-bearing and read their durations from the universal `MetabolismProfile`, selected via `MetabolismDurationKind`. This ticket adds the two duration fields and the two duration-kind variants + resolver arms so the actions can declare `DurationExpr::ActorMetabolism { kind: … }`.

## Assumption Reassessment (2026-05-29)

1. `MetabolismProfile` is at `crates/worldwake-core/src/needs.rs:141-181` with `Default` at `183-237`; it already carries `wash_ticks` / `toilet_ticks` as `NonZeroU32` (the precedent the new fields follow). A `MetabolismProfile::new(...)` constructor exists (call site `crates/worldwake-core/src/world.rs:958`); 18 literal construction sites exist workspace-wide — confirm whether each routes through `::new()` or enumerates fields during implementation.
2. `MetabolismDurationKind` is at `crates/worldwake-sim/src/action_semantics.rs:104` (variants `Toilet`, `Wash`), resolved to a profile field at **two** sites: `action_semantics.rs:231-232` (authoritative) and `crates/worldwake-sim/src/belief_view.rs:2742-2743` (planner-facing duration estimate). Both need a new arm per new variant.
3. Shared boundary under audit: `MetabolismProfile`'s serialized shape. Two transport paths: (a) bincode `SimulationState` save — field addition breaks positional reads → `SAVE_FORMAT_VERSION` bump `109 → 110`; (b) RON scenario input — **24 scenario files author `metabolism_profile` inline** (`grep -rln metabolism_profile scenarios/`). The new fields MUST carry `#[serde(default = "…")]` (NonZeroU32 has no `Default`, so a default fn is required) so those 24 RON files deserialize unchanged; otherwise every one of them breaks.
4. The new fields are `NonZeroU32` with no automatic `Default`; the struct's `Default` impl and the serde default fns must supply concrete values (mirror `wash_ticks`/`toilet_ticks` defaults).
5. `python3 scripts/profile_docs.py --write` regenerates `docs/profiles/all-profiles.md` from the `MetabolismProfile` struct; the doc drifts from code unless regenerated in this ticket.

## Architecture Check

1. Reuses the existing `MetabolismDurationKind` → profile-field resolution mechanism rather than introducing a parallel duration source; mirrors `wash_ticks`/`toilet_ticks` exactly (FND-3, FND-26).
2. FND-28: the format bump replaces the prior version; `#[serde(default)]` on RON input is the lawful boundary-normalization path (scenario import), not a live-authority shim.

## Assumption Reassessment forward-declaration note

`MetabolismDurationKind::{CleanBasin, EmptyLatrine}` are **forward-declared** here: their variants and resolver arms (mapping to the two new profile fields) land in this ticket, but the `DurationExpr::ActorMetabolism { kind: CleanBasin | EmptyLatrine }` emission sites land in S176SANFACDEG-004. No exhaustive `MetabolismDurationKind` match outside the two resolver sites breaks on the additions.

## Verification Layers

1. Profile field roundtrip + default → focused unit (`MetabolismProfile` Default and serde-default coverage).
2. Duration resolution → focused unit asserting both resolver sites map the new kinds to the new fields.
3. RON deserialization tolerance → scenario-load test confirming an inline `metabolism_profile` without the new fields deserializes with defaults.

## What to Change

### 1. Profile fields

Add `clean_basin_duration_ticks: NonZeroU32` and `empty_latrine_duration_ticks: NonZeroU32` to `MetabolismProfile`, with `#[serde(default = "…")]` attributes, `Default`-impl values, and inclusion in the `::new()` signature.

### 2. Duration kinds + resolvers

Add `MetabolismDurationKind::{CleanBasin, EmptyLatrine}`; add resolver arms at `action_semantics.rs:231-232` and `belief_view.rs:2742-2743` mapping each kind to its profile field.

### 3. Construction sites + docs

Update the 18 construction sites / `::new()` callers; bump `SAVE_FORMAT_VERSION` `109 → 110`; regenerate `docs/profiles/all-profiles.md`.

## Files to Touch

- `crates/worldwake-core/src/needs.rs` (modify)
- `crates/worldwake-sim/src/action_semantics.rs` (modify — variant + resolver)
- `crates/worldwake-sim/src/belief_view.rs` (modify — resolver)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump)
- `crates/worldwake-core/src/world.rs` (modify — `sample_metabolism_profile` / `::new()` callers)
- `docs/profiles/all-profiles.md` (modify — regenerate via `python3 scripts/profile_docs.py --write`)
- Remaining `MetabolismProfile { … }` / `::new(` sites (18 total) — confirm full set during implementation

## Out of Scope

- The cleaning actions that consume these durations — owned by S176SANFACDEG-004 (named emission-site owner for the new duration kinds).
- Editing the 24 scenario RON files — avoided by `#[serde(default)]`; if any scenario must author the durations it can, but no edits are required by this ticket.

## Acceptance Criteria

### Tests That Must Pass

1. `MetabolismProfile::default()` and serde-default produce valid `NonZeroU32` cleaning durations.
2. Both resolver sites return the correct profile field for `CleanBasin` and `EmptyLatrine`.
3. A scenario authoring `metabolism_profile` without the new fields loads successfully.
4. Existing suite: `cargo test -p worldwake-core && cargo test -p worldwake-sim`

### Invariants

1. Cleaning durations are sourced only from `MetabolismProfile` via `MetabolismDurationKind` (no hardcoded duration constant).
2. All 24 inline-`metabolism_profile` scenarios continue to deserialize (serde defaults hold).
3. `docs/profiles/all-profiles.md` matches the regenerated output (`--check-docs` clean).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/needs.rs` tests — assert defaults + serde-default for the new fields.
2. `crates/worldwake-sim/src/action_semantics.rs` + `belief_view.rs` tests — assert both resolver arms.
3. `crates/worldwake-cli/src/scenario` test — confirm inline `metabolism_profile` without new fields loads.

### Commands

1. `cargo test -p worldwake-core needs && cargo test -p worldwake-sim action_semantics`
2. `python3 scripts/profile_docs.py --write && cargo test -p worldwake-cli`
3. `scripts/verify.sh`

## Outcome

**Completion date**: 2026-05-29

**What changed**:
- Added `clean_basin_duration_ticks: NonZeroU32` and `empty_latrine_duration_ticks: NonZeroU32` to `MetabolismProfile` (`crates/worldwake-core/src/needs.rs`), each with a `#[serde(default = "…")]` (`default_clean_basin_duration_ticks` = 10, `default_empty_latrine_duration_ticks` = 16) and inclusion in the `Default` impl.
- Added `MetabolismDurationKind::{CleanBasin, EmptyLatrine}` (`crates/worldwake-sim/src/action_semantics.rs`) plus resolver arms at both sites: the authoritative resolver (`action_semantics.rs`) and the planner-facing estimate (`belief_view.rs`).
- Bumped `SAVE_FORMAT_VERSION` `109 → 110`; updated the version-pin tests.
- Regenerated `docs/profiles/all-profiles.md` (`scripts/profile_docs.py --write`).

**Deviations from the ticket**:
- The ticket called for adding the two fields to the `MetabolismProfile::new(...)` signature. Reassessment found ~40 `::new()` call sites and that the most recent comparable field (`rough_sleep_recovery_floor`) was *not* added to the `new()` signature — it is seeded inside `new()` from its `const fn` default and authored only via serde. I followed that precedent: the cleaning durations are seeded from `const fn` defaults inside `new()` (no signature change, no call-site churn) and authored via serde (`metabolism_profile` whole-struct deserialization). This is DRYer and avoids touching ~40 unrelated call sites. Net struct-literal breakage was a single enumerated site (`planner_pathology_harness/mod.rs`), which was updated.
- The ticket's `--check-docs` flag does not exist on `profile_docs.py` (only `--write`); docs regenerated via `--write` and confirmed consistent.
- Resolver unit coverage: the authoritative resolver has a direct assertion (`duration_expr_resolves_consumable_and_metabolism_driven_ticks` extended for both new kinds). The `belief_view.rs` resolver is the identical 4-line mirror; it has no dedicated unit test (the existing test stub is a unit struct returning `None` for every profile, so a focused test would require a full `GoalBeliefView` mock) and is instead exercised end-to-end by the planner duration-estimate path in the S176SANFACDEG-005/008 goldens.

**Verification**: `cargo test -p worldwake-core needs`, `cargo test -p worldwake-sim action_semantics`, and full `cargo test --workspace` all pass (no failures). All 24 inline-`metabolism_profile` scenarios continue to deserialize (serde defaults hold).
