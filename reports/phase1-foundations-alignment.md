# Phase 1 Foundations Alignment Report

**Date**: 2026-03-09
**Scope**: Phase 1 (E01-E08) — Core and Sim crates
**Methodology**: Principle-by-principle analysis of all Phase 1 code against the 13 foundational principles defined in `docs/FOUNDATIONS.md`. Evidence gathered through codebase exploration of `worldwake-core` and `worldwake-sim` modules.

---

## Scorecard

| # | Principle | Verdict | Severity | Primary Location |
|---|-----------|---------|----------|------------------|
| 1 | Maximal Emergence | **Aligned** | — | `canonical.rs`, `event_log.rs`, `cause.rs` |
| 2 | No Magic Numbers | **Violated** | Critical | `load.rs:7-38`, `topology.rs:46-47` |
| 3 | Concrete State Over Abstract Scores | **Violated** | Critical | `topology.rs:46-47` (`TravelEdge.danger`, `.visibility`) |
| 4 | Simulate Carriers of Consequence | **Aligned** | — | `items.rs`, `relations.rs`, `delta.rs` |
| 5 | World Runs Without Observers | **Aligned** | — | `tick_step.rs:93-128` |
| 6 | Every Action Has Physical Cost | **Aligned** | Minor caveat | `action_semantics.rs:36-47` |
| 7 | Locality of Information | **Violated** | Critical | `knowledge_view.rs:3-11`, `world_knowledge_view.rs:6-53` |
| 8 | Feedback Dampening | **Deferred** | Medium | No implementation yet |
| 9 | Agent Symmetry | **Aligned** | — | `control.rs:1-18` |
| 10 | Intelligent Agency | **Deferred** | — | AI crate (Phase 2: E13) |
| 11 | Agent Diversity | **Deferred** | — | AI crate (Phase 2: E13) |
| 12 | Systems Interact Through State | **Aligned** | — | `system_dispatch.rs`, `world_txn.rs` |
| 13 | No Backward Compatibility | **Aligned** | — | Codebase-wide |

**Summary**: 7 Aligned, 3 Violated, 3 Deferred

---

## I. Causal Foundations

### Principle 1: Maximal Emergence Through Causality

**Verdict**: Aligned

**Evidence**:
- The append-only `EventLog` (`crates/worldwake-core/src/event_log.rs`) is the authoritative causal record. Events are immutable once committed.
- Every `EventRecord` carries a `CauseRef` (`crates/worldwake-core/src/cause.rs`) linking it to a prior event, external input, system tick, or bootstrap. No event exists without a traceable cause.
- `WorldTxn` (`crates/worldwake-core/src/world_txn.rs:14-26`) journals all mutations as typed `StateDelta` records, committed atomically with a `PendingEvent` that captures the full causal chain.
- Canonical state hashing via `blake3` (`crates/worldwake-core/src/canonical.rs:42-61`) ensures deterministic state identity — any divergence from causal consistency is detectable.
- No scripted sequences exist anywhere in the codebase. The tick loop (`step_tick`) processes inputs, progresses actions, and runs systems — all driven by prior state, never by authored triggers.

**Analysis**: The architecture deeply encodes causality. The combination of append-only log, causal references, transactional mutations, and deterministic hashing makes it structurally difficult to introduce non-causal behavior. This is one of the strongest architectural commitments in Phase 1.

---

### Principle 2: No Magic Numbers

**Verdict**: **Violated** (Critical)

**Evidence**:

1. **Hardcoded weight lookup tables** in `crates/worldwake-core/src/load.rs:7-38`:

```rust
pub fn load_per_unit(commodity: CommodityKind) -> LoadUnits {
    match commodity {
        CommodityKind::Water => LoadUnits(2),
        CommodityKind::Firewood => LoadUnits(3),
        CommodityKind::Apple | CommodityKind::Grain | CommodityKind::Bread
        | CommodityKind::Medicine | CommodityKind::Coin | CommodityKind::Waste => LoadUnits(1),
    }
}

pub fn load_of_unique_item_kind(kind: UniqueItemKind) -> LoadUnits {
    match kind {
        UniqueItemKind::SimpleTool | UniqueItemKind::Artifact => LoadUnits(5),
        UniqueItemKind::Weapon => LoadUnits(10),
        UniqueItemKind::Contract => LoadUnits(1),
        UniqueItemKind::OfficeInsignia => LoadUnits(2),
        UniqueItemKind::Misc => LoadUnits(3),
    }
}
```

These weight values (Water=2, Firewood=3, Weapon=10, etc.) are designer-chosen constants with no derivation from world state. They are classic magic numbers — the "why" bottoms out at "the designer chose these values."

2. **Static `danger` and `visibility` on `TravelEdge`** (`crates/worldwake-core/src/topology.rs:46-47`):

```rust
pub struct TravelEdge {
    // ...
    danger: Permille,
    visibility: Permille,
}
```

These are `Permille` values set at topology construction time. They are not derived from entities present on the route — they are abstract scores assigned by the world builder.

3. **Loyalty strength as `Permille`** in `RelationValue::LoyalTo` (`crates/worldwake-core/src/delta.rs:106-110`):

```rust
LoyalTo {
    subject: EntityId,
    target: EntityId,
    strength: Permille,
}
```

Loyalty strength is stored as a raw numeric value. While this is a relation property rather than a tuning constant, it risks becoming a magic number if systems apply threshold-based logic to it (e.g., "if loyalty < 300, betray").

**Analysis**: The weight lookup tables are the clearest violation — they are the exact pattern FOUNDATIONS.md warns against: "Shortcutting with lookup tables what should emerge from agent interactions." The `danger`/`visibility` fields overlap with Principle 3 (see below). Loyalty strength is borderline; it could be acceptable if it tracks accumulated concrete interactions rather than being an arbitrary initial value.

**Recommendation**:
- **Item weights**: Move per-unit weight into a component field on `CommodityKind` or `ItemLot` itself, derived from material properties. In the prototype phase, hardcoded defaults are pragmatically acceptable if they are explicitly marked as provisional and tracked for replacement.
- **Danger/visibility**: See Principle 3 recommendation.
- **Loyalty strength**: Acceptable for Phase 1 if loyalty changes are always driven by concrete events (witnessed actions, shared hardship, betrayal). Ensure no system applies threshold logic directly to the `Permille` value.

---

### Principle 3: Concrete State Over Abstract Scores

**Verdict**: **Violated** (Critical)

**Evidence**:

`TravelEdge` in `crates/worldwake-core/src/topology.rs:40-48`:

```rust
pub struct TravelEdge {
    id: TravelEdgeId,
    from: EntityId,
    to: EntityId,
    travel_time_ticks: NonZeroU32,
    capacity: Option<NonZeroU16>,
    danger: Permille,     // <-- abstract score
    visibility: Permille, // <-- abstract score
}
```

FOUNDATIONS.md explicitly calls out this pattern: "Instead of assigning `danger_score: 0.7` to a road, model the actual bandits present on that route." The `danger` and `visibility` fields are static `Permille` values — they do not change when bandits arrive or leave, when lighting conditions change, or when patrols are posted.

**Analysis**: This is a direct violation of the stated test: "If a system uses a numeric score to represent a world condition, ask whether that condition could instead be derived from concrete entities and their states." Road danger *can* be derived from the presence of hostile entities along the route. Visibility *can* be derived from terrain type, weather, and time-of-day entities.

However, these fields are currently only used in `build_prototype_world()` for initial topology setup and are not consumed by any system logic yet (no systems exist in Phase 1). The violation is structural — the fields exist and invite score-based reasoning — but not yet behavioral.

