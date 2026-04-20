# S110DECHISEVE-001: Relocate MaterializationTag to worldwake-core

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — cross-crate type relocation, no behavioral change
**Deps**: None

## Problem

`MaterializationTag` currently lives in `crates/worldwake-sim/src/action_handler.rs` and is consumed by `worldwake-ai` and `worldwake-systems`. S110's new `ExpectationMismatchPayload` stores `Vec<MaterializationTag>` on `EventPayload`, which lives in `worldwake-core`. Core cannot depend on sim (workspace layering is `core → sim → systems → ai → cli`), so the tag must move down to core. This ticket relocates the type without changing its shape or any behavior, and preserves source compatibility for every existing consumer through a re-export.

## Assumption Reassessment (2026-04-20)

1. `MaterializationTag` is defined as a single-variant `Copy + Clone + Eq + PartialEq + Ord + PartialOrd + Hash + Serialize + Deserialize` enum at `crates/worldwake-sim/src/action_handler.rs:38`. Single variant: `SplitOffLot`. Companion struct `Materialization { tag: MaterializationTag, entity: EntityId }` at `action_handler.rs:32` stays in sim — it is not referenced by the event-log payload path.
2. Workspace-wide grep shows 29 `MaterializationTag` reference sites distributed across `crates/worldwake-ai/src` (including `agent_tick` and `search` subdirectories), `crates/worldwake-sim/src`, and `crates/worldwake-systems/src`. Every consumer imports by name; no consumer pattern-matches outside the `SplitOffLot` variant that would require a new match arm after relocation.
3. Shared abstraction boundary under audit: the `MaterializationTag` symbol identity. After relocation, the type is defined once in `worldwake-core`, re-exported from `worldwake-sim` at the same module path (`worldwake_sim::action_handler::MaterializationTag`), and the symbol resolves identically for every current consumer. No duplicate type lives simultaneously at both locations.
14. No mismatch or correction — the spec's D4 deliverable describes the relocation accurately after reassessment; ticket scope matches the spec.

## Architecture Check

1. Moving the type down the crate layering (sim → core) rather than up (forcing core to depend on sim) preserves FND-26's system-decoupling constraint: `worldwake-core` must remain dependency-free of sim/systems/ai so that event-log payload types can reference it from any crate. A re-export in sim is cleaner than duplicating the type or threading a generic parameter through `EventPayload`.
2. No backwards-compatibility shim is introduced. The re-export is an idiomatic Rust visibility alias, not a deprecated alias path — the type has a single canonical definition in core; sim exposes the same symbol under the historical import path so no consumer code changes.

## Verification Layers

1. Type identity invariant (`worldwake_sim::action_handler::MaterializationTag == worldwake_core::MaterializationTag`) → compile-time check; the workspace build fails if the re-export does not resolve or if two definitions coexist.
2. Consumer compatibility → `cargo build --workspace` and `cargo test --workspace` confirm every current importer still resolves the symbol.
6. Single-layer ticket — this is a pure relocation with no runtime behavior change, so layer mapping beyond compile-time identity is not applicable.

## What to Change

### 1. Define `MaterializationTag` in `worldwake-core`

Create `crates/worldwake-core/src/materialization_tag.rs` containing the relocated enum with identical derives:

```rust
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum MaterializationTag {
    SplitOffLot,
}
```

Register the module in `crates/worldwake-core/src/lib.rs` and add `pub use materialization_tag::MaterializationTag;` to the re-export list.

### 2. Remove the original definition from `worldwake-sim`

In `crates/worldwake-sim/src/action_handler.rs`, remove the enum definition and import the type from core instead: `use worldwake_core::MaterializationTag;`. The adjacent `Materialization` struct remains in sim and continues to use `MaterializationTag` via the import.

### 3. Re-export from `worldwake-sim` for source compatibility

In `crates/worldwake-sim/src/action_handler.rs`, add `pub use worldwake_core::MaterializationTag;` at the module level so existing consumers (`worldwake_sim::action_handler::MaterializationTag`, `worldwake_sim::MaterializationTag` via `lib.rs` re-export) continue to resolve. In `crates/worldwake-sim/src/lib.rs`, if the type is re-exported at the crate root, keep that re-export unchanged — it now points to the core definition transparently.

### 4. No consumer changes required

Consumers in `worldwake-ai` and `worldwake-systems` continue to import `MaterializationTag` via `worldwake_sim::action_handler` or whatever path they currently use. No import-line edits needed in those crates. If a consumer currently imports via `worldwake_sim::MaterializationTag` and the root re-export is kept, that too continues to work.

## Files to Touch

- `crates/worldwake-core/src/materialization_tag.rs` (new)
- `crates/worldwake-core/src/lib.rs` (modify — module declaration and re-export)
- `crates/worldwake-sim/src/action_handler.rs` (modify — remove local definition, add `use` and `pub use` from core)

## Out of Scope

- Moving `Materialization` (the struct wrapping `MaterializationTag` with an `EntityId`). It stays in sim; it is not referenced by core event-log types.
- Moving `ExpectedMaterialization` (`worldwake-ai/src/planner_ops.rs:808`). It wraps `HypotheticalEntityId` which is AI-internal and cannot live in core.
- Adding new `MaterializationTag` variants. Single variant `SplitOffLot` is preserved as-is.
- Any changes to serialization format — the enum's serde derives and wire shape are unchanged, so `SAVE_FORMAT_VERSION` does not bump for this ticket alone.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo build --workspace` — every crate resolves `MaterializationTag` without duplicate-definition or unresolved-import errors.
2. Existing tests that reference `MaterializationTag` (e.g., any planner_ops test constructing `ExpectedMaterialization { tag: MaterializationTag::SplitOffLot, .. }`) continue to pass unchanged.
3. Existing suite: `cargo test --workspace`
4. Clippy clean: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. `MaterializationTag` has exactly one canonical definition (in `worldwake-core`); all other references are re-exports or imports of that single definition.
2. No consumer code outside the three touched files changes — the re-export preserves every historical import path byte-for-byte.

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket from a test-surface perspective; verification is command-based and the existing workspace test suite exercises `MaterializationTag` at its consumer sites.

### Commands

1. `cargo build --workspace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-20.

- Moved the canonical `MaterializationTag` definition into `crates/worldwake-core/src/materialization_tag.rs`.
- Registered and re-exported `MaterializationTag` from `worldwake-core::lib`, so core-owned decision-event payload work can reference it without violating crate layering.
- Removed the local enum definition from `crates/worldwake-sim/src/action_handler.rs` and preserved the historical sim import path with `pub use worldwake_core::MaterializationTag;`.
- No downstream consumer edits were required; existing `worldwake-sim` root and module re-exports continued to resolve the same symbol identity.

## Verification Result

- Passed `cargo build --workspace`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
