# S134CANEFFSCH-002: EffectSink implementations (authoritative + hypothetical)

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds authoritative `EffectSink` impl in `worldwake-sim`/`worldwake-systems` and hypothetical `EffectSink` impl in `worldwake-ai` over `PlanningState`
**Deps**: archive/tickets/S134CANEFFSCH-001.md

## Problem

S134's unification depends on a single `apply_effects` evaluator dispatching to one of two sinks: `EffectMode::Authoritative` writes to the ECS through the scheduler's existing write paths, and `EffectMode::Hypothetical` writes to a `PlanningState` overlay. Ticket 001 introduces the `EffectSink` trait but leaves it without implementations. This ticket lands both implementations and fills in `apply_effects`'s per-`EffectStep` interpretation, so subsequent per-category tickets can call `apply_effects(..., Authoritative)` from action handler bodies and ticket 010 can switch the planner to `apply_effects(..., Hypothetical)`. Crate layering (`core → sim → systems → ai → cli`) is preserved: the trait is defined in `worldwake-sim` and ai's hypothetical impl is the only place `PlanningState` is named.

## Assumption Reassessment (2026-05-04)

1. The `EffectSink` trait stub from ticket 001 lives in `crates/worldwake-sim/src/effect_schema.rs` with one method per `EffectStep` variant (`write_transfer`, `write_consume`, `write_produce`, `write_wound`, `write_event`, `assert_expectation_fulfilled`, `consume_grant`). `apply_effects` interprets each `EffectStep` by dispatching to the appropriate sink method; the per-step interpretation logic is added in this ticket alongside the sink impls.
2. Authoritative write paths used by current action handlers: scheduler-mediated component mutations through the action `commit_*` handlers (e.g., `commit_attack`, `commit_eat`, `commit_harvest`). The authoritative `EffectSink` impl wraps these existing paths (it does not bypass the scheduler — FND-26). Existing handlers are *not* migrated in this ticket; they remain imperative and continue to run unchanged.
3. Hypothetical overlay shape: `PlanningState` lives at `crates/worldwake-ai/src/planning_state.rs:46` with 17 override fields including `entity_place_overrides: SharedMap<PlanningEntityRef, Option<EntityId>>`, `commodity_quantity_overrides: SharedMap<(PlanningEntityRef, CommodityKind), Quantity>`, `facility_queue_membership_overrides: SharedMap<EntityId, Option<HypotheticalQueueJoin>>`, `facility_grant_overrides: SharedMap<EntityId, Option<ContentionGrant>>`. The hypothetical `EffectSink` impl writes to these existing overrides — no new override fields are introduced unless a per-category ticket later surfaces a coverage gap.
4. Shared abstraction boundary under audit: the `EffectSink` trait surface — sim defines it, sim/systems implements authoritative, ai implements hypothetical. `worldwake-sim` must not name `PlanningState` (would violate FND-26 layering); the trait abstraction is the seam.
5. Existing focused/unit coverage to extend: `crates/worldwake-sim/src/effect_schema.rs` test module from ticket 001 gains tests covering the authoritative sink in unit-test isolation (mock scheduler write surface). `crates/worldwake-ai/src/planning_state.rs` inline tests cover the hypothetical sink against the overlay. Existing goldens are unaffected because no runtime code calls `apply_effects` yet.

## Architecture Check

1. The trait abstraction at `worldwake-sim::EffectSink` keeps `worldwake-sim` ignorant of `worldwake-ai` types (FND-26). The hypothetical impl lives in ai and names `PlanningState`; the authoritative impl lives in sim/systems and never names ai types. Workspace layering `core → sim → systems → ai → cli` is preserved.
2. Both sink impls write through the *same* state surfaces existing handlers and the existing `apply_hypothetical_transition` already use — no new authoritative state is introduced, no new overlay fields unless a coverage gap surfaces. The unification is interpretation-layer only (FND-12 — performance compresses computation, never causality).
3. `apply_effects`'s per-`EffectStep` interpretation is deterministic over `BTreeMap`-ordered inputs with no floats and no wall-clock time, matching CLAUDE.md's Determinism invariant.

## Verification Layers

1. Authoritative sink invariant → focused unit/runtime test: schema with each `EffectStep` variant runs through the authoritative sink against a mock or test-double scheduler write surface and produces the expected component mutations.
2. Hypothetical sink invariant → focused unit/runtime test: same schema run through the hypothetical sink against a fresh `PlanningState` produces matching overlay writes.
3. Mode-parity invariant → focused unit/runtime test: `apply_effects(schema, ..., Authoritative)` and `apply_effects(schema, ..., Hypothetical)` produce structurally equivalent `EffectFact` lists (the sinks differ only in destination, not in interpretation). This is the test that pins the spec's "mode parity at the evaluator layer" Design Goal 4.
4. Bitwise-identical event-log invariant → soak: workspace goldens still pass because no runtime code calls `apply_effects` yet (verified by grep — `apply_effects` callers are still zero outside the new test module). The sink impls are dormant infrastructure landing ahead of the per-category migrations.

## What to Change

