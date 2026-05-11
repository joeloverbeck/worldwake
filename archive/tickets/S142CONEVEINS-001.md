# S142CONEVEINS-001: Add `EventTag::ContentionResolved` and `ContentionEventPayload` foundation types

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` event-tag taxonomy, new core-resident payload module, save-format version bump
**Deps**: None

## Problem

`docs/FOUNDATIONS.md` FND-9 mandates that scheduling, simultaneity, and tie-breaking are part of the world model: tick order and container iteration order may not silently decide who saw the dropped coin first. Today the contention substrate (`crates/worldwake-core/src/contention.rs`) carries the *state* of contention (`ContentionQueue`, `ContentionGrant`, `ContentionWaiter`) but emits no event recording the *resolution moment* — the tick at which a winner was selected, the rule that fired, and the loser set. Spec S142 closes this gap by adding `EventTag::ContentionResolved` plus a typed payload. This ticket lands the type substrate and the helper that builds the payload from a pre-mutation queue snapshot, so subsequent emission tickets (003, 004) can call into a single helper rather than duplicating snapshot-and-build logic.

## Assumption Reassessment (2026-05-10)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `EventTag` enum lives at `crates/worldwake-core/src/event_tag.rs:7` and currently has 45 variants (verified by `ALL_EVENT_TAGS` listing at lines 66–110). `ContentionResolved` is not among them. `ALL_EVENT_TAGS` test asserts the variant count. The new variant must be added to both the enum body and `ALL_EVENT_TAGS`, and the test count assertion updated.
2. `ContentionQueue` (`contention.rs:10`), `ContentionGrant` (`:43`), `ContentionWaiter` (`:35`), and `ResourceExtractionQueues` (`:27`) all live in `worldwake-core`. `ContentionWaiter` carries `actor: EntityId`, `intended_action: ActionDefId`, `queued_at: Tick`. The waiter has no `queue_position` field — the position the spec proposes for `ContentionClaimant.queue_position` is derived from the `BTreeMap<u32, ContentionWaiter>` ordinal in `ContentionQueue.waiting` at emission time. The helper produced by this ticket reads ordinals from a snapshot taken before any mutation.
3. `SAVE_FORMAT_VERSION` was `74` at `crates/worldwake-sim/src/save_load.rs:6` during intake. Spec S142 calls for one increment to `75`. The bump must land in this ticket because `EventTag::ContentionResolved` enters the canonical event-tag taxonomy with the variant addition; later tickets emit events under that tag.
4. `worldwake-core`'s current external dependencies are `serde`, `bincode`, `blake3`. The reassessment confirmed S142 must NOT add `smallvec` — `ContentionEventPayload` uses `Vec<ContentionClaimant>` with runtime truncation to 8 + a `total_claimants: u16` overflow counter.
5. The shared abstraction boundary under audit is the event-log writer surface (`EventLog::events_by_tag` at `event_log.rs:124`) plus the typed payload carrier chain (`EventPayload`, `EventView`, `WorldTxn`). `ContentionResolved` must round-trip through serialize/deserialize and indexing identically to existing variants, and the payload must be persisted on the event record rather than existing only as a standalone helper type.

## Architecture Check

1. Per FND-3, the resolution rule is a typed enum (`ContentionResolutionRule::ArrivalTime` is the single live variant); the spec's substantial-redesign reassessment dropped speculative future variants whose substrate doesn't exist. Future variants land with their substrate per FND-28.
2. Per FND-28, no shim coexists with this addition: the new payload types and event variant are net-new; nothing pre-existing is being preserved alongside them.
3. The payload-builder helper centralizes snapshot-before-mutate ordering. Both downstream emission sites (facility-queue at `facility_queue.rs::promote_ready_head` and resource-extraction at `production_actions.rs::grant_or_signal_full`) call into the helper with a pre-mutation snapshot of `ContentionQueue.waiting`. This avoids duplicating the snapshot-derive logic and keeps the per-claimant `queue_position` derivation honest about its data source (BTreeMap ordinals at call time).

## Verification Layers

1. Helper builds correct payload from snapshot — focused unit test in `contention_event.rs`
2. `Vec<ContentionClaimant>` truncation policy (head-8 + `total_claimants: u16` overflow) — focused unit test
3. `queue_position` derivation from BTreeMap ordinal — focused unit test
4. `ContentionResolved` event round-trip through save/load (canonical-state hash compatibility) — `save_load.rs` round-trip test extended for the new tag and the bumped version
5. Single-layer ticket: no AI/runtime/golden coverage required here. Emission sites (003, 004) and end-to-end goldens (007) cover downstream verification.

## What to Change

### 1. New module `crates/worldwake-core/src/contention_event.rs`

Define the five types plus the payload-builder helper:

```rust
pub struct AffordanceKey {
    pub facility: EntityId,
    pub action: ActionDefId,
}

