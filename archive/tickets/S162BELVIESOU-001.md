# S162BELVIESOU-001: Contention-read co-location gates

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` belief-view contention accessors (`per_agent_belief_view.rs`)
**Deps**: Spec `../specs/S162-belief-view-source-gate-hardening.md` (D3)

## Problem

Five contention/scheduling accessors on `PerAgentBeliefView` read authoritative
queue/reservation state with **no co-location, own-ticket, or belief gate**, so a
distant actor's planner can perceive remote contention truth it never observed.
This violates FND-7 (locality) and FND-14 (world state ≠ belief state). The fix is
to mirror the gate the lawful sibling methods already use.

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Verified in `crates/worldwake-sim/src/per_agent_belief_view.rs` (2026-05-21): the
   five no-gate methods are `actor_can_claim_extraction_slot` (`:1177`),
   `has_extraction_queues` (`:1191`), `facility_queue_join_tick` (`:1220`),
   `reservation_conflicts` (`:1238`), `reservation_ranges` (`:1245`). Each reads
   `world.get_component_resource_extraction_queues` / `world.get_component_contention_queue`
   / `world.reservations_for` directly with no guard.
2. The lawful template already exists in the same file: `extraction_slot_queue_position`
   (`:1147`), `actor_holds_extraction_slot_grant` (`:1161`), `facility_queue_position`
   (`:1129`), `facility_grant` (`:1138`) all open with
   `if !self.has_authoritative_local_visibility(<entity>) { return ...; }`;
   `contention_queue_is_full` (`:1197`) additionally falls back to
   `believed_contention_state_of` when remote; `facility_queue_patience_ticks`
   (`:1232`) reads the actor's own profile (self-authoritative). Spec D3 and the
   `docs/spec-drafting-rules.md` Belief-View Accessor Source-Class Rule confirm these
   are the correct sources.
3. Shared boundary under audit: the `RuntimeBeliefView` contention accessor surface
   in `per_agent_belief_view.rs`. The only world-reading impls live in this file;
   `belief_view.rs` carries trait signatures/safe defaults only (verified during
   S162 reassessment 2026-05-21). No `belief_view.rs` edit is required.
4. Intended invariant: a remote authoritative contention change with no lawful
   carrier (co-location, the actor's own ticket/reservation, or belief) must not
   change the actor's planner-visible contention reads.
5. Live consumers to trace: these accessors feed candidate generation, ranking, and
   the planning snapshot (`planning_snapshot.rs` reads them via `view.*`). After the
   gate, a remote-only contention fact yields `false`/empty/`None`, so a candidate
   that depended on remote queue/reservation truth is correctly absent. This is the
   intended FND-14B behavior, not a regression.
6. `reservation_conflicts`/`reservation_ranges`: the lawful planner knowledge is
   local observation of the entity or the actor's own reservation belief. If no
   own-reservation belief surface exists today, gate strictly on
   `has_authoritative_local_visibility` and return `false`/empty when remote; an
   own-reservation belief read is an additive enhancement, not required here.
13. Adjacent contradiction: no focused belief-leak tests currently exist for these
    five methods (grep of the file's `#[cfg(test)]` block found none). Adversarial
    remote-vs-local goldens are deliberately deferred to S162BELVIESOU-005; this
    ticket adds focused unit coverage for the gate, the goldens prove the end-to-end
    planner consequence.

## Architecture Check

1. Reusing the existing `has_authoritative_local_visibility` gate keeps the lawful
   and unlawful contention accessors consistent — the inconsistency (some gated,
   some not) was itself the smell. No new abstraction is introduced; the dangerous
   methods simply adopt the proven pattern of their siblings.
2. No backwards-compatibility shim: the unlawful direct read is replaced outright
   (FND-28); no fallback path to the old behavior remains.

## Verified Layers

1. Remote contention change invisible to a non-co-located actor -> focused unit test
   on each gated accessor (actor at a different place than the resource; assert
   `false`/empty/`None` despite live queue/reservation state).
