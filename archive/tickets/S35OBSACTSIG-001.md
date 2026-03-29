# S35OBSACTSIG-001: Add `BelievedActivity` struct and extend `BelievedEntityState`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core belief types
**Deps**: E14 (perception & belief system, already complete)

## Problem

Agents cannot observe what other co-located agents are doing. The first step is adding the data type that represents an observed activity and extending the belief state to carry it.

## Assumption Reassessment (2026-03-29)

1. `BelievedEntityState` is defined at `crates/worldwake-core/src/belief.rs:653` with fields: `last_known_place`, `last_known_inventory`, `workstation_tag`, `resource_source`, `alive`, `wounds`, `last_known_courage`, `observed_tick`, `source`. No `believed_activity` field exists.
2. `ActionDomain` is defined at `crates/worldwake-sim/src/action_domain.rs:4` with variants: Generic, Needs, Production, Trade, Social, Epistemic, Travel, Transport, Combat, Care, Corpse. It derives Serialize/Deserialize.
3. `BelievedEntityState` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.
4. `EntityId`, `Tick` are in `worldwake-core::ids`.
5. Shared abstraction boundary under audit: the serialized belief schema in `crates/worldwake-core/src/belief.rs` and the workspace-wide action classification enum currently defined in `crates/worldwake-sim/src/action_domain.rs`. This ticket is a type-contract change at that boundary only; it does not yet change perception, AI ranking, or `GoalBeliefView`.
6. This ticket introduces a new belief-side type in core. Since core cannot depend on sim, the spec’s `BelievedActivity.action_domain: ActionDomain` only remains clean if `ActionDomain` moves to `worldwake-core`. A mirrored `BelievedActionDomain` would create duplicate lawful paths for the same fact and violate the repo’s no-alias / no-backcompat direction. The corrected scope therefore includes moving `ActionDomain` into core and updating all imports directly.
7. `ObservedEntitySnapshot::to_believed_entity_state()` in `crates/worldwake-core/src/belief.rs:631` and multiple direct `BelievedEntityState { ... }` literals across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` will need explicit `believed_activity: None` initialization after the schema change.
8. Live save/load uses an explicit `SAVE_FORMAT_VERSION` header in `crates/worldwake-sim/src/save_load.rs`. During implementation, a focused legacy-serialization check showed that adding a new field to `BelievedEntityState` is not backward-compatible for the existing `bincode` payload, even with `#[serde(default)]`. The original ticket’s old-save compatibility assumption is therefore wrong.
9. Given the repo-wide no-backcompat rule, the corrected scope is to bump the save format version rather than add a compatibility shim for pre-change saves. Silent same-version breakage would be the worst architecture here.
10. `cargo test -p worldwake-core -- --list` succeeds against the current tree, but `cargo test -p worldwake-ai -- --list` currently fails before this ticket’s implementation due to unrelated compile errors in `crates/worldwake-ai/src/search/tests.rs` (`result` name mismatch and `EntityId` dereference errors around lines 4422-4539). The original ticket’s blanket assumption that the existing suite is green is therefore false and the verification baseline must record that discrepancy explicitly.
11. No existing tests reference `BelievedActivity` or `believed_activity`.

## Architecture Check

1. Adding `BelievedActivity` as an `Option` field on `BelievedEntityState` is the minimal, clean extension. It follows P3 (concrete action domain + target, not abstract busyness score) and P12 (belief state, not authoritative state).
2. Moving `ActionDomain` to core is architecturally cleaner than introducing a second belief-only enum. The domain classification is not scheduler behavior; it is shared action vocabulary already consumed by sim, systems, AI, and now belief state. One canonical enum in core is the durable architecture.
3. An explicit save-format version bump is cleaner than pretending this schema change is wire-compatible. The runtime already has a save-version boundary, so the robust response is to use it honestly instead of smuggling in a partial compatibility story.
4. This ticket should stay narrow. It establishes the data contract needed by S35 without prematurely coupling perception or ranking policy into the type move. The follow-up tickets remain responsible for populating and consuming the field.

## Verification Layers

1. `BelievedActivity` construction and field access -> focused unit test
2. `BelievedEntityState.believed_activity` set/get -> focused unit test
3. `ActionDomain` canonical location move -> focused compile/use coverage in `worldwake-core` and workspace crates that import it
4. Save-schema incompatibility is surfaced explicitly -> save header/version verification in `worldwake-sim`
5. Single-layer ticket: type contract only, no perception/ranking cross-system behavior in scope yet

