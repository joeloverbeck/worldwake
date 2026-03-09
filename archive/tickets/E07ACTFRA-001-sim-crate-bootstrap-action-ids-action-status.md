# E07ACTFRA-001: Sim Crate Bootstrap + Action IDs + ActionStatus

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — establishes worldwake-sim module structure
**Deps**: E06 (event log complete in worldwake-core)

## Problem

The `worldwake-sim` crate is still only a crate doc plus an empty dependency list. E07 needs a minimal sim-side bootstrap before the action framework can grow: the crate must depend on `worldwake-core`, expose public modules, and define the action-specific ID/status value types that later tickets will compose into registries, affordances, and active instances.

## Assumption Reassessment (2026-03-09)

1. `worldwake-sim/Cargo.toml` has zero dependencies — confirmed.
2. `worldwake-sim/src/lib.rs` contains only a doc comment — confirmed.
3. Existing ID types live in `worldwake-core/src/ids.rs`, but they do not all follow a single `NewType(inner)` shape: most are tuple newtypes, while `EntityId` and `TickRange` are structured types. The real shared convention is deterministic value semantics (`Copy` where appropriate, stable ordering, `Display`, `Serialize`, `Deserialize`) rather than one exact layout — corrected.
4. `worldwake-core::event_tag::EventTag` already includes `ActionStarted`, `ActionCommitted`, and `ActionAborted` — confirmed, so this ticket should not introduce duplicate event tags.
5. `cargo test -p worldwake-sim` currently runs zero tests, while workspace policy checks such as `no_hash_map` and `no_hash_set` live in `worldwake-core/tests/policy.rs` — confirmed.
6. The workspace currently passes `cargo test --workspace`; this ticket must preserve that state while adding focused `worldwake-sim` coverage — confirmed.

## Architecture Check

1. Action IDs live in `worldwake-sim` (not core) because they are specific to the action framework, which is sim's responsibility. Systems and AI crates already depend on sim per the architecture diagram.
2. The important architectural match with `worldwake-core` is deterministic, serializable value types with explicit formatting, not forcing every ID into one identical internal representation.
3. `ActionStatus` belongs in `worldwake-sim` because it describes lifecycle state for active actions; keeping it close to the action framework avoids leaking sim-specific execution concepts into `worldwake-core`.
4. This bootstrap ticket should stay narrowly about durable public types and module wiring. It should not speculate about registries, id allocators, or lifecycle logic that later E07 tickets own.

## What to Change

### 1. Update `worldwake-sim/Cargo.toml`

Add dependencies:
- `worldwake-core = { path = "../worldwake-core" }`
- `serde = { version = "1", features = ["derive"] }`
- `bincode = "1"` as a dev-dependency for round-trip tests

### 2. Create `worldwake-sim/src/action_ids.rs`

Define three newtype IDs following core's pattern:
- `ActionDefId(u32)` — indexes into the static action definition registry
- `ActionHandlerId(u32)` — indexes into the handler registry
- `ActionInstanceId(u64)` — monotonic ID for active action instances

All must derive: `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`.
All must impl `Display`.

Formatting should be explicit and stable:
- `ActionDefId(3)` -> `"adef3"`
- `ActionHandlerId(7)` -> `"ah7"`
- `ActionInstanceId(11)` -> `"ai11"`

### 3. Create `worldwake-sim/src/action_status.rs`

Define:
```rust
enum ActionStatus {
    Pending,
    Active,
    Committed,
    Aborted,
    Interrupted,
}
```

Must derive: `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`.

Expose a canonical ordered variant list in tests rather than relying on ad hoc counting logic.

### 4. Update `worldwake-sim/src/lib.rs`

Declare modules and re-export public types.

## Files to Touch

- `crates/worldwake-sim/Cargo.toml` (modify)
- `crates/worldwake-sim/src/action_ids.rs` (new)
- `crates/worldwake-sim/src/action_status.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- ActionDef, ActionInstance, or any other compound types
- Handler registry or execution logic
- KnowledgeView, Affordance, or any query API
- Any changes to `worldwake-core`
- Semantic types (Constraint, Precondition, etc.)
- Allocator logic for issuing monotonic action IDs; this ticket defines the ID types only

## Acceptance Criteria

### Tests That Must Pass

1. `ActionDefId`, `ActionHandlerId`, `ActionInstanceId` satisfy `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + DeserializeOwned` (compile-time trait assertion)
2. `ActionStatus` satisfies the same trait bounds
3. All four types survive bincode round-trip
4. All three ID displays are explicit and stable (`"adef3"`, `"ah7"`, `"ai11"`)
5. `ActionStatus` exposes exactly 5 declaration-ordered variants and sorts in declaration order
6. Existing suite: `cargo test --workspace`

### Invariants

1. No `HashMap` or `HashSet` used anywhere in new code
2. All new types are deterministic and serializable
3. No compatibility aliases or duplicate type definitions are introduced; later E07 tickets should build on these exact public types

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_ids.rs` — trait bound assertions, display, bincode round-trip for each ID type
2. `crates/worldwake-sim/src/action_status.rs` — trait bound assertions, declaration-order check, canonical variant list coverage, bincode round-trip

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy -p worldwake-sim --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-09
- What actually changed:
  - Added `worldwake-core` and `serde` as `worldwake-sim` dependencies plus `bincode` as a test-only dev-dependency
  - Added `ActionDefId`, `ActionHandlerId`, and `ActionInstanceId` in `crates/worldwake-sim/src/action_ids.rs`
  - Added `ActionStatus` in `crates/worldwake-sim/src/action_status.rs`
  - Registered and re-exported the new public modules from `crates/worldwake-sim/src/lib.rs`
  - Added colocated unit tests covering trait bounds, stable display formatting, declaration-order stability, and bincode round-trip behavior
- Deviations from original plan:
  - Corrected the ticket assumptions before implementation to match the actual codebase conventions: core ID types do not share one exact internal shape, and workspace policy checks already cover `HashMap`/`HashSet` bans outside `worldwake-sim`
  - Strengthened the display acceptance criteria from one example (`ActionDefId`) to all three action ID types so later registry/debug output stays explicit and consistent
  - Kept monotonic ID allocation out of scope because this ticket defines value types only; allocation policy belongs to later lifecycle/registry work
- Verification results:
  - `cargo test -p worldwake-sim` passed
  - `cargo clippy -p worldwake-sim --all-targets -- -D warnings` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace --all-targets -- -D warnings` passed
  - `cargo fmt --all --check` passed
