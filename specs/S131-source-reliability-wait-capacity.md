# S131: Source Reliability Wait and Capacity Extension

## Summary

Extend the existing `SourceReliability` per-agent memory so agents who repeatedly contend for a resource at a given source can learn to expect waits and weight alternatives accordingly. Today, `ReliabilityRecord { successful_acquisitions, failed_attempts, last_attempt_tick }` captures success/failure ratio per (entity, commodity) — useful for "this well usually works" but blind to two operationally critical signals: how *long* the agent typically waited for access, and how *much* the source typically yielded. The narrative report shows Agent A and B competing at North Orchard with `BlockingFact(ReservationConflict)` events at ticks 7, 65, 66, 408, 1085 — five contention events the agent today cannot use to learn "this source is reliably contested in the morning, plan around it." The fix is concrete: add `average_wait_ticks` (running mean of observed wait time), `last_observed_capacity` (most recent perception-time `available_quantity`), and `freshness_decay_per_tick` so stale records lose weight before being evicted. The planner ranking gains a wait/capacity-aware tiebreak when more than one believed source could satisfy an `AcquireCommodity` goal — directly enabling the "repeated-game intelligence" PR-8 calls out without introducing speculative `SurvivalHabit` infrastructure.

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft.

## Crates

- `worldwake-core` — `ReliabilityRecord` field extensions (`average_wait_ticks: u32`, `wait_observation_count: u32`, `last_observed_capacity: u16`, `last_observed_capacity_tick: Tick`); no new component.
- `worldwake-systems` — wait observation hook in the contention queue handler: when an agent's queued reservation transitions from queued to granted, compute `wait_ticks = grant_tick - queue_enter_tick` and write into the agent's `SourceReliability`. Capacity observation hook in perception: when the agent perceives a `ResourceSource`, update `last_observed_capacity` for the (source, commodity) key.
- `worldwake-ai` — ranking integration: when multiple `AcquireCommodity` plans target different believed sources, the per-source wait/capacity signals feed into the existing source-trust ranking weight.
- `worldwake-cli` — no scenario authoring (per-agent runtime memory; existing `PreferenceProfile.source_trust_weight` is the sole tunable surface).

## Dependencies

- E07 / E14 / E15 — **completed**. `SourceReliability` and `PreferenceProfile` already exist in `crates/worldwake-core/src/experience.rs`.
- S08 / S60 (Contention Queue and Site Occupancy) — **completed**. The contention queue's grant transitions are the wait-observation source.
- S127 (Quantity-Aware Acquisition and Visible Source State) — **soft**. S127's `LastHarvestTrace` provides peer-observable evidence of source contention; this spec's per-agent memory builds on that observable substrate. Without S127, capacity-side learning still works (perception of `ResourceSource.available_quantity`); wait-side learning works whether S127 lands or not.
- S110 (Decision History Events) — **soft**. No new event tag — the existing `EventTag::QueueGrantPromoted` already records grant transitions; this spec adds an agent-side learning hook on the same transition.

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 8 (and related PR-5 framing): "Agent A repeatedly loses orchard access to Agent B … `Habit: harvest earlier when hunger projection crosses medium`." The narrative report confirms: "Three are visible in Section 3 (ticks 7, 65/66, 1085); twenty distinct `AcquireCommodity { Apple }` desires appear in the fully-blocked-desires count." The repeated-game pattern exists in the live data; the agent's learning state today does not capture it. PR-8's broader habit proposal is rejected as speculative; this narrowed extension lands the lawful learning surface (`average_wait_ticks`, `last_observed_capacity`) that future spec work can build on if a concrete habit consumer emerges.

## Design Goals

1. Field extension only — no new component, no new event. The existing `SourceReliability` is extended in-place. This is the minimum FND-30-compliant surface for the learning extension.
2. `average_wait_ticks` is a running mean. Updated as `(prev_mean × count + new_observation) / (count + 1)`, with `wait_observation_count` capped at 32 (after which new observations replace via exponential moving average with `α = 1/32`). Integer arithmetic only.
3. `last_observed_capacity` is observation-driven. Updated whenever the agent perceives the `ResourceSource`. Decays in *relevance* (older observations are weighted less in ranking) but not in stored value — the value remains readable until the next observation supersedes it.
4. Ranking integration is additive to the existing `source_trust_weight`. Existing failure-ratio scoring continues to work; the new fields enter the same composite score.
5. Capacity decay is freshness-based, not value-based. The stored `last_observed_capacity` is the agent's last actual observation; ranking discounts it based on `current_tick - last_observed_capacity_tick`. This preserves FND-29A (history is append-only) — we don't rewrite the agent's belief, we discount its weight.
6. Per FND-22, all per-agent variation continues through `PreferenceProfile`. No new profile component; one new field `wait_sensitivity_weight: Permille` on the existing struct.

