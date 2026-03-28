# S33: Opportunity-Scoped Goal Identity

**Status**: COMPLETED

## Summary

Separate desire-level identity from opportunity-level identity in the goal system. Currently `GoalKey` conflates "what condition the agent wants true" with "which specific source/tactic the agent is pursuing." This causes exhaustion of one source (e.g., orchard apples blocked) to suppress planning for unrelated alternatives (e.g., market apples). Introduce `OpportunityAnchor` and `OpportunityKey` so that blocking, exhaustion, and plan binding scope to specific opportunities while IntentionFrames and desire-level reasoning remain at `GoalKey` granularity.

## Source

Derived from ChatGPT architecture review WW-AI-002 (Separate desires from opportunities), filtered against the current codebase. The review correctly identified that `GoalKind` mixes desired conditions with chosen methods. The proposed `DesireKind` rewrite is overkill — `GoalKey` already serves as desire-level identity. The missing piece is opportunity-level scoping for exhaustion and blocking.

## Phase

Phase 3+: AI Architecture Overhaul, Step 13.5 Wave 5

## Crates

- `worldwake-core` (new types)
- `worldwake-ai` (exhaustion, candidate generation, blocking, search, plan selection, ranking)

## Dependencies

- S31 ✅ (goal-aware exhaustion invalidation — provides the invalidation condition infrastructure this spec extends)
- S23 ✅ (refined blocked intents — provides compound-keyed blocker records this spec extends)
- S22 ✅ (generalized intention frames — provides frame lifecycle this spec interacts with)

## FOUNDATIONS Alignment

- **P18** (Resource-Bounded Practical Reasoning): Agents should not lose access to valid alternatives because a different opportunity for the same desire failed. Blocking one source must not suppress reasoning about other sources.
- **P19** (Intentions Are Revisable Commitments): Switching from orchard-harvest to market-purchase for the same "acquire food" desire should preserve frame continuity — the desire hasn't changed, only the tactic.
- **P20** (Agent Diversity): Different agents may prefer different opportunities for the same desire based on their situation, making opportunity-level identity load-bearing for diverse behavior.
- **P3** (Concrete State Over Abstract Scores): Opportunity anchors reference concrete entities (places, sources), not abstract categories. Evidence sets on GroundedGoal reference concrete entities and places for planning scope.
- **P25** (Derived Summaries Are Caches, Never Truth): Both `OpportunityAnchor` and evidence sets on `GroundedGoal` are derived each tick from beliefs — neither is authoritative world state.
- **P26** (No Backward Compatibility): `GoalKey`-only exhaustion paths are removed, not shimmed.

## Design Goals

1. **Opportunity isolation**: Exhaustion of one opportunity does not suppress other opportunities for the same desire.
2. **Desire continuity**: IntentionFrame persists on desire (`GoalKey`), not opportunity. Tactic changes within the same desire do not destroy frame commitment.
3. **Blocker escalation**: Opportunity-level blocking escalates to desire-level only when ALL known opportunities are blocked (two-pass generation).
4. **Backward elimination**: No shims or compatibility layers — `GoalKey`-only exhaustion paths are removed (P26).
5. **Minimal invasion**: Ranking, priority classes, and feasibility remain at `GoalKey` level. Opportunity scoping applies only to exhaustion, blocking, and plan binding.

## Current Shape (What Exists)

- `GoalKey { kind, commodity, entity, place }` — canonical goal identity.
- `GroundedGoal { key, evidence_entities, evidence_places }` — aggregates ALL sources into one goal instance via `BTreeMap<GoalKey, GroundedGoal>` with evidence merging in `emit_candidate()`.
- `ExhaustionEntry` keyed by `GoalKey` — exhausting one source exhausts the entire desire.
- `BlockerKey { goal_key, place, target, action_def }` — place-scoped blocking exists but candidate generation checks global blockers first via `is_blocked(&key, None, None, None, tick)`.
- `IntentionFrame { goal: GoalKey, ... }` — persists on goal identity.
- `PlannedPlan { goal, steps, total_estimated_ticks, terminal_kind }` — no opportunity field.
- Candidate generation uses `BTreeMap<GoalKey, GroundedGoal>` which merges evidence when multiple sources contribute to the same GoalKey.
- `build_planning_snapshot()` uses `evidence_entities` and `evidence_places` from GroundedGoal to scope the belief view for plan search (BFS from evidence places, include evidence entities and their possessions).

