# S127: Quantity-Aware Acquisition and Visible Source State

## Summary

Make acquisition goals say *how much* the agent wants and over *what horizon*, and make resource sources expose the concrete state agents need to reason about depletion, recovery, and contention without asking the planner to read truth on their behalf. Today, `GoalKind::AcquireCommodity { commodity, purpose }` is a unit-quantity request — every agent wants exactly one apple at a time, repeated. `ResourceSource` exposes `available_quantity`, `max_quantity`, `regeneration_ticks_per_unit`, but exposes no slot/duration model and emits no `last_harvest_events`-style trace, so contention reasoning must run through the (lawful but invisible-to-other-agents) `BlockingFact(ReservationConflict)` path. This spec makes acquisition quantity-aware (`AcquisitionQuantity { desired_min, desired_target, horizon_ticks }`), exposes per-source extraction concurrency through new fields on `ResourceSource`, and turns "the source ran dry mid-harvest" into a partial-success outcome on the harvest action — the granular-aftermath piece of PR-11. Agents can then decide between "one apple now" vs. "three apples for the next 100 ticks" based on need projection (S126), source reliability (S131), and observed contention.

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft.

## Crates

- `worldwake-core` — `AcquisitionQuantity` struct on `GoalKind::AcquireCommodity`; new `extraction_slots: NonZeroU8` and `extraction_duration_ticks: NonZeroU32` fields on `ResourceSource`; `LastHarvestTrace` per-source ring buffer of recent harvest events (small bounded vec, append-on-harvest, prune-on-decay).
- `worldwake-systems` — harvest action handler updated to honor `extraction_slots` (existing single-slot semantics fall out as `extraction_slots = 1`), partial-success outcome path when source depletes mid-action, `LastHarvestTrace` append on commit, decay during the existing item-decay maintenance pass.
- `worldwake-ai` — candidate generation reads `AcquisitionQuantity` from goal seed (new factory takes need projection from S126 + carry capacity to choose `desired_target`); ranking uses S131 `SourceReliability.average_wait_ticks` to pick between sources when more than one is believed.
- `worldwake-cli` — `ResourceSourceDef` extended with `extraction_slots` (default 1) and `extraction_duration_ticks` (default = current `regeneration_ticks_per_unit` style fallback, but distinct), backwards-compatible RON omission.

## Dependencies

- S126 (Need Projection) — **soft**. The candidate generator computes `desired_target` from `need.until_tick - current_tick`. Without S126 it falls back to the current single-unit behavior; with S126 the quantity scales to projected horizon.
- S131 (Source Reliability Wait/Capacity) — **soft**. The ranker uses `average_wait_ticks` to choose between equally-believed sources. Without S131 it falls back to the existing first-believed-source heuristic.
- S106 (Ground Item Decay) — **completed**. `LastHarvestTrace` decay piggybacks on the existing item-decay maintenance pass (per-trace TTL, same FND-29A append-only model).
- S82 (Waste Disposal and Inventory Management) — **completed**. `FreeCarryCapacity` already exists; the candidate generator uses it to bound `desired_target` against carry headroom.
- E07 / E08 — **completed**. Resource source / harvest action substrate is in place.

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 2: "Eat and Drink currently work … harvest one, pick up, eat or drink, repeat … wells never exhaust, the orchard remains sufficient, agents rarely have reason to think beyond the next unit." The narrative report shows three agents each harvesting 12–17 apples and eating 24–33 across 1440 ticks — every harvest is a single unit. This forces the contention path through invisible blockers (Agent A and B repeatedly hit `BlockingFact(ReservationConflict)` at the orchard) instead of through visible "this source has one slot, queue" mechanics. The depth fix is *not* "make the orchard depletable" (it already is) but "let the agent decide to take three apples now because she expects to be away for the next 80 ticks."

## Design Goals

