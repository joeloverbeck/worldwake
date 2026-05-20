# S149PARPLASEG-002: PartialPlanSegment carrier type

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new ai-crate carrier type for suspended partial plans
**Deps**: archive/tickets/S149PARPLASEG-001.md

## Problem

S149 makes partial plans first-class: when a plan reaches a typed barrier, the planner stores the prefix steps that succeeded, the barrier type and fact, and the resume/abandon conditions that would clear or invalidate it. D4 introduces the `PartialPlanSegment` carrier (plus `PartialPlanSegmentId`, `BarrierFact`, `PlannedSkeletonStep`). This ticket defines the type only; storage on `AgendaEntry` (003) and the resumption logic (005) consume it.

## Assumption Reassessment (2026-05-20)

1. The type must live in `worldwake-ai`, not `worldwake-core`: its fields reference ai-resident types `PlanTerminalKind` (planner_ops.rs:388, post-001), `PlannedStep` (planner_ops.rs:258), `PlannerOpKind` (planner_ops.rs:14), `GoalOffer` (goal_model.rs:2227), `BeliefPredicate` (htn/method_schema.rs:72), and `PayloadTemplate` (htn/method_schema.rs:151). `worldwake-core` cannot depend on `worldwake-ai` (Cargo.toml: serde/bincode/blake3 only). This was the central reassessment correction — the spec originally mis-placed it in core.
2. Resume/abandon conditions reuse the already-landed S148 core enums `IntentionResumeCondition` / `IntentionAbandonCondition` (`crates/worldwake-core/src/intention_condition.rs:7,24`). S149 introduces NO parallel condition types (FND-28). Variants confirmed: `IntentionResumeCondition::{BeliefStatusChanged{subject,target_status},OpportunityVisible,LocationReached,TickElapsed,ArtifactLegalEffectActive}`; `IntentionAbandonCondition::PatienceExhausted` present.
3. Shared boundary: the new `partial_plan` module is a leaf type definition with no consumers in this ticket; downstream tickets (003 storage, 005 resumption, 004 mapping) are the consumers. No existing symbol is renamed or removed.
4. The spec's fabricated types are dropped: `InformationGapTopic` (use `TellTopic`), `PreconditionPredicate` (use `BeliefPredicate`), `SafetyHazard`/`BarrierFact::HazardPresent` (deferred with `SafetyBarrier` per spec Non-Goals). `BarrierFact` therefore has five variants: `MissingBelief`, `ContestedReservation`, `DepletedResource`, `NoAuthorityForAction`, `BudgetExhausted`.

## Architecture Check

1. Hosting `PartialPlanSegment` in `worldwake-ai` keeps all its field types in or below its own crate (core types are reachable; ai types are local) — no core-side mirror needed because the type is live runtime state, not a core-consumed historical record.
2. No backward-compat shim: condition fields reuse the single existing core types; no S149-specific resume/abandon enums are introduced.

## Verified Layers

1. Type derives + serde roundtrip -> focused unit test (`partial_plan::tests::partial_plan_segment_roundtrips_through_bincode_with_all_barrier_facts`), since the type will be persisted via `AgendaEntry` (003). Single-layer ticket: it is a pure type definition, so a focused unit test on derive bounds and serialization is the applicable proof surface.
2. Deterministic ID identity -> focused unit test (`partial_plan::tests::partial_plan_segment_id_preserves_tick_and_counter_identity`).

## Landed Changes

### 1. New module `partial_plan.rs`

Added `crates/worldwake-ai/src/partial_plan.rs` defining `PartialPlanSegment`, `PartialPlanSegmentId` (typed `Tick` + counter identity), `PlannedSkeletonStep { op: PlannerOpKind, target_template: PayloadTemplate, expected_pre: Vec<BeliefPredicate> }`, and `BarrierFact` (five variants per Assumption 4). `PartialPlanSegment` carries `id, goal: GoalOffer, completed_prefix: Vec<PlannedStep>, remaining_skeleton: Option<Vec<PlannedSkeletonStep>>, terminal_barrier: PlanTerminalKind, barrier_fact: BarrierFact, resume_conditions: Vec<IntentionResumeCondition>, abandon_conditions: Vec<IntentionAbandonCondition>, created_tick: Tick, last_resume_attempt_tick: Option<Tick>, resume_attempt_count: u8, causal_links: Vec<EventId>`. The carrier derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` so ticket 003 can embed it on `AgendaEntry`.

### 2. Re-export

Added `pub mod partial_plan;` and public re-exports for `PartialPlanSegment`, `PartialPlanSegmentId`, `PlannedSkeletonStep`, and `BarrierFact` in `crates/worldwake-ai/src/lib.rs`.

### 3. HTN template serde support

Added serde derives to the existing HTN template types that are transitive fields of `PlannedSkeletonStep`: `BeliefPredicate`, `PayloadTemplate`, `PayloadValueTemplate`, `LocationTemplate`, `EntityTemplate`, `CommodityTemplate`, and `RecipeTemplate`.

## Landed Files

- `crates/worldwake-ai/src/partial_plan.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modified) — module + re-exports
- `crates/worldwake-ai/src/htn/method_schema.rs` (modified) — serde derives for template fields embedded in `PlannedSkeletonStep`

## Out of Scope

- Storage on `AgendaEntry` and save/load (ticket 003).
- Constructing segments at barrier sites and resumption (tickets 004, 005).

## Acceptance Result

### Tests That Passed

1. `PartialPlanSegment` roundtrips through bincode with all `BarrierFact` variants.
2. `PartialPlanSegmentId` preserves deterministic, monotonically distinct ordering from `(Tick, counter)`.
3. Existing suite passed with `cargo test -p worldwake-ai`.

### Invariants Verified

1. No field type of `PartialPlanSegment` resolves outside `worldwake-ai` or `worldwake-core` (crate-boundary safety).
2. `resume_conditions`/`abandon_conditions` use the core `IntentionResumeCondition`/`IntentionAbandonCondition` types; no S149-local condition enum was added.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/partial_plan.rs` (inline) — serde roundtrip + id determinism.

### Commands Run

1. `cargo fmt --all`
2. `cargo test -p worldwake-ai partial_plan`
3. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-05-20.

- Added the `worldwake-ai::partial_plan` carrier surface for `PartialPlanSegment`, `PartialPlanSegmentId`, `PlannedSkeletonStep`, and `BarrierFact`.
- Re-exported the carrier types from `worldwake-ai`.
- Added serde support to the existing HTN template types required for `PlannedSkeletonStep` serialization.
- Left storage on `AgendaEntry`, segment construction, failure mapping, and resumption logic to the existing downstream S149 tickets.

## Deviations

- `crates/worldwake-ai/src/htn/method_schema.rs` was added to the landed file set because the drafted bincode roundtrip proof requires `BeliefPredicate` and `PayloadTemplate` to serialize through `PlannedSkeletonStep`.
- `scripts/verify.sh` was not run for this individual carrier ticket; the harness keeps the full pre-PR verification gate for final S149 branch push. The ticket-level broad proof was `cargo test -p worldwake-ai`, which covers the affected crate surface.

## Verification Result

- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-ai partial_plan`
- Passed `cargo test -p worldwake-ai`
