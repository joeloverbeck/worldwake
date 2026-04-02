# S38: Learned Route and Source Preferences

## Summary

Add per-agent experience memories for routes and commodity sources so that agents develop preferences based on their personal history. Currently all agents evaluate routes and sources identically — no agent remembers "that road was dangerous" or "that seller was unreliable." Introduce `RouteExperience` and `SourceReliability` components that record outcomes of past actions and influence future ranking decisions. These are beliefs (can be wrong, stale, evicted), never authoritative truth.

## Source

Derived from ChatGPT architecture review Feature A (Learned local preferences), validated against FOUNDATIONS principles. The codebase currently has zero route learning or source reliability tracking.

## Phase

Phase 4+: Economy & Trade

## Crates

- `worldwake-core` (new components, new profile)
- `worldwake-sim` (GoalBeliefView trait extension, PerAgentBeliefView implementation)
- `worldwake-systems` (action handler extensions for experience recording)
- `worldwake-ai` (ranking adjustments)

## Dependencies

- E14 ✅ (perception & belief — provides belief store architecture)
- S35 ✅ (observable activity signals — provides activity observation infrastructure that source reliability extends)
- S33 ✅ (opportunity-scoped goal identity — experience influences opportunity-level ranking)

## FOUNDATIONS Alignment

- **P22** (Agent Diversity Through Concrete Variation): Different agents accumulate different experiences, leading to different preferences. Two agents with the same role choose different routes because one was attacked and the other was not.
- **P15** (Knowledge Acquired Locally): Route experience is local — the agent must have actually traveled the route. Source reliability is local — the agent must have actually attempted acquisition.
- **P3** (Concrete State Over Abstract Scores): Experience records count concrete events (safe_trips, hostile_encounters, successful_acquisitions, failed_attempts), not abstract preference scores. Ratios are derived at query time via integer Permille arithmetic.
- **P18** (Memory and Records Are World State): Experience records are per-agent state that can be forgotten (capacity eviction), stale (old records), and wrong (a dangerous route may have become safe).
- **P10** (Outcomes Are Granular and Leave Aftermath): Aborted travel due to combat records hostile encounter experience. Failure is new state, not a dead end.

## Design Goals

1. **Experience from action**: Records update only when an agent completes, fails, or is interrupted during a relevant action (travel, harvest, trade). No abstract scoring.
2. **Belief, not truth**: Experience records are the agent's personal memory. They can be wrong (route danger changed), stale (old records), and evicted (memory capacity).
3. **Tie-breaking influence**: Experience adjusts ranking within the same priority class. It never overrides survival/danger priority or suppresses valid opportunities.
4. **Per-agent diversity**: `PreferenceProfile` weights control how much experience influences each agent's decisions (P22).
5. **Memory-bounded**: Records are bounded by capacity and retention, consistent with existing belief store constraints. Eviction is the sole staleness mechanism — no gradual decay.

## Deliverables

### 1. `RouteExperience` component (worldwake-core)

```rust
/// Per-agent accumulated experience with specific travel edges.
#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct RouteExperience {
    pub edges: BTreeMap<TravelEdgeId, EdgeExperience>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct EdgeExperience {
    /// Number of times the agent completed travel on this edge without hostile encounter.
    pub safe_trips: u16,
    /// Number of times the agent encountered hostiles while traveling this edge,
    /// including both completed and aborted (fled) travel.
    pub hostile_encounters: u16,
    /// Tick of the most recent travel completion or abort on this edge.
    pub last_travel_tick: Tick,
}
```

### 2. `SourceReliability` component (worldwake-core)

```rust
/// Per-agent accumulated experience with commodity sources and trade partners.
#[derive(Debug, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub struct SourceReliability {
    pub sources: BTreeMap<SourceKey, ReliabilityRecord>,
}

/// Identifies a specific source: an entity + the commodity the agent was seeking there.
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceKey {
    pub entity: EntityId,
    pub commodity: CommodityKind,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReliabilityRecord {
    /// Number of successful acquisitions from this source.
    pub successful_acquisitions: u16,
    /// Number of failed attempts at this source (source depleted, trade rejected, etc.).
    pub failed_attempts: u16,
    /// Tick of the most recent attempt.
    pub last_attempt_tick: Tick,
}
```

### 3. `PreferenceProfile` component (worldwake-core)

