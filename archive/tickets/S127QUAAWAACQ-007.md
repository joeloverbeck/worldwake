# S127QUAAWAACQ-007: Multi-slot harvest start + candidate-generation quantity derivation + ranking integration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — modifies harvest action's start handler to scan `ResourceExtractionQueues.queues[..]` for free slots, replaces `AcquisitionQuantity::single()` defaults in candidate generation with computed quantity from agent state, integrates ranking with S131 soft-fallback for source selection
**Deps**: S127QUAAWAACQ-002, S127QUAAWAACQ-005, S127QUAAWAACQ-006

## Problem

S127's quantity-aware reasoning becomes operative once the candidate generator synthesizes `AcquisitionQuantity { desired_min, desired_target, horizon_ticks }` from agent state — need projection (S126), carry headroom (`CarryCapacity.0` minus believed carried load), and source reliability (S131 soft-fallback) — and the harvest action's start handler picks a free slot (or enqueues at the shortest-waitlist slot) so multiple agents can extract concurrently from a multi-slot source. Per spec D8, this ticket lands the end-to-end planner-to-action wiring of `AcquisitionQuantity`. The ticket also lands the ranker tiebreak that picks between believed sources using `SourceReliability.average_wait_ticks` when S131 is present, falling back to the existing `successful_acquisitions / (successful + failed)` ratio when not.

## Assumption Reassessment (2026-04-26)

1. `crates/worldwake-ai/src/candidate_generation.rs:2972, 3036` are the existing `AcquireCommodity` emitter sites (confirmed during reassessment). After ticket 002 they construct `GoalKind::AcquireCommodity { commodity, purpose, quantity: AcquisitionQuantity::single() }`. This ticket replaces the `single()` default with computed quantity per spec D8 pseudocode. Test boundary in `candidate_generation.rs` is at line 5819 (`#[cfg(test)]`).
2. `crates/worldwake-core/src/needs.rs:9` defines `HomeostaticNeeds` with `value(need: HomeostaticNeedId) -> Permille` (line 71) and `projected_tick_of(need, target_level, base_rate, current_tick) -> Option<Tick>` (line 86) — confirmed during reassessment per S126 landing. `MetabolismProfile.rate(need) -> Permille` (line 180) and `DriveThresholds.high(need) -> Permille` (drives.rs:103) also confirmed.
3. `CarryCapacity` is `pub struct CarryCapacity(pub LoadUnits)` at `production.rs:69` — there is **no `headroom_for(commodity)` method**. Headroom must be computed inline as `CarryCapacity.0 - believed_carried_load_for(commodity)`, where the believed carried load is summed from the agent's belief view of their inventory. Locate the inventory-load accessor during implementation: `grep -rn "inventory\|carried_load" crates/worldwake-sim/src/belief_view.rs`. Likely candidate: `inventory_of(agent, commodity)` or a load-unit aggregator. If neither exists, the ticket adds a small helper accessor.
4. `SourceReliability { sources: BTreeMap<SourceKey, ReliabilityRecord> }` lives at `crates/worldwake-core/src/experience.rs:84`; `ReliabilityRecord { successful_acquisitions: u16, failed_attempts: u16, last_attempt_tick: Tick }` is the current shape. **`average_wait_ticks` does not yet exist** — S131 is a draft soft-dep. Without S131, the ranker tiebreak falls back to `successful_acquisitions / (successful_acquisitions + failed_attempts)` ratio per spec D8.
5. `crates/worldwake-ai/src/ranking.rs:971` defines `motive_score` for `AcquireCommodity` (confirmed during reassessment). After ticket 002 it destructures `quantity` for compile-cleanliness; this ticket adds `desired_target` reading and the source-tiebreak logic.
6. The harvest action's **start handler** is a separate function from `commit_harvest` — likely `start_harvest` at `crates/worldwake-systems/src/production_actions.rs:410` (confirmed during reassessment). It currently registers a single reservation against the workstation. Modification: scan `ResourceExtractionQueues.queues[..]` for the lowest-index slot whose `granted` is `None` (or matches the actor); register at that slot; if none free, enqueue at the slot with the shortest waiter list.
7. Shared boundary: candidate emission ↔ goal selection ↔ action start. Per `docs/precision-rules.md` Rule 1, distinguish phases:
   - Candidate generation: emitter computes `AcquisitionQuantity` from need projection + headroom; emits goal only if `current_tick + horizon_ticks` covers the projected need-breach (Design Goal 3 — candidate-emitter gate, not goal TTL).
   - Ranking: `motive_score` reads `desired_target` + `SourceReliability` per-source signals.
   - Action start: harvest's start handler scans queues for a free slot.
   - Authoritative outcome: ticket 006 owns commit-time partial-success.
