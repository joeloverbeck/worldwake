# S127: Quantity-Aware Acquisition and Visible Source State

## Summary

Make acquisition goals say *how much* the agent wants and over *what horizon*, and make resource sources expose the concrete state agents need to reason about depletion, recovery, and contention without asking the planner to read truth on their behalf. Today, `GoalKind::AcquireCommodity { commodity, purpose }` is a unit-quantity request — every agent wants exactly one apple at a time, repeated. `ResourceSource` exposes `available_quantity`, `max_quantity`, `regeneration_ticks_per_unit`, but exposes no slot/duration model and emits no `last_harvest_events`-style trace, so contention reasoning must run through the (lawful but invisible-to-other-agents) `BlockingFact(ReservationConflict)` path. This spec makes acquisition quantity-aware (`AcquisitionQuantity { desired_min, desired_target, horizon_ticks }`), exposes per-source extraction concurrency through new fields on `ResourceSource` and a new `ResourceExtractionQueues` component, and turns "the source ran dry mid-harvest" into a partial-success outcome surfaced through `CommitTraceData` and a new partial-aware `Materialization` representation. Agents can then decide between "one apple now" vs. "three apples for the next 100 ticks" based on need projection (S126), source reliability (S131), and observed contention.

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft.

## Crates

- `worldwake-core` — `AcquisitionQuantity` struct on `GoalKind::AcquireCommodity`; new `extraction_slots: NonZeroU8` and `extraction_duration_ticks: NonZeroU32` fields on `ResourceSource`; new `LastHarvestTrace` per-source ring buffer of recent harvest events (small bounded vec, append-on-harvest, prune-on-decay); new `ResourceExtractionQueues` component holding `Vec<ContentionQueue>` of length `extraction_slots`.
- `worldwake-sim` — new `GoalBeliefView` accessor `last_harvest_trace(entity) -> Option<LastHarvestTrace>`, with `RuntimeBeliefView` impl and `impl_goal_belief_view!` macro forwarding (existing `resource_source(entity)` accessor at `belief_view.rs:417` already surfaces the new `ResourceSource` field additions for free). `CommitTraceData` extended with a `partial_quantity: Option<Quantity>` field so the harvest commit handler can surface partial-completion through the existing trace surface without modifying `CommitOutcome`.
- `worldwake-systems` — harvest action handler updated to honor `extraction_slots` (existing single-slot semantics fall out as `extraction_slots = 1`), partial-success outcome path when source depletes mid-action, `LastHarvestTrace` append on commit, decay during the existing item-decay maintenance pass.
- `worldwake-ai` — candidate generation reads `AcquisitionQuantity` from goal seed (the factory takes need projection from S126 + carry capacity to choose `desired_target`); ranking uses S131 `SourceReliability.average_wait_ticks` to pick between sources when more than one is believed; payload-widening migration touches `goal_dispatch_decl.rs`, `goal_model.rs` (all 12 `GoalKindPlannerExt` methods, including the `is_satisfied` semantic change to compare against `desired_min`), `feasibility.rs`, and `ranking.rs`.
- `worldwake-cli` — `ResourceSourceDef` extended with `extraction_slots` (default 1) and `extraction_duration_ticks` (default 1), backwards-compatible RON omission.

## Dependencies

- S126 (Need Projection) — **soft**, archived at `archive/specs/S126-need-projection-time-budget.md`. The candidate generator computes `desired_target` from `needs.projected_tick_of(need, threshold, rate, current_tick) - current_tick`. Without S126 the path falls back to `desired_target = 1`, `desired_min = 1`; with S126 the projection drives the target.
- S131 (Source Reliability Wait/Capacity) — **soft**, draft at `specs/S131-source-reliability-wait-capacity.md`. The ranker uses `average_wait_ticks` to choose between equally-believed sources. Without S131 it falls back to the existing `successful_acquisitions / (successful_acquisitions + failed_attempts)` ratio on `ReliabilityRecord` (`crates/worldwake-core/src/experience.rs:84`).
- S106 (Ground Item Decay) — **completed**, archived at `archive/specs/S106-ground-item-decay.md`. `LastHarvestTrace` decay piggybacks on the existing `item_decay_system` maintenance pass (`crates/worldwake-systems/src/item_decay.rs:6-25`), per-trace TTL, same FND-29A append-only model.
- S82 (Waste Disposal and Inventory Management) — **completed**, archived at `archive/specs/S82-waste-disposal-inventory-management.md`. S82 introduced `GoalKind::FreeCarryCapacity` (the *goal* of dropping inventory) and the disposal action stack; this spec uses `CarryCapacity.0` (`crates/worldwake-core/src/production.rs:69`, a wrapper over `LoadUnits`) plus believed inventory load to bound `desired_target` against carry headroom — the headroom is computed at emission time, not read from a pre-existing accessor.
- E07 / E08 — **completed**. Resource source / harvest action substrate is in place.

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 2: "Eat and Drink currently work … harvest one, pick up, eat or drink, repeat … wells never exhaust, the orchard remains sufficient, agents rarely have reason to think beyond the next unit." The narrative report shows three agents each harvesting 12–17 apples and eating 24–33 across 1440 ticks — every harvest is a single unit. This forces the contention path through invisible blockers (Agent A and B repeatedly hit `BlockingFact(ReservationConflict)` at the orchard) instead of through visible "this source has one slot, queue" mechanics. The depth fix is *not* "make the orchard depletable" (it already is) but "let the agent decide to take three apples now because she expects to be away for the next 80 ticks."