```rust
/// Per-agent weights controlling how much experience influences decisions.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreferenceProfile {
    /// How much hostile route experience penalizes travel cost estimation.
    /// Permille(300) = 30% penalty per hostile encounter relative to safe trips.
    pub route_caution_weight: Permille,
    /// How much source failure experience discounts opportunity ranking.
    /// Permille(200) = 20% discount per failure relative to successes.
    pub source_trust_weight: Permille,
    /// Maximum route edge records before oldest eviction.
    pub route_memory_capacity: u32,
    /// Maximum source records before oldest eviction.
    pub source_memory_capacity: u32,
    /// Ticks after which experience records are considered stale and evicted.
    pub memory_retention_ticks: u64,
}
```

Capacity fields use `u32` to match the established `PerceptionProfile` pattern. Separate `route_memory_capacity` and `source_memory_capacity` follow the PerceptionProfile precedent of per-category capacity (it has separate `memory_capacity`, `institutional_memory_capacity`, and `conversation_memory_capacity` on TellProfile), enabling P22 diversity: a merchant may remember many sources but few routes, while a caravan guard may have the opposite.

Agents without `PreferenceProfile` use no experience influence (default behavior, P22 diversity).

### 4. Experience recording in action handlers (worldwake-systems)

#### Travel action commit (`commit_travel` in `travel_actions.rs`)
On successful travel completion:
- If no combat event occurred during the travel leg: increment `safe_trips` for the traveled `TravelEdgeId`.
- If combat occurred during the travel leg: increment `hostile_encounters`.
- Update `last_travel_tick`.

#### Travel action abort (`abort_travel` in `travel_actions.rs`)
On travel abort due to combat interruption (P10 — failure is new state):
- Increment `hostile_encounters` for the traveled `TravelEdgeId`.
- Update `last_travel_tick`.

On travel abort for non-combat reasons (e.g., replanning): no experience update.

#### Detection of "combat during travel"
Query the event log by intersecting `events_by_tag(EventTag::Combat)` with `events_by_actor(agent_id)`, then filtering to records where `tick >= travel_start_tick && tick < current_tick`. The travel start tick is available from `ActionState::Travel { departure_tick, .. }`.

#### Harvest action commit (`commit_harvest` in `production_actions.rs`)
On successful harvest: increment `successful_acquisitions` for `SourceKey { entity: source, commodity }`.
On harvest failure (source depleted, `StartFailed`): increment `failed_attempts`.
Update `last_attempt_tick`.

#### Harvest action abort
On harvest abort due to external interruption (combat, etc.): no `failed_attempts` increment — the source did not fail, the agent was interrupted. Source reliability tracks source-intrinsic failure, not external danger at the location.

#### Trade action commit (`commit_trade` in `trade_actions.rs`)
On successful trade: increment `successful_acquisitions` for `SourceKey { entity: counterparty, commodity }`.
On trade rejection: increment `failed_attempts`.
Update `last_attempt_tick`.

#### Trade action abort
On trade abort due to external interruption: no `failed_attempts` increment (same rationale as harvest abort).

### 5. Memory capacity and retention (worldwake-core)

On each record update, enforce capacity and retention:
- Evict records where `current_tick - last_travel_tick > memory_retention_ticks` (staleness). Eviction is the sole staleness mechanism — no gradual decay. This follows the established `PerceptionProfile` pattern of binary eviction.
- If `edges.len() > route_memory_capacity`, evict the record with the oldest `last_travel_tick`.
- If `sources.len() > source_memory_capacity`, evict the record with the oldest `last_attempt_tick`.

### 6. Ranking influence (worldwake-ai)

#### Route preference in travel cost estimation

Integration point: `GoalBeliefView::adjacent_places_with_travel_ticks()` in `per_agent_belief_view.rs` (line ~1343). Currently returns raw topology costs. The implementation extends this method (or layers a new method atop it) to apply experience-based cost penalties.

When estimating travel cost for an edge:
1. Look up `RouteExperience` for the edge's `TravelEdgeId`.
2. For edges with experience, compute danger ratio in Permille using integer arithmetic:
   ```
   let total = safe_trips + hostile_encounters;
   let danger_ratio_permille = if total == 0 { Permille(0) }
       else { Permille((hostile_encounters as u32 * 1000 / total as u32) as u16) };
   ```
3. Apply penalty: `effective_ticks = base_ticks * (1000 + route_caution_weight.value() * danger_ratio_permille.value() / 1000) / 1000`. All integer arithmetic, no floats.
4. Routes through dangerous edges cost more, making safer alternatives more attractive.
5. No experience = no penalty (unknown routes are neutral, not penalized).

#### Source reliability in opportunity ranking

