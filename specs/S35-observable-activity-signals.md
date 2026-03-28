# S35: Observable Activity Signals

## Summary

Make active actions observable through the perception system so co-located agents can see what others are doing. Currently perception receives `active_actions` but ignores them (`_active_actions` on line 23 of `perception.rs`). This means agents cannot observe "Bob is harvesting at the orchard" and consequently cannot avoid dogpiling on contested resources. Introduce `BelievedActivity` on `BelievedEntityState` and a ranking discount for observed competition.

## Source

Derived from ChatGPT architecture review WW-AI-005 (Tick phasing and visible local coordination), filtered to the observable activity component only. Tick phase restructuring is not needed — the current implicit phase ordering (belief refresh before deliberation) is already correct. The genuine gap is that agents are blind to co-located activity.

## Phase

Phase 3+: AI Architecture Overhaul, Step 13.5 Wave 5

## Crates

- `worldwake-core` (new belief field, new profile)
- `worldwake-systems` (perception extension)
- `worldwake-ai` (ranking discount)

## Dependencies

- E14 ✅ (perception & belief — provides `AgentBeliefStore`, `BelievedEntityState`, perception system)

## FOUNDATIONS Alignment

- **P7** (Locality of Interaction): Observing activity at your location IS local observation. This does not add telepathy — agents only see actions at their current place.
- **P8** (Every Action Has Occupancy): Actions occupy the actor visibly. If harvesting occupies a workstation, co-located agents should be able to see the occupancy.
- **P3** (Concrete State Over Abstract Scores): `BelievedActivity` records what the agent was seen doing (concrete action domain + target), not an abstract "busyness score."
- **Scenario E** (Competing Claimants): Multiple agents perceiving the same scarce resource should resolve contention through observable world state (visible activity), not hidden reservation or blind racing.

## Design Goals

1. **Observable occupancy**: Co-located agents can see what active action another agent is performing.
2. **Fidelity-gated**: Agents with low `observation_fidelity` may fail to notice activity (P20 diversity).
3. **Ranking influence, not suppression**: Observed competition discounts an opportunity's ranking score but does not suppress it. Agents may still choose to compete.
4. **Profile-driven weighting**: Per-agent `activity_awareness_weight` controls how much observed competition influences decisions (P20).
5. **Belief-mediated**: Activity observations are stored in the agent's belief store as `BelievedActivity`, not read from authoritative state.

## Current Shape

- `perception.rs` receives `active_actions: &Scheduler` (or equivalent) but prefixes it with `_` (unused).
- `BelievedEntityState` has no activity field.
- `GoalBeliefView` exposes no activity queries.
- Candidate generation and ranking have no concept of observed competition.
- Facility queues (S23 era) prevent same-facility dogpiles but not same-source harvest/trade dogpiles.

## Deliverables

### 1. `BelievedActivity` struct (worldwake-core)

```rust
/// What the agent was last observed doing at their location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BelievedActivity {
    /// The domain of the observed action (Needs, Production, Trade, Combat, etc.).
    pub action_domain: ActionDomain,
    /// The target of the action, if observable (e.g., the source being harvested,
    /// the entity being traded with).
    pub target: Option<EntityId>,
    /// Tick when this activity was observed.
    pub observed_tick: Tick,
}
```

### 2. `BelievedEntityState` extension (worldwake-core)

Add `pub believed_activity: Option<BelievedActivity>` field to `BelievedEntityState`.

- Set by perception when a co-located agent has an active action.
- Cleared (set to `None`) when the agent is observed with no active action, or when the agent is no longer co-located.
- Staleness: `believed_activity` is only valid while the observed agent remains at the same place. When the observer or observed moves, the activity observation becomes stale and is cleared on next passive perception pass.

### 3. Perception system extension (worldwake-systems)

In `observe_passive_local_entities()`, after recording entity position/inventory/wounds:

1. For each co-located agent with an active action in the scheduler:
   - Roll `observation_fidelity` check (same probability gate as entity observation).
   - If passed: Set `believed_activity` to `BelievedActivity { action_domain, target, observed_tick: current_tick }`.
   - If failed: Leave `believed_activity` unchanged (stale or None).
2. For co-located agents with no active action: Set `believed_activity = None`.

The `active_actions` parameter (currently `_active_actions`) is unwrapped and used.

