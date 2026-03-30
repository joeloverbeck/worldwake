# E19: Guard & Patrol Adaptation

**Status**: ✅ COMPLETED

## Epic Summary
Implement guard patrol routes, belief-driven intensity scaling, threat-based route adaptation, and the public order feedback loop. Guard crime response (investigation, accusation, punishment) is already delivered by E17 and the standard AI pipeline — this epic adds only the patrol layer.

## Phase
Phase 4: Group Adaptation, CLI & Verification

## Crate
`worldwake-systems` (Patrol action handler), `worldwake-core` (PatrolRoute, PatrolProfile components), `worldwake-ai` (patrol candidate generation)

## Dependencies
- E16 (public order metric, offices, factions)
- E16b (contested-office control state for patrol escalation)
- E16c (institutional beliefs & record consultation for vacancy/crime awareness)
- E17 (crime mechanics — SuspectedTheft violations, JusticeDispositionProfile, CrimeRegister; guard crime response already delivered)

---

## Deliverables

### 1. PatrolRoute Component

New component in `worldwake-core` (new file: `patrol.rs`):

```rust
pub struct PatrolRoute {
    /// Ordered list of places this guard visits.
    pub assigned_places: Vec<EntityId>,
    /// Current position in the route. Persistent — survives patrol interruptions
    /// so the guard resumes from where they left off. Analogous to HomeostaticNeeds
    /// storing mutable need levels as authoritative state.
    pub current_index: usize,
}
```

- Registered in `component_schema.rs` for `EntityKind::Agent`.
- `current_index` is authoritative stored state (not derived). It persists across action interruptions so a guard who stops to eat or fight resumes patrol from the correct waypoint.
- `assigned_places` references `EntityKind::Place` entities in the topology graph.

### 2. PatrolProfile Component

New per-agent profile component in `worldwake-core/src/patrol.rs`:

```rust
pub struct PatrolProfile {
    /// Minimum patrol dwell at each waypoint.
    /// Higher = guard spends more time standing watch before advancing.
    pub base_dwell_ticks: u32,
    /// Additional dwell contributed by vigilance at each waypoint.
    /// Final dwell is `base_dwell_ticks + vigilance * dwell_vigilance_scale_ticks / 1000`.
    pub dwell_vigilance_scale_ticks: u32,
    /// How thoroughly a guard observes at each stop (0–1000).
    /// Higher vigilance = longer dwell time at each waypoint, increasing the
    /// chance of witnessing crimes or receiving reports, but slowing route completion.
    pub vigilance: Permille,
    /// How quickly crime reports shift patrol routes (0–1000).
    /// High sensitivity = guard rapidly adds reported crime locations to their route.
    /// Low sensitivity = guard sticks to their assigned route unless evidence is overwhelming.
    pub route_adaptation_sensitivity: Permille,
    /// Motive weight for patrol goals. Competes with survival, enterprise, and
    /// justice goals via the standard UtilityProfile weighting system.
    pub patrol_motive_weight: Permille,
}
```

- Registered in `component_schema.rs` for `EntityKind::Agent`.
- Guards differ in patrol dwell, attentiveness, and adaptability through concrete per-agent parameters.

### 3. Patrol Action

New action registered in `worldwake-systems` (new file: `patrol_actions.rs`), registered in `action_registry.rs`.

**Identity:**
- Name: `"patrol"`
- Domain: `ActionDomain::Generic` (patrol is a general duty, not combat/trade/production)

**Preconditions (Principle 8):**
- Actor has `PatrolRoute` component with at least one assigned place.
- Actor has `PatrolProfile` component.
- Actor is at the current waypoint (`assigned_places[current_index]`), OR the planner's Travel op gets the guard there first.

**Duration:**
- Dwell phase: `dwell_ticks = base_dwell_ticks + (vigilance.value() * dwell_vigilance_scale_ticks / 1000)`. The dwell represents the guard standing watch, observing, and being available for reports.
- The full patrol cycle is: Travel to waypoint (separate Travel action) → Patrol dwell (this action) → advance `current_index` → next cycle via replanning.

**Cost:**
- Body cost: fatigue accumulates during dwell (same as other time-occupying actions).
- Time occupancy: the guard cannot do other things while on patrol dwell.

**Interruptibility:**
- Yes. Combat threats, critical needs (starvation, severe fatigue), and higher-priority goals (justice pursuit of a co-located suspect) can interrupt patrol.
- On interruption: `current_index` is NOT advanced. Guard resumes from same waypoint next patrol cycle.

**Commit behavior:**
- Advances `PatrolRoute.current_index` to `(current_index + 1) % assigned_places.len()`.
- Emits a `Patrolled` event tag for the waypoint (enables perception by co-located agents).