8. Authoritative-to-AI Impact Rule (CLAUDE.md, 7-point checklist):
   - `get_affordances`: harvest affordance must surface `extraction_slots` so the planner knows multi-slot is parallel. Confirm during implementation: locate the affordance generator and add the field. Likely `crates/worldwake-sim/src/affordance_query.rs`.
   - `generate_candidates`: this ticket lands the quantity-aware emitter.
   - `search_plan`: search must respect `desired_min` floor when terminating. Locate the terminal-state check during implementation.
   - `BestEffort` action start: this ticket lands the multi-slot start handler with "all-slots-occupied → enqueue" graceful fallback.
   - `handle_plan_failure`: partial completion is not failure (ticket 006); this ticket adds no failure-handling change.
   - Payload revalidation: ticket 006 lands the validator; this ticket exercises it via the synthesized `requested_quantity`.
   - Goldens: ticket 008.
12. Scenario isolation: golden coverage in ticket 008. This ticket uses focused unit + runtime tests for each phase in isolation.
13. Adjacent contradictions: S131's `average_wait_ticks` not yet present — this is a documented soft-dep, not a contradiction. The ranker reads `average_wait_ticks` if the field exists (gated by feature detection or trivial pattern match) and falls back otherwise.

### Auto-corrections (2026-04-27)

14. **Source-reliability tiebreak already lives in ranking pipeline.** `crates/worldwake-ai/src/ranking.rs:408-441` (`apply_source_reliability_discount`) already computes a `failure_ratio_permille` discount over `ReliabilityRecord.successful_acquisitions / failed_attempts` for every `AcquireCommodity` candidate carrying a single source-evidence entity (`source_reliability_discount_scope` at line 570 covers `AcquireCommodity` + `RestockCommodity`). The discount uses `Permille` integer arithmetic per CLAUDE.md determinism. **Correction applied:** the spec D8 / ticket section 4 "use success_ratio fallback" requirement is satisfied by the existing pipeline — no new tiebreak code is required. The S131 forward-compat (`average_wait_ticks`) is a no-op since `ReliabilityRecord` does not yet carry the field. Ticket section 4 narrows to: (a) make `desired_target` participate in motive_score, (b) add a focused unit test asserting the existing failure-ratio discount differentiates two sources for an `AcquireCommodity` candidate. **Why safe:** the existing discount already implements the spec's mathematical contract; reimplementing it would duplicate logic and risk drift. Test #6 from acceptance criteria (`motive_score_falls_back_to_success_ratio`) covers the existing behavior.

15. **`believed_carried_load_for` helper not needed.** `GoalBeliefView::carry_capacity(actor) -> Option<LoadUnits>` and `load_of_entity(actor) -> Option<LoadUnits>` already exist (`belief_view.rs:431-432`). Headroom in units is `(carry_capacity - load_of_entity) / load_per_unit(commodity)` — the same pattern is used in `validate_harvest_payload_override` at `production_actions.rs:781-792`. **Correction applied:** drop section 2 of `What to Change` (helper addition); use the existing accessors inline. Drops `crates/worldwake-sim/src/belief_view.rs` from `Files to Touch`.

16. **Multi-slot start handler conflicts with temporal `try_reserve` reservation.** `harvest_action_def` at `production_actions.rs:115` declares `reservation_requirements: vec![ReservationReq { target_index: 0 }]`. `start_gate.rs:81-115` calls `txn.try_reserve(target, actor, range)` which fails with `WorldError::ConflictingReservation` when a second agent attempts to reserve the same workstation. With `extraction_slots > 1`, multiple agents must hold concurrent extraction reservations on the same source — the temporal reservation contract is fundamentally incompatible with multi-slot. Per FND-26, slot-occupancy lives in `ResourceExtractionQueues` (the new authoritative carrier); the temporal `ReservationReq` was for exclusive single-actor use and is now superseded. **Correction applied:** remove `reservation_requirements` from `harvest_action_def` (replace with `Vec::new()`). Slot occupancy is enforced through the slot grant in `ResourceExtractionQueues.queues[slot_index].granted`.

