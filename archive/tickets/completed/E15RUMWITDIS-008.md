# E15RUMWITDIS-008: Passive Observation Discovery Events For Belief Mismatches

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes - perception-system mismatch detection and Discovery event emission for passive same-place observation
**Deps**: `archive/tickets/E15RUMWITDIS-002.md`, `archive/tickets/completed/E15RUMWITDIS-004.md`, `specs/E15-rumor-witness-discovery.md`, `specs/IMPLEMENTATION-ORDER.md`

## Problem

Passive same-place perception currently refreshes belief snapshots, but it still overwrites prior beliefs without emitting a Discovery event when the new observation violates an existing expectation. That leaves Principle 15 unimplemented on the passive observation path even though the core event/evidence types already exist.

## Assumption Reassessment (2026-03-14)

1. `observe_passive_local_entities()` in `crates/worldwake-systems/src/perception.rs` is the passive same-place path and still calls `store.update_entity(entity, snapshot)` without comparing against an existing belief first.
2. `AgentBeliefStore::get_entity()` already exists in `crates/worldwake-core/src/belief.rs`, so the prior belief can be read before overwrite without new storage APIs.
3. `EventTag::Discovery`, `MismatchKind`, `EvidenceRef::Mismatch`, `ActionDomain::Social`, `TellProfile`, and `SocialObservationKind::WitnessedTelling` are already implemented. This ticket must not reopen that scope.
4. `E15RUMWITDIS-004` is already completed and archived at `archive/tickets/completed/E15RUMWITDIS-004.md`; the original dependency path in this ticket was stale.
5. Passive same-place perception only observes entities actually present at the observer's current place. That means `AliveStatusChanged` and `InventoryDiscrepancy` are the material mismatches this ticket can detect honestly. `PlaceChanged` and `EntityMissing` require other paths and remain `E15RUMWITDIS-009`.
6. The current perception system batches belief-store writes through one hidden `WorldTxn` commit at the end of the system tick. Discovery events should remain separate append-only event-log records rather than being folded into that hidden belief-store mutation event.

## Architecture Check

1. Emitting Discovery from perception is beneficial relative to the current architecture because it keeps violated expectations in the append-only causal record instead of burying them inside an overwritten belief snapshot. That makes downstream investigation, office succession awareness, and debugability possible without coupling systems directly.
2. The clean boundary is: compare `prior` vs `new snapshot`, emit discovery records for concrete mismatches, then let the normal belief overwrite proceed unchanged. Discovery is aftermath, not a special belief type.
3. This ticket should introduce a small private helper for mismatch detection and event emission in `perception.rs` if that keeps `E15RUMWITDIS-009` from duplicating logic. A helper is justified here because the adjacent ticket explicitly extends the same mechanism to event-based perception and `EntityMissing`.
4. Passive same-place perception should not fabricate `PlaceChanged` discoveries. If an entity is seen at the observer's place, the observer has not perceived an alternate place transition; any place mismatch belongs to the event-based path.
5. No backwards-compatibility shims, alias behavior, or secondary evidence transport.

## What To Change

### 1. Compare prior and new passive snapshots before overwrite

In `crates/worldwake-systems/src/perception.rs`, before each `store.update_entity(entity, snapshot)` inside `observe_passive_local_entities()`:

- read the existing belief with `store.get_entity(&entity)`
- compare it with the new snapshot
- emit one Discovery event per material mismatch
- then update the belief normally

### 2. Emit Discovery only for passive mismatches this path can know

For passive same-place observation, emit Discovery for:

- `MismatchKind::AliveStatusChanged`
- `MismatchKind::InventoryDiscrepancy { commodity, believed, observed }`

Do not emit Discovery for:

- first observation with no prior belief
- exact matches
- `MismatchKind::PlaceChanged`
- `MismatchKind::EntityMissing`

### 3. Keep discovery emission append-only and observer-local

Each emitted Discovery event must:

- use `actor_id = Some(observer)`
- use `place_id = Some(observer_place)`
- carry `EvidenceRef::Mismatch { observer, subject, kind }`
- use `VisibilitySpec::ParticipantsOnly`
- tag `{EventTag::Discovery, EventTag::WorldMutation}`

The event should be emitted directly to `event_log`, not encoded as part of the hidden belief-store transaction.

### 4. Keep the implementation reusable for `E15RUMWITDIS-009`