## Deliverables

### 1. `OpportunityAnchor` enum (worldwake-core)

```rust
/// Concrete world-state anchor distinguishing one opportunity from another
/// for the same desire (GoalKey).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OpportunityAnchor {
    /// Opportunity tied to a specific place (e.g., harvest at orchard, buy at market).
    Place(EntityId),
    /// Opportunity tied to a specific entity (e.g., trade with merchant, treat patient).
    Entity(EntityId),
    /// Opportunity with no spatial anchor (e.g., self-care, consume from inventory).
    None,
}
```

Note: `Route { from, to }` variant omitted (YAGNI — no current or near-future goal type uses route-based opportunities). Can be added in S38 or later if a concrete use case materializes.

### 2. `OpportunityKey` struct (worldwake-core)

```rust
/// Identifies a specific opportunity: a desire + the concrete anchor being pursued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OpportunityKey {
    pub goal_key: GoalKey,
    pub anchor: OpportunityAnchor,
}
```

### 3. `GroundedGoal` refactoring (worldwake-ai)

Add `anchor: OpportunityAnchor` field while retaining `evidence_entities` and `evidence_places`. The anchor provides opportunity-level *identity* (for exhaustion/blocking keys). The evidence sets provide *planning scope* (for `build_planning_snapshot()` to scope the belief view). These serve different purposes and both are derived each tick.

```rust
pub struct GroundedGoal {
    pub key: GoalKey,
    pub anchor: OpportunityAnchor,
    pub evidence_entities: BTreeSet<EntityId>,
    pub evidence_places: BTreeSet<EntityId>,
}
```

Candidate generation emits **one `GroundedGoal` per opportunity**, not one per desire with merged evidence sets. The collection changes from `BTreeMap<GoalKey, GroundedGoal>` (with evidence merging) to a structure keyed by `OpportunityKey` (no merging across different anchors).

Candidate generation changes:
- `emit_acquire_goals()`: Instead of collecting all sources into one `evidence_places` set, emit separate `GroundedGoal` instances per source place. Each gets `OpportunityAnchor::Place(source_place)` with evidence scoped to that place's entities and reachability.
- `emit_produce_goals()`: One `GroundedGoal` per workstation/source combination. Anchor is `OpportunityAnchor::Place(workstation_place)`.
- `emit_sell_goals()`: One per known buyer location. Anchor is `OpportunityAnchor::Place(buyer_place)`.
- Survival/self-care goals (eat, drink, sleep, relieve, wash): Use `OpportunityAnchor::None` (no external source needed).
- Care goals: Use `OpportunityAnchor::Entity(patient)`.
- Political goals: Use `OpportunityAnchor::Entity(office)` or `OpportunityAnchor::Place(jurisdiction)`.

### 4. Two-pass candidate generation (worldwake-ai)

Replace the current single-pass generation (which checks `is_blocked()` during emission) with a two-pass approach:

- **Pass 1 (Emit)**: Generate all `GroundedGoal` instances without blocker filtering. One per opportunity. The early `is_blocked(&key, None, None, None, current_tick)` global check in `emit_candidate()` is removed.
- **Pass 2 (Filter)**: Filter individual `GroundedGoal` instances against opportunity-scoped blockers. For each candidate, check `is_blocked(&key, anchor_place, anchor_target, None, current_tick)` where `anchor_place`/`anchor_target` are derived from the `OpportunityAnchor`.
- **Desire-level escalation**: After filtering, if ALL opportunities for a `GoalKey` were blocked, record this in the decision trace as desire-level escalation. This is a diagnostic signal only — the GoalKey simply has no surviving candidates.

This resolves the chicken-and-egg problem: the full opportunity set is known before any blocking decisions are made.

### 5. Exhaustion keyed by `OpportunityKey` (worldwake-ai)

Change `exhaustion_cache: BTreeMap<GoalKey, ExhaustionEntry>` to `BTreeMap<OpportunityKey, ExhaustionEntry>`.