2. Co-located actor still observes contention -> focused unit test (actor at the
   resource's place; assert the gated accessor returns the live value, matching the
   lawful sibling behavior).
3. Planner candidate consequence (remote queue/reservation does not produce a
   candidate) -> deferred to S162BELVIESOU-005 goldens; this ticket's proof surface
   is the focused accessor tests above (strongest lower layer for the gate itself).

## Landed Changes

### 1. Gated the five contention accessors

`actor_can_claim_extraction_slot`, `has_extraction_queues`,
`facility_queue_join_tick`, `reservation_conflicts`, and `reservation_ranges` now
return `false` / empty / `None` before reading live queue or reservation state
unless `has_authoritative_local_visibility(<source/facility/entity>)` succeeds.
No own-ticket/own-reservation belief surface existed in this seam, so this ticket
landed the strict co-location gate described by S162 D3.

### 2. Focused unit coverage

Added `contention_accessors_require_local_visibility`, which proves the five
accessors return the live value for co-located resources/facilities and the empty
result for remote resources/facilities with authoritative queue or reservation
state.

## Landed Files

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modified — five accessor gates + `#[cfg(test)]` unit test)

## Out of Scope

- Adversarial end-to-end belief-wall goldens (S162BELVIESOU-005).
- Control/rights gates (`S162BELVIESOU-002.md`), institutional/social gates (`S162BELVIESOU-003.md`).
- Any new own-reservation belief substrate beyond what already exists — strict
  co-location gating is sufficient and lawful.

## Acceptance Result

### Tests Passed

1. Passed: each of the five accessors returns `false`/empty/`None` for a remote
   non-co-located resource/facility with live contention state.
2. Passed: each accessor returns the live value when the actor is co-located with
   the resource/facility.
3. Passed: `cargo test -p worldwake-sim`.

### Invariants

1. No `PerAgentBeliefView` contention accessor reads `world.*` queue/reservation state without a co-location/own-ticket/belief gate.
2. Co-located observation behavior is unchanged (FND-14A physical perception preserved).

## Test Plan Result

### Added Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (`#[cfg(test)]`) — remote-invisible + co-located-visible cases per gated accessor; rationale: prove the gate without relying on the deferred goldens.

### Commands Run

1. Passed `cargo test -p worldwake-sim contention_accessors_require_local_visibility`
2. Passed `cargo test -p worldwake-sim per_agent_belief_view`
3. Passed `cargo clippy -p worldwake-sim --all-targets -- -D warnings`
4. Passed `cargo test -p worldwake-sim`
5. Waived `./scripts/verify.sh` during this ticket iteration because the S162
   harness runs focused/package proof per ticket and reserves the full wrapper for
   final branch push.

## Outcome

Completed on 2026-05-21.

- Closed the S162 D3 contention leak by putting all five ungated
  `PerAgentBeliefView` queue/reservation reads behind the existing
  `has_authoritative_local_visibility` gate.
- Added focused lower-layer coverage proving remote authoritative contention state
  no longer leaks through the belief view while co-located observation remains
  lawful.
- No new belief substrate, action, component, or planner logic was introduced;
  S162BELVIESOU-005 remains the end-to-end golden owner for planner consequences.

## Deviations

- The ticket allowed an own-ticket/own-reservation belief gate if one already
  existed. Reassessment found no such existing surface for these methods, so the
  landed implementation uses strict co-location gating and returns the empty
  result when remote.

## Verification Result

- Passed `cargo test -p worldwake-sim contention_accessors_require_local_visibility`
- Passed `cargo test -p worldwake-sim per_agent_belief_view`
- Passed `cargo clippy -p worldwake-sim --all-targets -- -D warnings`
- Passed `cargo test -p worldwake-sim`
- Waived `./scripts/verify.sh` until final S162 branch push; the focused
  `worldwake-sim` test, package test, and package all-target clippy gates cover
  this ticket's landed source diff.