**Recommendation**:
- Remove `danger` and `visibility` from `TravelEdge`. These properties should emerge from querying entities present along the route.
- If a "terrain difficulty" concept is needed for pathfinding, model it as a `PlaceTag` or terrain component on the place entities, not as a numeric score on the edge.
- **Priority**: Address before Phase 2 systems begin consuming these fields. If systems are built that read `TravelEdge.danger`, removing it later becomes a breaking change.

---

## II. World Dynamics

### Principle 4: Simulate Carriers of Consequence

**Verdict**: Aligned

**Evidence**:
- The entity model (`crates/worldwake-core/src/entity.rs`) includes only consequence-carrying kinds: `Agent`, `Place`, `ItemLot`, `UniqueItem`, `Container`, `Office`, `Facility`, `Faction`, `Rumor`.
- Items track provenance (`ProvenanceEntry` in `items.rs`) — every quantity change records tick, event, operation, and related lot. Items are genuine carriers of consequence with traceable history.
- Relations (`delta.rs:50-127`) model consequence-relevant connections: ownership, possession, containment, loyalty, hostility, knowledge, belief. No decorative relations.
- The `CommodityKind` enum covers only causally relevant goods: food (Apple, Grain, Bread), resources (Water, Firewood), trade goods (Coin), medicine, and waste.

**Analysis**: The entity model is lean and consequence-focused. No atmospheric or decorative entities exist. Every modeled thing can propagate downstream effects through the action/event system.

---

### Principle 5: World Runs Without Observers

**Verdict**: Aligned

**Evidence**:

`step_tick()` in `crates/worldwake-sim/src/tick_step.rs:93-128`:

```rust
pub fn step_tick(
    world: &mut World,
    event_log: &mut EventLog,
    scheduler: &mut Scheduler,
    controller: &mut ControllerState,
    rng: &mut DeterministicRng,
    services: TickStepServices<'_>,
) -> Result<TickStepResult, TickStepError> { ... }
```

The tick loop has no branch on whether a human is present. It processes inputs (which may be empty), progresses active actions, runs all registered systems, and emits an end-of-tick marker — regardless of human observation.

The test `empty_tick_increments_and_emits_end_of_tick_event` (`tick_step.rs:558-595`) confirms the tick advances with zero inputs. The test `identical_runs_produce_identical_results_and_logs` (`tick_step.rs:870-922`) proves deterministic advancement without human interaction.

`ControllerState` tracks which entity the human controls but does not gate simulation execution. If `controlled_entity()` is `None`, the simulation still advances.

**Analysis**: The architecture guarantees observer-independent execution. There is no "pause when human is absent" mechanism, and the deterministic replay system (`replay_state.rs`, `replay_execution.rs`) validates that identical seeds and inputs produce identical worlds — with or without human agents.

---

### Principle 6: Every Action Has Physical Cost

**Verdict**: Aligned (with minor caveat)

**Evidence**:

`DurationExpr` in `crates/worldwake-sim/src/action_semantics.rs:35-47`:

```rust
pub enum DurationExpr {
    Fixed(u32),
}

impl DurationExpr {
    pub const fn resolve(self) -> u32 {
        match self {
            Self::Fixed(ticks) => ticks,
        }
    }
}
```

Every `ActionDef` requires a `DurationExpr` — actions inherently cost time. The `Constraint` and `Precondition` enums enforce additional costs: `ActorHasCommodity` requires material resources, `ActorAtPlace` requires co-location (which requires travel time to reach).

**Caveat**: `DurationExpr::Fixed(0)` is a valid construction. Zero-duration actions would violate the "nothing is free" principle. In tests, `DurationExpr::Fixed(0)` appears in the constant array `ALL_DURATION_EXPRS` (`action_semantics.rs:107`), confirming it is a valid variant.

**Recommendation**: Consider enforcing `DurationExpr::Fixed(n)` where `n >= 1` at the type level (using `NonZeroU32`), or add a validation check in `ActionDefRegistry::register()` that rejects zero-duration action definitions. This is a minor issue — no production action definitions with zero duration exist yet.

