# S35: Observable Activity Signals

**Status**: COMPLETED

## Summary

Make active actions observable through the perception system so co-located agents can see what others are doing. Currently `perception_system()` receives `active_actions` and `action_defs` via `SystemExecutionContext` but ignores both (`_active_actions`, `_action_defs`). This means agents cannot observe "Bob is harvesting at the orchard" and consequently cannot avoid dogpiling on contested resources. Introduce `BelievedActivity` on `BelievedEntityState`, a new `observe_active_actions()` perception helper, a ranking discount for observed competition in Production and Trade goal families, and decision trace support for the discount.

## Source

Derived from ChatGPT architecture review WW-AI-005 (Tick phasing and visible local coordination), filtered to the observable activity component only. Tick phase restructuring is not needed — the current implicit phase ordering (belief refresh before deliberation) is already correct. The genuine gap is that agents are blind to co-located activity.

## Phase

Phase 3+: AI Architecture Overhaul, Step 13.5 Wave 5

## Crates

- `worldwake-core` (new belief field, new profile field)
- `worldwake-systems` (perception extension)
- `worldwake-sim` (belief view extension)
- `worldwake-ai` (ranking discount, decision trace)

## Dependencies

- E14 ✅ (perception & belief — provides `AgentBeliefStore`, `BelievedEntityState`, perception system)

## FOUNDATIONS Alignment

- **P3** (Concrete State Over Abstract Scores): `BelievedActivity` records what the agent was seen doing (concrete action domain + target entity), not an abstract "busyness score."
- **P7** (Locality of Interaction): Observing activity at your location IS local observation. This does not add telepathy — agents only see actions at their current place.
- **P8** (Every Action Has Occupancy): Actions occupy the actor visibly. If harvesting occupies a workstation, co-located agents should be able to see the occupancy.
- **P12** (World State Is Not Belief State): Activity observations are stored in agent beliefs, never read directly from authoritative scheduler state during planning or ranking.
- **P20** (Agent Diversity): `observation_fidelity` gates who notices activity; `activity_awareness_weight` controls how much observed competition influences decisions.
- **P24** (Systems Through State): Perception reads the uniform `ActionInstance` + `ActionDef` interface to extract domain and targets. No coupling to `ActionPayload` variant internals or action handler logic.
- **P27** (Debuggability): Decision traces record competition discount details — which agents were competing, what domain, how much discount applied — so "why did the agent avoid this resource?" is answerable from trace data.
- **Scenario E** (Competing Claimants): Multiple agents perceiving the same scarce resource should resolve contention through observable world state (visible activity), not hidden reservation or blind racing.

## Design Goals

1. **Observable occupancy**: Co-located agents can see what active action another agent is performing.
2. **Fidelity-gated**: Agents with low `observation_fidelity` may fail to notice activity (P20 diversity).
3. **Ranking influence, not suppression**: Observed competition discounts an opportunity's ranking score but does not suppress it. Agents may still choose to compete.
4. **Profile-driven weighting**: Per-agent `activity_awareness_weight` controls how much observed competition influences decisions (P20).
5. **Belief-mediated**: Activity observations are stored in the agent's belief store as `BelievedActivity`, not read from authoritative state.
6. **Scoped discount**: Competition discount applies only to Production and Trade goal families where resource/merchant contention is meaningful.
7. **Traceable**: Decision traces record competition discount details for debuggability (P27).

## Current Shape

- `perception_system()` in `perception.rs` receives `active_actions: &BTreeMap<ActionInstanceId, ActionInstance>` and `action_defs: &ActionDefRegistry` via `SystemExecutionContext` but destructures both as unused (`_active_actions`, `_action_defs`).
- `observe_passive_local_entities()` is a separate helper that receives `(world, event_log, tick, rng, updated_stores)` — it does NOT receive active actions.
- `ActionInstance` carries `def_id: ActionDefId` but not `ActionDomain` directly. Domain is resolved via `action_defs.get(def_id).domain`.
- `ActionInstance.targets: Vec<EntityId>` contains the action's target entities (resource source for harvest, counterparty for trade, opponent for combat).
- `BelievedEntityState` has no activity field.
- `GoalBeliefView` exposes no activity queries. The trait is in `belief_view.rs` with a macro-based delegation in `impl_goal_belief_view!`.
- Candidate generation and ranking have no concept of observed competition.
- `motive_score` in ranking is `u32` (from `score_product(weight: Permille, pressure: Permille) -> u32`).
- Facility queues (S23 era) prevent same-facility dogpiles but not same-source harvest/trade dogpiles.

## Deliverables

### 1. `BelievedActivity` struct (worldwake-core)

```rust
/// What an agent was last observed doing at their location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BelievedActivity {
    /// The domain of the observed action (Needs, Production, Trade, Combat, etc.).
    pub action_domain: ActionDomain,
    /// The primary target of the action, if observable (e.g., the resource source
    /// being harvested, the entity being traded with).
    /// Extracted uniformly from `ActionInstance.targets.first()` (P24 — no payload coupling).
    pub target: Option<EntityId>,
    /// Tick when this activity was observed.
    pub observed_tick: Tick,
}
```