## Design Goals

1. Quantity is part of the goal, not a separate post-selection decision. `AcquireCommodity` carries `AcquisitionQuantity { desired_min, desired_target, horizon_ticks }` so every consumer (ranker, search, action selector, decision trace) sees the same intent.
2. `desired_min` is a hard floor — below this the goal is not satisfied. `desired_target` is the preferred amount. The planner is allowed to terminate with quantity `>= desired_min`; the ranker prefers plans projected to deliver `desired_target`. Invariant: `desired_min <= desired_target` (enforced at construction).
3. `horizon_ticks` is a candidate-emitter input, not a goal-level TTL. The candidate emitter only emits `AcquireCommodity` while `current_tick + horizon_ticks` covers the projected need-breach tick; once the projection passes, the emitter stops emitting and the goal naturally falls out of selection. The field stays on `AcquisitionQuantity` for decision-trace surfacing (FND-29) so debug can ask "why did the agent want three apples?". No new TTL infrastructure is added.
4. `ResourceSource.extraction_slots` is concrete world state, not derived. A well with one bucket has `extraction_slots = NonZeroU8::new(1)`. A river bank where five agents can fill water at once has `extraction_slots = NonZeroU8::new(5)`. Today's single-slot behavior corresponds to `extraction_slots = 1`.
5. `extraction_duration_ticks` is the per-extraction time cost, distinct from `regeneration_ticks_per_unit` (which is recovery-side). A fast well has short extraction; a slow orchard tree has long extraction. This is the time another agent would have to wait if all slots are occupied — replacing the invisible reservation-conflict cooldown with concrete world-time cost.
6. Partial-success on harvest. If the source depletes mid-action (regeneration races didn't keep up, or another concurrent slot drew it down), the action commits with the actual quantity in the materialization and surfaces `partial_quantity` via `CommitTraceData` instead of failing the whole start.
7. `LastHarvestTrace` is observable world state. Co-located agents can perceive it (FND-14A) without it becoming a global event log query. Agents who are not co-located may learn through `ShareBelief` per existing channels.
8. No backward compatibility shim for the old quantity-implicit acquisition. All existing `GoalKind::AcquireCommodity` construction and destructure sites (~344 across the workspace) are updated as part of the migration deliverable.
9. `quantity` does not participate in goal identity. `GoalKey::from(GoalKind::AcquireCommodity { commodity, purpose, quantity })` ignores `quantity` — two acquisition goals with the same commodity and purpose share a key regardless of `desired_target`. Goal-level identity remains commodity + purpose so the planner does not double-emit.

## Non-Goals

- `purpose: ReserveForSelf` as a distinct enum variant. The existing `Restock` purpose with `desired_target > 1` covers the same intent. Adding a new purpose would split the planner branch unnecessarily.
- "Acceptable travel cost" / "acceptable wait cost" as goal fields. Those are ranking inputs, not goal identity. The ranker reads the agent's `PreferenceProfile.source_trust_weight` and S131's `average_wait_ticks` without per-goal authoring.
- Time-to-digest / time-to-absorb on the consume side. That belongs in a follow-on consumption-mechanics spec; this spec stops at acquisition.
- Drinking → bladder side-effect latency. Same — S129 may revisit when authoring `WashBasinState.recovery_or_refill_process`, but PR-2's bladder-latency proposal doesn't have a concrete consumer yet (FND-5 / FND-30 test fails: no scenario class blocked by absence of latency).
- `last_observed_capacity` in `ReliabilityRecord`. That field belongs to S131 (source reliability extension), not here.
- Dynamic extraction slot count (e.g., a well that loses slots as it dries up). Static per-source authoring; dynamic shifts deferred until a concrete scenario needs them.
- Goal-level TTL infrastructure. `horizon_ticks` is enforced at the candidate-emitter level (Design Goal 3); no new goal-expiry component or suppression infrastructure is introduced. If a future scenario requires hard goal-level TTL, that will be its own spec.
- Adding a `partial: bool` flag to `CommitOutcome` itself. Partial-quantity is surfaced through `CommitTraceData` (Design Goal 6) so the foundational `CommitOutcome` shape stays stable across all action handlers.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `extraction_slots` and `extraction_duration_ticks` are concrete entity state, not derived "throughput score" abstractions. `LastHarvestTrace` is concrete event-aftermath state on the source entity. |
| FND-5 (Carriers of Consequence) | `LastHarvestTrace` is a new carrier of consequence — agents reasoning about whether a heavily-picked orchard is worth a trip can read it directly (FND-14A) instead of guessing from `available_quantity` alone. |
| FND-7 (Locality of Motion, Interaction, and Communication) | `LastHarvestTrace` is per-source state observable only by co-located agents through perception. Off-place propagation goes through existing `ShareBelief` / report channels. |
| FND-8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) | `extraction_slots` makes the source's occupancy explicit. `extraction_duration_ticks` makes the time cost explicit. The waiting agent's projected delay is concrete world time, not an opaque blocker cooldown. The new `ResourceExtractionQueues` component makes per-slot contention explicit and inspectable. |
| FND-10 (Outcomes Are Granular and Leave Aftermath) | Partial-harvest outcome is the canonical example: harvest completes with `quantity = 2` instead of failing because the source had 3 units when start checked but only 2 after concurrent draw. The aftermath: source drained, item lot of 2 created, `LastHarvestTrace` appended, `partial_quantity` surfaced through `CommitTraceData`. |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | "Hoard more apples → fewer trips → more apples available next time" is dampened by `CarryCapacity.0` (capped by S82 substrate). "More agents queue at the source → wait longer → some agents leave" is dampened by `extraction_duration_ticks` (concrete waiting time) plus S131-driven preference shifts. |
| FND-14 (World State Is Not Belief State) | The candidate generator and ranker read the agent's belief about source state, not authoritative world state. The harvest action handler reads authoritative world state at execution time (this is correct — actions execute against world state, beliefs only inform planning). |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Co-located agents perceive `extraction_slots`, `extraction_duration_ticks`, `available_quantity`, `LastHarvestTrace`, and `ResourceExtractionQueues` on a `ResourceSource` directly through the existing FND-14A path (these are physical properties of the source, not social/relational facts). |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | `desired_target` is a per-agent reasoning input, not an authored "script for how to acquire apples." Different agents derive different targets from their need projections, carry capacity, and reliability memories. |
| FND-21 (Intentions Are Revisable Commitments) | An acquisition with `desired_target = 3` that completes with quantity 1 because the source depleted mid-harvest may be revised: if `desired_min = 1` was the floor and the agent's need projection is now safe through horizon, the goal is satisfied; otherwise the goal persists with a new `desired_target` accounting for the partial. |
| FND-22 (Agent Diversity Through Concrete Variation) | Two agents with the same hunger pressure produce different `desired_target` values because their `MetabolismProfile.{need}_rate` and `CarryCapacity.0` differ. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | `LastHarvestTrace` lets the agent learn "the orchard was heavily picked recently" from perception, not from a hidden tracker. S131 then promotes this into the agent's source-reliability memory. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Production system writes `ResourceSource`, `LastHarvestTrace`, and `ResourceExtractionQueues`; perception reads them; AI reads beliefs; AI writes goals; production action reads goals at start. No imperative cross-calls. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | All ~344 existing `GoalKind::AcquireCommodity { commodity, purpose }` destructure and construction sites are updated to the quantity-aware shape (D2). The old quantity-implicit path is removed, not preserved beside the new one. `CommitOutcome` shape stays stable; partial-quantity uses the existing `CommitTraceData` extension surface, avoiding a foundational-type shim. |
| FND-29 (Debuggability Is a Product Feature) | Decision-trace records `(commodity, desired_min, desired_target, horizon_ticks)` per goal. Partial-harvest outcomes surface `partial_quantity` via `CommitTraceData` in the action commit trace. "Why did the agent only get 2 apples instead of 3?" is answerable from trace alone. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Partial-harvest outcomes go through the existing `EventTag::Inventory` / harvest-commit event surface; `LastHarvestTrace` appends are themselves authoritative state on the source. No erasure on partial completion — the partial-quantity event records what happened. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers the four-item declaration matching the project convention (S82, S109, S126 pattern). |

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** Goal authoring reads agent-local state (need projection, carry capacity computed from `CarryCapacity.0` minus believed inventory load, source reliability) — no remote queries. Source state (`extraction_slots`, `extraction_duration_ticks`, `available_quantity`, `LastHarvestTrace`, `ResourceExtractionQueues`) is visible to co-located agents through FND-14A perception via the existing `GoalBeliefView::resource_source(entity)` accessor and the new `last_harvest_trace(entity)` and `resource_extraction_queues(entity)` accessors; non-co-located agents must learn through `ShareBelief` via existing channels. The action handler reads authoritative source state at execution time (correct per FND-26 — actions are the legal mutators).
2. **Positive-feedback analysis.** Two loops: (a) "Bigger `desired_target` → carry more → fewer trips → bigger `desired_target` next time." Dampener: `CarryCapacity.0` bounds carry headroom; large carrying delays travel via `MetabolismProfile.travel_*_multiplier`. (b) "Long extraction duration → other agents queue → fewer alternatives → more pressure on this source." Dampener: queue-waiting time is now concrete (`extraction_duration_ticks` × `queue_position`), giving agents an explicit cost to weigh against alternatives; S131's reliability memory then biases away from chronically-contended sources.
3. **Concrete dampeners.** (a) `CarryCapacity.0` and the existing `MetabolismProfile.travel_*_multiplier` family — both already in the codebase. (b) `extraction_duration_ticks` itself, a per-source stored value. (c) S131-driven `average_wait_ticks` ranking shift. No numeric clamp does design work.
4. **Stored state vs. derived read-model.** Stored: `AcquisitionQuantity` fields on `GoalKind::AcquireCommodity`; `extraction_slots`, `extraction_duration_ticks`, `LastHarvestTrace`, `ResourceExtractionQueues` on resource-source entities. Derived: `expected_completion_quantity` (planner-computed against current source state and concurrent slot occupancy); `wait_estimate_ticks` (planner-computed from `extraction_duration_ticks × queue_position`); per-emission `headroom = CarryCapacity.0 - believed_carried_load(commodity)` (recomputed each emission, not stored).

