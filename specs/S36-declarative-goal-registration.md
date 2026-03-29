# S36: Declarative Goal Registration

## Summary

Introduce a centralized declarative registration system for AI goal dispatch that consolidates the static and strategy-selection tables currently scattered across multiple `worldwake-ai` files. The live code has already shown that `GoalKindTag` is too coarse to serve as the universal declaration key: some dispatch distinctions depend on payload shape inside one `GoalKindTag`. S36 therefore introduces a payload-aware AI-internal declaration key derived from concrete `GoalKind`, a declaration table keyed by that derived key, and exhaustive-match enforcement so incomplete dispatch registration fails compilation.

## Source

Derived from ChatGPT architecture review WW-AI-006 (Declarative registration and compile-time completeness), validated against the codebase. The report documented ~734 `GoalKind` match sites. The `archive/reports/ai-decision-architecture-analysis.md` report confirmed "Parallel dispatch on GoalKind requiring 8+ file edits per new goal" as a known issue.

## Phase

Phase 3+: AI Architecture Overhaul, Step 13.5 Wave 5

## Crates

- `worldwake-ai` (registration implementation, dispatch table consolidation)

## Dependencies

- S33 (opportunity-scoped goal identity — goal identity changes should land first so registration covers the final shape)
- S31 ✅ (exhaustion invalidation conditions — registration must eventually own invalidation strategy selection)
- S25 ✅ (feasibility sketching — registration must eventually own feasibility strategy selection)

Not a direct dependency:

- S22 ✅ (intention frames) — live reassessment shows `agent_tick/frame.rs::progress_op_kinds()` is `IntentionDomain`-owned, not goal-registration-owned. S36 must not silently absorb domain registration into goal registration.

## FOUNDATIONS Alignment

- **P26** (No Backward Compatibility): Consolidation removes duplicate dispatch paths. No shim or compatibility layer between old scattered tables and new registration.
- **P28** (Every System Spec Must Declare Causal Hooks): Registration is declaration, but only at the correct abstraction boundary. Static goal dispatch and dynamic strategy selection should be declared explicitly; domain-owned progress semantics should stay domain-owned until a separate domain-registration design exists.
- **P27** (Debuggability): Centralized registration makes it trivial to inspect what each goal kind supports.
- **P3** (Concrete State Over Abstract Scores): `GoalKind` remains the authoritative concrete goal identity. The new declaration key is a derived AI-internal read-model for dispatch, not a replacement source of truth.

## Design Goals

1. **Single source of truth**: Each dispatch-distinguishing goal shape declares its AI dispatch properties once, in one place.
2. **Payload-aware completeness**: The declaration substrate must be able to distinguish payload-sensitive static behavior such as `AcquireCommodity` purpose splits and any similar live distinctions.
3. **Static vs dynamic separation**: Static facts belong directly in declarations; dynamic invalidation/feasibility behavior belongs behind declaration-owned strategy selectors, not hard-coded free-floating `match GoalKind` tables.
4. **Exhaustive matches**: Remove wildcard (`_`) arms from dispatch code where adding a variant or dispatch-distinguishing payload branch should force review.
5. **No behavioral change**: This is a structural refactoring. All existing behavior preserved exactly.
6. **Incremental migration**: Can migrate one dispatch surface at a time to the registration system.

## Current Shape (Scattered Dispatch)

Per-goal-kind logic currently lives in:
1. `candidate_generation.rs` — which candidates to emit per goal family
2. `ranking.rs` — provenance family / ranking strategy selection plus motive and priority computation
3. `goal_model.rs` — `GoalKindPlannerExt` trait with `relevant_ops()`, `is_satisfied()`, `matches_binding()`, goal-to-op dispatch
4. `exhaustion.rs` — `derive_invalidation_conditions()` per goal kind
5. `feasibility.rs` — `goal_specific_feasibility()` per goal kind
6. `decision_trace.rs` — selected-goal summaries currently fall back to `Debug` formatting rather than a declaration-owned label surface
7. `planner_ops.rs` — reverse goal-membership tables per planner op
8. `agent_tick/` — intention frame `progress_op_kinds()` per `IntentionDomain` (important: currently domain-owned, not goal-owned)

## Deliverables

### 1. Payload-aware declaration key (worldwake-ai)

Introduce an AI-internal key derived from concrete `GoalKind` that is exhaustive over dispatch-distinguishing goal shapes, not just over coarse `GoalKindTag` variants.

Examples the live code already proves need payload-aware distinction:

- `AcquireCommodity::SelfConsume`
- `AcquireCommodity::Restock`
- `AcquireCommodity::RecipeInput`
- any other live goal shape where static dispatch differs inside one `GoalKindTag`

`GoalKind` remains the authoritative concrete identity. The declaration key is a derived dispatch read-model only.

### 2. `GoalDispatchDeclaration` struct (worldwake-ai)

Rather than a trait with dynamic dispatch, use a static declaration struct for the static part of AI goal dispatch plus explicit strategy selectors for dynamic surfaces:

```rust
/// Static declaration of AI dispatch properties for one dispatch-distinguishing goal shape.
pub struct GoalDispatchDeclaration {
    /// Human-readable label for traces and debugging.
    pub trace_label: &'static str,
    /// Structured ranked-goal provenance family, if any.
    pub provenance_family: Option<RankedGoalProvenanceFamily>,
    /// Which PlannerOpKinds are relevant for this goal (used in search filtering).
    pub relevant_ops: &'static [PlannerOpKind],
    /// Which invalidation strategy derives exhaustion conditions/baselines.
    pub invalidation_strategy: InvalidationStrategy,
    /// Which feasibility strategy derives cheap local-likelihood hints.
    pub feasibility_strategy: FeasibilityStrategy,
    /// Whether this goal uses exact binding (S03).
    pub exact_binding: bool,
}
```

