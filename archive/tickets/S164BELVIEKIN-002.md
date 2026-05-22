# S164BELVIEKIN-002: `entity_kind` stored-belief source-gate

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `EntityBeliefView::entity_kind` in the per-agent belief view (sim)
**Deps**: archive/tickets/S164BELVIEKIN-001.md

## Problem

`EntityBeliefView::entity_kind` (`per_agent_belief_view.rs:604-609`) returns
`Place` for places (lawful public topology) but otherwise reads
`self.world.entity_kind(entity)` gated only by `knows_entity`. `knows_entity`
(`:300-317`) returns true for last-seen-only remote entities, so this returns the
**live world kind** for an entity the actor had not co-located-observed that tick
— the FND-14/FND-14B leak this ticket closed. The kind of a remote entity now comes
from stored belief (which can go stale), not live world.

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

## Verified Layers

1. Remote kind is belief-sourced → focused unit tests
   `entity_kind_uses_stored_belief_for_remote_entity_not_live_world` and
   `entity_kind_uses_last_seen_kind_for_remote_entity_without_belief_store_entry`
   prove remote authoritative kind divergence does not refresh the accessor.
2. Co-located / possessed kind stays live → focused unit test
   `entity_kind_keeps_live_read_for_co_located_entity_without_stored_belief` plus
   existing regression `entity_kind_returns_item_lot_for_self_possessed_unknown_item`.
3. Same-domain affordance fixture fallout → `worldwake-systems` focused tests and
   crate suite prove remote suspect/office affordances still work when fixtures seed
   explicit believed kind.
4. Single-layer ticket (belief-view accessor): no action-trace/event-log surface
   applies; the cross-system adversarial golden remains ticket 005's owner.

## Landed Changes

### 1. Restructured `entity_kind` into explicit lawful-source branches

`EntityBeliefView::entity_kind` now resolves:

1. topology places from public topology,
2. self, co-located, and directly possessed entities from live authoritative kind,
3. remote known entities from `BelievedEntityState.believed_kind` or
   `LastSeenRecord.observed_kind`.

The branch that handles remote known non-place entities no longer reads
`world.entity_kind`.

### 2. Added focused regression coverage

Added focused `per_agent_belief_view` tests for stored-belief remote kind,
last-seen remote kind, and co-located live kind. Updated the existing remote-hostile
fixture to seed the believed target kind explicitly.

### 3. Updated same-domain systems fixtures

The broad verification pass exposed justice and office affordance fixtures that
seeded remote known suspects/offices without a believed kind. Those fixtures now
populate `believed_kind` from the seeded known entity (or `EntityKind::Office` for
the no-place office-belief case) so the tests model the lawful remote target
belief they depend on.

## Landed Files

- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify — accessor + focused tests + remote-hostile fixture)
- `crates/worldwake-systems/src/justice_actions.rs` (modify — test fixture believed kind)
- `crates/worldwake-systems/src/office_actions.rs` (modify — test fixture believed kind)
- `archive/specs/S164-belief-view-kind-source-gate.md` (modified — deliverable 1 source-class truth-sync before spec archival)

## Out of Scope

- The last-seen synthesis and carrier field (ticket 001).
- The bandit faction-policy gate (ticket 003) and `facility_controller_at` (ticket 004).
- Any change to authoritative `world.entity_kind`, `knows_entity`, or
  `has_authoritative_local_visibility`.

## Acceptance Result

### Verified Acceptance

1. Remote known entities return stored kind from either the belief store or last-seen
   memory, not the current authoritative kind.
2. Co-located and directly possessed entities keep lawful live kind reads.
3. Existing same-domain remote hostile, justice, and office affordance fixtures pass
   after seeding the believed kind they depend on.
4. Existing broad verification passed via `./scripts/verify.sh`.

### Invariants

1. `entity_kind` never reads `world.entity_kind` for a non-place entity that is
   neither co-located with nor directly possessed by the actor.
2. The remote branch's only sources are `believed_entity().believed_kind` and the
   last-seen record's `observed_kind`.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — added focused tests for
   remote belief-store kind, remote last-seen kind, and co-located live kind.
2. `crates/worldwake-sim/src/per_agent_belief_view.rs` — updated the remote-hostile
   fixture to include believed target kind.
3. `crates/worldwake-systems/src/justice_actions.rs` and
   `crates/worldwake-systems/src/office_actions.rs` — updated remote known
   suspect/office fixtures to carry explicit believed kind.

## Outcome

Completed on 2026-05-22.

- Replaced the old `knows_entity`-gated live kind read with explicit public
  topology, self/local/possessed, and stored-belief branches.
- Added regression coverage for remote kind divergence and co-located live kind.
- Updated same-domain affordance fixtures that depended on remote suspect/office
  identity to seed explicit believed kind.
- Truthed the active S164 deliverable text to include the landed self and direct
  possession branches.

## Deviations

- The accessor keeps an explicit self-authoritative branch in addition to the
  drafted place/co-located/possessed/stored branches. This preserves lawful self
  state even if the actor is not currently represented as co-located at a place.
- `./scripts/verify.sh` initially failed in `worldwake-systems` because several
  remote known-entity fixtures omitted `believed_kind`. The fixtures were corrected
  as same-contract fallout, and the wrapper passed after the fix.

## Verification Result

1. Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::entity_kind_uses_stored_belief_for_remote_entity_not_live_world -- --exact`.
2. Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::entity_kind_uses_last_seen_kind_for_remote_entity_without_belief_store_entry -- --exact`.
3. Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::entity_kind_keeps_live_read_for_co_located_entity_without_stored_belief -- --exact`.
4. Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::entity_kind_returns_item_lot_for_self_possessed_unknown_item -- --exact`.
5. Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::effective_place_uses_last_seen_without_refreshing_remote_truth -- --exact`.
6. Passed `cargo test -p worldwake-sim --lib per_agent_belief_view::tests::current_place_entities_use_authoritative_local_set_over_stale_beliefs -- --exact`.
7. Passed `cargo test -p worldwake-sim --lib per_agent_belief_view`.
8. Passed `cargo test -p worldwake-sim`.
9. Passed `cargo test -p worldwake-systems --lib justice_actions::tests::accuse_affordance_emits_payload_for_known_remote_suspect_observation -- --exact`.
10. Passed `cargo test -p worldwake-systems --lib office_actions::tests::press_force_claim_affordance_surfaces_payload_for_local_eligible_force_office -- --exact`.
11. Passed `cargo test -p worldwake-systems --lib office_actions::tests::press_force_claim_affordance_uses_office_jurisdiction_not_believed_office_place -- --exact`.
12. Passed `cargo test -p worldwake-systems --lib office_actions::tests::press_force_claim_affordance_filters_nonlocal_and_duplicate_cases -- --exact`.
13. Passed `cargo test -p worldwake-systems`.
14. Passed `./scripts/verify.sh`.
15. Passed scoped diff hygiene over the touched source files and the then-active ticket/spec closeout edits after post-wrapper ticket/spec closeout edits. After archival, the ticket path is `archive/tickets/S164BELVIEKIN-002.md`; the spec path is now `archive/specs/S164-belief-view-kind-source-gate.md`.
