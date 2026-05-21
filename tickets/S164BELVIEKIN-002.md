# S164BELVIEKIN-002: `entity_kind` stored-belief source-gate

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `EntityBeliefView::entity_kind` in the per-agent belief view (sim)
**Deps**: archive/tickets/S164BELVIEKIN-001.md

## Problem

`EntityBeliefView::entity_kind` (`per_agent_belief_view.rs:604-609`) returns
`Place` for places (lawful public topology) but otherwise reads
`self.world.entity_kind(entity)` gated only by `knows_entity`. `knows_entity`
(`:300-317`) returns true for last-seen-only remote entities, so this returns the
**current world kind** for an entity the actor has not co-located-observed this tick
— an FND-14/FND-14B leak. The kind of a remote entity must come from stored belief
(which can go stale), not live world.

## Assumption Reassessment (2026-05-22)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `entity_kind` (`crates/worldwake-sim/src/per_agent_belief_view.rs:604-609`) today
   has only two branches: `Place` → `Place`, and a single `knows_entity`-gated live
   read that fires for co-located **and** remote known entities alike. There is no
   separate co-located branch to "keep" — the fix must create the branch split.
2. `knows_entity` (`:300-317`) admits an entity via self, co-location
   (`has_authoritative_local_visibility`, `:293`), **direct possession**
   (`possessor_of(entity) == Some(self.agent)`), belief-store presence, institutional
   belief subject, or last-seen memory. `BelievedEntityState.believed_kind`
   (`crates/worldwake-core/src/belief.rs:1735`) already carries the stored kind for
   belief-store entities; ticket 001 adds `LastSeenRecord.observed_kind` for
   last-seen-only entities (which `believed_entity`, `:268`, does not cover).
3. Boundary under audit: the `EntityBeliefView::entity_kind` accessor source class.
   Source classes after fix: public topology (Place), same-tick local physical
   (co-located / directly possessed, FND-14A), belief-backed (remote known).
4. Mismatch + correction (adjacent contradiction — required consequence): the spec's
   D1 enumerated only Place + co-located + stored, but the existing inline test
   `entity_kind_returns_item_lot_for_self_possessed_unknown_item`
   (`per_agent_belief_view.rs:5650`) asserts a live kind read for a **self-possessed,
   not-in-belief-store, non-co-located** item. `knows_entity` lists possession
   separately from co-location, so a 3-branch restructure would regress this test.
   The fix must include a fourth branch: direct possession → live kind (FND-14A
   direct-possession sibling). This preserves the deliverable's contract (remote →
   stored) — possession is a lawful same-tick local observation, not a remote read.
5. Other inline tests exercising `entity_kind`/last-seen behavior to re-validate:
   `effective_place_uses_last_seen_without_refreshing_remote_truth` (`:2948`),
   `current_place_entities_use_authoritative_local_set_over_stale_beliefs` (`:2695`).
6. Intended layer is belief-view accessor logic (not authoritative world validation);
   verification is focused-unit on the accessor plus the cross-system golden in
   ticket 005.

## Architecture Check

1. Splitting the undifferentiated `knows_entity` gate into explicit lawful-source
   branches makes the accessor's source class legible per the belief-view
   source-class rule: each branch corresponds to exactly one lawful surface (public
   topology / same-tick local physical / belief-backed). The remote branch can never
   read live world.
2. No backward-compat shim: the live read for remote entities is removed, not aliased.
   The co-located and possession branches retain the lawful FND-14A live read.

## Verification Layers

1. Remote kind is belief-sourced → focused unit test: a remote known entity whose
   authoritative kind changes with no carrier keeps its stored `believed_kind` (or
   `None` if never stored).
2. Co-located / possessed kind stays live → focused unit test guarded by existing
   `entity_kind_returns_item_lot_for_self_possessed_unknown_item` (`:5650`) and a new
   co-located case.
3. Single-layer ticket (belief-view accessor): no action-trace/event-log surface
   applies; the authoritative world-state divergence is asserted in the ticket 005
   golden, not here.

## What to Change

### 1. Restructure `entity_kind` into four explicit branches

Replace the body of `entity_kind` (`:604-609`) with:

1. `self.world.entity_kind(entity) == Some(EntityKind::Place)` → `Some(Place)`
   (public topology).
2. `self.has_authoritative_local_visibility(entity)` → live `world.entity_kind`
   (FND-14A co-located physical).
3. `self.world.possessor_of(entity) == Some(self.agent)` → live `world.entity_kind`
   (FND-14A direct possession).
4. otherwise (remote known) → stored kind: `believed_entity(entity).believed_kind`
   if present, else the actor's last-seen record's `observed_kind`
   (`get_component_last_seen_memory(self.agent).records[entity].observed_kind`), else
   `None`. Never read `world.entity_kind` on this branch.

## Files to Touch

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — `entity_kind` `:604-609` + tests)

## Out of Scope

- The last-seen synthesis and carrier field (ticket 001).
- The bandit faction-policy gate (ticket 003) and `facility_controller_at` (ticket 004).
- Any change to authoritative `world.entity_kind`, `knows_entity`, or
  `has_authoritative_local_visibility`.

## Acceptance Criteria

### Tests That Must Pass

1. A remote known entity whose authoritative kind changes with no carrier returns the
   stored kind (or `None`), never the new live kind.
2. A co-located entity returns its live kind (FND-14A).
3. `entity_kind_returns_item_lot_for_self_possessed_unknown_item` (`:5650`) still
   passes — self-possessed unknown item returns its live kind via the possession
   branch.
4. Existing suite: `cargo test -p worldwake-sim`.

### Invariants

1. `entity_kind` never reads `world.entity_kind` for a non-place entity that is
   neither co-located with nor directly possessed by the actor.
2. The remote branch's only sources are `believed_entity().believed_kind` and the
   last-seen record's `observed_kind`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — new focused tests:
   remote-kind-change keeps stored kind; co-located returns live kind. Re-validate
   `:5650`, `:2948`, `:2695`.

### Commands

1. `cargo test -p worldwake-sim per_agent_belief_view`
2. `cargo test -p worldwake-sim`
3. `./scripts/verify.sh`
