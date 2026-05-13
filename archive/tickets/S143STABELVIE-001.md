# S143STABELVIE-001: Foundation read-shape types in `worldwake-sim` and `worldwake-core`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None (net-new types, no consumers yet)
**Deps**: spec `archive/specs/S143-static-belief-view-trait-separation.md`

## Problem

Tickets 002–006 needed four foundation types before any trait or migration work could land: `BeliefRead<T>` (the unified epistemic-read enum wrapping `BeliefValue<T>` per spec D1), `ObservedRead<T>` and `ObservationSource` (the co-located physical observation wrapper, also D1), and `EntityState` (the authoritative-state snapshot consumed by `DebugWorldView`, D4). Before this ticket, none of these existed in `crates/`, so the new trait signatures in ticket 002 could not compile.

## Assumption Reassessment (2026-05-13)

1. `BeliefValue<T>` lives at `crates/worldwake-sim/src/belief_view.rs:32` (struct with `value, confidence, acquired_tick, claimed_event_tick, status` fields), NOT in `worldwake-core` as the spec's Crates row 1 implied. `worldwake-core` has zero `BeliefValue<T>` consumers; `worldwake-sim` has 2 files, `worldwake-ai` has 6 files. Spec drafting assumed core placement; codebase truth places it in sim. `BeliefRead::Known(BeliefValue<T>)` requires `BeliefValue<T>` to be reachable, so `BeliefRead<T>` lands in `worldwake-sim/src/belief_view.rs` next to `BeliefValue<T>` rather than in `worldwake-core`.
2. `EntityState` references only `EntityKind` and `EntityId` (both in `worldwake-core`); placing it in `worldwake-core` keeps the spec's D4 framing intact. `worldwake-core/src/world/` is a directory module (`lifecycle.rs`, `ownership.rs`, `placement.rs`, etc.), not a single file — a new sibling module `crates/worldwake-core/src/debug_view.rs` is the cleanest placement.
3. Sub-check (d): zero existing struct literal or construction sites for any of the four new types (`grep -rn "EntityState\|BeliefRead\|ObservedRead\|ObservationSource" crates/` returns only the spec file). Pure additions with no blast radius.
4. Adjacent contradiction (was item 13): spec Crates row 1 claimed `worldwake-core` defines `BeliefRead<T>`. Classification: required consequence — corrected here because workspace layering forbids `worldwake-core → worldwake-sim`, not because of a separate bug. The Crates row and D1 placement snippet in `archive/specs/S143-static-belief-view-trait-separation.md` were updated to reflect the actual placement.
5. Mismatch + correction (was item 14): spec D1 places `BeliefRead<T>` and `ObservedRead<T>` in `worldwake-core/src/belief.rs (or a new sibling module)`. Correction: `BeliefRead<T>`, `ObservedRead<T>`, and `ObservationSource` land in `worldwake-sim/src/belief_view.rs` next to `BeliefValue<T>`. `EntityState` lands in a new `crates/worldwake-core/src/debug_view.rs`. The spec's intent (foundation types available to dependent tickets) is preserved.

## Architecture Check

1. Co-locating `BeliefRead<T>` with `BeliefValue<T>` keeps the belief read-shape family in one module. Future refactors of either type touch a single file.
2. `EntityState` in `worldwake-core/src/debug_view.rs` is reachable by `worldwake-sim`'s `DebugWorldView` trait (added in ticket 002) without crossing workspace layering (`sim → core` is allowed).
3. No backwards-compatibility shims; all four types are net-new.

## Verified Layers

1. Type definitions compile through `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.
2. `BeliefRead::Unknown`, `BeliefRead::Known(BeliefValue { … })`, `BeliefRead::Stale(BeliefValue { … })`, and `ObservedRead { value, observed_tick, source }` are covered by focused unit tests.
3. `EntityState::default()` and explicit field assignment are covered by focused unit tests.
4. Single-layer ticket (type addition only); broader layer mapping is N/A.

## Landed Changes

### 1. Read-shape types in `worldwake-sim/src/belief_view.rs`

Added near `BeliefValue<T>` in `crates/worldwake-sim/src/belief_view.rs`:

```rust
pub enum BeliefRead<T> {
    Unknown,
    Known(BeliefValue<T>),
    Stale(BeliefValue<T>),
}

pub struct ObservedRead<T> {
    pub value: T,
    pub observed_tick: Tick,
    pub source: ObservationSource,
}

