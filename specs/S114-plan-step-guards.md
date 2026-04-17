# S114: Plan Step Guards and Expectation Monitoring

## Summary

Annotate each `PlannedStep` with explicit *guards* (required-believed-facts, minimum-confidence thresholds, invalidators) and *expectations* (expected observations the step should produce). Revalidation and in-flight monitoring read guards to classify drift as irrelevant, repairable, plan-invalidating, or goal-changing — not just "affordance still matches." Expectation mismatches emit `EventTag::ExpectationMismatch` (S110), feed `DiscrepancyMemory` (S109), and become the primary signal for future PolicyPlan branching. Scoped to four core guard/expectation kinds (immediate / state / informed / regression); danger-spike, counterparty-unwilling, resource-partial, partial-execution-drift kinds are deferred.

## Phase and Status

Phase 9: Belief-First Continual Planning Structural. Status: Draft.

## Crates

- `worldwake-ai` — `PlannedStep` extension with `guard: Option<PlanGuard>` and `expectations: Vec<PlanExpectation>`; revalidation upgrade in `plan_revalidation.rs`; guard/expectation construction in planner
- `worldwake-core` — `PlanGuard`, `PlanExpectation`, `ExpectationKind` types shared between planning and sim
- `worldwake-sim` — step-execution observers emit `ExpectationMismatch` when guards fail mid-step

## Dependencies

- S109 (Typed Discrepancy Taxonomy) — expectation mismatches record typed discrepancies.
- S110 (Decision History Events) — `ExpectationMismatch` event variant exists.
- S113 (Belief Envelope) — guards reference `BeliefValue::confidence`.

## Design Goals

- Revalidation classifies drift precisely: a merchant's restock is irrelevant to a `TravelTo(destination)` step; it invalidates a `Purchase(merchant)` step only if a guard referenced that merchant's stock.
- Expectation monitoring catches the "agent arrived, target gone" failure *before* the action starts, not at handler time. The `BeliefStale` belief → `ExpectationMismatch` → `Discrepancy::BeliefStale` pipeline replaces ad-hoc `BlockingFact::AssumptionFailed` traps.
- Scoped guard kinds. Four core kinds cover ~80% of current failure pathways; the remaining kinds land when Phase 10 scenarios surface them.

## Non-Goals

