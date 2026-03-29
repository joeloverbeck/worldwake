# S35OBSACTSIG-002: Add `activity_awareness_weight` to `UtilityProfile`

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core` `UtilityProfile` schema plus explicit constructor updates
**Deps**: None

## Problem

The competition-aware ranking work needs a concrete per-agent parameter for how strongly an agent discounts crowded opportunities. The live authoritative utility profile already holds the other durable per-agent decision weights, but it has no `activity_awareness_weight`. Without that field, later ranking changes would either hardcode a global constant or introduce a parallel config path for one ranking concern.

## Assumption Reassessment (2026-03-29)

1. `UtilityProfile` is defined in `crates/worldwake-core/src/utility_profile.rs` and currently contains 11 `Permille` fields: `hunger_weight`, `thirst_weight`, `fatigue_weight`, `bladder_weight`, `dirtiness_weight`, `pain_weight`, `danger_weight`, `enterprise_weight`, `social_weight`, `courage`, and `care_weight`. No `activity_awareness_weight` exists today.
2. `UtilityProfile` derives `Clone`, `Debug`, `Eq`, `PartialEq`, `Serialize`, and `Deserialize`, and `Default` is hand-written in the same file. Existing unit coverage already checks default values and current-head bincode round-trip for the struct.
3. `rank_candidates()` in `crates/worldwake-ai/src/ranking.rs` already receives `&UtilityProfile` and derives motive inputs from utility fields. Adding one more durable per-agent ranking weight to `UtilityProfile` fits the current decision-data boundary better than creating a parallel ranking-profile type or a one-off constant.
4. The live workspace contains many explicit `UtilityProfile` struct literals, including shared fixtures in `crates/worldwake-core/src/test_utils.rs`, AI helpers in `crates/worldwake-ai/src/ranking.rs`, `crates/worldwake-ai/src/goal_explanation.rs`, and golden harness helpers in `crates/worldwake-ai/tests/golden_harness/mod.rs`, plus additional focused and golden tests across `worldwake-ai`, `worldwake-sim`, and `worldwake-systems`. This ticket must update those sites or compilation will fail.
5. This repository explicitly disallows backward-compatibility shims. Because `UtilityProfile` is a serialized core component, adding a field is an intentional forward-only schema change at current HEAD. The ticket must not prescribe `#[serde(default)]` or any old-save compatibility path for missing fields.
6. No live ranking path reads `activity_awareness_weight` yet. This ticket should remain schema-only: add the field, wire defaults and constructors, and strengthen focused tests around the new field. Competition arithmetic stays in later S35 tickets.
7. Mismatch and correction: the original ticket claimed “old serialized `UtilityProfile` data deserializes cleanly via `#[serde(default)]`.” That is both architecturally wrong for this repo and unnecessary for the intended change. The corrected scope is a clean schema cut with current-head serialization tests only.

## Architecture Check

1. Putting `activity_awareness_weight` directly on `UtilityProfile` is the cleanest long-term shape. Competition sensitivity is a stable per-agent decision trait, just like enterprise, social, courage, and care. A separate ranking-only wrapper would duplicate identity and create another configuration path for the same agent-level concept.
2. This change should stay forward-only. No `#[serde(default)]`, no alias field, and no migration shim for missing historical data.
3. The field should use the same concrete type as the other ranking weights: `Permille`. That preserves deterministic integer arithmetic and keeps later ranking math consistent with the existing utility-weight substrate.

## Verification Layers

1. New field exists on the authoritative component with the intended default -> focused `worldwake-core` unit test in `utility_profile.rs`.
2. Explicit non-default values survive current-head serialization -> focused `worldwake-core` bincode round-trip test in `utility_profile.rs`.
3. Workspace constructor churn is fully resolved -> compile-checked by `cargo test -p worldwake-ai`, then broader `cargo test --workspace`.
4. Additional runtime trace or golden-layer proof is not applicable here because this ticket intentionally does not change ranking behavior yet.

## What to Change

### 1. Extend `UtilityProfile`

In `crates/worldwake-core/src/utility_profile.rs`:

- add `pub activity_awareness_weight: Permille`
- set its default to `Permille::new_unchecked(200)` alongside the other balanced defaults
- extend focused unit coverage so defaults and current-head bincode round-trips assert the new field explicitly

Do not add `#[serde(default)]` or any compatibility helper for missing legacy fields.

### 2. Update explicit `UtilityProfile` construction sites

Search the workspace for `UtilityProfile {` and update every explicit struct literal to initialize `activity_awareness_weight`.

Preferred patterns:

- use `..UtilityProfile::default()` where the test only cares about a few fields
- otherwise set `activity_awareness_weight` explicitly to the intended test value

### 3. Keep behavior unchanged

Do not thread the new field into `rank_candidates()` or any other live ranking logic in this ticket. The only behavior change here should be the availability of a new per-agent parameter in current-head data.

## Files to Touch

- `crates/worldwake-core/src/utility_profile.rs` (modify)
- `crates/worldwake-core/src/test_utils.rs` (modify)
- explicit `UtilityProfile` constructor sites that fail to compile after the schema change (modify)

## Out of Scope

- competition discount logic in ranking
- perception changes or `BelievedActivity`
- new trace structs such as `CompetitionDiscount`
- any backward-compatibility path for older serialized `UtilityProfile` payloads

## Acceptance Criteria

### Tests That Must Pass

1. `UtilityProfile::default().activity_awareness_weight == Permille(200)`.
2. A `UtilityProfile` with a non-default `activity_awareness_weight` round-trips through current-head bincode serialization.
3. Explicit workspace `UtilityProfile` literals compile after the schema change.
4. Existing suites: `cargo test -p worldwake-core`, `cargo test -p worldwake-ai`, `cargo test --workspace`, and `cargo clippy --workspace`.

### Invariants

1. `UtilityProfile` remains the single authoritative per-agent decision-weight component; this ticket does not introduce a parallel ranking profile.
2. The schema change is forward-only: no compatibility shim, alias path, or missing-field fallback is introduced.
3. No live ranking arithmetic changes in this ticket.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/utility_profile.rs` — assert the new default weight and current-head serialization coverage for the added field.
2. `crates/worldwake-core/src/test_utils.rs` and affected focused/golden tests — keep shared fixtures and explicit literals compiling with an intentional value for the new field.

### Commands

1. `cargo test -p worldwake-core utility_profile`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- What actually changed:
  - added `activity_awareness_weight: Permille` to `UtilityProfile`
  - set the default value to `Permille(200)`
  - strengthened `worldwake-core` focused tests to assert the new default and current-head bincode round-trip
  - updated the exhaustive AI helper literals that construct full `UtilityProfile` values
  - updated the CLI scenario RON fixture and assertion to reflect the forward-only schema change
- Deviations from original plan:
  - explicitly rejected the original `#[serde(default)]` compatibility approach and treated this as a clean schema cut
  - no ranking behavior changed in this ticket
- Verification results:
  - `cargo test -p worldwake-core utility_profile` passed
  - `cargo test -p worldwake-ai` passed
  - `cargo test -p worldwake-cli scenario::types::tests::test_scenario_def_deserialize_full` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
