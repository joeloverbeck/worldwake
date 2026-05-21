# S162BELVIESOU-002: Control & rights belief gates

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-sim` belief-view control/rights accessors (`per_agent_belief_view.rs`)
**Deps**: Spec `specs/S162-belief-view-source-gate-hardening.md` (D1, D5)

## Problem

`has_control` reads `AgentData.control_source` for any entity with **no gate** —
control source is meta-control (FND-19), not character-perceivable. `believed_rights`
and `can_control` gate accessibility partly on authoritative
`world.possessor_of`/`world.owner_of` reads, and `can_control`'s unowned-item branch
returns `true` from a co-location + `world.owner_of(..).is_none()` check — but
ownership (and ownership *absence*) is a social fact requiring belief, not a physical
observation (FND-14A). This ticket gates all three to self/belief sources.

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Verified in `crates/worldwake-sim/src/per_agent_belief_view.rs` (2026-05-21):
   `has_control` (`:461-465`) reads `world.get_component_agent_data(entity).control_source`
   with no gate; `believed_rights` (`:428-439`) and `can_control` (`:441-459`) compute
   `accessible` via `entity == self.agent || self.believed_entity(entity).is_some() ||
   self.world.possessor_of(entity) == Some(self.agent) || self.world.owner_of(entity)
   == Some(self.agent)` then call `world.effective_rights`/`world.can_exercise_control`;
   `can_control`'s lines `:442-449` return `true` for a co-located, unowned, item-kind
   entity via `world.owner_of(entity).is_none()`.
2. Existing tests encoding current behavior (grep of `#[cfg(test)]`, 2026-05-21):
   `believed_rights_returns_rights_for_known_entity` (`:5476`),
   `believed_rights_returns_empty_for_unknown_entity` (`:5504`),
   `can_control_returns_false_for_belief_inaccessible_authoritatively_controlled_entity`
   (`:5527`), `can_control_returns_true_for_belief_accessible_controlled_entity`
   (`:5573`), `can_control_returns_true_for_colocated_unowned_item_without_belief`
   (`:5603`); and `affordance_query.rs::per_agent_belief_view_none_control_only_changes_actor_has_control_actions`
   (`:2323`). Spec D1/D5 and `docs/spec-drafting-rules.md` (social/relational facts are
   belief-gated even when co-located) govern the corrected contract.
3. Shared boundary under audit: the control/rights accessor surface in
   `per_agent_belief_view.rs` (`believed_rights`/`can_control`/`has_control`). Only
   this file holds world-reading impls; `belief_view.rs` carries trait
   signatures/safe defaults. Authoritative dispatch (`World::can_exercise_control` at
   action start) is unchanged — lawful at the dispatch boundary (FND-14B final
   paragraph). This ticket changes only the belief-facing accessors.
4. Intended invariant: a remote ownership/control change with no lawful carrier must
   not change `believed_rights`/`can_control`/`has_control`; control source is not
   character knowledge for non-self entities.
8. Heuristic being removed: the `world.possessor_of`/`world.owner_of` accessibility
   probes and the `world.owner_of(..).is_none()` unowned-item shortcut. The substrate
   they stand in for is the belief store (`believed_entity`) and, for self-relation,
   the actor's self-authoritative state. The change does not reopen regressions
   because lawful self/belief accessibility still permits every read an observer
   would lawfully have; only un-carried remote reads are removed.
11. ControlSource handling: `has_control` for non-self must not read
    `AgentData.control_source` (meta per FND-19). For `self`, self-authoritative is
    fine. For other entities, return `false` unless an explicit institutional control
    belief covers the subject. The `affordance_query.rs:2323` test asserts has_control
    actions change with control source — confirm it switches the *actor's own* control
    (self), which remains lawful; if it asserts visibility of another entity's control
    source, the test encodes the leak and must be revised to the gated contract.