### 3. Registration table (worldwake-ai)

A `const fn` or `static` table mapping each declaration key to its declaration:

```rust
impl GoalDispatchKey {
    pub const fn declaration(&self) -> &'static GoalDispatchDeclaration {
        match self {
            GoalDispatchKey::AcquireSelfConsume => &DECL_ACQUIRE_SELF_CONSUME,
            GoalDispatchKey::AcquireRestock => &DECL_ACQUIRE_RESTOCK,
            // ... exhaustive, no wildcard
        }
    }
}

static DECL_ACQUIRE_SELF_CONSUME: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "AcquireCommodity(SelfConsume)",
    provenance_family: Some(RankedGoalProvenanceFamily::Drive),
    relevant_ops: &[PlannerOpKind::Travel, PlannerOpKind::Trade, PlannerOpKind::Harvest],
    invalidation_strategy: InvalidationStrategy::AcquireSelfConsume,
    feasibility_strategy: FeasibilityStrategy::EvidencePlace,
    exact_binding: false,
};
// ... one declaration per dispatch-distinguishing key
```

### 4. Replace scattered dispatch with declaration lookups

For each existing dispatch site, replace the per-goal-kind match with a declaration lookup or declaration-owned strategy selection:

- `ranked_goal_provenance_family(goal_kind)` → `goal_kind.dispatch_key().declaration().provenance_family`
- `relevant_ops(goal_kind)` → `goal_kind.dispatch_key().declaration().relevant_ops`
- planner-op reverse membership → derived from the same declaration table rather than a second manual matrix
- trace labels → `goal_kind.dispatch_key().declaration().trace_label`
- `derive_invalidation_conditions(goal_kind)` → declaration-owned `invalidation_strategy`
- `goal_specific_feasibility(goal_kind)` → declaration-owned `feasibility_strategy`

Out of scope for this spec:

- `IntentionDomain` progress-op ownership. `progress_op_kinds()` remains domain-owned unless a separate future design introduces domain registration intentionally.

Note: Some dispatch sites require runtime payload data and live belief/recipe inputs. These should route through declaration-owned strategies, not through static copied data.

### 5. Exhaustive match enforcement

Audit all `match goal_kind { ... _ => ... }` patterns in the AI crate. For each:
- If the wildcard arm provides a meaningful default that's correct for all future variants: Keep it but add a `#[deny(unreachable_patterns)]` lint or explicit comment documenting why.
- If the wildcard arm is a shortcut that should be reviewed per variant: Replace with exhaustive match.

Priority targets (these MUST become exhaustive):
- declaration-key lookup — adding a goal without dispatch-key review is a correctness bug
- declaration table lookup — adding a dispatch key without declaration is a correctness bug
- `derive_invalidation_conditions()` strategy routing — adding a goal without invalidation-strategy review is a correctness bug
- `relevant_ops()` — adding a goal without planner relevance is a correctness bug
- `goal_specific_feasibility()` strategy routing — missing feasibility-strategy review is a correctness bug even if the default result remains `Uncertain`

### 6. `GoalKindTag` exhaustiveness

`GoalKindTag` should remain exhaustive where the coarse tag is still a legitimate contract, but S36 must not rely on `GoalKindTag` as the universal declaration key. The compile-time completeness target is:

- adding a `GoalKind` variant without updating the dispatch-key lookup fails compilation
- adding a new dispatch-distinguishing key without a declaration fails compilation

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
- **Stored**: `GoalDispatchDeclaration` plus explicit strategy enums (compile-time static data).
- **Derived**: payload-aware declaration key from concrete `GoalKind`; all static dispatch results; all dynamic invalidation/feasibility outcomes computed through declaration-owned strategies.

## Migration Strategy

1. Introduce the payload-aware declaration key and correct the S36 spec assumptions.
2. Create `GoalDispatchDeclaration` and one declaration per dispatch-distinguishing key.
3. Migrate static dispatch first:
   - provenance family / ranking strategy selection
   - goal-side relevant ops
   - planner-op reverse membership
   - declaration-owned trace labels where labels are the contract
4. Migrate dynamic strategy routing:
   - exhaustion invalidation strategy selection
   - feasibility strategy selection
5. After migrated surfaces land, audit remaining wildcard matches and convert to exhaustive where appropriate.
6. Run focused unit tests plus full `worldwake-ai` regression coverage after each migration step.

## Tests

### Compile-time tests
- [ ] Adding a `GoalKind` variant without a dispatch-key mapping fails compilation
- [ ] Adding a dispatch key without a corresponding declaration fails compilation
- [ ] Wildcard arms in priority dispatch sites (declaration key, invalidation strategy routing, relevant ops) are removed

### Behavioral equivalence tests
- [ ] All existing golden tests pass unchanged after migration
- [ ] All existing focused/unit tests pass unchanged
- [ ] Declaration-based dispatch produces identical results to pre-migration dispatch for every live dispatch-distinguishing goal shape

### Documentation tests
- [ ] dispatch-key lookup and declaration lookup return correct values for spot-checked payload-sensitive and payload-insensitive goal shapes

## Acceptance Criteria

1. Every dispatch-distinguishing goal shape has exactly one declaration entry.
2. Static goal dispatch routes through declarations, and dynamic invalidation/feasibility routing uses declaration-owned strategy selectors.
3. Adding a new `GoalKind` variant without dispatch-key/declaration review fails compilation.
4. All existing golden tests pass unchanged (zero behavioral change).
5. No backward-compatibility shims between old and new dispatch paths.
