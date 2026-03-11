# E13DECARC-010: Planner op kinds, action semantics table, and goal-to-op mapping

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None - AI-layer types and logic
**Deps**: E13DECARC-004, E13DECARC-009

## Problem

E13 still needs a planner-local way to classify real registered action definitions into a small set of planner-visible operation families. The engine does not expose declarative action effects, so the planner cannot infer semantics generically from `ActionDef`.

Without a semantics layer:

1. the planner cannot tell which affordances are relevant to a goal family
2. materialization barriers cannot be expressed consistently
3. future plan search would have to hardcode raw action names and domains at every callsite

## Assumption Reassessment (2026-03-11)

1. `ActionDefId` is a sequential index in `ActionDefRegistry` - confirmed in `crates/worldwake-sim/src/action_def_registry.rs`.
2. `ActionDef` has `id`, `name`, `domain`, `payload`, and other concrete execution metadata, but no declarative `effects` field - confirmed in `crates/worldwake-sim/src/action_def.rs`.
3. `GoalKind` is owned by `worldwake-core`, not `worldwake-ai` - confirmed in `crates/worldwake-core/src/goal.rs`.
4. `worldwake-ai/src/planner_ops.rs` already exists from E13DECARC-009 and currently defines `PlannedStep`, `PlannedPlan`, and `PlanTerminalKind`.
5. Current registered Phase 2 action families are:
   - `travel`
   - `eat`, `drink`, `sleep`, `toilet`, `wash`
   - `trade`
   - recipe-backed `harvest:*` and `craft:*`
   - `pick_up`, `put_down`
   - `attack`, `defend`, `heal`, `loot`
6. There is no current bury action definition or `commerce_actions.rs`; `BuryCorpse` remains a goal identity without an executable action family yet.
7. Trade is a single action family (`trade`) whose acquire-vs-sell intent is determined by payload and goal context, so a table keyed only by `ActionDefId` cannot lawfully distinguish `TradeAcquire` from `TradeSell`.
8. `GoalKind::relevant_op_kinds()` cannot be added as an inherent method in `worldwake-ai` because `GoalKind` is defined in another crate. This must be an AI-side extension trait instead.
9. `GoalSemantics` stubs would be premature here because `PlanningState` is not implemented yet. Defining a dead trait now would add speculative API surface without real integration value.

## Architecture Check

1. This remains planner-local. `ActionDef` stays execution-facing and engine-agnostic.
2. The op taxonomy must reflect real action families, not speculative aliases. In particular:
   - use `Trade`, not separate `TradeAcquire` / `TradeSell`
   - include `Defend`, because it exists today and matters to danger handling
   - do not add `Bury` until a real bury action family exists
3. `PlannedStep` should gain `op_kind: PlannerOpKind`. That keeps the chosen planner semantics attached to the exact step at planning time and avoids repeated reclassification later.
4. The semantics table should be built from actual registered defs using stable domain/name/payload-shape matching, not from registry index assumptions.
5. Goal-to-op mapping should live in `worldwake-ai` as an extension trait on `GoalKind`, preserving the dependency direction: `worldwake-core` stays AI-agnostic.
6. Unknown defs should not silently be forced into fake op kinds. The table should classify only known planner-visible Phase 2 families, while tests should verify full coverage for the assembled Phase 2 registry.

## Proposed Scope Decision

The proposed direction is still more beneficial than the current architecture, with three corrections:

- Add a real planner semantics layer now. That is the right abstraction seam for bounded search, revalidation, and materialization barriers.
- Replace the speculative `TradeAcquire` / `TradeSell` split with a single `Trade` op kind. The codebase has one `trade` action family, and pretending otherwise would bake in an alias the engine does not have.
- Do not add `GoalSemantics` stubs in this ticket. A thin, accurate semantics table plus goal-to-op mapping is useful now; a trait with no lawful state model behind it is not.

## What to Change

### 1. Define `PlannerOpKind` in `worldwake-ai/src/planner_ops.rs`

```rust
pub enum PlannerOpKind {
    Travel,
    Consume,
    Sleep,
    Relieve,
    Wash,
    Trade,
    Harvest,
    Craft,
    MoveCargo,
    Heal,
    Loot,
    Attack,
    Defend,
}
```

