# S131: Source Reliability Wait and Capacity Extension

## Summary

Extend the existing `SourceReliability` per-agent memory so agents who repeatedly contend for a resource at a given source can learn to expect waits and weight alternatives accordingly. Today, `ReliabilityRecord { successful_acquisitions, failed_attempts, last_attempt_tick }` captures success/failure ratio per (entity, commodity) — useful for "this well usually works" but blind to two operationally critical signals: how *long* the agent typically waited for access, and how *much* the source typically yielded. The narrative report shows Agent A and B competing at North Orchard with `BlockingFact(ReservationConflict)` events at ticks 7, 65, 66, 408, 1085 — five contention events the agent today cannot use to learn "this source is reliably contested in the morning, plan around it." The fix is concrete: add `average_wait_ticks` (running mean of observed wait time), `last_observed_capacity` (most recent perception-time `available_quantity`), and a freshness-relevance discount so stale records lose weight before being evicted. The planner ranking gains a wait/capacity-aware composite score on the existing per-candidate source-reliability discount path — directly enabling the "repeated-game intelligence" PR-8 calls out without introducing speculative `SurvivalHabit` infrastructure.

## Phase and Status

Phase 10: Survival Mechanic Depth (Adjunct). Status: Draft.

## Crates

- `worldwake-core` — `ReliabilityRecord` field extensions (`average_wait_ticks: u32`, `wait_observation_count: u32`, `last_observed_capacity: u16`, `last_observed_capacity_tick: Tick`); `ReliabilityRecord::new(last_attempt_tick: Tick)` constructor; `PreferenceProfile.wait_sensitivity_weight: Permille` field; updated `Default` impls. No new component.
- `worldwake-systems` — wait observation hooks in BOTH grant-promotion sites: `facility_queue.rs::promote_ready_head` (for `ContentionQueue` grants on facilities like wells) and `production_actions.rs::grant_or_signal_full` (for `ResourceExtractionQueues` grants on resource sources like orchards). Each hook reads the head waiter's `queued_at` BEFORE promotion, computes `wait_ticks = current_tick - queued_at`, and writes into the actor's `SourceReliability` for the (entity, commodity) key. Capacity observation hook in perception: when the agent perceives a `ResourceSource`, update `last_observed_capacity` for the (source, commodity) key.
- `worldwake-ai` — ranking integration: extend `apply_source_reliability_discount` (and its pending-failure variant) to compute a single composite (trust − wait_penalty + capacity_signal) on every per-candidate source-reliability evaluation, replacing the current failure-only discount.
- `worldwake-cli` — existing authored `AgentDef.preference_profile` blocks can tune the new `wait_sensitivity_weight`; agents without an authored profile inherit the universal `PreferenceProfile::default()` baseline.

## Dependencies

- E07 / E14 / E15 — **completed**. `SourceReliability` and `PreferenceProfile` already exist in `crates/worldwake-core/src/experience.rs`.
- S44 (Generalized Contention Substrate) — **completed**. `ContentionQueue` / `ContentionWaiter` (with `queued_at: Tick`) and `ResourceExtractionQueues` are the substrate this spec reads.
- S127 (Quantity-Aware Acquisition and Visible Source State) — **completed (soft)**. S127's `LastHarvestTrace` provides peer-observable evidence of source contention; this spec's per-agent memory builds on that observable substrate. Without S127, capacity-side learning still works (perception of `ResourceSource.available_quantity`); wait-side learning works regardless.
- S110 (Decision History Events) — **completed (soft)**. No new event tag — the existing `EventTag::QueueGrantPromoted` already records facility-queue grant transitions; the new resource-extraction wait hook does not require a new event tag (the grant is already recorded via the harvest start path).

## Motivating Evidence

`reports/proposed-gameplay-mechanic-changes.md` Section 8 (and related PR-5 framing): "Agent A repeatedly loses orchard access to Agent B … `Habit: harvest earlier when hunger projection crosses medium`." The narrative report confirms: "Three are visible in Section 3 (ticks 7, 65/66, 1085); twenty distinct `AcquireCommodity { Apple }` desires appear in the fully-blocked-desires count." The repeated-game pattern exists in the live data; the agent's learning state today does not capture it. PR-8's broader habit proposal is rejected as speculative; this narrowed extension lands the lawful learning surface (`average_wait_ticks`, `last_observed_capacity`) that future spec work can build on if a concrete habit consumer emerges.