### 2. `BelievedEntityState` extension (worldwake-core)

Add `pub believed_activity: Option<BelievedActivity>` field to `BelievedEntityState`.

- Set by perception when a co-located agent has an active action and fidelity check passes.
- Cleared (set to `None`) when the agent is observed with no active action, or when the agent is no longer co-located.
- Staleness: `believed_activity` is only valid while the observed agent remains at the same place. When the observer or observed moves, the activity observation becomes stale and is cleared on next passive perception pass.

### 3. Perception system extension (worldwake-systems)

Add a new function `observe_active_actions()` called from `perception_system()`:

```rust
fn observe_active_actions(
    world: &World,
    tick: Tick,
    rng: &mut DeterministicRng,
    active_actions: &BTreeMap<ActionInstanceId, ActionInstance>,
    action_defs: &ActionDefRegistry,
    updated_stores: &mut BTreeMap<EntityId, AgentBeliefStore>,
)
```

Called from `perception_system()` after `observe_passive_local_entities()`, using the `active_actions` and `action_defs` parameters from `SystemExecutionContext` (currently unused — remove `_` prefix).

**Logic:**

1. For each agent with a `PerceptionProfile` and a belief store in `updated_stores`:
   - Determine the agent's current place.
   - For each active action in `active_actions` whose actor is co-located (at the same place) and is not the observing agent:
     - Roll `passes_observation_check(profile.observation_fidelity, rng)`.
     - If passed: Look up `action_defs.get(instance.def_id)` to get `ActionDomain`. Extract target via `instance.targets.first().copied()`. Set `believed_activity = Some(BelievedActivity { action_domain, target, observed_tick: tick })` on the actor's `BelievedEntityState`.
     - If failed: Leave `believed_activity` unchanged (stale or `None`).
   - For co-located agents with no active action in the scheduler: Set `believed_activity = None` on their `BelievedEntityState`.

### 4. `GoalBeliefView` extension (worldwake-sim)

Add to `GoalBeliefView` trait and `impl_goal_belief_view!` macro:

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

`agents_active_at()` iterates `known_entities` in the agent's belief store, filtering for entities whose `last_known_place == Some(place)` and whose `believed_activity` matches the domain and (if specified) the target.

### 5. Ranking discount (worldwake-ai)

In `rank_candidates()` (ranking.rs), after motive score computation and before final `RankedGoal` construction:

For each candidate with a discountable goal kind:

1. **Determine competition domain and query parameters** using this mapping:

   | GoalKind | ActionDomain to check | Place source | Target filter |
   |----------|----------------------|--------------|---------------|
   | `ProduceCommodity` | `Production` | `anchor.place()` | `None` (any production at same place) |
   | `RestockCommodity` | `Production` | `anchor.place()` | `None` |
   | `AcquireCommodity` (trade-bound) | `Trade` | agent's place | `anchor.entity()` (specific merchant) |

   All other goal kinds: no discount applied.

2. Query `view.agents_active_at(place, domain, target)` from belief view.

3. If `competitors > 0`: Apply discount:
   ```rust
   let weight_val = u32::from(utility.activity_awareness_weight.value());
   let count = competitors.min(3) as u32;
   let factor = 1000u32.saturating_sub(weight_val * count);
   motive_score = (motive_score * factor / 1000).max(1);
   ```

4. Record `CompetitionDiscount` trace if tracing is enabled.

`activity_awareness_weight` on `UtilityProfile` (new field, default `Permille(200)`):
- At default: 1 competitor -> 20% discount, 2 -> 40%, 3+ -> 60%.
- Agents with `Permille(0)` ignore competition entirely.
- Agents with `Permille(500)` strongly avoid competition.

### 6. Decision trace extension (worldwake-ai)

```rust
/// Records the competition discount applied to a ranked goal's motive score.
#[derive(Debug, Clone)]
pub struct CompetitionDiscount {
    /// Agents observed competing for the same opportunity.
    pub observed_competitors: Vec<EntityId>,
    /// The action domain checked for competition.
    pub domain: ActionDomain,
    /// The effective discount as permille (e.g., Permille(200) = 20% discount).
    pub effective_discount: Permille,
    /// Motive score before discount.
    pub pre_discount_motive: u32,
    /// Motive score after discount.
    pub post_discount_motive: u32,
}
```

Added as `pub competition_discount: Option<CompetitionDiscount>` on `RankedGoal`. Zero-cost (`None`) when no competitors observed or goal kind is not discountable. Rendered by `dump_agent()` when present.

### 7. Save/load

`BelievedActivity` must serialize as part of `AgentBeliefStore` (via `BelievedEntityState`) when that field lands. `activity_awareness_weight` on `UtilityProfile` is a forward-only schema addition on an existing serialized component: current-head constructors, fixtures, scenarios, and save payloads must be updated to include the field. Do not use `#[serde(default)]` or any other backward-compatibility shim for the new `UtilityProfile` field.

## Component Registration