- `record_exhausted_goals()` takes `OpportunityKey` instead of `GoalKey`.
- `invalidate_exhausted_goals()` operates per-opportunity. Invalidation conditions (S31) apply per-opportunity baseline with unchanged condition semantics (PositionChanged, CommodityChanged, etc. fire on the same criteria as today). False invalidation of still-unreachable opportunities is bounded by S31's exponential backoff.
- `is_exhausted()` checks the specific `OpportunityKey`, not the `GoalKey`.
- Cooldown/retry logic (unchanged from S31 semantics) scopes to opportunity.

### 6. Ranked opportunity admission with exhaustion fallthrough (worldwake-ai)

Ranking continues to operate over `RankedGoal` entries whose identity is already opportunity-scoped through `GroundedGoal { key, anchor }`. Priority class and motive remain desire-driven, so sibling opportunities for one `GoalKey` can legitimately rank near each other, but they must not be collapsed back to desire identity before search. After ranking:

1. Sort all `RankedGoal` entries by the live ranking order.
2. Iterate ranked entries directly in that order.
3. Skip opportunities whose `OpportunityKey` is exhausted and currently suppresses planning.
4. Attempt plan search for each remaining opportunity in order until one yields a found plan or the planning budget candidate cap is reached.
5. If all ranked opportunities for a `GoalKey` are exhausted or fail search, later sibling opportunities must still be allowed to fall through within the same planning pass.

This replaces the temporary first-per-`GoalKey` planning gate that survived the initial S33 rollout. Canonical identity from ranking into admission is now the ranked opportunity stream itself, not a second post-rank dedup structure.

### 7. `build_candidate_plans()` iteration (worldwake-ai)

`build_candidate_plans()` in `agent_tick/planning.rs` iterates ranked opportunities directly. The key changes:

- `build_planning_snapshot()` receives the per-opportunity `evidence_entities` and `evidence_places` from each searched `GroundedGoal` — no changes to snapshot builder logic needed.
- `record_exhausted_goals()` records exhaustion under the `OpportunityKey` (goal_key + anchor) instead of bare `GoalKey`.
- traced planning attempts preserve ranked per-opportunity ordering so the debugging contract exposes which sibling opportunities were tried before search terminated
- When a plan is found, the resulting `PlannedPlan` carries the `OpportunityKey`.

### 8. IntentionFrame interaction

No structural change to `IntentionFrame` — it already persists on `GoalKey` (desire-level identity). When a plan switches from orchard-opportunity to market-opportunity for the same `GoalKey`, the frame continues rather than being cleared and recreated. The plan's `OpportunityKey` is tracked on `PlannedPlan` (new field).

```rust
pub struct PlannedPlan {
    pub goal: GoalKey,
    pub opportunity: OpportunityKey,  // NEW
    pub steps: Vec<PlannedStep>,
    pub total_estimated_ticks: u32,
    pub terminal_kind: PlanTerminalKind,
}
```

### 9. Blocker escalation (worldwake-ai)

With two-pass generation (Deliverable 4), desire-level escalation is a post-filter diagnostic:

- After Pass 2 filters per-opportunity blockers, check each `GoalKey` group.
- If a `GoalKey` had candidates in Pass 1 but zero survived Pass 2, all opportunities were blocked.
- Record this in `DecisionTrace` as `DesireFullyBlocked { goal_key, blocked_opportunities: Vec<OpportunityKey> }` for debugging (P27).
- No structural change to `BlockedIntentMemory` — it already supports place-scoped blockers. The change is in how candidate generation *queries* it (per-opportunity instead of global).

### 10. Save/load

`OpportunityKey` and `OpportunityAnchor` must serialize/deserialize. `SAVE_FORMAT_VERSION` bumps. Post-load pruning removes exhaustion entries referencing dead entities in their anchor.

## Component Registration

- `OpportunityAnchor`: Value type in `worldwake-core`, no ECS registration needed.
- `OpportunityKey`: Value type in `worldwake-core`, no ECS registration needed.
- No new ECS components. Changes are to existing runtime state (`exhaustion_cache`, `PlannedPlan`, `GroundedGoal`).

## FND-01 Section H Analysis

### Information-path analysis
No new information paths. Opportunity anchors are derived from existing belief-view queries (which places have commodity sources, which entities are merchants). The belief-view trait is not modified. Evidence sets continue to flow from belief queries through candidate generation into the planning snapshot builder — the path is unchanged, only the granularity changes (per-opportunity instead of merged).

