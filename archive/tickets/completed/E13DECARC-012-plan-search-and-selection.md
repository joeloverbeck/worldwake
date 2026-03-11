# E13DECARC-012: Bounded plan search and deterministic plan selection

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None - AI-layer logic and tests
**Deps**: `specs/E13-decision-architecture.md`, completed E13 planner primitives already in `worldwake-ai`

## Problem

E13 already has grounded candidate generation, deterministic ranking, planning budgets, planner-visible op classification, and candidate-scoped hypothetical state. What it still lacks is the layer that turns ranked grounded goals into executable candidate plans and chooses when a new plan is actually worth replacing the current runtime plan.

Without this ticket:

1. ranked goals stop at prioritization and never become exact `InputKind::RequestAction` sequences
2. `PlanningSnapshot` / `PlanningState` have no consumer
3. anti-thrashing policy exists only as budget/config intent, not behavior

## Assumption Reassessment (2026-03-11)

1. `worldwake-ai` already contains `budget.rs`, `goal_model.rs`, `planner_ops.rs`, `planning_snapshot.rs`, `planning_state.rs`, `candidate_generation.rs`, and `ranking.rs`.
2. `worldwake-ai` does not currently contain `search.rs` or `plan_selection.rs`; this ticket must add them rather than modify stubs.
3. `worldwake-ai/Cargo.toml` already includes the `worldwake-sim` dependency; the spec-level Cargo fix has already landed.
4. `BlockedIntentMemory` and `AgentDecisionRuntime` already exist. This ticket must consume `AgentDecisionRuntime`, not redefine runtime state.
5. The live planner taxonomy has one `PlannerOpKind::Trade`, not separate `TradeAcquire` / `TradeSell` aliases.
6. The live action model requires payload synthesis for some executable plan steps:
   - `trade` needs a `TradeActionPayload` override
   - `attack` needs a `CombatActionPayload` override for duration resolution and execution
   - `loot` may carry a `LootActionPayload` mirror of the corpse target
7. `Trade`, `Harvest`, `Craft`, and `Loot` are already marked materialization barriers in `planner_ops.rs`.
8. `Attack` and `Defend` are already marked leaf-only in `planner_ops.rs`, but only `Defend` is indefinite; `Attack` is finite and weapon-dependent.
9. Current candidate generation deliberately does not emit `SellCommodity`, `MoveCargo`, or `BuryCorpse` yet. This ticket should not force speculative search support for deferred goal families just to satisfy enum completeness.
10. `cargo test -p worldwake-ai` is green before this work. This is new planner functionality, not a repair of a failing suite.

## Architecture Check

1. Adding dedicated `search.rs` and `plan_selection.rs` is better than growing `planner_ops.rs` into a god-module. Classification, search, and replacement policy are separate concerns and should stay separate.
2. Search must stay belief-local. Successors still come from `get_affordances()`, but executable step construction also needs payload synthesis for the small set of action families whose real execution requires it.
3. Search should model only conservative planner-visible post-state:
   - travel changes location
   - self-care actions lower the relevant drive below the medium band
   - healing lowers pain below the medium band
   - materialization barriers do not pretend future concrete lot identities already exist
4. The clean robust seam is:
   - `planner_ops.rs`: classification metadata and exact plan-step types
   - `search.rs`: bounded expansion, payload synthesis, conservative state advance, goal satisfaction/progress checks
   - `plan_selection.rs`: deterministic best-plan choice and anti-thrashing
5. This is more beneficial than the current architecture because it finally connects grounded candidates to executable plans without inventing a fake generic action-effects system or leaking authoritative-world assumptions into AI planning.

## Scope Correction

This ticket delivers bounded search and plan selection for the goal families the current Phase 2 code can actually ground and execute lawfully:

- `ConsumeOwnedCommodity`
- `AcquireCommodity`
- `Sleep`
- `Relieve`
- `Wash`
- `ReduceDanger`
- `Heal`
- `ProduceCommodity`
- `RestockCommodity`
- `LootCorpse`

Out of scope:

- plan revalidation against fresh affordances
- blocked-intent writing
- reactive interrupts
- scheduler/agent tick integration
- speculative full support for deferred goal families not emitted by current candidate generation (`SellCommodity`, `MoveCargo`, `BuryCorpse`)
- pretending barrier steps preserve exact future lot identities

## What To Change

### 1. Add `crates/worldwake-ai/src/search.rs`

Implement:

```rust
pub fn search_plan(
    snapshot: &PlanningSnapshot,
    goal: &GroundedGoal,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
    registry: &ActionDefRegistry,
    budget: &PlanningBudget,
) -> Option<PlannedPlan>
```

Required behavior:

1. Start from `PlanningState::new(snapshot)`.
2. Expand nodes deterministically with explicit caps from `PlanningBudget`.
3. Generate successors only via `get_affordances(&planning_state, actor, registry)`.
4. Filter affordances to the current goal family’s relevant `PlannerOpKind`s.
5. Synthesize payload overrides only when execution actually needs them:
   - `TradeActionPayload`
   - `CombatActionPayload`
   - optional `LootActionPayload`
6. Reject branches whose duration cannot be estimated from belief-local data.
7. Treat leaf-only planner ops as valid only when the resulting step ends the plan.
8. Return `None` on budget exhaustion rather than a partial invalid plan.

### 2. Implement conservative goal satisfaction and progress-barrier checks in `search.rs`

