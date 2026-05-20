# S149PARPLASEG-007: Coordination-barrier resume listening

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — per-actor contention "watching" list and contention-state resume trigger
**Deps**: archive/tickets/S149PARPLASEG-004.md, archive/tickets/S149PARPLASEG-005.md, archive/tickets/S149PARPLASEG-010.md

## Problem

D8 lets an intention suspended on a `CoordinationBarrier` resume when the contested resource becomes available again. The agenda manager adds the suspended intention to a per-actor "watching" list keyed on `contested_resource`, and the existing contention lifecycle (grant expiry / re-grant / queue-head promotion) fires the resume condition. No new event tag is introduced.

## Assumption Reassessment (2026-05-20)

1. `ContentionGrant` is at `crates/worldwake-core/src/contention.rs:43` with `{ actor, intended_action, granted_at, expires_at }`. Its lifecycle is queue-state-mediated (expiry via `expires_at`); reassessment found NO discrete "grant invalidation" event type. The live queue-state signal is `ContentionQueue.granted`: `crates/worldwake-systems/src/facility_queue.rs` clears expired grants and promotes ready heads, while AI reads the facility grant through `RuntimeBeliefView::facility_grant` (`crates/worldwake-sim/src/belief_view.rs:1157`). The resume trigger must read that existing queue state, not a phantom event.
2. The barrier records `BlockingFact::ReservationConflict { affordance, contention_event }` (ticket 004); the resume condition is `IntentionResumeCondition::ArtifactLegalEffectActive(contested_resource)` or `OpportunityVisible` (ticket 004 derivation).
3. Shared boundary under audit: the per-actor watching list is derived AI-local bookkeeping from suspended `PartialPlanSegment`s on `AgendaState.suspended`, not a new persisted `AgendaState` field. This avoids a second stored representation of the same watched-resource fact and avoids a save-version bump. The existing contention-queue state surface remains authoritative. Phase distinction: this ticket is the resume-trigger wiring; the resume decision is ticket 005, executable segment writing/re-entry is ticket 010, and barrier attribution is ticket 004.
4. Information-path: the resume signal is a read of existing contention-queue state by the watching agent's agenda pass — no new transport path, no new event tag (FND-26, state-mediated).
5. Multi-substrate note: contention spans facility queues (`ContentionQueue`) and resource-extraction queues (`ResourceExtractionQueues`). This ticket hooks the existing `RuntimeBeliefView` read surface for both: facility queues through `facility_grant`, and extraction queues through the already-live `actor_holds_extraction_slot_grant` / `actor_can_claim_extraction_slot` methods. The golden in 009 uses an oven-reservation/facility-queue case, so focused proof must cover the facility path.

## Architecture Check

1. A derived read-side watching list that polls existing contention-queue state avoids inventing a new authoritative event, persisted agenda field, or grant-invalidation channel (FND-26/FND-28). The contention state remains the single source of truth; the watch list is agent-local runtime bookkeeping derived from the already persisted suspended segment.
2. No new event tag preserves the spec's "no new event tag for barrier transitions" non-goal; the resume condition is the existing `ArtifactLegalEffectActive`/`OpportunityVisible`.

## Verification Layers

1. Suspended coordination intention is added to the watching list keyed on `contested_resource` → focused runtime test.
2. Contention-state transition (grant expiry / re-grant) fires the resume condition for a watching intention → focused runtime test on the watching-list check against contention state.
3. Resume re-entry after re-availability → exercised by ticket 010's executable re-entry path + ticket 009 golden (oven-reservation case); this ticket asserts the trigger, not the full re-plan.

## Landed Changes

### 1. Derived per-actor watching list

Added `coordination_barrier_watch_list` on the agenda manager, keyed by `contested_resource`. A `CoordinationBarrier` suspension is visible in that derived list through its suspended `PartialPlanSegment`; no new persisted `AgendaState` field was added.

### 2. Resume trigger from contention state

`partial_plan_resume_condition_holds` now treats a coordination barrier's `ArtifactLegalEffectActive(contested_resource)` condition as satisfied when existing contention state says the resource is available again: a managed facility has no active grant, the grant belongs to the actor, or an extraction resource is claimable/held by the actor. Ordinary artifact legal-effect resume conditions still use the artifact-belief path.

## Landed Files

- `crates/worldwake-ai/src/agenda_manager.rs` — derived watch-list helper, coordination-resource resume check, and focused tests.
- `archive/tickets/S149PARPLASEG-007.md` — reassessment, closeout truthing, and archival.
- No change: `crates/worldwake-systems/src/facility_queue.rs`; reassessment confirmed it already clears expired grants and promotes ready queue heads.

## Out of Scope

- Adding any new event tag or grant-invalidation event (explicitly avoided).
- The resume decision mechanics (ticket 005).
- Executable segment writing and tactical re-entry (ticket 010).
- Information-barrier companions (ticket 006).

## Acceptance Result

### Tests That Passed

1. Added: `coordination_barrier_watch_list_indexes_suspended_segments_by_resource` proves a suspended `CoordinationBarrier` segment appears in the derived watch list keyed by `contested_resource`.
2. Added: `coordination_barrier_resume_waits_while_facility_grant_belongs_to_other_actor`, `coordination_barrier_resume_fires_when_facility_grant_is_available_again`, and `coordination_barrier_resume_fires_when_facility_grant_promotes_to_actor` prove the facility-queue resume trigger.
3. Existing suite passed: `cargo test -p worldwake-ai --quiet`.

### Invariants

1. No new `EventTag` variant was introduced; the resume signal is a read of existing contention-queue state.
2. The watching list is derived agent-local runtime bookkeeping, not authoritative contention state and not an additional persisted agenda field (FND-26/FND-28).

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` inline tests — watching-list registration and facility contention-state resume trigger.

## Outcome

Completed on 2026-05-20.

- Added a derived coordination-barrier watch-list helper from suspended `PartialPlanSegment`s.
- Wired coordination-barrier resume checks to the existing contention-state read surface: facilities resume when no grant blocks the actor or the grant belongs to the actor; extraction resources use the existing claimable/held slot methods.
- Preserved the spec's no-new-event-tag and state-mediated contention contract.

## Deviations

- The drafted ticket described "new ai runtime state"; live reassessment narrowed that to a derived watch list over already persisted suspended segments. This avoids duplicating the watched-resource fact and avoids a save-format bump.
- `crates/worldwake-systems/src/facility_queue.rs` was read for the queue-state signal but did not require edits.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib agenda_manager::tests::coordination_barrier_`
- Passed `cargo test -p worldwake-ai --quiet`
- Passed `cargo fmt --all`
- Waived `scripts/verify.sh` for this ticket iteration because the harness used the ticket's affected-crate proof before post-ticket review; the full repository pre-push gate remains the harness finalization requirement.