## Non-Goals

- `SurvivalHabit` with `trigger_condition`, `preferred_response`, `strength`, `origin_event_ids`. The assessment proposes this; rejected as speculative — no concrete consumer exists today, and emergent habit-like behavior should fall out of S126 + S127 + S130 + S131 acting in concert before authoring a new substrate.
- `BlockedIntentRecord` as a separate type from `BlockerMemory`. S109's `BlockerMemory` already records blocker class, key, observed/expires ticks, baseline snapshot. The fields PR-8 names that S109 lacks (`target_entity_id`, `blocker_agent_id`, `resolved_tick`, `chosen_fallback`) belong on `BlockerMemory` if anywhere — and adding them is a separate concern from this spec's source-reliability extension.
- Cross-agent reliability sharing. Agent A's reliability memory is private; cross-agent learning runs through `ShareBelief` per FND-15.
- Per-tick recomputation of `average_wait_ticks` against a sliding window. The exponential-moving-average pattern (capped count + α-blend) is FND-3-compliant (concrete state, not derived score) and computationally free.
- Capacity prediction (e.g., "this orchard usually has 5 apples by morning"). The agent stores `last_observed_capacity` as a single value; modeling capacity-over-time is deferred until a concrete consumer needs it.
- Hostile/non-hostile encounter reliability extension to `RouteExperience`. The same pattern would apply but routes are out of scope here.

## FOUNDATIONS Alignment

| Principle | How Satisfied |
|-----------|---------------|
| FND-3 (Concrete State Over Abstract Scores) | Each new field is a concrete observable quantity (wait ticks, observed capacity) or a concrete derived statistic (running mean), not an abstract reliability score. |
| FND-5 (Carriers of Consequence) | Wait and capacity memories carry consequence: agents who learn the orchard is reliably contested choose different acquisition strategies than agents who don't. |
| FND-7 (Locality of Motion, Interaction, and Communication) | All learning is per-agent. Cross-agent propagation requires `ShareBelief`. |
| FND-14A (Same-Tick Local Observation Is Belief-Equivalent) | Wait observation: the agent waited concretely (its own action history); the wait time is a fact about the agent's own activity. Capacity observation: co-located perception of the source's `available_quantity`. Both fall inside FND-14A. |
| FND-15 (Knowledge Is Acquired Locally and Travels Physically) | Wait and capacity memories carry source (the agent itself), tick, and confidence. Other agents learning these signals must hear them through `ShareBelief`. |
| FND-16 (Ignorance, Uncertainty, and Contradiction Are First-Class) | An agent with no prior observations of a source returns no reliability — ranking falls back to default ordering. Fresh observations supersede stale; the discrepancy between an old `last_observed_capacity` and new perception is a legitimate update. |
| FND-21 (Intentions Are Revisable Commitments) | An acquisition plan against a source with rising `average_wait_ticks` may be revised in favor of a less-contested alternative, even if the contested source has higher believed `available_quantity`. |
| FND-22 (Agent Diversity Through Concrete Variation) | `PreferenceProfile.wait_sensitivity_weight` is per-agent. Two agents with identical reliability memories rank differently because their patience differs. |
| FND-22A (Learning, Habits, and Preference Shifts Are Concrete State) | Wait and capacity learning are concrete agent state with explicit acquisition (queue grant or perception event), explicit decay (existing `memory_retention_ticks` + new freshness-relevance weighting), and explicit replacement (new observations overwrite). |
| FND-26 (Systems Interact Through State, Not Through Each Other) | Contention queue writes the grant transition; AI tick reads the grant + writes its own `SourceReliability`. No cross-system call. Perception writes belief; AI reads belief. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `average_wait_ticks` is stored as a running mean (concrete state); the per-tick freshness discount in ranking is derived. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The old `ReliabilityRecord` fields are kept but the type is extended; existing call sites read the same fields. New fields default to zero (no observations yet). |
| FND-29 (Debuggability Is a Product Feature) | Decision-trace surfaces "AcquireCommodity at North Orchard chosen over Camp Well: trust(950) vs trust(700) factoring wait(12 ticks at NO vs 0 at CW)." |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Wait and capacity observations are recorded in the agent's per-tick learning hooks; the existing `EventTag::QueueGrantPromoted` and perception events provide the authoritative causal history. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers all 18 declarations. |

