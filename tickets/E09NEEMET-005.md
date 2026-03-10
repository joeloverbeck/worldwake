# E09NEEMET-005: Metabolism system — basal progression and action body costs

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — first system implementation in worldwake-systems, Cargo.toml dep update
**Deps**: E09NEEMET-001, E09NEEMET-002, E09NEEMET-003, E09NEEMET-004

## Problem

The metabolism system must run once per tick for all living agents, applying basal physiological progression from `MetabolismProfile` and action body costs from active actions. This is the core tick-driven engine that makes needs change over time, giving the survival loop its teeth.

## Assumption Reassessment (2026-03-10)

1. `SystemId::Needs` already exists in `system_manifest.rs` (ordinal 0) — confirmed.
2. `SystemDispatchTable` expects a `SystemFn` signature: `fn(SystemExecutionContext<'_>) -> Result<(), SystemError>` — confirmed.
3. `SystemExecutionContext` provides `world`, `event_log`, `rng`, `tick`, `system_id` — confirmed.
4. `worldwake-systems/Cargo.toml` currently depends only on `worldwake-core` — must add `worldwake-sim`.
5. `Scheduler` tracks active `ActionInstance`s with `remaining_ticks` — the metabolism system needs to read active action state to apply body costs.
6. `Permille` arithmetic needs saturating add — `Permille::new()` validates range but there's no built-in saturating addition. May need to add `saturating_add` / `saturating_sub` to `Permille`.

## Architecture Check

1. The needs system is the first real `SystemFn` implementation — it establishes the pattern for all future systems (E10 production, E12 combat, etc.).
2. The system reads components from `World` and writes back via `WorldTxn` or direct mutable access — following E07/E08 patterns.
3. Body cost per tick is read from the active action's metadata. The mechanism for how `BodyCostPerTick` attaches to an action needs to be established here — likely via `ActionDef` extension or a lookup table keyed by `ActionDefId`.
4. Deprivation consequence logic is split into a separate ticket (E09NEEMET-006) to keep this one focused on the basic tick progression.

## What to Change

### 1. Update `crates/worldwake-systems/Cargo.toml`

Add `worldwake-sim = { path = "../worldwake-sim" }` dependency.

### 2. Add `Permille` arithmetic helpers in `crates/worldwake-core/src/numerics.rs`

Add methods to `Permille`:
- `saturating_add(self, other: Permille) -> Permille` — clamps at 1000
- `saturating_sub(self, other: Permille) -> Permille` — clamps at 0
- `checked_add(self, delta: u16) -> Permille` — add raw u16, clamp at 1000
- `checked_sub(self, delta: u16) -> Permille` — sub raw u16, clamp at 0

### 3. New module `crates/worldwake-systems/src/needs.rs`

Implement `pub fn needs_system(ctx: SystemExecutionContext<'_>) -> Result<(), SystemError>`:

Per tick, for each entity with `AgentData + HomeostaticNeeds + MetabolismProfile + DriveThresholds`:

1. **Basal progression**: Add `MetabolismProfile` rates to `HomeostaticNeeds` fields using saturating arithmetic.
2. **Action body cost**: If the agent has an active action with `BodyCostPerTick`, apply those deltas.
3. **Clamp**: All `Permille` values stay in valid range (handled by saturating ops).
4. **Update DeprivationExposure**: For each drive, if current value >= critical threshold from `DriveThresholds`, increment the corresponding `_critical_ticks` counter. Otherwise, reset it to 0.

### 4. Mechanism for BodyCostPerTick on actions

Define a lookup function or registry that maps `ActionDefId` to `BodyCostPerTick`. Options:
- A) Add `body_cost: Option<BodyCostPerTick>` field to `ActionDef` — simplest, extends the existing struct.
- B) A separate `BTreeMap<ActionDefId, BodyCostPerTick>` — more decoupled.

Recommend option A for simplicity. This requires modifying `crates/worldwake-sim/src/action_def.rs`.

### 5. Wire into `SystemDispatchTable`

Update the canonical noop table (or provide a real registration path) so that `SystemId::Needs` maps to `needs_system`.

### 6. Export from `crates/worldwake-systems/src/lib.rs`

Add `pub mod needs;` and re-export `needs_system`.

## Files to Touch

- `crates/worldwake-systems/Cargo.toml` (modify — add `worldwake-sim` dep)
- `crates/worldwake-systems/src/lib.rs` (modify — add module)
- `crates/worldwake-systems/src/needs.rs` (new — metabolism system implementation)
- `crates/worldwake-core/src/numerics.rs` (modify — add `Permille` saturating arithmetic)
- `crates/worldwake-sim/src/action_def.rs` (modify — add `body_cost: Option<BodyCostPerTick>` field)

## Out of Scope

- Deprivation consequences / wound generation (E09NEEMET-006)
- Forced collapse / involuntary relief (E09NEEMET-006)
- Consumption / care action handlers (E09NEEMET-007)
- Sleep recovery logic (E09NEEMET-007)
- AI decision-making based on needs (E13)

## Acceptance Criteria

### Tests That Must Pass

1. **T15: Need progression** — hunger/thirst/fatigue/bladder/dirtiness values evolve by simulation tick.
2. **T26: Camera independence** — physiology does not reset on any external event; only tick-driven changes.
3. Agent with default `MetabolismProfile` has all needs increase after N ticks of no action.
4. `Permille` saturating_add clamps at 1000, saturating_sub clamps at 0.
5. Agent performing an action with `BodyCostPerTick { fatigue_delta: Permille(5), .. }` gains extra fatigue beyond basal rate.
6. `DeprivationExposure` counters increment when drive >= critical threshold.
7. `DeprivationExposure` counters reset to 0 when drive falls below critical threshold.
8. Different `MetabolismProfile` values produce different progression rates for the same tick count.
9. Existing suite: `cargo test --workspace`

### Invariants

1. Need values stay within `Permille` range (0..=1000) — never overflow.
2. Physiology progresses regardless of visibility or camera — invariant 9.15.
3. Need changes only through time, action body cost, or explicit effects — invariant 9.16.
4. System reads/writes only through component tables — Principle 12.
5. No stored fear or wellness scores.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs.rs` (unit tests) — basal progression, body cost application, deprivation counter tracking, saturation behavior
2. `crates/worldwake-core/src/numerics.rs` (unit tests) — saturating arithmetic methods

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test -p worldwake-core`
3. `cargo clippy --workspace`
