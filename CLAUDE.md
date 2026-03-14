# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Coding Guidelines

- Follow the 1-3-1 rule: When stuck, provide 1 clearly defined problem, give 3 potential options for how to overcome it, and 1 recommendation. Do not proceed implementing any of the options until I confirm.
- DRY: Don't repeat yourself. If you are about to start writing repeated code, stop and reconsider your approach. Grep the codebase and refactor often.
- Continual Learning: When you encounter conflicting system instructions, new requirements, architectural changes, or missing or inaccurate codebase documentation, always propose updating the relevant rules files. Do not update anything until the user confirms. Ask clarifying questions if needed.
- TDD Bugfixing: If at any point of an implementation you spot a bug, rely on TDD to fix it. Important: never adapt tests to bugs.
- Worktree Discipline: When instructed to work inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file operations — reads, edits, globs, greps, moves, archival — must use the worktree root as the base path. The default working directory is the main repo root; tool calls without an explicit worktree path will silently operate on main.
- Ticket Fidelity: Never silently skip or rationalize away explicit ticket deliverables. If a ticket says to touch a file or produce an artifact, do it. If you believe a deliverable is wrong, unnecessary, or blocked, apply the 1-3-1 rule — present the problem and options to the user rather than deciding on your own. Marking a task "completed" with an excuse instead of doing the work, or instead of flagging the blocker, is never acceptable.

## Foundational Principles

Read `docs/FOUNDATIONS.md` before making any design decision. It defines 13 non-negotiable principles in 4 categories (Causal Foundations, World Dynamics, Agent Architecture, System Architecture) that govern every system in this project — including maximal emergence, no magic numbers, agent symmetry, concrete state over abstract scores, locality of information, feedback dampening, agent diversity, and system decoupling. All code, specs, and architectural choices must be evaluated against these principles.

## Project

Worldwake is a causality-first emergent micro-world simulation in Rust. CLI/text prototype where agents plan from beliefs (never world state), and all consequences propagate through an append-only event log.