pub enum ObservationSource {
    CoLocatedSameTick,
    BeliefStoreSnapshot,
}
```

Derives: `BeliefRead<T>` and `ObservedRead<T>` derive `Debug, Clone` (matching `BeliefValue<T>`'s derive set where T's bounds permit). `ObservationSource` derives `Copy, Clone, Debug, Eq, PartialEq, Hash`.

### 2. `EntityState` in `crates/worldwake-core/src/debug_view.rs`

```rust
pub struct EntityState {
    pub kind: Option<EntityKind>,
    pub place: Option<EntityId>,
    pub alive: bool,
    pub container: Option<EntityId>,
    pub possessor: Option<EntityId>,
}
```

Derives: `Debug, Clone, Default, Eq, PartialEq`.

### 3. Re-exports

- `crates/worldwake-core/src/lib.rs`: added `pub mod debug_view;` and `pub use debug_view::EntityState;`.
- `crates/worldwake-sim/src/lib.rs`: extended the existing `pub use belief_view::{…};` list to include `BeliefRead, ObservedRead, ObservationSource`.

## Landed Files

- `crates/worldwake-sim/src/belief_view.rs` — added read-shape types and focused tests.
- `crates/worldwake-sim/src/lib.rs` — extended the `belief_view` re-export list.
- `crates/worldwake-core/src/debug_view.rs` — added `EntityState` and focused tests.
- `crates/worldwake-core/src/lib.rs` — added `pub mod debug_view;` and re-exported `EntityState`.
- `archive/specs/S143-static-belief-view-trait-separation.md` — corrected read-shape type placement.
- `specs/IMPLEMENTATION-ORDER.md` — corrected the S143 trait label.

## Out of Scope

- No new traits (deferred to ticket 002).
- No method migrations (tickets 003 and 004).
- No changes to existing types — `BeliefValue<T>`, `BeliefSet<T>`, `BeliefStatus` remain in `worldwake-sim/src/belief_view.rs` unchanged.
- No move of `BeliefValue<T>` to `worldwake-core` — out-of-scope per Non-Goals; tracked here for future reassessment if the type's foundational role warrants the migration.

## Acceptance Result

### Focused Proof

1. Added `belief_view::tests::belief_read_encodes_unknown_known_and_stale`.
2. Added `belief_view::tests::observed_read_carries_tick_and_source`.
3. Added `debug_view::tests::entity_state_default_is_empty_and_not_alive`.
4. Added `debug_view::tests::entity_state_preserves_authoritative_snapshot_fields`.
5. Existing suite passed via `cargo test --workspace`.

### Invariants

1. `BeliefRead<T>` encodes exactly three epistemic states (Unknown, Known, Stale) — no fourth state and no other meaning encoded.
2. `ObservedRead<T>` carries `observed_tick` and `source` provenance; the wrapped `value` is the raw observation, never a wrapped or processed form.
3. `EntityState` is a pure data snapshot — no methods that read live world state at use time.

## Added Tests

1. `crates/worldwake-sim/src/belief_view.rs` `#[cfg(test)]` block — focused tests for `BeliefRead<T>`, `ObservedRead<T>`, and `ObservationSource`.
2. `crates/worldwake-core/src/debug_view.rs` `#[cfg(test)]` block — focused tests for `EntityState` default and field assignment.

## Outcome

Completed on 2026-05-13.

- Added `BeliefRead<T>`, `ObservedRead<T>`, and `ObservationSource` in `crates/worldwake-sim/src/belief_view.rs` beside `BeliefValue<T>`.
- Added `EntityState` in `crates/worldwake-core/src/debug_view.rs`.
- Re-exported `EntityState` from `worldwake-core` and the read-shape types from `worldwake-sim`.
- Updated `archive/specs/S143-static-belief-view-trait-separation.md` so the Crates row and D1 snippet match the live placement.
- Updated the S143 row in `specs/IMPLEMENTATION-ORDER.md` to name `BelievedAuthorityView` instead of the stale `BelievedSocialView` label.

## Deviations

- The spec draft originally placed `BeliefRead<T>` in `worldwake-core`; reassessment kept it in `worldwake-sim` because it wraps `BeliefValue<T>`, which is owned by `worldwake-sim`, and `worldwake-core` cannot depend on `worldwake-sim`.

## Verification Result

- Passed `cargo test -p worldwake-sim --lib belief_view::tests::belief_read_encodes_unknown_known_and_stale -- --exact`
- Passed `cargo test -p worldwake-sim --lib belief_view::tests::observed_read_carries_tick_and_source -- --exact`
- Passed `cargo test -p worldwake-core --lib debug_view::tests::entity_state_default_is_empty_and_not_alive -- --exact`
- Passed `cargo test -p worldwake-core --lib debug_view::tests::entity_state_preserves_authoritative_snapshot_fields -- --exact`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
