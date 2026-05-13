# S137PLACAULIN-006: plan_repair module — bounded localized repair search

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — new `plan_repair` module, planner emitter populates causal_links, classify_accepted_repair preserved as fall-through
**Deps**: archive/tickets/S137PLACAULIN-001.md (CausalLink, BreachSignature), archive/tickets/S137PLACAULIN-002.md (repair_budget_fraction, causal_links_per_step_cap), archive/tickets/S137PLACAULIN-003.md (new RepairKind variants), archive/tickets/S137PLACAULIN-004.md (completed PlanGuard.causal_links field), 005 (RepairMemory shape)

## Problem

S137 D5 introduces the `plan_repair` module — `PlanRepairContext`, bounded repair search, the typed `RepairKind` set emission, and the integration with `AgentDecisionRuntime.pending_repair_context`. This is the architectural shape S137 lands: bounded localized repair runs before full replan when a guard invalidator breaches. D1 (DiscrepancyClearing variant extension) is subsumed here — the audit of "do we need new clearing variants?" can only be done in the repair-search implementation context; if existing 5 variants suffice, no extension lands.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The `plan_repair` module does not exist — confirmed by `find crates/ -name "plan_repair*"` returning 0 matches. Existing infrastructure that S137 D5 builds on: `AgentDecisionRuntime.pending_repair_context: Option<PendingRepairContext>` (referenced from `crates/worldwake-ai/src/agent_tick/planning.rs:1222, 1366, 1457, 1540, 1593`), `classify_accepted_repair` at planning.rs:1452-1526 (post-hoc classifier — preserved as fall-through). Existing tests: `clear_current_plan_parks_committed_trade_goal_into_pending_repair:3531`, `resume_pending_repair_plan_restores_failed_trade_plan_when_counterparty_trigger_revives:3619`.
2. Spec `specs/S137-plan-causal-links-and-repair.md` D5 specifies module shape (`PlanRepairContext`, `RepairKind` search-axis 5 variants, `RepairOutcome`, `RepairFailure`, `attempt_repair_then_replan`). DiscrepancyClearing audit (D1, subsumed) reads existing 5 variants at `crates/worldwake-core/src/discrepancy.rs:57-70`.
3. Shared boundary: the `AgentDecisionRuntime.pending_repair_context` field. The new pre-failure repair path WRITES this field with a context capturing the breach; ticket 007's revalidation routing READS it via `attempt_repair_then_replan`. The existing post-hoc `classify_accepted_repair` (planning.rs:1452-1526) continues to consume this same field after the new path falls through to full replan. Per `references/worldwake-validation-patterns.md` Multi-Substrate Hook Coverage, the two paths share substrate without contradicting each other (FND-28-compatible coexistence: the pre-failure path is the primary surface; post-hoc classification is the fall-through when repair fails).
4. **Planner-driven ticket — live GoalKind under test**: golden coverage in ticket 010 exercises `GoalKind::TravelTo`, `GoalKind::Trade`, `GoalKind::ProduceCommodity` (the goal families that the existing post-hoc repair classifier handles). The new repair search operates over the same goal families — no goal-family addition in this ticket. The operator surfaces are: `PlannerOpKind::Travel`, `PlannerOpKind::Trade`, `PlannerOpKind::HarvestResource`, `PlannerOpKind::CraftRecipe` (existing planner ops). The bounded repair search uses `apply_effects_with_context` (`crates/worldwake-sim/src/effect_schema.rs`) for hypothetical evaluation — confirmed reachable from worldwake-ai per existing dependency.
5. **Ordering claim — repair vs. full replan**: the contract is "repair attempts run before full replan at the revalidation seam." Repair search runs synchronously within the same `agent_tick` call as the breach; if repair returns `Failed`, the agent immediately falls through to `handle_current_step_failure` in the same tick. No tick separation. The ordering is action-lifecycle-internal — repair and full replan are not separate authoritative actions; they are alternative paths inside the agent's tick step.
6. **Heuristic-removal note**: this ticket does NOT remove any existing heuristic. `classify_accepted_repair`'s post-hoc classification remains intact — it runs after `RepairOutcome::Failed` triggers full replan. The architectural addition is the pre-failure repair attempt; the post-hoc classifier is preserved because it provides the variant labeling for the full-replan path (where the new `RebindTarget` / `ReplaceProvider` semantics still apply).
7. **Adjacent contradiction**: existing `PendingRepairContext` (referenced by planning.rs:1222 etc.) was designed for the post-hoc classification path. The new pre-failure path may overload or coexist with it. Reassessment during implementation must determine: (a) is the existing struct shape sufficient for pre-failure breach data, or (b) does the new path need a parallel field on `AgentDecisionRuntime`? If (b), introduce `pre_failure_repair_context: Option<PreFailureRepairContext>` rather than mutating the existing field's contract. Classified as a required consequence — not deferred.

