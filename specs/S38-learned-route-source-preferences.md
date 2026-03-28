# S38: Learned Route and Source Preferences

## Summary

Add per-agent experience memories for routes and commodity sources so that agents develop preferences based on their personal history. Currently all agents evaluate routes and sources identically — no agent remembers "that road was dangerous" or "that seller was unreliable." Introduce `RouteExperience` and `SourceReliability` components that record outcomes of past actions and influence future ranking decisions. These are beliefs (can be wrong, stale, evicted), never authoritative truth.

## Source

Derived from ChatGPT architecture review Feature A (Learned local preferences), validated against FOUNDATIONS principles. The codebase currently has zero route learning or source reliability tracking.

## Phase

Phase 4+: Economy & Trade

## Crates

- `worldwake-core` (new components, new profile)
- `worldwake-systems` (action handler extensions for experience recording)
- `worldwake-ai` (ranking adjustments)

## Dependencies

- E14 ✅ (perception & belief — provides belief store architecture)
- S35 (observable activity signals — provides activity observation infrastructure that source reliability extends)
- S33 (opportunity-scoped goal identity — experience influences opportunity-level ranking)

## FOUNDATIONS Alignment

- **P20** (Agent Diversity Through Concrete Variation): Different agents accumulate different experiences, leading to different preferences. Two agents with the same role choose different routes because one was attacked and the other was not.
- **P13** (Knowledge Acquired Locally): Route experience is local — the agent must have actually traveled the route. Source reliability is local — the agent must have actually attempted acquisition.
- **P3** (Concrete State Over Abstract Scores): Experience records count concrete events (safe_trips, hostile_encounters, successful_acquisitions, failed_attempts), not abstract preference scores.
- **P16** (Memory and Records Are World State): Experience records are per-agent state that can be forgotten (capacity eviction), stale (old records), and wrong (a dangerous route may have become safe).

## Design Goals

1. **Experience from action**: Records update only when an agent completes or fails a relevant action (travel, harvest, trade). No abstract scoring.
2. **Belief, not truth**: Experience records are the agent's personal memory. They can be wrong (route danger changed), stale (old records), and evicted (memory capacity).
3. **Tie-breaking influence**: Experience adjusts ranking within the same priority class. It never overrides survival/danger priority or suppresses valid opportunities.
4. **Per-agent diversity**: `PreferenceProfile` weights control how much experience influences each agent's decisions (P20).
5. **Memory-bounded**: Records are bounded by capacity and retention, consistent with existing belief store constraints.

## Deliverables

### 1. `RouteExperience` component (worldwake-core)

```rust
/// Per-agent accumulated experience with specific travel edges.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouteExperience {
    pub edges: BTreeMap<TravelEdgeId, EdgeExperience>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeExperience {
    /// Number of times the agent completed travel on this edge without hostile encounter.
    pub safe_trips: u16,
    /// Number of times the agent encountered hostiles while traveling this edge.
    pub hostile_encounters: u16,
    /// Tick of the most recent travel completion on this edge.
    pub last_travel_tick: Tick,
}
```

### 2. `SourceReliability` component (worldwake-core)

```rust
/// Per-agent accumulated experience with commodity sources and trade partners.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceReliability {
    pub sources: BTreeMap<SourceKey, ReliabilityRecord>,
}

/// Identifies a specific source: an entity + the commodity the agent was seeking there.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceKey {
    pub entity: EntityId,
    pub commodity: CommodityKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceProfile {
    /// How much hostile route experience penalizes travel cost estimation.
    /// Permille(300) = 30% penalty per hostile encounter relative to safe trips.
    pub route_caution_weight: Permille,
    /// How much source failure experience discounts opportunity ranking.
    /// Permille(200) = 20% discount per failure relative to successes.
    pub source_trust_weight: Permille,
    /// Maximum records per experience category before oldest eviction.
    pub memory_capacity: u16,
    /// Ticks after which experience records are considered stale and evicted.
    pub memory_retention_ticks: u64,
}
```

Agents without `PreferenceProfile` use no experience influence (default behavior, P20 diversity).

### 4. Experience recording in action handlers (worldwake-systems)

#### Travel action commit
On successful travel completion:
- If no combat event occurred during the travel leg: increment `safe_trips` for the traveled `TravelEdgeId`.
- If combat occurred during the travel leg: increment `hostile_encounters`.
- Update `last_travel_tick`.

Detection of "combat during travel" uses the event log: check for combat events involving the agent between travel start and commit ticks.

#### Harvest action commit
On successful harvest: increment `successful_acquisitions` for `SourceKey { entity: source, commodity }`.
On harvest failure (source depleted, `StartFailed`): increment `failed_attempts`.
Update `last_attempt_tick`.

#### Trade action commit
On successful trade: increment `successful_acquisitions` for `SourceKey { entity: counterparty, commodity }`.
On trade rejection: increment `failed_attempts`.
Update `last_attempt_tick`.

### 5. Memory capacity and retention (worldwake-core)