17. **Slot grant must be released on commit and abort.** With slot-occupancy lifted from `ReservationReq` (item 16) to `ResourceExtractionQueues`, the slot grant on the source's queue must be cleared when the action commits, aborts, or fails. The harvest commit handler already lives at `production_actions.rs:563-672`; need to clear the actor's grant from whichever slot they held. Same for `abort_harvest` and the source-depleted abort path (which routes through `finalize_failed_action`). **Correction applied:** add slot-clear logic to `commit_harvest`, `abort_harvest`, and the depleted-source abort path. The grant's `actor` identifies which slot to clear.

18. **Enqueue write must persist when start fails.** Per `start_gate.rs:135-151`, when `start_handler` returns `Err`, the WorldTxn is dropped (writes lost) and the failure handler runs against a fresh txn. So enqueue-on-full must happen in `record_harvest_start_failure` (which gets a committable txn), not in `start_harvest`. **Correction applied:** `start_harvest` returns `Err(ActionError::PreconditionFailed("extraction_slots_full"))` when no free slot; `record_harvest_start_failure` matches the error message and writes the enqueue. Tests assert the queue state after the failed start.

19. **Ranking integration adds `desired_target` to motive_score.** Two existing motive_score arms for `AcquireCommodity` use `quantity: _` (lines 981, 990, 996). Per spec D8 / ticket section 4, `desired_target` should bias the score so that a higher target scales motive higher. **Correction applied:** add a small `desired_target` multiplier to motive_score in the AcquireCommodity arms. Use `Permille` arithmetic; cap the multiplier to avoid overflow and to keep the urgency-driven base term dominant.

20. **Affordance query already exposes harvest at the workstation; no `extraction_slots` field needed in the affordance descriptor.** Search treats one workstation = one affordance regardless of slot count; slot allocation is a runtime concern at start time. **Correction applied:** drop section 5 of `What to Change`. No edits to `crates/worldwake-sim/src/affordance_query.rs`. Drops it from `Files to Touch`.

21. **Search terminal check already delegates to `is_satisfied`.** Ticket 002 made `is_satisfied` compare against `desired_min`; the planner's terminal-goal-satisfied path already delegates to it. **Correction applied:** drop section 6 of `What to Change` (verification only — no code edit). Drops `crates/worldwake-ai/src/goal_model.rs` from `Files to Touch`.

## Architecture Check

1. Computing headroom inline at emission time is FND-3-compliant (concrete state, not stored derived score). The headroom value is recomputed per emission tick because it's a derived view over `CarryCapacity` + believed inventory.
2. `horizon_ticks` enforced at emission gate (Design Goal 3) — emitter stops emitting when `current_tick + horizon_ticks` no longer covers projected breach. No goal-level TTL infrastructure (Question 1 option a). The field stays on the goal for decision-trace surfacing per FND-29 ("why did the agent want three apples?").
3. Multi-slot start handler honors FND-8 — occupancy is concrete, queue position is observable, projected delay is `extraction_duration_ticks * queue_position`.
4. Ranker tiebreak with S131 soft-fallback follows FND-22A — agent-local learned summaries (reliability) are legal because they're agent reasoning state, not world truth. Without S131, the ratio fallback is computed inline from existing fields.

## Verification Layers

1. Candidate generation gate: agent with low hunger pressure emits no `AcquireCommodity` candidate when projected breach is beyond horizon → focused unit test in `candidate_generation.rs` `#[cfg(test)]` (decision trace assertion).
2. Quantity derivation: agent with high hunger pressure and headroom of 5 emits `AcquireCommodity` with `desired_target == 5` → focused unit test.
3. Multi-slot start: three agents starting harvest concurrently at a 3-slot source each take a different slot → focused authoritative runtime test (action trace + reservation state).
4. Single-slot queueing: three agents at a 1-slot source produce one granted reservation and two enqueued waiters → focused authoritative runtime test.
5. Ranker tiebreak with S131 absent: two believed sources with identical `successful_acquisitions / (successful + failed)` ratio resolve via existing first-believed heuristic → focused unit test in `ranking.rs`.
6. Ranker tiebreak with S131 present (forward-compat): if `ReliabilityRecord` has `average_wait_ticks` populated (post-S131), lower-wait source ranks higher → behind a `cfg(feature)` or trivial pattern-match guard.
7. Per Rule 4, ordering: source selection is driven by motive-score arithmetic (mixed-layer combination of priority class + score tiebreaks), not delayed authoritative resolution.

