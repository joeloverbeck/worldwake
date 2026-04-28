# S128SLEEPIPLA-001: Sleep core types and event tags

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new core components (`SleepEpisode`, `SleepQualityProfile`), new core enum (`WakeCondition`), new core payload structs and enum (`SleepEpisodeStartedPayload`, `SleepEpisodeEndedPayload`, `WakeReason`), two new `EventTag` variants, save format bump.
**Deps**: specs/S128-sleep-episode-place-quality.md (D1, D2, D3, D4)

## Problem

The current `tick_sleep` handler runs every tick and the planner re-selects sleep next tick, producing 143–146 separate `sleep` action commits per agent in a 1440-tick run (`reports/proposed-gameplay-mechanic-changes.md:191`). The deeper architectural problem is that sleep cannot have intent: there is no episode-level state to carry "sleep until fatigue is below comfort" or "sleep until thirst projection breaches", and there is no place-quality data that would let an agent prefer one sleep site over another. This ticket lays the data foundation so subsequent tickets can refactor the handler (S128SLEEPIPLA-004), add per-place ranking (S128SLEEPIPLA-005), and author scenario differentiation (S128SLEEPIPLA-006).

## Assumption Reassessment (2026-04-27)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-core/src/event_tag.rs` enumerates 39 `EventTag` variants today (lines 8-47); no `SleepEpisodeStarted` / `SleepEpisodeEnded` exist. `EventTag` is consumed via `event_log.events_by_tag(...)` queries — no exhaustive match on `EventTag` exists in the workspace, so the two new variants do not require match-arm updates.
2. `crates/worldwake-core/src/decision_event_payload.rs` follows the convention: `pub enum DecisionEventPayload { Variant(VariantPayload), ... }` with one struct per tag (lines 10-22, 26-294). `Sleep`-prefixed payload structs do not exist. The S128 spec (D4) requires `SleepEpisodeStartedPayload`, `SleepEpisodeEndedPayload`, and a new `WakeReason` enum following this convention.
3. Shared boundary under audit: serialized state. `crates/worldwake-sim/src/save_load.rs:6` has `SAVE_FORMAT_VERSION = 53`. New components serialized via `bincode` and new `EventTag` variants change the on-disk shape, requiring a single bump to `54` in this ticket. Later S128SLEEPIPLA-002 reassessment corrected the handoff: adding `min_sleep_ticks` to persisted `MetabolismProfile` also requires its own current-format bump to `55`, while `#[serde(default)]` only preserves authored scenario omission.
4. `crates/worldwake-core/src/component_schema.rs` `with_component_schema_entries!` macro (line 3) registers components by referencing types via `crate::TypeName`; both `SleepEpisode` and `SleepQualityProfile` must therefore live in `worldwake-core` (per the core-residence constraint at `references/worldwake-validation-patterns.md`). Existing place components register with `|kind| kind == EntityKind::Place` filter (lines 1660, 1685, 1710); `SleepQualityProfile` follows this filter. Existing agent components register with the agent kind filter; `SleepEpisode` follows that filter.
5. `WakeCondition::PlaceNoLongerSafe` is intentionally absent per spec Non-Goals — deferred until S60 lands `OccupancyClaim`/`OccupancyPosture`. The variant must NOT appear in this ticket. Spec Non-Goals (line 46) and D1 explanation (line 110) document the deferral.

## Architecture Check

1. `SleepEpisode` is runtime-generated state (created at sleep start, removed at commit) — exempt from the FND-22 Section 5 scenario contract per `docs/spec-drafting-rules.md:33`. Analogous to `AgendaEntry`, `IntentionFrame`, `WoundList`. No `AgentDef` field needed.
2. `SleepQualityProfile` is registered as a place component with a default `(Open, Earth, 1000)` representing "no modulation." This ticket establishes the core type/schema substrate; S128SLEEPIPLA-006 owns making scenario-spawned places always carry the component.
3. `WakeReason` mirrors `WakeCondition` for the end-event payload but adds extra context (e.g., `ProjectedNeedBreach { need, projected_breach_tick }`) so the decision trace can answer "why did Agent A wake at tick T?" from event log alone (FND-29).
4. No backward-compatibility shims: the SAVE_FORMAT_VERSION bump rejects older saves (`save_load.rs:1148` pattern), consistent with FND-28 in the live authority path; older saves do not silently coexist with new schema.