pub struct ContentionEventPayload {
    pub contested_affordance: AffordanceKey,
    pub place: EntityId,
    pub resolution_rule: ContentionResolutionRule,
    pub claimants: Vec<ContentionClaimant>,
    pub total_claimants: u16,
    pub winner: Option<EntityId>,
    pub at_tick: Tick,
}

pub struct ContentionClaimant {
    pub agent: EntityId,
    pub arrived_tick: Tick,
    pub queue_position: u16,
    pub outcome: ClaimantOutcome,
}

pub enum ClaimantOutcome {
    Granted,
    QueuedAhead,
    QueuedBehind,
    Denied { reason: DenialReason },
}

pub enum ContentionResolutionRule {
    ArrivalTime,
}
```

Derives: `AffordanceKey` derives `Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize`. `ContentionEventPayload` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize` (the `Vec<ContentionClaimant>` member precludes `Copy`). `ContentionResolutionRule`, `ClaimantOutcome`, and `ContentionClaimant` derive `Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize` (apart from `ContentionClaimant`, which is a pure struct — no `Hash` requirement, but include if convenient). `DenialReason` is a new typed enum local to this module; for the ArrivalTime-only resolution today, add a single variant `DenialReason::QueueFull` and document that future denial reasons land with their substrate.

### 2. Payload-builder helper

```rust
pub fn build_contention_event_payload(
    queue_snapshot: &ContentionQueue,   // PRE-MUTATION snapshot
    facility: EntityId,
    place: EntityId,
    action: ActionDefId,
    rule: ContentionResolutionRule,
    granted_actor: Option<EntityId>,
    tick: Tick,
) -> ContentionEventPayload
```

Reads `queue_snapshot.waiting` ordinals, classifies each claimant relative to `granted_actor`:
- `Granted` for the actor matching `granted_actor`
- `QueuedAhead` for claimants whose ordinal precedes the granted actor's ordinal (or all claimants when `granted_actor` is `None`'s "queue-only shift" case)
- `QueuedBehind` for claimants whose ordinal follows the granted actor's ordinal

Truncates `claimants` to head-8 by ordinal; `total_claimants` carries the full waiting count (including the granted head when present).

Placeholder, replaced by tickets 003 and 004: this helper has no callers in this ticket. The two emission tickets call it from the appropriate snapshot points.

### 3. Extend `EventTag` and `ALL_EVENT_TAGS`

Add `ContentionResolved` to the enum at `crates/worldwake-core/src/event_tag.rs:7` (placed in the queue-domain group alongside `QueueGrantExpired`, `QueueHeadFailed`, `QueueGrantPromoted` at lines 83–85) and to the `ALL_EVENT_TAGS` slice. Update the existing test asserting the variant count.

### 4. Add the typed event payload carrier

Add `contention_event_payload: Option<ContentionEventPayload>` to `EventPayload`, expose it through `EventView::contention_event_payload`, and add `WorldTxn::set_contention_event_payload`. The setter inserts `EventTag::ContentionResolved`, so downstream emission tickets can write the typed payload through the existing transaction-to-event-log path.

### 5. Re-export from `lib.rs`

Add `pub mod contention_event;` and re-export the public types from `crates/worldwake-core/src/lib.rs`.

### 6. Bump `SAVE_FORMAT_VERSION` 74 → 75

`crates/worldwake-sim/src/save_load.rs:6` — increment the constant. Update the round-trip test asserting version compatibility (line 1136). Add a save/load round-trip case with a `ContentionResolved` event carrying a non-default `ContentionEventPayload`. Per the repo's no-backwards-compatibility rule, pre-S142 save versions remain rejected at the header gate.

## Files to Touch

- `crates/worldwake-core/src/contention_event.rs` (new)
- `crates/worldwake-core/src/event_tag.rs` (modify — add variant, add to `ALL_EVENT_TAGS`, update count test)
- `crates/worldwake-core/src/event_record.rs` (modify — add typed payload carrier and `EventView` accessor)
- `crates/worldwake-core/src/world_txn.rs` (modify — add typed payload setter)
- `crates/worldwake-core/src/lib.rs` (modify — `pub mod contention_event;` and re-exports)
- `crates/worldwake-sim/src/save_load.rs` (modify — bump constant, update version-roundtrip test)