## Architecture Check

1. **FND-21-aligned**: bounded localized repair is the canonical "monitor assumptions and revise plans when assumptions break" shape from Intentions Are Revisable Commitments. Post-hoc classification approximates this; pre-failure repair lands the proper shape.
2. **FND-26 shared-domain service**: repair search calls `apply_effects_with_context` (worldwake-sim) for hypothetical effect evaluation — allowed shared-domain computation per the principle, not a privileged cross-system call.
3. **FND-20 bounded reasoning**: `repair_budget_fraction × max_node_expansions` caps the search; capped 5 `RepairKind` attempt classes; `RepairMemory.repairs[signature].expires_tick` TTL prevents repeat thrashing. No unbounded recursion.
4. **No back-compat shim**: post-hoc `classify_accepted_repair` is not deprecated — it is preserved as the fall-through classifier for the full-replan path. Both paths emit valid `RepairKind` variants under the migrated naming scheme.

## Verification Layers

1. Bounded repair budget invariant → focused unit test in `plan_repair.rs` asserting search terminates within `repair_budget_fraction × max_node_expansions` node expansions.
2. `RepairKind` attempt-order determinism → focused unit test asserting fixed `Ord`-derived attempt sequence.
3. `RepairMemory` consultation → focused unit test asserting recently-failed `(BreachSignature, RepairKind)` pairs are skipped.
4. `DiscrepancyClearing` audit (D1 subsumed) → if extension lands, focused unit tests for new variants; if no extension, document the audit conclusion in a code comment near the consumption site.
5. Pre-failure-vs-full-replan ordering at revalidation seam → ticket 007 holds the integration coverage; this ticket's verification stops at the module's public surface.

## What to Change

### 1. Create `crates/worldwake-ai/src/plan_repair.rs` module

Define:

```rust
pub struct PlanRepairContext<'a> {
    pub failed_step: u16,
    pub broken_link: CausalLink,
    pub preserved_prefix: &'a [PlannedStep],
    pub reusable_suffix: &'a [PlannedStep],
    pub new_evidence: &'a [BeliefRef],
    pub discrepancy_entry: &'a DiscrepancyEntry,
}

pub enum RepairOutcome {
    Repaired { kind: RepairKind, new_plan: PlannedPlan },
    Failed { tried: Vec<(RepairKind, RepairFailure)> },
}

pub enum RepairFailure {
    NoSiblingTargetFound,
    NoProviderReplacement,
    NoEpistemicSubstrate,   // S139 not landed; InsertVerification short-circuits
    BudgetExhausted,
    RecentlyFailed,
}

pub fn attempt_repair_then_replan(
    runtime: &mut AgentDecisionRuntime,
    cognitive: &CognitiveProfile,
    repair_memory: &RepairMemory,
    discrepancy_memory: &DiscrepancyMemory,
    // ... other ctx fields ...
) -> RepairOutcome { /* … */ }
```

The search attempts kinds in `Ord`-derived order: `RebindTarget`, `ReplaceProvider`, `InsertVerification`, `DowngradeToProgressBarrier`, `Abandon`. Each kind's handler returns `Result<PlannedPlan, RepairFailure>`. The first successful result is returned; failures accumulate in `tried`.

### 2. Update planner emitter to populate `PlanGuard.causal_links`

In the planner emitter (likely `crates/worldwake-ai/src/agent_tick/planning.rs` or a sibling planner module — implementer must grep for `PlanGuard {` literal-opening sites outside `#[cfg(test)]` to find the runtime emitter), populate `causal_links: Vec<CausalLink>` per step with the load-bearing precondition supporters. Cap the vec length at `cognitive.causal_links_per_step_cap`. If the cap is hit, emit a `DecisionTrace::CausalLinkCapHit` annotation (declared in ticket 008) so debuggers see the truncation.

### 3. DiscrepancyClearing audit (D1 subsumed)