This ticket should keep these checks local to search rather than creating a speculative cross-module trait.

Required minimum checks:

- self-care goals are satisfied when the relevant drive falls below the medium band
- `AcquireCommodity` may end at a valid progress barrier once the barrier step would materially acquire the desired commodity locally
- `Heal` is satisfied when pain drops below the medium band
- `ReduceDanger` is satisfied when active attackers/visible hostiles are cleared or pressure falls below the high band in the hypothetical state
- `ProduceCommodity`, `RestockCommodity`, and `LootCorpse` may end at lawful progress barriers instead of chaining through unknown future lot ids

### 3. Add `crates/worldwake-ai/src/plan_selection.rs`

Implement:

```rust
pub fn select_best_plan(
    candidates: &[RankedGoal],
    plans: &[(GoalKey, Option<PlannedPlan>)],
    current: &AgentDecisionRuntime,
    budget: &PlanningBudget,
) -> Option<PlannedPlan>
```

Selection rules:

1. choose highest `GoalPriorityClass`
2. then highest `motive_score`
3. then lowest `total_estimated_ticks`
4. then deterministic lexicographic step ordering

Anti-thrashing rules:

- do not replace the current plan if the current plan still corresponds to a currently ranked goal and the new plan is same-class but does not clear `switch_margin_permille`
- replace immediately if the current plan has no still-ranked goal or if the new plan is strictly higher priority class

### 4. Wire exports through `crates/worldwake-ai/src/lib.rs`

Export the new search and selection entry points without adding backward-compatibility aliases.

### 5. Tighten `PlanningState` locality where needed for hypothetical search

If search reveals that local threat queries do not respect hypothetical movement, correct that in `planning_state.rs` rather than papering over it in search logic.

## Files To Touch

- `crates/worldwake-ai/src/lib.rs`
- `crates/worldwake-ai/src/search.rs` (new)
- `crates/worldwake-ai/src/plan_selection.rs` (new)
- `crates/worldwake-ai/src/planning_state.rs` (targeted fixes only if search exposes locality gaps)
- tests in the above files

## Acceptance Criteria

### Tests That Must Pass

1. Search returns a 1-step consume plan when the actor already has usable food locally.
2. Search returns a travel-then-consume plan when the usable food is at an adjacent place the actor can reach.
3. Search returns a travel-then-trade barrier plan when acquisition requires a reachable seller and executable trade payload.
4. Search respects `max_plan_depth`.
5. Search respects `max_node_expansions`.
6. Search filters to the current goal’s relevant `PlannerOpKind`s.
7. Materialization barriers end plans with `PlanTerminalKind::ProgressBarrier`.
8. Leaf-only combat actions are never allowed as non-terminal middle steps.
9. Duration-estimation failure invalidates the branch.
10. Plan selection prefers higher priority class before motive score or plan cost.
11. Same-class replacement requires the configured switch margin.
12. Selection remains deterministic for identical inputs.
13. `cargo test -p worldwake-ai`
14. `cargo test --workspace`
15. `cargo clippy --workspace`

### Invariants

1. Successors come from `get_affordances()` only; planner-specific logic is limited to filtering, payload synthesis, and conservative hypothetical effects.
2. Search remains deterministic and bounded.
3. No `HashMap` / `HashSet`.
4. No backward-compatibility shims or alias op kinds.
5. Materialization barriers never pretend future exact lot identities already exist.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search.rs`
   Rationale: proves bounded deterministic search, payload synthesis, and conservative barrier handling against the live action model.
2. `crates/worldwake-ai/src/plan_selection.rs`
   Rationale: proves deterministic replacement policy and anti-thrashing behavior.
3. `crates/worldwake-ai/src/planning_state.rs`
   Rationale: guards any locality fix required so hypothetical movement affects local threat queries lawfully.

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

Completed: 2026-03-11
Outcome amended: 2026-03-11

What actually changed:
- Added `crates/worldwake-ai/src/search.rs` with deterministic bounded search over `PlanningState`, using `get_affordances()` as the successor source and synthesizing payload overrides only for executable families that require them (`trade`, `attack`, `loot`).
- Added `crates/worldwake-ai/src/plan_selection.rs` with deterministic best-plan ranking and same-class anti-thrashing based on `switch_margin_permille`.
- Wired both entry points through `crates/worldwake-ai/src/lib.rs`.
- Extended `PlanTerminalKind` so combat leaf commitments are represented explicitly as `CombatCommitment` instead of overloading `GoalSatisfied`.
- Tightened hypothetical locality in `crates/worldwake-ai/src/planning_state.rs` so hostile/attacker queries respect hypothetical movement instead of behaving like stale global threat lists.
- Tightened `crates/worldwake-ai/src/planning_snapshot.rs` so candidate snapshots include place entities themselves, which keeps travel affordances lawful under `TargetExists` and `TargetKind(Place)`.
- Added focused search, selection, and locality tests.

Deviations from original plan:
- The original ticket assumed empty search/selection stubs and outdated planner taxonomy. Those assumptions were corrected first.
- The implementation kept goal satisfaction/progress checks local to `search.rs` instead of extending `planner_ops.rs` with a speculative cross-module trait.
- The final search scope is intentionally aligned to goal families the current candidate generator and action model can execute lawfully today, rather than forcing speculative support for deferred goal kinds.
- `trade` planning is modeled as a materialization barrier with synthesized payload, not as a fake `TradeAcquire` alias op.

Verification:
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace`