## Out of Scope

- Emission of `ContentionResolved` from any code path (covered by tickets 003 and 004)
- `BlockingFact::ReservationConflict` payload widening (covered by ticket 002)
- AI population of `contention_event` (covered by ticket 005)
- Observer rendering (covered by ticket 006)
- End-to-end goldens (covered by ticket 007)
- New `EvidenceRef` / `ContentionEventRef` aliases — the spec's reassessment confirmed `EventId` is used directly without an alias

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit test: `build_contention_event_payload` constructs the payload with claimants in BTreeMap ordinal order, `Granted` flag on the actor matching `granted_actor`, `QueuedAhead` on earlier ordinals, `QueuedBehind` on later ordinals.
2. New focused unit test: queue with 12 waiters truncates to head-8 in `claimants` and sets `total_claimants = 12`.
3. New focused unit test: `granted_actor: None` returns winner=None and all claimants flagged `QueuedAhead` (queue-only shift).
4. Updated `event_tag.rs` count test passes with the new variant counted.
5. Updated `save_load.rs` round-trip test: a save written under version 75 with a `ContentionResolved` event roundtrips identically.
6. Existing suite: `cargo test -p worldwake-core` and `cargo test -p worldwake-sim`.

### Invariants

1. `ContentionEventPayload` does not depend on `smallvec` — uses `Vec<ContentionClaimant>` with runtime truncation only.
2. `AffordanceKey` derives `Copy` (both fields are `Copy`); compositions on `Copy`-deriving enums (e.g., `BlockingFact` in ticket 002) remain `Copy`-compatible.
3. `ContentionResolutionRule` has exactly one variant (`ArrivalTime`); future variants land with their substrate per FND-28.
4. `SAVE_FORMAT_VERSION` is `75` after this ticket lands.
5. `worldwake-core/Cargo.toml` external deps remain {`serde`, `bincode`, `blake3`} — no new external dependency added.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/contention_event.rs` (new file, inline `#[cfg(test)]` block) — 4–6 focused unit tests covering helper semantics, truncation, ordinal derivation.
2. `crates/worldwake-core/src/event_tag.rs` (existing `#[cfg(test)]` block) — extend variant-count test.
3. `crates/worldwake-sim/src/save_load.rs` (existing `#[cfg(test)]` block) — extend round-trip test for the new tag at version 75.

### Commands

1. `cargo test -p worldwake-core contention_event`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-10.

- Added `crates/worldwake-core/src/contention_event.rs` with `AffordanceKey`, `ContentionEventPayload`, claimant/outcome/rule enums, `DenialReason`, and `build_contention_event_payload`.
- Added `EventTag::ContentionResolved`, kept `ALL_EVENT_TAGS` declaration order stable, and re-exported the new public types from `worldwake-core`.
- Added the typed event carrier chain: `EventPayload.contention_event_payload`, `EventView::contention_event_payload`, and `WorldTxn::set_contention_event_payload`.
- Bumped `SAVE_FORMAT_VERSION` from 74 to 75 and added save/load proof for a non-default `ContentionResolved` payload.

## Deviations

- Reassessment widened the ticket from a standalone payload type to the full event payload carrier chain. Without the `EventPayload`/`EventView`/`WorldTxn` slot, later tickets could tag events but could not persist or inspect the typed payload.
- Existing `EventPayload` literals across workspace tests and helper emitters were updated with `contention_event_payload: None` as shared-shape constructor fallout. No production emitter writes `ContentionResolved` yet; tickets 003 and 004 still own runtime emission.
- No backwards-compatibility shim was added for older save versions; the version bump keeps the current-format-only policy intact.

## Verification Result

- Passed `cargo test -p worldwake-core --lib contention_event -- --list`
- Passed `cargo test -p worldwake-core --lib contention_event`
- Passed `cargo test -p worldwake-core --lib event_tag -- --list`
- Passed `cargo test -p worldwake-core --lib event_tag`
- Passed `cargo test -p worldwake-sim --lib save_to_bytes_roundtrip_preserves_contention_event_payloads -- --list`
- Passed `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_contention_event_payloads -- --exact`
- Passed `cargo test --workspace --no-run`
- Passed `cargo fmt --all`
- Passed `cargo test -p worldwake-core`
- Passed `cargo test -p worldwake-sim`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