1. Quantity is part of the goal, not a separate post-selection decision. `AcquireCommodity` carries `AcquisitionQuantity { desired_min, desired_target, horizon_ticks }` so every consumer (ranker, search, action selector, decision trace) sees the same intent.
2. `desired_min` is a hard floor — below this the goal is not satisfied. `desired_target` is the preferred amount. The planner is allowed to terminate with quantity `>= desired_min`; the ranker prefers plans projected to deliver `desired_target`.
3. `horizon_ticks` makes the goal time-bounded. After `horizon_ticks` the goal expires (S109 TTL semantics — same suppression infrastructure). This prevents "I want three apples eventually" from haunting the decision trace forever.
4. `ResourceSource.extraction_slots` is concrete world state, not derived. A well with one bucket has `extraction_slots = NonZeroU8::new(1)`. A river bank where five agents can fill water at once has `extraction_slots = NonZeroU8::new(5)`. Today's single-slot behavior corresponds to `extraction_slots = 1`.
5. `extraction_duration_ticks` is the per-extraction time cost, distinct from `regeneration_ticks_per_unit` (which is recovery-side). A fast well has short extraction; a slow orchard tree has long extraction. This is the time another agent would have to wait if all slots are occupied — replacing the invisible reservation-conflict cooldown with concrete world-time cost.
6. Partial-success on harvest. If the source depletes mid-action (regeneration races didn't keep up, or another concurrent slot drew it down), the action commits with `partial_quantity` and emits the partial-harvest aftermath instead of failing the whole start.
7. `LastHarvestTrace` is observable world state. Co-located agents can perceive it (FND-14A) without it becoming a global event log query. Agents who are not co-located may learn through `ShareBelief` per existing channels.
8. No backward compatibility shim for the old quantity-implicit acquisition. Existing call sites are updated to construct `AcquisitionQuantity::single()` (helper) where the old behavior is intended; per-call-site review during implementation.

## Non-Goals

- `purpose: ReserveForSelf` as a distinct enum variant. The existing `Restock` purpose with `desired_target > 1` covers the same intent. Adding a new purpose would split the planner branch unnecessarily.
- "Acceptable travel cost" / "acceptable wait cost" as goal fields. Those are ranking inputs, not goal identity. The ranker reads the agent's `PreferenceProfile.source_trust_weight` and S131's `average_wait_ticks` without per-goal authoring.
- Time-to-digest / time-to-absorb on the consume side. That belongs in a follow-on consumption-mechanics spec; this spec stops at acquisition.
- Drinking → bladder side-effect latency. Same — S129 may revisit when authoring `WashBasinState.recovery_or_refill_process`, but PR-2's bladder-latency proposal doesn't have a concrete consumer yet (FND-5 / FND-30 test fails: no scenario class blocked by absence of latency).
- `last_observed_capacity` in `ReliabilityRecord`. That field belongs to S131 (source reliability extension), not here.
- Dynamic extraction slot count (e.g., a well that loses slots as it dries up). Static per-source authoring; dynamic shifts deferred until a concrete scenario needs them.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | `extraction_slots` and `extraction_duration_ticks` are concrete entity state, not derived "throughput score" abstractions. `LastHarvestTrace` is concrete event-aftermath state on the source entity. |
| FND-5 (Carriers of Consequence) | `LastHarvestTrace` is a new carrier of consequence — agents reasoning about whether a heavily-picked orchard is worth a trip can read it directly (FND-14A) instead of guessing from `available_quantity` alone. |
| FND-7 (Locality of Motion, Interaction, and Communication) | `LastHarvestTrace` is per-source state observable only by co-located agents through perception. Off-place propagation goes through existing `ShareBelief` / report channels. |
| FND-8 (Every Action Has Preconditions, Duration, Cost, and Occupancy) | `extraction_slots` makes the source's occupancy explicit. `extraction_duration_ticks` makes the time cost explicit. The waiting agent's projected delay is concrete world time, not an opaque blocker cooldown. |
| FND-10 (Outcomes Are Granular and Leave Aftermath) | Partial-harvest outcome is the canonical example: harvest completes with `quantity = 2` instead of failing because the source had 3 units when start checked but only 2 after concurrent draw. The aftermath: source drained, item lot of 2 created, `LastHarvestTrace` appended. |
| FND-11 (Every Positive Feedback Loop Needs a Physical Dampener) | "Hoard more apples → fewer trips → more apples available next time" is dampened by `FreeCarryCapacity` (existing per S82). "More agents queue at the source → wait longer → some agents leave" is dampened by `extraction_duration_ticks` (concrete waiting time) plus S131-driven preference shifts. |
| FND-14 (World State Is Not Belief State) | The candidate generator and ranker read the agent's belief about source state, not authoritative world state. The harvest action handler reads authoritative world state at execution time (this is correct — actions execute against world state, beliefs only inform planning). |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Co-located agents perceive `extraction_slots`, `extraction_duration_ticks`, `available_quantity`, and `LastHarvestTrace` on a `ResourceSource` directly through the existing FND-14A path (these are physical properties of the source, not social/relational facts). |
| FND-20 (Resource-Bounded Practical Reasoning Over Scripts) | `desired_target` is a per-agent reasoning input, not an authored "script for how to acquire apples." Different agents derive different targets from their need projections, carry capacity, and reliability memories. |
| FND-21 (Intentions Are Revisable Commitments) | An acquisition with `desired_target = 3` that completes with quantity 1 because the source depleted mid-harvest may be revised: if `desired_min = 1` was the floor and the agent's need projection is now safe through horizon, the goal is satisfied; otherwise the goal persists with a new `desired_target` accounting for the partial. |
| FND-22 (Agent Diversity Through Concrete Variation) | Two agents with the same hunger pressure produce different `desired_target` values because their `MetabolismProfile.hunger_rate` and `CarryCapacity.max_units` differ. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | `LastHarvestTrace` lets the agent learn "the orchard was heavily picked recently" from perception, not from a hidden tracker. S131 then promotes this into the agent's source-reliability memory. |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Production system writes `ResourceSource` and `LastHarvestTrace`; perception reads them; AI reads beliefs; AI writes goals; production action reads goals at start. No imperative cross-calls. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | Existing single-unit `AcquireCommodity` call sites are updated to construct `AcquisitionQuantity::single()`. The old quantity-implicit path is removed, not preserved beside the new one. |
| FND-29 (Debuggability Is a Product Feature) | Decision-trace records `(commodity, desired_min, desired_target, horizon_ticks)` per goal. Partial-harvest aftermath is named in the action commit trace. "Why did the agent only get 2 apples instead of 3?" is answerable from trace alone. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Partial-harvest outcomes go through the existing `EventTag::Inventory` / harvest-commit event surface; `LastHarvestTrace` appends are themselves authoritative state on the source. No erasure on partial completion — the partial-quantity event records what happened. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers all 18 declarations. |

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** Goal authoring reads agent-local state (need projection, carry capacity, source reliability) — no remote queries. Source state (`extraction_slots`, `extraction_duration_ticks`, `available_quantity`, `LastHarvestTrace`) is visible to co-located agents through FND-14A perception; non-co-located agents must learn through `ShareBelief` via existing channels. The action handler reads authoritative source state at execution time (correct per FND-26 — actions are the legal mutators).
2. **Positive-feedback analysis.** Two loops: (a) "Bigger `desired_target` → carry more → fewer trips → bigger `desired_target` next time." Dampener: `FreeCarryCapacity` (S82) bounds carry headroom; large carrying delays travel via `MetabolismProfile.travel_*_multiplier`. (b) "Long extraction duration → other agents queue → fewer alternatives → more pressure on this source." Dampener: queue-waiting time is now concrete (`extraction_duration_ticks` × `queue_position`), giving agents an explicit cost to weigh against alternatives; S131's reliability memory then biases away from chronically-contended sources.
3. **Concrete dampeners.** (a) `CarryCapacity.max_units` and the existing `MetabolismProfile.travel_*_multiplier` family — both already in the codebase. (b) `extraction_duration_ticks` itself, a per-source stored value. (c) S131-driven `average_wait_ticks` ranking shift. No numeric clamp does design work.
4. **Stored state vs. derived read-model.** Stored: `AcquisitionQuantity` fields on `GoalKind::AcquireCommodity`; `extraction_slots`, `extraction_duration_ticks`, `LastHarvestTrace` on `ResourceSource`. Derived: `expected_completion_quantity` (planner-computed against current source state and concurrent slot occupancy); `wait_estimate_ticks` (planner-computed from `extraction_duration_ticks × queue_position`).

## Deliverables

### D1: `AcquisitionQuantity` struct

In `crates/worldwake-core/src/goal.rs`:

```rust
/// Quantity intent on an `AcquireCommodity` goal. The goal is satisfied
/// when the agent has obtained at least `desired_min` units; the planner
/// prefers plans projected to deliver `desired_target`. The goal expires
/// after `horizon_ticks` ticks via the existing S109 TTL machinery.
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

The variant is no longer `Copy` if `AcquisitionQuantity` is `Copy`. Confirm `AcquisitionQuantity` derives `Copy` so `GoalKind` retains `Copy`.

### D3: `ResourceSource` extension

In `crates/worldwake-core/src/production.rs`:

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

### D4: `LastHarvestTrace` component on resource source entities

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

Cap `entries.len()` at 8 (per-source bounded ring); on overflow, drop the oldest by `tick`. Pruning rule: entries older than `current_tick - HARVEST_TRACE_RETENTION_TICKS` are removed during the item-decay maintenance pass. `HARVEST_TRACE_RETENTION_TICKS` is `200` (matches existing evidence-decay default; tunable per-scenario via `ScenarioDef.harvest_trace_retention_ticks` if a future scenario needs it).

### D5: Harvest action partial-success path

In the harvest commit handler (`crates/worldwake-systems/src/production_actions.rs`):

- If `available_quantity` at commit time is less than the requested quantity but ≥ 1, commit a partial outcome: produce an `ItemLot` of `available_quantity`, set source `available_quantity = 0`, append `HarvestTraceEntry { harvester, tick, quantity: actual, partial: true }`.
- If `available_quantity` at commit time is `0`, fail with `ActionError::PreconditionFailed("source depleted during action")` and append `HarvestTraceEntry { harvester, tick, quantity: 0, partial: true }`.
- The `CommitOutcome` carries the partial flag so the AI tick step records the actual quantity in the agent's inventory and the planner sees the partial completion.

The existing single-quantity behavior corresponds to requesting 1 unit and either succeeding or failing.

### D6: Multi-slot extraction

The existing reservation/contention path continues to govern same-slot conflicts. With `extraction_slots > 1`, multiple agents may hold concurrent extraction reservations on the same source. The reservation key is widened to `(source, slot_index)`; the contention queue per source becomes per-slot. The existing `ContentionQueue` substrate already handles this via per-target queuing (S60 references `OccupancyClaim` for the parallel site case). Implementation note: a `ContentionQueue` per `(source, slot_index)` pair is the path of least friction — the source carries `Vec<ContentionQueue>` of length `extraction_slots`.

### D7: Candidate generation — `desired_target` derivation

In `crates/worldwake-ai/src/agent_tick/candidate_generation.rs` (or the relevant goal-seed generator), the `AcquireCommodity` factory takes the agent's current `(HomeostaticNeeds, MetabolismProfile, DriveThresholds, CarryCapacity)` and computes:

```rust
let need_pressure = needs.value(need).value();
let high = thresholds.high(need).value();
let projected_breach = needs.projected_tick_of(need, thresholds.high(need),
    metabolism.rate(need), current_tick);
let horizon = projected_breach.map(|t| t.0.saturating_sub(current_tick.0))
    .unwrap_or(DEFAULT_ACQUISITION_HORIZON);
let units_needed = ceil_div(horizon as u32 * metabolism.rate(need).value() as u32,
    consumable.units_per_satiation_unit());
let target = units_needed.min(carry.headroom_for(commodity));
```

Without S126 the path falls back to `desired_target = 1`. With S126 the projection drives the target.

### D8: CLI scenario authoring

In `crates/worldwake-cli/src/scenario/types.rs`, `ResourceSourceDef` gains:

```rust
pub struct ResourceSourceDef {
    // ... existing fields ...
    #[serde(default = "default_extraction_slots")]
    pub extraction_slots: u8,
    #[serde(default = "default_extraction_duration_ticks")]
    pub extraction_duration_ticks: u32,
}
```

Defaults: `extraction_slots = 1`, `extraction_duration_ticks = 1`. Existing `scenarios/*.ron` files require no change.

### D9: Decision-trace surfacing

The existing `AcquireCommodity` decision-trace lines (already emit `commodity` and `purpose`) add `desired_min`, `desired_target`, `horizon_ticks`. Partial-harvest outcomes appear in the action-commit trace with `quantity_actual / quantity_requested`.

### D10: Golden coverage

Add `crates/worldwake-ai/tests/golden_quantity_aware_acquisition.rs`:

- Scenario authoring `extraction_slots = 1` and three agents racing at one well — confirm queue forms via `ContentionQueue` and wait time is `extraction_duration_ticks × queue_position`.
- Scenario authoring `extraction_slots = 3` and three agents harvesting concurrently — confirm all three get water without queuing.
- Scenario where source depletes mid-second-harvest — confirm partial-success outcome (quantity 1 instead of 3 requested) and `LastHarvestTrace` records the partial.
- With S126 enabled (long horizon agent), confirm `desired_target` scales above 1 when need projection demands.

## SystemFn Integration

No new system tick. Harvest action handler updated in place. `LastHarvestTrace` decay piggybacks on the existing `item_decay_system` maintenance pass (same FND-29A append-only model).

## Component Registration

| Component | EntityKind | Classification | Default |
|-----------|-----------|----------------|---------|
| `LastHarvestTrace` | Place / Workstation (wherever `ResourceSource` lives today) | Role-specific | `Default` (empty) — only sources that have been used |

`ResourceSource` field extensions are inline (no new component registration). `AcquisitionQuantity` is a value type embedded in the goal variant, not a component.

Per FND-22 Section 5: no new agent profile component — `desired_target` derivation reads existing `MetabolismProfile`, `DriveThresholds`, `CarryCapacity`, `PreferenceProfile`. No agent-side authoring needed.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Production / Harvest action | Reads `extraction_slots`, `extraction_duration_ticks`; writes `LastHarvestTrace` | State-mediated |
| Contention (S08, ContentionQueue) | Per-slot queue per source; existing queue substrate handles ordering | State-mediated |
| Item decay (S106) | Prunes `LastHarvestTrace` entries past retention tick during maintenance pass | State-mediated |
| Need projection (S126) | Provides `until_tick` driving `desired_target` derivation | State-mediated |
| Source reliability (S131) | Reads `LastHarvestTrace` arrival pattern to update `average_wait_ticks` | State-mediated |
| Perception | Co-located agents observe `extraction_slots`, `extraction_duration_ticks`, `available_quantity`, `LastHarvestTrace` (FND-14A) | State-mediated |
| Decision history (S110) | `EventTag::Inventory` / harvest-commit events carry partial-quantity flag | State-mediated |

## Profile-Driven Parameters

Per-agent variation comes from existing profiles:

- `MetabolismProfile.{need}_rate` — drives `desired_target` scaling.
- `DriveThresholds.{need}.high()` — defines the projection target.
- `CarryCapacity.max_units` — bounds the upper edge of `desired_target`.
- `PreferenceProfile.source_trust_weight` — biases between sources of equal believed availability.

Per-source authoring lives in scenario RON via `ResourceSourceDef.{extraction_slots, extraction_duration_ticks}`. Shared default value (`HARVEST_TRACE_RETENTION_TICKS = 200`) is overridable at scenario level.

No magic numbers introduced in agent-side code — all numeric authoring runs through the profile or scenario surface.
