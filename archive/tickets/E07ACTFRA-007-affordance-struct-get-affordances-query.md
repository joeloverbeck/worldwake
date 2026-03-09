# E07ACTFRA-007: Affordance Struct + get_affordances Query

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — defines the action query surface
**Deps**: E07ACTFRA-002 (semantic types), E07ACTFRA-003 (ActionDef, ActionDefRegistry), E07ACTFRA-006 (KnowledgeView)
**Dependency Note**: Completed E07 prerequisites are archived under `archive/tickets/`. For this ticket, see `archive/tickets/E07ACTFRA-002-supporting-semantic-types.md`, `archive/tickets/E07ACTFRA-003-action-def-action-def-registry.md`, and `archive/tickets/E07ACTFRA-006-knowledge-view-trait.md`.

## Problem

Agents (human and AI) need to discover which actions are currently available to them. The affordance query evaluates all registered action definitions against the actor's perceived context and returns a deterministically sorted list of `Affordance` values. This is the single query surface used by both human UI and AI planning — spec 6.4 mandates identical pipelines.

## Assumption Reassessment (2026-03-09)

1. The active E07 spec is [`specs/E07-action-framework.corrected.md`](/home/joeloverbeck/projects/worldwake/specs/E07-action-framework.corrected.md), not `specs/E07ACTFRA-*.md`.
2. `ActionDefRegistry` from archived ticket E07ACTFRA-003 already provides deterministic iteration over registered `ActionDef` values.
3. `KnowledgeView` from archived ticket E07ACTFRA-006 already exposes the exact read-only queries needed for Phase 1 affordance legality checks: liveness, kind, effective place, colocated entities, commodity quantity, control presence, and reservation conflicts.
4. `Constraint`, `TargetSpec`, and `Precondition` from archived ticket E07ACTFRA-002 are the data model to evaluate. Their current Phase 1 variants are intentionally narrow and fully concrete.
5. `ControlSource` in core must not create separate human-vs-AI legality branches. However, the existing `Constraint::ActorHasControl` and `KnowledgeView::has_control()` intentionally distinguish `ControlSource::None` from actively controlled agents. This ticket must preserve that distinction instead of over-specifying full `Human == Ai == None` equivalence.
6. `worldwake-sim` currently has no `affordance.rs` or `affordance_query.rs`; this ticket is still the first implementation of the affordance query surface.

## Architecture Check

1. `Affordance` is a value type — not a reference to `ActionDef`. It carries `def_id`, `actor`, `bound_targets`, and an optional explanation.
2. Sorting is deterministic: primary by `ActionDefId`, secondary by bound target IDs. This ensures identical results regardless of internal iteration order.
3. The affordance query is a free function, not a method on any registry or world type — this keeps it composable and testable.
4. The current architecture is missing exactly one clean seam between static action schema and later lifecycle work: a pure affordance evaluator over `KnowledgeView`. Adding that seam is more beneficial than folding legality checks into `WorldKnowledgeView`, `ActionDefRegistry`, or future start/commit code because it preserves spec 6.4 symmetry and keeps `ActionDef` semantics evaluable without direct world coupling.
5. This ticket should not invent a second legality vocabulary. If later tickets need shared start/commit legality helpers, they should reuse the affordance evaluator's pure constraint/precondition machinery rather than duplicate it.

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

Implementation notes for scope control:
- `enumerate_targets` should return only currently valid entity bindings. For `SpecificEntity`, archived/nonexistent entities should not bind. For `EntityAtActorPlace`, bindings should be filtered by current co-location and kind.
- Multi-target actions should enumerate the deterministic cartesian product of per-slot bindings, then filter by preconditions.
- Duplicate affordances for the same `(def_id, actor, bound_targets)` should not survive the final result.
- This ticket stops at query-time legality. Reservation checks stay out of scope until start-gate work unless a precondition/constraint later models them explicitly.

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
3. **T12 partial**: Affordance results are identical for `ControlSource::Human` and `ControlSource::Ai` on the same world state
4. `ControlSource::None` only changes affordance results when an action definition explicitly uses `Constraint::ActorHasControl`
5. An action with unsatisfied actor constraints does not appear in affordances
6. An action with no valid target bindings does not appear in affordances
7. An action with malformed target-index preconditions does not appear in affordances and does not panic
8. `Affordance` satisfies `Clone + Eq + Ord + Debug + Serialize + DeserializeOwned`
9. Existing suite: `cargo test --workspace`

### Invariants

1. Spec 9.12: affordance query does not branch on human-vs-AI player status
2. Spec 6.4: human and AI code use the same affordance query
3. Affordance ordering is deterministic (same inputs → same order)
4. Query depends only on `KnowledgeView`, never on `&World` directly

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/affordance.rs` — trait assertions, sort order tests
2. `crates/worldwake-sim/src/affordance_query.rs` — unit tests for constraint/precondition helpers and deterministic enumeration/deduping
3. `crates/worldwake-sim/src/affordance_query.rs` — integration tests with `WorldKnowledgeView`: constraint filtering, precondition filtering, target enumeration, Human-vs-Ai symmetry, and `ControlSource::None` behavior only when `ActorHasControl` is part of the schema

### Commands

1. `cargo test -p worldwake-sim`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completed: 2026-03-09
- What actually changed:
  - Added `crates/worldwake-sim/src/affordance.rs` with the serializable `Affordance` value type and deterministic ordering behavior.
  - Added `crates/worldwake-sim/src/affordance_query.rs` with pure affordance evaluation over `KnowledgeView`, including actor-constraint checks, target enumeration, precondition checks, deterministic sorting, and duplicate-result collapse.
  - Updated `crates/worldwake-sim/src/lib.rs` to export the new affordance APIs.
  - Corrected the ticket before implementation so its assumptions now match the current E07 spec path, archived prerequisite locations, and the real `ControlSource` semantics in `KnowledgeView`.
- Changed vs. the original plan:
  - Tightened the control-source invariant from the overbroad `Human == Ai == None` claim to the correct rule: affordance legality must not branch on human-vs-AI status, while `ControlSource::None` remains meaningful only when an action explicitly uses `Constraint::ActorHasControl`.
  - Added explicit coverage for malformed precondition target indices so bad schema data fails closed instead of panicking.
  - Kept the affordance evaluator as a pure free-function layer over `KnowledgeView` rather than coupling legality checks to `WorldKnowledgeView` or `ActionDefRegistry`. That is the cleaner long-term seam for reusing the same legality rules in later start/commit work.
- Verification:
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
