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

- S33 ✅ (opportunity-scoped goal identity — goal identity changes landed; registration covers the final shape)
- S31 ✅ (exhaustion invalidation conditions — registration must eventually own invalidation strategy selection)
- S25 ✅ (feasibility sketching — registration must eventually own feasibility strategy selection)

Not a direct dependency:

- S22 ✅ (intention frames) — live reassessment shows `agent_tick/frame.rs::progress_op_kinds()` is `IntentionDomain`-owned, not goal-registration-owned. S36 must not silently absorb domain registration into goal registration.

## FOUNDATIONS Alignment

- **P3** (Concrete State Over Abstract Scores): `GoalKind` remains the authoritative concrete goal identity. The new declaration key is a derived AI-internal read-model for dispatch, not a replacement source of truth. Strategy selectors route to computations that consume concrete state (thresholds, beliefs, inventories), not to abstract score lookups.
- **P24** (Systems Interact Through State, Not Through Each Other): The declaration table mediates between dispatch consumers (ranking, planner ops, exhaustion, feasibility, trace) without introducing cross-module direct calls. Each consumer reads declaration metadata rather than importing another consumer's logic.
- **P25** (Derived Summaries Are Caches, Never Truth): The dispatch key is derived from `GoalKind`. The declaration table is a static derived read-model over goal dispatch properties. `GoalKind` in `worldwake-core` remains the authoritative identity; the declaration never replaces or competes with it.
- **P26** (No Backward Compatibility): Consolidation removes duplicate dispatch paths. No shim or compatibility layer between old scattered tables and new registration. `GoalKindTag` survives only where a coarse family label is still intentionally the contract, not as a shadow registration path.
- **P27** (Debuggability): Centralized registration makes it trivial to inspect what each goal kind supports. Declaration-owned trace labels replace raw `Debug` formatting where stable labels are the contract. Payload detail is preserved where it matters for answering "why did this agent do that?"
- **P28** (Every System Spec Must Declare Causal Hooks): Registration is declaration, but only at the correct abstraction boundary. Static goal dispatch and dynamic strategy selection should be declared explicitly; domain-owned progress semantics should stay domain-owned until a separate domain-registration design exists.

## Design Goals

1. **Single source of truth**: Each dispatch-distinguishing goal shape declares its AI dispatch properties once, in one place.
2. **Payload-aware completeness**: The declaration substrate must be able to distinguish payload-sensitive static behavior such as `AcquireCommodity` purpose splits and any similar live distinctions.
3. **Static vs dynamic separation**: Static facts belong directly in declarations; dynamic invalidation/feasibility behavior belongs behind declaration-owned strategy selectors, not hard-coded free-floating `match GoalKind` tables. Strategy selectors are compile-time routing decisions that choose which computation runs; the computation itself consumes live state.
4. **Exhaustive matches**: Remove wildcard (`_`) arms from dispatch code where adding a variant or dispatch-distinguishing payload branch should force review.
5. **No behavioral change**: This is a structural refactoring. All existing behavior preserved exactly.
6. **Incremental migration**: Can migrate one dispatch surface at a time to the registration system.

## Architecture Decisions

### Declaration key placement

The payload-aware declaration key lives in `worldwake-ai`, not `worldwake-core`. It is an AI dispatch concern — moving it into core would leak AI refactor structure into the canonical cross-crate goal type without a gameplay reason.

### GoalKindTag survival

`GoalKindTag` may survive only where a deliberately coarse family contract is still the actual contract (e.g., as a human-readable family label in UI). It must not serve as a second competing declaration identity alongside the payload-aware key. The declaration key replaces `GoalKindTag` as the registration substrate, not sits beside it.

### Strategy selectors vs static data

`InvalidationStrategy` and `FeasibilityStrategy` are compile-time strategy selector enums. They do not encode conditions or data — they route to family-specific helper functions that take concrete `GoalKind`, belief views, recipe inputs, threshold data, and target IDs as runtime arguments. This preserves P3: concrete state drives the actual invalidation and feasibility results; the declaration only selects which lawful computation to run.

Example: `derive_invalidation_conditions()` depends on live threshold bands, recipe inputs, target IDs, and baseline snapshots. The strategy selector says "use the AcquireSelfConsume invalidation strategy"; the strategy implementation then inspects the concrete goal payload and belief state to produce conditions.

### Planner-op reverse membership derivation

The `PlannerOpSemantics.relevant_goal_kinds` field and `GOALS_*` arrays in `planner_ops.rs` should be derived by iterating the declaration table and inverting the `relevant_ops` mapping. This replaces the manually-maintained second matrix with a computed one, eliminating a real two-table drift risk. The planner should not maintain a second manually curated matrix for the same static relationship.

### Trace label reality

