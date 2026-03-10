# E09NEEMET-005: Metabolism system — basal progression and action body costs

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — first real `SystemFn`, system-context extension, physiology `WorldTxn` support
**Deps**: E09NEEMET-001, E09NEEMET-002, E09NEEMET-003, E09NEEMET-004

## Problem

The metabolism system must run once per tick for all living agents, applying basal physiological progression from `MetabolismProfile` and action body costs from active actions. This is the core tick-driven engine that makes needs change over time, giving the survival loop its teeth.

## Assumption Reassessment (2026-03-10)

1. `SystemId::Needs` already exists in `crates/worldwake-sim/src/system_manifest.rs` as ordinal `0` — confirmed.
2. `SystemDispatchTable` expects a plain `SystemFn`: `fn(SystemExecutionContext<'_>) -> Result<(), SystemError>` — confirmed.
3. `SystemExecutionContext` currently exposes only `world`, `event_log`, `rng`, `tick`, and `system_id`. It does **not** expose active actions or the action registry, so body-cost application is not implementable in `worldwake-systems` as currently wired.
4. `worldwake-systems/Cargo.toml` currently depends only on `worldwake-core` — it must add `worldwake-sim`.
5. `worldwake-sim::ActionDef` already includes mandatory `body_cost_per_tick: BodyCostPerTick`. The authoritative body-cost schema is settled.
6. `Scheduler` already owns the active `ActionInstance` map, and `TickStepServices` already owns `ActionDefRegistry`; the clean missing seam is read-only exposure of both through `SystemExecutionContext`.
7. `WorldTxn` is already the established event-sourced mutation boundary used by systems in `worldwake-sim` tests, but it currently lacks physiology-specific component update helpers.
8. `Permille` currently has only validated construction and raw accessors. Saturating helpers would improve readability here, but raw-`u16` methods named `checked_*` would be misleading because they would not actually return `Option`/`Result`.

## Architecture Check

1. The needs system is the first real `SystemFn` implementation. It sets the pattern for future systems and should establish a clean, explicit mutation path rather than bypassing the event log.
2. The system should mutate authoritative physiology through `WorldTxn`, not through crate-private `World` internals and not through direct `pub(crate)` component-table access. That keeps system-side world changes aligned with the append-only event log.
3. Body cost per tick should be read directly from `ActionDef.body_cost_per_tick`, keyed by each active action instance's `def_id`. Duplicating body cost onto another schema or temporary lookup would create an alias path and weaken the action architecture.
4. `SystemExecutionContext` should gain read-only action-runtime access (`active_actions`, `action_defs`) rather than giving systems write access to the scheduler. Systems need to observe current action load, not orchestrate scheduling internals.
5. `SystemDispatchTable::canonical_noop()` should remain a real noop fixture for low-level sim tests. This ticket should add an explicit dispatch-table constructor in `worldwake-systems` instead of mutating the meaning of a testing helper.
6. Deprivation consequences remain split into E09NEEMET-006. This ticket stops at basal progression, action body costs, and deprivation exposure tracking.
7. If `Permille` helpers are added, they should be true saturating semantics with accurate names. Misnamed `checked_*` methods would be a long-term API smell and are not justified.

## Scope Correction

This ticket should:

1. Extend `SystemExecutionContext` so systems can read the current active actions and the `ActionDefRegistry`.
2. Add physiology mutation helpers to `WorldTxn` so systems can update `HomeostaticNeeds` and `DeprivationExposure` through the event-sourced boundary.
3. Implement the metabolism / needs system in `worldwake-systems` for basal progression, action body cost application, and deprivation exposure tracking.
4. Provide an explicit dispatch-table constructor in `worldwake-systems` that registers `needs_system` for `SystemId::Needs` and uses local noops for the remaining slots.
5. Add focused tests in `worldwake-core`, `worldwake-sim`, and `worldwake-systems`, then run the relevant crate tests plus workspace lint/test verification.

This ticket should not:

1. Reinterpret `canonical_noop()` as production wiring.
2. Introduce duplicate body-cost metadata or cache action costs onto agents.
3. Implement deprivation wounds, forced sleep/collapse, or involuntary relief.
4. Add misleading `Permille::checked_*` APIs that do saturating work under a checked name.

