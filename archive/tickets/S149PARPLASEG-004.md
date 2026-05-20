# S149PARPLASEG-004: Barrier to failure-attribution mapping

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — barrier-to-Discrepancy/BlockingFact routing and resume-condition derivation
**Deps**: archive/tickets/S149PARPLASEG-001.md, archive/tickets/S149PARPLASEG-002.md

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

## Verified Layers

1. Each typed barrier maps to the correct failure surface → `partial_plan::tests::typed_terminals_map_to_existing_discrepancy_surface` covers the four `Discrepancy` mappings and confirms `CoordinationBarrier`, `GoalSatisfied`, and `CombatCommitment` do not produce discrepancies.
2. `CoordinationBarrier` records a `BlockingFact::ReservationConflict` → `partial_plan::tests::coordination_barrier_records_reservation_conflict_blocker` asserts the blocker write carries the contested affordance and contention event through `BlockerMemory`.
3. Resume-condition derivation → `partial_plan::tests::barrier_facts_derive_existing_resume_conditions` maps every `BarrierFact` variant to an existing `IntentionResumeCondition`; fixed-entity `MissingBelief` predicates derive `BeliefStatusChanged`, commodity/location facts use their dedicated concrete barrier facts, and `BudgetExhausted` uses `CognitiveProfile.search_exhaustion_backoff_ticks`.

## Landed Changes

### 1. `terminal_to_discrepancy`

Added `terminal_to_discrepancy` in `crates/worldwake-ai/src/partial_plan.rs` and re-exported it from the ai crate. The helper maps `InformationBarrier → MissingObservation`, `ResourceBarrier → BeliefStale`, `JurisdictionBarrier → NoLegalBinding`, `SearchBudgetExhausted → SearchBudgetExhausted`, and returns `None` for `GoalSatisfied`, `CombatCommitment`, and `CoordinationBarrier`.

### 2. CoordinationBarrier → BlockingFact

Added `coordination_barrier_blocking_fact`, `CoordinationBarrierBlockerRecord`, and `record_coordination_barrier_blocker` in `partial_plan.rs`, with ai-crate re-exports. The write path records `BlockingFact::ReservationConflict { affordance, contention_event }` through `BlockerMemory`, uses `BlockerClearingCondition::ContentionChanged`, and rejects terminals whose `contested_resource` does not match the supplied affordance facility.

### 3. Resume-condition derivation from `BarrierFact`

Added `resume_conditions_for_barrier_fact` in `partial_plan.rs`. It maps fixed-entity `MissingBelief` predicates to `BeliefStatusChanged { subject, target_status: Certain }`, `ContestedReservation(target)` to `ArtifactLegalEffectActive(target)`, `DepletedResource { place, .. }` to `BeliefStatusChanged { subject: place, target_status: Certain }`, `NoAuthorityForAction(authority)` to `ArtifactLegalEffectActive(authority)`, and `BudgetExhausted` to `TickElapsed(cognitive.search_exhaustion_backoff_ticks)`. Commodity-shaped `MissingBelief` predicates intentionally return no invented entity condition at this helper seam because the concrete commodity/place resume path is represented by `BarrierFact::DepletedResource`.

## Landed Files

- `crates/worldwake-ai/src/partial_plan.rs` — canonical barrier-to-discrepancy, coordination-blocker, and resume-condition helpers plus focused unit tests.
- `crates/worldwake-ai/src/lib.rs` — ai-crate re-exports for the new partial-plan helper surface.

## Out of Scope

- Performing resumption when a condition holds (ticket 005).
- Companion `AskWitness` synthesis (006) and coordination watching list (007).
- Adding any new `Discrepancy` or `IntentionResumeCondition` variant (all reused).

## Acceptance Criteria

### Acceptance Result

1. Passed: `terminal_to_discrepancy` returns the four expected `Discrepancy` values and `None` for success + `CoordinationBarrier`.
2. Passed: `CoordinationBarrier` records `BlockingFact::ReservationConflict` with the contested affordance and contention event.
3. Passed: `BudgetExhausted` derives `TickElapsed(search_exhaustion_backoff_ticks)` and each other concrete `BarrierFact` derives its expected existing condition.
4. Passed: `cargo test -p worldwake-ai`.

### Invariants

1. The landed implementation reused existing `Discrepancy` and `IntentionResumeCondition` variants only.
2. Coordination barriers never produce a `Discrepancy`; they produce a `BlockingFact::ReservationConflict` when supplied a matching contested affordance.

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/src/partial_plan.rs` inline unit tests cover `terminal_to_discrepancy`, coordination blocker routing, and resume-condition derivation.

### Executed Commands

1. Passed `cargo test -p worldwake-ai --lib partial_plan::tests`.
2. Passed `cargo test -p worldwake-ai`.

## Deviations

- The helper surface landed beside `BarrierFact` in `partial_plan.rs`, not in `agenda_manager.rs` or `failure_handling.rs`, because this ticket adds reusable partial-plan attribution helpers rather than live resumption behavior.
- `MissingBelief` only derives `BeliefStatusChanged` when the predicate contains a fixed entity subject. Commodity-shaped missing beliefs do not synthesize a fake subject; commodity/place resume is represented by `BarrierFact::DepletedResource`.
- `scripts/verify.sh` was not run for this iteration because the ticket's required behavioral proof is the `worldwake-ai` crate suite; the harness will run the full pre-PR gate at final branch push.

## Outcome

Completed on 2026-05-20.

- Added the canonical typed-terminal-to-discrepancy helper for S149 partial-plan barriers.
- Added coordination-barrier blocker construction and recording through the existing `BlockerMemory` / `BlockingFact::ReservationConflict` surface.
- Added resume-condition derivation from `BarrierFact` using existing `IntentionResumeCondition` variants and existing `CognitiveProfile.search_exhaustion_backoff_ticks`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib partial_plan::tests` after the final source diff.
- Passed `cargo test -p worldwake-ai` after the final source diff.
