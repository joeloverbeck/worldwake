# S100: Tiered Belief Retention for Infrastructure Entities

**Status**: DRAFT

## Summary

Agents forget visited places, facilities, and resource sources after `memory_retention_ticks` expires (default 48 ticks), losing all knowledge of where resources exist. S77 introduced tiered *capacity* eviction (infrastructure beliefs survive eviction before transient items), but time-based decay via `within_retention_window()` in `enforce_capacity()` and `enforce_entity_claim_capacity()` applies uniformly to all beliefs regardless of tier.

This spec adds a separate, longer retention window for infrastructure-tier beliefs (Places, Facilities, resource sources, living Agents) — the same entities S77 already protects from capacity eviction. An agent who visits Thornwall Village and observes its Well, Mill, and Grain will retain those beliefs for `infrastructure_retention_ticks` (default: 480) instead of `memory_retention_ticks` (default: 48), giving them 10x longer to return before forgetting.

## Phase

Post-Phase 7 adjunct (extends S77 belief capacity prioritization)

## Crates

- `worldwake-core` (belief store retention logic, PerceptionProfile)
- `worldwake-cli` (scenario system support, RON field updates)

## Dependencies

- S77 (Belief Capacity Prioritization) — completed (archived). Provides the tiered eviction framework (`entity_eviction_tier`, `claim_eviction_tier`) that this spec reuses.

## Design Goals

- Infrastructure beliefs survive longer than transient observations under time-based decay.
- Reuse existing S77 tier classification — no new tier system.
- Profile-driven retention parameters (per-agent, scenario-configurable).
- No "permanent" memory — all beliefs still decay, just at different rates.

## Non-Goals

- Changing capacity eviction logic (S77 is sufficient).
- Adding new belief types or memory systems.
- Modifying perception or observation scope.
- Addressing planner budget wall issues (separate from belief retention).

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P3 (Concrete State Over Abstract Scores) | Retention window derived from concrete entity properties (EntityKind, EntityBeliefAspect) via existing tier classification, not abstract priority scores |
| P7 (Locality of Information) | Agents build mental maps through observation; this spec ensures structural knowledge (where is the well?) persists longer than transient observations (what items are on the ground?) |
| P8 (No Magic Numbers) | `infrastructure_retention_ticks` is a per-agent profile parameter, scenario-configurable, with a concrete default |
| P14 (World State Is Not Belief State) | Beliefs remain separate from world state. Infrastructure retention is longer, not permanent — beliefs about a destroyed facility will eventually decay |
| P16 (Ignorance, Uncertainty, Contradiction) | All beliefs still decay. Infrastructure beliefs decay slower, not never. Agents can still hold stale beliefs about places that have changed |
| P20 (Resource-Bounded Practical Reasoning) | Planners need place and facility beliefs to generate multi-location plans (travel → acquire → consume). Without them, agents cannot reason about leaving their current location |
| P22A (Learning and Preference Shifts) | Infrastructure retention models the cognitive reality that structural/spatial knowledge is more durable than episodic memory of specific items |
| P26 (Systems Interact Through State) | No cross-system calls. Perception writes beliefs; retention logic preserves them; planner reads them |
| P28 (No Backward Compatibility) | Old uniform retention behavior is replaced, not shimmed |

## Section H: Causal Hooks

### H.1 Motivating Gap

Agents who visit a resource-rich location and then travel to a barren one forget the resource-rich location entirely after `memory_retention_ticks`. Without beliefs about where resources exist, the planner cannot generate `AcquireCommodity` goals targeting remote locations, and the exploration system's `need_has_known_acquisition_path()` returns false — but exploration may also fail if the agent has non-self-care candidates blocking `emit_exploration_candidates()`. The agent becomes permanently stranded.

Existing S77 tiered eviction only protects against capacity-based eviction (too many entities remembered). It does not protect against time-based decay (entity observed too long ago). Both mechanisms independently purge beliefs. This spec closes the time-decay gap.

### H.2 Entities and Relations

No new entities or relations. This spec modifies retention behavior of existing `BelievedEntityState` and `EntityBeliefClaim` within the existing `AgentBeliefStore`.

### H.3 Mutations

- `enforce_capacity()` mutated to use `infrastructure_retention_ticks` for entities classified as infrastructure-tier by `entity_eviction_tier()`.
- `enforce_entity_claim_capacity()` mutated to use `infrastructure_retention_ticks` for claims classified as infrastructure-tier by `claim_eviction_tier()`.
- `PerceptionProfile` gains one new field: `infrastructure_retention_ticks: u64`.

### H.4 Information and Observability

