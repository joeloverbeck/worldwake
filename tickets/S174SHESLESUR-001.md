# S174SHESLESUR-001: Foundation types — RestCapacity, RestOccupancy, SleepFailureCause, ActionTraceDetail::SleepInterrupted

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new ECS components on `EntityKind::Place`; existing cross-crate enum (`WakeReason`) variant payload widened; new variant on `ActionTraceDetail`; `SAVE_FORMAT_VERSION` bumped from 107 to 108
**Deps**: `specs/S174-shelter-sleep-surfaces-safe-rest.md` (D1, D3, D6, D7), `archive/specs/S128-sleep-episode-place-quality.md`, `archive/specs/S173-self-care-interruption-occupancy.md`

## Problem

Sleep currently has no rest-site occupancy carrier — multiple agents can intend to sleep at the same shelter without contention. `WakeReason::LocalDisturbance` is a bare unit variant with no structured cause, so wake events cannot answer "why did this sleep fail?" with typed evidence. `ActionTraceDetail` has no sleep-specific interruption variant, so action-trace consumers cannot distinguish sleep aborts from other self-care interruptions with structured cause information. These four deliverables (D1, D3, D6, D7) land together as a single atomic foundation because the `WakeReason` payload widening breaks serialization (cannot land independently of a save-format bump) and the new `SleepFailureCause` enum is referenced by both `WakeReason` and the new `ActionTraceDetail::SleepInterrupted` variant.

## Assumption Reassessment (2026-05-26)