---

### Principle 7: Locality of Information

**Verdict**: **Violated** (Critical)

**Evidence**:

`KnowledgeView` trait in `crates/worldwake-sim/src/knowledge_view.rs:3-11`:

```rust
pub trait KnowledgeView {
    fn is_alive(&self, entity: EntityId) -> bool;
    fn entity_kind(&self, entity: EntityId) -> Option<EntityKind>;
    fn effective_place(&self, entity: EntityId) -> Option<EntityId>;
    fn entities_at(&self, place: EntityId) -> Vec<EntityId>;
    fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity;
    fn has_control(&self, entity: EntityId) -> bool;
    fn reservation_conflicts(&self, entity: EntityId, range: TickRange) -> bool;
}
```

`WorldKnowledgeView` implementation in `crates/worldwake-sim/src/world_knowledge_view.rs:17-53`:

```rust
impl KnowledgeView for WorldKnowledgeView<'_> {
    fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
        self.world.entities_effectively_at(place)
    }

    fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
        self.world.effective_place(entity)
    }
    // ...
}
```

The `KnowledgeView` trait allows any caller to:
1. Query `entities_at(any_place)` — returns entities at *any* place in the world, regardless of the querying agent's location.
2. Query `effective_place(any_entity)` — reveals the location of any entity, regardless of whether the querier could know this.
3. Query `is_alive(any_entity)` — reveals liveness of any entity globally.
4. Query `commodity_quantity(any_holder, kind)` — reveals inventory of any entity.

`WorldKnowledgeView` wraps the authoritative `World` without any locality filtering. There is no concept of "who is asking" — the trait has no `observer: EntityId` parameter.

**Analysis**: This is the most critical architectural violation in Phase 1. FOUNDATIONS.md states: "No system may query global world state on behalf of an agent. Information propagates at finite speed." The current `KnowledgeView` gives every agent omniscient access to the entire world.

The trait is used in `affordance_query.rs` (via `get_affordances()`) and in `tick_step.rs:233-234` (via `resolve_affordance()`). Any system or AI planner built on this trait will inherit the locality violation.

**Recommendation**:
- `KnowledgeView` must become agent-scoped. Add an `observer: EntityId` parameter to the trait or its constructor.
- `WorldKnowledgeView` must filter results by the observer's perception range (same-place entities, known entities from prior percepts).
- Introduce a `PerceptStore` or `BeliefState` per agent that records what each agent has observed. `KnowledgeView` queries should read from this belief state, not from authoritative world state.
- **Priority**: Must be resolved before E13 (AI decision architecture). Building GOAP planning on omniscient world access would deeply embed the violation.

---

### Principle 8: Every Amplifying Loop Must Have a Physical Dampener

**Verdict**: **Deferred** (Medium severity)

**Evidence**: No systems exist in Phase 1 (the `worldwake-systems` crate is empty). Therefore, no feedback loops can yet form. However, the infrastructure that *would* support dampening is present:

- Event causality tracking (`CauseRef`) allows identifying feedback chains.
- Action costs (`DurationExpr`, `Constraint::ActorHasCommodity`) provide resource pressure that naturally dampens pure growth loops.
- Conservation invariants (`crates/worldwake-core/src/conservation.rs`) prevent item creation from nothing — a fundamental dampener on economic loops.

**Analysis**: The principle cannot be evaluated until systems interact. The infrastructure is favorable but no explicit dampening mechanisms (resource exhaustion curves, diminishing returns, competing pressures) are implemented.

**Recommendation**: When Phase 2 systems are implemented (E09 needs, E10 production/trade, E11 combat), each system spec must include a feedback analysis section identifying potential amplifying loops and their concrete dampeners, as required by FOUNDATIONS.md.

---

## III. Agent Architecture

### Principle 9: Agent Symmetry

