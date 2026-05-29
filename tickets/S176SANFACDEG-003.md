# S176SANFACDEG-003: Wash/toilet degradation gates

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `Precondition` enum (sim, 2 variants + evaluation), `apply_wash` effectiveness scaling and `wash_preconditions` (systems), `toilet` inline precondition (systems)
**Deps**: S176SANFACDEG-001 (`WashBasinState.max_effective_dirtiness`)

## Problem

A filthy basin washes as well as a clean one, and a full latrine still relieves perfectly (S176 D2/D3). This ticket makes wash relief scale with basin dirtiness and fail above the authored threshold, and blocks the Toilet action when the latrine is full — forcing the lawful fallback/recovery branches.

## Assumption Reassessment (2026-05-29)

1. `Precondition` is at `crates/worldwake-sim/src/action_semantics.rs:47`; the existing `TargetHasWashBasinClean { target_index: u8, min: u16 }` is the shape template. `Precondition` is matched exhaustively across **5** files: `affordance_query.rs:336/477`, `action_validation.rs:110`, `action_semantics.rs`, `needs_actions.rs`, and `ranking.rs` — the spec named only the first two; arms (or confirmed `_ =>` catch-all semantics) must be checked in all five.
2. `wash_preconditions()` exists at `needs_actions.rs:277` and is wired at the `wash` registration (`needs_actions.rs:102`); `wash` targets `[TargetSpec::EntityAtActorPlace { kind: Facility }]` (target 0 = basin). There is **no `toilet_preconditions` function**: `toilet` is registered inline at `needs_actions.rs:88-97` with `preconditions: vec![Precondition::ActorAlive]` (`:92`), `targets: [TargetSpec::ActorPlace]` (target 0 = actor's latrine place, `:174`), and `actor_constraints: [ActorAlive, ActorAtPlaceTag(PlaceTag::Latrine)]` (`:161-163`). The latrine gate is therefore a target-indexed precondition on target 0 added to the inline vec at `:92` — the spec's "in `toilet_preconditions`" framing was corrected at reassessment.
3. `apply_wash` is at `needs_actions.rs:1208`; it computes `agent_dirtiness_delta` at `:1229-1233` and applies `needs.dirtiness.saturating_sub(agent_dirtiness_delta)` at `:1254`. Scaling multiplies the delta by `effective_fraction` in `Permille`.
4. Adjacent contradictions (classified as required consequences of D3, FND-28): existing inline tests assert the old "toilet always succeeds + overflow" contract and must be rewritten — `toilet_already_over_threshold_emits_waste_created_each_tick:2462` (toilet now fails to start at/above threshold, so per-tick overflow via toilet no longer occurs) and `toilet_latrine_fullness_saturates_at_max:2509` (fill can no longer reach saturation through repeated toilet use). `toilet_under_threshold_does_not_emit_waste_created:2433` and `toilet_reduces_bladder_and_creates_waste:2287` remain valid below threshold. Wash tests using clean basins (`wash_full_success_consumes_basin_water_and_clears_dirtiness:2707`) still pass because `effective_fraction == 1` at `dirtiness_level == 0`.
5. Live planner surface: `GoalKind::Wash` (`goal.rs:73`) and `GoalKind::Relieve` (`goal.rs:72`) are unchanged — this ticket gates the existing wash/toilet ops via preconditions; no new `GoalKind`. The cleaning prerequisite that unblocks a rejected gate is owned by S176SANFACDEG-005.

## Architecture Check

1. Effectiveness is a pure function of the concrete `dirtiness_level` and the authored `max_effective_dirtiness`; the gate reuses the existing target-indexed `Precondition` machinery (mirrors `TargetHasWashBasinClean`) rather than a bespoke check (FND-3, FND-8).
2. FND-28: the toilet "always succeeds" path is replaced by the fullness gate, not aliased; affected tests are rewritten in-scope.

## Verification Layers

1. Wash effectiveness scaling → focused unit on `apply_wash` (relief halves at half-dirtiness).
2. Precondition rejection (basin too dirty / latrine full) → affordance suppression via `affordance_query` focused coverage + action-trace start failure.
3. Authoritative mutation (no relief / no overflow when blocked) → event-log delta / authoritative world state.

## What to Change

### 1. Precondition variants + evaluation

Add `Precondition::{TargetWashBasinNotTooDirty { target_index: u8 }, PlaceLatrineNotFull { target_index: u8 }}`. Add evaluation arms in `action_validation.rs` and `affordance_query.rs` (read target facility/place `WashBasinState`/`LatrineFullness`); confirm arms or catch-all semantics in `action_semantics.rs`, `needs_actions.rs`, `ranking.rs`.

### 2. Wash effectiveness scaling + gate

In `apply_wash`, multiply `agent_dirtiness_delta` by `effective_fraction = (max_effective_dirtiness - dirtiness_level) / max_effective_dirtiness` clamped to `[0,1]`. Add `TargetWashBasinNotTooDirty { target_index: 0 }` to `wash_preconditions()`.

### 3. Latrine fullness gate

Add `PlaceLatrineNotFull { target_index: 0 }` to the `toilet` inline precondition vec (`needs_actions.rs:92`).

### 4. Test rewrites

Rewrite the inline tests named in Assumption Reassessment item 4 to assert the new gated behavior.

## Files to Touch

- `crates/worldwake-sim/src/action_semantics.rs` (modify — variants)
- `crates/worldwake-sim/src/action_validation.rs` (modify — eval arms)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — eval arms)
- `crates/worldwake-systems/src/needs_actions.rs` (modify — `apply_wash`, `wash_preconditions`, toilet inline vec, test rewrites)
- `crates/worldwake-ai/src/ranking.rs` (modify — confirm exhaustive `Precondition` arm or catch-all)

## Out of Scope

- The `clean_wash_basin` / `empty_latrine` actions that recover the blocked state — S176SANFACDEG-004.
- Planner insertion of the cleaning prerequisite on rejection — S176SANFACDEG-005.
- Forensic recording of the degraded outcome — S176SANFACDEG-006.

## Acceptance Criteria

### Tests That Must Pass

1. Wash at a half-dirty basin yields ~half the dirtiness relief; wash at/above threshold fails the precondition (no episode).
2. Toilet at/above `critical_threshold` fails to start; `relieve_wilderness` remains available.
3. Rewritten `toilet_already_over_threshold_*` and `toilet_latrine_fullness_saturates_at_max` assert the gated behavior.
4. Existing suite: `cargo test -p worldwake-systems && cargo test -p worldwake-sim`

### Invariants

1. Wash effectiveness derives only from `dirtiness_level` / `max_effective_dirtiness` (no quality score).
2. A blocked wash/toilet produces no relief and no commit; the lawful fallback (`relieve_wilderness`) stays reachable (FND-21).
3. No new `GoalKind`; existing `Wash`/`Relieve` ops are gated, not replaced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` — new: wash effectiveness scaling, wash too-dirty rejection, toilet-full rejection; modified: the two old-contract toilet tests.
2. `crates/worldwake-sim/src/affordance_query.rs` — new: affordance suppression when basin too dirty / latrine full.

### Commands

1. `cargo test -p worldwake-systems needs_actions`
2. `cargo test -p worldwake-sim affordance_query && cargo test -p worldwake-ai`
3. `scripts/verify.sh`