## Deliverables

### D1: `AcquisitionQuantity` struct

In `crates/worldwake-core/src/goal.rs`:

```rust
/// Quantity intent on an `AcquireCommodity` goal. The goal is satisfied
/// when the agent has obtained at least `desired_min` units; the planner
/// prefers plans projected to deliver `desired_target`. `horizon_ticks`
/// is consumed by the candidate emitter — it stops emitting when
/// `current_tick + horizon_ticks` no longer covers the projected
/// need-breach tick. The field is retained on the goal for decision-trace
/// surfacing (FND-29) so debug can answer "why did the agent want
/// three apples?".
///
/// Invariant: `desired_min <= desired_target`. Constructors enforce this.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct AcquisitionQuantity {
    pub desired_min: NonZeroU16,
    pub desired_target: NonZeroU16,
    pub horizon_ticks: NonZeroU32,
}

impl AcquisitionQuantity {
    /// Single-unit acquisition with a default 200-tick horizon — used
    /// where call-sites previously implied `quantity = 1`.
    #[must_use]
    pub const fn single() -> Self {
        Self {
            desired_min: NonZeroU16::MIN,
            desired_target: NonZeroU16::MIN,
            horizon_ticks: NonZeroU32::new(200).unwrap(),
        }
    }
}
```

