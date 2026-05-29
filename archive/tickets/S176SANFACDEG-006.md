# S176SANFACDEG-006: Survival forensics — DegradedSelfCareOpportunity

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `SurvivalForensicExtractor` + `CriticalWindowFrame` (ai); derived forensic state only (no authoritative state, no save-format bump)
**Deps**: S176SANFACDEG-003 (degradation preconditions), S176SANFACDEG-004 (cleaning/fallback outcomes)

## Problem

"Why did this agent relieve in the wild / wash poorly / not wash at all?" is not answerable from typed records (S176 D8). This ticket adds a `DegradedSelfCareOpportunity` forensic record, mirroring the existing `FailedRestOpportunity`, so blocked/degraded self-care leaves traceable evidence in the critical-window forensics.

## Assumption Reassessment (2026-05-29)

1. `SurvivalForensicExtractor` is at `crates/worldwake-ai/src/survival_forensics.rs:215`. The precedent `FailedRestOpportunity` is at `:54-67` (`{ tick, place, kind: FailedRestKind, was_rough }`); `CriticalWindowFrame` is at `:39-51` and already carries `failed_rest_opportunities: Vec<FailedRestOpportunity>` with `#[serde(default)]` (`:50`). The new record + frame field follow this shape exactly.
2. `CriticalWindowFrame` is **not** part of serialized authoritative `SimulationState`: it is absent from `crates/worldwake-sim/src/save_load.rs` and `crates/worldwake-core/src/delta.rs` (forensic/decision-trace state, FND-27). Therefore **no `SAVE_FORMAT_VERSION` bump** is required; the `#[serde(default)]` on the new frame field is for decision-trace serialization tolerance only.
3. Shared boundary under audit: the forensic derived view over event/trace log — never authoritative (FND-27). The record is written by the extractor, read by observer/CLI/tests.
4. The degradation causes (`BasinTooDirty`, `BasinDry`, `LatrineFull`) and outcomes (`WildernessRelief`, `Cleaned`, `Queued`, `DidNothing`) originate from the gates (S176SANFACDEG-003) and cleaning/fallback paths (S176SANFACDEG-004/005); the extractor derives them from the observed decision frame, not from a new authoritative signal.

## Architecture Check

1. Mirrors the landed `FailedRestOpportunity` precedent precisely — same extractor, same frame-field pattern, same derived-view classification (FND-27). No new authoritative state.
2. FND-29A: the record reconstructs the knowledge/causal path for degraded self-care from the active critical window without inventing a parallel truth source.

## Verification Layers

1. Record population on degraded/blocked self-care → focused unit on `SurvivalForensicExtractor` (frame carries the expected cause/outcome).
2. Derived-view non-authority → focused unit confirming the record is absent from authoritative world state / save-format.

## What to Change

### 1. DegradedSelfCareOpportunity type

Add `DegradedSelfCareOpportunity { tick, facility, cause: DegradedSelfCareCause, outcome: DegradedSelfCareOutcome }` plus the `cause`/`outcome` enums to `survival_forensics.rs`.

### 2. Frame field + population

Add `degraded_self_care_opportunities: Vec<DegradedSelfCareOpportunity>` (`#[serde(default)]`) to `CriticalWindowFrame`; populate it in the extractor when a self-care opportunity is blocked/degraded in the active window.

## Files to Touch

- `crates/worldwake-ai/src/survival_forensics.rs` (modify)

## Out of Scope

- The gates and actions that produce the degraded outcomes — S176SANFACDEG-003/004/005.
- Observer rendering of the new records — surfaced automatically by existing forensic/trace rendering; no observer code change required here.

## Acceptance Criteria

### Tests That Must Pass

1. A blocked wash (basin too dirty) in the active window records a `DegradedSelfCareOpportunity { cause: BasinTooDirty, … }`.
2. A blocked toilet that falls back to wilderness relief records `{ cause: LatrineFull, outcome: WildernessRelief }`.
3. Existing suite: `cargo test -p worldwake-ai survival_forensics`

### Invariants

1. `DegradedSelfCareOpportunity` is derived forensic state, never authoritative (FND-27).
2. No `SAVE_FORMAT_VERSION` bump (frame is not serialized authoritative state).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/survival_forensics.rs` — new: degraded-self-care record population for each cause/outcome combination exercised.

### Commands

1. `cargo test -p worldwake-ai survival_forensics`
2. `cargo test -p worldwake-ai`
3. `scripts/verify.sh`

## Outcome

**Completion date**: 2026-05-29

**What changed**:
- Added `DegradedSelfCareOpportunity { tick, facility, cause, outcome }` + `DegradedSelfCareCause {BasinTooDirty, BasinDry, LatrineFull}` + `DegradedSelfCareOutcome {WildernessRelief, Cleaned, Queued, DidNothing}` to `survival_forensics.rs`, exported from `lib.rs`.
- Added `degraded_self_care_opportunities: Vec<…>` (`#[serde(default)]`) to `CriticalWindowFrame`, populated in `build_frame`, and included in `frame_change_detected`.
- Derivation reads the tick's action-trace events: committed `clean_wash_basin` (Dirtiness window → BasinTooDirty/Cleaned), committed `empty_latrine` (Bladder → LatrineFull/Cleaned), committed `relieve_wilderness` (Bladder + latrine present → LatrineFull/WildernessRelief), and `wash`/`toilet` `StartFailed` whose reason (`{precondition:?}` from `action_validation.rs`) names `TargetWashBasinNotTooDirty` / `TargetHasWashBasinClean` / `PlaceLatrineNotFull` (→ DidNothing).

**Deviations / decisions**:
- **`facility` = the actor's place.** `ActionTraceEvent` carries no targets, and `build_frame` has no world access, so the record's `facility` is `local_state.place` (the locus of the degraded self-care). For a latrine this *is* the facility (LatrineFullness is on the place); for a basin it is the basin's place rather than the basin entity — sufficient to answer "where/why self-care degraded."
- **Added `latrine_present: bool` to `LocalSurvivalStateSummary`** (mirrors `wash_basin_present`) so a committed `relieve_wilderness` is only classified as degraded *latrine* self-care when a latrine is actually present — otherwise it is ordinary outdoor relief and is not recorded. Updated `capture` + the observer's `place_survival_state_summary` and all test/fixture construction sites.
- The `StartFailed` path is derivation-only (constructing a `StartFailed` event in a unit test needs a full `ResolvedRequestTrace`); the focused unit test covers the committed-action signals (the path the goldens exercise). No save-format bump (forensic frame is not authoritative state, FND-27).

**Verification**: `cargo test -p worldwake-ai survival_forensics` (incl. the new record test), full `cargo test --workspace` (no failures), and `cargo clippy --workspace --all-targets -- -D warnings` clean.
