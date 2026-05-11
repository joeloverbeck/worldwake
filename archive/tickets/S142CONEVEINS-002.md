# S142CONEVEINS-002: Widen `BlockingFact::ReservationConflict` to carry `AffordanceKey` and `Option<EventId>`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-core` typed-blocker enum payload widening; `worldwake-ai` migration of runtime/test match sites plus local suppression-detail indirection; `worldwake-sim` current-format save/load proof; `worldwake-cli` observer fixture fallout
**Deps**: archive/tickets/S142CONEVEINS-001.md (provides `AffordanceKey`)

## Problem

S142's headline causal chain is "agent's `ReservationConflict` blocker → resolved by which contention event?". Before this ticket, the chain was broken: `BlockingFact::ReservationConflict` was a unit variant at `crates/worldwake-core/src/blocker_memory.rs:197` carrying no affordance identity and no reference to the resolving event. Without `affordance: AffordanceKey`, the AI could not disambiguate between two simultaneous reservation conflicts on different facilities. Without `contention_event: Option<EventId>`, decision traces could not point a debugger to the resolution that produced the loss. This ticket widened the variant from unit-form to struct-form and migrated the construction, destructuring, persistence, and fixture fallout.

## Assumption Reassessment (2026-05-10)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Pre-implementation, `BlockingFact::ReservationConflict` was a unit variant at `blocker_memory.rs:197`. Workspace-wide use sites totaled 17, all in `worldwake-ai`: 1 in `agent_tick/mod.rs:235` (exhaustive match arm), 16 in `failure_handling.rs` (split by the `#[cfg(test)]` boundary at `:1488` into 10 runtime sites — `:420`, `:525`, `:541`, `:736`, `:771`, `:838`, `:959`, `:1234`, `:1250`, `:1450` — and 6 test sites — `:2704`, `:2912`, `:2944`, `:3277`, `:3311`, `:3593`). Post-implementation, additional intentional references exist in `worldwake-core` tests, `worldwake-sim` save/load proof, and the `worldwake-cli` observer fixture.
2. The 7 inline test functions in `failure_handling.rs` that exercise `BlockingFact::ReservationConflict` construction and equality assertions are: `derive_clearing_condition_contention_blockers_capture_queue_baseline_when_available:2687`, `is_blocker_cleared_contention_changed:2869`, `derive_clearing_condition_reservation_conflict_uses_extraction_slot_position:2922`, `is_blocker_cleared_holds_when_position_decreases_but_slot_still_held:2960`, `is_blocker_cleared_when_actor_is_promoted_to_extraction_slot_grant:2982`, `is_blocker_cleared_when_actor_left_queue_and_slot_is_available:2998`, `is_blocker_cleared_holds_when_extraction_position_unchanged_and_no_slot_available:3019`. Each constructs the variant in unit form via either direct construction or as part of a `FailureClassification::Blocker(...)` wrap. All 7 must shift to the struct form.
3. The shared abstraction boundary under audit is the `BlockingFact` enum's typed-cause taxonomy. The widening preserves the existing variant count (no addition or removal of variants), changes only the shape of one variant from unit to struct.
4. Live correction: `BlockingFact` derives `Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize`; it does not derive `Ord`, `PartialOrd`, or `Hash`. `AffordanceKey` is `Copy` and `Option<EventId>` is `Copy` (`EventId` is a wrapper around `u64`). Therefore `BlockingFact` retains `Copy` after this widening.
5. Per `docs/precision-rules.md` Rule 12 (heuristic removal discipline): no heuristic is being removed here. The widening adds first-class identity to an existing typed cause, replacing implicit single-conflict-at-a-time disambiguation with explicit per-affordance routing.
6. Per Rule 16 (information-path refactors): the widening creates exactly one canonical path for `BlockingFact::ReservationConflict` to carry affordance identity. No alias path coexists. Ticket 005 will populate `contention_event` from the event-log lookup; ticket 002 only widens the surface.

## Architecture Check