No new information paths. Existing perception → belief → planner path is unchanged. The only change is how long beliefs persist in the store before time-based decay removes them.

Agents cannot observe the retention parameters of other agents. Each agent's retention is private to their own belief store.

### H.5 Conserved Quantities

No conserved quantities affected. Beliefs are agent-local cognitive state, not world-state items.

### H.6 Contention

No contention introduced. Belief retention is per-agent, non-shared state.

### H.7 Partial Failures

If `infrastructure_retention_ticks` is set equal to `memory_retention_ticks`, the behavior is identical to the current system (no regression). If set to 0, all beliefs decay immediately (extreme but non-breaking). The parameter degrades gracefully across its range.

### Information-Path Analysis

No new information paths. The existing path is:
1. Agent co-locates with entity → perception system observes → `build_observed_entity_snapshot()` creates snapshot → claims generated and stored in `AgentBeliefStore`
2. `enforce_capacity()` / `enforce_entity_claim_capacity()` called after perception batch → time-based retention applied → **this is where the change occurs**: infrastructure entities now use `infrastructure_retention_ticks` instead of `memory_retention_ticks`
3. Planner reads `known_entities` via `PerAgentBeliefView` → generates goals targeting believed entities

The change affects step 2 only. Steps 1 and 3 are unchanged.

### Positive-Feedback Analysis

No positive-feedback loops introduced. Longer retention for infrastructure beliefs does not cause more infrastructure beliefs to be generated — perception scope is unchanged.

Potential concern: could longer-lived place beliefs cause agents to make stale plans? Yes, but this is an intended property of the system (P16: beliefs can be stale). The staleness is bounded by `infrastructure_retention_ticks` and by confidence decay (`staleness_penalty_per_tick` in `BeliefConfidencePolicy`). An agent who returns to a place and finds their belief is stale will observe the current state and update.

### Concrete Dampeners

N/A — no positive-feedback loops.

The `infrastructure_retention_ticks` parameter itself is a dampener: it ensures infrastructure beliefs DO eventually decay rather than persisting forever. The `staleness_penalty_per_tick` in `BeliefConfidencePolicy` further dampens stale beliefs by reducing their confidence over time, even while they remain in the store.

### Stored State vs. Derived

- **Stored state (authoritative)**: `PerceptionProfile.infrastructure_retention_ticks` — new per-agent configuration parameter.
- **Stored state (unchanged)**: `AgentBeliefStore.entity_claims`, `AgentBeliefStore.known_entities` — unchanged storage, changed retention window selection.
- **Derived (computed at enforcement time)**: Which retention window to use for a given entity/claim — determined by `entity_eviction_tier()` and `claim_eviction_tier()` at enforcement time, not stored.

---

## Deliverables

### D1: Add `infrastructure_retention_ticks` to `PerceptionProfile`

**File**: `crates/worldwake-core/src/belief.rs`

Add a new field to `PerceptionProfile` (line 2179):

```rust
pub struct PerceptionProfile {
    pub entity_memory_capacity: u32,
    pub entity_claim_capacity: u32,
    pub memory_retention_ticks: u64,
    pub infrastructure_retention_ticks: u64,  // NEW
    pub observation_fidelity: Permille,
    pub confidence_policy: BeliefConfidencePolicy,
    pub institutional_memory_capacity: u32,
    pub consultation_speed_factor: Permille,
    pub contradiction_tolerance: Permille,
}
```

Default value in `impl Default for PerceptionProfile` (line 2192):

```rust
infrastructure_retention_ticks: 480,  // 10x default memory_retention_ticks
```

### D2: Tiered retention in `enforce_capacity`

**File**: `crates/worldwake-core/src/belief.rs`

Modify `enforce_capacity()` (line 178) to use tier-appropriate retention for `known_entities`:

Current code (line 195-204):
```rust
self.known_entities.retain(|entity, state| {
    if self.entity_claims.contains_key(entity) {
        return true;
    }
    within_retention_window(
        state.observed_tick,
        current_tick,
        profile.memory_retention_ticks,
    )
});
```

Changed to:
```rust
self.known_entities.retain(|entity, state| {
    if self.entity_claims.contains_key(entity) {
        return true;
    }
    let retention = if entity_eviction_tier(state) > 0 {
        profile.infrastructure_retention_ticks
    } else {
        profile.memory_retention_ticks
    };
    within_retention_window(state.observed_tick, current_tick, retention)
});
```

Note: `entity_eviction_tier` returns higher values for infrastructure entities (Places, Facilities, living Agents). The tier > 0 check matches the same entities that S77 already protects from capacity eviction.