### 1. `apply_effects` per-step interpretation (in `worldwake-sim::effect_schema`)

Fill in the function body so each `EffectStep` variant dispatches to the corresponding sink method. Validate `EffectPrecondition`s before the step list executes; on failure, return the appropriate `Discrepancy` variant from `worldwake-core/src/discrepancy.rs:8` (11 variants — `BeliefStale`, `BeliefContradicted`, `SourceInvalidated`, `ImproperPlanningState`, `MissingObservation`, `NoLegalBinding`, `NoWillingCounterparty`, `RouteUnknown`, `SearchBudgetExhausted`, `PartialExecutionDrift`, `NeedHorizonExceeded`).

`PartialOnFailure` handling: try `primary` step list; if any step fails, roll back partial writes and execute `fallback`. The exact rollback discipline depends on whether the sink supports transactional writes (the authoritative sink writes through the scheduler, which already ordering-arbitrates; the hypothetical sink writes to a `PlanningState` clone or snapshot — establish during reassessment whether `SharedMap` supports cheap snapshotting).

### 2. Authoritative `EffectSink` impl

Implement `EffectSink` for the authoritative write context. The impl wraps the existing scheduler write surface and writes through the same paths action handler bodies use today (component mutations, event-log appends, contention-grant consumption). Likely placement: `crates/worldwake-sim/src/effect_schema.rs` if the surface is generic, or `crates/worldwake-systems/src/effect_sink_authoritative.rs` if it needs systems-level access. Decide during reassessment based on which crate carries the necessary write helpers.

### 3. Hypothetical `EffectSink` impl

Implement `EffectSink` for `PlanningState` at `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (new file). The impl writes to the existing 17 override fields (`commodity_quantity_overrides`, `entity_place_overrides`, `facility_grant_overrides`, etc.). Re-export the impl from `crates/worldwake-ai/src/lib.rs` so ticket 010's planner-side switch can construct the sink at the call site.

### 4. Mode-parity unit test

In `crates/worldwake-sim/src/effect_schema.rs` (or a sibling test file), add a focused test that runs a non-trivial schema (e.g., a `Transfer` + `Consume` + `EmitEvent` chain) through both sinks against test fixtures and asserts the resulting `EffectFact` lists match. This is the canonical mode-parity check.

## Files to Touch

- `crates/worldwake-sim/src/effect_schema.rs` (modify — fill in `apply_effects` body, add authoritative sink if surface fits)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` (likely new — confirm placement during reassessment)
- `crates/worldwake-systems/src/lib.rs` (modify — re-export if authoritative sink lives here)
- `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export hypothetical sink)
- Test files: `crates/worldwake-sim/src/effect_schema.rs` `#[cfg(test)]` block (extend); possibly a new `crates/worldwake-sim/tests/effect_sink_mode_parity.rs` for the mode-parity test if it spans both crates.

## Out of Scope

- Populating any non-empty `EffectSchema` literal in production action registrations (per-category tickets 003–009).
- Switching the planner search call site to `apply_effects(..., Hypothetical)` (ticket 010).
- Switching action handler bodies to `apply_effects(..., Authoritative)` (per-category tickets).
- Deleting `apply_hypothetical_transition`, `PlannerTransitionKind`, or `apply_planner_step` (ticket 010).

## Acceptance Criteria

### Tests That Must Pass

1. New focused tests in `effect_schema.rs` (authoritative sink interpretation, hypothetical sink interpretation, mode-parity).
2. `cargo test -p worldwake-sim`, `cargo test -p worldwake-systems`, `cargo test -p worldwake-ai` — all existing tests pass (no runtime code calls `apply_effects` outside new tests).
3. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. `worldwake-sim` Cargo.toml does not gain a dependency on `worldwake-ai` (verified by `cargo tree -p worldwake-sim`). The trait abstraction is the only seam.
2. Authoritative sink and hypothetical sink produce structurally equivalent `EffectFact` lists for the same input schema (mode parity, verified by the mode-parity unit test).
3. `apply_effects` is deterministic over `BTreeMap`-ordered inputs — no `HashMap`/`HashSet` introduced, no floats, no wall-clock reads.
4. Bitwise-identical canonical state hash on the soak scenarios — no runtime code path is altered yet.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/effect_schema.rs` `#[cfg(test)]` block — extend ticket 001's tests with authoritative-sink interpretation per `EffectStep` variant and per `EffectPrecondition` failure path.
2. `crates/worldwake-ai/src/effect_sink_hypothetical.rs` `#[cfg(test)]` block — hypothetical sink writes to `PlanningState` overlay verified against constructed fixture.
3. Mode-parity test (placement TBD during reassessment — likely `crates/worldwake-sim/tests/effect_sink_mode_parity.rs`) — runs identical non-trivial schema through both sinks and asserts `EffectFact` equivalence.

### Commands

1. `cargo test -p worldwake-sim effect_schema`
2. `cargo test -p worldwake-ai effect_sink_hypothetical`
3. `cargo test -p worldwake-ai golden_survival` (regression smoke)
4. `./scripts/verify.sh`