## Design Goals

1. Field extension only — no new component, no new event. The existing `SourceReliability` is extended in-place. This is the minimum FND-30-compliant surface for the learning extension.
2. `average_wait_ticks` is a deterministic integer running estimate. Updated as `(prev_mean × count + new_observation) / (count + 1)`, with `wait_observation_count` capped at 32 (after which new observations replace via exponential moving average with `α = 1/32`). Integer arithmetic only; because no accumulated total is stored, the value may differ from the exact arithmetic mean after truncating intermediate observations.
3. `last_observed_capacity` is observation-driven. Updated whenever the agent perceives the `ResourceSource`. Decays in *relevance* (older observations are weighted less in ranking) but not in stored value — the value remains readable until the next observation supersedes it.
4. Ranking integration is a single composite computation on the existing per-candidate source-reliability path. The current failure-ratio discount is subsumed into the composite; existing trust-weight semantics are preserved.
5. Capacity decay is freshness-based, not value-based. The stored `last_observed_capacity` is the agent's last actual observation; ranking discounts it based on `current_tick - last_observed_capacity_tick`. This preserves FND-29A (history is append-only) — we don't rewrite the agent's belief, we discount its weight.
6. Per FND-22, all per-agent variation continues through `PreferenceProfile`. No new profile component; one new field `wait_sensitivity_weight: Permille` on the existing struct.
7. `observe_wait` and `observe_capacity` are inherent methods on `ReliabilityRecord` (`&mut self`). The existing `failure_ratio_permille` remains a free function (read-only computation over `&ReliabilityRecord`); the placement convention is "mutating helpers as methods, pure projections as free functions."

## Non-Goals

- `SurvivalHabit` with `trigger_condition`, `preferred_response`, `strength`, `origin_event_ids`. The assessment proposes this; rejected as speculative — no concrete consumer exists today, and emergent habit-like behavior should fall out of S126 + S127 + S130 + S131 acting in concert before authoring a new substrate.
- `BlockedIntentRecord` as a separate type from `BlockerMemory`. The existing `BlockerMemory` (`crates/worldwake-core/src/blocker_memory.rs`) already records blocker class, key, observed/expires ticks, baseline snapshot. The fields PR-8 names that `BlockerMemory` lacks (`target_entity_id`, `blocker_agent_id`, `resolved_tick`, `chosen_fallback`) belong on `BlockerMemory` if anywhere — and adding them is a separate concern from this spec's source-reliability extension.
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
| FND-26 (Systems Interact Through State, Not Through Each Other) | Contention queue (and resource extraction queue) handlers write the grant transition; AI tick reads the agent's `SourceReliability`. No cross-system call. Perception writes belief; AI reads belief. |
| FND-27 (Derived Summaries Are Caches, Never Truth) | `average_wait_ticks` is stored as a running mean (concrete state); the per-tick freshness discount in ranking is derived. |
| FND-28 (No Backward Compatibility in Live Authority Paths) | The old `ReliabilityRecord` fields are kept but the type is extended; existing call sites read the same fields. The composite-score change to `apply_source_reliability_discount` *replaces* the failure-only discount path — no parallel discount route is preserved. New fields default to zero for newly constructed records. Current save format advances when these persisted fields land; old saves are not migrated. |
| FND-29 (Debuggability Is a Product Feature) | The existing `SourceReliabilityDiscount` trace struct (`crates/worldwake-ai/src/decision_trace.rs:546`) is extended with the composite components; observer Section 3/4 surfaces the new fields per D6. |
| FND-29A (Causal History Is Authoritative, Append-Only, Queryable) | Wait observations fire on grant promotion sites that already emit authoritative grant transitions; capacity observations fire on perception-time belief writes. Both are tied to existing causal events. |
| FND-30 (Every New System Spec Must Declare Its Causal Hooks) | Section H below covers the 4 declarations relevant to this extension. |

