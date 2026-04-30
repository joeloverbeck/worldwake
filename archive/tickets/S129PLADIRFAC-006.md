# S129PLADIRFAC-006: toilet writes LatrineFullness with overflow handling

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `toilet` commit handler reads/writes per-place `LatrineFullness` and emits `WasteCreated` with `WasteSource::OvercapacityLatrine` on overflow
**Deps**: archive/tickets/S129PLADIRFAC-001.md, archive/tickets/S129PLADIRFAC-002.md

## Problem

Today's `toilet` commit handler (`crates/worldwake-systems/src/needs_actions.rs:617–646`) zeros bladder and creates a Waste lot at the place, but the latrine itself never develops fullness state. Without a fullness counter, latrines never become "full enough that an agent prefers wilderness", so the depth fix S129 calls out (latrines fill up → biased candidate ranking) is impossible. This ticket extends the handler to track per-latrine fullness, branch on the critical-threshold transition to handle overflow as wilderness-relief-equivalent place dirtiness, and emit `WasteCreated` with the appropriate source.

## Assumption Reassessment (2026-04-29)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `toilet` commit handler at `crates/worldwake-systems/src/needs_actions.rs:617–646`. Existing behavior verified during reassessment: gets actor's place via `txn.effective_place(instance.actor)` (line 626); creates Waste lot at place via `txn.create_item_lot(CommodityKind::Waste, Quantity(1))` + `txn.set_ground_location(...)` (lines 629–633); zeros bladder via `set_actor_needs(...)` with `bladder=pm(0)` (lines 634–644). Existing inline test: `toilet_reduces_bladder_and_creates_waste` (line 1305).
2. The action's existing precondition `Precondition::ActorAtPlaceTag(Latrine)` (verified at `needs_actions.rs:146–150`) guarantees the commit handler's `effective_place(actor)` resolves to a `PlaceTag::Latrine` place. Combined with ticket 011's tag-conditional spawn pattern, every reachable code path means `get_component_latrine_fullness(place)` returns `Some(_)` — but defensive `unwrap_or_default()` is still appropriate for safety.
3. The shared abstraction boundary under audit is the same commit-handler ↔ event-log surface as ticket 005, with the additional branch on `fill < critical_threshold` that decides between under-capacity (increment fill) and over-capacity (increment fill, additionally write `PlaceDirtiness`, emit with `WasteSource::OvercapacityLatrine`).
4. Per spec D6, `WasteCreated` with `OvercapacityLatrine` should fire (a) on the commit that crosses `critical_threshold` for the first time, and (b) on every commit thereafter while still over-capacity. The ticket implements (a)+(b): if `new_fill >= critical_threshold`, emit with `OvercapacityLatrine`; otherwise emit with `WildernessRelief` (no — spec D6 reads more carefully: under-threshold commits do not emit `WasteCreated` per the spec. Re-reading spec D6: "Emit `WasteCreated` with `WasteSource::OvercapacityLatrine` only if the new fill crossed `critical_threshold` on this tick (so the first overflow event records the transition)." and "If at or over threshold: still zero bladder, but additionally increment `PlaceDirtiness.value` by `dirtiness_per_use` and emit `WasteCreated` with `WasteSource::OvercapacityLatrine`." So: under-threshold commits emit nothing on the WasteCreated channel; the first crossing emits; subsequent over-threshold commits emit. Confirm during implementation.)
5. Existing focused/unit coverage: `toilet_reduces_bladder_and_creates_waste` (line 1305), `toilet_affordance_requires_latrine_tagged_place` (line 1374). The first will be extended; the second remains unchanged (it asserts affordance precondition, not commit behavior).

## Architecture Check

