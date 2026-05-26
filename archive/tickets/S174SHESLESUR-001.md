# S174SHESLESUR-001: Foundation types — RestCapacity, RestOccupancy, SleepFailureCause, ActionTraceDetail::SleepInterrupted

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new ECS components on `EntityKind::Place`; existing cross-crate enum (`WakeReason`) variant payload widened; new variant on `ActionTraceDetail`; `SAVE_FORMAT_VERSION` bumped from 107 to 108
**Deps**: `specs/S174-shelter-sleep-surfaces-safe-rest.md` (D1, D3, D6, D7), `archive/specs/S128-sleep-episode-place-quality.md`, `archive/specs/S173-self-care-interruption-occupancy.md`

## Problem

Sleep currently has no rest-site occupancy carrier — multiple agents can intend to sleep at the same shelter without contention. `WakeReason::LocalDisturbance` is a bare unit variant with no structured cause, so wake events cannot answer "why did this sleep fail?" with typed evidence. `ActionTraceDetail` has no sleep-specific interruption variant, so action-trace consumers cannot distinguish sleep aborts from other self-care interruptions with structured cause information. These four deliverables (D1, D3, D6, D7) land together as a single atomic foundation because the `WakeReason` payload widening breaks serialization (cannot land independently of a save-format bump) and the new `SleepFailureCause` enum is referenced by both `WakeReason` and the new `ActionTraceDetail::SleepInterrupted` variant.

## Assumption Reassessment (2026-05-26)