Notes:

- `Consume` covers both `eat` and `drink`
- `MoveCargo` covers both `pick_up` and `put_down`
- `Trade` covers both acquisition and sale use-cases

### 2. Define `PlannerOpSemantics`

```rust
pub struct PlannerOpSemantics {
    pub op_kind: PlannerOpKind,
    pub may_appear_mid_plan: bool,
    pub is_materialization_barrier: bool,
    pub relevant_goal_kinds: &'static [GoalKindTag],
}
```

Where `GoalKindTag` is a fieldless AI-side enum mirroring `GoalKind` variants by family.

### 3. Build the semantics table from the live registry

```rust
pub fn build_semantics_table(
    registry: &ActionDefRegistry,
) -> BTreeMap<ActionDefId, PlannerOpSemantics>
```

Classification rules must match the actual registered defs:

- `Travel / "travel"` -> `PlannerOpKind::Travel`
- `Needs / "eat"` and `Needs / "drink"` -> `PlannerOpKind::Consume`
- `Needs / "sleep"` -> `PlannerOpKind::Sleep`
- `Needs / "toilet"` -> `PlannerOpKind::Relieve`
- `Needs / "wash"` -> `PlannerOpKind::Wash`
- `Trade / "trade"` -> `PlannerOpKind::Trade`
- `Production / payload Harvest(_)` and `name.starts_with("harvest:")` -> `PlannerOpKind::Harvest`
- `Production / payload Craft(_)` and `name.starts_with("craft:")` -> `PlannerOpKind::Craft`
- `Transport / "pick_up"` and `Transport / "put_down"` -> `PlannerOpKind::MoveCargo`
- `Care / "heal"` -> `PlannerOpKind::Heal`
- `Loot / "loot"` -> `PlannerOpKind::Loot`
- `Combat / "attack"` -> `PlannerOpKind::Attack`
- `Combat / "defend"` -> `PlannerOpKind::Defend`

Barrier rules:

- `Trade`, `Harvest`, `Craft`, and `Loot` are materialization barriers
- `Travel`, `Consume`, `Sleep`, `Relieve`, `Wash`, `MoveCargo`, `Heal`, `Attack`, and `Defend` are not

Leaf / mid-plan rules:

- `Attack` and `Defend` are leaf-only in Phase 2
- all other current op kinds may appear mid-plan

### 4. Add a goal-to-op extension trait in `worldwake-ai/src/goal_model.rs`

```rust
pub trait GoalKindPlannerExt {
    fn goal_kind_tag(&self) -> GoalKindTag;
    fn relevant_op_kinds(&self) -> &'static [PlannerOpKind];
}
```

Implement it for `worldwake_core::GoalKind`.

This mapping must reflect current architecture:

- `ConsumeOwnedCommodity` -> `Consume`, `Travel`, `Trade`, `Harvest`, `Craft`, `MoveCargo`
- `AcquireCommodity` -> `Travel`, `Trade`, `Harvest`, `Craft`, `MoveCargo`
- `Sleep` -> `Sleep`, `Travel`
- `Relieve` -> `Relieve`, `Travel`
- `Wash` -> `Wash`, `Travel`, `MoveCargo`
- `ReduceDanger` -> `Travel`, `Attack`, `Defend`, `Heal`
- `Heal` -> `Travel`, `Heal`, `Trade`, `Craft`
- `ProduceCommodity` -> `Travel`, `Craft`, `MoveCargo`
- `SellCommodity` -> `Travel`, `Trade`, `MoveCargo`
- `RestockCommodity` -> `Travel`, `Trade`, `Harvest`, `Craft`, `MoveCargo`
- `MoveCargo` -> `Travel`, `MoveCargo`
- `LootCorpse` -> `Travel`, `Loot`
- `BuryCorpse` -> `&[]` for now because there is no executable bury action family yet

### 5. Extend `PlannedStep` with `op_kind`

```rust
pub struct PlannedStep {
    pub def_id: ActionDefId,
    pub targets: Vec<EntityId>,
    pub payload_override: Option<ActionPayload>,
    pub op_kind: PlannerOpKind,
    pub estimated_ticks: u32,
    pub is_materialization_barrier: bool,
}
```

