# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Coding Guidelines

- Follow the 1-3-1 rule: When stuck, provide 1 clearly defined problem, give 3 potential options for how to overcome it, and 1 recommendation. Do not proceed implementing any of the options until I confirm.
- DRY: Don't repeat yourself. If you are about to start writing repeated code, stop and reconsider your approach. Grep the codebase and refactor often.
- Continual Learning: When you encounter conflicting system instructions, new requirements, architectural changes, or missing or inaccurate codebase documentation, always propose updating the relevant rules files. Do not update anything until the user confirms. Ask clarifying questions if needed.
- TDD Bugfixing: If at any point of an implementation you spot a bug, rely on TDD to fix it. Important: never adapt tests to bugs.
- Worktree Discipline: When instructed to work inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file operations — reads, edits, globs, greps, moves, archival — must use the worktree root as the base path. The default working directory is the main repo root; tool calls without an explicit worktree path will silently operate on main.
- Ticket Fidelity: Never silently skip or rationalize away explicit ticket deliverables. If a ticket says to touch a file or produce an artifact, do it. If you believe a deliverable is wrong, unnecessary, or blocked, apply the 1-3-1 rule — present the problem and options to the user rather than deciding on your own. Marking a task "completed" with an excuse instead of doing the work, or instead of flagging the blocker, is never acceptable.
- Git Safety: Before running `git reset --hard`, `git checkout -- .`, or any command that discards local changes, always run `git status` first. If uncommitted changes exist, commit or stash them before proceeding. Prefer `git pull` over `git fetch && git reset --hard` when syncing with remote.

## Ticket Expectations

- Reassess every ticket against current code, focused tests, golden coverage, and harness setup before implementation. If current code and ticket assumptions diverge, update the ticket first.
- Follow `docs/precision-rules.md` for all technical claims in tickets, specs, and golden test rationale.

## Foundational Principles

Read `docs/FOUNDATIONS.md` before making any design decision. It defines 28 non-negotiable principles in 5 categories (Causal Standard, World Dynamics, Knowledge/Belief/Evidence, Agents/Institutions/Social Order, System Architecture) that govern every system in this project — including maximal emergence, no magic numbers, agent symmetry, concrete state over abstract scores, locality of information, feedback dampening, agent diversity, system decoupling, and no backward compatibility. The preamble also mandates that every change be an architecturally comprehensive solution — no hacks, patches, or workarounds. All code, specs, and architectural choices must be evaluated against these principles.

## Project

Worldwake is a causality-first emergent micro-world simulation in Rust. CLI/text prototype where agents plan from beliefs (never world state), and all consequences propagate through an append-only event log.

## Build Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p worldwake-core           # single crate
cargo test -p worldwake-core test_name # single test
```

The clippy command must match CI exactly: `--all-targets` includes test/bench/example targets, and `-D warnings` promotes all warnings to errors. Running `cargo clippy --workspace` alone will miss test-target lints that CI enforces.

Run the narrowest command that verifies your change first, then expand to broader workspace checks when warranted.

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

See `docs/module-reference.md` for per-module type listings.

## Critical Invariants

These are non-negotiable design rules enforced by tests:

- **No `Player` type** — only `ControlSource = Human | Ai | None`
- **Belief-only planning** — agents plan from beliefs (FND-14), with one narrow exception: same-tick direct observation of a co-located entity's physical properties (kind, item-lot commodity/quantity, workstation tag, resource source, container contents) may read world state, since a correct perception pipeline would deliver those facts on the same tick (FND-14A). Social/relational facts (ownership, effective rights, institutional claims, jurisdiction) always require an explicit belief entry even when the subject is co-located. See `crates/worldwake-sim/src/per_agent_belief_view.rs` for the canonical split implementation.
- **Information locality** — no system queries global state on behalf of an agent; information propagates through perception, reports, witnesses, and travel over the place graph (FND-7, FND-15)
- **System decoupling** — system modules in `worldwake-systems` depend only on `worldwake-core` and `worldwake-sim`, never on each other (FND-26)
- **Append-only event log** — causal source of truth, never mutated
- **Determinism** — `ChaCha8Rng` seeded, `BTreeMap`/`BTreeSet` only in authoritative state (no `HashMap`/`HashSet`), no floats, no wall-clock time
- **Conservation** — items cannot be created/destroyed except through explicit actions; enforced by `verify_conservation`
- **Unique location** — every entity exists in exactly one place
- **No backward compatibility layers** — when a design changes, update or remove the old path instead of adding shims, redirects, or deprecated wrappers
- **Scenario profile completeness** — every agent profile component registered on `EntityKind::Agent` must be scenario-definable via `AgentDef` + `spawn_agent()`. Universal profiles are always applied with defaults. See `docs/spec-drafting-rules.md` section 5 for the checklist.

## Authoritative-to-AI Impact Rule

Any change to authoritative validation (action preconditions, `validate_*` functions, `can_exercise_control`) must trace the full agent decision cycle before claiming completion. See `docs/debugging-traces.md` for trace tooling. The checklist:

1. `get_affordances` still produces correct candidates (affordance_query.rs)
2. `generate_candidates` emits the right goal kinds (candidate_generation.rs)
3. `search_plan` finds valid plans — check terminal ordering and barrier logic (search.rs)
4. `BestEffort` action start handles the new validation gracefully (tick_step.rs)
5. `handle_plan_failure` replans correctly after the new check rejects (agent_tick.rs)
6. **Payload revalidation**: If the action uses planner-synthesized payloads (not affordance-derived), does the handler have `with_payload_override_validator` registered? `plan_revalidation.rs` calls `requested_affordance_matches` which delegates to the handler's validator for untargeted actions with synthesized payloads. Without it, the step silently fails revalidation.
7. ALL golden tests pass (`cargo test -p worldwake-ai`)

Golden production tests require `PerceptionProfile` on agents that need to observe post-production output. Tests without perception profiles will silently fail to observe newly created entities.

## Debugging

For debugging AI decisions or action execution, see `docs/debugging-traces.md` (decision traces, action traces, tick alignment, observation strategy, system tick ordering, force-control lifecycle).

## Spec Drafting Rules

All new spec drafts MUST:
1. Use `Permille` for any [0,1] or [0,1000] range values — never `f32` or `f64`
2. Include FND-01 Section H analyses — see `docs/spec-drafting-rules.md` for full requirements
3. Use profile-driven parameters (per-agent structs) instead of hardcoded numeric constants
4. Include SystemFn Integration and Component Registration sections
5. Document cross-system interactions via Principle 12 (state-mediated, never direct calls)

## Implementation Plan

Specs live in `specs/`. Dependency graph and phase gates are in `specs/IMPLEMENTATION-ORDER.md`. Completed specs and tickets are archived under `archive/specs/` and `archive/tickets/`.

## External Dependencies

Minimal: `serde`, `bincode`, `rand_chacha`, `blake3` (canonical state hashing). No external ECS crate.

## Key References

- Active specs: `specs/`
- Archived completed specs: `archive/specs/`

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

## Archiving Tickets and Specs

Follow the canonical archival policy in `docs/archival-workflow.md`.

Do not duplicate or drift this procedure in other files; update `docs/archival-workflow.md` as the source of truth.
