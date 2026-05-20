# S149PARPLASEG-007: Coordination-barrier resume listening

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — per-actor contention "watching" list and contention-state resume trigger
**Deps**: archive/tickets/S149PARPLASEG-004.md, archive/tickets/S149PARPLASEG-005.md, archive/tickets/S149PARPLASEG-010.md

## Problem

D8 lets an intention suspended on a `CoordinationBarrier` resume when the contested resource becomes available again. The agenda manager adds the suspended intention to a per-actor "watching" list keyed on `contested_resource`, and the existing contention lifecycle (grant expiry / re-grant / queue-head promotion) fires the resume condition. No new event tag is introduced.

## Assumption Reassessment (2026-05-20)

1. `ContentionGrant` is at `crates/worldwake-core/src/contention.rs:43` with `{ actor, intended_action, granted_at, expires_at }`. Its lifecycle is queue-state-mediated (expiry via `expires_at`); reassessment found NO discrete "grant invalidation" event type — the resume trigger must hook contention-queue state transitions, not a phantom event. (Confirm the exact queue-state signal during implementation; `Likely: crates/worldwake-systems/src/facility_queue.rs` or the contention-queue state module — grep `ContentionGrant` / `ContentionQueue` consumers.)
2. The barrier records `BlockingFact::ReservationConflict { affordance, contention_event }` (ticket 004); the resume condition is `IntentionResumeCondition::ArtifactLegalEffectActive(contested_resource)` or `OpportunityVisible` (ticket 004 derivation).
3. Shared boundary under audit: the per-actor watching list (new ai runtime state on the agenda manager) and the existing contention-queue state surface. Phase distinction: this ticket is the resume-trigger wiring; the resume decision is ticket 005, executable segment writing/re-entry is ticket 010, and barrier attribution is ticket 004.
4. Information-path: the resume signal is a read of existing contention-queue state by the watching agent's agenda pass — no new transport path, no new event tag (FND-26, state-mediated).
5. Multi-substrate note: contention spans facility queues (`ContentionQueue`) and resource-extraction queues (`ResourceExtractionQueues`). This ticket hooks the substrate(s) carrying the `contested_resource` for the coordination-barrier scenario; confirm which substrate the watching list reads during implementation and scope the hook to it (the golden in 009 uses an oven-reservation/facility-queue case).

## Architecture Check

1. A read-side watching list that polls existing contention-queue state avoids inventing a new authoritative event or grant-invalidation channel (FND-26/FND-28). The contention state remains the single source of truth; the watching list is agent-local runtime bookkeeping.
2. No new event tag preserves the spec's "no new event tag for barrier transitions" non-goal; the resume condition is the existing `ArtifactLegalEffectActive`/`OpportunityVisible`.

## Verification Layers

1. Suspended coordination intention is added to the watching list keyed on `contested_resource` → focused runtime test.
2. Contention-state transition (grant expiry / re-grant) fires the resume condition for a watching intention → focused runtime test on the watching-list check against contention state.
3. Resume re-entry after re-availability → exercised by ticket 010's executable re-entry path + ticket 009 golden (oven-reservation case); this ticket asserts the trigger, not the full re-plan.

## What to Change

### 1. Per-actor watching list

Add per-actor watching-list state on the agenda manager keyed by `contested_resource`. On `CoordinationBarrier` suspension, register the intention.

### 2. Resume trigger from contention state

During the agenda tick pass, check the watching list against current contention-queue state; when the contested resource becomes available (grant expired / re-granted / queue-head promoted to this actor), satisfy the intention's `ArtifactLegalEffectActive`/`OpportunityVisible` resume condition so ticket 005's resume path picks it up.

## Files to Touch

- `crates/worldwake-ai/src/agenda_manager.rs` (modify) — watching list + resume trigger
- `Likely: crates/worldwake-systems/src/facility_queue.rs` (read-only consumer reference) — grep `ContentionQueue`/`ContentionGrant` state to pin the queue-state signal the watching check reads

## Out of Scope

- Adding any new event tag or grant-invalidation event (explicitly avoided).
- The resume decision mechanics (ticket 005).
- Executable segment writing and tactical re-entry (ticket 010).
- Information-barrier companions (ticket 006).

## Acceptance Criteria

### Tests That Must Pass

1. New: a `CoordinationBarrier`-suspended intention is registered in the per-actor watching list keyed on `contested_resource`.
2. New: a contention-state transition that re-makes the resource available satisfies the watching intention's resume condition.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No new `EventTag` variant is introduced; the resume signal is a read of existing contention-queue state.
2. The watching list is agent-local runtime state, not authoritative contention state (FND-27).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` (inline) — watching-list registration + contention-state resume trigger.

### Commands

1. `cargo test -p worldwake-ai`
2. `scripts/verify.sh`