Integration point: `ranking.rs`, applied after `ranked_motive_score()` (line ~142) and before or alongside `apply_competition_discount()` (line ~163). Source reliability discount is a separate step in the ranking pipeline, analogous to how `apply_competition_discount` already adjusts motive post-computation.

When ranking opportunities for commodity acquisition:
1. Look up `SourceReliability` for the opportunity's source entity + commodity.
2. Compute failure ratio in Permille using integer arithmetic:
   ```
   let total = successful_acquisitions + failed_attempts;
   let failure_ratio_permille = if total == 0 { Permille(0) }
       else { Permille((failed_attempts as u32 * 1000 / total as u32) as u16) };
   ```
3. Apply discount: `adjusted_motive = motive * (1000 - source_trust_weight.value() * failure_ratio_permille.value() / 1000) / 1000`.
4. Motive never drops below 1.
5. No experience = no discount (unknown sources are neutral).

### 7. GoalBeliefView extension (worldwake-sim)

Add to `GoalBeliefView` trait in `belief_view.rs`:

```rust
fn route_experience(&self, agent: EntityId) -> Option<&RouteExperience>;
fn source_reliability(&self, agent: EntityId) -> Option<&SourceReliability>;
fn preference_profile(&self, agent: EntityId) -> Option<&PreferenceProfile>;
```

Implement in `PerAgentBeliefView` (`per_agent_belief_view.rs`) as self-authoritative reads (the agent reads its own experience and preferences).

### 8. Save/load

`RouteExperience`, `SourceReliability`, and `PreferenceProfile` are ECS components — they serialize/deserialize with the world snapshot. Post-load pruning removes entries referencing dead entities (edges to deleted places, sources for archived entities).

## Component Registration

- `RouteExperience`: Register on `EntityKind::Agent` in component schema.
- `SourceReliability`: Register on `EntityKind::Agent` in component schema.
- `PreferenceProfile`: Register on `EntityKind::Agent` in component schema.

## FND-01 Section H Analysis (P30 Causal Hooks)

### 1. Entities, relations, and records introduced
- `RouteExperience`: per-agent component mapping `TravelEdgeId` → `EdgeExperience` (safe_trips, hostile_encounters, last_travel_tick).
- `SourceReliability`: per-agent component mapping `SourceKey` (entity + commodity) → `ReliabilityRecord` (successful_acquisitions, failed_attempts, last_attempt_tick).
- `PreferenceProfile`: per-agent component with decision weights and memory bounds.

### 2. Actions and world processes that mutate them
- `commit_travel` / `abort_travel` → updates `RouteExperience` for the traveled edge.
- `commit_harvest` / harvest `StartFailed` → updates `SourceReliability` for the harvest source.
- `commit_trade` / trade rejection → updates `SourceReliability` for the trade counterparty.
- Memory eviction (on record update) → removes stale or excess entries.
- Harvest/trade aborts from external interruption do NOT update source reliability.

### 3. Information produced, travel, and observation
Agent completes/aborts action → action handler records experience in component → next planning tick reads experience through `GoalBeliefView` → ranking/search applies cost penalty or motive discount. All information is local to the acting agent. Experience records are not communicated to other agents (agents cannot share route experience — they must travel themselves to learn).

### 4. Conserved quantities
No conserved quantities. Experience records are informational state (belief), not physical resources. They are created by action outcomes and destroyed by eviction. No source/sink accounting required.

### 5. Scarce capacities and contention
`route_memory_capacity` and `source_memory_capacity` are bounded resources per agent. When full, oldest records are evicted. No contention between agents — each agent's experience is private.

### 6. Partial failures and aftermath
- Travel interrupted by combat: records hostile encounter (P10 — failure is new state).
- Harvest/trade interrupted externally: no source reliability update (interruption was not source-intrinsic).
- Eviction of valuable records when capacity is full: agent loses old experience, may revisit dangerous routes or unreliable sources. This is a feature, not a bug (P16 — beliefs decay).

### 7. Positive feedback loops
**Cautious-avoidance loop**: Agent encounters hostiles on route → records danger → avoids route → fewer observations of route → stale record persists → agent continues avoiding.

### 8. Physical dampeners
1. **Memory retention**: `memory_retention_ticks` evicts old records. A route that was dangerous 500 ticks ago is eventually forgotten.
2. **Capacity eviction**: `route_memory_capacity` / `source_memory_capacity` limits tracked entries. Oldest records evicted first.
3. **No suppression**: Experience only adjusts ranking within the same priority class, never suppresses opportunities. An agent with no alternatives will still choose a dangerous route.
4. **Tie-breaking only**: Experience never overrides survival/danger priority. A starving agent still goes to the only available food source even if it has high failure history.

