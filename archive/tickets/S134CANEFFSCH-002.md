# S134CANEFFSCH-002: EffectSink implementations (authoritative + hypothetical)

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds authoritative `EffectSink` impl in `worldwake-sim`/`worldwake-systems` and hypothetical `EffectSink` impl in `worldwake-ai` over `PlanningState`
**Deps**: archive/tickets/S134CANEFFSCH-001.md

## Problem

S134's unification depends on a single `apply_effects` evaluator dispatching to one of two sinks: `EffectMode::Authoritative` writes through the authoritative transaction surface, and `EffectMode::Hypothetical` writes to a `PlanningState` overlay. Ticket 001 introduces the `EffectSink` trait but leaves it without implementations. This ticket lands both implementations and fills in `apply_effects`'s per-`EffectStep` interpretation, so subsequent per-category tickets can call `apply_effects(..., Authoritative)` from action handler bodies and ticket 010 can switch the planner to `apply_effects(..., Hypothetical)`. Crate layering (`core → sim → systems → ai → cli`) is preserved: the trait is defined in `worldwake-sim` and ai's hypothetical impl is the only place `PlanningState` is named.

## Assumption Reassessment (2026-05-04)

1. The `EffectSink` trait stub from ticket 001 lives in `crates/worldwake-sim/src/effect_schema.rs` with one method per `EffectStep` variant (`write_transfer`, `write_consume`, `write_produce`, `write_wound`, `write_event`, `assert_expectation_fulfilled`, `consume_grant`). `apply_effects` now interprets each `EffectStep` by dispatching to the corresponding fallible sink method and emits `EffectFact`s only after successful sink writes.
2. Authoritative write paths used by current action handlers are action-local `WorldTxn` mutations. The authoritative `EffectSink` impl therefore lives in `worldwake-systems` and wraps the current `WorldTxn` surfaces for commodity quantity writes, event tags, expectations, and contention-grant clearing. Existing handlers are *not* migrated in this ticket; they remain imperative and continue to run unchanged.
3. Hypothetical overlay shape: `PlanningState` lives at `crates/worldwake-ai/src/planning_state.rs:46` with 17 override fields including `entity_place_overrides: SharedMap<PlanningEntityRef, Option<EntityId>>`, `commodity_quantity_overrides: SharedMap<(PlanningEntityRef, CommodityKind), Quantity>`, `facility_queue_membership_overrides: SharedMap<EntityId, Option<HypotheticalQueueJoin>>`, `facility_grant_overrides: SharedMap<EntityId, Option<ContentionGrant>>`. The hypothetical `EffectSink` impl writes to these existing overrides — no new override fields are introduced unless a per-category ticket later surfaces a coverage gap.
4. Shared abstraction boundary under audit: the `EffectSink` trait surface — sim defines it, sim/systems implements authoritative, ai implements hypothetical. `worldwake-sim` must not name `PlanningState` (would violate FND-26 layering); the trait abstraction is the seam.
5. Existing focused/unit coverage to extend: `crates/worldwake-sim/src/effect_schema.rs` test module from ticket 001 gains evaluator-dispatch, rollback, and mode-parity tests. `crates/worldwake-ai/src/planning_state.rs` inline tests cover the hypothetical sink against the overlay. `crates/worldwake-systems/src/effect_sink_authoritative.rs` tests cover authoritative commodity/event/rollback behavior against real `WorldTxn` fixtures. Existing goldens are unaffected because no runtime code calls `apply_effects` yet.
6. Category-boundary correction: `ApplyWound` is represented in the schema enum but does not carry enough data for real combat wound construction, so both real sinks currently return `Discrepancy::ImproperPlanningState` for that step. Future wound-category work must extend the schema payload before relying on this step.
7. Rollback-boundary correction: `PartialOnFailure` rollback is implemented generically through `EffectSink::checkpoint`/`restore`, and the hypothetical sink supports it with cloned `PlanningState` snapshots. The authoritative sink rejects `restore` because `WorldTxn` has no public generic transaction snapshot. Authoritative schemas that need fallback rollback must wait for a dedicated atomic/transaction-snapshot ticket rather than assuming rollback is available here.

## Architecture Check

1. The trait abstraction at `worldwake-sim::EffectSink` keeps `worldwake-sim` ignorant of `worldwake-ai` types (FND-26). The hypothetical impl lives in ai and names `PlanningState`; the authoritative impl lives in sim/systems and never names ai types. Workspace layering `core → sim → systems → ai → cli` is preserved.
2. Both sink impls write through the *same* state surfaces existing handlers and the existing `apply_hypothetical_transition` already use — no new authoritative state is introduced, no new overlay fields unless a coverage gap surfaces. The unification is interpretation-layer only (FND-12 — performance compresses computation, never causality).
3. `apply_effects`'s per-`EffectStep` interpretation is deterministic over `BTreeMap`-ordered inputs with no floats and no wall-clock time, matching CLAUDE.md's Determinism invariant.

## Verification Layers

1. Authoritative sink invariant → focused unit/runtime test: commodity transfer/event-tag paths run through `AuthoritativeEffectSink` against real `WorldTxn` fixtures and produce the expected component mutations or transaction metadata.
2. Hypothetical sink invariant → focused unit/runtime test: transfer and rollback paths run through `HypotheticalEffectSink` against a fresh `PlanningState` and produce the expected overlay writes.
3. Mode-parity invariant → focused evaluator unit test: `apply_effects(schema, ..., Authoritative)` and `apply_effects(schema, ..., Hypothetical)` produce structurally equivalent `EffectFact` lists against recording sinks. Real sink destination coverage is split by crate because no runtime schema calls both sinks yet.
4. Bitwise-identical event-log invariant → `./scripts/verify.sh`: workspace tests and scenario coverage still pass because no runtime code calls `apply_effects` yet (verified by grep — `apply_effects` callers are still zero outside the new test modules). The sink impls are dormant infrastructure landing ahead of the per-category migrations.