**Verdict**: Aligned

**Evidence**:

`ControlSource` in `crates/worldwake-core/src/control.rs:10-18`:

```rust
/// There is no `Player` type — `ControlSource::Human` means the entity's
/// decisions come from a human player, but simulation rules treat all
/// control sources identically.
pub enum ControlSource {
    Human,
    Ai,
    None,
}
```

A grep for `ControlSource::Human` across the entire codebase (`crates/`) reveals it appears only in:
- The type definition and its doc comment (`control.rs:7,42`)
- Test code (creating agents with various control sources for testing)
- `WorldKnowledgeView::has_control()` — which checks `!= ControlSource::None`, treating Human and Ai identically

No production code branches on `ControlSource::Human` vs `ControlSource::Ai`. The affordance system (`affordance_query.rs`), action execution (`action_execution.rs`), tick step (`tick_step.rs`), and all precondition checks operate identically regardless of control source.

`ControllerState` (`controller_state.rs`) tracks which entity the human controls for input routing only — it does not affect simulation rules.

**Analysis**: Agent symmetry is structurally enforced. The `ControlSource` enum determines only the input source, never the available actions or world rules. This is reinforced by the policy test in `crates/worldwake-core/tests/policy.rs` which programmatically asserts architectural invariants.

---

### Principle 10: Intelligent Agency Over Behavioral Scripts

**Verdict**: **Deferred**

The AI crate (`worldwake-ai`) is not yet implemented. It is scheduled for Phase 2: E13 (GOAP planner + utility scoring). Phase 1 established the `KnowledgeView` trait and affordance system that the AI will consume, but no decision-making logic exists yet.

**Note**: The `KnowledgeView` violation (Principle 7) must be resolved before E13 implementation, or the AI planner will be built on omniscient world access — which would make "belief-only planning" impossible to enforce.

---

### Principle 11: Agent Diversity Through Concrete Variation

**Verdict**: **Deferred**

Per-agent parameters (need rates, utility weights, risk tolerance) are not yet implemented. The `AgentData` component currently contains only `control_source: ControlSource`. Agent diversity will be addressed in E13 when the AI architecture introduces per-agent decision parameters.

---

## IV. System Architecture

### Principle 12: Systems Interact Through State, Not Through Each Other

**Verdict**: Aligned

**Evidence**:

The `SystemFn` signature enforces state-mediated interaction. Systems receive a `SystemExecutionContext` (`tick_step.rs:324`) containing `&mut World`, `&mut EventLog`, `&mut DeterministicRng`, `Tick`, and `SystemId`. They have no access to other systems' logic.

The `worldwake-systems` crate is correctly empty in Phase 1 — no system modules exist yet to violate this principle.

The `SystemDispatchTable` (`system_dispatch.rs`) maps `SystemId` to function pointers, executed in manifest order by `run_systems()` (`tick_step.rs:313-338`). Each system runs independently with only world state as the communication channel.

The `WorldTxn` journaling system ensures that mutations from one system are atomically committed to world state before the next system reads — preventing partial-state interactions.

Cargo dependency structure enforces this at the crate level: `worldwake-systems` depends on `worldwake-core` and `worldwake-sim`, but system modules within the crate cannot depend on each other (enforced by module visibility).

**Analysis**: The architecture strongly enforces state-mediated interaction. The combination of function-pointer dispatch, `WorldTxn` atomicity, and crate-level dependency constraints makes direct system coupling structurally difficult.

---

### Principle 13: No Backward Compatibility

**Verdict**: Aligned

**Evidence**:
- Zero `#[deprecated]` annotations in the entire codebase.
- Zero `TODO` or `FIXME` comments in the entire codebase (verified by grep).
- No compatibility shims, wrapper functions, or redirect modules.
- No renamed-but-preserved exports or re-exports.
- The strict type system (custom newtypes: `Quantity`, `LoadUnits`, `Permille`, `Tick`, `EntityId`, `EventId`, `Seed`) enforces clean API boundaries — changing a type forces all consumers to update.
- Policy tests (`crates/worldwake-core/tests/policy.rs:79-84`) programmatically forbid `HashMap` and `HashSet` — demonstrating willingness to enforce constraints without backward compatibility exceptions.