### 9. Derived views and optimizations
- **Stored**: `RouteExperience` (per-agent), `SourceReliability` (per-agent), `PreferenceProfile` (per-agent).
- **Derived**: `danger_ratio_permille`, `failure_ratio_permille`, `effective_ticks`, `adjusted_motive` — all recomputed at query time from stored counts via integer Permille arithmetic. These are caches (P27), never truth.

### 10. How agents become wrong and correct errors
- Route danger changes (bandits eliminated, new threat appears) → agent's experience becomes stale → eviction eventually clears old records → agent re-evaluates with fresh experience or no experience (neutral).
- Source reliability changes (depleted source restocked, reliable source fails) → same eviction mechanism.
- Agents cannot proactively learn that a route became safe without traveling it again. This is correct per P15 (knowledge acquired locally).

### 11. Temporal resolution and scheduling
Experience updates occur during action handler execution (commit/abort phase), which runs within the simulation tick. Eviction runs on each record update, not as a separate per-tick system. This is consistent with the existing belief store eviction pattern in `AgentBeliefStore::enforce_limits()`.

### 12. Boundary conditions
Off-map travel edges: if agents travel edges that cross map boundaries, experience is recorded for those `TravelEdgeId`s normally. No special boundary handling needed — edges are edges regardless of map position.

### 13. Target patterns and invariants
See Tests section below. Key invariants:
- Experience records never suppress opportunities (only adjust motive within priority class).
- Agents without `PreferenceProfile` behave identically to pre-spec behavior.
- All arithmetic uses integer Permille — no floats.
- Capacity and retention bounds are always enforced.

### 14. Save/load and replay
All three components serialize/deserialize with the world snapshot. Post-load pruning removes entries referencing dead entities. Deterministic replay is preserved — all experience updates derive from deterministic action outcomes and seeded RNG.

## Tests

### Focused tests
- [ ] `EdgeExperience` updates correctly on safe travel completion
- [ ] `EdgeExperience` updates correctly when combat occurs during travel
- [ ] `EdgeExperience` updates on travel abort due to combat (hostile_encounters incremented)
- [ ] `EdgeExperience` does NOT update on travel abort for non-combat reasons
- [ ] `ReliabilityRecord` updates on successful harvest
- [ ] `ReliabilityRecord` updates on failed harvest (source depleted)
- [ ] `ReliabilityRecord` does NOT update on harvest abort from external interruption
- [ ] `ReliabilityRecord` updates on trade completion and rejection
- [ ] `ReliabilityRecord` does NOT update on trade abort from external interruption
- [ ] Memory retention evicts stale records (binary eviction, no gradual decay)
- [ ] Route memory capacity evicts oldest edge records when full
- [ ] Source memory capacity evicts oldest source records when full
- [ ] Route cost penalty applied proportionally to danger_ratio (integer Permille arithmetic)
- [ ] Source discount applied proportionally to failure_ratio (integer Permille arithmetic)
- [ ] No penalty for unknown routes (no experience)
- [ ] No discount for unknown sources (no experience)
- [ ] Agents without `PreferenceProfile` ignore experience entirely
- [ ] Motive never drops below 1 after source discount
- [ ] Save/load round-trip preserves experience components
- [ ] Post-load pruning removes dead-entity references

### Golden tests
- [ ] Agent attacked during travel records hostile encounter → next planning tick prefers safer alternative route (longer but no hostile history)
- [ ] Agent whose travel is aborted by combat also records hostile encounter → prefers safer route on replan
- [ ] Two agents with different `PreferenceProfile` weights make different route choices for the same destination
- [ ] Deterministic replay companions

## Acceptance Criteria

1. Agents accumulate route and source experience from completed, failed, and combat-aborted actions.
2. Experience records are per-agent beliefs with capacity/retention bounds, not authoritative truth.
3. Route danger penalizes travel cost estimation via integer Permille arithmetic; source unreliability discounts opportunity ranking motive.
4. Experience never suppresses opportunities, only influences tie-breaking within same priority class.
5. `PreferenceProfile` provides per-agent diversity in how much experience matters (P22), with separate route and source memory capacities.
6. Agents without `PreferenceProfile` behave identically to pre-spec behavior.
7. Memory retention and capacity eviction (binary, no gradual decay) prevent unbounded growth.
8. Travel abort due to combat records hostile encounter (P10). Harvest/trade abort from external interruption does not penalize source reliability.
