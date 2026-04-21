# S114PLASTGUA-007: Revalidation upgrade — classify_revalidation with guard-check pass

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `RevalidationOutcome` enum; new `classify_revalidation` fn; refactored `revalidate_next_step`; wired into `BestEffort` action start.
**Deps**: `archive/tickets/S114PLASTGUA-002.md`, `archive/tickets/S114PLASTGUA-003.md`

## Problem

S114 D5 introduces a guard-check pass before affordance matching so revalidation can classify drift precisely: a merchant's restock is irrelevant to a `TravelTo(destination)` step; it invalidates a `Purchase(merchant)` step only if a guard referenced that merchant's stock. `revalidate_next_step` today returns `bool` — insufficient to convey the specific invalidator that fired. A companion `classify_revalidation` returning `RevalidationOutcome` surfaces the reason to the `BestEffort` action-start caller without plumbing an out-parameter through every existing call site.

## Assumption Reassessment (2026-04-21)

1. `revalidate_next_step` lives at `crates/worldwake-ai/src/plan_revalidation.rs:14` and currently returns `bool`. Its 19 inline `#[test]` functions at lines 910-1472 exercise affordance-match + payload-override scenarios; none today assert the presence/absence of `step.guard` because the field itself lands in ticket 003. After the guard-check pass is added, every existing test continues to pass unchanged because default `step.guard = None` short-circuits the new pass.
2. S114 spec D5 at `specs/S114-plan-step-guards.md:268-321` defines `RevalidationOutcome { Valid, Invalidated { reason: PlanInvalidationReason } }`, the `classify_revalidation` signature, and the check_guard helper's required semantics. `PlanInvalidationReason::ExpectationMismatch { step_index }` already exists at `crates/worldwake-core/src/decision_event_payload.rs:179` — the guard-breach path reuses it, not a new `GuardBreach` variant.
3. Shared boundary under audit: the `plan_revalidation.rs` module's public API — `revalidate_next_step` continues to be `bool`-returning for drop-in compatibility with existing callers (per spec: "Call sites of the existing `revalidate_next_step` continue to compile unchanged"). `classify_revalidation` is the new seam.
4. S113 envelope accessors are the belief-side read channel: `believed_target_location`, `believed_commodity_stock`, `believed_entities_at` — exposed on the envelope types per `archive/specs/S113-belief-envelope.md`. S109's `BlockerMemory` is consulted for `NewBlockerRecorded { baseline_tick }` invalidators. Guards read only the agent's own belief store (FND-14) and own blocker memory (FND-26). No cross-agent data flow introduced.
5. Authoritative-to-AI Impact Rule walkthrough (CLAUDE.md): D5 gates step execution. The `BestEffort` action-start path must invoke `classify_revalidation` (not `revalidate_next_step`) so that a guard breach routes through `handle_plan_failure` with the specific `PlanInvalidationReason::ExpectationMismatch`. The start path is currently at `crates/worldwake-sim/src/tick_step.rs` / action-start machinery; verify the exact call site at implementation time.
6. `guard_min_confidence_ceiling` from ticket 002 caps the effective `min_confidence` at guard evaluation time: `effective = min(guard.min_confidence, profile.guard_min_confidence_ceiling)`. A lower ceiling on the agent's profile lets less-careful agents act on weaker beliefs.
7. `RouteKnown` required fact should evaluate through the existing `RuntimeBeliefView::route_exists(from, to)` seam. This is a public-topology read already used elsewhere for `Discrepancy::RouteUnknown`; it does not require a new S113-style belief envelope.
8. Heuristic-removal discipline: this ticket does not remove or weaken any existing heuristic. It adds a new guard-check pass *before* the existing affordance match — pre-S114 revalidation behavior is preserved when `step.guard = None`, which is true for every pre-ticket-006 action.

## Architecture Check