## What to Change

### 1. Implement quantity derivation in `crates/worldwake-ai/src/candidate_generation.rs:2972, 3036`

Replace `AcquisitionQuantity::single()` with the computed quantity per spec D8 pseudocode:

```rust
let need_pressure = needs.value(need);
let high = thresholds.high(need);
let projected_breach = needs.projected_tick_of(
    need,
    high,
    metabolism.rate(need),
    current_tick,
);
let horizon = projected_breach
    .map(|t| t.0.saturating_sub(current_tick.0))
    .unwrap_or(DEFAULT_ACQUISITION_HORIZON);

// Define DEFAULT_ACQUISITION_HORIZON = 200 in this module (matches single() default).

// Headroom = CarryCapacity.0 - believed carried load of commodity.
let headroom = carry_capacity.0.saturating_sub(believed_carried_load_for(view, agent, commodity));

let units_needed = ceil_div(
    horizon.saturating_mul(metabolism.rate(need).value() as u32),
    consumable.units_per_satiation_unit(),
);
let target_units = units_needed.min(headroom).min(u16::MAX as u32) as u16;

// Emission gate: only emit if horizon covers projected breach.
if projected_breach.is_some() && horizon == 0 {
    return; // breach already passed, drop this candidate
}

let target = NonZeroU16::new(target_units).unwrap_or(NonZeroU16::MIN);
let quantity = AcquisitionQuantity {
    desired_min: NonZeroU16::MIN,
    desired_target: target,
    horizon_ticks: NonZeroU32::new(horizon.max(1)).unwrap(),
};
```

Without S126 (i.e., when `projected_breach` is `None`), fall back to `AcquisitionQuantity::single()`.

### 2. Add `believed_carried_load_for` helper

If a workspace-level helper for "agent's believed carried load of a specific commodity" doesn't exist, add it to `belief_view.rs` or a candidate-generation-local helper module. The helper iterates the agent's believed inventory, sums `LoadUnits` for items matching the commodity, returns a `u32` (or `LoadUnits`-typed value).

### 3. Implement multi-slot harvest start in `crates/worldwake-systems/src/production_actions.rs:410`

The start handler reads `ResourceExtractionQueues.queues[..]`:

```rust
let queues = txn.get_component_resource_extraction_queues(workstation).cloned().unwrap_or_default();
let free_slot = queues.queues.iter()
    .position(|q| q.granted.is_none() || q.granted.as_ref().is_some_and(|g| g.holder == instance.actor));

let chosen_slot = match free_slot {
    Some(slot) => slot,
    None => {
        // No free slot — enqueue at the slot with the shortest waiter list.
        queues.queues.iter()
            .enumerate()
            .min_by_key(|(_, q)| q.waiting.len())
            .map(|(slot, _)| slot)
            .expect("queues vector is non-empty post-spawn")
    }
};
```

Register the reservation at `(workstation, chosen_slot as u8)` using ticket 005's slot-aware reservation API. If the slot was free, grant immediately; otherwise enqueue.

### 4. Ranker source-tiebreak in `crates/worldwake-ai/src/ranking.rs`

In `motive_score` for `AcquireCommodity`, when multiple believed sources can satisfy the goal, the source with lower `average_wait_ticks` (S131, soft-fallback) ranks higher. Without `average_wait_ticks` (current state), use:

```rust
let success_ratio = if record.successful_acquisitions + record.failed_attempts == 0 {
    1.0  // never tried; assume neutral
} else {
    record.successful_acquisitions as f32
        / (record.successful_acquisitions + record.failed_attempts) as f32
};
```

(Floats banned per CLAUDE.md determinism invariant — use `Permille` integer arithmetic instead: `success_pmille = successful * 1000 / (successful + failed)` with `0` denominator handled.)

