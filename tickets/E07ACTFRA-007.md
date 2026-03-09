# E07ACTFRA-007: Affordance Struct + get_affordances Query

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines the action query surface
**Deps**: E07ACTFRA-002 (semantic types), E07ACTFRA-003 (ActionDef, ActionDefRegistry), E07ACTFRA-006 (KnowledgeView)
**Dependency Note**: Completed E07 prerequisites are archived under `archive/tickets/`. For this ticket, see `archive/tickets/E07ACTFRA-002-supporting-semantic-types.md` and `archive/tickets/E07ACTFRA-003-action-def-action-def-registry.md`.

## Problem

Agents (human and AI) need to discover which actions are currently available to them. The affordance query evaluates all registered action definitions against the actor's perceived context and returns a deterministically sorted list of `Affordance` values. This is the single query surface used by both human UI and AI planning — spec 6.4 mandates identical pipelines.

## Assumption Reassessment (2026-03-09)

1. Spec 6.4: "The human-controlled agent uses the exact same action query and execution pipeline as NPCs."
2. `ActionDefRegistry` from E07ACTFRA-003 provides iteration over all registered action defs.
3. `KnowledgeView` from E07ACTFRA-006 provides the perceived-context queries.
4. `Constraint`, `TargetSpec`, `Precondition` from E07ACTFRA-002 are the types to evaluate.
5. `ControlSource` exists in core but must NOT influence affordance results (spec 9.12).

## Architecture Check

1. `Affordance` is a value type — not a reference to `ActionDef`. It carries `def_id`, `actor`, `bound_targets`, and an optional explanation.
2. Sorting is deterministic: primary by `ActionDefId`, secondary by bound target IDs. This ensures identical results regardless of internal iteration order.
3. The affordance query is a free function, not a method on any registry or world type — this keeps it composable and testable.

## What to Change

### 1. Create `worldwake-sim/src/affordance.rs`

Define `Affordance`:
```rust
pub struct Affordance {
    pub def_id: ActionDefId,
    pub actor: EntityId,
    pub bound_targets: Vec<EntityId>,
    pub explanation: Option<String>,
}
```

Must derive: `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.
Must impl `Ord`/`PartialOrd` using the deterministic sort key (def_id, then bound_targets).

### 2. Create `worldwake-sim/src/affordance_query.rs`

Implement the core query:
```rust
pub fn get_affordances(
    view: &dyn KnowledgeView,
    actor: EntityId,
    registry: &ActionDefRegistry,
) -> Vec<Affordance>
```

Logic:
1. For each `ActionDef` in the registry:
   a. Evaluate `actor_constraints` against the view
   b. If constraints pass, enumerate valid target bindings from `targets` specs
   c. For each valid binding, evaluate `preconditions` against the view
   d. If all preconditions pass, produce an `Affordance`
2. Sort the result deterministically
3. Return

Also implement constraint/precondition evaluation helpers:
- `fn evaluate_constraint(constraint: &Constraint, actor: EntityId, view: &dyn KnowledgeView) -> bool`
- `fn evaluate_precondition(precondition: &Precondition, actor: EntityId, targets: &[EntityId], view: &dyn KnowledgeView) -> bool`
- `fn enumerate_targets(spec: &TargetSpec, actor: EntityId, view: &dyn KnowledgeView) -> Vec<EntityId>`

### 3. Update `worldwake-sim/src/lib.rs`

Declare modules, re-export `Affordance` and `get_affordances`.

## Files to Touch

- `crates/worldwake-sim/src/affordance.rs` (new)
- `crates/worldwake-sim/src/affordance_query.rs` (new)
- `crates/worldwake-sim/src/lib.rs` (modify)

## Out of Scope

- Start gate / action execution (E07ACTFRA-008)
- Commit-time re-evaluation (E07ACTFRA-009)
- Handler logic (E07ACTFRA-005)
- Belief-backed KnowledgeView (E14)
- Registering concrete game actions (later epics)

## Acceptance Criteria

### Tests That Must Pass

1. **T05**: Affordance query never returns an action with false start preconditions in the acting view
2. Affordance results are sorted deterministically by (def_id, bound_targets)
3. **T12 partial**: Affordance results are identical regardless of `ControlSource` — test with Human, Ai, None on the same entity and assert equal affordances
4. An action with unsatisfied actor constraints does not appear in affordances
5. An action with no valid target bindings does not appear in affordances
6. `Affordance` satisfies `Clone + Eq + Ord + Debug + Serialize + DeserializeOwned`
7. Existing suite: `cargo test --workspace`

### Invariants

1. Spec 9.12: affordance query does not branch on `ControlSource`
2. Spec 6.4: human and AI code use the same affordance query
3. Affordance ordering is deterministic (same inputs → same order)
4. Query depends only on `KnowledgeView`, never on `&World` directly

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/affordance.rs` — trait assertions, sort order tests
2. `crates/worldwake-sim/src/affordance_query.rs` — integration tests with WorldKnowledgeView: constraint filtering, precondition filtering, target enumeration, deterministic ordering, ControlSource independence

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace && cargo test --workspace`