13. Adjacent contradictions (classified): `can_control_returns_true_for_colocated_unowned_item_without_belief`
    (`:5603`) asserts the exact unlawful behavior D5 removes — this is a **required
    consequence** of the ticket (a spec-driven contract change, not adapting a test to
    a bug). Revise it to assert that an unowned item requires a belief (or a lawful
    physical-observation expression) rather than a bare ownership-absence read. The
    `:5527`/`:5573` tests gate on `believed_entity` and should survive; re-verify after
    removing the owner/possessor probes.

## Architecture Check

1. Gating on self + `believed_entity` (and institutional control belief where it
   exists) makes the accessor's source class match its name and the
   `docs/spec-drafting-rules.md` rule. Removing the authoritative owner/possessor
   probes eliminates the FND-14A violation at the root rather than masking it. The
   actual confrontation/control check at dispatch is untouched, preserving correct
   authoritative enforcement.
2. No backwards-compatibility shim: the unlawful probes are deleted (FND-28), not
   wrapped; tests encoding the old contract are revised, not duplicated.

## Verification Layers

1. Remote ownership/control change invisible (no carrier) -> focused unit test:
   actor believes old state, world changes remotely, `believed_rights`/`can_control`
   unchanged.
2. `has_control` returns `false` for a non-self, un-believed entity regardless of its
   live control source -> focused unit test.
3. Self/believed accessibility still grants lawful reads -> revised `:5527`/`:5573`
   tests plus a self-control case.
4. Planner/affordance consequence (no new control/rights affordance from a remote
   change) -> deferred to S162BELVIESOU-005 goldens; this ticket's proof is the
   focused accessor tests (strongest lower layer for the gate).

## What to Change

### 1. `has_control` (`:461`)

Gate to self-authoritative for `entity == self.agent`; for other entities, return
`false` unless an explicit institutional control belief exists for the subject. Do
not read `AgentData.control_source` for non-self entities.

### 2. `believed_rights` (`:428`) / `can_control` (`:441`)

Remove the `world.possessor_of(entity) == Some(self.agent)` and
`world.owner_of(entity) == Some(self.agent)` accessibility probes (`:433-434`,
`:453-454`); keep `entity == self.agent || self.believed_entity(entity).is_some()`.
Replace `can_control`'s unowned-item branch (`:442-449`) so the unownedness it relies
on comes from belief (or a lawful FND-14A physical observation expressed as such),
not `world.owner_of(..).is_none()`. The authoritative `effective_rights`/
`can_exercise_control` calls remain only behind the lawful self/belief gate.

### 3. Revise leak-encoding tests

Update `can_control_returns_true_for_colocated_unowned_item_without_belief` (`:5603`)
to the gated contract; re-verify `:5527`/`:5573` and `affordance_query.rs:2323`
against the corrected behavior per Assumption Reassessment 11/13.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — three accessors + revised `#[cfg(test)]` tests)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — re-verify/adjust `per_agent_belief_view_none_control_only_changes_actor_has_control_actions` if it asserted non-self control visibility)

## Out of Scope

- Contention gates (S162BELVIESOU-001), institutional/social gates (S162BELVIESOU-003).
- Authoritative `can_exercise_control` / dispatch-time validation — unchanged.
- Adversarial end-to-end goldens (S162BELVIESOU-005).

## Acceptance Criteria

### Tests That Must Pass

1. New: `has_control` returns `false` for a non-self entity with a live non-`None` control source the actor has no belief about.
2. Revised: `can_control` requires belief (or lawful physical observation) for a co-located unowned item — no bare ownership-absence read.
3. New: remote ownership/control change with no carrier leaves `believed_rights`/`can_control` unchanged.
4. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. No belief-facing control/rights accessor reads `world.*` ownership/control/possession state outside a self or belief gate.
2. Authoritative dispatch enforcement (`can_exercise_control`) is unchanged.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (`#[cfg(test)]`) — new has_control non-self gate test, new remote-change-invisible test, revised `:5603` unowned-item test; rationale: prove the gated contract and the removed leak.
2. `crates/worldwake-sim/src/affordance_query.rs` (`#[cfg(test)]`) — re-verify `:2323` reflects self-control, not non-self control-source visibility.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-sim affordance_query`
3. `./scripts/verify.sh` (before PR)
