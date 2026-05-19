# S149PARPLASEG-004: Barrier to failure-attribution mapping

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — barrier-to-Discrepancy/BlockingFact routing and resume-condition derivation
**Deps**: archive/tickets/S149PARPLASEG-001.md, S149PARPLASEG-002

## Problem

Each typed barrier must flow through the existing failure-handling pipeline so it is not silent (D3 design goal). D6 maps four barriers onto existing `Discrepancy` variants, routes `CoordinationBarrier` onto the existing `BlockingFact::ReservationConflict` blocker surface, and derives resume conditions from the barrier fact using the existing core `IntentionResumeCondition` variants.

## Assumption Reassessment (2026-05-20)

1. `Discrepancy` is at `crates/worldwake-core/src/discrepancy.rs:9`. Confirmed present: `MissingObservation`, `BeliefStale`, `NoLegalBinding`, `NeedHorizonExceeded`, `SearchBudgetExhausted`. There is NO `Discrepancy::ReservationConflict` — that variant lives on `BlockingFact::ReservationConflict { affordance, contention_event }` (`crates/worldwake-core/src/blocker_memory.rs:241`). So `CoordinationBarrier` routes to `BlockingFact`, not `Discrepancy`.
2. Resume conditions use the existing `IntentionResumeCondition` variants (`crates/worldwake-core/src/intention_condition.rs:7`): `BeliefStatusChanged { subject, target_status: BeliefStatusTag }`, `OpportunityVisible`, `LocationReached`, `TickElapsed(u32)`, `ArtifactLegalEffectActive(EntityId)`. The spec's original `BeliefUpdated`/`ArtifactValid` names do not exist; the corrected derivation uses the real variants.
3. Shared boundary under audit: `terminal_to_discrepancy` (new ai fn) over the post-001 `PlanTerminalKind`, plus the existing `DiscrepancyMemory` path (S109) and the existing `BlockerMemory`/`BlockingFact` path. `BarrierFact` (ticket 002) is the input to resume-condition derivation. Phase distinction: this is the failure-attribution + resume-condition-derivation phase; it does not itself perform resumption (ticket 005) or spawn subgoals (006/007).
4. Budget cooldown reuses the existing `CognitiveProfile.search_exhaustion_backoff_ticks` (`crates/worldwake-core/src/cognitive_profile.rs:56`); no new `budget_cooldown_ticks` field is introduced (FND-28 — would duplicate an existing field).
5. `discrepancy_clearing_dispatch_covers_all_variants` (`plan_repair.rs:484`) exercises `Discrepancy` exhaustiveness — reuse of existing variants means no new arm, but verify the test still passes since this ticket constructs new `Discrepancy` values from barriers.

## Architecture Check

1. Reusing existing `Discrepancy` variants and the `BlockingFact::ReservationConflict` blocker surface avoids extending the typed-failure taxonomy (FND-28: no parallel surface). Routing coordination to `BlockingFact` honors the post-S109 split where contention attribution lives on the blocker taxonomy, not on `Discrepancy`.
2. Resume-condition derivation reads barrier facts and emits the existing core condition enum — state-mediated, no cross-system command (FND-26).

## Verification Layers

1. Each typed barrier maps to the correct failure surface → focused unit test on `terminal_to_discrepancy` (4 `Discrepancy` mappings; `CoordinationBarrier` and the success terminals → `None`).
2. `CoordinationBarrier` records a `BlockingFact::ReservationConflict` → focused unit test asserting the blocker is written with the contested affordance + contention event.
3. Resume-condition derivation → focused unit test mapping each `BarrierFact` variant to the expected `IntentionResumeCondition` (e.g. `BudgetExhausted` → `TickElapsed(search_exhaustion_backoff_ticks)`).

## What to Change

### 1. `terminal_to_discrepancy`

Add the mapping fn in the ai crate (near the agenda manager / failure-handling): `InformationBarrier → MissingObservation`, `ResourceBarrier → BeliefStale`, `JurisdictionBarrier → NoLegalBinding`, `SearchBudgetExhausted → SearchBudgetExhausted`, `GoalSatisfied`/`CombatCommitment`/`CoordinationBarrier → None`. Record through the existing `DiscrepancyMemory` path.

### 2. CoordinationBarrier → BlockingFact

For `CoordinationBarrier { contested_resource }`, record `BlockingFact::ReservationConflict { affordance, contention_event }` through the existing `BlockerMemory` path, deriving `affordance` from the contested affordance and `contention_event` from the blocking contention event.

### 3. Resume-condition derivation from `BarrierFact`

Map each `BarrierFact` to a `Vec<IntentionResumeCondition>`: `MissingBelief → BeliefStatusChanged{subject,target_status}`; `ContestedReservation(target) → ArtifactLegalEffectActive(target)` (or `OpportunityVisible`); `DepletedResource{place,..} → BeliefStatusChanged{subject:place,..}`; `NoAuthorityForAction(authority) → ArtifactLegalEffectActive(authority)`; `BudgetExhausted → TickElapsed(cognitive.search_exhaustion_backoff_ticks)`.

## Files to Touch

- `Likely: crates/worldwake-ai/src/agenda_manager.rs` or `crates/worldwake-ai/src/failure_handling.rs` (modify) — `terminal_to_discrepancy`, blocker routing, resume-condition derivation; grep `DiscrepancyMemory` and `BlockerMemory` write sites to pin placement
- `crates/worldwake-ai/src/partial_plan.rs` (modify, if the derivation helper lives beside `BarrierFact`)

## Out of Scope

- Performing resumption when a condition holds (ticket 005).
- Companion `AskWitness` synthesis (006) and coordination watching list (007).
- Adding any new `Discrepancy` or `IntentionResumeCondition` variant (all reused).

## Acceptance Criteria

### Tests That Must Pass

1. New: `terminal_to_discrepancy` returns the four expected `Discrepancy` values and `None` for success + `CoordinationBarrier`.
2. New: `CoordinationBarrier` records `BlockingFact::ReservationConflict` with the contested affordance and contention event.
3. New: `BudgetExhausted` derives `TickElapsed(search_exhaustion_backoff_ticks)`; each other `BarrierFact` derives its expected condition.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No new `Discrepancy` variant and no new `IntentionResumeCondition` variant are introduced (reuse only).
2. Coordination barriers never produce a `Discrepancy`; they produce a `BlockingFact::ReservationConflict`.

## Test Plan

### New/Modified Tests

1. ai crate (inline, beside the mapping fn) — `terminal_to_discrepancy`, blocker routing, and resume-condition derivation cases.

### Commands

1. `cargo test -p worldwake-ai`
2. `scripts/verify.sh`