1. Additive seam: `classify_revalidation` is new; `revalidate_next_step` is refactored to delegate. No call-site plumbing required for existing consumers. The `BestEffort` start path migrates to `classify_revalidation` specifically (it needs the reason), not blanket.
2. `check_guard` returns `Option<InvalidatorTag>` — `None` means guard satisfied, `Some(tag)` carries the specific breach reason for the event-log payload (consumed by ticket 009 when it eventually populates `mismatch_detail` on `ExpectationMismatchPayload`). This ticket does not itself emit events; it returns the classification.
3. Per `docs/precision-rules.md` Rule 6 (Decision-Trace Preference): guard-breach classification lives in the agent's decision path. `RevalidationOutcome::Invalidated { reason }` is captured by `handle_plan_failure` which already writes `PlanInvalidatedPayload` to the event log per S110.

## Verification Layers

1. Guard-check classification (`TargetPresent` required fact + `TargetMoved` invalidator fires when belief envelope returns different `at_place`) → focused unit test in `plan_revalidation.rs` tests module.
2. Confidence-floor classification (`min_confidence: 700` vs `BeliefValue::confidence: 500` fails) → focused unit test.
3. Irrelevant-drift isolation (unrelated merchant restock does not trigger any invalidator) → focused unit test.
4. Drop-in compatibility (`revalidate_next_step` returns `bool` identical to pre-S114 behavior when `step.guard = None`) → existing 19 tests at `plan_revalidation.rs:910-1472` stay green.
5. Action-start integration (`BestEffort` start path routes guard breach through `handle_plan_failure`) → action trace shows the `PlanInvalidationReason::ExpectationMismatch { step_index }` arm is taken; focused runtime coverage in `agent_tick` tests (add one test if none already exists for guard-breach start).
6. Profile-ceiling contract (`guard_min_confidence_ceiling` caps effective confidence) → focused unit test constructs a profile with ceiling < guard.min_confidence and asserts the guard passes (lower ceiling = looser effective requirement).

## What to Change

### 1. Introduce `RevalidationOutcome`

In `crates/worldwake-ai/src/plan_revalidation.rs`, add:

```rust
pub enum RevalidationOutcome {
    Valid,
    Invalidated { reason: PlanInvalidationReason },
}

impl RevalidationOutcome {
    pub fn is_valid(&self) -> bool {
        matches!(self, RevalidationOutcome::Valid)
    }
}
```

### 2. Add `classify_revalidation`

Signature matches existing `revalidate_next_step` plus returns `RevalidationOutcome`. Pseudocode:

```rust
pub fn classify_revalidation(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    step: &PlannedStep,
    bindings: &MaterializationBindings,
    registry: &ActionDefRegistry,
    handlers: &ActionHandlerRegistry,
) -> RevalidationOutcome {
    if let Some(guard) = &step.guard {
        if let Some(tag) = check_guard(view, actor, guard) {
            return RevalidationOutcome::Invalidated {
                reason: PlanInvalidationReason::ExpectationMismatch {
                    step_index: bindings.step_index(),
                },
            };
        }
    }
    if requested_affordance_matches(view, actor, step, bindings, registry, handlers) {
        RevalidationOutcome::Valid
    } else {
        RevalidationOutcome::Invalidated {
            reason: PlanInvalidationReason::TargetGone {
                target: bindings.primary_target_entity(),
            },
        }
    }
}
```

### 3. Refactor `revalidate_next_step` to delegate

```rust
pub fn revalidate_next_step(/* same args */) -> bool {
    classify_revalidation(/* same args */).is_valid()
}
```

### 4. Add `check_guard` helper

```rust
fn check_guard(
    view: &dyn RuntimeBeliefView,
    actor: EntityId,
    guard: &PlanGuard,
) -> Option<InvalidatorTag> {
    let profile = view.cognitive_profile(actor);
    let effective_min_confidence = guard
        .min_confidence
        .min(profile.guard_min_confidence_ceiling);

    for fact in &guard.required_facts {
        match fact {
            RequiredFact::TargetPresent { target, at_place } => {
                let believed = view.believed_target_location(actor, *target);
                // Compare `believed.at_place` vs `*at_place`; if mismatch or
                // confidence < effective_min_confidence, return the invalidator
                // tag matching the closest-fit Invalidator on the guard's
                // `invalidators` list.
            }
            RequiredFact::CommodityAvailable { place, kind, min_quantity } => { /* ... */ }
            RequiredFact::RouteKnown { from, to } => {
                if !view.route_exists(*from, *to) {
                    return Some(InvalidatorTag::BeliefStatusChange);
                }
            }
            RequiredFact::ResourceAccess { resource, agent_holds_permission } => { /* ... */ }
        }
    }
    // Check standalone invalidators (TargetMoved, CommodityDepleted,
    // NewBlockerRecorded { baseline_tick }) against envelope / BlockerMemory.
    None
}
```

