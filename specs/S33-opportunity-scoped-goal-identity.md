# S33: Opportunity-Scoped Goal Identity

## Summary

Separate desire-level identity from opportunity-level identity in the goal system. Currently `GoalKey` conflates "what condition the agent wants true" with "which specific source/tactic the agent is pursuing." This causes exhaustion of one source (e.g., orchard apples blocked) to suppress planning for unrelated alternatives (e.g., market apples). Introduce `OpportunityAnchor` and `OpportunityKey` so that blocking, exhaustion, and plan binding scope to specific opportunities while IntentionFrames and desire-level reasoning remain at `GoalKey` granularity.

## Source

Derived from ChatGPT architecture review WW-AI-002 (Separate desires from opportunities), filtered against the current codebase. The review correctly identified that `GoalKind` mixes desired conditions with chosen methods. The proposed `DesireKind` rewrite is overkill — `GoalKey` already serves as desire-level identity. The missing piece is opportunity-level scoping for exhaustion and blocking.

## Phase

Phase 3+: AI Architecture Overhaul, Step 13.5 Wave 5

## Crates

- `worldwake-core` (new types)
- `worldwake-ai` (exhaustion, candidate generation, blocking, search, plan selection)

## Dependencies

- S31 ✅ (goal-aware exhaustion invalidation — provides the invalidation condition infrastructure this spec extends)
- S23 ✅ (refined blocked intents — provides compound-keyed blocker records this spec extends)
- S22 ✅ (generalized intention frames — provides frame lifecycle this spec interacts with)

## FOUNDATIONS Alignment

- **P18** (Resource-Bounded Practical Reasoning): Agents should not lose access to valid alternatives because a different opportunity for the same desire failed. Blocking one source must not suppress reasoning about other sources.
- **P19** (Intentions Are Revisable Commitments): Switching from orchard-harvest to market-purchase for the same "acquire food" desire should preserve frame continuity — the desire hasn't changed, only the tactic.
- **P20** (Agent Diversity): Different agents may prefer different opportunities for the same desire based on their situation, making opportunity-level identity load-bearing for diverse behavior.
- **P3** (Concrete State Over Abstract Scores): Opportunity anchors reference concrete entities (places, sources, routes), not abstract categories.

## Design Goals

1. **Opportunity isolation**: Exhaustion of one opportunity does not suppress other opportunities for the same desire.
2. **Desire continuity**: IntentionFrame persists on desire (`GoalKey`), not opportunity. Tactic changes within the same desire do not destroy frame commitment.
3. **Blocker escalation**: Opportunity-level blocking escalates to desire-level only when ALL known opportunities are blocked.
4. **Backward elimination**: No shims or compatibility layers — `GoalKey`-only exhaustion paths are removed (P26).
5. **Minimal invasion**: Ranking, priority classes, and feasibility remain at `GoalKey` level. Opportunity scoping applies only to exhaustion, blocking, and plan binding.

## Current Shape (What Exists)

- `GoalKey { kind, commodity, entity, place }` — canonical goal identity.
- `GroundedGoal { key, evidence_entities, evidence_places }` — aggregates ALL sources into one goal instance.
- `ExhaustionEntry` keyed by `GoalKey` — exhausting one source exhausts the entire desire.
- `BlockerKey { goal_key, place, target, action_def }` — place-scoped blocking exists but candidate generation checks global blockers first.
- `IntentionFrame { goal: GoalKey, ... }` — persists on goal identity.

## Deliverables

### 1. `OpportunityAnchor` enum (worldwake-core)

```rust
/// Concrete world-state anchor distinguishing one opportunity from another
/// for the same desire (GoalKey).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OpportunityAnchor {
    /// Opportunity tied to a specific place (e.g., harvest at orchard, buy at market).
    Place(EntityId),
    /// Opportunity tied to a specific entity (e.g., trade with merchant, treat patient).
    Entity(EntityId),
    /// Opportunity tied to a specific route.
    Route { from: EntityId, to: EntityId },
    /// Opportunity with no spatial anchor (e.g., self-care, consume from inventory).
    None,
}
```

### 2. `OpportunityKey` struct (worldwake-core)

```rust
/// Identifies a specific opportunity: a desire + the concrete anchor being pursued.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OpportunityKey {
    pub goal_key: GoalKey,
    pub anchor: OpportunityAnchor,
}
```

### 3. `GroundedGoal` refactoring (worldwake-ai)

Replace the merged `evidence_entities: BTreeSet<EntityId>` and `evidence_places: BTreeSet<EntityId>` with a single `anchor: OpportunityAnchor`. Candidate generation emits **one `GroundedGoal` per opportunity**, not one per desire with merged evidence sets.

```rust
pub struct GroundedGoal {
    pub key: GoalKey,
    pub anchor: OpportunityAnchor,
}
```

Candidate generation changes:
- `emit_acquire_goals()`: Instead of collecting all sources into one `evidence_places` set, emit separate `GroundedGoal` instances per source place.
- `emit_produce_goals()`: One `GroundedGoal` per workstation/source combination.
- `emit_sell_goals()`: One per known buyer location.
- Survival/self-care goals: Use `OpportunityAnchor::None` (no external source needed).
- Care goals: Use `OpportunityAnchor::Entity(patient)`.
- Political goals: Use `OpportunityAnchor::Entity(office)` or `OpportunityAnchor::Place(jurisdiction)`.

