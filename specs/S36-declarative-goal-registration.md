# S36: Declarative Goal Registration

## Summary

Introduce a centralized declarative registration system for `GoalKind` variants that consolidates the per-goal dispatch tables currently scattered across 8+ files and ~734 match sites. Adding a new goal kind currently requires parallel edits to candidate generation, ranking, goal_model, exhaustion, feasibility, intention frame progress, planner op relevance, and trace labeling — with no compile-time enforcement that all required declarations exist. This spec introduces a `GoalRegistration` trait and exhaustive-match enforcement so that incomplete goal declarations fail compilation.

## Source

Derived from ChatGPT architecture review WW-AI-006 (Declarative registration and compile-time completeness), validated against the codebase. The report documented ~734 `GoalKind` match sites. The `reports/ai-decision-architecture-analysis.md` report confirmed "Parallel dispatch on GoalKind requiring 8+ file edits per new goal" as a known issue.

## Phase

Phase 3+: AI Architecture Overhaul, Step 13.5 Wave 5

## Crates

- `worldwake-core` (GoalKind, possible trait definition)
- `worldwake-ai` (registration implementation, dispatch table consolidation)

## Dependencies

- S33 (opportunity-scoped goal identity — goal identity changes should land first so registration covers the final shape)
- S31 ✅ (exhaustion invalidation conditions — registration must include invalidation condition declarations)
- S22 ✅ (intention frames — registration must include progress op declarations)
- S25 ✅ (feasibility sketching — registration must include feasibility hint declarations)

## FOUNDATIONS Alignment

- **P26** (No Backward Compatibility): Consolidation removes duplicate dispatch paths. No shim or compatibility layer between old scattered tables and new registration.
- **P28** (Every System Spec Must Declare Causal Hooks): Registration IS declaration — each goal declares its ranking family, invalidation conditions, planner semantics, and belief requirements in one place.
- **P27** (Debuggability): Centralized registration makes it trivial to inspect what each goal kind supports.

## Design Goals

1. **Single source of truth**: Each goal kind's dispatch properties declared once, in one place.
2. **Compile-time completeness**: Adding a `GoalKind` variant without registering all required properties fails compilation.
3. **Exhaustive matches**: Remove wildcard (`_`) arms from goal-dispatch matches where adding a variant should force review.
4. **No behavioral change**: This is a structural refactoring. All existing behavior preserved exactly.
5. **Incremental migration**: Can migrate one goal kind at a time to the registration system.

## Current Shape (Scattered Dispatch)

Per-goal-kind logic currently lives in:
1. `candidate_generation.rs` — which candidates to emit per goal family
2. `ranking.rs` — ranking family, motive computation, policy evaluation
3. `goal_model.rs` — `GoalKindPlannerExt` trait with `relevant_ops()`, `is_satisfied()`, `matches_binding()`, goal-to-op dispatch
4. `exhaustion.rs` — `derive_invalidation_conditions()` per goal kind
5. `feasibility.rs` — `feasibility_hint()` per goal kind
6. `agent_tick/` — intention frame `progress_op_kinds()` per domain
7. `decision_trace.rs` — trace labels per goal kind
8. `planner_ops.rs` — hypothetical transition per op kind (cross-cuts goals)

## Deliverables

### 1. `GoalKindDeclaration` struct (worldwake-ai)

Rather than a trait with dynamic dispatch (which would require `dyn` complexity), use a static struct with all required fields:

```rust
/// Static declaration of all dispatch properties for a GoalKind variant.
pub struct GoalKindDeclaration {
    /// Human-readable label for traces and debugging.
    pub trace_label: &'static str,
    /// Ranking family (Survival, Danger, Normal, etc.).
    pub ranking_family: RankingFamily,
    /// Which PlannerOpKinds are relevant for this goal (used in search filtering).
    pub relevant_ops: &'static [PlannerOpKind],
    /// PlannerOpKinds that indicate progress for intention frames.
    pub progress_ops: &'static [PlannerOpKind],
    /// Invalidation conditions for exhaustion cache (S31).
    pub invalidation_conditions: &'static [ExhaustionInvalidationCondition],
    /// Whether this goal uses exact binding (S03).
    pub exact_binding: bool,
}
```

### 2. Registration table (worldwake-ai)

A `const fn` or `static` table mapping each `GoalKindTag` to its `GoalKindDeclaration`:

```rust
/// Introduce a tag enum mirroring GoalKind variants without payload.
/// GoalKindTag already exists in the codebase — extend with declaration lookup.
impl GoalKindTag {
    pub const fn declaration(&self) -> &'static GoalKindDeclaration {
        match self {
            GoalKindTag::Eat => &DECL_EAT,
            GoalKindTag::Drink => &DECL_DRINK,
            // ... exhaustive, no wildcard
        }
    }
}

static DECL_EAT: GoalKindDeclaration = GoalKindDeclaration {
    trace_label: "Eat",
    ranking_family: RankingFamily::Survival,
    relevant_ops: &[PlannerOpKind::Consume, PlannerOpKind::Travel],
    progress_ops: &[PlannerOpKind::Consume, PlannerOpKind::Travel],
    invalidation_conditions: &[
        ExhaustionInvalidationCondition::NeedCrossedThreshold {
            need: HomeostaticNeedId::Hunger,
            delta: Permille(100),
        },
        ExhaustionInvalidationCondition::CommodityChanged,
        ExhaustionInvalidationCondition::PositionChanged,
    ],
    exact_binding: false,
};
// ... one declaration per GoalKindTag variant
```

### 3. Replace scattered dispatch with declaration lookups

For each existing dispatch site, replace the per-goal-kind match with a declaration lookup:

- `derive_invalidation_conditions(goal_kind)` → `goal_kind.tag().declaration().invalidation_conditions`
- `relevant_ops(goal_kind)` → `goal_kind.tag().declaration().relevant_ops`
- `progress_op_kinds(domain, goal_kind)` → `goal_kind.tag().declaration().progress_ops`
- Trace labels → `goal_kind.tag().declaration().trace_label`
- Ranking family → `goal_kind.tag().declaration().ranking_family`

Note: Some dispatch sites require runtime payload data (e.g., `is_satisfied()` needs the specific entity/commodity from the goal). These remain as methods on `GoalKindPlannerExt` but with exhaustive matches (no wildcard arms).

### 4. Exhaustive match enforcement

Audit all `match goal_kind { ... _ => ... }` patterns in the AI crate. For each:
- If the wildcard arm provides a meaningful default that's correct for all future variants: Keep it but add a `#[deny(unreachable_patterns)]` lint or explicit comment documenting why.
- If the wildcard arm is a shortcut that should be reviewed per variant: Replace with exhaustive match.

Priority targets (these MUST become exhaustive):
- `derive_invalidation_conditions()` — adding a goal without invalidation rules is a correctness bug
- `relevant_ops()` — adding a goal without planner relevance is a correctness bug
- `feasibility_hint()` — missing feasibility dispatch defaults to `Uncertain` (acceptable as explicit default)

### 5. `GoalKindTag` exhaustiveness

Ensure `GoalKindTag` has a variant for every `GoalKind` variant. The `From<&GoalKind>` conversion must be exhaustive (no wildcard). Adding a `GoalKind` variant without a corresponding `GoalKindTag` variant and declaration fails compilation.

## Component Registration

No new ECS components. This is an AI-internal structural refactoring.

## FND-01 Section H Analysis

### Information-path analysis
N/A — no new information paths. Pure structural refactoring.

### Positive-feedback analysis
N/A — no new feedback loops.

### Concrete dampeners
N/A.

### Stored state vs. derived read-model list
- **Stored**: `GoalKindDeclaration` (compile-time static data).
- **Derived**: All existing dispatch results (now derived from declarations instead of scattered matches).

## Migration Strategy

1. Create `GoalKindDeclaration` struct and one declaration per existing `GoalKindTag` variant.
2. Add `declaration()` method on `GoalKindTag`.
3. Replace dispatch sites one at a time:
   - Start with `derive_invalidation_conditions()` (most mechanically verifiable).
   - Then `relevant_ops()`.
   - Then `progress_ops`.
   - Then trace labels and ranking family.
4. After all dispatch sites migrated, audit remaining wildcard matches and convert to exhaustive where appropriate.
5. Run all golden tests after each dispatch site migration to verify behavioral equivalence.

## Tests

### Compile-time tests
- [ ] Adding a `GoalKindTag` variant without a corresponding `GoalKindDeclaration` fails compilation
- [ ] Adding a `GoalKind` variant without a `GoalKindTag` variant fails compilation
- [ ] Wildcard arms in priority dispatch sites (invalidation, relevant_ops) are removed

### Behavioral equivalence tests
- [ ] All existing golden tests pass unchanged after migration
- [ ] All existing focused/unit tests pass unchanged
- [ ] Declaration-based dispatch produces identical results to pre-migration match-based dispatch for every GoalKindTag variant

### Documentation tests
- [ ] `declaration()` returns correct values for spot-checked goal kinds (at least 5)

## Acceptance Criteria

1. Every `GoalKindTag` variant has exactly one `GoalKindDeclaration`.
2. `derive_invalidation_conditions()` and `relevant_ops()` dispatch through declarations, not scattered matches.
3. Adding a new `GoalKind` variant without declaration fails compilation.
4. All existing golden tests pass unchanged (zero behavioral change).
5. No backward-compatibility shims between old and new dispatch paths.