## What to Change

### 1. Move `ActionDomain` enum to worldwake-core

Move `ActionDomain` from `crates/worldwake-sim/src/action_domain.rs` to a new `crates/worldwake-core/src/action_domain.rs`. Update all imports across workspace crates to use `worldwake_core::ActionDomain`. Remove the old file from sim and update sim's `lib.rs` to re-export from core during the transition — actually, per P26 (no backward compatibility layers), update all sim consumers directly.

### 2. Add `BelievedActivity` struct to `crates/worldwake-core/src/belief.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BelievedActivity {
    pub action_domain: ActionDomain,
    pub target: Option<EntityId>,
    pub observed_tick: Tick,
}
```

### 3. Extend `BelievedEntityState`

Add `pub believed_activity: Option<BelievedActivity>` field.

### 4. Update all `BelievedEntityState` constructors

Every place that constructs `BelievedEntityState` must now include `believed_activity: None`.

### 5. Bump save format version

Because `BelievedEntityState` is serialized into save payloads and the repo does not support backward-compatibility shims, bump `SAVE_FORMAT_VERSION` in `worldwake-sim` so pre-change saves fail with an explicit unsupported-version error instead of an opaque deserialization failure.

## Files to Touch

- `crates/worldwake-core/src/action_domain.rs` (new — moved from sim)
- `crates/worldwake-core/src/lib.rs` (modify — register new module, export `ActionDomain`)
- `crates/worldwake-core/src/belief.rs` (modify — add `BelievedActivity` struct, extend `BelievedEntityState`)
- `crates/worldwake-sim/src/action_domain.rs` (delete — moved to core)
- `crates/worldwake-sim/src/lib.rs` (modify — remove old module, import from core)
- All files importing `ActionDomain` from sim (modify — update imports)

## Out of Scope

- Perception system changes (S35OBSACTSIG-003)
- `GoalBeliefView` extensions (S35OBSACTSIG-004)
- Ranking discount logic (S35OBSACTSIG-006)
- `UtilityProfile` changes (S35OBSACTSIG-002)
- Save/load round-trip testing (S35OBSACTSIG-008)

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `BelievedActivity` can be constructed with all `ActionDomain` variants, `Some`/`None` target, and a tick value.
2. Unit test: `BelievedEntityState` with `believed_activity: Some(...)` and `believed_activity: None` both construct and compare correctly.
3. Focused verification for `worldwake-core` passes after the schema move.
4. Broader workspace verification is rerun after implementation; any pre-existing unrelated failure is called out explicitly rather than misattributed to this ticket.

### Invariants

1. `ActionDomain` is now authoritative in `worldwake-core` — no duplicate definition exists in sim.
2. All existing `BelievedEntityState` construction sites initialize `believed_activity: None`.
3. `BelievedEntityState` equality comparison includes the new field.
4. The current save format version is bumped so pre-change saves are rejected explicitly rather than failing under a misleading unchanged version number.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` — unit tests for `BelievedActivity` construction and `BelievedEntityState` equality/roundtrip behavior.
2. `crates/worldwake-core/src/action_domain.rs` — moved enum keeps trait/roundtrip coverage at its new canonical location.
3. `crates/worldwake-sim/src/save_load.rs` — save header/version tests cover the explicit format bump.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test -p worldwake-ai -- --list`
3. `cargo test --workspace`
4. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-29
- Actual changes:
  - Moved `ActionDomain` from `worldwake-sim` to `worldwake-core` and updated workspace imports to the new canonical location.
  - Added `BelievedActivity` and `BelievedEntityState.believed_activity` in `worldwake-core`.
  - Updated all live `BelievedEntityState` construction sites to initialize `believed_activity: None`.
  - Bumped `SAVE_FORMAT_VERSION` in `worldwake-sim` so the schema change is represented honestly at the save boundary.
  - Repaired the pre-existing `worldwake-ai` test breakage discovered during reassessment so full workspace verification could complete.
- Deviations from original plan:
  - Did not preserve old-save deserialization. Reassessment plus focused testing showed the proposed `#[serde(default)]` approach was not actually compatible with the existing `bincode` payload shape, and the repo’s no-backcompat rule favored an explicit format bump instead.
  - Broadened the touch set beyond the original “types only” expectation because the field addition forced test and constructor updates across workspace crates, and full verification required fixing unrelated pre-existing `worldwake-ai` test compile failures.
- Verification results:
  - `cargo test -p worldwake-core` passed.
  - `cargo test -p worldwake-sim` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