## FND-01 Section H — Causal Hooks Declaration

1. **Information-path analysis.** Wait observation: agent's own queue-position history (FND-14A — facts about the agent's own activity). Capacity observation: agent's perception of co-located `ResourceSource.available_quantity`. Cross-agent transfer: existing `ShareBelief` channel.
2. **Positive-feedback analysis.** "Agent learns source is contested → avoids it → other agents have less competition → first agent's avoidance perpetuates." Dampener: avoidance also means no new observations refresh the memory; `memory_retention_ticks` (existing in `PreferenceProfile`) eventually evicts the record and the agent is willing to re-evaluate.
3. **Concrete dampeners.** (a) `memory_retention_ticks` per `PreferenceProfile` — existing. (b) Capacity observation freshness discount in ranking (new, derived per-tick). (c) Running-mean cap on `wait_observation_count = 32` keeps the EMA responsive — old observations don't dominate forever.
4. **Stored state vs. derived read-model.** Stored: `ReliabilityRecord.{average_wait_ticks, wait_observation_count, last_observed_capacity, last_observed_capacity_tick}`. Derived: ranking score per source per tick (composite of trust, wait, capacity).

## Deliverables

### D1: `ReliabilityRecord` field extension

In `crates/worldwake-core/src/experience.rs`:

```rust
pub struct ReliabilityRecord {
    pub successful_acquisitions: u16,
    pub failed_attempts: u16,
    pub last_attempt_tick: Tick,
    /// Running mean of observed wait ticks at this source.
    /// Updated via `average_wait_update` each grant.
    pub average_wait_ticks: u32,
    /// Capped at 32; after that, EMA replaces running mean.
    pub wait_observation_count: u32,
    /// Most recent perception-time `available_quantity`. Zero means
    /// either "never observed" (check `last_observed_capacity_tick`)
    /// or "observed empty."
    pub last_observed_capacity: u16,
    /// When `last_observed_capacity` was last refreshed by perception.
    /// Used for freshness discounting in ranking.
    pub last_observed_capacity_tick: Tick,
}

impl ReliabilityRecord {
    /// Update running mean of wait ticks. Caps `wait_observation_count`
    /// at 32; afterwards uses exponential moving average with α = 1/32.
    pub fn observe_wait(&mut self, wait_ticks: u32) {
        if self.wait_observation_count < 32 {
            let total = self.average_wait_ticks
                .saturating_mul(self.wait_observation_count)
                .saturating_add(wait_ticks);
            self.wait_observation_count += 1;
            self.average_wait_ticks = total / self.wait_observation_count;
        } else {
            // EMA: α = 1/32 → new = (31 × old + 1 × wait) / 32
            let blended = self.average_wait_ticks
                .saturating_mul(31)
                .saturating_add(wait_ticks);
            self.average_wait_ticks = blended / 32;
        }
    }

    pub fn observe_capacity(&mut self, capacity: u16, tick: Tick) {
        self.last_observed_capacity = capacity;
        self.last_observed_capacity_tick = tick;
    }
}
```

Existing call sites that construct `ReliabilityRecord` literally (test fixtures) are updated to default the new fields to zero.

### D2: Wait observation hook

In `crates/worldwake-systems/src/facility_queue_actions.rs`, when a grant transitions from queued to granted (the existing `QueueGrantPromoted` event emission site), look up the actor's queue-enter tick from the `ContentionQueue` waiter list, compute `wait_ticks = current_tick - queue_enter_tick`, and call `actor.source_reliability.observe_wait(wait_ticks)` for the source's (entity, commodity) key.

The source-and-commodity context comes from the queued action's payload: `queue_for_facility_use` carries the action it's queued *for*; if the queued action is `harvest`, the source's commodity is read from the source's `ResourceSource.commodity`.

### D3: Capacity observation hook

In `crates/worldwake-systems/src/perception.rs`, after the existing perception writes for a `ResourceSource`-bearing place, call `actor.source_reliability.observe_capacity(source.available_quantity, current_tick)` for each perceived `(source_entity, source.commodity)` key.

This piggybacks on the existing perception walk over co-located entities; no new system tick.

### D4: Ranking integration

