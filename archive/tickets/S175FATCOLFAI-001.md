# S175FATCOLFAI-001: Add `DeprivationKind::Exhaustion` variant

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-core` wounds enum
**Deps**: None

## Problem

`DeprivationKind` (`crates/worldwake-core/src/wounds.rs:30`) has only `Starvation` and `Dehydration`. S175 wires the unimplemented `MetabolismProfile.exhaustion_collapse_ticks` into a concrete deprivation-wound path, which requires a third sibling variant `Exhaustion` so that fatigue critical exposure can create `WoundCause::Deprivation(DeprivationKind::Exhaustion)` wounds. This ticket is the foundation: the variant exists before any consumer (002) wires it.

## Assumption Reassessment (2026-05-28)

1. `DeprivationKind` is at `crates/worldwake-core/src/wounds.rs:30` with exactly two variants (`Starvation`, `Dehydration`) and derives `#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]` (line 29). `Exhaustion` is a unit variant carrying no payload, so every existing derive is satisfied without change.
2. Spec `specs/S175-fatigue-collapse-and-failed-rest-traceability.md` D1 specifies the variant slots in "as a third equal sibling" with no new severity ladder, load function, or death cause. Confirmed against the spec's Non-Goals.
3. Cross-crate blast radius (this is a variant addition to a cross-crate enum): a workspace grep for exhaustive match arms `DeprivationKind::Starvation =>` / `DeprivationKind::Dehydration =>` returned **0** sites. Every `DeprivationKind` use site is either a constructor (`WoundCause::Deprivation(kind)`) or an accessor that takes a `DeprivationKind` parameter (`WoundList::find_deprivation_wound(kind)` at `wounds.rs:103`, `find_deprivation_wound_mut` at `wounds.rs:112`). Adding `Exhaustion` therefore breaks no exhaustive match anywhere in the workspace.
4. Save-format: `DeprivationKind` is serialized inside `WoundList` (a `SimulationState` component). `SAVE_FORMAT_VERSION` is 108 (`crates/worldwake-sim/src/save_load.rs:7`). Adding `Exhaustion` as a trailing variant (bincode index 2) does not shift the existing indices (`Starvation`=0, `Dehydration`=1), so saves at format 108 deserialize identically under the new code. No bump is required for backward-read compatibility, matching the spec's "No new save-format consideration" (Section H point 18). Confirm during implementation that no defensive bump is project policy; the spec asserts none.

## Architecture Check

1. Extending the existing `DeprivationKind` enum (rather than introducing a parallel "fatigue wound" type) keeps all three deprivation causes on one substrate, so the existing wound severity ladder, wound-load contribution, and death path (S17/S81) carry `Exhaustion` with zero new machinery (FND-3, FND-28).
2. No backwards-compatibility aliasing: the variant is net-new; there is no old "fatigue path" to shim around.

## Verification Layers

1. Variant exists and satisfies all enum derives -> focused unit/runtime test (compile + a roundtrip assertion if the module has one).
2. Single-layer ticket: this is a pure type addition in `worldwake-core` with no runtime behavior of its own (consumers land in 002+). No action-trace / event-log mapping applies until the consumer ticket; additional layer mapping is not applicable here.

## What to Change

### 1. Add the `Exhaustion` variant

Add `Exhaustion` to `DeprivationKind` in `crates/worldwake-core/src/wounds.rs` after `Dehydration`. Keep it a unit variant (no payload). No change to derives, to `WoundCause`, or to `find_deprivation_wound` / `find_deprivation_wound_mut` (they already accept any `DeprivationKind`).

### 2. Confirm existing inline tests still cover the enum

If `wounds.rs`'s `#[cfg(test)]` block (boundary at `wounds.rs:141`) has a derive/roundtrip assertion enumerating `DeprivationKind` values, extend it to include `Exhaustion`. Otherwise no test change is required for a unit-variant addition.

## Files to Touch

- `crates/worldwake-core/src/wounds.rs` (modify)

## Out of Scope

- Wiring `exhaustion_collapse_ticks` into `apply_deprivation_consequences` and the death-cause attribution (S175FATCOLFAI-002).
- The `exhaustion_collapse_observed` forensic flag (S175FATCOLFAI-003).
- Scenarios (S175FATCOLFAI-004).
- Any new wound severity ladder, wound-load function, or death cause (spec Non-Goal — the variant reuses the existing substrate).

## Acceptance Criteria

### Tests That Must Pass

1. `DeprivationKind::Exhaustion` constructs and participates in `WoundCause::Deprivation(DeprivationKind::Exhaustion)`.
2. The enum still satisfies its derives (`Copy`, `Eq`, `Ord`, `Hash`, `Serialize`/`Deserialize`) with the new variant.
3. Existing suite: `cargo test -p worldwake-core wounds`

### Invariants

1. Adding the variant breaks no exhaustive match in the workspace (no `match d { DeprivationKind::… }` site exists outside parameterized accessors).
2. `Starvation` and `Dehydration` retain their serialized discriminant indices (0 and 1); `Exhaustion` is the trailing index, preserving save-load backward read.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/wounds.rs` (`#[cfg(test)]`) — extend an existing derive/roundtrip assertion to include `Exhaustion` if one exists; otherwise no new test (unit-variant addition is exercised by 002's consumer tests).

### Commands

1. `cargo test -p worldwake-core wounds`
2. `cargo clippy -p worldwake-core --all-targets -- -D warnings`
3. `cargo build --workspace` (confirms no consumer crate breaks on the new variant)

## Outcome

**Completion date**: 2026-05-28

**What changed**:
- Added the `Exhaustion` unit variant to `DeprivationKind` in `crates/worldwake-core/src/wounds.rs` as the trailing variant (after `Dehydration`). No derive, `WoundCause`, or accessor changes were needed — the unit variant satisfies all existing derives and slots into the parameterized `find_deprivation_wound`/`find_deprivation_wound_mut` accessors unchanged.
- Added three focused tests in the module's `#[cfg(test)]` block: `exhaustion_deprivation_cause_roundtrips_through_bincode` (bincode roundtrip of `WoundCause::Deprivation(Exhaustion)`), `deprivation_kind_variants_have_stable_serialized_indices` (asserts Starvation=0, Dehydration=1, Exhaustion=2 — proving save-load backward read is preserved), and `find_deprivation_wound_distinguishes_exhaustion` (accessor selects the Exhaustion wound and rejects a non-matching kind).

**Deviations from plan**: None. The reassessment was accurate — no exhaustive match sites exist on `DeprivationKind` anywhere in the workspace (grep confirmed 0), so the variant addition broke nothing.

**Verification**:
- `cargo test -p worldwake-core wounds` — 18 passed, 0 failed.
- `cargo clippy -p worldwake-core --all-targets -- -D warnings` — clean.
- `cargo build --workspace` — all crates compile, no consumer break.
