# S162BELVIESOU-002: Control & rights belief gates

**Status**: COMPLETED
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
   (`:5573`), the pre-implementation
   `can_control_returns_true_for_colocated_unowned_item_without_belief` leak test
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
13. Adjacent contradictions (classified): the old
    `can_control_returns_true_for_colocated_unowned_item_without_belief` assertion
    encoded the exact unlawful behavior D5 removed — this was a **required consequence**
    of the ticket (a spec-driven contract change, not adapting a test to a bug). It was
    replaced by `can_control_returns_false_for_colocated_unowned_item_without_belief`.
    The `:5527`/`:5573` tests stayed green after removing the owner/possessor probes.

## Architecture Check

1. Gating on self + `believed_entity` (and institutional control belief where it
   exists) makes the accessor's source class match its name and the
   `docs/spec-drafting-rules.md` rule. Removing the authoritative owner/possessor
   probes eliminates the FND-14A violation at the root rather than masking it. The
   actual confrontation/control check at dispatch is untouched, preserving correct
   authoritative enforcement.
2. No backwards-compatibility shim: the unlawful probes are deleted (FND-28), not
   wrapped; tests encoding the old contract are revised, not duplicated.

## Verified Layers

1. Remote ownership/control change invisible (no carrier) -> focused unit test:
   actor has no belief carrier for the remote subject, world ownership changes remotely,
   `believed_rights`/`can_control` stay unchanged.
2. `has_control` returns `false` for a non-self, un-believed entity regardless of its
   live control source -> focused unit test.
3. Self/believed accessibility still grants lawful reads -> revised `:5527`/`:5573`
   tests plus a self-control case.
4. Planner/affordance consequence (no new control/rights affordance from a remote
   change) -> deferred to S162BELVIESOU-005 goldens; this ticket proved the strongest
   lower-layer accessor gates.

## Landed Changes

### 1. `has_control` (`:461`)

`has_control` is self-authoritative only. For `entity != self.agent`, it returns
`false` without reading `AgentData.control_source`.

### 2. `believed_rights` (`:428`) / `can_control` (`:441`)

Removed the `world.possessor_of(entity) == Some(self.agent)` and
`world.owner_of(entity) == Some(self.agent)` accessibility probes. Both accessors now
use `entity == self.agent || self.believed_entity(entity).is_some()`. Removed
`can_control`'s co-located unowned-item shortcut, so a bare live ownership-absence
read no longer grants control.

### 3. Revise leak-encoding tests

Replaced the leak-encoding colocated-unowned assertion with the gated contract, added
non-self `has_control` coverage, and added a remote no-carrier control/rights
invisibility regression. Re-verified the existing belief-accessible control tests and
`affordance_query.rs` control-filter coverage.

## Landed Files

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modified — three accessors + focused `#[cfg(test)]` tests)
- `crates/worldwake-sim/src/affordance_query.rs` (checked; no source change required because the live test covers self-control only)

## Out of Scope

- Contention gates (S162BELVIESOU-001), institutional/social gates (S162BELVIESOU-003).
- Authoritative `can_exercise_control` / dispatch-time validation — unchanged.
- Adversarial end-to-end goldens (S162BELVIESOU-005).

## Acceptance Result

### Tests Passed

1. Added: `has_control_returns_false_for_non_self_live_control_source_without_belief`.
2. Revised: `can_control_returns_false_for_colocated_unowned_item_without_belief`.
3. Added: `remote_control_and_rights_changes_without_carrier_stay_invisible`.
4. Passed existing suite: `cargo test -p worldwake-sim`.

### Invariants

1. No belief-facing control/rights accessor reads `world.*` ownership/control/possession state outside a self or belief gate.
2. Authoritative dispatch enforcement (`can_exercise_control`) is unchanged.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` (`#[cfg(test)]`) — added the non-self `has_control` gate test, added the remote-change-invisible test, and revised the unowned-item test to prove the gated contract and removed leak.
2. `crates/worldwake-sim/src/affordance_query.rs` (`#[cfg(test)]`) — re-verified the existing self-control test without source changes.

## Outcome

Completed on 2026-05-21.

- `PerAgentBeliefView::has_control` no longer reads live control source for non-self entities.
- `believed_rights` and `can_control` no longer use live owner/possessor relations as access gates.
- `can_control` no longer treats co-located unowned items as controllable based on a live ownership-absence read.
- No `affordance_query.rs` source change was needed; the cited test already covers the actor's own control source.

## Verification Result

- Passed `cargo test -p worldwake-sim per_agent_belief_view`
- Passed `cargo test -p worldwake-sim affordance_query`
- Passed `cargo test -p worldwake-sim`
