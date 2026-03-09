# E07ACTFRA-001: Sim Crate Bootstrap + Action IDs + ActionStatus

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — establishes worldwake-sim module structure
**Deps**: E06 (event log complete in worldwake-core)

## Problem

The `worldwake-sim` crate is empty (only a doc comment in `lib.rs`, no dependencies in `Cargo.toml`). Before any action framework work can begin, the crate needs its dependency on `worldwake-core` and the foundational ID/status types the rest of E07 builds on.

## Assumption Reassessment (2026-03-09)

1. `worldwake-sim/Cargo.toml` has zero dependencies — confirmed.
2. `worldwake-sim/src/lib.rs` contains only a doc comment — confirmed.
3. Existing ID types (`EntityId`, `EventId`, `ReservationId`, `Tick`) live in `worldwake-core/src/ids.rs` and follow a consistent `NewType(inner)` pattern with `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + Deserialize` — confirmed.
4. `EventTag` already has `ActionStarted`, `ActionCommitted`, `ActionAborted` variants — confirmed, no new tags needed for this ticket.

## Architecture Check

1. Action IDs live in `worldwake-sim` (not core) because they are specific to the action framework, which is sim's responsibility. Systems and AI crates already depend on sim per the architecture diagram.
2. Follows the exact same newtype-with-traits pattern established in `worldwake-core/src/ids.rs`.

## What to Change

### 1. Update `worldwake-sim/Cargo.toml`

Add dependencies:
- `worldwake-core = { path = "../worldwake-core" }`
- `serde = { version = "1", features = ["derive"] }`
- `bincode = "1"` (dev-dependency for round-trip tests)

### 2. Create `worldwake-sim/src/action_ids.rs`

Define three newtype IDs following core's pattern:
- `ActionDefId(u32)` — indexes into the static action definition registry
- `ActionHandlerId(u32)` — indexes into the handler registry
- `ActionInstanceId(u64)` — monotonic ID for active action instances

All must derive: `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize`.
All must impl `Display`.

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

## Acceptance Criteria

### Tests That Must Pass

1. `ActionDefId`, `ActionHandlerId`, `ActionInstanceId` satisfy `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + DeserializeOwned` (compile-time trait assertion)
2. `ActionStatus` satisfies the same trait bounds
3. All four types survive bincode round-trip
4. `ActionDefId` display format is consistent (e.g., `"adef3"`)
5. `ActionStatus` has exactly 5 variants
6. Existing suite: `cargo test --workspace`

### Invariants

1. No `HashMap` or `HashSet` used anywhere in new code
2. All new types are deterministic and serializable
3. IDs are monotonic — no constructor randomizes or reuses values

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/action_ids.rs` — trait bound assertions, display, bincode round-trip for each ID type
2. `crates/worldwake-sim/src/action_status.rs` — trait bound assertions, variant count, bincode round-trip

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`
