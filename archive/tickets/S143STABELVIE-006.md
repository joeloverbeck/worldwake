# S143STABELVIE-006: Belief-wall trap golden + `compile_fail` doctest on `DebugWorldView`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None (test infrastructure only; new golden + doctest)
**Deps**: archive/tickets/S143STABELVIE-002.md, archive/tickets/S143STABELVIE-003.md, archive/tickets/S143STABELVIE-004.md, archive/tickets/S143STABELVIE-005.md

## Problem

The spec's two type-level guarantees need explicit regression coverage:

1. **FND-14A wall** — "co-location does not tell you who owns the chest". Without a golden that exercises co-located physical observation alongside authority-belief absence, a future refactor could silently re-introduce an authoritative read on a `BelievedAuthorityView` impl, returning a real owner identity for a co-located chest where the agent has no owner-belief. The CI grep lint (ticket 005) doesn't catch this — it catches only `DebugWorldView` imports.
2. **`DebugWorldView` unreachability** — the cfg-gate (ticket 002) and CI grep lint (ticket 005) provide enforcement, but a `compile_fail` doctest on the trait's documentation is the type-level witness that future refactors cannot accidentally widen the trait's reachability. Compile-fail doctests run under `cargo test --doc` and produce a clear failure mode when the boundary is breached.

## Assumption Reassessment (2026-05-13)