`decision_trace.rs` currently renders goals via `Debug` formatting (`format!("{:?}", g.kind)`), not via a real label table. The declaration `trace_label` provides the first stable label surface. Decision trace rendering should consume this label where a stable dispatch-family label is the contract, while preserving payload detail where it matters for debugging (P27). The label source itself comes from the declaration; supplementary payload context may still use the concrete `GoalKind`.

## Current Shape (Scattered Dispatch)

Per-goal-kind logic currently lives in:
1. `candidate_generation.rs` — which candidates to emit per goal family. *Note: candidate generation is fundamentally runtime-dynamic (belief-gated, threshold-driven, inventory-dependent, reachability-computed). It uses 9 sequential emit functions driven by agent state, not a dispatch table. It cannot be reduced to static declaration data and is explicitly out of scope for S36.*
2. `ranking.rs` — provenance family / ranking strategy selection plus motive and priority computation
3. `goal_model.rs` — `GoalKindPlannerExt` trait with `relevant_ops()`, `is_satisfied()`, `matches_binding()`, goal-to-op dispatch
4. `exhaustion.rs` — `derive_invalidation_conditions()` per goal kind
5. `feasibility.rs` — `goal_specific_feasibility()` per goal kind
6. `decision_trace.rs` — selected-goal summaries currently fall back to `Debug` formatting rather than a declaration-owned label surface
7. `planner_ops.rs` — reverse goal-membership tables per planner op (`GOALS_*` arrays and `PlannerOpSemantics.relevant_goal_kinds`)
8. `agent_tick/` — intention frame `progress_op_kinds()` per `IntentionDomain` (important: currently domain-owned, not goal-owned)

## Deliverables

### 1. Payload-aware declaration key (worldwake-ai)

Introduce an AI-internal key derived from concrete `GoalKind` that is exhaustive over dispatch-distinguishing goal shapes, not just over coarse `GoalKindTag` variants.

Examples the live code already proves need payload-aware distinction:

- `AcquireCommodity::SelfConsume`
- `AcquireCommodity::Restock`
- `AcquireCommodity::RecipeInput`
- `PunishAccused::Fine`
- `PunishAccused::Exile`
- any other live goal shape where static dispatch differs inside one `GoalKindTag`

If implementation finds more live payload-sensitive static distinctions, include them in-scope rather than leaving a refined ad hoc match behind.

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
}
```

Implementation note: static fields (`trace_label`, `provenance_family`, `relevant_ops`) can land first; strategy selectors (`invalidation_strategy`, `feasibility_strategy`) can be added when the dynamic migration phase lands. Adding fields to the struct is incremental and does not require splitting into multiple types.

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
};
// ... one declaration per dispatch-distinguishing key
```

### 4. Replace scattered dispatch with declaration lookups

For each existing dispatch site, replace the per-goal-kind match with a declaration lookup or declaration-owned strategy selection:

- `ranked_goal_provenance_family(goal_kind)` → `goal_kind.dispatch_key().declaration().provenance_family`
- `relevant_ops(goal_kind)` → `goal_kind.dispatch_key().declaration().relevant_ops`
- planner-op reverse membership → derived from the declaration table by iterating all declarations and inverting the `relevant_ops` mapping, replacing the manually-maintained `GOALS_*` arrays
- trace labels → `goal_kind.dispatch_key().declaration().trace_label` where stable labels are the contract, supplemented by concrete payload context where debugging requires it
- `derive_invalidation_conditions(goal_kind)` → declaration-owned `invalidation_strategy` selects the computation; the computation still takes concrete `GoalKind`, belief views, recipe inputs, and threshold data as runtime arguments
- `goal_specific_feasibility(goal_kind)` → declaration-owned `feasibility_strategy` selects the computation; the computation still takes live local belief state and blocker memory as runtime arguments

Note: Some dispatch sites require runtime payload data and live belief/recipe inputs. These route through declaration-owned strategies, not through static copied data. The strategy selector is the static declaration; the strategy implementation is the runtime computation.

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

## Out of Scope

The following are explicitly excluded from S36:

- **Candidate generation** (`candidate_generation.rs`): Fundamentally runtime-dynamic — driven by homeostatic thresholds, belief state, inventory checks, and reachability computation across 9 sequential emit functions. Cannot be reduced to static declaration data or strategy selection. P3 (Concrete State Over Abstract Scores) directly supports this exclusion: candidate generation derives from actual needs, inventory, and beliefs.
- **`IntentionDomain` progress-op ownership** (`agent_tick/frame.rs::progress_op_kinds()`): Currently domain-owned, keyed by `IntentionDomain`, not by `GoalKind` or `GoalKindTag`. Forcing it into goal registration without a separate domain-registration design would be a category error. Stays domain-owned unless a future design introduces domain registration intentionally.
- **Goal-semantic methods in `GoalKindPlannerExt`**: The following methods are goal *behavior* during planning, not dispatch *routing* properties. They remain as trait method implementations on `GoalKind`:
  - `is_satisfied()` — goal completion evaluation against planning state
  - `matches_binding()` — goal-to-action binding checks
  - `build_payload_override()` — action payload construction from affordances and state
  - `apply_planner_step()` — planning state mutation during search
  - `is_progress_barrier()` — progress identification during plan execution
  - `goal_relevant_places()` — place enumeration for planning heuristics
  - `relevant_observed_commodities()` — commodity extraction for observation/invalidation
  - `prerequisite_places()` — prerequisite location computation