## What to Change

### 1. Update `crates/worldwake-systems/Cargo.toml`

Add `worldwake-sim = { path = "../worldwake-sim" }` dependency.

### 2. Add `Permille` saturating helpers in `crates/worldwake-core/src/numerics.rs`

Add methods to `Permille`:
- `saturating_add(self, other: Permille) -> Permille` — clamps at 1000
- `saturating_sub(self, other: Permille) -> Permille` — clamps at 0

### 3. Extend system execution context in `crates/worldwake-sim`

Update `SystemExecutionContext` to expose:
- `active_actions: &'a BTreeMap<ActionInstanceId, ActionInstance>`
- `action_defs: &'a ActionDefRegistry`

Update `tick_step.rs` and all tests constructing `SystemExecutionContext` accordingly.

### 4. Add physiology mutation helpers to `crates/worldwake-core/src/world_txn.rs`

Add targeted setters for:
- `set_homeostatic_needs(entity, HomeostaticNeeds)`
- `set_deprivation_exposure(entity, DeprivationExposure)`

These should stage the updated component value on `staged_world` and emit the corresponding `ComponentDelta::Set`.

### 5. New module `crates/worldwake-systems/src/needs.rs`

Implement `pub fn needs_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>`:

Per tick, for each entity with `AgentData + HomeostaticNeeds + MetabolismProfile + DriveThresholds`:

1. **Basal progression**: Add `MetabolismProfile` rates to `HomeostaticNeeds` fields using saturating arithmetic.
2. **Action body cost**: If the agent has an active action, read its `ActionDef.body_cost_per_tick` through `ctx.active_actions` + `ctx.action_defs` and apply those deltas.
3. **Clamp**: All `Permille` values stay in valid range (handled by saturating ops).
4. **Update DeprivationExposure**: For each drive, if current value >= critical threshold from `DriveThresholds`, increment the corresponding `_critical_ticks` counter. Otherwise, reset it to 0.
5. **Commit once**: Apply all changed physiology via a single hidden `WorldTxn` commit for the system tick.

### 6. Wire into an explicit `worldwake-systems` dispatch table

Add a constructor such as `pub fn dispatch_table() -> SystemDispatchTable` so that `SystemId::Needs` maps to `needs_system` and the remaining systems use local noops. Do **not** change `SystemDispatchTable::canonical_noop()`.

### 7. Export from `crates/worldwake-systems/src/lib.rs`

Add `pub mod needs;` and re-export `dispatch_table` plus `needs_system`.

## Files to Touch

- `crates/worldwake-systems/Cargo.toml` (modify — add `worldwake-sim` dep)
- `crates/worldwake-systems/src/lib.rs` (modify — add module)
- `crates/worldwake-systems/src/needs.rs` (new — metabolism system implementation)
- `crates/worldwake-core/src/numerics.rs` (modify — add `Permille` saturating arithmetic)
- `crates/worldwake-core/src/world_txn.rs` (modify — add physiology mutation helpers)
- `crates/worldwake-sim/src/system_dispatch.rs` (modify — extend `SystemExecutionContext`)
- `crates/worldwake-sim/src/tick_step.rs` (modify — pass active actions and action defs into system context)

## Out of Scope

- Deprivation consequences / wound generation (E09NEEMET-006)
- Forced collapse / involuntary relief (E09NEEMET-006)
- Consumption / care action handlers (E09NEEMET-007)
- Sleep recovery logic (E09NEEMET-007)
- AI decision-making based on needs (E13)
- Reworking action metadata architecture or adding alternate body-cost lookup tables
- Changing `SystemDispatchTable::canonical_noop()`

## Acceptance Criteria

### Tests That Must Pass