In `crates/worldwake-ai/src/ranking.rs`, the `AcquireCommodity` ranking arm that already reads `PreferenceProfile.source_trust_weight` against `failure_ratio_permille` extends to:

```rust
let trust_score = (1000 - failure_ratio_permille(record)) * profile.source_trust_weight;
let wait_penalty = record.average_wait_ticks
    .saturating_mul(profile.wait_sensitivity_weight)
    / 1000;
let capacity_freshness = current_tick.0
    .saturating_sub(record.last_observed_capacity_tick.0);
let capacity_signal = if capacity_freshness > profile.memory_retention_ticks {
    0 // stale observation contributes nothing
} else {
    let freshness_factor = 1000 - (capacity_freshness * 1000 / profile.memory_retention_ticks);
    record.last_observed_capacity as u32 * freshness_factor / 1000
};
let composite = trust_score
    .saturating_sub(wait_penalty)
    .saturating_add(capacity_signal);
```

The composite score replaces the single-axis trust score in the existing source-tiebreak path. When only one source is believed, the score is informative for diagnostics but ranking outcome is unchanged.

### D5: `PreferenceProfile.wait_sensitivity_weight`

In the same `experience.rs`:

```rust
pub struct PreferenceProfile {
    pub route_caution_weight: Permille,
    pub source_trust_weight: Permille,
    pub route_memory_capacity: u32,
    pub source_memory_capacity: u32,
    pub memory_retention_ticks: u64,
    /// How strongly the agent weights expected wait time when
    /// choosing among believed sources of the same commodity.
    pub wait_sensitivity_weight: Permille,
}
```

Default `wait_sensitivity_weight = Permille::new_unchecked(150)` — modest baseline; per-agent tunable.

### D6: Decision-trace surfacing

The existing source-tiebreak ranking-trace lines extend with `(trust=X, wait=Y, cap=Z, composite=C)` per source. Observer Section 3 / Section 4 gain readable lines for "Agent A chose Camp Well over North Orchard: composite 880 vs 620, North Orchard wait_ticks=12 (16 observations)."

### D7: Golden coverage

Add `crates/worldwake-ai/tests/golden_source_reliability.rs`:

- Agent acquires water at the same well 5 times with wait_ticks `(0, 3, 5, 8, 12)`. Confirm `average_wait_ticks` matches the running mean.
- Two agents at one well; second agent has `wait_sensitivity_weight = 800`, prefers a slightly worse alternative well after 3 wait observations on the first.
- Capacity freshness: agent observes well at capacity 18 at tick 100, then waits 500 ticks (`memory_retention_ticks = 400`); confirm capacity signal in ranking falls to zero (stale).
- After 32 wait observations, the EMA replaces the running mean — confirm the 33rd observation shifts the average correctly per α = 1/32.

## SystemFn Integration

No new SystemFn. Wait observation lives in `facility_queue_actions.rs` (existing handler); capacity observation lives in `perception.rs` (existing handler). Memory eviction continues through `enforce_limits` per `PreferenceProfile.memory_retention_ticks`.

## Component Registration

No new components. `SourceReliability` already registered as universal per-agent (existing); new fields land in the existing component schema. `PreferenceProfile` already registered as universal; one new field added.

Per FND-22 Section 5: the new field `wait_sensitivity_weight` follows the universal pattern with `Default` impl.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Contention queue (S08) | Grant transitions trigger wait observation in agent's `SourceReliability` | State-mediated |
| Perception | Co-located observation of `ResourceSource` triggers capacity observation | State-mediated |
| AI ranking | Reads `SourceReliability` composite score for source tiebreak | State-mediated |
| Quantity-aware acquisition (S127) | `LastHarvestTrace` provides peer-observable contention evidence; `SourceReliability` consumes the perception-level signal | State-mediated |
| `ShareBelief` (existing) | Cross-agent reliability propagation runs through existing tell channels (no new propagation primitive) | State-mediated |

## Profile-Driven Parameters

Per-agent (universal `PreferenceProfile`):

- `source_trust_weight` — existing.
- `wait_sensitivity_weight` — new (default 150).
- `memory_retention_ticks` — existing; bounds capacity freshness window.
- `source_memory_capacity` — existing; bounds total reliability records.

No magic numbers introduced in agent-side code. The `wait_observation_count` cap of 32 is a structural choice (FND-3 — bounded running statistic) documented in code-side comments and discoverable via debug inspection; not a designer dial that changes drama.