1. Branching commit logic on the `fill < critical_threshold` boundary lets a single handler express the two distinct outcomes (clean latrine use vs. overflowed latrine use) without splitting into two action variants. The over-capacity case mutates `PlaceDirtiness` on the latrine's place (the latrine has overflowed and the place itself accumulates the consequence) — this is the natural wilderness-relief-equivalent state, mirroring ticket 005's path. Per FND-10 (outcomes are granular and leave aftermath), an overflowed latrine still completes the bladder zeroing but produces "more state" (place dirtiness).
2. No backward-compat shim: today the toilet handler does not read per-latrine state at all; this ticket adds the read-and-mutate path additively. The Waste lot creation pattern is unchanged — overflow Waste lots are the same `ItemLot` shape as wilderness-relief Waste lots, distinguished only by the `WasteSource` variant in the decision event payload.

## Verification Layers

1. Per-latrine fullness mutation occurs at commit time → focused unit test seeding `LatrineFullness { fill: pm(0), fill_per_use: pm(80), critical_threshold: pm(800) }`, running commit, asserting `fill` increments to `pm(80)`.
2. Over-threshold commit additionally writes `PlaceDirtiness` and emits `WasteCreated::OvercapacityLatrine` → focused unit test seeding `LatrineFullness { fill: pm(750), fill_per_use: pm(80), critical_threshold: pm(800) }`, running commit, asserting (a) `fill == pm(830)`, (b) `PlaceDirtiness.value` incremented by the place's `dirtiness_per_use`, (c) `WasteCreated` event with `OvercapacityLatrine` source.
3. Under-threshold commit does NOT emit `WasteCreated` → focused unit test asserting absence per spec D6's emission semantics.
4. Saturation bound on `LatrineFullness.fill` — saturating add so `fill + fill_per_use` does not exceed `pm(1000)`.

## What to Change

### 1. Extend `toilet` commit handler in `needs_actions.rs:617–646`

Insert read/branch/write logic between the Waste lot creation and the bladder zeroing. Pseudocode (verify against existing handler structure during implementation):

```rust
let place = txn.effective_place(instance.actor).ok_or_else(|| {
    ActionError::InternalError(format!("actor {} has no place", instance.actor))
})?;
let mut latrine = txn.get_component_latrine_fullness(place).copied().unwrap_or_default();
let prev_fill = latrine.fill;
latrine.fill = latrine.fill.saturating_add(latrine.fill_per_use);
txn.set_component_latrine_fullness(place, latrine)?;

let crossed_threshold_now = prev_fill < latrine.critical_threshold && latrine.fill >= latrine.critical_threshold;
let was_already_over = prev_fill >= latrine.critical_threshold;

if crossed_threshold_now || was_already_over {
    let mut place_dirt = txn.get_component_place_dirtiness(place).copied().unwrap_or_default();
    let prev_value = place_dirt.value;
    place_dirt.value = place_dirt.value.saturating_add(place_dirt.dirtiness_per_use);
    let delta = Permille::new_unchecked(place_dirt.value.value().saturating_sub(prev_value.value()));
    txn.set_component_place_dirtiness(place, place_dirt)?;

    txn.add_tag(EventTag::WasteCreated)
        .set_decision_payload(DecisionEventPayload::WasteCreated(WasteCreatedPayload {
        creator: instance.actor,
        place,
        waste_lot,
        source: WasteSource::OvercapacityLatrine,
        place_dirtiness_delta: delta,
    }));
}
```

The under-threshold branch does NOT emit `WasteCreated` — per spec D6's explicit emission semantics. (Cross-check with the `relieve_wilderness` ticket 005 pattern; the two handlers differ in this respect.)

### 2. Update existing test

`toilet_reduces_bladder_and_creates_waste` (line 1305): retain the existing bladder + Waste-lot assertions; additionally seed `LatrineFullness` and assert `fill` increments correctly.

### 3. New focused tests

- `toilet_overflow_emits_waste_created_with_overcapacity_source` — seed `fill: pm(750)`, `critical_threshold: pm(800)`, run commit, assert event-log entry + `PlaceDirtiness` increment.
- `toilet_under_threshold_does_not_emit_waste_created` — seed `fill: pm(0)`, run commit, assert no `WasteCreated` event.
- `toilet_already_over_threshold_emits_waste_created_each_tick` — seed `fill: pm(900)`, run commit, assert event fires; `PlaceDirtiness` increments; `fill` saturates at `pm(1000)`.
- `toilet_latrine_fullness_saturates_at_max` — seed `fill: pm(950), fill_per_use: pm(80)`, assert post-commit `fill == pm(1000)`.

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify — handler extension at lines 617–646; test extension at 1305; new tests added near 1374)