**Visibility:**
- `SamePlace` — co-located agents perceive the guard is patrolling.

### 4. Patrol Candidate Generation

New function `emit_patrol_candidates()` in `worldwake-ai/src/candidate_generation.rs`, called from `generate_candidates_with_travel_horizon()`.

**Pattern:** Follows existing `emit_need_candidates()` / `emit_justice_candidates()` pattern.

**Logic:**
1. Check if agent has `PatrolRoute` and `PatrolProfile`.
2. Compute patrol motive from `PatrolProfile.patrol_motive_weight`.
3. **Belief-driven urgency modifiers** (Principle 14 — no world state queries):
   - Count unresolved `SuspectedTheft` violations in agent's `ViolationMemory` → more known crimes = higher urgency.
   - Check `believed_office_holder()` for offices in jurisdiction: if agent believes office is vacant (via institutional beliefs from E16c), urgency increases.
   - Check for `InstitutionalClaim::ForceControl` beliefs: if agent believes an office is contested (E16b), urgency increases.
4. Emit `GoalKind::Patrol { place: next_waypoint }` candidate with computed motive.
5. Planner produces plan: `[Travel(to waypoint), Patrol(dwell)]`.

**New GoalKind variant:**
```rust
GoalKind::Patrol { place: EntityId }
```

**New PlannerOpKind variant:**
```rust
PlannerOpKind::Patrol
```

With semantics: `may_appear_mid_plan: false`, `is_materialization_barrier: false`, `transition_kind: GoalModelFallback`.

### 5. Belief-Driven Intensity Scaling

Guards do NOT query `public_order()` or any world-state derived metric. Instead, patrol urgency emerges from the guard's belief state:

| Belief Source | Effect on Patrol Motive | Mechanism |
|---|---|---|
| Unresolved crime reports (ViolationMemory) | Increases urgency | More known crimes → guard feels patrol is more needed |
| Believed office vacancy (E16c institutional beliefs) | Increases urgency | Guard believes authority is weakened → more vigilant |
| Believed contested office (E16b force-control beliefs) | Increases urgency | Political instability → heightened alertness |
| No recent crime reports, stable institutions | Base urgency | Guard patrols at normal cadence |

The `public_order()` function in `offices.rs` is extended with a `guard_presence_factor()` contribution: the presence of patrolling guards at a place increases the derived public order value. This is a **derived view for designers/CLI** — agents never read it.

```rust
// Extension to public_order() in offices.rs:
// Add after existing vacancy/hostility penalties:
fn guard_presence_factor(place: EntityId, world: &World) -> Permille {
    let patrolling_guards = world.entities_effectively_at(place)
        .filter(|e| world.get_component_patrol_route(*e).is_some())
        .count();
    Permille::new((patrolling_guards as u16 * GUARD_PRESENCE_BONUS).min(MAX_GUARD_ORDER_BONUS))
}
```

### 6. Route Adaptation

Guards modify their patrol routes based on belief state, not world state:

**Adding waypoints:**
- When a guard receives a crime report (via Tell or personal observation) about a place within their jurisdiction, and `route_adaptation_sensitivity` exceeds a threshold relative to the report's recency, the place is added to `assigned_places` if not already present.
- Implementation: In the patrol candidate generation or as part of a per-tick patrol system hook, check new violations in `ViolationMemory` against current route.

**Deprioritizing waypoints:**
- Places that the guard has not received crime reports about within a staleness window (derived from `route_adaptation_sensitivity`) may be moved to the end of the route or skipped.
- No waypoints are permanently removed — only reordered. Route shrinkage requires explicit reassignment (future captain orders system).

**Belief-only guarantee (Principle 14):**
- Route adaptation reads only from the guard's `ViolationMemory`, `known_social_observations`, and institutional beliefs.
- A guard at a remote location does NOT learn about crimes at their patrol route until information reaches them via Tell, perception, or record consultation.

### 7. Guard Integration with Office System

- Guards are loyal to the office holder via `LoyalTo` relation (existing).
- When a guard believes the ruler has changed (via institutional belief update from E16c), the guard's patrol behavior may shift:
  - If the new holder is from a different faction, the guard's loyalty check determines whether they continue serving.
  - If the office is vacant, belief-driven urgency increases patrol motive (Section 5).
  - If the office is contested (E16b), guards with `PatrolProfile` escalate patrol frequency around the jurisdiction.
- **Captain orders are deferred to a future epic.** E19 guards operate autonomously based on their beliefs and profiles. A future system may add an `AssignPatrolRoute` action where a captain can modify subordinate routes.

### 8. Public Order Feedback Loop

The feedback loop operates through real agent decisions, not scripted responses:

```
Crime occurs at place
  → Witnesses Tell guards (E17 pipeline)
    → Guard's ViolationMemory grows
      → Guard's patrol motive increases (Section 5)
        → Guard patrols more frequently
          → Guard presence at place increases
            → public_order(place) rises (Section 5 guard_presence_factor)
              → Fewer crimes occur (thieves avoid guarded places via their own beliefs)
                → Guard receives fewer reports
                  → Patrol motive decreases
                    → Guard patrols less frequently
```

**This is a stabilizing negative feedback loop.** It self-corrects: crime causes patrols, patrols suppress crime.

---

## FND-01 Section H: Foundational Analysis

### H.1 Information-Path Analysis (Principle 7)

| Information | Source | Path to Guard | Latency |
|---|---|---|---|
| Crime at location | Victim/witness perception | Witness → Tell → guard's belief store | Travel time of witness + Tell action duration |
| Office vacancy | Institutional event (death/removal) | Perception at jurisdiction → Tell chain → guard's institutional beliefs | Co-located: same tick. Remote: travel time of messenger |
| Office contested | PressForceClaim event (E16b) | Perception at jurisdiction → institutional belief projection → guard | Co-located: same tick. Remote: Tell chain |
| Bandit sighting | Observer perception | Observer → Tell → guard | Travel time of observer + Tell duration |
| Area cleared | Guard's own patrol observation | Direct perception during patrol dwell | Immediate (guard is co-located) |

No guard acquires information without a traceable carrier path.

### H.2 Positive-Feedback Analysis (Principle 11)

**Loop 1: Crime-Patrol Escalation (NEGATIVE — self-correcting)**
Crime ↑ → Reports ↑ → Patrol motive ↑ → Guard presence ↑ → Crime ↓ → Reports ↓ → Patrol motive ↓

This is a negative feedback loop, not positive. No dampener needed for the loop itself.

## Outcome