### 4. `GoalBeliefView` extension (worldwake-sim)

Add to `GoalBeliefView` trait:

```rust
/// Returns the believed activity of the specified entity, if any.
fn believed_activity_of(&self, entity: EntityId) -> Option<&BelievedActivity>;

/// Returns all observed agents at the given place who are performing actions
/// in the specified domain, optionally targeting a specific entity.
fn agents_active_at(
    &self,
    place: EntityId,
    domain: ActionDomain,
    target: Option<EntityId>,
) -> Vec<EntityId>;
```

### 5. Ranking discount (worldwake-ai)

In the ranking pipeline, after motive score computation and before feasibility:

For each `GroundedGoal` with a place-anchored opportunity:
1. Query `agents_active_at(place, relevant_domain, relevant_target)` from belief view.
2. If competitors > 0: Apply discount `motive = motive * (Permille(1000) - activity_awareness_weight * min(competitors, 3))`.
3. The discount reduces attractiveness of contested opportunities but never drops motive below `Permille(1)`.

`activity_awareness_weight` from `UtilityProfile` (new field, default `Permille(200)`):
- At default: 1 competitor → 20% discount, 2 → 40%, 3+ → 60%.
- Agents with `Permille(0)` ignore competition entirely.
- Agents with `Permille(500)` strongly avoid competition.

### 6. Save/load

`BelievedActivity` must serialize as part of `AgentBeliefStore`. No `SAVE_FORMAT_VERSION` bump needed if `BelievedActivity` is added as an `Option` field (deserializes as `None` for old saves).

## Component Registration

- `BelievedActivity`: Value type on `BelievedEntityState`, no separate component registration.
- `activity_awareness_weight`: New field on existing `UtilityProfile` component.

## FND-01 Section H Analysis

### Information-path analysis
Agent A performs action at place → scheduler records active action → perception system runs for co-located agent B → B's `observation_fidelity` check passes → B's belief store records `BelievedActivity` for A → B's ranking reads activity through belief view → B's motive score discounted for that opportunity. Path is: action → scheduler → perception → belief → ranking. All local, all through existing perception infrastructure.

### Positive-feedback analysis
Mild negative feedback loop (self-correcting): observed competition → agents avoid contested resources → resources become uncontested → agents return. This is desirable behavior, not a problem. No amplifying loops.

### Concrete dampeners
- Observation fidelity gate: low-fidelity agents miss activity signals.
- `activity_awareness_weight`: per-agent control over discount magnitude.
- Discount cap at 3 competitors: prevents complete suppression.
- Staleness: activity beliefs clear when agents move apart.

### Stored state vs. derived read-model list
- **Stored**: `BelievedActivity` on `BelievedEntityState` (belief store, per-agent). `activity_awareness_weight` on `UtilityProfile` (per-agent component).
- **Derived**: Ranking discount (recomputed each tick). `agents_active_at()` query result (recomputed from belief store).

## Tests

### Focused tests
- [ ] `BelievedActivity` set when co-located agent has active action and fidelity check passes
- [ ] `BelievedActivity` not set when fidelity check fails
- [ ] `BelievedActivity` cleared when observed agent has no active action
- [ ] `BelievedActivity` cleared when observed agent departs (no longer co-located)
- [ ] `agents_active_at()` returns correct co-located agents for domain + target
- [ ] Ranking discount applied proportionally to competitor count
- [ ] Ranking discount respects `activity_awareness_weight = Permille(0)` (no effect)
- [ ] Ranking discount capped at 3 competitors
- [ ] Motive never drops below `Permille(1)` from competition discount
- [ ] Save/load round-trip preserves `BelievedActivity`

### Golden tests
- [ ] Two agents at same harvest source — second agent (high awareness) discounts occupied source and picks alternative; first agent (low awareness) would not be deterred
- [ ] Deterministic replay companion

## Acceptance Criteria

1. Co-located agents can observe each other's active actions through the belief system.
2. Observation is gated by `observation_fidelity` — not all agents notice all activity.
3. Observed competition discounts opportunity ranking but never suppresses it.
4. `activity_awareness_weight` on `UtilityProfile` provides per-agent diversity.
5. Activity beliefs are local (same place) and stale correctly when agents move apart.
6. No telepathy — agents cannot observe activity at other locations.
7. All existing golden tests pass unchanged.