## FND-01 Section H — Causal Hooks Declaration

This is a system extension, not a new system. The 18-point coverage required by FND-30 was established for `SourceReliability` and `PreferenceProfile` by E07 / E14 / E15. The four declarations below cover what this extension changes.

1. **Information-path analysis.** Wait observation: agent's own queue-position history (FND-14A — facts about the agent's own activity). The grant transition is observed by the actor itself at the moment of promotion. Capacity observation: agent's perception of co-located `ResourceSource.available_quantity`. Cross-agent transfer: existing `ShareBelief` channel.
2. **Positive-feedback analysis.** "Agent learns source is contested → avoids it → other agents have less competition → first agent's avoidance perpetuates." Dampener: avoidance also means no new observations refresh the memory; `memory_retention_ticks` (existing in `PreferenceProfile`) eventually evicts the record and the agent is willing to re-evaluate.
3. **Concrete dampeners.** (a) `memory_retention_ticks` per `PreferenceProfile` — existing. (b) Capacity observation freshness discount in ranking (new, derived per-tick). (c) Running-mean cap on `wait_observation_count = 32` keeps the EMA responsive — old observations don't dominate forever.
4. **Stored state vs. derived read-model.** Stored: `ReliabilityRecord.{average_wait_ticks, wait_observation_count, last_observed_capacity, last_observed_capacity_tick}`. Derived: composite ranking score per source per tick (combines trust, wait, capacity).

## Deliverables

### D1: `ReliabilityRecord` field extension and constructor

In `crates/worldwake-core/src/experience.rs`:

```rust
pub struct ReliabilityRecord {
    pub successful_acquisitions: u16,
    pub failed_attempts: u16,
    pub last_attempt_tick: Tick,
    /// Integer running estimate of observed wait ticks at this source.
    /// Updated via `observe_wait` each grant.
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
    /// Construct a fresh record at `last_attempt_tick` with all observation
    /// counters and capacity fields zero. Use this at runtime construction
    /// sites (e.g., `experience_recording.rs`, `ranking.rs`) so new fields
    /// default consistently. Test fixtures may continue to use struct-literal
    /// form when they need to set specific field values.
    pub fn new(last_attempt_tick: Tick) -> Self {
        Self {
            successful_acquisitions: 0,
            failed_attempts: 0,
            last_attempt_tick,
            average_wait_ticks: 0,
            wait_observation_count: 0,
            last_observed_capacity: 0,
            last_observed_capacity_tick: Tick(0),
        }
    }

    /// Update the integer running wait estimate. Caps `wait_observation_count`
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

**Call-site migration.** All current `ReliabilityRecord { ... }` literal-construction sites must be updated. The following runtime sites should migrate to `ReliabilityRecord::new(tick)` (preserving any field assignments that follow):

- `crates/worldwake-systems/src/experience_recording.rs:15` — `.or_insert(...)` in agent learning.
- `crates/worldwake-ai/src/ranking.rs:557` — default record in `apply_source_reliability_discount_with_pending_failures`.

Test fixtures (`crates/worldwake-core/src/experience.rs:238–262`, `crates/worldwake-core/src/test_utils.rs:146`, `crates/worldwake-systems/src/{trade_actions,production_actions}.rs` test sites, `crates/worldwake-ai/src/agent_tick/tests.rs`, `crates/worldwake-ai/src/agent_tick/mod.rs:2114`) may keep struct-literal form or use struct-update defaults when the test does not own wait/capacity values.

Style note: `observe_wait` and `observe_capacity` are inherent methods on `ReliabilityRecord` because they mutate `&mut self`. The existing `failure_ratio_permille(record: &ReliabilityRecord) -> u32` (free function in the same module) remains a free function — it is a pure projection over `&ReliabilityRecord`, not a mutation. The placement convention for this module is "mutating helpers as methods; pure projections as free functions."

### D2: Wait observation hooks (two grant paths)

The motivating scenario (orchard contention) and the analogous facility-queue case (well/forge contention) run through different grant substrates. Wait observation must fire in both.

**D2a: Facility-queue grant path** — in `crates/worldwake-systems/src/facility_queue.rs::promote_ready_head` (lines 315–367), the head waiter's `queued_at` is read at line 337 (`let Some(queued) = queue.waiting.values().next() else { return Ok(()); };`) BEFORE the `queue.promote_head(...)` call at line 351 removes the waiter. After confirming `head_is_ready_to_start` (line 340), capture `wait_ticks = tick - queued.queued_at` and the actor identity, then — after the existing `commit_queue_update` call that emits `EventTag::QueueGrantPromoted` (line 362) — look up the source's `(entity, commodity)` key (the facility entity itself, with the commodity derived from the queued action's payload — `harvest`-class actions read `ResourceSource.commodity` from the facility; non-harvest queued actions are skipped here because they have no commodity association) and invoke `actor.source_reliability.observe_wait(wait_ticks)` for that key on the actor's stored `SourceReliability`. Use the same world-transaction pattern as `commit_queue_update` to write the updated `SourceReliability` component back to the actor.

**D2b: Resource-extraction grant path** — in `crates/worldwake-systems/src/production_actions.rs::grant_or_signal_full` (lines 462–515), the head waiter for the chosen slot is read at line 484 (`queue.waiting.values().next()`) BEFORE `queue.remove_actor(actor)` at line 499. When the chosen actor was previously queued (i.e., `queue.granted.is_none()` and the head waiter equals `actor`), capture `wait_ticks = txn.tick() - head.queued_at` BEFORE the remove/grant mutation. After the existing `txn.set_component_resource_extraction_queues(...)` write at line 506, fetch the actor's `SourceReliability`, look up the `(workstation, source.commodity)` key (commodity comes from the workstation's `ResourceSource.commodity`), and invoke `observe_wait(wait_ticks)`, writing the updated component back through the same `txn`. When the actor takes a free slot with no prior queue position (head equals actor because `waiting` was empty), `wait_ticks = 0` and no observation is recorded (skip the call — recording zero waits dilutes the running mean for legitimate wait events).

In both paths, the `(entity, commodity)` key the spec uses for `SourceReliability.sources` is `SourceKey { entity: <facility-or-source-entity>, commodity: <derived from ResourceSource> }`, matching the existing key convention in `apply_source_reliability_discount`.

### D3: Capacity observation hook

In `crates/worldwake-systems/src/perception.rs`, after the existing perception writes for a `ResourceSource`-bearing place (line 1797 and the analogous belief-write sites at lines 4217, 4250 — confirm at implementation time which sites correspond to *fresh* perception of co-located resource sources versus belief-store maintenance), fetch the actor's `SourceReliability`, look up or insert the `SourceKey { entity: source_entity, commodity: source.commodity }` record (use `ReliabilityRecord::new(current_tick)` for inserts), and call `record.observe_capacity(source.available_quantity, current_tick)` — converting `Quantity` to `u16` via the existing conversion (or saturating cast if `Quantity` is wider) before storage. Write the updated `SourceReliability` back through the same world transaction.

This piggybacks on the existing perception walk over co-located entities; no new system tick.

### D4: Composite ranking integration

In `crates/worldwake-ai/src/ranking.rs`, restructure `apply_source_reliability_discount` (lines 419–452) and its variant `apply_source_reliability_discount_with_pending_failures` (lines 532–579) to compute a single composite score on every per-candidate source-reliability evaluation. The existing functions short-circuit when `failure_ratio == 0` (lines 436–438, 564–566) — that early-out is removed so wait_penalty and capacity_signal apply even when the agent has no failure history at this source.

Pseudocode for the restructured `apply_source_reliability_discount`:

```rust
fn apply_source_reliability_discount(
    candidate: &GoalOffer,
    context: &RankingContext<'_>,
    motive_score: u32,
) -> Option<SourceReliabilityDiscount> {
    if motive_score == 0 {
        return None;
    }

    let (source_entity, commodity) = source_reliability_discount_scope(candidate)?;
    let source_reliability = context.view.source_reliability(context.agent)?;
    let profile = context.view.preference_profile(context.agent)?;
    let record = source_reliability.sources.get(&SourceKey {
        entity: source_entity,
        commodity,
    })?;

    let trust_weight = u32::from(profile.source_trust_weight.value());
    let failure_ratio = failure_ratio_permille(record);
    let trust_discount = trust_weight.saturating_mul(failure_ratio) / 1000;

    let wait_weight = u32::from(profile.wait_sensitivity_weight.value());
    let wait_penalty = record.average_wait_ticks
        .saturating_mul(wait_weight) / 1000;

    let capacity_freshness = context.current_tick.0
        .saturating_sub(record.last_observed_capacity_tick.0);
    let capacity_signal = if capacity_freshness > profile.memory_retention_ticks {
        0 // stale observation contributes nothing
    } else if profile.memory_retention_ticks == 0 {
        u32::from(record.last_observed_capacity)
    } else {
        let freshness_factor = 1000_u64
            - (capacity_freshness as u64 * 1000 / profile.memory_retention_ticks);
        (u32::from(record.last_observed_capacity)
            .saturating_mul(freshness_factor as u32)) / 1000
    };

    // Composite: subtract trust-failure discount and wait penalty,
    // add capacity signal. All-zero observations leave motive unchanged.
    let post = motive_score
        .saturating_mul(1000u32.saturating_sub(trust_discount)) / 1000;
    let post = post.saturating_sub(wait_penalty).saturating_add(capacity_signal).max(1);

    if post == motive_score {
        // No-op composite — no observations of any kind; skip recording.
        return None;
    }

    Some(SourceReliabilityDiscount {
        source_entity,
        commodity,
        failure_ratio_permille: failure_ratio,
        average_wait_ticks: record.average_wait_ticks,
        wait_penalty,
        last_observed_capacity: record.last_observed_capacity,
        capacity_freshness_ticks: capacity_freshness,
        capacity_signal,
        pre_discount_motive: motive_score,
        post_discount_motive: post,
    })
}
```

The pending-failure variant `apply_source_reliability_discount_with_pending_failures` follows the same structure with the synthetic incremented `failed_attempts` retained from current behavior (line 562); compute `wait_penalty` and `capacity_signal` from the actual stored `record` (not the synthetic copy) since wait/capacity are independent of the pending failure count.

`SourceReliabilityDiscount` is the existing trace struct in `crates/worldwake-ai/src/decision_trace.rs:546–552`. Extend it in place with the new fields (`average_wait_ticks`, `wait_penalty`, `last_observed_capacity`, `capacity_freshness_ticks`, `capacity_signal`); the type name is preserved (the rename to `SourceCompositeAdjustment` was considered and rejected — preserving the name avoids cascading import edits in `agenda_manager`, `plan_selection`, and trace consumers, and "discount" is still a fair characterization since trust+wait subtract from motive).

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

impl Default for PreferenceProfile {
    fn default() -> Self {
        Self {
            route_caution_weight: Permille::new_unchecked(300),
            source_trust_weight: Permille::new_unchecked(200),
            route_memory_capacity: 24,
            source_memory_capacity: 18,
            memory_retention_ticks: 400,
            wait_sensitivity_weight: Permille::new_unchecked(150),
        }
    }
}
```