1. Per FND-28, the unit form is *replaced*, not aliased. There is no `BlockingFact::LegacyReservationConflict` shim; the migration is one-shot. Ticket 001 already bumped `SAVE_FORMAT_VERSION` to 75, so pre-S142 save formats remain rejected at the header gate; this ticket only needs to preserve current-format round-trip behavior for the struct form.
2. The widening does not introduce a new enum variant — exhaustive match coverage cost is bounded to updating the existing arm patterns, not adding new arms. This minimizes the audit surface relative to introducing `BlockingFact::ReservationConflictV2`.
3. Some construction sites in `failure_handling.rs` may not have ready access to `(facility, action)` context — for those sites, the affordance must be threaded through the surrounding helper functions. The reassessment confirmed that the failure-handling code paths uniformly know the failed action's facility and action def, since those are required to classify a failure as a reservation conflict in the first place.

## Verification Layers

1. Variant shape change — focused unit test in `blocker_memory.rs` constructs the new struct variant and asserts bincode roundtrip.
2. Save-load current-format proof — `worldwake-sim` round-trip test extended with a current-format `BlockingFact::ReservationConflict { affordance, contention_event }` value. Do not add unit-form compatibility or a custom migration shim; older save formats are rejected by the version header after ticket 001.
3. Existing inline tests in `failure_handling.rs` updated to construct/match the struct form — focused runtime coverage already in place; the test names enumerated in Assumption Reassessment item 2 are the targets.
4. Single-layer ticket on the typed-blocker surface; no decision-trace, action-trace, or golden coverage is required here. Tickets 005 and 007 cover downstream propagation and end-to-end attribution.

## What to Change

### 1. Widen the variant in `blocker_memory.rs:197`

```rust
pub enum BlockingFact {
    // ... existing variants ...
    ReservationConflict {
        affordance: AffordanceKey,
        contention_event: Option<EventId>,
    },
    // ... existing variants ...
}
```

No `#[serde(default)]` compatibility path is added for the removed unit form. Ticket 001's version bump makes this a current-format-only shape change.

### 2. Migrate the 11 runtime construction/destructuring sites in `worldwake-ai`

For each construction site (`failure_handling.rs:525`, `:541`, `:736`, `:771`, `:838`, `:959`, `:1234`, `:1250`, `:1450`, `:2704` — and `agent_tick/mod.rs:235` for the exhaustive match arm), replace the unit form with the struct form. Each site has access to the failed action's facility ID and action def (verified during reassessment); compose `affordance: AffordanceKey { facility, action }` at the call site. `contention_event` defaults to `None` here — ticket 005 will populate it via event-log lookup.

For destructuring/match sites (`failure_handling.rs:420`, `:959`, `:1234`, `:1250`, `:1450`, `:2912`, `:2944`), update the pattern to bind the new fields (e.g., `BlockingFact::ReservationConflict { affordance, contention_event }` or `BlockingFact::ReservationConflict { .. }` where the bound fields are unused).

### 3. Migrate the 6 test sites in `failure_handling.rs`

Update tests at lines 2704, 2912, 2944, 3277, 3311, 3593 to construct the new struct form. The 7 named test functions in Assumption Reassessment item 2 must pass after migration. Where a test asserts `assert_eq!(fact, BlockingFact::ReservationConflict)` on the unit form, replace with the struct form using a known-good affordance fixture (e.g., a constructed `AffordanceKey` matching the test's setup).

### 4. Extend save-load round-trip test

In `crates/worldwake-sim/src/save_load.rs` `#[cfg(test)]` block, add or extend a current-format round-trip test that carries `BlockingFact::ReservationConflict { affordance, contention_event }` with non-default values and verifies both fields survive save/load.

## Files to Touch

- `crates/worldwake-core/src/blocker_memory.rs` (modify — variant shape)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — exhaustive match arm at line 235)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — box suppression blocker detail to satisfy `clippy::large_enum_variant` after payload widening)
- `crates/worldwake-ai/src/failure_handling.rs` (modify — 16 sites: 10 runtime + 6 test)
- `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` (modify — observer fixture fallout from rendered struct form)
- `crates/worldwake-sim/src/save_load.rs` (modify — extend round-trip test)