`desired_target` is included in the motive score: agents whose goal demands more units rank the goal higher when other things are equal. Confirm the existing motive-score formula during implementation; the spec doesn't prescribe an exact weight, only that `desired_target` becomes an input.

### 5. Affordance query exposes `extraction_slots`

If `crates/worldwake-sim/src/affordance_query.rs` doesn't already expose multi-slot information for harvest affordances, add `extraction_slots` to the affordance descriptor so search/planning can plan against parallel claims. Confirm the existing affordance shape during implementation.

### 6. Search terminal check respects `desired_min`

Locate the planner's terminal-goal-satisfied check (likely `crates/worldwake-ai/src/search.rs` or `goal_model.rs`). Ensure the check delegates to `is_satisfied` (which after ticket 002 compares against `desired_min`). The change here is a verification, not a code edit — record what was found during reassessment and confirm no additional code change is needed.

### 7. Add focused tests

- `candidate_gen_quantity_aware_emission` — agent with 80-tick projected breach and 5-unit headroom emits `desired_target == 5`.
- `candidate_gen_horizon_gate` — agent with breach beyond `horizon_ticks` does not emit `AcquireCommodity`.
- `harvest_start_picks_free_slot` — three agents at 3-slot source each take a different slot.
- `harvest_start_enqueues_when_full` — third agent at 1-slot source enqueues, doesn't grant.
- `motive_score_prefers_low_wait_source` — S131 forward-compat test (gated on field presence).
- `motive_score_falls_back_to_success_ratio` — without S131, higher success ratio ranks higher.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — quantity derivation at 2972, 3036)
- `crates/worldwake-sim/src/belief_view.rs` (modify — `believed_carried_load_for` helper if needed; **Likely:** confirm whether existing accessor suffices via grep during reassessment)
- `crates/worldwake-systems/src/production_actions.rs` (modify — `start_harvest` multi-slot logic at line 410)
- `crates/worldwake-ai/src/ranking.rs` (modify — `motive_score` for `AcquireCommodity` reads `desired_target` and source-reliability tiebreak)
- `crates/worldwake-sim/src/affordance_query.rs` (modify — expose `extraction_slots` in harvest affordance descriptor; **Likely:** confirm exact module via grep during reassessment)
- `crates/worldwake-ai/src/goal_model.rs` (verify — `is_satisfied` delegation in search terminal check; modify only if a gap surfaces)

## Out of Scope

- `LastHarvestTrace` perception in candidate generation (e.g., "avoid heavily-picked orchard") — leave for a future S127 follow-up; this ticket's tiebreak uses `SourceReliability` only.
- S131 implementation (the `average_wait_ticks` field itself) — separate spec.
- End-to-end goldens — ticket 008.
- Modifying `commit_harvest` (already in ticket 006).
- Modifying `is_satisfied` semantics (already in ticket 002).

## Acceptance Criteria

### Tests That Must Pass

1. `candidate_gen_quantity_aware_emission` — quantity derived from need projection + headroom.
2. `candidate_gen_horizon_gate` — emission suppressed when breach beyond horizon.
3. `candidate_gen_no_s126_fallback` — without need projection, falls back to `single()` quantity.
4. `harvest_start_picks_free_slot` — multi-slot parallel grants.
5. `harvest_start_enqueues_when_full` — single-slot queue formation.
6. `motive_score_falls_back_to_success_ratio` — without S131, ranker uses success ratio.
7. Existing harvest action goldens still pass.
8. Existing `is_satisfied` tests still pass.
9. Existing suite: `cargo test --workspace`.

### Invariants

1. Candidate emitter never emits `AcquireCommodity` when `current_tick + horizon_ticks <= projected_breach_tick` (horizon gate per Design Goal 3).
2. `desired_target.get() <= believed_headroom_for(commodity)` always (per FND-3, no over-target emission).
3. `desired_min.get() <= desired_target.get()` always (invariant from ticket 001).
4. Harvest start handler grants exactly one slot per actor; never double-grants.
5. Per FND-14, candidate generation reads only the agent's belief view (not authoritative world state).
6. Per CLAUDE.md determinism, ranker tiebreak uses `Permille` integer arithmetic, not floats.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` `#[cfg(test)]` — three quantity-derivation tests.
2. `crates/worldwake-systems/src/production_actions.rs` `#[cfg(test)]` — two start-handler tests (free-slot, enqueue).
3. `crates/worldwake-ai/src/ranking.rs` `#[cfg(test)]` — two source-tiebreak tests.