Default `wait_sensitivity_weight = 150` (Permille) — modest baseline. An agent with `wait_sensitivity_weight = 0` is the "patient agent" baseline and ranks sources by trust + capacity only; an agent with `wait_sensitivity_weight = 800` heavily penalizes contested sources. Per-agent tuning comes through the existing `AgentDef.preference_profile` surface; agents without authored profiles use `PreferenceProfile::default()`.

### D6: Decision-trace surfacing

Extend the existing `SourceReliabilityDiscount` struct in `crates/worldwake-ai/src/decision_trace.rs:546–552` with the composite component fields (per D4), and update its `Display` / formatting site at `decision_trace.rs:1952–1961` from the current

```
source_reliability=entity=_ commodity=_ failure=_ pre=_ post=_
```

to

```
source_reliability=entity=_ commodity=_ failure=_ wait_avg=_ wait_pen=_ cap=_ cap_age=_ cap_sig=_ pre=_ post=_
```

Observer Section 3 / Section 4 then surface readable lines for "Agent A chose Camp Well over North Orchard: composite 880 vs 620, North Orchard wait_avg=12 (16 observations), cap=3 (age=200t)." Update the field initializers at the other `SourceReliabilityDiscount {...}` construction sites (`ranking.rs:5721, 5780, 5923-5924, 6003, 6122`; `goal_model.rs:2839`; `agent_tick/planning.rs:4139`) — most are test/golden fixtures that need new fields defaulted to zero.

