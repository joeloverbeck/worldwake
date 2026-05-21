# S158BELVIEWLEAK-003: Contention accessor leak closure

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `PerAgentBeliefView` contention/queue/grant accessors
**Deps**: None

## Problem

`PerAgentBeliefView`'s contention accessors read current authoritative queue/grant
state with no method-level co-location/belief gate (mitigated today only by caller
discipline). An agent could optimize against a remote facility's queue or grant
state that changed unseen — an FND-14 violation that should not depend on callers
remembering to pass only local facilities. S158 D1 (contention), proven by S158
D4 contention golden.

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Confirmed ungated reads in `crates/worldwake-sim/src/per_agent_belief_view.rs`:
   `facility_queue_position` (1121), `facility_grant` (1127),
   `extraction_slot_queue_position` (1133), `actor_holds_extraction_slot_grant`
   (1144), `contention_queue_is_full` (1177) — all read `world` contention/queue
   components directly with no co-location/belief gate.
2. Source authority: `specs/S158-belief-view-remote-truth-leak-closure.md` D1
   (contention bullet). Remote contention state is belief-backed by the existing
   `EntityBeliefAspect::ContentionState`
   (`crates/worldwake-core/src/entity_belief_claim.rs:27,44`); no new aspect is
   introduced.
3. Shared boundary under audit: the temporal-contention accessor surface of
   `PerAgentBeliefView` consumed by `affordance_query.rs` contention logic and by
   failure classification. Gate predicate: actor is co-located at the facility
   (FND-14A) or holds a `ContentionState` belief; reuse the co-location predicate
   used by `direct_container`.
4. Intended invariant: a remote workstation's reservation/queue/grant changing
   unseen must NOT change the agent's candidates, plans, or affordance set until a
   lawful carrier arrives.
5. Live `GoalKind` under audit: production/restock and any goal whose plan queues
   at a contended facility. Exact surface: the contention reads feeding affordance
   contention handling and failure classification (verify the live failure
   classifier symbols in `failure_handling.rs` during reassessment).
6. Intended verification layer: golden E2E in `belief_wall_trap.rs`; full action
   registries required (contention/reservation).
13. Adjacent contradiction: callers that previously relied on de-facto local-only
    usage now get the gate enforced at the method. Verify no co-located contention
    affordance regresses (negative control). Required consequence, not a new bug.

## Architecture Check

1. Method-level gating removes reliance on caller discipline — the belief view
   enforces the source-class rule itself, so a future caller cannot accidentally
   leak remote contention. Remote contention is sourced from the existing
   `ContentionState` belief aspect; no new stored state, no `Sourced<T>`.
2. No backward-compatibility shim: ungated reads are gated in place; no parallel
   `believed_contention_*` methods (FND-28).

## Verification Layers

1. Remote queue/grant change does not change candidates/plans → decision trace.
2. Remote queue/grant change does not change the affordance set → affordance
   fingerprint.
3. AI and Human control sources see identical lawful affordances → control-source
   swap fingerprint (pattern at line 598).
4. Co-located observed queue still affects the agent's action (negative control)
   → affordance fingerprint / decision trace on a co-located fixture.

## What to Change

### 1. Gate the five contention accessors

In `crates/worldwake-sim/src/per_agent_belief_view.rs`, gate each of
`facility_queue_position`, `facility_grant`, `extraction_slot_queue_position`,
`actor_holds_extraction_slot_grant`, `contention_queue_is_full` on co-location of
the facility/source with the observing agent (FND-14A) or an existing
`ContentionState` belief; return `None` / `false` for remote facilities with no
belief.

### 2. Add contention golden (failing-first)

In `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs`:
- `golden_belief_wall_trap_remote_queue_grant_unseen` — remote facility's
  queue/grant changes unseen; assert the planner cannot optimize against it and
  contention affordances are unchanged.
- Extend the control-source-swap fingerprint to this scenario.
- Negative control: a co-located observed queue still affects the agent's action.
The new leak golden must fail against current `main` and pass after section 1.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` (modify)

## Out of Scope

- Economic (ticket 001), production/physical (ticket 002) accessors.
- `can_control` / `believed_rights` (S158 Non-Goals).
- New `EntityBeliefAspect` variants (uses existing `ContentionState`).
- Doc updates (ticket 004).

## Acceptance Criteria

### Tests That Must Pass

1. `golden_belief_wall_trap_remote_queue_grant_unseen` — no remote contention
   knowledge; contention affordances unchanged.
2. Negative control: co-located observed queue still affects the agent's action
   for AI and Human control sources.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. No contention accessor returns a remote facility's current world value;
   remote contention knowledge arrives only via belief carriers (FND-14).
2. AI and Human control sources produce identical lawful affordances (FND-19).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/scenarios/belief_wall_trap.rs` — remote
   queue/grant golden + co-located negative control + control-swap fingerprint
   extension; rationale: prove the contention leak is closed at the method, not by
   caller discipline, without over-suppressing co-located contention.

### Commands

1. `cargo test -p worldwake-ai --test golden_ai belief_wall_trap` (confirm names
   with `cargo test -p worldwake-ai --test golden_ai -- --list`)
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `./scripts/verify.sh`