## Out of Scope

- `clean_latrine` action and `LatrineMaintained` event tag — explicitly deferred per spec Non-Goals (settlement-staff role spec required).
- AI ranking integration that prefers latrines below `critical_threshold` (deferred to ticket 010).
- Per-place latrine candidate emission (deferred to ticket 009).
- Golden coverage of latrine overcapacity behavior across multiple uses (deferred to ticket 012).

## Acceptance Criteria

### Tests That Must Pass

1. Updated `toilet_reduces_bladder_and_creates_waste` — bladder + Waste lot + `LatrineFullness.fill` increment all asserted.
2. New focused test `toilet_overflow_emits_waste_created_with_overcapacity_source` — first crossing of `critical_threshold` emits `WasteCreated::OvercapacityLatrine` with `PlaceDirtiness` increment.
3. New focused test `toilet_under_threshold_does_not_emit_waste_created` — under-threshold commits emit no `WasteCreated`.
4. New focused test `toilet_already_over_threshold_emits_waste_created_each_tick` — repeated over-threshold commits each emit.
5. New focused test `toilet_latrine_fullness_saturates_at_max` — saturation bound enforced.
6. Existing `toilet_affordance_requires_latrine_tagged_place` continues to pass unchanged.
7. Existing suite: `cargo test -p worldwake-systems`.

### Invariants

1. `LatrineFullness.fill` is `saturating_add`-bounded at `pm(1000)`; no overflow.
2. `WasteCreated` with `OvercapacityLatrine` fires if and only if (post-increment) `fill >= critical_threshold`. Under-threshold commits emit nothing on this channel.
3. Bladder zeroing path unchanged regardless of whether the latrine was over capacity — the action still completes successfully (per spec D6 + FND-10: overflow is degraded outcome, not failure).
4. Over-capacity commits write `PlaceDirtiness` on the latrine's place using the place's own `dirtiness_per_use` (not the agent's metabolism penalty) — same source-of-truth as ticket 005.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` test module — extend `toilet_reduces_bladder_and_creates_waste`, add four new tests covering threshold-crossing semantics.

### Commands

1. `cargo test -p worldwake-systems toilet`
2. `cargo test -p worldwake-systems`
3. `cargo build --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-30.

- Extended `commit_toilet` so every toilet commit increments per-place `LatrineFullness.fill` with `saturating_add`.
- Added the overflow branch: when the post-use fill is at or above `critical_threshold`, the handler increments the latrine place's `PlaceDirtiness.value` by that place's `dirtiness_per_use` and attaches a `WasteCreated` decision payload with `WasteSource::OvercapacityLatrine`.
- Preserved the under-threshold event contract: ordinary under-capacity toilet use still creates the Waste lot and zeros bladder, but does not emit `WasteCreated`.
- Extended the focused toilet coverage for under-threshold, first-crossing, repeated over-threshold, and saturation behavior.

## Deviations

- The landed handler uses the live post-increment predicate `latrine.fill >= latrine.critical_threshold` rather than carrying explicit `crossed_threshold_now` / `was_already_over` booleans; this is equivalent to the final invariant after `LatrineFullness.fill` is monotonically updated by `saturating_add`.
- `EventTag::WasteCreated` was not added to the `toilet` action definition's unconditional `causal_event_tags`, because under-threshold toilet commits must not emit that tag. The commit handler attaches it only on overflow events.
- The existing `relieve_wilderness` place-dirtiness delta calculation was locally simplified to the same `Permille::saturating_sub` shape used by the new overflow branch; this is behavior-preserving for the existing monotonic increment path.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib toilet -- --list` (resolved 6 intended toilet tests).
- Passed `cargo test -p worldwake-systems --lib toilet`.
- Passed `cargo test -p worldwake-systems`.
- Passed `cargo build --workspace`.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
