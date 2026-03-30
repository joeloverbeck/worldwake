# E19GUAPAT-005: Extend public_order() with guard_presence_factor()

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — extend the `public_order()` derived aggregator in `worldwake-systems`
**Deps**: [specs/E19-guard-patrol.md](/home/joeloverbeck/projects/worldwake/specs/E19-guard-patrol.md), [archive/tickets/guard-patrol/E19GUAPAT-001.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-001.md), [archive/tickets/guard-patrol/E19GUAPAT-003.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-003.md), [archive/tickets/guard-patrol/E19GUAPAT-004.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-004.md)

## Problem

[`public_order()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L130) currently models vacancy and hostile-faction pressure but still omits the E19 guard-presence extension. That leaves the derived public-order view lagging behind the live patrol architecture: guards can now exist, patrol, and rank patrol urgency from beliefs, but the designer/CLI public-order readout does not reflect their stabilizing presence at a place.

## Assumption Reassessment (2026-03-30)

1. The live symbol under audit is [`public_order(place, world)`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L130) in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs). It starts from `PUBLIC_ORDER_BASELINE`, subtracts `VACANT_OFFICE_PENALTY` per vacant office in jurisdiction, and subtracts `HOSTILE_FACTION_PAIR_PENALTY` per present hostile faction pair.
2. The current place-query surface in authoritative world code is [`World::entities_effectively_at(place)`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/world/placement.rs#L79), not `world.entities_at(place)`. The original ticket draft used the wrong query shape and would not match the live codebase.
3. Patrol substrate is already live. [`PatrolRoute`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs) and [`PatrolProfile`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/patrol.rs) exist in `worldwake-core`; the authoritative `"patrol"` action is already implemented in [`crates/worldwake-systems/src/patrol_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/patrol_actions.rs); and `GoalKind::Patrol` candidate emission plus ranking are already live in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) and [`crates/worldwake-ai/src/ranking.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/ranking.rs). The old dependency wording implied future work that has already been delivered and archived.
4. The spec’s intended guard factor is still a derived view only. [`specs/E19-guard-patrol.md`](/home/joeloverbeck/projects/worldwake/specs/E19-guard-patrol.md) states that agents do not read `public_order()`; guard urgency comes from beliefs, while `public_order()` is a designer/CLI aggregation surface.
5. The clean live “guard presence” proxy is the presence of a colocated living agent with a `PatrolRoute` component. That matches the spec’s chosen identity surface without adding a second `is_guard` flag. This ticket should not broaden scope into “currently active patrol action only” detection, which would couple a stable place-level derived view to transient action execution state.
6. Focused `public_order` tests already live in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs): `public_order_baseline_is_stable_when_place_has_no_vacancy_or_hostility`, `public_order_subtracts_vacant_office_penalties`, `hostile_faction_pairs_count_one_way_hostility_once`, and `public_order_combines_vacancy_and_hostility_and_saturates_at_zero`. There is currently no guard-presence coverage.
7. This is a single-layer derived-view ticket. No AI/operator/shared-runtime boundary changes are needed because `public_order()` is not part of the AI read path.
8. Mismatch + correction: the original ticket’s test plan relied on approximate filters such as `cargo test -p worldwake-systems -- offices` and referred to possible test locations under `tests/`. The live focused coverage sits in `offices.rs`, and the ticket should name exact test targets or exact crate targets to satisfy the repo’s ticket contract.
9. Adjacent architectural note, not in scope: `public_order()` still uses module-local constants rather than profile-driven tuning. Given the current E16 architecture, extending the existing constant-based aggregator is cleaner than introducing speculative configuration plumbing here. If settlement-specific public-order dynamics become a real requirement later, that should be a direct model upgrade, not a side effect of this small derived-view ticket.

## Architecture Check

1. Extending the existing `public_order()` aggregator with a private `guard_presence_factor()` helper is cleaner than introducing a second public-order function, a stored cache, or a separate “guard order” side channel. One derived aggregator remains the single source of public-order composition.
2. Counting colocated `PatrolRoute` holders is cleaner than adding a new `is_guard` alias component or tying the metric to active-action state. Patrol assignment is the durable architectural identity of a guard in the current model.
3. The change remains architecturally bounded: no new authoritative state, no AI read-path changes, no backward-compatibility shim, and no aliasing of guard identity.

## Verification Layers

1. Baseline/no-guard behavior remains unchanged -> focused unit test in `offices.rs`
2. Colocated patrol-assigned guard raises the derived value -> focused unit test in `offices.rs`
3. Non-guard colocated agent does not contribute -> focused unit test in `offices.rs`
4. Guard contribution cap is enforced independently of guard count -> focused unit test in `offices.rs`
5. Existing vacancy/hostility aggregation still composes correctly -> existing focused `public_order` tests plus `cargo test -p worldwake-systems`
6. Additional layer mapping is not applicable because this ticket changes only a pure derived-view function, not AI planning, runtime request handling, or authoritative mutation ordering.

## What to Change

### 1. Add `guard_presence_factor()` in `crates/worldwake-systems/src/offices.rs`

Implement a private helper that:

- iterates `world.entities_effectively_at(place)`
- counts living colocated agents with `PatrolRoute`
- converts that count into a `Permille` bonus via named constants
- caps the additive bonus

### 2. Integrate the helper into `public_order()`

Add the guard bonus as an additive term after the existing vacancy and hostility deductions, using the existing `Permille` saturating arithmetic pattern so the function remains a single composed derived value.

### 3. Add focused public-order regression coverage

Extend the existing `offices.rs` test module with guard-specific cases for:

- one guard raises order
- non-guard agents do not raise order
- multiple guards respect the additive cap
- guard bonus composes with vacancy/hostility deductions rather than replacing them

## Files to Touch

- `crates/worldwake-systems/src/offices.rs` (modify)

## Out of Scope

- Patrol action execution or duration behavior
- Patrol candidate generation or patrol ranking
- Route adaptation / waypoint mutation
- Golden feedback-loop testing in `worldwake-ai`
- Any new stored public-order state, caches, or AI reads of `public_order()`
- Any new alias path for guard identity

## Acceptance Criteria

### Tests That Must Pass

1. `public_order_baseline_is_stable_when_place_has_no_vacancy_or_hostility`
2. New focused test: one colocated patrol-route holder increases `public_order()`
3. New focused test: colocated non-guard agents do not change the guard bonus
4. New focused test: many colocated patrol-route holders cap the additive guard bonus
5. Existing suite: `cargo test -p worldwake-systems`
6. Existing suite: `cargo test --workspace`
7. `cargo clippy --workspace`

### Invariants

1. `public_order()` remains a derived function and is never stored as authoritative state
2. Guard detection stays on the existing patrol identity surface (`PatrolRoute` presence), with no alias component or backward-compatibility shim
3. Arithmetic stays deterministic and integer/`Permille` based
4. The change does not alter the AI patrol motive path, which remains belief-driven rather than `public_order()`-driven

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/offices.rs` — add focused guard-presence tests proving additive increase, cap enforcement, and non-guard exclusion
2. `crates/worldwake-systems/src/offices.rs` — extend composition coverage so guard bonus is proven to stack with existing vacancy/hostility deductions instead of bypassing them

### Commands

1. `cargo test -p worldwake-systems public_order_`
2. `cargo test -p worldwake-systems hostile_faction_pairs_count_one_way_hostility_once`
3. `cargo test -p worldwake-systems`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-30
- What actually changed:
  - extended [`public_order()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L132) with a private [`guard_presence_factor()`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L150)
  - added named guard-order constants in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L18)
  - added focused guard-presence regression tests in [`crates/worldwake-systems/src/offices.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/offices.rs#L2192)
- Deviations from original plan:
  - used `entities_effectively_at()` rather than the earlier ticket draft’s stale `entities_at()` reference
  - kept guard identity on the existing `PatrolRoute` surface only; no separate guard marker or active-action check was added
  - fixed the ticket’s stale dependency references and verification plan before implementation
- Verification results:
  - `cargo test -p worldwake-systems public_order_` passed
  - `cargo test -p worldwake-systems hostile_faction_pairs_count_one_way_hostility_once` passed
  - `cargo test -p worldwake-systems` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
