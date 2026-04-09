# S81GLDGAP-001: DeathCause enum, DeadAt restructure, EventTag::Death

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes -- worldwake-core type definitions (DeadAt, DeathCause, HomeostaticNeedId, EventTag)
**Deps**: None

## Problem

Death events carry no cause information. `DeadAt` is a bare `Tick` wrapper with no indication of why the agent died. This blocks death traceability (P29), richer downstream consequences (P5), and queryable death events in the event log.

## Assumption Reassessment (2026-04-09)

1. `DeadAt` is a tuple struct `DeadAt(pub Tick)` at `crates/worldwake-core/src/combat.rs:59` with derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`. Confirmed via grep.
2. `HomeostaticNeedId` at `crates/worldwake-core/src/needs.rs:18` derives `Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize` -- missing `Hash`. Confirmed via grep.
3. `EventTag` enum at `crates/worldwake-core/src/event_tag.rs:7` has 24 variants. `ALL_EVENT_TAGS` const at line 44 and count assertion at line 78. No `Death` variant. Confirmed via grep.
4. `Tick` at `crates/worldwake-core/src/ids.rs:56` already derives `Ord, PartialOrd, Hash` -- the widening of `DeadAt` to include these is safe.
5. Re-export at `crates/worldwake-core/src/lib.rs:118`: `pub use combat::{CombatProfile, CombatStance, DeadAt};` -- needs `DeathCause` added.
6. Macro expansion sites for `DeadAt`: `delta.rs` (line 8 import), `world.rs` (component schema usage), `component_tables.rs` (2 references). These are macro-generated from `component_schema.rs` and will compile once the type shape changes -- no manual updates needed beyond the struct definition change.
7. Ticket mismatch: direct `DeadAt(...)` construction and equality assertions exist across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-ai`, including production call sites in `crates/worldwake-sim/src/tick_step.rs` and `crates/worldwake-systems/src/combat.rs`. The original claim that other-crate constructor fallout is out of scope is incompatible with the live codebase because removing the tuple struct would otherwise leave the branch uncompilable. Safe correction: include all direct fallout required by the shared type change.

## Architecture Check

1. Adding `DeathCause` as a field on `DeadAt` enforces the invariant at the type level -- you cannot create a `DeadAt` without specifying the cause. This is cleaner than a separate optional `DeathCause` component which could become desynchronized.
2. No backward-compatibility shims. The tuple struct is replaced entirely; all construction sites must update.

## Verification Layers

1. `DeadAt` struct shape changed -> compilation of all crates (type-level proof)
2. `HomeostaticNeedId` has `Hash` -> `DeathCause` containing it compiles (type-level proof)
3. `EventTag::Death` exists -> event_tag.rs unit tests pass (focused unit test)
4. Re-export includes `DeathCause` -> downstream crates can import it (compilation proof)
5. Direct `DeadAt` constructor/equality fallout across workspace crates compiles and passes targeted tests (shared-shape proof)

## What to Change

### 1. Add Hash derive to HomeostaticNeedId

In `crates/worldwake-core/src/needs.rs:18`, add `Hash` to the derive list:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum HomeostaticNeedId {
```

### 2. Add DeathCause enum to combat.rs

In `crates/worldwake-core/src/combat.rs`, before the `DeadAt` definition, add:

```rust
/// Cause of an agent's death, set alongside DeadAt.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DeathCause {
    /// Died from an unmet need reaching lethal wound load.
    NeedDeprivation { need: HomeostaticNeedId },
    /// Died from combat wounds.
    CombatWounds,
}
```

This requires adding `use crate::HomeostaticNeedId;` to the combat.rs imports if not already present.

### 3. Restructure DeadAt from tuple to named-field struct

Replace the current `DeadAt` definition with:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct DeadAt {
    pub tick: Tick,
    pub cause: DeathCause,
}

impl Component for DeadAt {}
```

### 4. Update re-export in lib.rs

