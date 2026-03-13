# EXCFACACCQUE-008 — Search-Layer Queue Routing Verification for Exclusive Facilities

**Status**: COMPLETED

**Spec sections**: `specs/DRAFT-exclusive-facility-access-queues.md` §11
**Crates**: `worldwake-ai`

## Summary

The original ticket aimed at `candidate_generation.rs` and `affordance_query.rs`, but current code no longer routes exclusive-facility use there. Queue/grant routing already lives in planner search over a generic `queue_for_facility_use` affordance. This ticket should therefore verify and harden the existing search-layer architecture rather than push goal-specific queue logic down into candidate generation or the sim affordance query.

## Assumption Reassessment (2026-03-13)

1. `crates/worldwake-ai/src/candidate_generation.rs` does not emit action requests or queue actions. It emits top-level `GoalKind`s such as `ProduceCommodity`, `AcquireCommodity`, and `RestockCommodity`.
2. Exclusive-facility queue routing is already implemented in `crates/worldwake-ai/src/search.rs`:
   - raw `queue_for_facility_use` affordances are expanded into concrete `QueueForFacilityUsePayload { intended_action }`
   - matching grants suppress queue insertion
   - already-queued actors are prevented from receiving duplicate queue candidates
3. `crates/worldwake-sim/src/affordance_query.rs` is intentionally goal-agnostic. It enumerates the raw `queue_for_facility_use` affordance; search binds the intended exclusive action from the grounded goal plus facility facts.
4. The ticket assumption that `get_affordances()` should enumerate one queue affordance per exclusive operation is architecturally stale. That would move goal-specific routing into the sim layer and widen the raw affordance surface for every consumer.
5. The core queue/grant invariants from EXCFACACCQUE-005 through EXCFACACCQUE-007 are already present:
   - belief queries for queue position and grants
   - planning snapshot/state support
   - planner op semantics
   - search-time queue candidate synthesis
6. The remaining value of this ticket is narrower:
   - verify the existing search-layer routing is correct
   - strengthen tests for edge cases not yet covered explicitly
   - archive the ticket with the corrected architectural rationale

## Scope Correction

This ticket should not modify `candidate_generation.rs` or `affordance_query.rs` unless testing exposes a real bug.

The correct scope is:

1. Confirm that exclusive-facility queue routing remains owned by `search.rs`.
2. Add or strengthen tests for missing search-layer invariants.
3. Preserve the current architecture where:
   - candidate generation stays goal-level
   - sim affordance enumeration stays generic
   - planner search performs goal-aware queue payload binding

## Architecture Reassessment

### Preferred design

Keep queue routing in planner search.

That architecture is cleaner than the original ticket proposal because:

1. `candidate_generation.rs` should stay responsible for deciding *what the agent wants*, not *which concrete action payload should satisfy it*.
2. `affordance_query.rs` should stay responsible for enumerating locally possible action shapes, not for inferring goal-specific intended operations.
3. `search.rs` is the first layer that has all three inputs needed to bind queue use correctly:
   - the grounded goal family
   - the concrete facility target
   - the registered harvest/craft action defs and current planning state

### Why the original proposal is worse

Pushing one queue affordance per intended operation into `get_affordances()` would:

1. couple a goal-independent sim query to AI goal semantics
2. duplicate selection logic between search and affordance generation
3. broaden the raw affordance set with action variants that only matter for certain goals
4. make future exclusive systems harder to extend cleanly

### Long-term ideal architecture

The durable shape is:

1. `GoalKind` remains the top-level intent surface.
2. Search derives executable queue steps from goal + facility + planning state.
3. Queue/grant legality remains explicit in world state and belief-safe reads.
4. No compatibility layer and no alias path for autonomous direct exclusive use without a grant.

## What to Change

### 1. Verify existing routing at the correct layer

Keep the existing routing in `crates/worldwake-ai/src/search.rs` and strengthen tests around it.

### 2. Add missing edge-case coverage

Add focused tests proving:

- an actor already queued at an exclusive facility does not receive duplicate queue candidates
- a single raw queue affordance can expand into multiple intended exclusive actions when the goal and registry admit multiple matching operations

## Files to Touch

- `crates/worldwake-ai/src/search.rs` — tests only, unless the tests expose a real bug

## Out of Scope

- Moving queue routing into `crates/worldwake-ai/src/candidate_generation.rs`
- Moving intended-action expansion into `crates/worldwake-sim/src/affordance_query.rs`
- Ranking/runtime changes from EXCFACACCQUE-009
- Failure handling from EXCFACACCQUE-010
- Human-control bypass policy

## Acceptance Criteria

### Tests that must pass

- Search test: actor without matching grant at an exclusive facility gets a queue barrier plan
- Search test: actor with matching grant skips the queue and gets the direct exclusive action plan
- Search test: actor already queued does not receive a duplicate queue candidate
- Search test: one raw queue affordance expands into one concrete queue candidate per matching intended exclusive action
- `cargo test -p worldwake-ai search`
- `cargo test --workspace`
- `cargo clippy --workspace`

### Invariants that must remain true

- Autonomous exclusive-facility routing stays queue/grant based
- Candidate generation remains goal-level and deterministic
- Affordance enumeration remains generic rather than goal-specific
- Search owns binding `QueueForFacilityUsePayload { intended_action }`
- No duplicate queue entries are proposed for an already-queued actor

## Tests

### New or modified tests

- `crates/worldwake-ai/src/search.rs` — `search_does_not_offer_duplicate_queue_candidate_when_actor_is_already_queued`
- `crates/worldwake-ai/src/search.rs` — `queue_affordance_expands_to_one_candidate_per_matching_intended_action`

## Outcome

- Completion date: 2026-03-13
- What actually changed:
  - Corrected the ticket scope from candidate-generation and sim-affordance rewiring to search-layer verification.
  - Added search-layer coverage for duplicate-queue suppression when the actor is already queued.
  - Added search-layer coverage proving one raw queue affordance expands into one concrete queue candidate per matching intended exclusive action.
  - Left production architecture unchanged because the current `candidate_generation -> affordance_query -> search` layering is already cleaner than the original proposal.
- Deviations from the corrected plan:
  - No runtime or production-code fix was needed after reassessment; the value of the ticket was test hardening plus ticket correction.
  - While adding coverage, one initial test assumption was dropped: an already-queued actor does not necessarily imply `search_plan(...) == None`; the real invariant is that search must not synthesize another queue candidate.
- Verification results:
  - `cargo test -p worldwake-ai search`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