### D7: Golden coverage

Add `crates/worldwake-ai/tests/golden_source_reliability.rs`:

- Agent acquires water at the same well 5 times with wait_ticks `(0, 3, 5, 8, 12)`. Confirm `average_wait_ticks` matches the documented integer running estimate.
- Two agents at one well; second agent has `wait_sensitivity_weight = 800`, prefers a slightly worse alternative well after 3 wait observations on the first.
- Capacity freshness: agent observes well at capacity 18 at tick 100, then waits 500 ticks (`memory_retention_ticks = 400`); confirm `capacity_signal` in ranking falls to zero (stale).
- After 32 wait observations, the EMA replaces the running mean — confirm the 33rd observation shifts the average correctly per α = 1/32.
- Resource-extraction wait observation: two agents queue at an orchard's `ResourceExtractionQueues`; confirm the second agent records a non-zero `average_wait_ticks` after the first agent's harvest commits and the second is granted the slot.

## SystemFn Integration

No new SystemFn. Wait observation lives in `facility_queue.rs::promote_ready_head` (existing ContentionQueue handler) and `production_actions.rs::grant_or_signal_full` (existing harvest start path). Capacity observation lives in `perception.rs` (existing handler). Memory eviction continues through `enforce_limits` (`crates/worldwake-core/src/experience.rs:91`) per `PreferenceProfile.memory_retention_ticks`.

## Component Registration

No new components. `SourceReliability` already registered as universal per-agent (existing `component_schema.rs:358–380`); new fields land in the existing component schema. `PreferenceProfile` already registered as universal (`component_schema.rs:383–405`); one new field added.

Per FND-22 Section 5: the new field `wait_sensitivity_weight` follows the universal pattern with `Default` impl (per D5 above), inheriting through `AgentDef.preference_profile.unwrap_or_default()` at `crates/worldwake-cli/src/scenario/mod.rs:653`.

## Cross-System Interactions

| System | Interaction | Direction |
|--------|-------------|-----------|
| Generalized contention substrate (S44) | `ContentionQueue` grant promotion in `facility_queue.rs::promote_ready_head` triggers wait observation in agent's `SourceReliability` | State-mediated |
| Resource extraction queues | `ResourceExtractionQueues` slot grant in `production_actions.rs::grant_or_signal_full` triggers wait observation | State-mediated |
| Perception | Co-located observation of `ResourceSource` triggers capacity observation | State-mediated |
| AI ranking | Reads `SourceReliability` composite components for per-candidate composite score in `apply_source_reliability_discount` | State-mediated |
| Quantity-aware acquisition (S127) | `LastHarvestTrace` provides peer-observable contention evidence; `SourceReliability` consumes the perception-level signal | State-mediated |
| `ShareBelief` (existing) | Cross-agent reliability propagation runs through existing tell channels (no new propagation primitive) | State-mediated |

## Profile-Driven Parameters

Per-agent (universal `PreferenceProfile`):

- `source_trust_weight` — existing.
- `wait_sensitivity_weight` — new (default 150).
- `memory_retention_ticks` — existing; bounds capacity freshness window.
- `source_memory_capacity` — existing; bounds total reliability records.

No magic numbers introduced in agent-side code. The `wait_observation_count` cap of 32 is a structural choice (FND-3 — bounded running statistic) documented in code-side comments and discoverable via debug inspection; not a designer dial that changes drama.