### D2: `GoalKind::AcquireCommodity` extension

```rust
GoalKind::AcquireCommodity {
    commodity: CommodityKind,
    purpose: CommodityPurpose,
    quantity: AcquisitionQuantity,
}
```

`AcquisitionQuantity` derives `Copy` so `GoalKind` retains `Copy`. `GoalKey::from(GoalKind)` ignores the `quantity` field — goal identity remains `(commodity, purpose)` so the planner does not double-emit (Design Goal 9).

### D3: `GoalKind::AcquireCommodity` payload-widening migration

Adding `quantity` to the variant breaks all existing destructure and construction sites (~344 across the workspace). The migration touches:

- **`crates/worldwake-ai/src/goal_dispatch_decl.rs:738-746`** — three `GoalDispatchKey → GoalKind` constructors (`AcquireSelfConsume`, `AcquireRecipeInput`, `AcquireRestock`) need a default `AcquisitionQuantity::single()`.
- **`crates/worldwake-ai/src/goal_model.rs`** — all 12 `GoalKindPlannerExt` methods that destructure or match `AcquireCommodity` (current sites at lines 567, 611, 740, 797, 1074, 1279, 1320, 1362, 1496, 1760, 2357, 2377). Most can ignore the new field with `{ commodity, purpose, .. }` patterns. The exception is **`is_satisfied`** (current implementation at the matched line in goal_model.rs): semantics change from "agent has any of the commodity" to "agent has at least `desired_min` units of the commodity." The check reads the agent's believed inventory of `commodity` and compares against `desired_min.get()`.
- **`crates/worldwake-ai/src/feasibility.rs:1031, 1140`** — test-fixture constructions need updating.
- **`crates/worldwake-ai/src/ranking.rs`** — destructures and constructions at the ~30 sites listed in the codebase grep. Ranking reads `desired_target` for the new wait/capacity tiebreak (D8) and includes it in motive_score logging.
- **`crates/worldwake-ai/src/candidate_generation.rs:2972, 3036`** — emitters compute `AcquisitionQuantity` from agent state (D8) and embed it in the `GoalKind::AcquireCommodity` they emit.
- **`crates/worldwake-cli/src/display.rs`** — display formatting for `AcquireCommodity` includes the quantity tuple.
- **All goldens and unit tests under `crates/worldwake-ai/tests/`** that construct `AcquireCommodity` — replace with `AcquisitionQuantity::single()` for compile-time fixtures or per-scenario quantities for new goldens.