1. **T15: Need progression** — hunger/thirst/fatigue/bladder/dirtiness values evolve by simulation tick.
2. **T26: Camera independence** — physiology does not reset on any external event; only tick-driven changes.
3. Agent with default `MetabolismProfile` has all needs increase after N ticks of no action.
4. `Permille::saturating_add` clamps at 1000 and `Permille::saturating_sub` clamps at 0.
5. Agent performing an action whose `ActionDef.body_cost_per_tick` has `fatigue_delta: Permille(5)` gains extra fatigue beyond basal rate.
6. `DeprivationExposure` counters increment when drive >= critical threshold.
7. `DeprivationExposure` counters reset to 0 when drive falls below critical threshold.
8. Different `MetabolismProfile` values produce different progression rates for the same tick count.
9. `worldwake-systems::dispatch_table()` registers `needs_system` for `SystemId::Needs` without changing noop-fixture behavior elsewhere.
10. Existing suite: `cargo test --workspace`

### Invariants

1. Need values stay within `Permille` range (0..=1000) — never overflow.
2. Physiology progresses regardless of visibility or camera — invariant 9.15.
3. Need changes only through time, action body cost, or explicit effects — invariant 9.16.
4. System reads/writes only through component tables — Principle 12.
5. No stored fear or wellness scores.
6. Action body costs are sourced from `ActionDef.body_cost_per_tick`; this ticket must not introduce duplicate body-cost metadata paths.
7. System-side physiology mutation goes through `WorldTxn`, preserving the append-only event log.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs.rs` (unit tests) — basal progression, body cost application, deprivation counter tracking, dispatch-table registration
2. `crates/worldwake-core/src/numerics.rs` (unit tests) — saturating arithmetic methods
3. `crates/worldwake-core/src/world_txn.rs` (unit tests) — physiology component updates emit the expected deltas
4. `crates/worldwake-sim/src/system_dispatch.rs` and/or `crates/worldwake-sim/src/tick_step.rs` (tests) — extended system context wiring with active actions and action defs

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-core`
3. `cargo test -p worldwake-sim`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `cargo test --workspace`

## Outcome

Completion date: 2026-03-10

Outcome amended: 2026-03-10

What actually changed:

1. Added `Permille::saturating_add` and `Permille::saturating_sub` in `crates/worldwake-core/src/numerics.rs`.
2. Extended `WorldTxn` with physiology-oriented component setters for `HomeostaticNeeds`, `DeprivationExposure`, `MetabolismProfile`, and `DriveThresholds`, so systems and external crates can seed and mutate physiology through the event-sourced boundary.
3. Extended `SystemExecutionContext` to expose read-only `active_actions` and `action_defs`, and updated `tick_step` so system dispatch receives the live scheduler/action-registry view.
4. Implemented `crates/worldwake-systems/src/needs.rs` with basal need progression, action body-cost application, deprivation exposure tracking, and a `dispatch_table()` constructor that wires only the needs slot while keeping other slots noop.
5. Added focused tests across `worldwake-core`, `worldwake-sim`, and `worldwake-systems` for saturation helpers, physiology transaction deltas, system-context wiring, dispatch registration, basal progression, active-action body costs, and deprivation counter behavior.
6. Refined `WorldTxn` again after archival so simple authoritative component setters are generated from a schema-backed macro (`with_txn_simple_set_components`) instead of being hand-added one by one.

Differences from the original plan:

1. The ticket was corrected before implementation because the original wording assumed `SystemExecutionContext` and `WorldTxn` already exposed the data and mutation seams the system needed.
2. `SystemDispatchTable::canonical_noop()` was intentionally left unchanged; a separate `worldwake_systems::dispatch_table()` now provides real wiring without breaking low-level sim tests that rely on a noop fixture.
3. The original proposal to add raw-`u16` `Permille::checked_*` helpers was rejected as poor API design. Only true saturating helpers were added.
4. `WorldTxn` first gained `MetabolismProfile` and `DriveThresholds` setters in addition to the ticket's minimum physiology mutation helpers so external crates can set up full physiology state without weakening `World` visibility.
5. That follow-up was then tightened into the cleaner long-term shape: schema-generated `set_component_*` methods for simple authoritative component replacements, removing the ad hoc physiology-specific setter naming.

Verification results:

1. `cargo test -p worldwake-systems` passed.
2. `cargo test -p worldwake-core` passed.
3. `cargo test -p worldwake-sim` passed.
4. `cargo clippy --workspace --all-targets -- -D warnings` passed.
5. `cargo test --workspace` passed.
