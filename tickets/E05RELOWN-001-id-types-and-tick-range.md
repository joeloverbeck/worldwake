# E05RELOWN-001: ReservationId, FactId, and TickRange types

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: None
**Deps**: E03 (entity store — completed), E04 (items — completed)

## Problem

The relation layer needs stable identity types for reservations and knowledge facts, plus a half-open tick interval type for reservation windows. Without these foundational types, no reservation or knowledge relation can be stored.

## Assumption Reassessment (2026-03-09)

1. `ids.rs` already defines `EntityId`, `Tick`, `EventId`, `TravelEdgeId`, `Seed` — confirmed
2. No existing `ReservationId` or `FactId` types — confirmed via codebase scan
3. `Tick` already supports `Add<u64>`, `Sub<u64>`, `Ord` — confirmed, sufficient for range comparisons
4. The crate enforces deterministic-authoritative-state policy — confirmed
5. All ID types follow the pattern `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + Deserialize` — confirmed

## Architecture Check

1. `ReservationId(u64)` and `FactId(u64)` follow the existing newtype ID pattern in `ids.rs`
2. `TickRange { start: Tick, end: Tick }` is a pure value type that belongs in `ids.rs` alongside `Tick`
3. No allocator or counter is introduced here — ID generation will be handled by the relation tables that consume these types

## What to Change

### 1. Add `ReservationId` and `FactId` to `ids.rs`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct ReservationId(pub u64);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct FactId(pub u64);
```

With `Display` impls following the existing `"r{}"` / `"f{}"` pattern.

### 2. Add `TickRange` to `ids.rs`

```rust
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct TickRange {
    pub start: Tick,
    pub end: Tick,
}
```

With:
- Constructor `TickRange::new(start, end) -> Result<Self, &'static str>` that enforces `end > start`
- `TickRange::overlaps(&self, other: &TickRange) -> bool` using half-open `[start, end)` semantics
- `TickRange::contains_tick(&self, tick: Tick) -> bool`

### 3. Re-export from `lib.rs`

Add `ReservationId`, `FactId`, `TickRange` to the pub use list from `ids`.

## Files to Touch

- `crates/worldwake-core/src/ids.rs` (modify)
- `crates/worldwake-core/src/lib.rs` (modify — add re-exports)

## Out of Scope

- Reservation storage or tables (E05RELOWN-002)
- Knowledge/belief propagation semantics (future epic)
- ID allocator/counter logic (handled by relation tables)
- Any relation storage or APIs

## Acceptance Criteria

### Tests That Must Pass

1. `ReservationId` and `FactId` satisfy `Copy + Clone + Eq + Ord + Hash + Debug + Display + Serialize + DeserializeOwned`
2. Both ID types round-trip through bincode
3. `TickRange::new(Tick(5), Tick(10))` succeeds; `TickRange::new(Tick(5), Tick(5))` and `TickRange::new(Tick(10), Tick(5))` fail
4. `TickRange::overlaps` returns true for `[3,7)` vs `[5,10)`, false for `[3,5)` vs `[5,10)` (adjacent = no conflict)
5. `TickRange::overlaps` is symmetric
6. `TickRange::contains_tick` returns true for `start`, false for `end` (half-open)
7. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. No `HashMap` or `HashSet` usage
2. All new types are deterministic and serializable
3. `TickRange` uses half-open `[start, end)` semantics — adjacent ranges do NOT overlap

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/ids.rs` (inline `#[cfg(test)]`) — trait bounds, Display, bincode round-trips, TickRange construction/overlap/contains

### Commands

1. `cargo test -p worldwake-core ids`
2. `cargo clippy --workspace && cargo test --workspace`
