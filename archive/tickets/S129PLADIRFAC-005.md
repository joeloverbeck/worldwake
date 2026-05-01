# S129PLADIRFAC-005: relieve_wilderness writes PlaceDirtiness

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `relieve_wilderness` commit handler additionally mutates per-place `PlaceDirtiness` and emits `EventTag::WasteCreated` with `WasteSource::WildernessRelief`
**Deps**: archive/tickets/S129PLADIRFAC-001.md, archive/tickets/S129PLADIRFAC-002.md

## Problem

Today's `relieve_wilderness` commit handler (`crates/worldwake-systems/src/needs_actions.rs:648–691`) creates a Waste `ItemLot` at the place, increments the actor's per-agent `dirtiness`, and emits a `DisturbanceMarker` evidence — but the place itself does not develop any property that other agents perceive. The S129 narrative-report evidence (Agent C relieving 26 times at Fertile Fields without consequence) traces directly to this gap. This ticket extends the handler to write `PlaceDirtiness` on the actor's place and emit `WasteCreated` so the consequence chain becomes legible (FND-29).

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `relieve_wilderness` commit handler at `crates/worldwake-systems/src/needs_actions.rs:648–691`. Existing behavior verified during reassessment: gets actor's place via `txn.effective_place(instance.actor)` (line 658); creates Waste lot via `txn.create_item_lot(CommodityKind::Waste, Quantity(1))` + `txn.set_ground_location(...)` (lines 661–665); emits `DisturbanceMarker` evidence (lines 667–677); increments agent dirtiness via `needs.dirtiness.saturating_add(profile.wilderness_relief_dirtiness_penalty)` (line 686). Existing inline tests: `relieve_wilderness_commit_effects` (line 1774), `relieve_wilderness_visibility_is_same_place` (1808), `relieve_wilderness_has_wilderness_relief_event_tag` (1815), `relieve_wilderness_commit_emits_scene_evidence` (1825).
2. `PlaceDirtiness` component (from ticket 001) carries `value: Permille`, `decay_per_tick: Permille`, `dirtiness_per_use: Permille` per spec D1. Universal-on-Place — every place implicitly has one with `Default` values; no missing-component path.
3. `MetabolismProfile.wilderness_relief_dirtiness_penalty: Permille` at `crates/worldwake-core/src/needs.rs:177` is the per-agent dirtiness contribution for the *agent's* hygiene only — it remains unchanged by this ticket. Spec D5 + reassessment finding M3 explicitly use **per-place** `PlaceDirtiness.dirtiness_per_use` (not the per-agent penalty) for the place-side increment, so per-agent variation in cleanliness tolerance does not unphysically alter how dirty a relief makes the place.
4. The shared abstraction boundary under audit is the commit-handler ↔ event-log surface: handler reads existing per-agent state, mutates per-place state via `set_component_place_dirtiness`, and emits a `DecisionEventPayload::WasteCreated` carrying `WasteCreatedPayload { creator, place, waste_lot, source: WasteSource::WildernessRelief, place_dirtiness_delta }`. Event payload (ticket 002) provides the `WasteSource::WildernessRelief` variant.
5. Existing test `relieve_wilderness_commit_effects` (line 1774) currently asserts agent dirtiness incremented + Waste lot created at place; this ticket extends those assertions to also cover `PlaceDirtiness.value` increment + `WasteCreated` event log entry. Existing test `relieve_wilderness_has_wilderness_relief_event_tag` (line 1815) asserts the existing `WildernessRelief` event tag (or whatever the current name is); this ticket may add a second tag emission (`WasteCreated`) — confirm during implementation whether both tags should fire or only the new one supersedes the older one (the spec keeps the existing `DisturbanceMarker` and adds `WasteCreated`, so both fire in parallel).

## Architecture Check

1. Reading current `PlaceDirtiness` via `txn.get_component_place_dirtiness(place).copied().unwrap_or_default()` and writing back via `set_component_place_dirtiness` keeps the mutation path inside the existing commit-handler structure — no new system, no new dispatch entry. Mirrors the per-agent-dirtiness increment pattern already in this handler. Per-place dirtiness contribution is read from the place's own component (`dirtiness_per_use`), not from the agent's profile, so the place is the authoritative source of "how dirty does relief make me" — preserves locality (FND-7) and concrete state (FND-3).
2. No backward-compat shim: the existing per-agent dirtiness path is preserved; per-place dirtiness is additive, not a replacement. The `DisturbanceMarker` evidence emission is unchanged.

## Verification Layers

1. Per-place dirtiness mutation occurs at commit time → focused unit test seeding a Place with `PlaceDirtiness { value: pm(0), dirtiness_per_use: pm(80), .. }`, running one `relieve_wilderness` commit, asserting `world.get_component_place_dirtiness(place).unwrap().value == pm(80)`.
2. `WasteCreated` event with `WildernessRelief` source emitted to the event log → event-log delta assertion on the same focused test.
3. Per-agent dirtiness path unchanged → existing `relieve_wilderness_commit_effects` continues to assert agent dirtiness increment by `wilderness_relief_dirtiness_penalty` (no regression).
4. Saturation bound respected → focused unit test with `value: pm(950), dirtiness_per_use: pm(80)` — assert post-commit value is `pm(1000)`, not overflow.

## What to Change

### 1. Extend `relieve_wilderness` commit handler in `needs_actions.rs:648–691`