### Positive-feedback analysis
No amplifying loops introduced. Opportunity-level exhaustion is strictly a restriction mechanism. The two-pass generation and ranked opportunity admission are stateless per-tick computations.

### Concrete dampeners
N/A — no positive feedback loops.

### Stored state vs. derived read-model list
- **Stored**: `OpportunityKey` on `ExhaustionEntry` (runtime cache, not authoritative world state). `OpportunityKey` on `PlannedPlan` (runtime state).
- **Derived**: `OpportunityAnchor` on `GroundedGoal` (recomputed each tick from beliefs). Evidence sets on `GroundedGoal` (recomputed each tick). Desire-level blocker escalation (recomputed from current blocker set + candidate set). Ranked admission order and per-pass fallthrough result (recomputed each tick).

## Tests

### Focused tests
- [ ] Blocking orchard-anchored opportunity does NOT suppress market-anchored opportunity for same `GoalKey`
- [ ] Exhausting search for `OpportunityKey { AcquireCommodity(Apple), Place(orchard) }` leaves `OpportunityKey { AcquireCommodity(Apple), Place(market) }` plannable
- [ ] Desire-level escalation fires only when ALL opportunities for a `GoalKey` are blocked (two-pass filter)
- [ ] Candidate generation emits separate `GroundedGoal` per source place for `AcquireCommodity`
- [ ] Each per-opportunity `GroundedGoal` carries evidence scoped to its anchor (not merged across opportunities)
- [ ] Frame persists when plan switches from orchard-opportunity to market-opportunity (same `GoalKey`)
- [ ] `PlannedPlan.opportunity` correctly reflects the searched opportunity
- [ ] Save/load round-trip preserves `OpportunityKey` in exhaustion cache
- [ ] Post-load pruning removes exhaustion entries with dead-entity anchors
- [ ] Ranked admission searches same-goal sibling opportunities in order without collapsing them before search
- [ ] Ranked admission falls through to the next same-goal opportunity when a higher-ranked sibling is exhausted
- [ ] When all opportunities for a `GoalKey` are exhausted, no candidate proceeds to plan search
- [ ] `build_planning_snapshot()` receives per-opportunity evidence and scopes correctly
- [ ] Decision trace records `DesireFullyBlocked` when all opportunities for a GoalKey are blocked

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
7. Two-pass candidate generation correctly separates emission from blocker filtering.
8. Ranked opportunity admission with exhaustion fallthrough preserves same-pass sibling search rather than re-collapsing to desire identity before planning.
9. `build_planning_snapshot()` continues to receive per-opportunity evidence for belief scoping.

## Outcome

- Completed: 2026-03-28
- What actually changed:
  - established `OpportunityAnchor` / `OpportunityKey` as the canonical concrete opportunity identity across candidate generation, blocker diagnostics, exhaustion state, ranked planning admission, and `PlannedPlan`
  - refactored `GroundedGoal` emission to one candidate per concrete opportunity with isolated evidence scope instead of desire-level evidence merging
  - moved blocker handling to post-emission opportunity filtering with desire-level `DesireFullyBlocked` diagnostics
  - re-keyed exhaustion/runtime persistence to `OpportunityKey`
  - removed the temporary first-per-`GoalKey` planning admission collapse so ranked sibling opportunities are searched directly in order
  - closed the remaining golden gap by strengthening blocked-source recovery coverage and adding exhausted-opportunity selection coverage
- Deviations from original plan:
  - the work landed as a sequence of narrower S33 tickets rather than one monolithic patch; the archived ticket trail is the authoritative implementation record
  - the final exhausted-opportunity golden stops at the selected-opportunity boundary for its loose-lot scenario, because that is the strongest isolated proof surface for the S33 invariant without conflating a separate downstream execution contradiction
  - save/load/version effects were absorbed by the concrete runtime-shape tickets instead of a single end-spec change list
- Verification results:
  - focused candidate-generation, planning, decision-trace, persistence, and search tests for opportunity-scoped identity all passed
  - golden coverage now includes blocked-source and exhausted-opportunity source switching
  - `cargo test -p worldwake-ai` passed
  - `cargo clippy --workspace` passed
  - `cargo test --workspace` passed