- Full PolicyPlan branching. Guards and expectations are the substrate branches will read, but this spec keeps plans linear.
- Automatic repair on guard failure. S114 records the mismatch and returns `PlanInvalidated`; the existing `handle_plan_failure` path decides whether to replan.
- Step-level contract generation by the planner from first principles. Guards and expectations are authored per-action-kind (in each action's registration) plus narrow planner-side augmentations.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-16 (Uncertainty / Contradiction First-Class) | Guards read belief confidence. A step that requires `confidence ≥ 700` fails fast when the underlying belief decayed. |
| FND-17 (Surprise Comes From Violated Expectation) | Expectations encode what the agent *expected* to observe. Mismatch is the authoritative surprise signal, not a noise-level log. |
| FND-20 (Resource-Bounded Practical Reasoning) | Revalidation short-circuits on guard failure before running affordance matching. Irrelevant drift is ignored cheaply. |
| FND-29A (Causal History) | Expectation mismatches are recorded events (S110), preserving the "what did the agent expect that didn't happen?" audit trail. |

## Deliverables

### D1: `PlanGuard` and `PlanExpectation` types

```rust
pub struct PlanGuard {
    pub required_facts: Vec<RequiredFact>,
    pub min_confidence: Permille,
    pub invalidators: Vec<Invalidator>,
}

pub enum RequiredFact {
    TargetPresent { target: EntityId, at_place: EntityId },
    CommodityAvailable { place: EntityId, kind: CommodityKind, min_quantity: Quantity },
    RouteKnown { from: EntityId, to: EntityId },
    ResourceAccess { resource: EntityId, agent_holds_permission: bool },
}

pub enum Invalidator {
    /// Belief about `target` drops below min_confidence or is contradicted.
    BeliefStatusChange { claim: BeliefClaimKey },
    /// Target entity moved away from `at_place`.
    TargetMoved { target: EntityId },
    /// Commodity stock at `place` drops below `min_quantity`.
    CommodityDepleted { place: EntityId, kind: CommodityKind },
    /// Blocker memory records a new suppressive entry for this goal/place/target.
    NewBlockerRecorded,
}

pub struct PlanExpectation {
    pub kind: ExpectationKind,
    pub observe_by: Option<Tick>, // absolute tick; None = by step completion
}

pub enum ExpectationKind {
    /// Immediate: the step's own completion handler emits an expected event.
    Immediate { event_tag: EventTag },
    /// State: the step leaves the world in a specified state (quantity
    /// increased, entity transferred, claim established).
    State { predicate: StatePredicate },
    /// Informed: the agent perceives an expected evidence artifact or
    /// observation after the step.
    Informed { observation: ObservationPredicate },
    /// Regression: a prior-step side effect should still hold (e.g., the
    /// tool we picked up earlier is still in inventory).
    Regression { predicate: StatePredicate },
}
```

### D2: `PlannedStep` extension

Extend the existing `PlannedStep` in `crates/worldwake-ai/src/planner_ops.rs` with `guard: Option<PlanGuard>` and `expectations: Vec<PlanExpectation>`. `expected_materializations` (the current field) stays as the concrete post-action materialization tag set but is now a subset of `expectations` expressed as `ExpectationKind::State`.

### D3: Guard authoring

Each `ActionDef` registration site declares its default guard and expectation template:

```rust
actions.register(ActionDef {
    // ... existing fields ...
    binding_strictness: BindingStrictness::ExactIdentity,
    guard_template: |step| PlanGuard {
        required_facts: vec![
            RequiredFact::TargetPresent {
                target: step.primary_target(),
                at_place: step.target_place(),
            },
        ],
        min_confidence: Permille::new(500),
        invalidators: vec![
            Invalidator::TargetMoved { target: step.primary_target() },
            Invalidator::BeliefStatusChange { claim: step.target_claim() },
        ],
    },
    expectation_template: |step| vec![
        PlanExpectation {
            kind: ExpectationKind::Immediate {
                event_tag: EventTag::ActionCommitted,
            },
            observe_by: Some(step.expected_complete_tick()),
        },
    ],
});
```

The planner instantiates the template per step from the ranked goal's evidence and beliefs. Authoring is per-action-kind, not per-step; the template computes concrete fields from the step's bound targets.

### D4: Revalidation upgrade

`plan_revalidation.rs::revalidate_next_step` gains a guard-check pass before affordance matching:

```rust
pub fn revalidate_next_step(...) -> RevalidationOutcome {
    let step = &plan.steps[next_idx];

    // 1. Guard check
    if let Some(guard) = &step.guard {
        if let Some(breach) = check_guard(guard, agent_beliefs, blocker_memory, current_tick) {
            return RevalidationOutcome::Invalidated {
                reason: PlanInvalidationReason::GuardBreach(breach),
            };
        }
    }

    // 2. Existing affordance match (S108 strictness-aware)
    match requested_affordance_matches(...) { ... }
}
```

`GuardBreach` carries the specific `Invalidator` that tripped. `PlanInvalidatedPayload` (S110) records the breach for the event log.

### D5: In-flight expectation monitoring

A new `expectation_monitor_system` runs at tick-end after action commitments have been recorded:

```rust
pub fn expectation_monitor_system(world: &mut World, event_log: &mut EventLog, tick: Tick) {
    for agent in agents_with_active_plans(world) {
        let plan = world.get::<ActivePlan>(agent);
        let committed_step = plan.current_step();
        for expectation in &committed_step.expectations {
            if let Some(deadline) = expectation.observe_by {
                if tick > deadline {
                    let observed = check_expectation(world, agent, expectation);
                    if !observed {
                        emit_expectation_mismatch(event_log, agent, plan.id, expectation);
                        record_discrepancy(world, agent, expectation);
                    }
                }
            }
        }
    }
}
```

This is the first SystemFn added by the Phase 8–9 planner sequence. Placed late in the tick order (after action-commit and belief-update).

### D6: Discrepancy recording on mismatch

When an expectation times out without being met:

- `ExpectationKind::Immediate` mismatch → `Discrepancy::PartialExecutionDrift`
- `ExpectationKind::State` mismatch → `Discrepancy::BeliefContradicted` (the agent believed the step would produce state X; world says otherwise)
- `ExpectationKind::Informed` mismatch → `Discrepancy::MissingObservation`
- `ExpectationKind::Regression` mismatch → `Discrepancy::BeliefContradicted`

Each records into `DiscrepancyMemory` with class-specific TTL (S109) and emits `EventTag::ExpectationMismatch` + `EventTag::BlockerRecorded` (S110).

## FND-01 Section H: Causal Hooks

1. **Information-path analysis**: Guards read the agent's belief store (local). Expectations read event-log tags and observations. No cross-agent information flow introduced.
2. **Positive-feedback analysis**: A guard that always breaches → plan always invalidates → replan → same guard breaches. Dampener: the repeat is recorded as `Discrepancy` with TTL; the goal is suppressed until TTL expires. Loops cannot run faster than `min(TTL)` for the relevant discrepancy class.
3. **Concrete dampeners**: Discrepancy TTL per class (S109). Additionally, guard templates bound `min_confidence` to a finite floor (per-action-kind), so a guard cannot require impossible certainty.
4. **Stored state vs. derived read-model**: Guards are authored templates (static metadata on `ActionDef`). `PlanGuard` instances attached to `PlannedStep` are derived per-plan. Expectation monitoring produces event-log entries (authoritative) and memory updates (authoritative).

## SystemFn Integration

**New SystemFn**: `expectation_monitor_system`. Placement: tick-phase order after `commit_actions` and `perception`, before `belief_decay`. Runs once per tick per active agent with a live plan.

## Component Registration

- `ActivePlan` (existing or runtime-generated) gains a `step_expectation_state` field tracking per-step expectation observation status. Runtime-generated; exempt from scenario authoring per spec-drafting-rules.md §5.

## Cross-System Interactions

- **Planner ↔ action registry**: Planner reads each `ActionDef`'s guard/expectation templates at plan-build time.
- **Revalidation ↔ memory**: Revalidation reads `DiscrepancyMemory` and `BlockerMemory` to evaluate invalidators.
- **Expectation monitor ↔ event log**: Monitor reads action-commit events and emits mismatch events.
- **Expectation monitor ↔ discrepancy memory**: Monitor writes typed discrepancies on mismatch.

## Profile-Driven Parameters

| Parameter | Profile | Type | Default | Purpose |
|-----------|---------|------|---------|---------|
| `expectation_tolerance_ticks` | `CognitiveProfile` | `u32` | 2 | Slack added to `observe_by` deadlines per agent |
| `guard_min_confidence_override` | `CognitiveProfile` | `Permille` | `Permille::new(0)` | Per-agent floor that guard `min_confidence` cannot exceed (a less careful agent can act on weaker beliefs) |

## Validation and Falsification

### Unit tests

1. Guard with `TargetPresent` invalidator fires when belief-store shows target moved.
2. Guard with `min_confidence: 700` fails when `BeliefValue::confidence` is 500.
3. Irrelevant drift (unrelated merchant restock) does not trigger any guard invalidator.
4. Expectation `ExpectationKind::Immediate` with `observe_by = tick+5` fires mismatch at tick+6 if no `ActionCommitted` event landed.

### Integration tests

5. Existing target-gone golden: with guards, the `BeliefContradicted` replan path is taken, not the `AssumptionFailed` fallback.
6. Survival scenarios pass: no increase in false-positive guard breaches on trivial paths (eat, sleep, wash).

### Golden test

7. New scenario: agent plans to purchase from merchant A; merchant A departs before arrival. Guard breach fires on arrival tick; `ExpectationMismatch` event appears in event log; `DiscrepancyMemory` records `BeliefContradicted`; agent replans within 2 ticks.

## Outcome

To be filled in at completion.