If the code becomes cleaner by extracting a private helper such as `detect_passive_mismatches()` or `emit_discovery_event()`, do it now. The helper should stay private to `perception.rs` and model concrete mismatches only; `-009` can extend the same mechanism for event-based updates and `EntityMissing`.

## Files To Touch

- `tickets/E15RUMWITDIS-008.md` (modify first - correct assumptions and scope)
- `crates/worldwake-systems/src/perception.rs` (modify - passive mismatch detection and Discovery emission)

## Out Of Scope

- `MismatchKind`, `EvidenceRef::Mismatch`, `EventTag::Discovery`, `TellProfile`, `ActionDomain::Social`, or `WitnessedTelling` definitions
- event-based perception mismatch detection
- `MismatchKind::EntityMissing`
- `MismatchKind::PlaceChanged`
- Tell action behavior
- planner/investigation goal generation
- `belief_confidence()` derivation helper

## Acceptance Criteria

### Tests That Must Pass

1. Passive same-place observation emits a Discovery event for `AliveStatusChanged` when the observer had a prior belief and now sees the subject dead.
2. Passive same-place observation emits a Discovery event for `InventoryDiscrepancy` when the observer had a prior inventory belief and now sees a different quantity.
3. First observation with no prior belief emits no Discovery event.
4. Matching prior belief emits no Discovery event.
5. Discovery events from this path use `VisibilitySpec::ParticipantsOnly`.
6. Discovery events from this path are tagged with `EventTag::Discovery` and `EventTag::WorldMutation`.
7. Discovery events from this path carry the correct `EvidenceRef::Mismatch`.
8. Multiple commodity mismatches on one entity emit one event per commodity mismatch.
9. Existing perception tests continue to pass.
10. `cargo test -p worldwake-systems perception`
11. `cargo clippy --workspace --all-targets -- -D warnings`
12. `cargo test --workspace`

### Invariants

1. Discovery emission must not change the resulting belief snapshot; beliefs still update through the normal store path.
2. Discovery is emitted only when a prior belief exists and the new passive observation materially contradicts it.
3. Passive same-place perception does not claim knowledge it does not have: no `PlaceChanged`, no `EntityMissing`.
4. Discovery remains append-only event-log state, not a hidden field on belief records.
5. Any helper introduced here must remain reusable for `E15RUMWITDIS-009` rather than creating passive-only duplication.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/perception.rs` - passive observation emits `AliveStatusChanged` Discovery when prior belief expected the subject alive
2. `crates/worldwake-systems/src/perception.rs` - passive observation emits `InventoryDiscrepancy` Discovery when observed inventory differs from prior belief
3. `crates/worldwake-systems/src/perception.rs` - passive observation with no prior belief emits no Discovery
4. `crates/worldwake-systems/src/perception.rs` - passive observation with matching prior belief emits no Discovery

### Commands

1. `cargo test -p worldwake-systems perception`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`

## Outcome

- Completion date: 2026-03-14
- What actually changed:
  - Corrected the ticket first so it matched the live E15 architecture: archived dependency paths, already-landed E15 primitives removed from scope, and passive same-place mismatch detection narrowed to the mismatches this path can honestly know.
  - Added passive observation Discovery emission in `crates/worldwake-systems/src/perception.rs` for `AliveStatusChanged` and per-commodity `InventoryDiscrepancy`.
  - Kept the implementation architecture clean by separating mismatch comparison from belief overwrite and by emitting Discovery as append-only event-log state rather than embedding it in belief data.
  - Added perception regression tests covering alive-status mismatch, inventory mismatch, no-prior-belief, and matching-belief cases.
  - Cleared repo-level lint blockers required for final verification by fixing one new clippy issue in `perception.rs` and pre-existing test-only clippy complaints in `crates/worldwake-systems/src/tell_actions.rs`.
- Deviations from original plan:
  - The original ticket claimed broader pending scope than was still real. `Discovery`, `MismatchKind`, `EvidenceRef::Mismatch`, `TellProfile`, `ActionDomain::Social`, and `WitnessedTelling` were already implemented, so the ticket was narrowed before code changes.
  - `PlaceChanged` was explicitly kept out of this ticket because passive same-place observation cannot perceive that mismatch honestly; it remains with the event-based follow-up in `E15RUMWITDIS-009`.
- Verification results:
  - `cargo test -p worldwake-systems perception`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
