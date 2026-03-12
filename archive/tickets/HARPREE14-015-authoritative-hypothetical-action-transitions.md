# HARPREE14-015: Planner-owned pickup hypothetical transitions

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes -- AI search semantics ownership for hypothetical pickup transitions
**Deps**: HARPREE14-010 (goal semantics extraction, archived), HARPREE14-011 (local crafted-output acquisition, archived)
**Spec Reference**: HARDENING-PRE-E14.md, HARDEN-A01, Principle 3, Principle 12, Principle 13

## Problem

`crates/worldwake-ai/src/search.rs` currently contains an ad hoc hypothetical-state patch for `pick_up` so local ground lots can satisfy `AcquireCommodity`. That closed a real hole, but the current shape is still wrong:

1. Search owns a transport-specific hypothetical transition.
2. Search dispatches that transition by literal action name.
3. The pickup-specific state mutation is not part of planner semantics, so future hardening work has no clean extension point.

The immediate issue is architectural, not that the planner already supports a broad family of transport transitions incorrectly. The current bug is the search-local ownership of pickup semantics.

## Reassessed Assumptions (2026-03-12)

1. `apply_affordance_transition()` in `crates/worldwake-ai/src/search.rs` special-cases `PlannerOpKind::MoveCargo` plus `def.name == "pick_up"` -- confirmed.
2. The authoritative pickup behavior lives in `crates/worldwake-systems/src/transport_actions.rs::execute_pick_up()` and includes capacity-aware partial splits -- confirmed.
3. `PlannerOpSemantics` currently classifies actions and barrier properties only; it does not carry hypothetical transition ownership -- confirmed.
4. `PlanningState` can override locations, possession, quantities, needs, and removals, but it cannot create a new hypothetical lot identity that could later appear in an executable `PlannedStep` -- confirmed.
5. The planner belief surface does not expose carry-capacity or remaining-load information, so planner-side pickup transitions cannot currently be capacity-correct even before the hypothetical-entity problem is addressed -- confirmed.
6. Because `PlannedStep` targets concrete `EntityId`s, exact planner-side mirroring of authoritative partial-split pickup semantics is not a small follow-on to the current hack. It requires a separate design for hypothetical entity identity, capacity visibility, and execution-time rebinding -- confirmed.
7. `put_down` does not currently need a planner-owned hypothetical transition for any supported goal path. The current ticket should not pretend otherwise just to generalize the seam prematurely -- confirmed.

## Architecture Decision

1. Move pickup hypothetical transition ownership out of `search.rs` and into planner semantics.
2. Keep the transition surface explicit and extensible, but do not overbuild a generic action-family framework that the current planner cannot use correctly yet.
3. Do not claim exact authoritative partial-split or capacity-correct pickup mirroring in this ticket. That belongs in a follow-up ticket that first adds the missing planning-state capabilities.

This is the cleanest durable change available now:

- search becomes generic again
- pickup hypothetical behavior has one planner-owned home
- no compatibility shims remain
- the ticket does not paper over a deeper identity-model gap with a fake “authoritative” approximation

## Scope

### In Scope

1. Introduce a planner-owned hypothetical transition discriminator or equivalent semantics surface.
2. Route pickup hypothetical state updates through that planner-owned surface instead of a literal action-name branch in `search.rs`.
3. Preserve the existing useful local-pickup planning behavior while moving ownership into planner semantics.
4. Leave a clean seam for future transition kinds without claiming support that does not exist.
5. Add tests proving the new ownership boundary and fallback behavior.

### Explicitly Out of Scope

1. Exact planner-side modeling of authoritative partial-split pickup semantics.
2. Planner-side carry-capacity-aware pickup evaluation.
3. Hypothetical entity creation, synthetic lot IDs, or execution-time rebinding of hypothetical entities to authoritative entities.
4. Planner-side `put_down` transition semantics.
5. Recursive harvest -> craft multi-step planning across materialization barriers.
6. Commodity-balance changes such as making `Grain` non-edible.

## What To Change

### 1. Introduce planner-owned hypothetical transition semantics

Add an explicit planner transition surface owned by the AI semantics layer. One acceptable shape:

- extend `PlannerOpSemantics` with a hypothetical transition kind, or
- add a dedicated planner transition table keyed by `ActionDefId`

Requirements:

- search calls the transition surface generically
- search does not dispatch hypothetical transitions by action name
- unsupported actions continue to use the existing goal-model fallback transition path

### 2. Move pickup transition logic behind that surface

The planner-owned pickup transition must replace the current search-local `pick_up` branch for the currently supported local-pickup planning case.

Requirements:

- preserve current local ground-lot acquisition behavior
- keep the state transition concrete in `PlanningState`
- do not add compatibility wrappers around the old search helper

### 3. Encode the current architectural boundary honestly

The implementation and tests must make the current boundary clear:

- pickup has planner-owned hypothetical semantics
- generic actions still use goal-model fallback updates
- exact partial-split and capacity-correct pickup modeling are not implemented here because the planner cannot yet represent executable hypothetical lot identities or carry-capacity state cleanly

## Files To Touch

- `crates/worldwake-ai/src/search.rs` (modify)
- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/planning_state.rs` (modify only if transition helpers belong there)
- `crates/worldwake-ai/src/goal_model.rs` (modify only if fallback ownership needs cleanup)
- `crates/worldwake-ai/src/lib.rs` (modify if new semantics are exported)
- `crates/worldwake-ai/src/<new planner transition module>.rs` (new, if needed)

## Acceptance Criteria

### Tests That Must Pass

1. A search test proves local pickup still satisfies acquisition goals through planner-owned hypothetical transition semantics.
2. A search or semantics test proves search no longer contains a literal `pick_up` transition branch.
3. A regression test proves a generic non-pickup action still uses the fallback hypothetical transition path unchanged.
4. Existing targeted suites for search and golden e2e pass.
5. Existing suite: `cargo test --workspace`
6. Existing lint: `cargo clippy --workspace`

### Invariants

1. Search remains generic and does not contain pickup-specific transition logic.
2. Hypothetical pickup transition ownership lives in planner semantics, not in the search algorithm.
3. No backward-compatibility aliasing or duplicate transition paths remain after the refactor.
4. The ticket does not claim authoritative partial-split or capacity-correct pickup behavior that the current planner architecture cannot represent cleanly.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs` -- verify local pickup uses planner-owned transition semantics and still satisfies acquisition goals.
2. `crates/worldwake-ai/src/planner_ops.rs` or a new planner transition module -- verify pickup transition classification/dispatch is semantics-owned rather than search-owned.
3. `crates/worldwake-ai/src/goal_model.rs` or `crates/worldwake-ai/src/search.rs` -- verify a non-pickup action still relies on the generic fallback hypothetical transition path.
4. `crates/worldwake-ai/tests/golden_e2e.rs` -- ensure existing end-to-end planner behavior still passes after the refactor.

### Commands

1. `cargo test -p worldwake-ai search`
2. `cargo test -p worldwake-ai planner_ops`
3. `cargo test -p worldwake-ai goal_model`
4. `cargo test -p worldwake-ai --test golden_e2e`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Follow-Up

Open a separate ticket if we want exact planner-side partial pickup modeling. That ticket must first design:

1. hypothetical entity identity in `PlanningState`
2. how planner beliefs expose carry-capacity / remaining-load state without violating locality rules
3. how `PlannedStep` can refer to entities created only in hypothetical search
4. how execution/revalidation rebinds those hypothetical entities to authoritative post-commit entities without compatibility shims

## Outcome

- Completion date: 2026-03-12
- What actually changed:
  - moved pickup hypothetical transition ownership out of `search.rs` and into planner semantics via `PlannerTransitionKind` plus `apply_hypothetical_transition()`
  - preserved current local pickup planning behavior while removing the search-local `pick_up` string branch
  - added semantics-focused tests proving `pick_up` is transition-owned by planner semantics and non-pickup actions still use goal-model fallback behavior
- Deviations from original plan:
  - did not implement authoritative partial-split or capacity-correct pickup modeling after reassessment showed the planner lacks both hypothetical entity identity and carry-capacity belief data
  - did not add planner-side `put_down` transition semantics because no supported goal path currently needs it
- Verification results:
  - `cargo test -p worldwake-ai planner_ops`
  - `cargo test -p worldwake-ai goal_model`
  - `cargo test -p worldwake-ai search`
  - `cargo test -p worldwake-systems transport_actions`
  - `cargo test -p worldwake-ai --test golden_e2e`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
