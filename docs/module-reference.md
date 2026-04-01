# Module Reference

Per-crate module listings for the worldwake workspace.

## worldwake-core modules

The foundation crate contains all authoritative types and the ECS world boundary:

| Module | Purpose |
|--------|---------|
| `ids` | `EntityId` (slot+generation), `Tick`, `EventId`, `Seed`, `TravelEdgeId` |
| `entity` | `EntityKind` enum (Agent, Place, ItemLot, UniqueItem, Container, …), `EntityMeta` |
| `allocator` | Generational slot allocator with archive/purge lifecycle |
| `component_tables` | Macro-generated typed storage for all component types |
| `component_schema` | Declarative component registration (which kinds accept which components) |
| `blocked_intent` | `BlockedIntentMemory`, `BlockedIntent` — agent memory of temporarily blocked goals with expiration |
| `combat` | `CombatProfile` — per-agent combat parameters (wound capacity, skills, attack/guard, bleed rates) |
| `components` | Domain components: `AgentData`, `Name` |
| `control` | `ControlSource` enum (Human, Ai, None) |
| `drives` | `ThresholdBand` — urgency thresholds (low, medium, high, critical) shared by physiology and AI |
| `topology` | `Place`, `PlaceTag`, `TravelEdge`, `Route`, `Topology`, Dijkstra pathfinding, `build_prototype_world` |
| `trade` | `MerchandiseProfile`, `DemandMemory`, `TradeDispositionProfile` — merchant sales intent and demand observation |
| `items` | `CommodityKind`, `ItemLot`, `UniqueItem`, `UniqueItemKind`, `Container`, `LotOperation`, `ProvenanceEntry`, `TradeCategory` |
| `load` | Weight/load accounting: per-unit loads, container capacity checks |
| `needs` | `HomeostaticNeeds` — per-agent metabolic state (hunger, thirst, fatigue, bladder, dirtiness) |
| `conservation` | `total_live_lot_quantity`, `total_authoritative_commodity_quantity`, `verify_live_lot_conservation`, `verify_authoritative_conservation` — explicit lot-only vs authoritative material accounting |
| `numerics` | Newtype wrappers: `Quantity`, `LoadUnits`, `Permille` |
| `production` | `WorkstationTag` — workstation types (Forge, Loom, Mill, ChoppingBlock, WashBasin, OrchardRow, FieldPlot) |
| `traits` | `Component` and `RelationRecord` trait definitions |
| `utility_profile` | `UtilityProfile` — per-agent utility weights for AI decision making across needs and enterprise |
| `relations` | `RelationTables`, placement/ownership/reservation/social APIs, `ArchiveDependency` |
| `event_log` | Append-only `EventLog` — authoritative causal record |
| `event_record` | `PendingEvent` → committed `EventRecord` flow |
| `event_tag` | `EventTag` — event classification |
| `goal` | `GoalKind`, `CommodityPurpose` — goal identity types (consume, acquire, sleep, heal, produce, etc.) |
| `cause` | `CauseRef` — links events to prior causes in the log |
| `witness` | `WitnessData`, `VisibilitySpec` — who observed an event |
| `delta` | `ComponentDelta`, `RelationDelta`, `ReservationDelta` — immutable change records |
| `world_txn` | `WorldTxn` — mutation journal with staged atomic commit |
| `canonical` | `canonical_bytes()`, `hash_world()`, `hash_event_log()`, `StateHash` — deterministic hashing |
| `verification` | `verify_live_lot_conservation()`, `verify_authoritative_conservation()`, `verify_completeness()` — invariant enforcement |
| `visibility` | Event visibility and witness tracking |
| `world` | `World` struct — authoritative boundary over allocator, component tables, topology, and relations |
| `wounds` | `WoundId`, `BodyPart`, `DeprivationKind` — wound tracking schema for deprivation and combat consequences |
| `error` | `WorldError` enum |
| `test_utils` | Shared test helpers |

## worldwake-sim modules

The simulation crate contains the action framework, scheduler, tick loop, and replay/persistence:

| Module | Purpose |
|--------|---------|
| `action_def` | `ActionDef` — declarative action type definitions |
| `action_def_registry` | Registry of all available action definitions |
| `action_domain` | `ActionDomain` — enum categorizing actions by domain (Generic, Needs, Production, Trade, Travel, Transport, Combat, Care, Loot) |
| `action_duration` | `ActionDuration` — resolved runtime duration for active actions (always finite) with tick advancement |
| `action_instance` | `ActionInstance`, `ActionInstanceId` — specific running action |
| `action_state` | `ActionState` — action lifecycle state machine |
| `action_status` | `ActionStatus` — outcome tracking |
| `action_handler` | Action execution handler trait |
| `action_handler_registry` | Registry mapping action defs to handlers |
| `action_payload` | `ActionPayload` — enum holding domain-specific data for actions (Harvest, Craft, Trade, Combat, Loot) |
| `action_semantics` | `Constraint`, `Precondition`, `DurationExpr` — action preconditions and costs |
| `action_execution` | `start_action()` — initiate actions with precondition/cost checks |
| `action_ids` | Action-specific ID types |
| `action_termination` | Action completion and interruption handling |
| `action_validation` | Authoritative action legality checks against `World` / `WorldTxn` |
| `scheduler` | `Scheduler` — manages tick loop, active actions, system execution order |
| `tick_step` | `step_tick()` — execute one full tick (drain inputs → progress actions → run systems) |
| `tick_action` | `tick_action()` — progress individual action state per tick |
| `start_gate` | Action start precondition validation |
| `interrupt_abort` | Action interruption and abort handling |
| `replan_needed` | Signals for agent replanning |
| `simulation_state` | `SimulationState` — root state: world + event log + scheduler + replay + rng |
| `controller_state` | `ControllerState` — tracks human vs AI control per agent |
| `input_event` | `InputEvent`, `InputKind` — player/AI input (RequestAction, CancelAction, SwitchControl) |
| `input_queue` | `InputQueue` — deterministic input ordering by `(tick, sequence_no)` |
| `deterministic_rng` | `DeterministicRng` — ChaCha8 wrapper for seeded randomness |
| `belief_view` | `BeliefView` trait — agent belief interface |
| `omniscient_belief_view` | `OmniscientBeliefView` — omniscient stand-in until E14 |
| `autonomous_controller` | `AutonomousController` trait — interface for AI/autonomous systems to claim and control agents |
| `affordance` | `Affordance` — available actions for an agent |
| `affordance_query` | `get_affordances()` — query available actions |
| `recipe_def` | `RecipeDefinition` — data-driven production recipe with inputs, outputs, workstation requirements, and body cost |
| `recipe_registry` | `RecipeRegistry` — deterministic registry of all production recipes, indexed by workstation tag |
| `replay_state` | `ReplayState`, `ReplayCheckpoint` — record initial state, seed, inputs, per-tick hashes |
| `replay_execution` | `replay_and_verify()` — deterministic replay validation |
| `save_load` | `save()` / `load()` — serializable world snapshots (bincode format) |
| `system_manifest` | `SystemManifest` — declares which systems run each tick |
| `system_dispatch` | `SystemDispatch` — routes tick execution to registered systems |
| `tick_input_producer` | `TickInputContext` — context struct passed to input producers for autonomous AI tick integration |
| `trade_valuation` | `TradeAcceptance`, `TradeRejectionReason` — trade decision enums with valuation snapshot utilities |
| `action_trace` | `ActionTraceSink`, `ActionTraceEvent`, `ActionTraceKind` — opt-in action lifecycle recording for debugging |

## worldwake-systems modules

The systems crate contains domain simulation systems (needs, production, trade, combat) and their action handlers:

| Module | Purpose |
|--------|---------|
| `combat` | `run_combat_system()` + handlers for combat and loot actions — resolves weapon attacks, wounds, and looting |
| `inventory` | `controlled_entity_ids()`, `controlled_entity_load()`, `consume_one_unit()` — load/capacity tracking and possession hierarchy helpers |
| `needs` | `needs_system()` — processes homeostatic needs (hunger, thirst, sleep) and applies deprivation wounds each tick |
| `needs_actions` | `eat`, `drink`, `sleep` action definitions + handlers with `ConsumableEffect` — agents satisfy needs through action framework |
| `production` | `resource_regeneration_system()` — regenerates commodities at resource sources (`ResourceSource` component) each tick |
| `production_actions` | Harvest + craft action definitions + handlers — agents gather raw resources and craft items via `RecipeRegistry` |
| `trade` | `trade_system_tick()` — ages trade demand memories, applies forgotten commodity preferences over time |
| `trade_actions` | Trade action definition + handler — negotiates and executes two-agent trades with valuation |
| `transport_actions` | Pick-up + put-down action handlers — agents move items between containers/direct possession |
| `travel_actions` | Travel action definition + handler — agents move between places via `TravelEdge` edges |

## worldwake-ai modules

The AI crate contains the decision architecture: pressure-based goal ranking, GOAP-style plan search, and per-tick agent control:

| Module | Purpose |
|--------|---------|
| `agent_tick` | `AgentTickDriver` — manages per-agent decision runtime and semantics caching for tick-driven AI execution |
| `budget` | `PlanningBudget` — tunable planning constraints (candidates, depth, expansions, beam width, margins, blocking periods) |
| `candidate_generation` | Goal candidate enumeration — derives goal candidates from agent beliefs (needs, pressure, enterprise signals) |
| `decision_trace` | `DecisionTraceSink`, `AgentDecisionTrace`, `DecisionOutcome` — opt-in structured trace of per-agent per-tick decisions; `dump_agent()` for interactive debugging, `summary()` for one-line outcome strings |
| `decision_runtime` | `AgentDecisionRuntime` — per-agent persistent state (current goal/plan/step, dirty flags, last observations) |
| `enterprise` | Merchant enterprise logic — restock gap, opportunity signals for trading/commerce planning |
| `failure_handling` | `PlanFailureContext`, `handle_plan_failure()` — analyzes plan breakdowns, updates blocked memory with barriers |
| `goal_model` | `GoalKindTag`, `GoalKindPlannerExt` — goal-to-planner-op mapping (ConsumeCommodity, AcquireCommodity, etc.) |
| `goal_switching` | `GoalSwitchKind`, `compare_goal_switch()` — priority-based goal interruption logic with margin thresholds |
| `interrupts` | `InterruptDecision`, `InterruptTrigger`, `evaluate_interrupt()` — determines when running action should be interrupted for replan |
| `plan_revalidation` | `revalidate_next_step()` — checks if planned step remains executable against current affordances |
| `plan_selection` | `select_best_plan()` — chooses best plan from candidates by priority/motive with goal-switching logic |
| `planner_ops` | `PlannerOpKind`, `PlannerOpSemantics` — declarative action-type semantics (barriers, mid-plan viability, goal relevance) |
| `planning_snapshot` | `SnapshotEntity` — immutable read-only belief state snapshot for planning (positions, inventory, wounds, profiles) |
| `planning_state` | `PlanningState` — mutable planning simulation state (overrides, shadows, removed entities for hypothetical execution) |
| `pressure` | Pressure derivation — pain/danger permille calculations from wounds and active threats |
| `ranking` | `RankedGoal`, `rank_candidates()` — scores goals by priority class and motive value |
| `search` | `SearchNode`, `search_plan()` — GOAP-style best-first search for multi-step action plans toward goals |