1. Compile-fail doctest precedents in worldwake-ai: `crates/worldwake-ai/src/planning_snapshot.rs:402-414` (proves the `DistanceMatrix` field is unreachable from external code; verified during prior reassessment) and `crates/worldwake-ai/src/ranking.rs:7-19` (proves the `compare_ranked_goals` private function and the `OrderedRanked::from_sorted_for_test` test-only accessor are unreachable from external code). The pattern uses `/// ```compile_fail` doctests in the trait/struct doc comment.
2. Adjacent existing goldens that exercise observation and belief boundaries: `crates/worldwake-ai/tests/golden_epistemic_sensing.rs`, `golden_perception_omission.rs`, `golden_perception_exposure.rs` (all confirmed to exist via path batch). The new belief-wall trap golden composes with these — it does not replace them; the scope is distinct (authority-belief absence given physical co-location).
3. Scenario authoring approach: golden tests in `crates/worldwake-ai/tests/` typically construct scenarios programmatically or via RON loading (e.g., `scenarios/*.ron`). The belief-wall trap golden authors a small scenario inline or as a dedicated RON file with: one agent, one chest (item-lot), one place (office building); agent's belief store has co-location observation but no owner-belief, holder-belief, jurisdiction-belief, or office-holder-belief for the chest/building.

## Architecture Check

1. The golden lives in the standard `crates/worldwake-ai/tests/golden_*.rs` location, following existing convention.
2. The `compile_fail` doctest lands on `DebugWorldView`'s trait doc comment in `crates/worldwake-sim/src/belief_view.rs`. Compile-fail doctests are exercised by `cargo test --doc -p worldwake-sim`; they produce a clear failure signal if the cfg-gate is ever relaxed or the trait is moved.
3. The golden's assertions cover both the legal path (physical observation succeeds for co-located entities) and the illegal-by-FND-14A path (authority-belief reads return `BeliefRead::Unknown` despite co-location).

## Outcome

Completed on 2026-05-13. The implementation landed the belief-wall trap golden,
the `DebugWorldView` compile-fail doctest, generated golden inventory updates,
and S143 spec/order wording that matches the two live proof surfaces.

## Verified Layers

1. **Decision-trace assertion (FND-14A wall)**: After the agent observes the chest, no `Steal` candidate appears in the agent's decision trace — verified via `AgentDecisionTrace` inspection in the golden. This is the primary behavioral proof that the wall holds end-to-end.
2. **Authority-belief absence (focused)**: A focused unit test within the golden directly verifies `view.believed_owner_of(chest) == BeliefRead::Unknown` and `view.believed_office_holder(office) == BeliefRead::Unknown` for the test agent at scenario start. This is the lower-layer proof surface for the type-system contract.
3. **Co-located physical observation (legal path)**: A focused unit test verifies `view.colocated_entities(actor).value.contains(&chest)` and `value.contains(&building)` — confirming the legal `LocalPhysicalObservationView` reads succeed.
4. **Compile-time enforcement (`DebugWorldView`)**: `cargo test --doc -p worldwake-sim` exercises the `compile_fail` doctest — its absence of compile failure (i.e., if the doctest unexpectedly compiles) means the trait surface no longer enforces the boundary; the doctest fails the test run.

## Landed Changes

### 1. Golden test `crates/worldwake-ai/tests/golden_belief_wall_trap.rs`

Per spec D8. The landed golden uses an inline programmatic fixture rather than a
RON scenario because the owned seam is the trait/read surface and planner
candidate boundary. Scenario 420 (`Belief Wall Trap Suppresses Theft`) sets up:
- One agent, located at one place ("office_building").
- One chest (`ItemLot` entity) at the same place, owned (in authoritative world state) by a second agent the test agent has never met.
- The office building has an institutional record (`OfficeHeld`) indicating a magistrate's jurisdiction.
- The agent's `AgentBeliefStore`:
  - Has co-location observation entries for the chest and building (per perception output).
  - Has **no** owner-belief for the chest (`believed_owner_of(chest) == Unknown`).
  - Has **no** holder-belief, access-right-belief, jurisdiction-belief, or office-holder-belief for the building/office.

Assertions:
- `view.colocated_entities(actor).value` contains the chest and building entities.
- `view.believed_owner_of(chest) == BeliefRead::Unknown`.
- `view.believed_holder_of(chest) == BeliefRead::Unknown`.
- `view.believed_jurisdiction(building) == BeliefRead::Unknown`.
- `view.believed_office_holder(office) == BeliefRead::Unknown`.
- After running the agent's decision tick, the decision trace contains no `Steal` candidate (the legality predicate requires `believed_owner_of` which returns `Unknown`).
- After running the decision tick, no `Steal` action is committed to the event log (regression guard at the authoritative outcome layer).

### 2. `compile_fail` doctest on `DebugWorldView` in `crates/worldwake-sim/src/belief_view.rs`

The ticket's drafted cfg/import doctest was reassessed as stale: live doctests
run in a context where `#[cfg(any(debug_assertions, test))]` makes the trait
visible, so that snippet would not truthfully prove release-style import
unreachability. The landed doctest instead proves the FND-14A type boundary that
matters for runtime planner composition: a `RuntimeBeliefView` cannot call
`DebugWorldView::world_owner_of`.

```rust
/// Debug/observer access to authoritative world state.
///
/// `DebugWorldView` is deliberately outside the runtime belief-view trait
/// composition. Planner-facing code may read through `RuntimeBeliefView`, but
/// adding debug-world methods to that surface would pierce the FND-14A wall.
///
/// ```compile_fail
/// use worldwake_core::EntityId;
/// use worldwake_sim::{DebugWorldView, RuntimeBeliefView};
///
/// fn debug_read_from_runtime_view<T: RuntimeBeliefView + ?Sized>(
///     view: &T,
///     entity: EntityId,
/// ) {
///     let _ = view.world_owner_of(entity);
/// }
/// ```
#[cfg(any(debug_assertions, test))]
pub trait DebugWorldView {
    // ...existing trait body from ticket 002...
}
```

## Landed Files

- `crates/worldwake-ai/tests/golden_belief_wall_trap.rs` (new)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add `compile_fail` doctest on `DebugWorldView`)
- `docs/generated/golden-scenario-details/belief-wall-trap.md` (new)
- `docs/generated/golden-coverage-matrix.md` (regenerated)
- `docs/generated/golden-e2e-inventory.md` (regenerated)
- `docs/generated/golden-scenario-index.md` (regenerated)
- `docs/generated/golden-scenario-details/opportunity-compiler.md` (regenerated source-line metadata)
- `docs/generated/golden-scenario-details/survival-justice.md` (regenerated source-line metadata)

## Out of Scope

- Modifying existing goldens (`golden_epistemic_sensing.rs`, etc.) — the new golden composes with them, not replaces.
- The CI grep lint — ticket 005 (already lands enforcement on `DebugWorldView` imports, separately from the doctest).
- Trait migration — tickets 003 and 004.

## Acceptance Result

### Passed Tests

1. `cargo test -p worldwake-ai --test golden_belief_wall_trap` passed.
2. `cargo test -p worldwake-sim --doc` passed and exercised the `DebugWorldView` compile-fail doctest.
3. Adjacent goldens passed as separate cargo invocations: `cargo test -p worldwake-ai --test golden_epistemic_sensing`, `cargo test -p worldwake-ai --test golden_perception_omission`, and `cargo test -p worldwake-ai --test golden_perception_exposure`.
4. `cargo test -p worldwake-ai` passed.
5. `cargo test --workspace` passed.
6. `cargo clippy --workspace --all-targets -- -D warnings` passed.
7. `python3 scripts/golden_inventory.py --write --check-docs` passed and refreshed the generated golden inventory.

### Invariants

1. The golden encodes the FND-14A "co-location does not tell you who owns the chest" rule as a regression check. It proves co-located physical reads succeed while authority reads remain `BeliefRead::Unknown`.
2. The `compile_fail` doctest is the type-level witness that `DebugWorldView` remains outside `RuntimeBeliefView`, so runtime planner composition does not gain debug-world authority reads.
3. The golden's decision-trace and event-log assertions are layered per `docs/precision-rules.md` Rule 5: decision trace for AI candidate suppression, event log for authoritative no-commit outcome.

## Verification Result

1. Passed `cargo test -p worldwake-ai --test golden_belief_wall_trap`.
2. Passed `cargo test -p worldwake-sim --doc`.
3. Passed `cargo test -p worldwake-ai --test golden_epistemic_sensing`.
4. Passed `cargo test -p worldwake-ai --test golden_perception_omission`.
5. Passed `cargo test -p worldwake-ai --test golden_perception_exposure`.
6. Passed `python3 scripts/golden_inventory.py --write --check-docs`.
7. Passed `cargo test -p worldwake-ai`.
8. Passed `cargo test --workspace`.
9. Passed `cargo clippy --workspace --all-targets -- -D warnings`.
10. Passed `./scripts/verify.sh`.