1. Verified current code state: `WakeReason` enum at `crates/worldwake-core/src/decision_event_payload.rs:97` has variants `IntendedDuration`, `TargetRecovery`, `ProjectedNeedBreach { need, projected_breach_tick }`, `ScheduledCommitment`, `LocalDisturbance` (bare unit). `WakeCondition` enum at `crates/worldwake-core/src/sleep_episode.rs:30` mirrors but with `LocalDisturbance` also bare. `ActionTraceDetail` at `crates/worldwake-sim/src/action_trace.rs:32-70` derives `Clone, Debug, Eq, PartialEq` — all proposed payload field types (`EntityId`, `SleepFailureCause`, `Permille`, `bool`) satisfy these. `SAVE_FORMAT_VERSION` is currently `107` at `crates/worldwake-sim/src/save_load.rs:7`.
2. Spec assumption verified against S174 D1, D3, D6, D7 and `docs/spec-drafting-rules.md` (Component Registration, Belief-View Accessor Source-Class Rule). The asymmetric WakeReason vs. WakeCondition design (only WakeReason carries structured cause) was approved during reassessment Q1=(b) — `WakeCondition::LocalDisturbance` stays bare, with the rationale documented at S174 D3.
3. Shared abstraction boundary under audit: serialized event-payload shape (`WakeReason`) + ECS component registration (`RestCapacity`, `RestOccupancy` on `EntityKind::Place`) + action-trace sink shape (`ActionTraceDetail`). All three changes touch cross-crate serialized formats, so the SAVE_FORMAT_VERSION bump is attributed to this ticket (the first ticket carrying an incompatible serialized-format change).
4. The only `WakeReason::LocalDisturbance` construction site in the workspace is `abort_sleep_episode` at `crates/worldwake-systems/src/needs_actions.rs:679`. This site is updated as part of this ticket (threading `SleepFailureCause` into the construction). Sibling spec S175 will read the structured cause from `WakeReason::LocalDisturbance { cause }` but does not need to construct it.
5. Belief-view accessor read surface is unchanged by this ticket — the new components are introduced but their belief-view accessors land in ticket 003. Per the placeholder-replace pattern, the components are usable by ticket 003 immediately (read paths just don't exist yet); no transient dead-code issue because component absence is the well-defined "no rest capacity here" state.
6. Cross-crate enum variant additions: `ActionTraceDetail::SleepInterrupted` is purely additive (no existing exhaustive match on `ActionTraceDetail` — `tick_step.rs::abort_trace_detail_for_instance` uses conditional returns, not exhaustive match). The new variant compiles without forcing match-arm updates anywhere.
7. Existing inline tests exercising the affected types: `crates/worldwake-core/src/sleep_episode.rs:134` (asserts bare `WakeCondition::LocalDisturbance`) — no update needed since `WakeCondition::LocalDisturbance` stays bare per Q1=(b). `crates/worldwake-ai/tests/scenarios/sleep_episode.rs` golden tests at lines 170, 229, 284, 309, 361 — these assert `SleepEpisode` lifecycle and may need updates only if they construct `WakeReason::LocalDisturbance` directly (verify: the interrupted-sleep golden at line 284 likely asserts the wake reason).

## Architecture Check

1. The four deliverables are bundled because `WakeReason` payload widening cannot land independently of a `SAVE_FORMAT_VERSION` bump, and bundling the new component types + new `ActionTraceDetail` variant with the same bump avoids a separate cascade. Per `spec-to-tickets/SKILL.md` Step 3 FND-28-driven combining rule: splitting the bundle into "components-only" + "WakeReason restructure" + "ActionTraceDetail variant" would leave intermediate states where the workspace compiles but the live authority path is half-migrated.
2. `RestOccupancy` is a separate component from `SelfCareOccupancy` (rather than extending S173's single-occupant struct) because rest capacity is multi-occupant per FOUNDATIONS FND-8 — a shelter with 3 bedrolls hosts 3 simultaneous sleepers. Promoting `SelfCareOccupancy` to multi-occupant would break S173's contract for Wash/Latrine (which are strictly single-use). The dedicated `RestOccupancy` carrier follows FND-28: a new mechanism rather than a backwards-compatible widening of an existing one.
3. `WakeReason::LocalDisturbance` payload widening (asymmetric with `WakeCondition::LocalDisturbance`) aligns with FND-28: one structured cause surface, not two. The `WakeCondition` enum stays bare per spec Q1=(b) because `WakeCondition::LocalDisturbance` is currently a soft trigger predicate that does not currently produce any `WakeReason` (per `sleep_wake_reason()` at `needs_actions.rs:610-634` returning `None` for the `LocalDisturbance` arm); manufacturing a synthetic cause at the 4 `sleep_synthesis.rs` push sites would add ceremony without semantic value.

## Verification Layers

1. Component lifecycle correctness (RestCapacity scenario-authored, RestOccupancy runtime-managed) -> focused unit test in `crates/worldwake-core/src/rest_site.rs` + component_schema registration test
2. WakeReason payload serialization round-trip (existing pre-bump saves rejected; new format saves round-trip) -> `crates/worldwake-sim/src/save_load.rs` integration test
3. ActionTraceDetail::SleepInterrupted construction + Eq derive -> focused unit test in `crates/worldwake-sim/src/action_trace.rs` tests
4. `SAVE_FORMAT_VERSION` bump is the only format-breaking change in this ticket (multiple deliverables bundled to share one bump) -> documented in Files to Touch and the Merge note below
5. No new authoritative event variant introduced (FND-28: enrich existing `EventTag::SleepEpisodeEnded` via payload widening, do not parallel it) -> spec text D6 + this ticket's What to Change confirm

## What to Change

### 1. New core module: `crates/worldwake-core/src/rest_site.rs`

Define the two new components:

```rust
//! Rest-site capacity and occupancy state for sleep affordances.

use crate::{Component, EntityId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

/// Maximum simultaneous sleepers a Place can host as a "known rest site."
/// A Place without `RestCapacity` is not a known rest site — only Rough Sleep
/// is available there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestCapacity(pub NonZeroU32);

impl Component for RestCapacity {}

/// Authoritative occupancy state. Multi-occupant: a shelter with capacity 2
/// can carry two `EntityId`s simultaneously. Empty when no agent is sleeping
/// at this rest site.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestOccupancy {
    pub occupants: BTreeSet<EntityId>,
}

impl Component for RestOccupancy {}
```

Add `pub mod rest_site;` and re-export `RestCapacity`, `RestOccupancy` in `crates/worldwake-core/src/lib.rs`.

### 2. Component schema registration in `crates/worldwake-core/src/component_schema.rs`

Following the `SleepQualityProfile` precedent (registered on `EntityKind::Place`), add entries for both new components. The filter is `|kind| kind == EntityKind::Place`. `RestCapacity` is scenario-authored (optional, present only when explicitly defined by `PlaceDef.rest_capacity`); `RestOccupancy` is runtime-managed (absent by default, written/updated by the sleep action handler).

Import `RestCapacity` and `RestOccupancy` in any macro expansion site (`delta.rs`, `world.rs`, `component_tables.rs`) per `tickets/README.md` check #13.

### 3. WakeReason payload widening at `crates/worldwake-core/src/decision_event_payload.rs:97-107`

Add the new `SleepFailureCause` enum:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum SleepFailureCause {
    /// Hostile actor entered same place mid-sleep.
    HostileProximity,
    /// Another actor took the rest slot (only possible in degenerate races; the
    /// occupancy precondition normally prevents this).
    RestSiteContended,
    /// Place dirtiness or other place-state change invalidated the surface.
    SurfaceInvalidated,
    /// Actor was wounded/incapacitated during sleep through ordinary processes.
    ActorIncapacitated,
    /// Generic disturbance — preserved as a fallback for sources that do not
    /// yet classify their cause. New disturbance sources should map to a
    /// specific cause; `Generic` is a transitional bucket.
    Generic,
}
```

Restructure `WakeReason::LocalDisturbance` from a unit variant to a struct variant:

```rust
pub enum WakeReason {
    IntendedDuration,
    TargetRecovery,
    ProjectedNeedBreach {
        need: HomeostaticNeedId,
        projected_breach_tick: Tick,
    },
    ScheduledCommitment,
    LocalDisturbance { cause: SleepFailureCause },   // was: LocalDisturbance,
}
```

`WakeCondition::LocalDisturbance` at `sleep_episode.rs:30-36` stays bare (asymmetric design per Q1=(b)). No change required to `WakeCondition`.

### 4. Update the single `WakeReason::LocalDisturbance` construction site

`crates/worldwake-systems/src/needs_actions.rs:679` in `abort_sleep_episode` currently constructs `WakeReason::LocalDisturbance` as a bare unit. Update to thread a `SleepFailureCause` value through. The abort caller in this ticket supplies `SleepFailureCause::Generic` as the transitional default; ticket 004 (sleep handler RestOccupancy lifecycle) and ticket 009 (HostileProximity scenario) refine this per the abort context.

### 5. ActionTraceDetail::SleepInterrupted variant at `crates/worldwake-sim/src/action_trace.rs`

Add the new variant to the `ActionTraceDetail` enum (current span lines 32-70):

```rust
SleepInterrupted {
    place: EntityId,
    cause: SleepFailureCause,
    accumulated_recovery: Permille,
    was_rough_sleep: bool,
},
```

Import `SleepFailureCause` from `worldwake_core::decision_event_payload`. The variant is purely additive — no exhaustive `match ActionTraceDetail` sites exist in the workspace (only conditional-return construction at `tick_step.rs::abort_trace_detail_for_instance`), so this addition compiles without forcing arm updates. Population of this variant lands in ticket 006.

### 6. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs:7`, bump `SAVE_FORMAT_VERSION` from `107` to `108`. The bump is attributed to this ticket because the `WakeReason` payload widening is the first incompatible serialized-format change in the S174 wave. Subsequent S174 tickets that add fields (e.g., ticket 002's `rough_sleep_recovery_floor`) ride this bump and do not need their own.

## Files to Touch

- `crates/worldwake-core/src/rest_site.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — add `pub mod rest_site;` and re-exports)
- `crates/worldwake-core/src/component_schema.rs` (modify — register `RestCapacity` and `RestOccupancy`)
- `crates/worldwake-core/src/delta.rs` (modify — import new types for macro expansion)
- `crates/worldwake-core/src/world.rs` (modify — import new types for macro expansion)
- `crates/worldwake-core/src/component_tables.rs` (modify — import new types for macro expansion; verify file exists at this path or locate via `find crates -name component_tables.rs`)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify — add `SleepFailureCause` enum; restructure `WakeReason::LocalDisturbance`)
- `crates/worldwake-sim/src/action_trace.rs` (modify — add `ActionTraceDetail::SleepInterrupted` variant)
- `crates/worldwake-systems/src/needs_actions.rs` (modify — update `WakeReason::LocalDisturbance` construction at line 679 to supply `SleepFailureCause::Generic`)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump `SAVE_FORMAT_VERSION` 107 → 108)
- Likely: existing inline test at `crates/worldwake-core/src/sleep_episode.rs:134` (verify whether it constructs `WakeReason::LocalDisturbance` directly; if so, update with `cause: SleepFailureCause::Generic`)
- Likely: golden test at `crates/worldwake-ai/tests/scenarios/sleep_episode.rs:284` (`interrupted_sleep_records_partial_recovery`) — verify whether it asserts `WakeReason::LocalDisturbance`; if so, update assertion to include `cause` field

## Out of Scope

- No `WakeCondition::LocalDisturbance` restructuring (stays bare per Q1=(b))
- No belief-view accessors for `RestCapacity`/`RestOccupancy` (ticket 003)
- No sleep action handler changes to write/release `RestOccupancy` (ticket 004)
- No goal schema changes (ticket 005)
- No forensic record additions (ticket 006)
- No population of `ActionTraceDetail::SleepInterrupted` at the abort path — variant is added here, populated in ticket 006
- No predator/exposure/disease model — S174 explicitly defers these per its Non-Goals
- No reuse of `SelfCareOccupancy::Sleep` for rest sites — that enum variant remains but does not carry rest-site occupancy; ticket 006 will redirect the existing `SelfCareInterrupted { kind: Sleep, ... }` trace path to the new `SleepInterrupted` variant

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit test: `RestCapacity` and `RestOccupancy` registration on `EntityKind::Place` round-trips through component schema
2. New focused unit test: `SleepFailureCause` enum variants are constructible with all 5 variants and derive `Copy, Eq, Ord, Hash, Serialize, Deserialize`
3. New focused unit test: `WakeReason::LocalDisturbance { cause: SleepFailureCause::Generic }` constructs and pattern-matches correctly
4. New focused unit test: `ActionTraceDetail::SleepInterrupted { place, cause, accumulated_recovery, was_rough_sleep }` constructs and equality-compares correctly
5. Existing suite: `cargo test -p worldwake-core` passes (component registration, sleep_episode types)
6. Existing suite: `cargo test -p worldwake-sim` passes (action_trace types, save_load round-trip)
7. Existing suite: `cargo test -p worldwake-systems` passes (needs_actions sleep abort path)
8. Existing suite: `cargo test -p worldwake-ai` passes (sleep_episode goldens after WakeReason payload update)

### Invariants

1. `RestCapacity` is only registered on `EntityKind::Place` (not Agent, not Facility)
2. `RestOccupancy` is only registered on `EntityKind::Place`; absent by default; cleared back to empty when last occupant releases
3. `WakeReason::LocalDisturbance` cannot be constructed without an explicit `cause` value (the bare unit variant is removed)
4. `WakeCondition::LocalDisturbance` remains a bare unit variant (asymmetric design per Q1=(b))
5. `SAVE_FORMAT_VERSION` is exactly `108` after this ticket; no subsequent S174 ticket bumps it again

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/rest_site.rs` (new tests) — focused unit coverage for `RestCapacity`/`RestOccupancy` component derives and registration round-trips
2. `crates/worldwake-core/src/decision_event_payload.rs` (extend existing tests) — `WakeReason::LocalDisturbance { cause }` constructor + pattern-match coverage; `SleepFailureCause` variant enumeration
3. `crates/worldwake-sim/src/action_trace.rs` (extend existing tests) — `ActionTraceDetail::SleepInterrupted` variant construction + equality
4. `crates/worldwake-sim/src/save_load.rs` (extend existing tests) — confirm `SAVE_FORMAT_VERSION == 108`; round-trip test for serialized state with new `WakeReason::LocalDisturbance { cause }` payload
5. `crates/worldwake-ai/tests/scenarios/sleep_episode.rs:284` (likely modify) — if the existing `interrupted_sleep_records_partial_recovery` golden asserts `WakeReason::LocalDisturbance`, update the assertion to match the new struct-variant form

### Commands

1. `cargo test -p worldwake-core rest_site` (new tests)
2. `cargo test -p worldwake-core decision_event_payload` (WakeReason coverage)
3. `cargo test -p worldwake-sim action_trace save_load` (ActionTraceDetail + SAVE_FORMAT_VERSION)
4. `cargo test --workspace` (full suite must pass; primary risk is sleep_episode golden assertion)
5. `./scripts/verify.sh` (final pre-PR gate)

Merge note: Ticket 001 bumps `SAVE_FORMAT_VERSION` 107→108; sibling S174 tickets (002 adds `rough_sleep_recovery_floor` to MetabolismProfile, 006 adds `FailedRestOpportunity` to `CriticalWindowFrame`) deliberately ride this bump via `#[serde(default)]` on the new fields rather than bumping again.