## What to Change

### 1. `apply_effects` per-step interpretation (in `worldwake-sim::effect_schema`)

Fill in the function body so each `EffectStep` variant dispatches to the corresponding sink method. Validate `EffectPrecondition`s before the step list executes; on failure, return the appropriate `Discrepancy` variant from `worldwake-core/src/discrepancy.rs:8` (11 variants — `BeliefStale`, `BeliefContradicted`, `SourceInvalidated`, `ImproperPlanningState`, `MissingObservation`, `NoLegalBinding`, `NoWillingCounterparty`, `RouteUnknown`, `SearchBudgetExhausted`, `PartialExecutionDrift`, `NeedHorizonExceeded`).

`PartialOnFailure` handling: try `primary` step list; if any step fails, roll back partial writes and execute `fallback`. The generic evaluator now delegates rollback to sink checkpoints; the hypothetical sink supports this, while the authoritative `WorldTxn` sink reports `ImproperPlanningState` until a later atomic authoritative transaction surface exists.

### 2. Authoritative `EffectSink` impl

Implement `EffectSink` for the authoritative write context. The impl wraps the existing `WorldTxn` surface and writes through the same state paths action handler bodies use today for commodity lots, transaction tags, expectation records, and contention-grant consumption. Placement is `crates/worldwake-systems/src/effect_sink_authoritative.rs` because the sink needs systems-level transaction helpers.

### 3. Hypothetical `EffectSink` impl

Implement `EffectSink` for `PlanningState` at `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (new file). The impl writes to the existing 17 override fields (`commodity_quantity_overrides`, `entity_place_overrides`, `facility_grant_overrides`, etc.). Re-export the impl from `crates/worldwake-ai/src/lib.rs` so ticket 010's planner-side switch can construct the sink at the call site.

### 4. Mode-parity unit test

In `crates/worldwake-sim/src/effect_schema.rs` (or a sibling test file), add a focused test that runs a non-trivial schema (e.g., a `Transfer` + `Consume` + `EmitEvent` chain) through both sinks against test fixtures and asserts the resulting `EffectFact` lists match. This is the canonical mode-parity check.

## Files to Touch

- `crates/worldwake-sim/src/effect_schema.rs` (modify — fill in `apply_effects` body and evaluator tests)
- `crates/worldwake-systems/src/effect_sink_authoritative.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — re-export if authoritative sink lives here)
- `crates/worldwake-ai/src/effect_sink_hypothetical.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export hypothetical sink)
- `crates/worldwake-ai/src/planning_state.rs` (modify — hypothetical sink focused tests)
- Test files: `crates/worldwake-sim/src/effect_schema.rs` `#[cfg(test)]` block (extend); `crates/worldwake-systems/src/effect_sink_authoritative.rs` `#[cfg(test)]` block (new).

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
4. `./scripts/verify.sh` passes — no runtime code path is altered yet.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/effect_schema.rs` `#[cfg(test)]` block — extends ticket 001's tests with evaluator dispatch, `PartialOnFailure` rollback through sink checkpoints, and mode-parity interpretation.
2. `crates/worldwake-ai/src/planning_state.rs` inline tests — hypothetical sink writes to `PlanningState` overlay and restores a checkpoint against constructed fixtures.
3. `crates/worldwake-systems/src/effect_sink_authoritative.rs` inline tests — authoritative sink writes controlled commodity lots and event tags through `WorldTxn`, and documents that generic rollback restore is rejected.

### Commands

1. `cargo test -p worldwake-sim effect_schema`
2. `cargo test -p worldwake-ai effect_sink_hypothetical`
3. `cargo test -p worldwake-systems --lib effect_sink_authoritative`
4. `cargo test -p worldwake-sim`
5. `cargo test -p worldwake-systems`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`
8. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-04.

- Filled in `apply_effects` with precondition checks, fallible per-step dispatch, fact emission, and checkpoint-backed `PartialOnFailure` interpretation.
- Added `AuthoritativeEffectSink` over `WorldTxn` in `worldwake-systems` for commodity transfer/consume/produce, event tags, expectation recording, and contention-grant clearing.
- Added `HypotheticalEffectSink` over `PlanningState` in `worldwake-ai` for commodity quantity overlays, expectation/event recording, contention grant consumption, and snapshot rollback.
- Added focused evaluator, hypothetical-overlay, and authoritative-transaction tests. Runtime action handlers remain unchanged; `rg -n "apply_effects\\(" crates/` shows no production callers yet.

## Deviations

- `ApplyWound` remains staged and returns `Discrepancy::ImproperPlanningState` in both real sinks because the current schema step does not carry enough combat wound payload to construct a real wound.
- Generic authoritative rollback for `PartialOnFailure` is not implemented because `WorldTxn` does not expose a generic snapshot/restore surface. Hypothetical rollback is implemented; authoritative schemas that require fallback rollback need a later atomic transaction ticket.
- The authoritative proof uses real `WorldTxn` fixtures rather than a mock scheduler write surface.

## Verification Result

- `cargo test -p worldwake-sim --lib effect_schema` — passed.
- `cargo test -p worldwake-ai --lib hypothetical_effect_sink` — passed.
- `cargo test -p worldwake-systems --lib effect_sink_authoritative` — passed.
- `cargo test -p worldwake-sim` — passed.
- `cargo test -p worldwake-systems` — passed.
- `cargo test -p worldwake-ai` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `./scripts/verify.sh` — passed.