The migration is a single atomic change — per FND-28, no shim or alias is added.

### D4: `ResourceSource` extension

In `crates/worldwake-core/src/production.rs:74-83`:

```rust
pub struct ResourceSource {
    pub commodity: CommodityKind,
    pub available_quantity: Quantity,
    pub max_quantity: Quantity,
    pub regeneration_ticks_per_unit: Option<NonZeroU32>,
    pub last_regeneration_tick: Option<Tick>,
    /// How many actors can extract simultaneously. `1` matches existing
    /// single-slot behavior. Larger values represent shared affordances
    /// (a river bank, a wide orchard row).
    pub extraction_slots: NonZeroU8,
    /// Per-extraction time cost. The waiting agent's projected delay
    /// is `extraction_duration_ticks * queue_position`.
    pub extraction_duration_ticks: NonZeroU32,
}
```

### D5: `LastHarvestTrace` component on resource source entities

In `crates/worldwake-core/src/production.rs`:

```rust
/// Bounded ring of recent harvest events at this source. Decays
/// alongside other site evidence in the existing item-decay
/// maintenance pass.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct LastHarvestTrace {
    pub entries: Vec<HarvestTraceEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HarvestTraceEntry {
    pub harvester: EntityId,
    pub tick: Tick,
    pub quantity: u16,
    pub partial: bool,
}

impl Component for LastHarvestTrace {}
```

Cap `entries.len()` at 8 (per-source bounded ring); on overflow, drop the oldest by `tick`. Pruning rule: entries older than `current_tick - HARVEST_TRACE_RETENTION_TICKS` are removed during the existing `item_decay_system` maintenance pass (`crates/worldwake-systems/src/item_decay.rs:6-25`). `HARVEST_TRACE_RETENTION_TICKS = 200` is a new constant introduced by this spec — the value is anchored to the narrative observation horizon documented in `reports/proposed-gameplay-mechanic-changes.md` Section 2 (the "agent reasons about the next 80–100 ticks of need" framing). The constant is tunable per-scenario via `ScenarioDef.harvest_trace_retention_ticks` (`#[serde(default)]`) if a future scenario needs different decay.

### D6: `ResourceExtractionQueues` component

In `crates/worldwake-core/src/contention.rs` (alongside the existing `ContentionQueue` at `contention.rs:10-14`):

```rust
/// Per-slot contention queues for resource extraction. Length matches
/// the host source's `extraction_slots`. Each slot has its own queue
/// with the existing `ContentionQueue` substrate (FIFO ordinal, granted
/// holder, waiters by ordinal). Reservations key by (source_entity,
/// slot_index); the existing per-entity reservation registration paths
/// extend to accept a slot index.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResourceExtractionQueues {
    pub queues: Vec<ContentionQueue>,
}

impl Component for ResourceExtractionQueues {}
```

The component is registered on resource-source entities at scenario spawn (one queue per slot, all empty). Existing reservation-registration call sites that key by `entity` are updated to take an additional `slot_index: u8` parameter; the harvest action's start handler picks the lowest-index free slot when multiple are open. Per FND-26, this separates extraction-state (`ResourceSource`'s commodity/quantity fields) from reservation-state (`ResourceExtractionQueues`); both live on the source entity but are independently registered.

S131's wait observation hook (queue-enter-tick → grant-tick) reads from `ResourceExtractionQueues.queues[slot].granted` to identify the slot transition.

### D7: Harvest action partial-success path

In `crates/worldwake-systems/src/production_actions.rs:553-617` (the `commit_harvest` handler):