- **Authoritative goal identity changes in `worldwake-core`**: `GoalKind` and `GoalKindTag` definitions stay in `worldwake-core`. The declaration key is a derived AI-internal type in `worldwake-ai`.

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
- **Derived**: payload-aware declaration key from concrete `GoalKind`; all static dispatch results; planner-op reverse membership (computed from declaration table); all dynamic invalidation/feasibility outcomes computed through declaration-owned strategies using live state.

## Migration Strategy

Implementation should proceed in three phases, each independently shippable:

### Phase 1: Declaration key

1. Introduce the payload-aware declaration key (`GoalDispatchKey` or similar) in `worldwake-ai`.
2. Add exhaustive `GoalKind → GoalDispatchKey` lookup that splits only where static dispatch actually differs.
3. Add focused tests proving payload-sensitive shapes map to different keys and payload-insensitive shapes collapse.
4. Verify the lookup is exhaustive: adding a `GoalKind` variant without a key mapping fails compilation.

### Phase 2: Static dispatch migration

1. Create `GoalDispatchDeclaration` struct with static fields (`trace_label`, `provenance_family`, `relevant_ops`).
2. Create one declaration per dispatch-distinguishing key.
3. Migrate static dispatch to declaration lookups:
   - provenance family / ranking strategy selection
   - goal-side relevant ops
   - planner-op reverse membership (derived from declarations, replacing `GOALS_*` arrays)
   - declaration-owned trace labels
4. Run focused unit tests plus full `worldwake-ai` regression coverage.

### Phase 3: Dynamic strategy migration

1. Extend `GoalDispatchDeclaration` with `invalidation_strategy` and `feasibility_strategy` fields.
2. Define strategy selector enums with variants matching the live dispatch families.
3. Refactor `derive_invalidation_conditions()` and `goal_specific_feasibility()` to route through declaration-owned strategy selectors.
4. Family-specific helpers may still branch on concrete payload where the strategy genuinely needs payload facts.
5. After migrated surfaces land, audit remaining wildcard matches and convert to exhaustive where appropriate.
6. Run focused tests plus full `worldwake-ai` regression coverage after each step.

## Tests

### Compile-time tests
- [ ] Adding a `GoalKind` variant without a dispatch-key mapping fails compilation
- [ ] Adding a dispatch key without a corresponding declaration fails compilation
- [ ] Wildcard arms in priority dispatch sites (declaration key, invalidation strategy routing, relevant ops) are removed

### Behavioral equivalence tests
- [ ] All existing golden tests pass unchanged after migration
- [ ] All existing focused/unit tests pass unchanged
- [ ] Declaration-based dispatch produces identical results to pre-migration dispatch for every live dispatch-distinguishing goal shape

### Structural regression tests
- [ ] Payload-sensitive shapes map to different declaration keys where live dispatch differs
- [ ] Payload-insensitive shapes collapse to the same declaration key where live dispatch is intentionally shared
- [ ] Goal-side relevant-op lookup resolves through declaration path
- [ ] Planner-op reverse membership is derived from declarations, not maintained as a manual matrix
- [ ] Provenance-family selection resolves from declaration metadata
- [ ] Decision traces use stable declaration-owned goal labels where labels are part of the contract
- [ ] Invalidation strategy routing resolves through declaration while preserving live invalidation behavior
- [ ] Feasibility strategy routing resolves through declaration while preserving live feasibility behavior

### Documentation tests
- [ ] Dispatch-key lookup and declaration lookup return correct values for spot-checked payload-sensitive and payload-insensitive goal shapes

## Acceptance Criteria

1. Every dispatch-distinguishing goal shape has exactly one declaration entry.
2. Static goal dispatch routes through declarations, and dynamic invalidation/feasibility routing uses declaration-owned strategy selectors.
3. Adding a new `GoalKind` variant without dispatch-key/declaration review fails compilation.
4. All existing golden tests pass unchanged (zero behavioral change).
5. No backward-compatibility shims between old and new dispatch paths.
6. Planner-op reverse membership is derived from declarations, not maintained as a second manual matrix.
7. Declaration-owned trace labels replace raw `Debug` formatting where stable labels are the contract.

## Ticket Note

Previous tickets S36DECGOAL-002, S36DECGOAL-003, and S36DECGOAL-004 were created during S35 implementation. Their insights have been incorporated into this spec. Those tickets should be deleted and new implementation tickets generated from this updated spec, following the three-phase migration strategy above.