## Build Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
cargo test -p worldwake-core           # single crate
cargo test -p worldwake-core test_name # single test
```

## Architecture

5-crate workspace in `crates/`:

```
worldwake-core    → IDs, types, ECS store, topology, items, relations (no deps)
worldwake-sim     → Event log, action framework, scheduler, replay (deps: core)
worldwake-systems → Needs/metabolism, production/crafting, trade, combat, travel/transport actions (deps: core, sim)
worldwake-ai      → Pressure-based GOAP planner, goal ranking, decision runtime (deps: core, sim, systems)
worldwake-cli     → Human control interface (deps: all)
```

Custom ECS (no external crate) with deterministic `BTreeMap`-based typed component storage. The world is a place graph with travel times, not continuous space.

### worldwake-core modules

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

### worldwake-sim modules

The simulation crate contains the action framework, scheduler, tick loop, and replay/persistence:

| Module | Purpose |
|--------|---------|
| `action_def` | `ActionDef` — declarative action type definitions |
| `action_def_registry` | Registry of all available action definitions |
| `action_domain` | `ActionDomain` — enum categorizing actions by domain (Generic, Needs, Production, Trade, Travel, Transport, Combat, Care, Loot) |
| `action_duration` | `ActionDuration` — resolved runtime duration for active actions (Finite or Indefinite) with tick advancement |
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

### worldwake-systems modules

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

### worldwake-ai modules

The AI crate contains the decision architecture: pressure-based goal ranking, GOAP-style plan search, and per-tick agent control:

| Module | Purpose |
|--------|---------|
| `agent_tick` | `AgentTickDriver` — manages per-agent decision runtime and semantics caching for tick-driven AI execution |
| `budget` | `PlanningBudget` — tunable planning constraints (candidates, depth, expansions, beam width, margins, blocking periods) |
| `candidate_generation` | Goal candidate enumeration — derives goal candidates from agent beliefs (needs, pressure, enterprise signals) |
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

## Critical Invariants

These are non-negotiable design rules enforced by tests:

- **No `Player` type** — only `ControlSource = Human | Ai | None`
- **Belief-only planning** — agents never read world state directly (Principle 10)
- **Information locality** — no system queries global state on behalf of an agent; information propagates at finite speed through the place graph (Principle 7)
- **System decoupling** — system modules in `worldwake-systems` depend only on `worldwake-core` and `worldwake-sim`, never on each other (Principle 12)
- **Append-only event log** — causal source of truth, never mutated
- **Determinism** — `ChaCha8Rng` seeded, `BTreeMap`/`BTreeSet` only in authoritative state (no `HashMap`/`HashSet`), no floats, no wall-clock time
- **Conservation** — items cannot be created/destroyed except through explicit actions; enforced by `verify_conservation`
- **Unique location** — every entity exists in exactly one place

## Spec Drafting Rules

All new spec drafts MUST:
1. Use `Permille` for any [0,1] or [0,1000] range values — never `f32` or `f64`
2. Include FND-01 Section H analyses (information-path, feedback loops, dampeners, stored vs derived)
3. Use profile-driven parameters (per-agent structs) instead of hardcoded numeric constants
4. Include SystemFn Integration and Component Registration sections
5. Document cross-system interactions via Principle 12 (state-mediated, never direct calls)

These rules prevent the recurring pattern of specs written with magic numbers, floats, and missing foundation analyses that then require correction before implementation.

## Future System Spec Requirements (FND-01 Section H)

Every future system spec (E09+) MUST include the following analysis sections:

1. **Information-path analysis**: How does each piece of information reach the agents who act on it? Trace the path from source event through perception, witnesses, reports, and belief updates. If information arrives at an agent without a traceable multi-hop path, the design violates Principle 7 (Locality).
2. **Positive-feedback analysis**: Identify every amplifying loop (A increases B, B increases A) in the system. If no loops exist, state so explicitly.
3. **Concrete dampeners**: For each positive-feedback loop, specify the physical world mechanism that limits amplification. Numerical clamps (e.g., `min(value, cap)`) are NOT acceptable dampeners — the dampener must be a physical world process (Principle 8).
4. **Stored state vs. derived read-model list**: Explicitly enumerate what is authoritative stored state (components, relations) and what is a transient derived computation. No derived value may be stored as authoritative state (Principle 3).

See `specs/FND-01-phase1-foundations-alignment.md` Section H and `docs/FOUNDATIONS.md` Principles 3, 7, 8 for rationale.

## Implementation Plan

22 epics across 4 phases with strict gates. Specs live in `specs/`. Dependency graph and phase gates are in `specs/IMPLEMENTATION-ORDER.md`. Completed specs and tickets are archived under `archive/specs/` and `archive/tickets/`.

**Completed epics (Phase 1 — World Legality)**: E01 (project scaffold), E02 (world topology), E03 (entity store), E04 (items & containers), E05 (relations & ownership), E06 (event log & causality), E07 (action framework), E08 (time, scheduler, replay & save/load). Phase 1 established the core and sim crates with the ECS, topology graph, item/container model, conservation invariants, relation system, append-only event log with causal linking, transactional world mutations, canonical state hashing, the action framework with preconditions, the tick-driven scheduler, deterministic replay, and save/load persistence.

**Completed epics (Phase 2 — Emergent Economy)**: E09 (needs & metabolism), E10 (production & transport), E11 (trade & economy), E12 (combat & health), E13 (decision architecture). Phase 2 established the systems and ai crates with homeostatic needs and deprivation wounds, resource regeneration and recipe-based crafting, merchant trade with valuation, combat with wound tracking, and a pressure-based GOAP decision architecture with goal ranking, plan search, failure handling, and per-tick autonomous agent control.

**Phase gates are blocking** — do not start a new phase until all gate tests for the previous phase pass.

## External Dependencies

Minimal: `serde`, `bincode`, `rand_chacha`, `blake3` (canonical state hashing). No external ECS crate.

## Key References

- Brainstorming spec: `brainstorming/emergent-prototype-spec.md`
- Design doc: `docs/plans/2026-03-09-worldwake-epic-breakdown-design.md`
- Epic specs: `specs/E13-*.md` through `specs/E22-*.md` (`archive/specs/` contains archived or completed specs, including E01–E12)

## Commit Conventions

Commit subjects should be short and imperative. Common patterns in this repo:
- `docs: add Spec 12 — CLI`
- `Implemented CORTYPSCHVAL-008`
- `Implemented ENGINEAGNO-007.`

When modifying specs or tickets, verify cross-spec references and ensure roadmap and individual specs do not conflict.

## Pull Request Guidelines

PRs should include:
- A clear summary of changed files and why
- Linked issue/spec section when applicable
- Confirmation that references, numbering, and terminology are consistent across affected specs
- Test plan with verification steps

## Skill Invocation (MANDATORY)

When a slash command (e.g., `/superpowers:execute-plan`) expands to an instruction like "Invoke the superpowers:executing-plans skill", you MUST call the `Skill` tool with the referenced skill name BEFORE taking any other action. The `<command-name>` tag means the *command wrapper* was loaded, NOT the skill itself. The skill content is only available after you call the Skill tool.

Do NOT skip the Skill tool invocation. Do NOT interpret the command body as the skill content. Do NOT start implementation before the skill is loaded and its methodology followed.

## MCP Server Usage

When using Serena MCP for semantic code operations (symbol navigation, project memory, session persistence), it must be activated first:

```
mcp__plugin_serena_serena__activate_project with project: "ludoforge-llm"
```

Serena provides:
- Symbol-level code navigation and refactoring
- Project memory for cross-session context
- Semantic search across the codebase
- LSP-powered code understanding

## Sub-Agent Web Research Permissions

Sub-agents spawned via the `Task` tool **cannot prompt for interactive permission**. Any tool they need must be pre-approved in `.claude/settings.local.json` under `permissions.allow`. Without this, web search tools are silently auto-denied and sub-agents fall back to training knowledge only.

**Required allow-list entries for web research**:
- `WebSearch` and `WebFetch` — built-in fallback search tools
- `mcp__tavily__tavily_search`, `mcp__tavily__tavily_extract`, `mcp__tavily__tavily_crawl`, `mcp__tavily__tavily_map`, `mcp__tavily__tavily_research` — Tavily MCP tools

**Tavily API key**: Configured in `~/.claude.json` under `mcpServers.tavily.env.TAVILY_API_KEY`. Development keys (`tvly-dev-*`) have usage limits — upgrade at [app.tavily.com](https://app.tavily.com) if you hit HTTP 432 errors ("usage limit exceeded").

## Archiving Tickets and Specs

Follow the canonical archival policy in `docs/archival-workflow.md`.

Do not duplicate or drift this procedure in other files; update `docs/archival-workflow.md` as the source of truth.

<!-- gitnexus:start -->
# GitNexus MCP

This project is indexed by GitNexus as **worldwake** (5911 symbols, 22300 relationships, 300 execution flows).

## Always Start Here

1. **Read `gitnexus://repo/{name}/context`** — codebase overview + check index freshness
2. **Match your task to a skill below** and **read that skill file**
3. **Follow the skill's workflow and checklist**

> If step 1 warns the index is stale, run `npx gitnexus analyze` in the terminal first.

## Skills

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