**Analysis**: The codebase is clean and forward-only. No legacy code, no compatibility layers, no deprecated paths. The `lib.rs` deny list (`HashMap`, `HashSet` banned from authoritative state) shows active enforcement of design decisions without compromise.

---

## Recommendations

| Priority | Principle | Issue | Effort | Suggested Phase |
|----------|-----------|-------|--------|-----------------|
| **Critical** | P7 | `KnowledgeView` allows unrestricted global queries; must become agent-scoped with belief state | High | Phase 2 (before E13) |
| **Critical** | P3 | Remove `TravelEdge.danger` and `.visibility` static scores | Medium | Phase 2 (before E09/E10 consume them) |
| **Critical** | P2 | `load_per_unit()` and `load_of_unique_item_kind()` hardcode weight lookup tables | Medium | Phase 2 (before E09 needs system) |
| **Medium** | P8 | No dampening mechanisms; add feedback analysis to all Phase 2 system specs | Low | Phase 2 (E09-E11 specs) |
| **Minor** | P6 | `DurationExpr::Fixed(0)` allows zero-cost actions | Low | Phase 2 (validate in `ActionDefRegistry::register()`) |
| **Minor** | P2 | `RelationValue::LoyalTo.strength` as raw `Permille` risks magic-number threshold logic | Low | Phase 2 (ensure changes are event-driven) |

---

## Appendix: Design Strengths

Phase 1 establishes several architectural patterns that strongly enforce the foundational principles:

### WorldTxn Atomicity
All world mutations go through `WorldTxn` (`world_txn.rs`), which journals every change as a typed `StateDelta` and commits atomically with a `PendingEvent`. This prevents partial mutations, ensures every change is causally linked, and provides complete audit trails. This is the backbone of Principle 1 (causality) and Principle 12 (state-mediated interaction).

### Provenance Tracking
Every `ItemLot` carries a `Vec<ProvenanceEntry>` recording every quantity change with tick, event, operation type, and related lot. This makes item history fully traceable — a direct enforcement of Principle 1 and Principle 4.

### BTreeMap-Only Determinism
`HashMap` and `HashSet` are banned from authoritative state (enforced by policy tests in `policy.rs`). Only `BTreeMap` and `BTreeSet` are used, guaranteeing deterministic iteration order. Combined with `ChaCha8Rng` (seeded via `Seed([u8; 32])`), this ensures bit-exact deterministic replay.

### Generational Entity IDs
`EntityId` uses slot+generation design (`ids.rs`), preventing use-after-free bugs where a stale ID accidentally refers to a recycled entity. The generational allocator provides type-safe lifecycle management.

### Canonical State Hashing
`blake3`-based canonical hashing (`canonical.rs`) over `bincode`-serialized world state enables per-tick hash verification during replay. Any non-determinism or state corruption is immediately detectable.

### Conservation Invariants
`verify_conservation()` (`conservation.rs`) enforces that commodity quantities are never created or destroyed except through explicit actions. This is a concrete dampener (Principle 8) on economic exploits and a structural guarantee of Principle 1.

### No-Float Policy
The entire codebase uses integer arithmetic (`u32`, `u64`) and custom newtypes (`Permille` for ratios). No floating-point values exist in authoritative state, eliminating a common source of non-determinism.

### Strict Type Boundaries
Custom newtypes (`Quantity`, `LoadUnits`, `Permille`, `Tick`, `EntityId`, `EventId`, `Seed`, `TravelEdgeId`) prevent accidental type confusion. A `Quantity` cannot be accidentally used where a `LoadUnits` is expected, catching logic errors at compile time.