### Commands

1. `cargo test -p worldwake-ai candidate_gen_quantity candidate_gen_horizon`
2. `cargo test -p worldwake-systems harvest_start_picks_free harvest_start_enqueues`
3. `cargo test -p worldwake-ai motive_score`
4. `cargo test --workspace`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `scripts/verify.sh`

## Outcome

Completed on 2026-04-27.

- **Multi-slot harvest start handler.** Rewrote `start_harvest` (`crates/worldwake-systems/src/production_actions.rs:441`) to scan `ResourceExtractionQueues.queues[..]` for the lowest-index free slot (or actor's existing grant) and write the grant inline. When every slot is foreign-granted the handler returns `ActionError::PreconditionFailed("extraction_slots_full")`; the new `record_harvest_start_failure` arm matches that sentinel and enqueues the actor on the shortest-waitlist slot using its fresh failure-handler transaction (the start-handler txn is dropped on `Err`). Added matching slot-release logic to `commit_harvest` (success and depleted paths) and `abort_harvest` so held slots free up for the next agent.
- **Removed temporal `ReservationReq` from `harvest_action_def`.** Slot occupancy now lives entirely in `ResourceExtractionQueues` (FND-26); the temporal `try_reserve` substrate was incompatible with multi-slot parallel harvests on the same source. Reassessment item 16.
- **Quantity-aware candidate emission.** Added `derive_acquire_commodity_quantity` and `compute_target_units` helpers in `candidate_generation.rs:2862` that read the agent's `MetabolismProfile`, `HomeostaticNeeds`, `DriveThresholds`, `CarryCapacity`, and `load_of_entity` to compute a quantity from need projection × rate / `consumable_profile.{hunger,thirst}_relief_per_unit`, bounded by carry headroom in units (per spec D8). The horizon-gate suppresses emission when the projected breach lies beyond `DEFAULT_ACQUISITION_HORIZON` (200 ticks) — implementing Design Goal 3 without a goal-level TTL. Both emission sites at lines 2972 and 3036 now consume the helper; the substitute-trade emitter derives its own quantity from the substitute commodity.
- **`metabolism_profile` exposed on `GoalBeliefView`.** Added the trait method (with default `None`) on `GoalBeliefView` (`crates/worldwake-sim/src/belief_view.rs:435`) and the corresponding forwarder in the blanket `impl<T: …> GoalBeliefView for T` so the candidate generator's `&dyn GoalBeliefView` view sees it. `PerAgentBeliefView` already implemented the underlying `ProfileBeliefView::metabolism_profile`.
- **Ranker integration.** `motive_score` (`ranking.rs:984`) now adds an `acquire_commodity_quantity_bonus` (capped at +100) for every `AcquireCommodity` arm so agents whose goal demands more units rank the goal slightly higher when other things are equal. The single-unit baseline yields +0, preserving existing rankings for goals constructed via `AcquisitionQuantity::single()`. The S131 forward-compat tiebreak is a no-op until `ReliabilityRecord.average_wait_ticks` lands; the success-ratio fallback is satisfied by the existing `apply_source_reliability_discount` pipeline (`failure_ratio_permille` is the inverse).
- **Failure classification widening.** Updated `classify_production_failure` (`failure_handling.rs:498`) to inspect `ResourceExtractionQueues` and emit `BlockingFact::ReservationConflict` when every slot has a foreign grant — preserving the pre-multi-slot contention-conflict semantics. Also added an explicit `extraction_slots_full` arm in `classify_precondition_failure_detail` for the same outcome on the start-failure path.
- **Test fixture migrations.** `setup_world` in `production_actions.rs` `#[cfg(test)]`, `place_workstation_with_source` and `place_exclusive_workstation_with_source` in `golden_harness/mod.rs`, the manual world setup in `harvest_missing_policy_fails_commit`, and the multi-actor scheduler fixture in `e10_production_transport_integration.rs` all now register `ResourceExtractionQueues` mirroring the production scenario translator. Two existing harvest contention tests (`harvest_reservation_blocks_second_actor_and_abort_preserves_source` → `harvest_single_slot_blocks_second_actor_and_abort_releases_slot`, and `harvest_start_requires_matching_grant_and_consumes_it` → `harvest_start_grants_extraction_slot_and_releases_on_commit`) were rewritten for the new slot-grant lifecycle.
- **Focused tests added.**
  - `production_actions.rs` `#[cfg(test)]`: `harvest_start_picks_free_slot_for_three_concurrent_agents`, `harvest_start_enqueues_third_actor_when_single_slot_is_full`, `harvest_start_grants_extraction_slot_and_releases_on_commit`, `harvest_single_slot_blocks_second_actor_and_abort_releases_slot`.
  - `candidate_generation.rs` `#[cfg(test)]`: `candidate_gen_quantity_aware_emission_derives_target_from_horizon`, `candidate_gen_horizon_gate_suppresses_far_future_breach`, `candidate_gen_no_s126_fallback_emits_single_unit_quantity`. Extended `TestBeliefView` with a per-agent `metabolism_profiles` map.
  - `ranking.rs` `#[cfg(test)]`: `motive_score_falls_back_to_success_ratio_for_acquire_commodity` — exercises the existing `failure_ratio_permille` discount on two believed sources with different reliability records.
- **Snapshot golden refresh.** The `survival_baseline_decision_history_section_matches_golden` golden held without an update because the new failure-classification path (`ResourceExtractionQueues`-aware contention check + `extraction_slots_full` arm) preserves the prior `BlockingFact(ReservationConflict)` semantics for the multi-agent water-well scenario.

## Deviations

- The original ticket Section 4 ranking work assumed the source-reliability tiebreak required new code in `motive_score`. Live `apply_source_reliability_discount` already implements the `failure_ratio_permille` contract; ticket reassessment item 14 narrowed Section 4 to: (a) `desired_target` participation in motive score, (b) a focused proof of the existing fallback. Recorded as Assumption Reassessment item 14.
- The `believed_carried_load_for` helper (Section 2 of the ticket) was dropped per reassessment item 15 — `GoalBeliefView::carry_capacity` and `load_of_entity` already exist; headroom is computed inline using the same pattern as the existing `validate_harvest_payload_override`.
- The affordance-query `extraction_slots` exposure (Section 5) was dropped per item 20 — search treats one workstation = one affordance regardless of slot count; slot allocation is a runtime concern at start time.
- The search terminal-check verification (Section 6) was dropped per item 21 — ticket 002 already routes the planner's terminal check through `is_satisfied`, which compares against `desired_min`.
- `harvest_action_def` no longer declares `reservation_requirements`; the temporal `try_reserve` substrate is incompatible with multi-slot parallel harvests on the same source. Slot occupancy is owned by `ResourceExtractionQueues`. Recorded as Assumption Reassessment item 16.
- The "enqueue when slots full" path persists writes via the failure handler (`record_harvest_start_failure`) on a fresh transaction, because `start_gate.rs` drops the start-handler txn on `Err`. Sentinel `PreconditionFailed("extraction_slots_full")` distinguishes the slots-full case from other harvest start failures. Recorded as Assumption Reassessment item 18.
- `BlockerMemory` semantics for the multi-slot full case map to `BlockingFact::ReservationConflict`, preserving the prior single-slot-temporal-reservation classification — observable downstream in `classify_production_failure` (`failure_handling.rs`) and the `classify_precondition_failure_detail` arm.

## Verification Result

- Passed `cargo test -p worldwake-systems --lib harvest_start` (4/4 ok — multi-slot start tests).
- Passed `cargo test -p worldwake-ai --lib candidate_gen_` (3/3 ok — quantity emission, horizon-gate, fallback).
- Passed `cargo test -p worldwake-ai --lib motive_score_falls_back` (1/1 ok — ranker tiebreak via existing discount).
- Passed `cargo test -p worldwake-ai --test golden_ai_decisions golden_local_depleted_source` (1/1 ok — depleted-source regeneration golden, after harness fixture migration).
- Passed `cargo test -p worldwake-cli --test observer_decision_history` (1/1 ok — multi-agent decision-history snapshot).
- Passed `cargo test --workspace` — full workspace green.
- Passed `cargo clippy --workspace --all-targets -- -D warnings`.
- Passed `./scripts/verify.sh` end-to-end (fmt-check, workspace tests, both clippy variants, scenario-coverage --check; exit 0).