During implementation of the repair-search clearing-condition dispatch, audit whether the existing 5 variants at `crates/worldwake-core/src/discrepancy.rs:57-70` (`TtlExpiry`, `ReobservationOf`, `BeliefUpdate`, `CommodityAvailabilityChanged`, `WorldStructureChange`) cover every recovery signal the repair search needs. Likely missing candidates: `OnRelationshipShift { other: EntityId }`, `OnPriceShift { commodity: CommodityKind, place: EntityId }`, `OnThreatLifted { source: EntityId }`, `OnNeedRecovered { need: HomeostaticNeedId }`. If extension is needed: extend the enum with the required variant(s) preserving the existing derives; if not, document the audit conclusion in a code comment near the dispatch site.

### 4. Reuse `pending_repair_context`

Determine during reassessment whether the existing `AgentDecisionRuntime.pending_repair_context` field can carry pre-failure breach data, or whether a parallel field is needed. If parallel: introduce `pre_failure_repair_context: Option<PreFailureRepairContext>`; if reuse: extend the existing struct.

### 5. Module exports

Add `pub mod plan_repair;` in `crates/worldwake-ai/src/lib.rs` with re-exports of `PlanRepairContext`, `RepairOutcome`, `RepairFailure`, `attempt_repair_then_replan`.

## Files to Touch

- `crates/worldwake-ai/src/plan_repair.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — module declaration + re-exports)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — planner emitter populates causal_links; subsumed DiscrepancyClearing audit)
- Likely: `crates/worldwake-ai/src/decision_runtime.rs` (modify — `AgentDecisionRuntime` field for pre-failure repair context, if parallel approach chosen)
- Likely: `crates/worldwake-core/src/discrepancy.rs` (modify — `DiscrepancyClearing` variant extension if audit concludes extension needed)
- Likely: planner emitter file (grep for `PlanGuard {` outside `#[cfg(test)]` to confirm the exact file/function)

## Out of Scope

- Revalidation routing change at `agent_tick/execution.rs` — ticket 007.
- Decision-trace `RepairAttemptTrace` surface — ticket 008.
- `S139::AskWitness` / `InspectContainer` integration for `InsertVerification` — S139 is a soft dep; `InsertVerification` short-circuits to `RepairFailure::NoEpistemicSubstrate` if S139 not landed.
- Cross-tick repair continuation — explicit Non-Goal per spec.
- HTN method substitution — `SubstituteMethodBranch` deferred to Phase 12 per spec.
- Removing `classify_accepted_repair` — explicit Non-Goal; the post-hoc classifier is preserved as fall-through.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai plan_repair` — new module's focused tests cover bounded budget, deterministic attempt order, `RepairMemory` consultation, and `DiscrepancyClearing` dispatch.
2. `cargo test -p worldwake-ai classify_accepted_repair` — existing post-hoc classification tests continue to pass (fall-through preservation).
3. Existing suite: `cargo test --workspace`.
4. `cargo clippy --workspace --all-targets -- -D warnings`.

### Invariants

1. Repair search node-expansion count ≤ `cognitive.repair_budget_fraction × cognitive.max_node_expansions / 1000`.
2. `RepairKind` attempt order is deterministic across runs given the same `BreachSignature` and `RepairMemory` state.
3. `RepairOutcome::Failed.tried` contains entries in attempt order; same set across deterministic runs.
4. `PlanGuard.causal_links.len() ≤ cognitive.causal_links_per_step_cap` at emit time.
5. Existing `pending_repair_context` post-hoc classification semantics unchanged after this ticket lands.
6. `classify_accepted_repair` continues to emit `RebindTarget` / `ReplaceProvider` for the previously-handled axes.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_repair.rs` `#[cfg(test)]` — new tests:
   - `repair_search_terminates_within_budget`
   - `repair_kind_attempt_order_is_deterministic`
   - `repair_memory_skips_recently_failed_kinds`
   - `insert_verification_returns_no_epistemic_substrate_without_s139`
   - `discrepancy_clearing_dispatch_covers_all_variants`
2. `crates/worldwake-ai/src/agent_tick/planning.rs` — existing classify tests unchanged; new test `planner_emitter_caps_causal_links_per_step` if planner emitter file is the same module.

### Commands

1. `cargo test -p worldwake-ai plan_repair`
2. `cargo test -p worldwake-ai classify_accepted_repair`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