- The harvest action payload carries `requested_quantity: Quantity` (replacing the current fixed `output_quantity`). The commit handler computes `actual = min(source.available_quantity, requested_quantity)`.
- If `actual >= 1`: commit succeeds, produces an `ItemLot` of `actual` units (replacing the current fixed-quantity lot creation at line 602-609), sets `source.available_quantity -= actual`, appends `HarvestTraceEntry { harvester, tick, quantity: actual, partial: actual < requested_quantity }` to the source's `LastHarvestTrace`. The returned `CommitOutcome` carries `trace: Some(CommitTraceData { …, partial_quantity: Some(Quantity(actual)) })` if `actual < requested_quantity`, else `partial_quantity: None`.
- If `actual == 0`: fail with `ActionError::PreconditionFailed("source depleted during action")` and append `HarvestTraceEntry { harvester, tick, quantity: 0, partial: true }`.

`CommitTraceData` (in `crates/worldwake-sim/src/action_handler.rs` near `CommitOutcome:13-30`) gains a new `partial_quantity: Option<Quantity>` field. The field is `None` for all non-harvest commit handlers and for full-quantity harvest commits, so no other action handler needs modification beyond updating their `CommitTraceData` constructors with `partial_quantity: None` (or relying on `..Default::default()` if the type derives `Default`).

The AI tick step reads `partial_quantity` from the commit trace to record the agent's actual inventory delta and to feed `is_satisfied` (`desired_min` floor check) on the next planning tick. The single-quantity behavior for callers requesting 1 unit corresponds to `requested_quantity = 1`, succeed-or-fail.

### D8: Multi-slot extraction and candidate-generation quantity derivation

The existing reservation/contention path continues to govern same-slot conflicts. With `extraction_slots > 1`, multiple agents may hold concurrent extraction reservations on the same source by claiming different slot indices in `ResourceExtractionQueues.queues`. The harvest action's start handler scans `queues[..]` for the lowest-index slot whose `granted` is `None` (or whose `granted` matches the actor); if none are free, the actor enqueues at the slot with the shortest waiter list. The reservation-registration paths extend to take a `slot_index: u8` parameter alongside the existing entity key.

Candidate generation in `crates/worldwake-ai/src/candidate_generation.rs:2972` (and `:3036` for the second emitter site) computes `AcquisitionQuantity` from agent state:

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
// Headroom is computed at emission time, not read from a pre-existing
// accessor. CarryCapacity is a wrapper over LoadUnits at production.rs:69;
// believed carried load is summed from the agent's belief view of their
// inventory.
let headroom = carry_capacity.0.saturating_sub(believed_carried_load);
let units_needed = ceil_div(
    horizon * metabolism.rate(need).value() as u32,
    consumable.units_per_satiation_unit(),
);
let target = NonZeroU16::new(units_needed.min(headroom).min(u16::MAX as u32) as u16)
    .unwrap_or(NonZeroU16::MIN);
let quantity = AcquisitionQuantity {
    desired_min: NonZeroU16::MIN,
    desired_target: target,
    horizon_ticks: NonZeroU32::new(horizon.max(1)).unwrap(),
};
```

Without S126 the path falls back to `desired_target = NonZeroU16::MIN`, `desired_min = NonZeroU16::MIN`, `horizon_ticks = DEFAULT_ACQUISITION_HORIZON` (200 ticks). With S126 the projection drives the target.

The candidate emitter respects `horizon_ticks` by *not emitting* the goal when the projected need-breach is more than `horizon_ticks` ahead of `current_tick` — this is how `horizon_ticks` is enforced (Design Goal 3), without any goal-level TTL infrastructure.

Ranking integration (`crates/worldwake-ai/src/ranking.rs`): when multiple believed sources can satisfy an `AcquireCommodity`, the ranker reads `SourceReliability.average_wait_ticks` (S131) to bias selection toward lower-wait sources. Without S131 it falls back to `successful_acquisitions / (successful_acquisitions + failed_attempts)` ratio on the existing `ReliabilityRecord` fields (`crates/worldwake-core/src/experience.rs:84`).

### D9: `GoalBeliefView` accessor for `LastHarvestTrace`

In `crates/worldwake-sim/src/belief_view.rs` (alongside the existing `resource_source(entity)` accessor at line 417 and `source_reliability(agent)` at line 569):

- New trait method `fn last_harvest_trace(&self, entity: EntityId) -> Option<LastHarvestTrace>` on the appropriate sub-trait (likely `EntityBeliefView` to mirror the location accessor pattern).
- New `RuntimeBeliefView` impl reading `world.get_component_last_harvest_trace(entity).cloned()`, gated by FND-14A co-location check matching the existing `resource_source` impl.
- `impl_goal_belief_view!` macro forwarding for the new method.

The new `ResourceExtractionQueues` component is read through the existing `resource_source` accessor pattern (a parallel `resource_extraction_queues(entity)` accessor) — same FND-14A co-location gating.

### D10: CLI scenario authoring

In `crates/worldwake-cli/src/scenario/types.rs:500` (`ResourceSourceDef`), add:

```rust
pub struct ResourceSourceDef {
    // ... existing fields ...
    #[serde(default = "default_extraction_slots")]
    pub extraction_slots: u8,
    #[serde(default = "default_extraction_duration_ticks")]
    pub extraction_duration_ticks: u32,
}