### 4. Exhaustion keyed by `OpportunityKey` (worldwake-ai)

Change `exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>` to `BTreeMap<OpportunityKey, ExhaustionEntry>`.

- `record_exhausted_goal()` takes `OpportunityKey` instead of `GoalKey`.
- `invalidate_exhausted_goals()` operates per-opportunity. Invalidation conditions (S31) apply per-opportunity baseline.
- `is_exhausted()` checks the specific `OpportunityKey`, not the `GoalKey`.
- Cooldown/retry logic (unchanged from S31 semantics) scopes to opportunity.

### 5. Blocker escalation (worldwake-ai)

Extend `BlockedIntentMemory` with desire-level escalation:

- `is_desire_blocked(goal_key, current_tick) -> bool`: Returns true only when ALL non-expired blockers for this `GoalKey` cover all known opportunities (checked against current candidate set).
- Candidate generation: Remove the early `is_blocked(&key, None, None, None, current_tick)` global check. Instead, filter individual `GroundedGoal` instances against opportunity-scoped blockers.
- Search-level `is_blocked_for_search()` (S23) continues to check place-scoped blockers per candidate.

### 6. IntentionFrame interaction

No structural change to `IntentionFrame` — it already persists on `GoalKey` (desire-level identity). When a plan switches from orchard-opportunity to market-opportunity for the same `GoalKey`, the frame continues rather than being cleared and recreated. The plan's `OpportunityKey` is tracked on `PlannedPlan` (new field).

```rust
pub struct PlannedPlan {
    pub goal: GoalKey,
    pub opportunity: OpportunityKey,  // NEW
    pub steps: Vec<PlannedStep>,
    pub terminal_kind: PlanTerminalKind,
}
```

### 7. Ranking interaction

Ranking continues to operate at `GoalKey` level for priority class and motive score. When multiple opportunities exist for the same `GoalKey`, ranking selects the highest-scoring opportunity. The planner then searches only that opportunity.

Deduplication rule: Within the same `GoalKey`, only the top-ranked opportunity (by feasibility, then motive, then stable ordering) proceeds to plan search. This prevents budget waste searching multiple paths for the same desire.

### 8. Save/load

`OpportunityKey` and `OpportunityAnchor` must serialize/deserialize. `SAVE_FORMAT_VERSION` bumps. Post-load pruning removes exhaustion entries referencing dead entities in their anchor.

## Component Registration

- `OpportunityAnchor`: Value type in `worldwake-core`, no ECS registration needed.
- `OpportunityKey`: Value type in `worldwake-core`, no ECS registration needed.
- No new ECS components. Changes are to existing runtime state (`exhaustion_cache`, `PlannedPlan`, `GroundedGoal`).

## FND-01 Section H Analysis

### Information-path analysis
No new information paths. Opportunity anchors are derived from existing belief-view queries (which places have commodity sources, which entities are merchants). The belief-view trait is not modified.

### Positive-feedback analysis
No amplifying loops introduced. Opportunity-level exhaustion is strictly a restriction mechanism.

### Concrete dampeners
N/A — no positive feedback loops.

### Stored state vs. derived read-model list
- **Stored**: `OpportunityKey` on `ExhaustionEntry` (runtime cache, not authoritative world state). `OpportunityKey` on `PlannedPlan` (runtime state).
- **Derived**: `OpportunityAnchor` on `GroundedGoal` (recomputed each tick from beliefs). Desire-level blocker escalation (recomputed from current blocker set + candidate set).

## Tests

### Focused tests
- [ ] Blocking orchard-anchored opportunity does NOT suppress market-anchored opportunity for same `GoalKey`
- [ ] Exhausting search for `OpportunityKey { AcquireCommodity(Apple), Place(orchard) }` leaves `OpportunityKey { AcquireCommodity(Apple), Place(market) }` plannable
- [ ] Desire-level escalation fires only when ALL opportunities for a `GoalKey` are blocked
- [ ] Candidate generation emits separate `GroundedGoal` per source place for `AcquireCommodity`
- [ ] Frame persists when plan switches from orchard-opportunity to market-opportunity (same `GoalKey`)
- [ ] `PlannedPlan.opportunity` correctly reflects the searched opportunity
- [ ] Save/load round-trip preserves `OpportunityKey` in exhaustion cache
- [ ] Post-load pruning removes exhaustion entries with dead-entity anchors
- [ ] Ranking deduplicates to top opportunity per `GoalKey` before plan search

### Golden tests
- [ ] Agent with two known apple sources: blocks one, autonomously switches to alternative source
- [ ] Agent exhausts search at orchard (source depleted), travels to market instead (separate opportunity)
- [ ] Deterministic replay companion for each golden

## Acceptance Criteria

1. Blocking one source for a commodity does not suppress planning for alternative sources of the same commodity.
2. Exhaustion is scoped per-opportunity, not per-desire.
3. IntentionFrame continuity is maintained when tactic switches within the same desire.
4. No backward-compatibility shims — `GoalKey`-only exhaustion paths are removed.
5. All existing golden tests pass (behavioral equivalence for single-source scenarios).
6. Save/load round-trip preserves opportunity-scoped exhaustion state.