1. At pre-implementation reassessment, `WakeReason` enum at `crates/worldwake-core/src/decision_event_payload.rs` had variants `IntendedDuration`, `TargetRecovery`, `ProjectedNeedBreach { need, projected_breach_tick }`, `ScheduledCommitment`, and bare `LocalDisturbance`; this ticket replaced that final variant with `LocalDisturbance { cause: SleepFailureCause }`. `WakeCondition` enum at `crates/worldwake-core/src/sleep_episode.rs` still has bare `LocalDisturbance` by design. `ActionTraceDetail` at `crates/worldwake-sim/src/action_trace.rs` derives `Clone, Debug, Eq, PartialEq`; all landed payload field types (`EntityId`, `SleepFailureCause`, `Permille`, `bool`) satisfy these bounds. `SAVE_FORMAT_VERSION` was `107` before this ticket and is now `108`.
2. Spec assumption verified against S174 D1, D3, D6, D7 and `docs/spec-drafting-rules.md` (Component Registration, Belief-View Accessor Source-Class Rule). The asymmetric WakeReason vs. WakeCondition design (only WakeReason carries structured cause) was approved during reassessment Q1=(b) — `WakeCondition::LocalDisturbance` stays bare, with the rationale documented at S174 D3.
3. Shared abstraction boundary under audit: serialized event-payload shape (`WakeReason`) + ECS component registration (`RestCapacity`, `RestOccupancy` on `EntityKind::Place`) + action-trace sink shape (`ActionTraceDetail`). All three changes touch cross-crate serialized formats, so the SAVE_FORMAT_VERSION bump is attributed to this ticket (the first ticket carrying an incompatible serialized-format change).
4. The only production `WakeReason::LocalDisturbance` construction site in the workspace remains `abort_sleep_episode` at `crates/worldwake-systems/src/needs_actions.rs`; this ticket updated it to construct `WakeReason::LocalDisturbance { cause: SleepFailureCause::Generic }`. Sibling spec S175 will read the structured cause from `WakeReason::LocalDisturbance { cause }` but does not need to construct it.
5. Belief-view accessor read surface is unchanged by this ticket — the new components are introduced but their belief-view accessors land in ticket 003. Per the placeholder-replace pattern, the components are usable by ticket 003 immediately (read paths just don't exist yet); no transient dead-code issue because component absence is the well-defined "no rest capacity here" state.
6. Cross-crate enum variant addition: `ActionTraceDetail::SleepInterrupted` landed as an additive variant. The existing sleep-abort trace routing still emits `SelfCareInterrupted { kind: Sleep, ... }`; ticket 006 owns switching the abort helper to populate `SleepInterrupted`.
7. Existing inline tests exercising the affected types remained compatible: `crates/worldwake-core/src/sleep_episode.rs` still asserts bare `WakeCondition::LocalDisturbance`; `crates/worldwake-ai/tests/scenarios/sleep_episode.rs` sleep goldens passed without assertion edits because they do not construct the bare `WakeReason::LocalDisturbance` variant directly.

## Architecture Check

1. The four deliverables are bundled because `WakeReason` payload widening cannot land independently of a `SAVE_FORMAT_VERSION` bump, and bundling the new component types + new `ActionTraceDetail` variant with the same bump avoids a separate cascade. Per `spec-to-tickets/SKILL.md` Step 3 FND-28-driven combining rule: splitting the bundle into "components-only" + "WakeReason restructure" + "ActionTraceDetail variant" would leave intermediate states where the workspace compiles but the live authority path is half-migrated.
2. `RestOccupancy` is a separate component from `SelfCareOccupancy` (rather than extending S173's single-occupant struct) because rest capacity is multi-occupant per FOUNDATIONS FND-8 — a shelter with 3 bedrolls hosts 3 simultaneous sleepers. Promoting `SelfCareOccupancy` to multi-occupant would break S173's contract for Wash/Latrine (which are strictly single-use). The dedicated `RestOccupancy` carrier follows FND-28: a new mechanism rather than a backwards-compatible widening of an existing one.
3. `WakeReason::LocalDisturbance` payload widening (asymmetric with `WakeCondition::LocalDisturbance`) aligns with FND-28: one structured cause surface, not two. The `WakeCondition` enum stays bare per spec Q1=(b) because `WakeCondition::LocalDisturbance` is currently a soft trigger predicate that does not currently produce any `WakeReason` (per `sleep_wake_reason()` at `needs_actions.rs:610-634` returning `None` for the `LocalDisturbance` arm); manufacturing a synthetic cause at the 4 `sleep_synthesis.rs` push sites would add ceremony without semantic value.

## Verified Layers

1. Component lifecycle correctness (RestCapacity scenario-authored, RestOccupancy runtime-managed) -> focused unit test in `crates/worldwake-core/src/rest_site.rs` + component_schema registration test
2. WakeReason payload serialization round-trip (pre-bump saves rejected; current format saves round-trip) -> `crates/worldwake-sim/src/save_load.rs` integration test
3. ActionTraceDetail::SleepInterrupted construction + Eq derive -> focused unit test in `crates/worldwake-sim/src/action_trace.rs` tests
4. `SAVE_FORMAT_VERSION` bump is the only format-breaking change in this ticket (multiple deliverables bundled to share one bump) -> documented in Files to Touch and the Merge note below
5. No additional authoritative event variant introduced (FND-28: enrich existing `EventTag::SleepEpisodeEnded` via payload widening, do not parallel it) -> spec text D6 + this ticket's Landed Changes confirm

## Landed Changes

### 1. New core module: `crates/worldwake-core/src/rest_site.rs`

Defined the two rest-site components:

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

Added `pub mod rest_site;` and re-exported `RestCapacity`, `RestOccupancy` in `crates/worldwake-core/src/lib.rs`.

### 2. Component schema registration in `crates/worldwake-core/src/component_schema.rs`

Following the `SleepQualityProfile` precedent (registered on `EntityKind::Place`), added entries for both new components. The filter is `|kind| kind == EntityKind::Place`. `RestCapacity` is scenario-authored in a later ticket; `RestOccupancy` is runtime-managed in a later ticket.

Imported `RestCapacity` and `RestOccupancy` in macro expansion sites (`delta.rs`, `world.rs`, `component_tables.rs`) per `tickets/README.md` check #13.

### 3. WakeReason payload widening at `crates/worldwake-core/src/decision_event_payload.rs:97-107`

Added the `SleepFailureCause` enum:

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

Restructured `WakeReason::LocalDisturbance` from a unit variant to a struct variant:

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

`WakeCondition::LocalDisturbance` at `sleep_episode.rs` stayed bare (asymmetric design per Q1=(b)).

### 4. Update the single `WakeReason::LocalDisturbance` construction site

`crates/worldwake-systems/src/needs_actions.rs` now constructs `WakeReason::LocalDisturbance { cause: SleepFailureCause::Generic }` in `abort_sleep_episode`. Ticket 004 (sleep handler `RestOccupancy` lifecycle) and ticket 009 (HostileProximity scenario) refine this per the abort context.

### 5. ActionTraceDetail::SleepInterrupted variant at `crates/worldwake-sim/src/action_trace.rs`

Added the new variant to the `ActionTraceDetail` enum:

```rust
SleepInterrupted {
    place: EntityId,
    cause: SleepFailureCause,
    accumulated_recovery: Permille,
    was_rough_sleep: bool,
},
```

Imported `SleepFailureCause` from `worldwake_core`. The variant is purely additive; population of this variant lands in ticket 006.

### 6. Bump SAVE_FORMAT_VERSION

In `crates/worldwake-sim/src/save_load.rs`, bumped `SAVE_FORMAT_VERSION` from `107` to `108`. The bump is attributed to this ticket because the `WakeReason` payload widening is the first incompatible serialized-format change in the S174 wave. Subsequent S174 tickets that add fields (e.g., ticket 002's `rough_sleep_recovery_floor`) ride this bump and do not need their own.

## Landed Files

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
- `specs/S174-shelter-sleep-surfaces-safe-rest.md` (modify — corrected the stored-state summary so `WakeCondition::LocalDisturbance` remains bare)
- No change required: `crates/worldwake-core/src/sleep_episode.rs` (continues to own bare `WakeCondition::LocalDisturbance`)
- No change required: `crates/worldwake-ai/tests/scenarios/sleep_episode.rs` (sleep goldens passed after the structured `WakeReason` change)

## Out of Scope

- No `WakeCondition::LocalDisturbance` restructuring (stays bare per Q1=(b))
- No belief-view accessors for `RestCapacity`/`RestOccupancy` (ticket 003)
- No sleep action handler changes to write/release `RestOccupancy` (ticket 004)
- No goal schema changes (ticket 005)
- No forensic record additions (ticket 006)
- No population of `ActionTraceDetail::SleepInterrupted` at the abort path — variant is added here, populated in ticket 006
- No predator/exposure/disease model — S174 explicitly defers these per its Non-Goals
- No reuse of `SelfCareOccupancy::Sleep` for rest sites — that enum variant remains but does not carry rest-site occupancy; ticket 006 will redirect the existing `SelfCareInterrupted { kind: Sleep, ... }` trace path to the new `SleepInterrupted` variant

## Acceptance Result

### Verified Behavior

1. Passed focused unit test: `RestCapacity` and `RestOccupancy` registration on `EntityKind::Place` through component schema.
2. Passed focused unit test: `SleepFailureCause` variants are constructible with all 5 variants and satisfy the required copy/hash/serde bounds.
3. Passed focused unit test: `WakeReason::LocalDisturbance { cause: SleepFailureCause::Generic }` constructs and pattern-matches correctly.
4. Passed focused unit test: `ActionTraceDetail::SleepInterrupted { place, cause, accumulated_recovery, was_rough_sleep }` constructs and equality-compares correctly.
5. Passed existing suite: `cargo test -p worldwake-core` (component registration, sleep episode types).
6. Passed existing suite: `cargo test -p worldwake-sim` (action trace types, save/load round-trip).
7. Passed existing suite: `cargo test -p worldwake-systems` (sleep abort path).
8. Passed existing suite: `cargo test -p worldwake-ai` (sleep episode goldens after `WakeReason` payload update).

### Verified Invariants

1. `RestCapacity` is only registered on `EntityKind::Place`.
2. `RestOccupancy` is only registered on `EntityKind::Place`; lifecycle mutation remains owned by ticket 004.
3. `WakeReason::LocalDisturbance` cannot be constructed without an explicit `cause` value.
4. `WakeCondition::LocalDisturbance` remains a bare unit variant.
5. `SAVE_FORMAT_VERSION` is exactly `108`; later S174 tickets must not bump it again unless reassessment proves a separate incompatible shape change.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-core/src/rest_site.rs` — focused unit coverage for `RestCapacity`/`RestOccupancy` component derives.
2. `crates/worldwake-core/src/component_schema.rs` — focused registration coverage proving both components are Place-only.
3. `crates/worldwake-core/src/decision_event_payload.rs` — `WakeReason::LocalDisturbance { cause }` constructor + pattern-match coverage; `SleepFailureCause` variant enumeration and bound coverage.
4. `crates/worldwake-sim/src/action_trace.rs` — `ActionTraceDetail::SleepInterrupted` variant construction + equality + summary formatting.
5. `crates/worldwake-sim/src/save_load.rs` — confirms `SAVE_FORMAT_VERSION == 108`; round-trip preserves `RestCapacity`, `RestOccupancy`, and a serialized `WakeReason::LocalDisturbance { cause }` payload.
6. `crates/worldwake-ai/tests/scenarios/sleep_episode.rs` — no source change required; the existing sleep goldens passed.

### Commands Run

1. Passed `cargo test -p worldwake-core rest_site`
2. Passed `cargo test -p worldwake-core component_schema::tests::rest_capacity_and_occupancy_are_registered_for_places_only -- --exact`
3. Passed `cargo test -p worldwake-core decision_event_payload::tests::structured_local_disturbance_wake_reason_carries_sleep_failure_cause -- --exact`
4. Passed `cargo test -p worldwake-sim action_trace::tests::sleep_interrupted_variant_constructs_and_derives -- --exact`
5. Passed `cargo test -p worldwake-sim save_load::tests::save_format_version_is_108_after_rest_site_foundation -- --exact`
6. Passed `cargo test -p worldwake-sim save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`
7. Passed `cargo test -p worldwake-systems --lib sleep`
8. Passed `cargo test -p worldwake-core`
9. Passed `cargo test -p worldwake-sim`
10. Passed `cargo test -p worldwake-systems`
11. Passed `cargo test -p worldwake-ai`
12. Passed `cargo fmt --all`
13. Passed `cargo test --workspace`
14. Waived `./scripts/verify.sh` for this per-ticket closeout because the `implement-spec-tickets` harness final branch phase owns the full pre-PR gate before push.

Merge note: Ticket 001 bumps `SAVE_FORMAT_VERSION` 107→108; sibling S174 tickets (002 adds `rough_sleep_recovery_floor` to MetabolismProfile, 006 adds `FailedRestOpportunity` to `CriticalWindowFrame`) deliberately ride this bump via `#[serde(default)]` on the new fields rather than bumping again.

## Outcome

Completed on 2026-05-26.

- Added `RestCapacity` and `RestOccupancy` as Place-only ECS components and wired them through the core component-schema/macro expansion surfaces.
- Replaced bare `WakeReason::LocalDisturbance` with `WakeReason::LocalDisturbance { cause: SleepFailureCause }`, added the five-cause `SleepFailureCause` enum, and updated the sleep abort path to use `SleepFailureCause::Generic` as the transitional cause.
- Added additive `ActionTraceDetail::SleepInterrupted` support without changing the sleep-abort trace producer; ticket 006 owns population.
- Bumped `SAVE_FORMAT_VERSION` from 107 to 108 and expanded save/load round-trip proof for the new rest-site components and structured sleep wake payload.
- Corrected the active S174 spec summary so `WakeCondition::LocalDisturbance` remains a bare trigger predicate while only `WakeReason` carries a structured cause.

## Deviations

- `ActionTraceDetail::SleepInterrupted` landed as an additive variant only. The producer redirect from `SelfCareInterrupted { kind: Sleep }` to `SleepInterrupted` remains owned by S174SHESLESUR-006.
- No sleep golden source edits were required; `cargo test -p worldwake-ai` proved the existing sleep goldens still pass with the structured wake-reason payload.

## Verification Result

- Passed `cargo test -p worldwake-core rest_site`
- Passed `cargo test -p worldwake-core component_schema::tests::rest_capacity_and_occupancy_are_registered_for_places_only -- --exact`
- Passed `cargo test -p worldwake-core decision_event_payload::tests::structured_local_disturbance_wake_reason_carries_sleep_failure_cause -- --exact`
- Passed `cargo test -p worldwake-sim action_trace::tests::sleep_interrupted_variant_constructs_and_derives -- --exact`
- Passed `cargo test -p worldwake-sim save_load::tests::save_format_version_is_108_after_rest_site_foundation -- --exact`
- Passed `cargo test -p worldwake-sim save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`
- Passed `cargo test -p worldwake-systems --lib sleep`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test -p worldwake-systems`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo fmt --all`
- Passed `cargo test --workspace`
- Passed stale-constructor scan: `rg -n "WakeReason::LocalDisturbance(?!\\s*\\{)" crates --pcre2` returned zero matches.
- Passed stale-version scan: `rg -n "SAVE_FORMAT_VERSION.*107|107.*SAVE_FORMAT_VERSION|self-care facility occupancy" crates/worldwake-sim/src/save_load.rs` returned zero matches.
- Waived `./scripts/verify.sh` because the full harness final branch phase owns the pre-PR verification gate before push.