## Verification Layers

1. New components registered and accessible via macro-generated accessors → focused unit tests in `component_schema.rs` test module asserting `set_component_sleep_episode` / `get_component_sleep_episode` round-trip on Agent and `set_component_sleep_quality_profile` / `get_component_sleep_quality_profile` round-trip on Place.
2. Serialization round-trip for both components → focused unit tests in `crates/worldwake-sim/src/save_load.rs` test module asserting bincode serialize → deserialize preserves field values, including all `WakeCondition` variants and `WakeReason` variants.
3. SAVE_FORMAT_VERSION bump — `cargo test -p worldwake-sim save_load` confirms the version constant changed and old-version rejection still trips at the existing assertion site (`save_load.rs:1140-1148`).
4. EventTag variants added — `cargo test -p worldwake-core event_tag` (existing tests assert variant uniqueness, ordering); spot-check that the two new variants serialize/deserialize.
5. Single-layer ticket: this is pure data-foundation work with no behavioral changes to action handlers, AI candidate generation, or runtime systems — those layers land in subsequent tickets (003-007). Verification therefore lives entirely at the focused unit / round-trip layer; no decision trace, action trace, or golden E2E assertions are appropriate yet.

## What to Change

### 1. New module `crates/worldwake-core/src/sleep_episode.rs`

Create the module with:

- `pub enum WakeCondition` with variants `IntendedDurationReached`, `TargetRecoveryReached`, `ProjectedNeedBreach { need: HomeostaticNeedId }`, `ScheduledCommitmentDue { tick: Tick }`, `LocalDisturbance`. Derives `Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize`. (`Hash`/`Ord` for determinism in any future `BTreeSet<WakeCondition>` storage.)
- `pub struct SleepEpisode` with fields `place: EntityId`, `start_tick: Tick`, `intended_min_ticks: NonZeroU32`, `intended_max_ticks: NonZeroU32`, `target_recovery: Permille`, `accumulated_recovery: Permille`, `recovery_modifier: Permille`, `wake_conditions: Vec<WakeCondition>`. Derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `impl Component for SleepEpisode {}`.
- `pub enum ShelterTag` with variants `Open, PartialCover, Roofed, Shelter`. Standard derives + `Copy`.
- `pub enum GroundComfortTag` with variants `Hard, Earth, Soft`. Standard derives + `Copy`.
- `pub struct SleepQualityProfile` with fields `shelter: ShelterTag`, `ground_comfort: GroundComfortTag`, `recovery_modifier: Permille`. Standard derives + `Copy`. `impl Component for SleepQualityProfile {}`. `impl Default for SleepQualityProfile { fn default() -> Self { Self { shelter: ShelterTag::Open, ground_comfort: GroundComfortTag::Earth, recovery_modifier: Permille::new_unchecked(1000) } } }`.

Re-export at `crates/worldwake-core/src/lib.rs`: `pub use sleep_episode::{SleepEpisode, SleepQualityProfile, WakeCondition, ShelterTag, GroundComfortTag};`.

### 2. Register components in `crates/worldwake-core/src/component_schema.rs`

Add two entries in the `with_component_schema_entries!` macro invocation following the existing patterns:

- `SleepEpisode` registered with the agent kind filter (mirrors `AgendaEntry` / `IntentionFrame` registrations); insert/get/clear accessors generated as `set_component_sleep_episode`, `get_component_sleep_episode`, `clear_component_sleep_episode`.
- `SleepQualityProfile` registered with `|kind| kind == EntityKind::Place`; accessors generated as `set_component_sleep_quality_profile`, `get_component_sleep_quality_profile`, `clear_component_sleep_quality_profile`.

Verify both new types are imported at all macro expansion sites per `tickets/README.md` check #13: `delta.rs`, `world.rs`, `component_tables.rs`. Add `use crate::sleep_episode::{SleepEpisode, SleepQualityProfile};` at each site.

### 3. New `EventTag` variants in `crates/worldwake-core/src/event_tag.rs`

Add `SleepEpisodeStarted` and `SleepEpisodeEnded` to the `EventTag` enum (around line 47, preserving the existing variant ordering convention — alphabetical or grouped, follow the surrounding pattern).

### 4. New payload types in `crates/worldwake-core/src/decision_event_payload.rs`

