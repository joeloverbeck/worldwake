# E05RELOWN-009: Randomized invariant integration tests (T01, T04, T13)

**Status**: COMPLETED
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

1. All placement and reservation APIs needed for T01, T04, and T13 already exist in `worldwake-core` — confirmed (`set_ground_location`, `put_into_container`, `remove_from_container`, `move_container_subtree`, `effective_place`, `entities_effectively_at`, `ground_entities_at`, `try_reserve`, `release_reservation`)
2. `test_utils::deterministic_seed()` provides a fixed seed — confirmed
3. `ChaCha8Rng` is the deterministic RNG — confirmed from project policy
4. `rand_chacha` exists in the workspace lockfile and in `worldwake-sim`, but not yet in `worldwake-core`'s manifest — correction: this ticket must add the narrow test dependency needed to compile these randomized tests
5. Existing E05 relation coverage already lives mostly as inline unit tests in `crates/worldwake-core/src/world.rs` — confirmed
6. Current world behavior still allows live entities with no effective place until explicitly placed — confirmed by existing tests, so T01 must only assert the invariant for entities the test scenario has intentionally placed into the placement graph

## Architecture Check

1. Keep the existing inline unit tests in `world.rs`; add this file as an integration-style black-box invariant suite because it exercises only public `World` APIs and should not couple to private relation tables
2. Each test uses `ChaCha8Rng` seeded with `deterministic_seed()` for reproducibility
3. Tests perform hundreds of random operations and assert invariants hold after each operation
4. Tests should iterate across multiple derived seeds in a deterministic loop for broader coverage without introducing flaky behavior
5. These tests are additive, not a replacement for the existing focused deterministic tests already covering single-step behavior

## What to Change

### 1. Add test support dependency

- Add `rand_chacha = "0.3"` to `crates/worldwake-core` test-only dependencies if needed

### 2. Create `crates/worldwake-core/tests/relation_invariants.rs`

#### T01: Unique placement invariant
- Create a world with multiple places, agents, containers, items
- For N iterations (e.g., 200):
  - Randomly choose: move entity to ground, put into container, move container, remove from container
  - Track which entities are expected to be placed because the test has explicitly grounded them or inserted them under a placed container
  - After each operation, assert `effective_place(entity)` returns `Some` for every entity in that tracked placed set
  - Assert no tracked placed entity appears in `entities_effectively_at` for more than one place
  - Assert `ground_entities_at(place)` is always a subset of `entities_effectively_at(place)`
  - Assert container descendants share the same effective place as the root placed container after moves

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
  - Assert max chain depth never exceeds the number of live containers in the scenario

### 3. Verify with multiple seeds

Run each test across at least 5 deterministic seeds derived from `deterministic_seed()` to increase coverage without weakening reproducibility.

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

1. T01: 200+ random placement operations across 5 deterministic seeds → no explicitly placed entity ever has multiple effective locations
2. T04: 200+ random reservation attempts across 5 seeds → no overlapping reservations succeed for the same entity
3. T13: 200+ random container nesting attempts across 5 seeds → containment graph remains acyclic
4. All three tests are deterministic: same seed produces same operation sequence and outcome
5. Existing suite: `cargo test -p worldwake-core`

### Invariants

1. Spec 9.4: unique physical placement for entities currently in the placement graph (T01)
2. Spec 9.8: reservation exclusivity (T04)
3. Spec 9.18: no circular containment (T13)
4. Determinism: tests use `ChaCha8Rng` with fixed seeds

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/tests/relation_invariants.rs` — T01, T04, T13 randomized invariant tests

### Commands

1. `cargo test -p worldwake-core --test relation_invariants`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`
4. `cargo test --workspace`

## Outcome

- Outcome amended: 2026-03-09
- Completion date: 2026-03-09
- What actually changed:
  - Added `crates/worldwake-core/tests/relation_invariants.rs` with deterministic randomized coverage for T01, T04, and T13 across five derived seeds
  - Added `rand_chacha = "0.3"` as a `worldwake-core` test dependency
  - Corrected the ticket's assumptions before implementation to match the current codebase and test architecture
  - Added explicit `in_transit` placement state for physical entities so unplaced physical entities are no longer represented only by missing `LocatedIn`
- Deviations from original plan:
  - Kept the new coverage as a black-box integration-style test file, but explicitly documented that existing focused relation tests remain inline in `src/world.rs`
  - Narrowed T01 from "every physical entity is always placed" to "every entity intentionally placed into the placement graph keeps a unique effective place", which matches current world behavior
  - Follow-up refinement implemented the spec-aligned long-term placement model (`effective place` or explicit transit state) rather than leaving the architecture at nullable placement only
- Verification results:
  - `cargo test -p worldwake-core --test relation_invariants` passed
  - `cargo test -p worldwake-core` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