After the existing per-agent dirtiness increment block, before the function returns, add:

```rust
let place = txn.effective_place(instance.actor).ok_or_else(|| {
    ActionError::InternalError(format!("actor {} has no place", instance.actor))
})?;
let mut place_dirt = txn.get_component_place_dirtiness(place).copied().unwrap_or_default();
let prev_value = place_dirt.value;
place_dirt.value = place_dirt.value.saturating_add(place_dirt.dirtiness_per_use);
let delta = Permille::new_unchecked(place_dirt.value.value().saturating_sub(prev_value.value()));
txn.set_component_place_dirtiness(place, place_dirt)?;

txn.add_tag(EventTag::WasteCreated)
    .set_decision_payload(DecisionEventPayload::WasteCreated(WasteCreatedPayload {
    creator: instance.actor,
    place,
    waste_lot,           // already in scope from the existing Waste creation block
    source: WasteSource::WildernessRelief,
    place_dirtiness_delta: delta,
}));
```

The live event API is transaction-tag based: add the tag and decision payload on the committed action transaction with `txn.add_tag(...).set_decision_payload(...)`.

### 2. Update existing tests

- `relieve_wilderness_commit_effects` (line 1774): seed the place with explicit `PlaceDirtiness { dirtiness_per_use: pm(80), .. }`, assert post-commit `place_dirtiness.value == pm(80)` and a `WasteCreated` event with the correct payload.
- `relieve_wilderness_has_wilderness_relief_event_tag` (line 1815): if the existing assertion checks for a single tag, extend to also assert `WasteCreated` is in the event log.

### 3. New focused test

Add `relieve_wilderness_place_dirtiness_saturates` to the same test module — seed `PlaceDirtiness { value: pm(950), dirtiness_per_use: pm(80) }`, run commit, assert `value == pm(1000)` (not 1030).

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify — handler extension at lines 648–691; test updates at 1774 and 1815; new test added near 1825)

## Out of Scope

- Toilet handler (deferred to ticket 006).
- Wash handler (deferred to ticket 007).
- Per-tick `PlaceDirtiness` decay (deferred to ticket 008).
- AI ranking that reads `PlaceDirtiness` (deferred to ticket 010).
- Golden coverage of place-dirtiness accumulation across multiple agents (deferred to ticket 012).

## Acceptance Criteria

### Tests That Must Pass

1. Updated `relieve_wilderness_commit_effects` — asserts both per-agent dirtiness (existing) and per-place `PlaceDirtiness.value` increment (new) plus a `WasteCreated` event with `WildernessRelief` source.
2. Updated `relieve_wilderness_has_wilderness_relief_event_tag` (or equivalent) — asserts `WasteCreated` event tag fires alongside any existing tag.
3. New focused test `relieve_wilderness_place_dirtiness_saturates` — saturation bound at `pm(1000)`.
4. Existing `relieve_wilderness_visibility_is_same_place` and `relieve_wilderness_commit_emits_scene_evidence` continue to pass unchanged.
5. Existing suite: `cargo test -p worldwake-systems`.

### Invariants

1. Per-agent dirtiness path unchanged — `HomeostaticNeeds.dirtiness` increment via `wilderness_relief_dirtiness_penalty` is preserved.
2. `PlaceDirtiness.value` saturates at `Permille::new_unchecked(1000)` (via `saturating_add`); no overflow.
3. Every `relieve_wilderness` commit emits exactly one `WasteCreated` event tag with `WasteSource::WildernessRelief` — no double emission, no missing emission.
4. The `WasteCreatedPayload.waste_lot` field references the same `EntityId` returned by the existing `create_item_lot` call.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` test module — extend `relieve_wilderness_commit_effects`, extend `relieve_wilderness_has_wilderness_relief_event_tag`, add `relieve_wilderness_place_dirtiness_saturates`.

### Commands

1. `cargo test -p worldwake-systems relieve_wilderness`
2. `cargo test -p worldwake-systems`
3. `cargo build --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-30.

- Extended `commit_relieve_wilderness` so each completed wilderness relief reads the actor's current place, increments that place's `PlaceDirtiness.value` by the place-authored `dirtiness_per_use`, and saturates at `pm(1000)`.
- Preserved the existing per-agent dirtiness increment, Waste `ItemLot` creation, and `DisturbanceMarker` scene evidence path.
- Added `EventTag::WasteCreated` to the `relieve_wilderness` action definition and attached a `DecisionEventPayload::WasteCreated` payload with `WasteSource::WildernessRelief`, the created waste lot id, and the actual saturated place-dirtiness delta.
- Extended focused `relieve_wilderness` tests to prove place dirtiness mutation, payload contents, causal tag registration, and saturation.

## Deviations

- The live event API is `txn.add_tag(...).set_decision_payload(...)`, so the implementation emits the `WasteCreated` payload on the committed action transaction rather than through the drafted `txn.emit_decision_event(...)` helper.
- Sibling tickets `S129PLADIRFAC-006` and `S129PLADIRFAC-007` had their drafted event-emission snippets retargeted to the same live transaction-tag API.
- No active spec or implementation-order truthing was required; S129 already lists this handler extension as an active deliverable and the ticket dependencies already point to archived prerequisites.

## Verification Result

Passed:

1. `cargo test -p worldwake-systems relieve_wilderness`
2. `cargo fmt --all`
3. `cargo test -p worldwake-systems`
4. `cargo build --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