- `BelievedActivity`: Value type on `BelievedEntityState`, no separate component registration.
- `activity_awareness_weight`: New field on existing `UtilityProfile` component.
- `CompetitionDiscount`: Trace-only struct, no component registration.

## FND-01 Section H Analysis

### Information-path analysis
Agent A performs action at place -> scheduler records active `ActionInstance` with `def_id` and `targets` -> perception system runs for co-located agent B -> `observe_active_actions()` resolves `ActionDomain` via `action_defs.get(def_id)` and target via `instance.targets.first()` -> B's `observation_fidelity` check passes -> B's belief store records `BelievedActivity` for A -> B's ranking reads activity through `GoalBeliefView.agents_active_at()` -> B's motive score discounted for that opportunity. Path is: action -> scheduler -> perception -> belief -> ranking. All local, all through existing perception infrastructure.

### Positive-feedback analysis
Mild negative feedback loop (self-correcting): observed competition -> agents avoid contested resources -> resources become uncontested -> agents return. This is desirable behavior, not a problem. No amplifying loops.

### Concrete dampeners
- **Observation fidelity gate**: Low-fidelity agents miss activity signals (physical dampener: imperfect perception).
- **`activity_awareness_weight`**: Per-agent control over discount magnitude (physical dampener: individual temperament/stubbornness).
- **Competitor count cap at 3**: Prevents extreme suppression from large crowds.
- **Staleness**: Activity beliefs clear when agents move apart (physical dampener: line-of-sight locality).
- **Discount floor**: Motive never drops below 1, so opportunities are never fully suppressed by competition alone.

### Stored state vs. derived read-model list
- **Stored**: `BelievedActivity` on `BelievedEntityState` (belief store, per-agent). `activity_awareness_weight` on `UtilityProfile` (per-agent component).
- **Derived**: Ranking discount (recomputed each tick from belief queries). `agents_active_at()` query result (recomputed from belief store). `CompetitionDiscount` trace data (ephemeral, not persisted).

## Tests

### Focused tests
- [ ] `BelievedActivity` set when co-located agent has active action and fidelity check passes
- [ ] `BelievedActivity` not set when fidelity check fails
- [ ] `BelievedActivity` cleared when observed agent has no active action
- [ ] `BelievedActivity` cleared when observed agent departs (no longer co-located)
- [ ] `agents_active_at()` returns correct co-located agents for domain + target filter
- [ ] `agents_active_at()` returns empty when no competitors in specified domain
- [ ] Ranking discount applied proportionally to competitor count (1, 2, 3)
- [ ] Ranking discount respects `activity_awareness_weight = Permille(0)` (no effect)
- [ ] Ranking discount capped at 3 competitors (4th competitor adds no further discount)
- [ ] Motive never drops below 1 from competition discount
- [ ] Discount applies to `ProduceCommodity` / `RestockCommodity` goals
- [ ] Discount applies to `AcquireCommodity` trade-bound goals
- [ ] Discount does NOT apply to Needs goals (`SelfConsumeCommodity`, `Sleep`, etc.) even with co-located activity
- [ ] `CompetitionDiscount` trace populated when discount applied
- [ ] `CompetitionDiscount` trace absent (`None`) when no competitors or non-discountable goal
- [ ] Save/load round-trip preserves `BelievedActivity`

### Golden tests
- [ ] Two agents at same harvest source — second agent (high `activity_awareness_weight`) discounts occupied source and picks alternative; first agent (low awareness) would not be deterred
- [ ] Deterministic replay companion

## Acceptance Criteria

1. Co-located agents can observe each other's active actions through the belief system.
2. Observation is gated by `observation_fidelity` — not all agents notice all activity.
3. Observed competition discounts opportunity ranking but never suppresses it.
4. `activity_awareness_weight` on `UtilityProfile` provides per-agent diversity.
5. Activity beliefs are local (same place) and stale correctly when agents move apart.
6. No telepathy — agents cannot observe activity at other locations.
7. Discount applies only to Production and Trade goal families.
8. Decision traces record competition discount details when tracing is enabled.
9. All existing golden tests pass unchanged.

## Outcome

Completion date: 2026-03-29

What actually changed:
- Added `BelievedActivity` to `BelievedEntityState` and wired active-action observation into perception so co-located agents can record visible activity in belief state.
- Extended `GoalBeliefView` / runtime belief views with activity queries and integrated observed-competition discounts plus decision-trace reporting in the AI ranking path.
- Added `activity_awareness_weight` to `UtilityProfile` and covered the behavior with focused and golden tests, including save/load persistence proof for `BelievedActivity`.
- Bumped `SAVE_FORMAT_VERSION` to 11 to keep persisted-schema evolution explicit.

Deviations from original plan:
- Persistence landed as an explicit save-format bump rather than any missing-field compatibility shim.
- The archived implementation follows the repo's forward-only save contract: older same-version bytes are rejected rather than silently defaulting missing fields.

Verification results:
- `cargo test -p worldwake-sim save_load` passed.
- `cargo test --workspace` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
