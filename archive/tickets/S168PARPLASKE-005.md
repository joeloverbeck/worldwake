# S168PARPLASKE-005: Planner skeleton source carrier

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/search/mod.rs` (expose a lawful preservable skeleton source from the search result/trace metadata); `crates/worldwake-ai/src/agent_tick/planning.rs` (carry the source beside `CandidatePlanSearch` for later segment population); focused tests.
**Deps**: `archive/specs/S168-partial-plan-skeleton-reuse.md` (D1.a/D1.b causal-equivalence requirement); `archive/tickets/S168PARPLASKE-001.md` (revalidation contract that later consumes the skeleton); `archive/specs/S149-partial-plan-segments-and-typed-terminals.md` (owns `PartialPlanSegment` and `PlannedSkeletonStep`).

## Problem

The now-archived `archive/tickets/S168PARPLASKE-002.md` was drafted as if the planner already exposed a remaining high-level skeleton at the suspension boundary. Live reassessment on 2026-05-24 disproved that premise:

1. `PlanSearchResult::BudgetExhausted` currently carries only `expansions_used`; it does not carry the remaining strategic/search shape.
2. `write_budget_exhausted_partial_plan_segments` can therefore only construct `remaining_skeleton: None` unless a new planner-owned carrier is added first.
3. `spawn_information_barrier_companions` in `agenda_manager.rs` consumes existing suspended `PartialPlanSegment`s; it is not the producer of those segments.

Under `docs/FOUNDATIONS.md` FND-12 and FND-27, `remaining_skeleton` may be populated only from a lawful derived cache whose higher-fidelity referent is the planner/search work the agent already performed. This ticket adds that source carrier before ticket 002 threads it into `PartialPlanSegment`.

## Assumption Reassessment (2026-05-24)

1. **Live code checked**. `crates/worldwake-ai/src/search/mod.rs::PlanSearchResult::BudgetExhausted` has shape `{ expansions_used: u16 }`. `crates/worldwake-ai/src/agent_tick/planning.rs::CandidatePlanSearch` stores `result`, trace metadata, binding rejections, and expansion summaries, but no skeleton source. `write_budget_exhausted_partial_plan_segments` calls `budget_exhausted_partial_plan_segment` with no skeleton input.
2. **Planner contract checked**. `docs/planner-contracts.md` says planner-visible inputs and snapshots must stay belief-backed or lawful boundary artifacts. This ticket does not add new planner-visible facts; it preserves an already-derived planning shape for later reuse.
3. **FOUNDATIONS boundary**. The skeleton source is a derived planning cache, not truth. It must be deleteable without changing lawful behavior: ticket 003 must still fall back to ordinary unseeded search when no skeleton exists or revalidation fails.
4. **Shared data contract under audit**. The owned boundary is a new AI-internal skeleton-source carrier that can later populate `PartialPlanSegmentSeed.remaining_skeleton`. The carrier may describe only high-level `PlannerOpKind`, `PayloadTemplate`, and `BeliefPredicate` expectations already representable by `PlannedSkeletonStep`.
5. **Exclusions**. Combat ops and target-identity-bound payload templates must remain absent from the preservable source. Ticket 002 will enforce the construction filter, but this ticket must not create a source that requires forbidden preservation to be useful.
6. **Adjacent ticket impact**. `S168PARPLASKE-002` must depend on this ticket. `S168PARPLASKE-003` must continue to depend on 002, because it consumes populated `PartialPlanSegment.remaining_skeleton`, not this intermediate carrier directly.

## Architecture Check

1. **Carrier before population** is cleaner than synthesizing a skeleton in `write_budget_exhausted_partial_plan_segments`, because the planner/search layer owns the high-level shape and can record it while the referent still exists.
2. **No world-truth reads**. The carrier is built from planner-local strategic/method/search structures, not from authoritative world state.
3. **No replay surface**. The carrier stores templates only. It does not store `ActionDefId`, resolved targets, payloads, grants, or dispatch-ready steps.
4. **No backward compatibility shim**. The current fallback of `remaining_skeleton: None` remains lawful when no preservable source exists.

## Verified Layers

1. **Budget-exhaustion source presence** -> focused unit test in `search` or `agent_tick::planning`: a budget-exhausted plan with a noncombat, non-fixed-template strategic/method remainder exposes a non-empty preservable skeleton source.
2. **No source when no meaningful remainder exists** -> focused unit test: cold budget exhaustion or empty strategic/method shape yields `None`.
3. **Exclusion preservation** -> focused unit test: combat or fixed-target/template-bound steps are not represented in the source.
4. **Caller carriage** -> focused unit test on `CandidatePlanSearch`/planning pass showing the skeleton source is retained beside the budget-exhausted result for ticket 002 to consume.
5. **No save-format change** -> compile and focused tests only; no serialized authoritative type changes in this ticket.

## Landed Changes

### 1. Defined the planner skeleton source carrier

Added an AI-internal carrier in `crates/worldwake-ai/src/search/mod.rs`:

```rust
pub(crate) struct PartialPlanSkeletonSource {
    pub(crate) remaining_skeleton: Vec<PlannedSkeletonStep>,
}
```

`SearchTraceMetadata` now carries `skeleton_source: Option<PartialPlanSkeletonSource>`, and `CandidatePlanSearch` retains the same source at the planning-pass boundary for ticket 002.

### 2. Derived the source while planner/search context is live

`crates/worldwake-ai/src/search/strategic.rs` now derives `PlannedSkeletonStep` values from the selected `MethodSchema` while the strategic planner still has access to the selected method's subgoal templates and belief preconditions.

The derivation:

- preserves stable order;
- includes only `PlannerOpKind`, `PayloadTemplate`, and belief-precondition data already present in the selected method;
- returns `None` when the source would be empty;
- excludes `Attack`/`Defend` and fixed-entity payload templates.

### 3. Carried the source to the segment writer boundary

`CandidatePlanSearch` now stores `skeleton_source: Option<PartialPlanSkeletonSource>`, copied from `SearchTraceMetadata` as candidate planning results are assembled. `write_budget_exhausted_partial_plan_segments` reads the source field so the carrier is live and available for ticket 002 without rerunning planning.

Ticket 002 will thread this source into `budget_exhausted_partial_plan_segment`; this ticket stops at exposing and testing the source carrier.

## Landed Files

- `crates/worldwake-ai/src/search/mod.rs` (modified — `PartialPlanSkeletonSource`, metadata field)
- `crates/worldwake-ai/src/search/strategic.rs` (modified — selected-method skeleton derivation and focused tests)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modified — `CandidatePlanSearch` source carriage and focused test)
- `archive/tickets/S168PARPLASKE-002.md` (then active; modified — dependency and reassessment truth-sync)
- `.codex/run-state/implement-spec-tickets.json` (modified — queue retargeting)

## Out of Scope

- Populating `PartialPlanSegment.remaining_skeleton` — ticket 002.
- `filter_preservable_skeleton` at segment construction — ticket 002.
- `search_plan_seeded` and resume consumption — ticket 003.
- Validation goldens and save/load enclosing-state proof — ticket 004.
- Concrete committed-step replay or action dispatch from skeletons.

## Acceptance Result

### Tests Passed

1. Passed focused skeleton-source derivation tests via `cargo test -p worldwake-ai --lib skeleton_source_for_method`.
2. Passed focused planning-carriage test via `cargo test -p worldwake-ai --lib candidate_plan_search_retains_partial_plan_skeleton_source`.
3. Passed `cargo test -p worldwake-ai --lib search`.
4. Passed `cargo test -p worldwake-ai`.
5. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.

### Invariants

1. A populated source is derived only from selected method/search work already performed.
2. The source can be absent without changing lawful behavior; absence still leaves ticket 002 free to preserve `remaining_skeleton: None`.
3. The source stores no dispatch-ready action details: no `ActionDefId`, resolved target list, concrete action payload, grant, or committed step.
4. Combat and fixed-entity target templates are excluded from the source.

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/src/search/strategic.rs::skeleton_source_for_method_preserves_template_only_action_steps`.
2. `crates/worldwake-ai/src/search/strategic.rs::skeleton_source_for_method_excludes_combat_and_fixed_target_steps`.
3. `crates/worldwake-ai/src/agent_tick/planning.rs::candidate_plan_search_retains_partial_plan_skeleton_source`.

