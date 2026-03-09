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
worldwake-systems → Needs, production, trade, combat, perception, politics (deps: core, sim)
worldwake-ai      → GOAP planner, utility scoring, decision architecture (deps: core, sim, systems)
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
| `components` | Domain components: `AgentData`, `Name` |
| `control` | `ControlSource` enum (Human, Ai, None) |
| `topology` | `Place`, `PlaceTag`, `TravelEdge`, `Route`, `Topology`, Dijkstra pathfinding, `build_prototype_world` |
| `items` | `CommodityKind`, `ItemLot`, `UniqueItem`, `UniqueItemKind`, `Container`, `LotOperation`, `ProvenanceEntry`, `TradeCategory` |
| `load` | Weight/load accounting: per-unit loads, container capacity checks |
| `conservation` | `total_commodity_quantity`, `verify_conservation` — enforces the conservation invariant |
| `numerics` | Newtype wrappers: `Quantity`, `LoadUnits`, `Permille` |
| `traits` | `Component` and `RelationRecord` trait definitions |
| `world` | `World` struct — authoritative boundary over allocator, component tables, and topology |
| `error` | `WorldError` enum |
| `test_utils` | Shared test helpers |

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

## Implementation Plan

22 epics across 4 phases with strict gates. Specs live in `specs/`. Dependency graph and phase gates are in `specs/IMPLEMENTATION-ORDER.md`. Completed specs and tickets are archived under `archive/specs/` and `archive/tickets/`.

**Completed epics**: E01 (project scaffold), E02 (world topology), E03 (entity store), E04 (items & containers). These established the foundation crate with the ECS, topology graph, item/container model, and conservation invariants.

**Phase gates are blocking** — do not start a new phase until all gate tests for the previous phase pass.

## External Dependencies

Minimal: `serde`, `bincode`, `rand_chacha`. No external ECS crate.

## Key References

- Brainstorming spec: `brainstorming/emergent-prototype-spec.md`
- Design doc: `docs/plans/2026-03-09-worldwake-epic-breakdown-design.md`
- Epic specs: `specs/E05-*.md` through `specs/E22-*.md` (E01–E04 archived in `archive/specs/`)

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

This project is indexed by GitNexus as **worldwake** (1940 symbols, 6516 relationships, 161 execution flows).

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