Each tick (or on record update), enforce capacity and retention:
- Evict records where `current_tick - last_travel_tick > memory_retention_ticks` (staleness).
- If `edges.len() > memory_capacity`, evict the record with the oldest `last_travel_tick`.
- Same for `sources`.

### 6. Ranking influence (worldwake-ai)

#### Route preference in travel cost estimation

When the planner or ranking estimates travel cost for a route:
1. Look up `RouteExperience` for each edge in the route.
2. For edges with experience: compute `danger_ratio = hostile_encounters / (safe_trips + hostile_encounters)`.
3. Apply penalty: `effective_cost = base_cost * (Permille(1000) + route_caution_weight * danger_ratio_permille)`.
4. Routes through dangerous edges cost more, making safer alternatives more attractive.
5. No experience = no penalty (unknown routes are neutral, not penalized).

#### Source reliability in opportunity ranking

When ranking opportunities for commodity acquisition:
1. Look up `SourceReliability` for the opportunity's source entity + commodity.
2. Compute `failure_ratio = failed_attempts / (successful_acquisitions + failed_attempts)`.
3. Apply discount: `adjusted_motive = motive * (Permille(1000) - source_trust_weight * failure_ratio_permille)`.
4. Motive never drops below `Permille(1)`.
5. No experience = no discount (unknown sources are neutral).

### 7. GoalBeliefView extension

Add to `GoalBeliefView`:

```rust
fn route_experience(&self, agent: EntityId) -> Option<&RouteExperience>;
fn source_reliability(&self, agent: EntityId) -> Option<&SourceReliability>;
fn preference_profile(&self, agent: EntityId) -> Option<&PreferenceProfile>;
```

### 8. Save/load

`RouteExperience`, `SourceReliability`, and `PreferenceProfile` are ECS components — they serialize/deserialize with the world snapshot. Post-load pruning removes entries referencing dead entities.

## Component Registration

- `RouteExperience`: Register on `EntityKind::Agent` in component schema.
- `SourceReliability`: Register on `EntityKind::Agent` in component schema.
- `PreferenceProfile`: Register on `EntityKind::Agent` in component schema.

## FND-01 Section H Analysis

### Information-path analysis
Agent completes travel → action handler records experience in `RouteExperience` component → next planning tick reads experience through `GoalBeliefView` → ranking/search applies cost penalty. Path: action outcome → component update → belief read → ranking influence. All local to the acting agent.

### Positive-feedback analysis
**Cautious-avoidance loop**: Agent encounters hostiles on route → records danger → avoids route → fewer observations of route → stale record persists → agent continues avoiding. This is a mild positive feedback loop (avoidance reinforces avoidance).

### Concrete dampeners
1. **Memory retention**: `memory_retention_ticks` evicts old records. A route that was dangerous 500 ticks ago may no longer be avoided.
2. **Staleness discount**: Records older than half the retention window could receive implicit staleness discount (the agent hasn't traveled this route recently, so experience may be outdated).
3. **No suppression**: Experience only discounts ranking, never suppresses opportunities. An agent with no alternatives will still choose a dangerous route.
4. **Capacity eviction**: `memory_capacity` limits the number of tracked edges/sources. Oldest records evicted first.

### Stored state vs. derived read-model list
- **Stored**: `RouteExperience` (per-agent component), `SourceReliability` (per-agent component), `PreferenceProfile` (per-agent component).
- **Derived**: `danger_ratio`, `failure_ratio`, `effective_cost`, `adjusted_motive` (all recomputed at query time from stored counts).

## Tests

### Focused tests
- [ ] `EdgeExperience` updates correctly on safe travel completion
- [ ] `EdgeExperience` updates correctly when combat occurs during travel
- [ ] `ReliabilityRecord` updates on successful harvest
- [ ] `ReliabilityRecord` updates on failed harvest (source depleted)
- [ ] `ReliabilityRecord` updates on trade completion and rejection
- [ ] Memory retention evicts stale records
- [ ] Memory capacity evicts oldest records when full
- [ ] Route cost penalty applied proportionally to danger_ratio
- [ ] Source discount applied proportionally to failure_ratio
- [ ] No penalty for unknown routes (no experience)
- [ ] No discount for unknown sources (no experience)
- [ ] Agents without `PreferenceProfile` ignore experience entirely
- [ ] Save/load round-trip preserves experience components
- [ ] Post-load pruning removes dead-entity references

### Golden tests
- [ ] Agent attacked during travel records hostile encounter → next planning tick prefers safer alternative route (longer but no hostile history)
- [ ] Two agents with different `PreferenceProfile` weights make different route choices for the same destination
- [ ] Deterministic replay companions

## Acceptance Criteria

1. Agents accumulate route and source experience from completed actions.
2. Experience records are per-agent beliefs with capacity/retention bounds, not authoritative truth.
3. Route danger discounts travel cost estimation; source unreliability discounts opportunity ranking.
4. Experience never suppresses opportunities, only influences tie-breaking within same priority class.
5. `PreferenceProfile` provides per-agent diversity in how much experience matters (P20).
6. Agents without `PreferenceProfile` behave identically to pre-spec behavior.
7. Memory retention and capacity eviction prevent unbounded growth.