## Out of Scope

- Adding `EventTag::ContentionResolved` (ticket 001)
- Defining `AffordanceKey` (ticket 001)
- Bumping `SAVE_FORMAT_VERSION` (ticket 001)
- Emitting `ContentionResolved` events (tickets 003, 004)
- Populating `BlockingFact::ReservationConflict.contention_event` from event-log lookup (ticket 005)
- Observer rendering (ticket 006)
- End-to-end goldens (ticket 007)

## Acceptance Criteria

### Tests That Must Pass

1. The named inline tests in `failure_handling.rs` (per Assumption Reassessment item 2) pass after migration to struct form.
2. New focused test: a `BlockingFact::ReservationConflict { affordance, contention_event }` value roundtrips through bincode serialization with both fields preserved.
3. New focused/current-format save-load proof preserves a struct-form `ReservationConflict` with non-default `affordance` and `contention_event`.
4. Existing suite: `cargo test -p worldwake-ai`, `cargo test -p worldwake-core`, `cargo test -p worldwake-sim`, and `cargo test --workspace`.

### Invariants

1. `BlockingFact` retains `Copy` (`AffordanceKey` is `Copy`, `Option<EventId>` is `Copy`).
2. Production construction/destructuring sites are migrated to struct form. Post-implementation references outside `worldwake-ai` are intentional proof/fixture sites in `worldwake-core`, `worldwake-sim`, and `worldwake-cli`, verified by `rg -n "BlockingFact::ReservationConflict|ReservationConflict" crates/`.
3. The variant count of `BlockingFact` is unchanged (the migration widens an existing variant; no addition or removal).
4. Per FND-28: no parallel "legacy" variant or alias coexists with the struct form.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/failure_handling.rs` (existing `#[cfg(test)]` block) — update named test functions per Assumption Reassessment item 2.
2. `crates/worldwake-core/src/blocker_memory.rs` — add focused struct-form bincode roundtrip test.
3. `crates/worldwake-sim/src/save_load.rs` (existing `#[cfg(test)]` block) — add current-format struct-form round-trip test.

### Commands

1. `cargo test -p worldwake-ai failure_handling`
2. `cargo test -p worldwake-core --lib blocker_memory::tests::reservation_conflict_blocking_fact_roundtrips_with_affordance_and_event -- --exact`
3. `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_decision_event_payloads -- --exact`
4. `cargo test -p worldwake-cli --test observer_decision_history`
5. `cargo test -p worldwake-ai`
6. `cargo test -p worldwake-core`
7. `cargo test -p worldwake-sim`
8. `cargo test --workspace`
9. `cargo clippy --workspace --all-targets -- -D warnings`
10. `git diff --check`

## Outcome

Completed on 2026-05-11. `BlockingFact::ReservationConflict` is now:

```rust
ReservationConflict {
    affordance: AffordanceKey,
    contention_event: Option<EventId>,
}
```

The old unit form was replaced without an alias or compatibility shim. Runtime construction in `failure_handling.rs` now records `affordance` and defaults `contention_event` to `None`; ticket 005 remains the owner for looking up and populating the event id. Current-format persistence proof uses a non-`None` `contention_event`, and the observer decision-history fixture was updated for the new rendered struct form.

## Verification Result

Passed:

1. `cargo test -p worldwake-core --lib blocker_memory::tests::reservation_conflict_blocking_fact_roundtrips_with_affordance_and_event -- --exact`
2. `cargo test -p worldwake-ai --lib failure_handling`
3. `cargo test -p worldwake-sim --lib save_load::tests::save_to_bytes_roundtrip_preserves_decision_event_payloads -- --exact`
4. `cargo test -p worldwake-ai`
5. `cargo test -p worldwake-core`
6. `cargo test -p worldwake-sim`
7. `cargo test -p worldwake-cli --test observer_decision_history`
8. `cargo test --workspace`
9. `cargo clippy --workspace --all-targets -- -D warnings`
