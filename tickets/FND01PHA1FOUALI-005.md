# FND01PHA1FOUALI-005: Constrain Loyalty Mutations with Doc-Comments and Regression Tests

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — documentation and verification only, no behavioral changes
**Deps**: None (independent)

## Problem

`LoyalTo.strength: Permille` is a scalar disposition. While scalars are not banned outright, they require constraints to prevent script-like threshold logic ("if loyalty < 400 then betray") that bypasses emergent decision-making. Currently, `set_loyalty()` and `clear_loyalty()` lack documentation of these constraints, and there are no regression tests verifying that loyalty mutations are event-sourced via `RelationDelta`.

## Assumption Reassessment (2026-03-10)

1. `World::set_loyalty()` at `world/social.rs:78-94` — confirmed. Calls `Self::set_weighted_relation()`. Visibility: `pub(crate)`.
2. `World::clear_loyalty()` at `world/social.rs:96-110` — confirmed. Calls `Self::clear_weighted_relation()`. Visibility: `pub(crate)`.
3. `WorldTxn::set_loyalty()` at `world_txn.rs:440-451` — confirmed. Calls `self.push_weighted_relation_delta(subject, target, before, after)` — event-sourcing is already in place.
4. `WorldTxn::clear_loyalty()` at `world_txn.rs:453-459` — confirmed. Same pattern — calls `push_weighted_relation_delta`.
5. `RelationDelta` types exist for tracking relation changes — confirmed from delta module.
6. No existing test explicitly verifies that loyalty set/clear produces `RelationDelta` entries — this is the gap to fill.

## Architecture Check

1. This ticket adds documentation constraints and regression tests only — no behavioral changes.
2. The event-sourcing mechanism already works (via `push_weighted_relation_delta`). We are adding explicit tests to prevent regressions and doc-comments to guide future implementers.
3. Constraints documented here will be enforced by E14 (perception) and AI crate (decision architecture).

## What to Change

### 1. Add doc-comments to `World::set_loyalty()`

In `world/social.rs`, add doc-comment above `set_loyalty()`:

```rust
/// Sets the loyalty relation from `subject` toward `target` with the given strength.
///
/// # Constraints (FND-01, Principle 1 & 3)
///
/// - Initial loyalty values MUST come from seeded agent traits, background, or bootstrap events.
/// - All runtime changes MUST flow through `WorldTxn::set_loyalty()` to ensure event-sourcing
///   via `RelationDelta` recording.
/// - No system may use loyalty as a direct threshold for scripted behavior
///   (e.g., "if loyalty < 400 then betray"). Decisions involving loyalty MUST flow through
///   the agent's beliefs, goals, and utility evaluation.
```

### 2. Add doc-comments to `World::clear_loyalty()`

Same pattern:

```rust
/// Removes the loyalty relation from `subject` toward `target`.
///
/// # Constraints (FND-01, Principle 1 & 3)
///
/// - All removals MUST flow through `WorldTxn::clear_loyalty()` to ensure event-sourcing
///   via `RelationDelta` recording.
/// - Loyalty removal should emerge from world events (betrayal, abandonment, death),
///   not from arbitrary script triggers.
```

### 3. Add doc-comments to `WorldTxn::set_loyalty()` and `WorldTxn::clear_loyalty()`

In `world_txn.rs`:

```rust
/// Sets loyalty and records a `RelationDelta` for event-sourcing.
/// See `World::set_loyalty()` for constraint documentation.
```

```rust
/// Clears loyalty and records a `RelationDelta` for event-sourcing.
/// See `World::clear_loyalty()` for constraint documentation.
```

### 4. Add regression test: set_loyalty produces RelationDelta

Create a test that:
1. Creates a world with two agents.
2. Opens a `WorldTxn`.
3. Calls `txn.set_loyalty(agent_a, agent_b, Permille::new(500).unwrap())`.
4. Commits the transaction.
5. Inspects the committed event's deltas for a `RelationDelta::Added` with `RelationKind::LoyalTo` (or equivalent weighted relation delta).

### 5. Add regression test: clear_loyalty produces RelationDelta

Create a test that:
1. Creates a world with two agents that have an existing loyalty relation.
2. Opens a `WorldTxn`.
3. Calls `txn.clear_loyalty(agent_a, agent_b)`.
4. Commits the transaction.
5. Inspects the committed event's deltas for a `RelationDelta::Removed` with `RelationKind::LoyalTo` (or equivalent).

## Files to Touch

- `crates/worldwake-core/src/world/social.rs` (modify — add doc-comments)
- `crates/worldwake-core/src/world_txn.rs` (modify — add doc-comments + regression tests)

## Out of Scope

- Do NOT change loyalty mutation behavior (already correct).
- Do NOT modify `Permille`, `RelationDelta`, or `RelationTables`.
- Do NOT add validation logic that rejects loyalty values (future concern).
- Do NOT implement belief-based loyalty evaluation (that's E14 + AI crate).
- Do NOT touch `WorldTxn::set_loyalty()` or `clear_loyalty()` implementation code.

## Acceptance Criteria

### Tests That Must Pass

1. `set_loyalty()` and `clear_loyalty()` on `World` have doc-comments stating the FND-01 constraints.
2. `set_loyalty()` and `clear_loyalty()` on `WorldTxn` have doc-comments referencing constraint documentation.
3. Regression test: `WorldTxn::set_loyalty()` commit produces a relation delta for loyalty.
4. Regression test: `WorldTxn::clear_loyalty()` commit produces a relation delta for loyalty removal.
5. Existing suite: `cargo test -p worldwake-core`
6. Full suite: `cargo test --workspace`
7. `cargo clippy --workspace` clean.

### Invariants

1. No behavioral code changes — loyalty mutation logic is identical before and after.
2. All existing relation and social tests pass unchanged.
3. Event-sourcing via `RelationDelta` is verified, not assumed.

## Test Plan

### New/Modified Tests

1. `world_txn.rs::set_loyalty_via_txn_produces_relation_delta` — new regression test.
2. `world_txn.rs::clear_loyalty_via_txn_produces_relation_delta` — new regression test.

### Commands

1. `cargo test -p worldwake-core -- loyalty`
2. `cargo test -p worldwake-core -- world_txn`
3. `cargo test --workspace && cargo clippy --workspace`