This keeps the exact executable affordance identity from E13DECARC-009 while also preserving the planner's semantic classification.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/goal_model.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify for exports if needed)

## Out of Scope

- `PlanningSnapshot` / `PlanningState` - E13DECARC-011
- search-node state updates per op kind - E13DECARC-011
- generic goal satisfaction / progress-barrier traits - defer until `PlanningState` exists
- plan search algorithm - E13DECARC-012
- any engine-side `ActionDef` schema changes
- adding a bury action family or candidate generation for deferred goal kinds

## Acceptance Criteria

### Tests That Must Pass

1. `PlannerOpKind` has exactly 13 variants.
2. `build_semantics_table()` classifies all currently registered Phase 2 planner-visible action defs in an assembled registry built from the existing registration functions.
3. `Attack` and `Defend` have `may_appear_mid_plan = false`.
4. `Trade`, `Harvest`, `Craft`, and `Loot` have `is_materialization_barrier = true`.
5. `Travel`, `Consume`, `Sleep`, `Relieve`, `Wash`, `MoveCargo`, `Heal`, `Attack`, and `Defend` have `is_materialization_barrier = false`.
6. `GoalKind::ConsumeOwnedCommodity.relevant_op_kinds()` includes `Consume` and `Travel` and does not include `Attack`.
7. `GoalKind::ReduceDanger.relevant_op_kinds()` includes `Travel`, `Attack`, `Defend`, and `Heal`.
8. `GoalKind::RestockCommodity.relevant_op_kinds()` includes `Travel`, `Trade`, `Harvest`, `Craft`, and `MoveCargo`.
9. `GoalKind::BuryCorpse.relevant_op_kinds()` is empty until a bury action family exists.
10. `PlannedStep::to_request_action(actor)` still preserves exact `def_id`, ordered `targets`, and `payload_override` after adding `op_kind`.
11. Table uses `BTreeMap`, not `HashMap`.
12. Existing suite: `cargo test -p worldwake-ai`, `cargo test --workspace`, `cargo clippy --workspace`.

### Invariants

1. Semantics table remains planner-local and does not modify `ActionDef`.
2. `Trade` is modeled as one real action family, not split into fake alias op kinds.
3. `Attack` and `Defend` are leaf-only in Phase 2.
4. Materialization barriers are tagged only for action families that can lawfully invalidate future target binding.
5. No Phase 3+ speculative op kinds are added.
6. No `HashMap` / `HashSet`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` - assembled registry coverage, barrier/leaf assertions, `PlannedStep` conversion after `op_kind` addition
2. `crates/worldwake-ai/src/goal_model.rs` - goal-tag conversion and relevant-op-kind coverage

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-11
- What actually changed:
  - corrected the ticket assumptions before implementation to match the real action registry, crate ownership, and current deferred goal coverage
  - added `PlannerOpKind`, `PlannerOpSemantics`, and `build_semantics_table()` in `crates/worldwake-ai/src/planner_ops.rs`
  - classified the live Phase 2 action families as `Travel`, `Consume`, `Sleep`, `Relieve`, `Wash`, `Trade`, `Harvest`, `Craft`, `MoveCargo`, `Heal`, `Loot`, `Attack`, and `Defend`
  - extended `PlannedStep` with `op_kind` while preserving exact request-action conversion
  - added `GoalKindTag` plus the `GoalKindPlannerExt` AI-side extension trait in `crates/worldwake-ai/src/goal_model.rs`
- Deviations from original plan:
  - replaced speculative `TradeAcquire` / `TradeSell` op kinds with one real `Trade` op kind because the engine exposes one `trade` action family
  - included `Defend`, which already exists in the current combat action set and is relevant to `ReduceDanger`
  - did not add `Bury` or `GoalSemantics` stubs because there is no bury action family yet and `PlanningState` does not exist
  - kept goal-to-op mapping in `worldwake-ai` as an extension trait instead of trying to add an inherent method to `worldwake-core::GoalKind`
- Verification results:
  - `cargo test -p worldwake-ai` passed
  - `cargo test --workspace` passed
  - `cargo clippy --workspace` passed