fn default_extraction_slots() -> u8 { 1 }
fn default_extraction_duration_ticks() -> u32 { 1 }
```

Defaults: `extraction_slots = 1`, `extraction_duration_ticks = 1`. The 19 existing `scenarios/*.ron` files require no change (verified via `serde(default)` precedent on existing tunables in `ScenarioDef`).

`ScenarioDef` gains a `#[serde(default)]` `harvest_trace_retention_ticks: Option<u32>` field; if `Some`, overrides the global `HARVEST_TRACE_RETENTION_TICKS` constant for that scenario. The `spawn_scenario` translator constructs `ResourceSource` with `NonZeroU8::new(def.extraction_slots).unwrap_or(NonZeroU8::MIN)` and `NonZeroU32::new(def.extraction_duration_ticks).unwrap_or(NonZeroU32::MIN)`, and registers a fresh `ResourceExtractionQueues { queues: vec![ContentionQueue::default(); slot_count] }` on the source entity.

### D11: Decision-trace surfacing

The existing `AcquireCommodity` decision-trace lines (already emit `commodity` and `purpose`) add `desired_min`, `desired_target`, `horizon_ticks`. Partial-harvest outcomes appear in the action-commit trace via the new `CommitTraceData.partial_quantity` field, surfacing as `quantity_actual / quantity_requested` in the trace formatter.

### D12: Golden coverage

Add `crates/worldwake-ai/tests/golden_quantity_aware_acquisition.rs`:

1. Scenario authoring `extraction_slots = 1` and three agents racing at one well — confirm queue forms via `ResourceExtractionQueues.queues[0]` and wait time is `extraction_duration_ticks × queue_position`.
2. Scenario authoring `extraction_slots = 3` and three agents harvesting concurrently — confirm all three get water without queuing (each takes a different slot index).
3. Scenario where source depletes mid-second-harvest — confirm partial-success outcome (quantity 1 instead of 3 requested), `LastHarvestTrace` records the partial, `CommitTraceData.partial_quantity = Some(Quantity(1))`.
4. With S126 enabled (long horizon agent), confirm `desired_target` scales above 1 when need projection demands.
5. **FOUNDATIONS Scenario E coverage** — three agents queue at a single-slot source; one queued agent's hunger is satisfied by an alternative path (e.g., a separate item lot they pick up) and they abandon the queue; the next agent in line is granted the slot. Confirms FND-Section VI Scenario E (Competing Claimants → Queue or Race → Expiry/Prune → Next Actor Acts).

## SystemFn Integration

No new system tick. Harvest action handler updated in place. `LastHarvestTrace` decay piggybacks on the existing `item_decay_system` (`crates/worldwake-systems/src/item_decay.rs:6-25`) maintenance pass (same FND-29A append-only model).

## Authoritative-to-AI Impact Trace

Per CLAUDE.md's Authoritative-to-AI Impact Rule, this spec modifies action preconditions (D7 partial-success path), affordance generation (D6 multi-slot), and validation surface (`extraction_slots`). The 7-point checklist:

1. **`get_affordances` (`affordance_query.rs`)** — affordance query for `Harvest` exposes `extraction_slots` so the planner knows multiple parallel claims are possible against the same source. The affordance entry includes `slot_index` so search can plan against specific slots. Verified at implementation time by golden #2 (3-slot parallel harvest).
2. **`generate_candidates` (`candidate_generation.rs:2972, 3036`)** — emitters compute `AcquisitionQuantity` from agent state (D8) and embed it in the emitted `GoalKind::AcquireCommodity`. The emitter respects `horizon_ticks` by gating emission on projected breach within horizon (Design Goal 3).
3. **`search_plan` (search core)** — search respects `desired_min` as the termination floor: a plan that delivers `>= desired_min` units is a valid termination even if `< desired_target`. Search prefers plans projected to deliver `desired_target`. Verified by golden #3 (partial completion satisfies `desired_min = 1`).
4. **`BestEffort` action start (`tick_step.rs`)** — when all `extraction_slots` are occupied, the action start enqueues the actor at the shortest-waitlist slot rather than failing. The `BestEffort` mode permits queueing as a graceful fallback. Verified by golden #1 (queue formation).
5. **`handle_plan_failure` (`agent_tick.rs`)** — partial completion is *not* a plan failure: the commit succeeds and `is_satisfied` re-evaluates against `desired_min`. If the new inventory satisfies `desired_min`, the goal is done; otherwise the agent replans for the remainder. Verified by golden #3.
6. **Payload revalidation (`plan_revalidation.rs`)** — synthesized harvest payloads carrying `requested_quantity` register a payload override validator via `with_payload_override_validator` so `requested_affordance_matches` accepts the synthesized quantity. The validator confirms the requested quantity is within the source's `available_quantity` and within the agent's carry headroom at revalidation time.
7. **Goldens** — D12 covers the main paths (single-slot queue, parallel slots, partial-success, S126-driven target, FOUNDATIONS Scenario E).

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `LastHarvestTrace` | Place / Workstation (wherever `ResourceSource` lives today) | Role-specific | `Default` (empty) — only sources that have been used |
| `ResourceExtractionQueues` | Place / Workstation (same as `ResourceSource`) | Role-specific | Constructed at scenario spawn with `vec![ContentionQueue::default(); extraction_slots]` |

`ResourceSource` field extensions are inline (no new component registration). `AcquisitionQuantity` is a value type embedded in the goal variant, not a component.

Per FND-22 Section 5: no new agent profile component — `desired_target` derivation reads existing `MetabolismProfile`, `DriveThresholds`, `CarryCapacity`, `PreferenceProfile`. No agent-side authoring needed.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Production / Harvest action (`worldwake-systems/production_actions.rs`) | Reads `extraction_slots`, `extraction_duration_ticks`, `ResourceExtractionQueues`; writes `LastHarvestTrace`, `ResourceSource.available_quantity`, `CommitTraceData.partial_quantity` | State-mediated |
| Contention (existing `ContentionQueue` substrate) | Per-slot queue per source via `ResourceExtractionQueues.queues[slot]`; existing queue substrate handles ordering | State-mediated |
| Item decay (S106, `worldwake-systems/item_decay.rs`) | Prunes `LastHarvestTrace` entries past retention tick during maintenance pass | State-mediated |
| Need projection (S126) | Provides `projected_tick_of(need, threshold, rate, current_tick)` driving `horizon` and `desired_target` derivation | State-mediated |
| Source reliability (S131) | Reads `LastHarvestTrace` arrival pattern to update `average_wait_ticks`; ranking reads `average_wait_ticks` for tiebreak | State-mediated |
| Perception | Co-located agents observe `extraction_slots`, `extraction_duration_ticks`, `available_quantity`, `LastHarvestTrace`, `ResourceExtractionQueues` (FND-14A) via `GoalBeliefView` accessors | State-mediated |
| Decision history (S110) | `EventTag::Inventory` / harvest-commit events carry the partial-quantity flag through `CommitTraceData` | State-mediated |
| AI tick step (`worldwake-ai/agent_tick/tick_step.rs`) | Reads `CommitTraceData.partial_quantity` after harvest commit to record actual inventory delta and feed `is_satisfied` re-evaluation | State-mediated |

## Profile-Driven Parameters

Per-agent variation comes from existing profiles:

- `MetabolismProfile.{need}_rate` (`crates/worldwake-core/src/needs.rs:142`) — drives `desired_target` scaling.
- `DriveThresholds.{need}.high()` (`crates/worldwake-core/src/drives.rs:103`) — defines the projection target.
- `CarryCapacity.0` (`crates/worldwake-core/src/production.rs:69`, a wrapper over `LoadUnits`) — bounds the upper edge of `desired_target` via emission-time headroom computation. There is no `headroom_for(commodity)` accessor — headroom is computed inline as `CarryCapacity.0 - believed_carried_load` at emission time (per Design Goal — derived view, not stored).
- `PreferenceProfile.source_trust_weight` (`crates/worldwake-core/src/experience.rs:129`) — biases between sources of equal believed availability.

Per-source authoring lives in scenario RON via `ResourceSourceDef.{extraction_slots, extraction_duration_ticks}`. The shared default `HARVEST_TRACE_RETENTION_TICKS = 200` is overridable at scenario level via `ScenarioDef.harvest_trace_retention_ticks`.

No magic numbers introduced in agent-side code — all numeric authoring runs through the profile or scenario surface.
