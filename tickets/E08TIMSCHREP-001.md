# E08TIMSCHREP-001: SystemId type and fixed system-order manifest

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — new type in worldwake-sim
**Deps**: E07 (action framework complete)

## Problem

The scheduler needs a stable, deterministic ordering of registered simulation systems. Without a `SystemId` type and a fixed manifest, system execution order could drift based on registration order or hash-map iteration, violating determinism (Spec 9.2).

## Assumption Reassessment (2026-03-09)

1. `worldwake-sim` exists with action framework types — confirmed via `crates/worldwake-sim/src/lib.rs`
2. No `SystemId` type exists yet — confirmed, no hits in sim crate
3. `ActionInstanceId` uses `u64` inner — confirmed in `action_ids.rs`

## Architecture Check

1. `SystemId` is a simple newtype over `u32` following the same pattern as `ActionDefId` — minimal, consistent
2. The system-order manifest is a `Vec<SystemId>` defined once, not dynamically registered — prevents nondeterminism by construction

## What to Change

### 1. New type: `SystemId`

Define `SystemId(u32)` with the same derive set as `ActionDefId`: `Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize, Display`. Use the `action_id_type!` macro or a parallel pattern.

### 2. New type: `SystemManifest`

A thin wrapper around `Vec<SystemId>` that enforces:
- No duplicate `SystemId` entries
- Immutable after construction (built once, queried many times)
- Provides `ordered_ids() -> &[SystemId]` accessor

## Files to Touch

- `crates/worldwake-sim/src/system_id.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify — add module + re-export)

## Out of Scope

- Actual system registration or dispatch logic (that's E08TIMSCHREP-006)
- Mapping `SystemId` to concrete system functions (deferred to per-tick flow ticket)
- Any systems crate changes

## Acceptance Criteria

### Tests That Must Pass

1. `SystemId` satisfies `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + Deserialize`
2. `SystemId` bincode round-trips correctly
3. `SystemId` display format is stable (e.g., `"sys3"`)
4. `SystemManifest` rejects duplicate `SystemId` entries
5. `SystemManifest` preserves insertion order via `ordered_ids()`
6. `SystemManifest` is serializable and round-trips through bincode
7. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. `SystemId` ordering is deterministic (derived `Ord` on inner `u32`)
2. No `HashMap` or `HashSet` in any new type — `BTreeSet` for duplicate detection only if needed

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/system_id.rs` (inline `#[cfg(test)]` module) — trait bounds, display, bincode, manifest invariants

### Commands

1. `cargo test -p worldwake-sim system_id`
2. `cargo clippy --workspace && cargo test --workspace`
