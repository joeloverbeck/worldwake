# S09INDACTREEVA-007: Collapse `ActionDuration` into a finite runtime duration newtype

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-sim` runtime duration type shape, save/load schema version, downstream call sites/tests
**Deps**: archive/tickets/completed/S09INDACTREEVA-006.md, specs/S09-indefinite-action-re-evaluation.md

## Problem

S09INDACTREEVA-006 removed indefinite live durations from the runtime and planner. After that change, `ActionDuration` in [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs) no longer represents a runtime sum type. It is now only `Finite(u32)`, but the API still exposes enum-shaped semantics such as `fixed_ticks() -> Option<u32>`.

That leaves the runtime duration layer in an awkward middle state:

- the architecture correctly forbids indefinite live durations
- the type surface still advertises an obsolete “maybe multiple runtime duration kinds” shape
- downstream callers still pattern-match on a type that now represents only one concrete concept: remaining ticks on an active action instance

This is not a behavioral bug, but it is type-level drift. It weakens Principle 3 by making the type less concrete than the runtime state it actually carries, and it keeps obsolete optionality in a boundary that should now be explicit and finite-only.

## Assumption Reassessment (2026-03-20)

1. `ActionDuration` currently lives in [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs) as:
   - `pub enum ActionDuration { Finite(u32) }`
   - `fixed_ticks(self) -> Option<u32>`
   - `advance(&mut self) -> bool`
   This confirms the old indefinite-branch API shape survived even though the only runtime case is finite.
2. The runtime meaning of `ActionDuration` is now specifically “resolved remaining duration for an active action instance,” as stored in [action_instance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs). It is no longer a planning-time or semantic-duration union.
3. The true semantic branching boundary is already `DurationExpr` in [action_semantics.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_semantics.rs), which still correctly models fixed vs actor/target/payload-derived durations. This ticket does not change that boundary; it only cleans the resolved runtime type after that resolution step.
4. Current downstream runtime usage is finite-only and mechanically broad:
   - active instance storage in [action_instance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs)
   - authoritative start and reservation handling in [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs)
   - action ticking in [tick_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs)
   - CLI presentation in [tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/tick.rs) and [world_overview.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/world_overview.rs)
   - focused/system/golden test fixtures that seed `remaining_duration`
   There is no remaining live `Indefinite` branch in `crates/`.
5. The current API shape already causes low-value destructuring noise. For example, [search.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search.rs) previously needed an infallible enum match that `clippy` correctly rejected once the indefinite variant was gone. This is a concrete sign that the type representation is now more general than the architecture needs.
6. `ActionDuration` participates in persisted simulation state through [save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs). Changing it from enum to newtype will change serialized bincode layout for [SimulationState](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/simulation_state.rs) and [ActionInstance](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs). Because this repo explicitly avoids backward-compatibility shims, the clean architecture path is to bump `SAVE_FORMAT_VERSION`, not to add transitional deserializers.
7. Existing coverage is already present at the right layers for this cleanup:
   - focused/unit: `action_duration_roundtrips_through_bincode` in [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs), `action_instance_roundtrips_*` in [action_instance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs)
   - runtime focused: `tick_action_*finite*` in [tick_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs), `start_action_supports_*finite*` in [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs)
   - persistence boundary: `save_to_bytes_roundtrip_preserves_full_nondefault_state` and version rejection tests in [save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs)
   - workspace regression: `cargo test --workspace`, `cargo clippy --workspace`
8. This is not an AI-regression ticket. Candidate generation, ranking, plan search, and action semantics should remain behaviorally unchanged. The intended work is runtime type-surface and persistence-contract cleanup.
9. No action-lifecycle ordering contract changes here. The only ordering-sensitive code touched should be countdown storage and reservation span derivation, both of which already operate on finite ticks today.
10. Mismatch + correction: there is no current ticket owning this post-S09 type-surface cleanup. It is not part of S09INDACTREEVA-006’s required invariant fix, but it is a reasonable follow-up to make the runtime duration layer honest and concrete.

## Architecture Check

1. A dedicated finite runtime duration type is cleaner than a single-variant enum because it encodes the actual post-S09 invariant directly: active action durations are always finite remaining ticks. This better satisfies Principle 3 in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md) by making the authoritative runtime type match the concrete state it carries.
2. The right cleanup is a true representation change, not more helper methods layered on the enum. Leaving the enum in place and renaming methods would preserve obsolete shape and keep callers destructuring a pseudo-sum-type that no longer exists architecturally.
3. The type should remain capable of representing zero remaining ticks because zero is meaningful during active runtime countdown and reservation handling in [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs). `NonZeroU32` would be the wrong abstraction here.
4. No backward-compatibility aliasing or custom migration shims should be added. If the serialized representation changes, bump `SAVE_FORMAT_VERSION` and make older saves fail explicitly through the existing unsupported-version path.

## Verification Layers

1. Runtime duration type now encodes finite-only remaining ticks without obsolete optionality -> focused unit coverage in [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs)
2. Active action instances still roundtrip through serde/bincode with the new type shape -> focused persistence coverage in [action_instance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs)
3. Countdown and reservation behavior remain unchanged for finite actions -> focused runtime coverage in [tick_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs) and [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs)
4. Whole-simulation save/load remains correct and the representation change is made explicit at the boundary -> save/load tests in [save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs), including version handling
5. This is not a planning or action-semantic behavior ticket, so decision trace and action trace are not primary proof surfaces; focused runtime/persistence coverage plus workspace regression are the correct verification boundary

## What to Change

### 1. Replace the single-variant enum with a finite runtime-duration value type

In [action_duration.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_duration.rs):

- replace `pub enum ActionDuration { Finite(u32) }` with a dedicated finite runtime duration type
- recommended shape:
  - `pub struct ActionDuration(u32);`
  - constructor/accessor APIs that keep the finite-only invariant explicit
  - keep zero remaining ticks representable
- remove enum-style APIs that imply extinct branching, especially `fixed_ticks() -> Option<u32>`
- replace them with direct finite APIs such as `ticks()` / `remaining_ticks()` as appropriate
- keep the countdown behavior explicit and deterministic

### 2. Normalize downstream call sites to the finite runtime-duration API

In runtime/storage/consumer code such as:

- [action_instance.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_instance.rs)
- [start_gate.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/start_gate.rs)
- [tick_action.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_action.rs)
- [tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/tick.rs)
- [world_overview.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/handlers/world_overview.rs)
- focused/system/golden tests that seed active actions

- remove obsolete `ActionDuration::Finite(...)` pattern syntax
- update countdown, display, and reservation logic to use the new finite-only API directly
- prefer explicit finite accessors over recreating enum-like matches

### 3. Make the serialization boundary honest

In [save_load.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/save_load.rs):

- bump `SAVE_FORMAT_VERSION`
- update persistence tests to reflect the intentional wire-format change
- keep the existing explicit unsupported-version rejection path for older saves
- do not add compatibility deserializers or multi-version branching

### 4. Strengthen focused coverage around the new runtime-duration contract

- update `action_duration` tests so they assert the new finite-only API shape directly
- update `action_instance` roundtrip tests so the new value type remains covered through serde
- update save/load tests so the version bump is intentional and verified
- keep finite runtime behavior checks in `start_gate` / `tick_action` aligned with the new API

## Files to Touch

- `crates/worldwake-sim/src/action_duration.rs` (modify)
- `crates/worldwake-sim/src/action_instance.rs` (modify)
- `crates/worldwake-sim/src/start_gate.rs` (modify)
- `crates/worldwake-sim/src/tick_action.rs` (modify)
- `crates/worldwake-sim/src/save_load.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify if re-export/docs need adjustment)
- `crates/worldwake-ai/src/search.rs` (modify only if any finite-accessor cleanup remains)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify)
- focused/system/golden tests that seed `remaining_duration` values across `crates/worldwake-ai/`, `crates/worldwake-systems/`, and `crates/worldwake-sim/` (modify as needed)

