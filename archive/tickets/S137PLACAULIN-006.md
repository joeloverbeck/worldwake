# S137PLACAULIN-006: plan_repair module - causal-link emission and bounded repair attempt surface

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - new `plan_repair` module, planner guard emitter populates capped causal links, S137 follow-up graph corrected
**Deps**: archive/tickets/S137PLACAULIN-001.md (CausalLink, BreachSignature), archive/tickets/S137PLACAULIN-002.md (repair_budget_fraction, causal_links_per_step_cap), archive/tickets/S137PLACAULIN-003.md (new RepairKind variants), archive/tickets/S137PLACAULIN-004.md (PlanGuard.causal_links), archive/tickets/S137PLACAULIN-005.md (RepairMemory shape)

## Problem

S137 D5 needed a concrete `worldwake-ai::plan_repair` public surface and runtime-created `PlanGuard.causal_links` so later revalidation routing can reason from breached causal links instead of opaque invalidator labels. Reassessment showed that parts of the draft ticket were already landed by earlier S137 work (`CausalLink`, `BreachSignature`, migrated `RepairKind`, `PlanGuard.causal_links`, `RepairMemory`, and post-hoc `classify_accepted_repair` variants), while successful localized repair strategy construction was still too broad to truthfully hide inside this staging ticket.

This ticket therefore lands the bounded attempt surface and causal-link emission needed by the next integration ticket, and opens `tickets/S137PLACAULIN-011.md` for actual successful `RebindTarget` / `ReplaceProvider` strategy construction before goldens can claim plan repair reduces replans.

## Assumption Reassessment (2026-05-13)

1. The live S137 substrate already includes `CausalLink`, `BreachSignature`, `RepairKind::{RebindTarget, ReplaceProvider, InsertVerification, DowngradeToProgressBarrier, Abandon}`, `PlanGuard.causal_links`, and `RepairMemory.repairs: BTreeMap<BreachSignature, RepairEntry>`. This ticket did not duplicate or migrate those shapes.
2. `crates/worldwake-ai/src/plan_repair.rs` did not exist before implementation. The landed module now defines `PlanRepairContext`, `RepairOutcome`, `RepairFailure`, `repair_budget`, deterministic `attempt_order`, and `attempt_repair_then_replan`.
3. Shared boundary under audit: planned-step guard construction. The runtime planner emitter path builds `PlanGuard` through `crates/worldwake-ai/src/plan_guard_build.rs`; `crates/worldwake-ai/src/search/transition.rs` now passes `CognitiveProfile` into successor construction so emitted guards can cap causal links by `cognitive.causal_links_per_step_cap`.
4. Planner-driven surface: this ticket is goal-family agnostic. It maps existing `RequiredFact` values (`TargetPresent`, `CommodityAvailable`, `RouteKnown`, `ResourceAccess`) into `PlanningFact` and `CausalProvider` entries for the planned step that consumes the fact.
5. Ordering claim correction: this ticket does not wire repair before full replan at the agent-tick revalidation seam. That remains ticket 007 after the public API and strategy handlers exist.
6. DiscrepancyClearing audit: the five variants are exhaustively consumed by `discrepancy_clearing_is_repair_search_visible`; no core enum extension was needed for this staged repair-search surface.
7. Adjacent contradiction found during implementation: successful localized repair handlers are not present. Returning `RepairOutcome::Repaired` is shape-tested, but live `attempt_repair_then_replan` records deterministic failures only. Classified as a required follow-up, not silently deferred; see `tickets/S137PLACAULIN-011.md`.

## Architecture Check

1. `PlanGuard.causal_links` now has one canonical runtime construction path in `build_plan_guard_with_causal_links`, while the older `build_plan_guard` helper remains as a zero-cap wrapper for existing callers and tests.
2. Bounded repair search uses profile data (`repair_budget_fraction`, `max_node_expansions`) and `RepairMemory` instead of hardcoded retry loops.
3. No backward-compatibility alias was added. The public follow-up API is the current `PlanRepairContext` + `attempt_repair_then_replan(&PlanRepairContext, &CognitiveProfile, &RepairMemory)` surface.
4. No cross-system direct call was introduced. The successful hypothetical strategy work that may need effect-schema evaluation is owned by `S137PLACAULIN-011`.

## Verification Layers

1. Repair budget arithmetic -> focused unit test in `plan_repair.rs` proving attempt count stops at `repair_budget_fraction * max_node_expansions / 1000`.
2. Deterministic strategy ordering -> focused unit test over the five `RepairKind` variants.
3. `RepairMemory` consultation -> focused unit test proving a recently failed `(BreachSignature, RepairKind)` is skipped with `RepairFailure::RecentlyFailed`.
4. `InsertVerification` without S139 -> focused unit test proving `RepairFailure::NoEpistemicSubstrate`.
5. `DiscrepancyClearing` audit -> focused unit test exhaustively constructing all current variants.
6. Runtime guard causal-link cap -> focused unit test in `plan_guard_build.rs` asserting `PlanGuard.causal_links.len() <= cognitive.causal_links_per_step_cap`.

## What Changed

### 1. Added `crates/worldwake-ai/src/plan_repair.rs`

The module now exposes:

```rust
pub struct PlanRepairContext<'a> {
    pub failed_step: u16,
    pub broken_link: CausalLink,
    pub breach_signature: BreachSignature,
    pub preserved_prefix: &'a [PlannedStep],
    pub reusable_suffix: &'a [PlannedStep],
    pub new_evidence: &'a [BeliefRef],
    pub discrepancy_entry: &'a DiscrepancyEntry,
}

pub enum RepairOutcome {
    Repaired { kind: RepairKind, new_plan: Box<PlannedPlan> },
    Failed { tried: Vec<(RepairKind, RepairFailure)> },
}
```

`RepairOutcome` and `RepairFailure` derive serde traits so ticket 008 can embed repair failures in trace payloads.

### 2. Added causal-link construction to planner guard emission

`build_plan_guard_with_causal_links` maps resolved `RequiredFact` entries to `CausalLink` entries and caps the vector at `CognitiveProfile.causal_links_per_step_cap`. Search successor construction now passes the active cognitive profile into guard construction.

### 3. Preserved existing full-replan classification

The post-hoc `classify_accepted_repair` path remains unchanged and still emits the migrated `RepairKind` variants for full-replan fall-through.

### 4. Added follow-up for successful localized strategies

`tickets/S137PLACAULIN-011.md` now owns successful `RebindTarget` / `ReplaceProvider` / `DowngradeToProgressBarrier` / `Abandon` strategy construction. Tickets 007 and 010 now depend on that work before claiming live routing success or Phase 11 replan reduction.

## Files Touched

- `crates/worldwake-ai/src/plan_repair.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (module declaration and re-exports)
- `crates/worldwake-ai/src/plan_guard_build.rs` (causal-link construction and tests)
- `crates/worldwake-ai/src/search/transition.rs` (passes `CognitiveProfile` into successor guard construction)
- `crates/worldwake-ai/src/search/mod.rs` (passes `CognitiveProfile` through search)
- `crates/worldwake-ai/src/search/tests.rs` (updated direct successor-builder calls)
- `specs/S137-plan-causal-links-and-repair.md` (D5 truth-sync)
- `archive/tickets/S137PLACAULIN-007.md` (dependency/API truth-sync)
- `archive/tickets/S137PLACAULIN-010.md` (dependency truth-sync)
- `tickets/S137PLACAULIN-011.md` (new follow-up)

## Out of Scope

- Revalidation routing at the invalidator seam - ticket 007.
- Successful localized repair strategy construction - ticket 011.
- Decision trace `RepairAttemptTrace` emission - ticket 008.
- Observer rendering - ticket 009.
- Golden plan-repair scenarios and Phase 11 gate - ticket 010.
- S139 `AskWitness` / `InspectContainer` integration for `InsertVerification`.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai plan_repair`
2. `cargo test -p worldwake-ai plan_guard_build`
3. `cargo test -p worldwake-ai classify_accepted_repair`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Repair attempt count is bounded by `cognitive.repair_budget_fraction * cognitive.max_node_expansions / 1000`.
2. `RepairOutcome::Failed.tried` preserves deterministic attempt order.
3. Recently failed repair kinds are skipped through `RepairMemory`.
4. `PlanGuard.causal_links.len() <= cognitive.causal_links_per_step_cap` at emit time.
5. Existing post-hoc `classify_accepted_repair` behavior remains unchanged.

## Outcome

The ticket landed the S137 D5 repair-attempt surface and the runtime causal-link emitter path. `PlanRepairContext` now carries the breach signature needed by `RepairMemory`, `RepairOutcome::Repaired` carries a boxed `PlannedPlan`, and `RepairFailure` is serializable for the later trace payload work. The module remains intentionally staged for strategy success: it records deterministic bounded failure attempts until `tickets/S137PLACAULIN-011.md` implements successful localized replacement handlers.

The planner search path now passes `CognitiveProfile` into guard construction and emits capped `PlanGuard.causal_links` entries from resolved required facts. The S137 spec and dependent tickets were updated so ticket 007 and the Phase 11 goldens depend on the new strategy-handler ticket before claiming live repair success.

## Verification Result

1. Passed `cargo test -p worldwake-ai plan_repair` - 7 focused repair-module tests passed.
2. Passed `cargo test -p worldwake-ai plan_guard_build` - 5 guard-construction tests passed, including capped causal-link emission.
3. Passed `cargo test -p worldwake-ai classify_accepted_repair` - 3 post-hoc repair-classifier tests passed.
4. Passed `cargo test -p worldwake-ai` - worldwake-ai package tests passed after the repair module and causal-link emitter changes.
5. Passed `cargo test --workspace` - workspace test suite and doctests passed.
6. Passed `cargo clippy --workspace --all-targets -- -D warnings` - CI-matching clippy gate passed.

## Verified Test Changes

### Tests Changed

1. `crates/worldwake-ai/src/plan_repair.rs` - budget, deterministic ordering, repair-memory skip, `InsertVerification` S139 short-circuit, `DiscrepancyClearing` dispatch, repair-outcome shape, and `RepairFailure` serialization.
2. `crates/worldwake-ai/src/plan_guard_build.rs` - capped causal-link emission from guard construction.
3. `crates/worldwake-ai/src/search/tests.rs` - updated direct search-transition callers to pass `CognitiveProfile`.

### Commands

1. `cargo test -p worldwake-ai plan_repair`
2. `cargo test -p worldwake-ai plan_guard_build`
3. `cargo test -p worldwake-ai classify_accepted_repair`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace --all-targets -- -D warnings`