Change `crates/worldwake-core/src/lib.rs:118`:

```rust
pub use combat::{CombatProfile, CombatStance, DeadAt, DeathCause};
```

### 5. Update direct DeadAt construction/equality fallout

Replace tuple-style `DeadAt(...)` construction and tuple-value equality assertions at the live ownership sites with the named-field shape. Production sites must use the correct cause:

- Combat fatality path -> `DeathCause::CombatWounds`
- Generic already-dead setup used only to mark an entity as dead -> `DeathCause::CombatWounds` unless the test explicitly models need-deprivation semantics

This includes the direct production sites in:

- `crates/worldwake-sim/src/tick_step.rs`
- `crates/worldwake-systems/src/combat.rs`

and all affected helper/test/golden-harness sites across the workspace that currently construct or compare `DeadAt(...)`.

### 6. Add EventTag::Death variant

In `crates/worldwake-core/src/event_tag.rs`, add `Death` variant to the enum. Update `ALL_EVENT_TAGS` const array to include `EventTag::Death` and change the count assertion from 24 to 25.

## Files to Touch

- `crates/worldwake-core/src/needs.rs` (modify -- Hash derive on HomeostaticNeedId)
- `crates/worldwake-core/src/combat.rs` (modify -- DeathCause enum, DeadAt restructure)
- `crates/worldwake-core/src/lib.rs` (modify -- re-export DeathCause)
- `crates/worldwake-core/src/event_tag.rs` (modify -- Death variant + test updates)
- `crates/worldwake-sim/src/tick_step.rs` (modify -- production DeadAt construction plus affected assertions/tests)
- `crates/worldwake-systems/src/combat.rs` (modify -- combat fatality construction plus affected assertions/tests)
- Additional test/helper files across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, and `worldwake-ai` with direct `DeadAt(...)` construction or equality assertions, as required by the shared shape change

## Out of Scope

- Need-based mortality logic (that is S81GLDGAP-003)
- Tagging existing events with EventTag::Death (that is S81GLDGAP-003)
- Golden tests (S81GLDGAP-004 through S81GLDGAP-006)

## Acceptance Criteria

### Tests That Must Pass

1. `event_tag_satisfies_required_traits` -- confirms EventTag still satisfies trait bounds
2. `event_tag_includes_all_required_variants` -- confirms count is 25
3. `event_tag_order_is_declaration_stable` -- confirms sort order
4. `event_tag_bincode_roundtrip_covers_every_variant` -- confirms serialization
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. `DeadAt` cannot be constructed without specifying a `DeathCause` (type-level invariant)
2. `DeathCause` is `Copy + Clone + Serialize + Deserialize` (required for component storage)
3. `EventTag::Death` sorts after `BladderAccident` in declaration order (append-only enum convention)
4. Direct `DeadAt` construction/equality fallout is updated so tuple-struct syntax no longer exists on the branch

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/event_tag.rs` tests -- update ALL_EVENT_TAGS const and count assertion
2. Update affected unit/golden assertions that compare `DeadAt` values to the named-field shape where needed

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace --no-run`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

- Added `Hash` to `HomeostaticNeedId`, introduced `DeathCause`, and reshaped `DeadAt` into a named-field struct carrying `tick` plus `cause`.
- Re-exported `DeathCause` from `worldwake-core` and added `EventTag::Death` to the stable tag inventory/tests.
- Updated all live `DeadAt(...)` construction, equality, pattern-match, and field-access fallout across `worldwake-core`, `worldwake-sim`, `worldwake-systems`, `worldwake-ai`, and `worldwake-cli` so the tuple shape no longer exists on the branch.

## Deviations

- Reassessment showed the original ticket under-scoped the shared-type fallout. Direct constructor and assertion updates outside `worldwake-core` were absorbed because removing the tuple struct without those edits would leave the workspace uncompilable.

## Verification Result

- Passed `cargo test -p worldwake-core`
- Passed `cargo test --workspace --no-run`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