Add the following per S110 conventions (struct per variant, payload owned by enum variant):

```rust
pub struct SleepEpisodeStartedPayload {
    pub sleeper: EntityId,
    pub place: EntityId,
    pub intended_min_ticks: NonZeroU32,
    pub intended_max_ticks: NonZeroU32,
    pub target_recovery: Permille,
    pub wake_conditions: Vec<WakeCondition>,
    pub recovery_modifier: Permille,
}

pub struct SleepEpisodeEndedPayload {
    pub sleeper: EntityId,
    pub place: EntityId,
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub end_reason: WakeReason,
    pub accumulated_recovery: Permille,
    pub final_fatigue: Permille,
}

pub enum WakeReason {
    IntendedDuration,
    TargetRecovery,
    ProjectedNeedBreach { need: HomeostaticNeedId, projected_breach_tick: Tick },
    ScheduledCommitment,
    LocalDisturbance,
}
```

Both payload structs derive `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. `WakeReason` derives the same plus `Copy`. Add corresponding variants to the `DecisionEventPayload` enum at the file top (lines 10-22).

### 5. Save-format bump

In `crates/worldwake-sim/src/save_load.rs:6`, bump `SAVE_FORMAT_VERSION` from `53` to `54`. Update the assertion at line 883 if it carries the literal value. Re-run the version-rejection test at `save_load.rs:1140-1148` — it asserts `SAVE_FORMAT_VERSION - 1` is rejected, which still holds after the bump.

## Files to Touch

- `crates/worldwake-core/src/sleep_episode.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — re-export new types)
- `crates/worldwake-core/src/component_schema.rs` (modify — register two components)
- `crates/worldwake-core/src/delta.rs` (modify — import new component types per macro expansion site)
- `crates/worldwake-core/src/world.rs` (modify — import new component types per macro expansion site)
- `crates/worldwake-core/src/component_tables.rs` (modify — import new component types per macro expansion site)
- `crates/worldwake-core/src/event_tag.rs` (modify — add 2 variants)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify — add 2 payload structs + 1 enum + 2 enum variants)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump version constant)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — shared payload fallout; render the new decision-event payloads explicitly)

## Out of Scope

- `WakeCondition::PlaceNoLongerSafe` — deferred per spec Non-Goals (S60 dependency not yet implemented)
- `MetabolismProfile.min_sleep_ticks` field addition — handled by archive/tickets/S128SLEEPIPLA-002.md
- `DurationExpr::Variable { min, max }` — handled by S128SLEEPIPLA-003
- `GoalBeliefView::place_sleep_quality_profile` accessor — handled by S128SLEEPIPLA-003
- `tick_sleep` handler refactor — handled by S128SLEEPIPLA-004
- Per-place sleep candidate emission — handled by S128SLEEPIPLA-005
- `PlaceDef.sleep_quality` scenario authoring — handled by S128SLEEPIPLA-006
- Golden tests for sleep episodes — handled by S128SLEEPIPLA-007

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-core sleep_episode` — focused unit tests in the new module asserting `SleepQualityProfile::default()` returns `(Open, Earth, Permille(1000))` and that all `WakeCondition` variants and `WakeReason` variants serialize/deserialize symmetrically via bincode.
2. `cargo test -p worldwake-core component_schema` — existing schema test sweep extended to assert both new components round-trip via `set_component_*`/`get_component_*` accessors and that `clear_component_*` removes them.
3. `cargo test -p worldwake-sim save_load` — existing save-format tests pass after the version bump; the version-rejection test still rejects the prior version.
4. Existing suite: `cargo test --workspace`.

### Invariants

1. `SleepEpisode` and `SleepQualityProfile` live in `worldwake-core` (core-residence constraint per `references/worldwake-validation-patterns.md`).
2. `WakeCondition` does not include a `PlaceNoLongerSafe` variant (deferred until S60 lands).
3. `SleepQualityProfile::default().recovery_modifier == Permille::new_unchecked(1000)` — "no modulation" exactly matches existing per-tick recovery rate.
4. `SAVE_FORMAT_VERSION == 54` after this ticket; older saves are rejected with the existing `Mismatch` error path.
5. `EventTag` retains its existing `Ord`/`Hash` derive compatibility — adding two unit variants does not require derive widening.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/sleep_episode.rs` (new — module-internal `#[cfg(test)]` block) — round-trip tests for `SleepEpisode`, `SleepQualityProfile`, `WakeCondition`, `WakeReason`; `Default` assertion for `SleepQualityProfile`.
2. `crates/worldwake-core/src/component_schema.rs` (modify — extend the existing schema test sweep around line 2266 with two new test functions: one inserting/reading `SleepEpisode` on an Agent, one inserting/reading `SleepQualityProfile` on a Place).
3. `crates/worldwake-sim/src/save_load.rs` (modify — extend the existing round-trip test sweep to seed a world containing one agent with a `SleepEpisode` component and one place with an authored `SleepQualityProfile`, save, load, and assert byte-identical state hash).