### Commands Run

1. Passed `cargo test -p worldwake-ai --lib skeleton_source_for_method`.
2. Passed `cargo test -p worldwake-ai --lib candidate_plan_search_retains_partial_plan_skeleton_source`.
3. Passed `cargo test -p worldwake-ai --lib search`.
4. Passed `cargo test -p worldwake-ai`.
5. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.

## Outcome

Completed on 2026-05-24.

- Added a planner-owned `PartialPlanSkeletonSource` carrier and attached it to `SearchTraceMetadata`.
- Derived preservable skeleton steps from selected method action subgoals while the planner method referent is still live.
- Carried the source through `CandidatePlanSearch` so ticket 002 can populate `PartialPlanSegment.remaining_skeleton` without synthesizing a skeleton after the fact.
- Updated S168PARPLASKE-002 (now `archive/tickets/S168PARPLASKE-002.md`) to depend on this prerequisite and to record the FND-12/FND-27 reassessment correction.

## Deviations

- The landed source derives from selected method `PerformAction` subgoals rather than every possible strategic stage. That is the strongest honest first carrier because method action subgoals already preserve `PlannerOpKind`, `PayloadTemplate`, and belief preconditions. Generic fallback strategic stages do not carry enough action-template data to populate `PlannedSkeletonStep` without inference.
- Fixed-entity payload templates are excluded here. Ticket 002 may still apply an additional construction filter, but this ticket avoids making fixed target identity necessary for the source to be useful.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib skeleton_source_for_method`.
- Passed `cargo test -p worldwake-ai --lib candidate_plan_search_retains_partial_plan_skeleton_source`.
- Passed `cargo test -p worldwake-ai --lib search`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