### 5. Wire `classify_revalidation` into `BestEffort` action start

At the action-start call site (`crates/worldwake-sim/src/tick_step.rs` or equivalent — verify the exact file at implementation time), replace `revalidate_next_step` with `classify_revalidation` and route `RevalidationOutcome::Invalidated { reason }` through `handle_plan_failure` with the specific `PlanInvalidationReason`.

## Files to Touch

- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — new fn, new enum, refactor existing, new `check_guard` helper)
- `crates/worldwake-sim/src/tick_step.rs` (modify — `BestEffort` start-path migration) — verify exact file at implementation time
- `crates/worldwake-ai/src/agent_tick/*.rs` — any call site that already plumbs the `PlanInvalidationReason` should migrate to `classify_revalidation`; identify via grep at implementation time

## Out of Scope

- Event-log emission of `ExpectationMismatch` events from the guard-breach path — ticket 009 owns the AI-side tick step that does the actual emission. This ticket produces the *classification*; the emission path lives elsewhere.
- Populating `mismatch_detail` on `ExpectationMismatchPayload` with `InvalidatorTag` — ticket 009 reads `check_guard`'s return when wiring the overdue-expectation emission.
- Golden test (ticket 010).
- New invalidator kinds beyond the four in S114 D1 — danger-spike, counterparty-unwilling, resource-partial, partial-execution-drift are deferred per spec.

## Acceptance Criteria

### Tests That Must Pass

1. `classify_revalidation_fires_target_moved_on_believed_location_divergence` — new test: guard with `RequiredFact::TargetPresent { target, at_place }` returns `Invalidated { reason: ExpectationMismatch { step_index } }` when envelope's `believed_target_location` places the target elsewhere.
2. `classify_revalidation_fires_on_low_confidence` — guard with `min_confidence: Permille::new(700)` fails when envelope's `BeliefValue::confidence` is `Permille::new(500)`.
3. `classify_revalidation_ignores_irrelevant_drift` — unrelated merchant restock does not trigger an invalidator.
4. `classify_revalidation_respects_profile_confidence_ceiling` — profile with `guard_min_confidence_ceiling: Permille::new(400)` relaxes a guard whose `min_confidence: Permille::new(700)` — effective min is 400, guard passes at confidence 500.
5. `classify_revalidation_route_known_reads_route_exists` — guard with `RequiredFact::RouteKnown` invalidates only when `view.route_exists(from, to)` is false.
6. `revalidate_next_step_delegates_to_classify_revalidation` — the `bool` form returns identical results to `classify_revalidation(...).is_valid()` across a representative sample of the existing 19 tests.
7. Existing suite: all 19 tests at `plan_revalidation.rs:910-1472` stay green byte-for-byte (default `step.guard = None` short-circuits the new pass).
8. Action-trace coverage: `BestEffort` start with a breached guard routes to `handle_plan_failure` with `PlanInvalidationReason::ExpectationMismatch` — add one focused runtime test to `agent_tick/tests.rs` if none exists.

### Invariants

1. `revalidate_next_step` signature unchanged — callers that only need `bool` keep compiling.
2. Guard-check pass runs *before* affordance matching (so a guard breach short-circuits a more expensive affordance re-query).
3. No new `PlanInvalidationReason` variant introduced — reuse `ExpectationMismatch { step_index }`.
4. `check_guard` reads only agent-local state: own belief envelope + own `BlockerMemory`. No cross-agent reads (FND-14, FND-15).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/plan_revalidation.rs` tests module (new tests listed above in Acceptance Criteria 1-6).
2. `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — add action-trace test for guard-breach start path).

### Commands

1. `cargo test -p worldwake-ai plan_revalidation`
2. `cargo test -p worldwake-ai agent_tick`
3. `cargo test -p worldwake-ai` (full AI-crate suite — catches any regression in the 19 existing revalidation tests)
4. `scripts/verify.sh`