### Commands

1. Focused exact selectors listed in `Verification Result` for `sleep_episode`, `component_schema`, `decision_event_payload`, `event_tag`, and `save_load`.
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-sim`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `./scripts/verify.sh`

## Outcome

Completed on 2026-04-28.

Outcome amended: 2026-04-28.

- Added `SleepEpisode`, `WakeCondition`, `SleepQualityProfile`, `ShelterTag`, and `GroundComfortTag` in `worldwake-core`, with crate re-exports and focused bincode/default tests.
- Registered `SleepEpisode` for agents and `SleepQualityProfile` for places through the component schema macro, including `ComponentKind` / `ComponentValue` sample coverage and focused component-schema tests.
- Added `SleepEpisodeStarted` / `SleepEpisodeEnded` event tags, matching `DecisionEventPayload` variants, payload structs, and `WakeReason`.
- Bumped `SAVE_FORMAT_VERSION` to `54` and extended save/load round-trip coverage to preserve non-default sleep components and the new decision-event payloads.
- Updated the CLI observer's exhaustive decision-payload rendering so the new sleep episode events have an agent, event name, and compact payload summary.
- Amended the S128SLEEPIPLA-002 handoff note after live reassessment proved the later persisted `MetabolismProfile.min_sleep_ticks` field needs a separate `SAVE_FORMAT_VERSION` bump to `55`.

## Deviations

- The ticket's drafted file list missed `crates/worldwake-cli/src/bin/observer.rs`; workspace verification exposed it as legitimate shared-payload fallout.
- `SleepQualityProfile` is place-valid and defaultable after this ticket, but scenario-spawned places are not universally seeded here; S128SLEEPIPLA-006 still owns the `PlaceDef.sleep_quality` and `spawn_place` wiring.
- No handler, AI, scenario authoring, or golden behavior landed here. The metabolism-profile substrate is now archived in S128SLEEPIPLA-002; the remaining behavior and proof slices stay owned by S128SLEEPIPLA-003 through S128SLEEPIPLA-007 as drafted.

## Verification Result

- Passed `cargo test -p worldwake-core --lib sleep_episode::tests::sleep_episode_roundtrips_through_bincode -- --exact`.
- Passed `cargo test -p worldwake-core --lib sleep_episode::tests::sleep_quality_profile_roundtrips_through_bincode -- --exact`.
- Passed `cargo test -p worldwake-core --lib sleep_episode::tests::sleep_quality_profile_default_is_unmodulated_open_earth -- --exact`.
- Passed `cargo test -p worldwake-core --lib component_schema::tests::sleep_episode_is_registered_for_agents_only -- --exact`.
- Passed `cargo test -p worldwake-core --lib component_schema::tests::sleep_quality_profile_is_registered_for_places_only -- --exact`.
- Passed `cargo test -p worldwake-core --lib decision_event_payload::tests::decision_event_payload_variants_roundtrip_through_bincode -- --exact`.
- Passed `cargo test -p worldwake-core --lib event_tag::tests::event_tag_bincode_roundtrip_covers_every_variant -- --exact`.
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_full_nondefault_state -- --exact`.
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_decision_event_payloads -- --exact`.
- Passed `cargo test -p worldwake-sim --lib save_load::tests::load_rejects_wrong_version -- --exact`.
- Passed `cargo test -p worldwake-core`.
- Passed `cargo test -p worldwake-sim`.
- Passed `cargo test --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh`, whose live gate set is `cargo fmt --all -- --check`, `cargo test --workspace`, `bash scripts/check_active_goal_removed.sh`, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