### D3: Tiered retention in `enforce_entity_claim_capacity`

**File**: `crates/worldwake-core/src/belief.rs`

Modify `enforce_entity_claim_capacity()` (line 226) to use tier-appropriate retention for individual claims:

Current code (line 250-256):
```rust
claims.retain(|claim| {
    within_retention_window(
        claim.acquired_tick,
        current_tick,
        profile.memory_retention_ticks,
    )
});
```

Changed to:
```rust
claims.retain(|claim| {
    let retention = if claim_eviction_tier(claim.aspect, believed_kind) == 0 {
        profile.infrastructure_retention_ticks
    } else {
        profile.memory_retention_ticks
    };
    within_retention_window(claim.acquired_tick, current_tick, retention)
});
```

Note: `believed_kind` is already in scope from line 242 of the enclosing `for entity in &affected_entities` loop, which looks it up via `self.known_entities.get(entity).and_then(|state| state.believed_kind)`.

Note: `claim_eviction_tier` returns 0 for infrastructure claims (ResourceAvailable, WorkstationPresent, Location of Places/Facilities, Alive for Agents). Tier 0 claims are the ones that should survive longer.

### D4: Scenario system support

**File**: `crates/worldwake-cli/src/scenario/types.rs`

The `PerceptionProfile` is already fully represented in the scenario RON format. The new `infrastructure_retention_ticks` field will be serialized/deserialized automatically via serde. Since `PerceptionProfile` is a universal profile applied via `unwrap_or_default()` in `spawn_agent()`, agents without explicit perception profiles in the scenario get the default (480 ticks).

`PerceptionProfile` does not use `#[serde(default)]` on individual fields, so omitting the new field in RON will cause a parse error. All existing scenario files with explicit perception profiles must be updated to include the new field. Currently only `scenarios/cli-evaluation.ron` has explicit perception profiles (Kael and Guard Theron).

**File**: `scenarios/cli-evaluation.ron`

Update Kael and Guard Theron's explicit `perception_profile` blocks to include `infrastructure_retention_ticks: 480` (or a custom value).

### D5: Social observation retention

**File**: `crates/worldwake-core/src/belief.rs`

The `social_observations` retention at line 179-185 uses `memory_retention_ticks` uniformly. Social observations (tells, shared beliefs) are NOT infrastructure — they are transient social interactions. No change needed here; they continue using `memory_retention_ticks`.

---

## Cross-System Interactions (P26)

This spec modifies only `worldwake-core` belief retention. No cross-system calls are introduced.

- **Perception system** (worldwake-systems): Unchanged. Continues to write beliefs through the same API.
- **Planner** (worldwake-ai): Unchanged. Continues to read beliefs through `PerAgentBeliefView`. Benefits indirectly because infrastructure beliefs persist longer, providing more planning targets.
- **Strategic planner** (worldwake-ai/search/strategic.rs): Benefits indirectly — place beliefs required for multi-location itineraries survive longer.
- **Exploration system** (worldwake-ai/candidate_generation.rs): Benefits indirectly — `need_has_known_acquisition_path()` finds more known paths when resource source beliefs persist longer.

---

## Verification

### Unit Tests

1. **Infrastructure entities survive longer than transient entities** — Create a belief store with a Place entity and an ItemLot entity, both observed at tick 0. Advance to tick 100 (between `memory_retention_ticks=48` and `infrastructure_retention_ticks=480`). After `enforce_capacity()`, the Place should survive and the ItemLot should be evicted.

2. **Infrastructure claims survive longer than transient claims** — Create claims for ResourceAvailable (tier 0) and Inventory (tier 1) on the same entity. Advance past `memory_retention_ticks`. ResourceAvailable claim should survive; Inventory claim should be evicted.

3. **Equal retention when parameters match** — Set `infrastructure_retention_ticks == memory_retention_ticks`. Verify behavior is identical to pre-change (no regression).

4. **Infrastructure entities do eventually decay** — Advance past `infrastructure_retention_ticks`. Verify infrastructure beliefs ARE evicted (not permanent).

5. **Social observations unaffected** — Verify social observations still use `memory_retention_ticks`, not `infrastructure_retention_ticks`.

### Golden Test

A golden test should verify the end-to-end effect: an agent who visits a resource-rich location, travels to a barren one, and later returns to the resource-rich location because they still remember it exists. This test guards against the DM-1/DM-3 regression (Kael stranded at Dusty Trail forgetting Thornwall Village).

### Commands

```bash
cargo test -p worldwake-core -- infrastructure_retention
cargo test -p worldwake-core
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```