- Completion date: 2026-03-30
- What actually changed:
  - `PatrolRoute`/`PatrolProfile`, patrol action execution, patrol candidate generation/ranking, patrol route adaptation, and `public_order()` guard-presence contribution all landed in the live codebase.
  - End-to-end patrol verification now lives in [`crates/worldwake-ai/tests/golden_patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs), with replay, interruption, urgency-scaling, adaptation, and locality coverage.
  - AI runtime/planning now snapshots patrol-route state explicitly, and patrol snapshot continuation is opportunity-scoped.
  - Patrol affordance generation now matches authoritative legality by only surfacing patrol when the actor is at the current waypoint.
- Deviations from original plan:
  - The spec's full "public order feedback loop" was not completed as written. The live thief architecture still uses local witness deterrence rather than a canonical `public_order()` consumer, so the archived completion scope stops at guard-side patrol architecture plus the derived `public_order()` bonus.
  - The route-adaptation proof surface settled on decision traces, action traces, and authoritative `PatrolRoute` state rather than a larger settlement-wide convergence scenario.
- Verification results:
  - `cargo test -p worldwake-systems patrol_actions -- --nocapture`
  - `cargo test -p worldwake-ai golden_patrol -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`

**Loop 2: Route Bloat (POSITIVE — requires dampener)**
More crime reports → More waypoints added to route → Longer route completion time → Less time at each waypoint → Crimes go unwitnessed → More crime reports

**Dampeners for Loop 2:**
- **Travel time**: Each added waypoint costs real travel ticks. A guard with 10 waypoints spends most of their time traveling, not observing. This physical cost naturally limits route expansion.
- **Guard needs**: Hunger, fatigue, and sleep compete with patrol goals. A guard who patrols too aggressively will need to eat and sleep, creating natural gaps.
- **Route adaptation sensitivity**: Per-agent `route_adaptation_sensitivity` parameter controls how aggressively waypoints are added. Low-sensitivity guards resist route bloat.
- **Finite guards**: A settlement has a fixed number of guards. Adding more waypoints to one guard's route doesn't create more guards.

### H.3 Stored State vs. Derived Views

**Authoritative Stored State (Components/Relations):**
- `PatrolRoute` — route assignment and current position
- `PatrolProfile` — per-agent patrol behavior parameters
- `PatrolRoute.current_index` — persistent patrol progress

**Derived Views (Recomputed, Never Stored):**
- `public_order(place)` with `guard_presence_factor()` — derived from co-located guard count
- Patrol urgency/motive — derived from guard's belief state each candidate generation cycle
- Route adaptation decisions — derived from ViolationMemory contents at decision time

### H.4 Principle 30 Declaration: Causal Hooks

1. **Entities introduced**: `PatrolRoute` component, `PatrolProfile` component. No new entity kinds.
2. **Mutations**: Patrol commit advances `current_index`. Route adaptation modifies `assigned_places`. Both via `WorldTxn`.
3. **Information produced**: `Patrolled` event tag at waypoint, visible to co-located agents via `SamePlace` perception.
4. **Conserved quantities**: None. Patrol does not create, transfer, or destroy items.
5. **Scarce capacities**: Guard's action occupancy (one action at a time). No new reservation/queue system.
6. **Partial failures**: Interrupted patrol does not advance index. Guard may fail to reach waypoint (blocked route, combat). Patrol at empty location still completes (guard observes nothing noteworthy).
7. **Positive feedback loops**: Route bloat loop (Section H.2). Dampened by travel time, needs, sensitivity parameter, finite guards.
8. **Physical dampeners**: Travel time, guard needs (hunger/fatigue/sleep), finite guard count, per-agent sensitivity parameter.
9. **Derived views**: `public_order()` guard presence factor, patrol urgency motive.
10. **Agent error**: Guard may patrol stale route (crime moved elsewhere). Guard may miss crimes during travel between waypoints. Guard with low `vigilance` may not witness co-located crimes during dwell. Guard beliefs about vacancy/contestation may be outdated.
11. **Temporal resolution**: Patrol dwell is tick-based duration. Route adaptation happens at candidate generation (per-tick when guard needs new goal). Patrol index advances on action commit.
12. **Boundary conditions**: Guards with no `PatrolRoute` never generate patrol candidates. Guards at off-map boundary places patrol normally (boundary places are real topology nodes).
13. **Validation patterns**: Golden tests verify patrol cycle completion, route adaptation on crime reports, belief-driven urgency changes, public order feedback loop convergence.
14. **Save/load/replay**: `PatrolRoute` and `PatrolProfile` are serde-serializable components. `current_index` persists through save/load. Deterministic replay preserved (no HashMap, no floats, no wall-clock time).

---

## Tests

### Patrol Mechanics
- [ ] Guard with PatrolRoute visits assigned places in order (current_index advances on commit)
- [ ] Interrupted patrol preserves current_index (guard resumes from correct waypoint)
- [ ] Patrol dwell duration scales with PatrolProfile.vigilance
- [ ] Guard without PatrolRoute never generates Patrol candidates
- [ ] Patrol action emits Patrolled event tag visible to co-located agents

### Belief-Driven Intensity
- [ ] Guard who receives crime report via Tell has higher patrol motive than guard with no reports
- [ ] Guard who believes office is vacant has higher patrol motive than guard with stable institution
- [ ] Guard who believes office is contested (E16b) increases patrol urgency
- [ ] Guard at remote location does NOT increase urgency for crime they haven't heard about (Principle 7)

### Route Adaptation
- [ ] Crime report at new location adds that place to guard's patrol route
- [ ] Route adaptation respects route_adaptation_sensitivity (low sensitivity = no route change)
- [ ] Route adaptation reads from guard's beliefs only, not world state
- [ ] Guard with many waypoints spends proportionally more time traveling (natural dampener)

### Public Order Feedback Loop
- [ ] Guard presence at a place increases derived public_order() value
- [ ] More patrols → higher order → (via thief belief system) fewer thefts → fewer reports → lower patrol urgency: feedback loop converges
- [ ] Removal of guards from a place causes public_order() to drop

### Office Integration
- [ ] Guard loyal to office holder continues patrol when holder is alive
- [ ] Guard's patrol urgency increases when guard believes office is vacant
- [ ] Guard's belief about office state updates only through legitimate information paths (Tell, perception, record consultation)

## Acceptance Criteria
- Guards patrol real routes through the world graph with persistent progress
- Patrol behavior adapts to threats via belief state, never world state
- Per-agent PatrolProfile enables diverse guard behavior (Principle 22)
- Public order feedback loop creates emergent stability/instability through real agent decisions
- No scripted patrol changes, no omniscient intensity queries
- All dampeners are physical (travel time, needs, finite guards), not numeric caps
- Guard crime response remains delivered by E17 — no duplicate crime-response code in E19
- Captain-mediated patrol orders deferred to future epic

## Cross-References
- **E17 (Crime/Justice)**: Guards respond to crime through `emit_justice_candidates()`, `Accuse`, `PunishAccused`, `InvestigateViolation` — all delivered by E17. E19 does NOT reimplement crime response.
- **E16 (Public Order)**: E19 extends `public_order()` with `guard_presence_factor()`.
- **E16b (Force Legitimacy)**: E19 reads `InstitutionalClaim::ForceControl` beliefs for contested-office awareness.
- **E16c (Institutional Beliefs)**: E19 reads `believed_office_holder()` for vacancy awareness. Guards use `ConsultRecord` for CrimeRegister access.

## Spec References
- Section 4.5 (core systems include guard behavior)
- Section 7.4 (institutional propagation: law enforcement, patrol intensity)
- Section 3.9 (public-order consequences)
