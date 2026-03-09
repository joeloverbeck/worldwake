# E05RELOWN-009: Randomized invariant integration tests (T01, T04, T13)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: E05RELOWN-004, E05RELOWN-005, E05RELOWN-006, E05RELOWN-007, E05RELOWN-008 (all relation APIs complete)

## Problem

The spec requires three randomized invariant tests that stress the relation layer under adversarial random inputs. These are Phase 1 gate tests that must pass before proceeding to E06:

- **T01**: Randomized moves never produce multiple effective locations
- **T04**: Overlapping reservations for the same entity cannot both succeed
- **T13**: Randomized container nesting never produces cycles

## Assumption Reassessment (2026-03-09)

1. All placement, ownership, reservation, and social relation APIs exist after E05RELOWN-004 through E05RELOWN-008 — assumed
2. `test_utils::deterministic_seed()` provides a fixed seed — confirmed
3. `ChaCha8Rng` is the deterministic RNG — confirmed from project policy
4. `rand_chacha` is an existing dependency — confirmed from Cargo.toml

## Architecture Check

1. These are integration tests in `tests/` directory (not inline unit tests) because they exercise the full `World` API
2. Each test uses `ChaCha8Rng` seeded with `deterministic_seed()` for reproducibility
3. Tests perform hundreds of random operations and assert invariants hold after each operation
4. Tests should be parameterized across multiple seeds for broader coverage

## What to Change

### 1. Create `crates/worldwake-core/tests/relation_invariants.rs`

#### T01: Unique placement invariant
- Create a world with multiple places, agents, containers, items
- For N iterations (e.g., 200):
  - Randomly choose: move entity to ground, put into container, move container, remove from container
  - After each operation, assert every entity has exactly one `LocatedIn` entry
  - Assert `effective_place(entity)` returns `Some` for every placed entity
  - Assert no entity appears in `entities_at` for more than one place

#### T04: Reservation exclusivity
- Create a world with several entities
- For N iterations (e.g., 200):
  - Generate random `TickRange` windows
  - Attempt `try_reserve` on random entities
  - Track which reservations succeeded
  - Assert: no two successful reservations for the same entity have overlapping ranges
  - Randomly release some reservations and re-reserve

#### T13: Acyclic containment
- Create a world with multiple containers
- For N iterations (e.g., 200):
  - Randomly attempt to nest containers inside each other
  - Assert: `put_into_container` rejects any operation that would create a cycle
  - After all operations, walk every `ContainedBy` chain; assert no chain revisits a node
  - Assert max chain depth is bounded (sanity check)

### 2. Verify with multiple seeds

Run each test with at least 5 different seeds to increase confidence.

## Files to Touch

- `crates/worldwake-core/tests/relation_invariants.rs` (new)

## Out of Scope

- Non-randomized unit tests (already covered in E05RELOWN-004 through -008)
- T02, T03 (conservation — covered by E04)
- T05-T09 (action/event/replay — E06-E08)
- Performance benchmarks
- Soak tests (E22)

## Acceptance Criteria

### Tests That Must Pass

1. T01: 200+ random placement operations across 5 seeds → no entity ever has multiple effective locations
2. T04: 200+ random reservation attempts across 5 seeds → no overlapping reservations succeed for the same entity
3. T13: 200+ random container nesting attempts across 5 seeds → containment graph remains acyclic
4. All three tests are deterministic: same seed produces same operation sequence and outcome
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Spec 9.4: unique physical placement (T01)
2. Spec 9.8: reservation exclusivity (T04)
3. Spec 9.18: no circular containment (T13)
4. Determinism: tests use `ChaCha8Rng` with fixed seeds

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/tests/relation_invariants.rs` — T01, T04, T13 randomized invariant tests

### Commands

1. `cargo test -p worldwake-core --test relation_invariants`
2. `cargo clippy --workspace && cargo test --workspace`