## Out of Scope

- Reintroducing indefinite durations or any alternate runtime duration variants
- Changing `DurationExpr` or authoritative duration-resolution semantics
- Planner-cost or defend-behavior changes; those were addressed by S09INDACTREEVA-006
- Save-file migration shims, backward-compatibility loaders, or alias representations for the old enum shape

## Acceptance Criteria

### Tests That Must Pass

1. Focused finite-duration API and countdown tests in `worldwake-sim` pass
2. Focused persistence tests for `ActionInstance` and full `SimulationState` pass, including explicit save-format version expectations
3. Existing suite: `cargo test -p worldwake-sim`
4. Existing suite: `cargo test --workspace`
5. Existing suite: `cargo clippy --workspace`

### Invariants

1. `ActionDuration` no longer exposes an enum shape or optional fixed-ticks API that implies non-finite runtime cases
2. Active action remaining duration still supports `0` as a concrete runtime state
3. No backward-compatibility save shim is added; representation change is made explicit through save-format versioning
4. Runtime countdown, reservation span derivation, and CLI display remain behaviorally unchanged under the new finite-only type

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_duration.rs` — replace enum-shaped tests with direct finite-value-type API checks; rationale: the type itself is the primary invariant under change.
2. `crates/worldwake-sim/src/action_instance.rs` — keep serde/bincode roundtrip coverage for active instances under the new duration representation; rationale: this is the smallest direct persistence boundary for the type.
3. `crates/worldwake-sim/src/save_load.rs` — update full-state save/load and version-rejection assertions; rationale: this ticket intentionally changes the serialized payload shape and must own that boundary explicitly.
4. `crates/worldwake-sim/src/start_gate.rs` — adjust finite-duration reservation/runtime assertions to the new API surface; rationale: reservation windows are one of the few places that interpret remaining ticks directly.
5. `crates/worldwake-sim/src/tick_action.rs` — adjust countdown/commit lifecycle assertions to the new API surface; rationale: this proves the runtime behavior is unchanged while the representation is cleaned up.

### Commands

1. `cargo test -p worldwake-sim action_duration`
2. `cargo test -p worldwake-sim action_instance_roundtrips_with_some_local_state`
3. `cargo test -p worldwake-sim save_to_bytes_roundtrip_preserves_full_nondefault_state`
4. `cargo test -p worldwake-sim`
5. `cargo test --workspace`
6. `cargo clippy --workspace`
