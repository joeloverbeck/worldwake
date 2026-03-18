# AGENTS.md

This file provides guidance to coding agents working in this repository.

## Agent Workflow

- Read this file before making changes.
- Start with the GitNexus context resource and follow the matching GitNexus skill when the task calls for codebase exploration, impact analysis, debugging, or refactoring.
- Keep edits minimal and targeted. Do not refactor unrelated code while completing the requested task.
- If instructions, specs, or repo documentation appear incomplete or contradictory, propose an update to the relevant rules or docs files. Do not make those documentation changes unless the user asks for them.

## Working Rules

- Follow the 1-3-1 rule when blocked by an unclear or risky decision: present 1 concrete problem, 3 viable options, and 1 recommendation. Do not implement one of those options until the user confirms.
- Prefer DRY solutions. If implementation starts to repeat existing logic, stop and search for an existing abstraction or a place to refactor.
- Use TDD for bug fixes. Add or adjust tests to capture the bug, then fix the behavior. Never adapt tests to preserve a bug.
- Respect worktree boundaries. If the user asks you to work inside `.claude/worktrees/<name>/`, use that worktree root for all reads, writes, searches, moves, and archival actions.
- Maintain ticket fidelity. Do not silently skip explicit deliverables from a spec or ticket. If a deliverable seems wrong or blocked, surface it with the 1-3-1 rule instead of deciding unilaterally.

## Ticket Expectations

- Follow `tickets/README.md` when creating or revising tickets. Start from `tickets/_TEMPLATE.md`.
- Reassess every ticket against current code, focused tests, golden coverage, and harness setup before implementation. If current code and ticket assumptions diverge, update the ticket first.
- Do not leave a ticket marked `Engine Changes: None` or “tests only” when the requested invariant actually exposes an architectural contradiction in production code. Correct the scope first.
- When a ticket claims a testing gap, distinguish missing focused/unit coverage from missing golden/E2E coverage.
- Name the exact layer and symbol for non-trivial claims. Do not collapse AI/planning behavior, authoritative action validation, and system resolution into one vague statement.
- If a test relies on timing, state whether the contract is action-lifecycle ordering, event-log ordering, or authoritative world-state ordering.
- Prefer decision-trace assertions for AI candidate absence, suppression, or planner behavior rather than relying only on missing events or missing committed actions.

## Foundational Principles

Read `docs/FOUNDATIONS.md` before making any design decision. It defines 13 non-negotiable principles in 4 categories (Causal Foundations, World Dynamics, Agent Architecture, System Architecture) that govern every system in this project — including maximal emergence, no magic numbers, concrete state over abstract scores, locality of information, physical dampeners for feedback loops, agent symmetry, agent diversity, system decoupling, and no backward compatibility. All code, specs, and architectural choices must be evaluated against these principles.

## Project

Worldwake is a causality-first emergent micro-world simulation in Rust. It is currently a CLI/text prototype where agents plan from beliefs rather than world state, and all consequences propagate through an append-only event log.

## Build And Test

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
cargo test -p worldwake-core
cargo test -p worldwake-core test_name
```

Run the narrowest command that verifies your change first, then expand to broader workspace checks when warranted.

## Architecture

The workspace currently has five crates under `crates/`:

```text
worldwake-core    -> IDs, types, ECS store, topology, items, relations
worldwake-sim     -> Event log, action framework, scheduler, replay, save/load
worldwake-systems -> Needs, production, trade, combat, travel, transport actions
worldwake-ai      -> Pressure-based GOAP planner, goal ranking, decision architecture
worldwake-cli     -> Human control interface
```

Additional architecture notes:

- `worldwake-core` should stay dependency-light.
- The ECS is custom and uses deterministic `BTreeMap`-backed typed component storage.
- The world is modeled as a place graph with travel times, not continuous space.

## Critical Invariants

These design rules are intentional and should be preserved unless the user explicitly changes them:

- No `Player` type. Use `ControlSource = Human | Ai | None`.
- Belief-only planning. Agents do not read authoritative world state directly.
- Information locality. No system queries global world state on behalf of an agent; information propagates through perception, reports, witnesses, and travel over the place graph.
- Systems interact through state, not through each other. System modules in `worldwake-systems` depend on `worldwake-core` and `worldwake-sim`, never on each other.
- Append-only event log. The causal record is not mutated in place.
- Determinism. Use seeded randomness such as `ChaCha8Rng`; use `BTreeMap`/`BTreeSet` rather than `HashMap`/`HashSet` in authoritative state; avoid floats and wall-clock time.
- Conservation. Items are not created or destroyed except through explicit actions.
- Unique location. Every entity exists in exactly one place.
- No backward compatibility layers. When a design changes, update or remove the old path instead of adding shims, redirects, or deprecated wrappers.

## Authoritative-To-AI Impact Rule

Any change to authoritative validation or control checks such as action preconditions, `validate_*` functions, or `can_exercise_control` must be verified across the full AI decision pipeline before it is considered complete:

1. `get_affordances` still exposes the expected candidates.
2. `generate_candidates` still emits the expected goal kinds.
3. `search_plan` still finds valid plans, including terminal ordering and barrier handling.
4. Action start in `tick_step` still handles newly rejected plans gracefully.
5. `handle_plan_failure` still records blockers and replans correctly after rejection.
6. Relevant golden coverage passes, and changes that touch AI behavior should normally include `cargo test -p worldwake-ai`.

Golden tests that expect agents to observe produced or newly materialized output need an appropriate `PerceptionProfile`. Without it, tests can fail by never observing the new state.

## Spec Drafting Rules

All new spec drafts must:

1. Use `Permille` for any [0,1] or [0,1000] range values. Do not use `f32` or `f64`.
2. Include FND-01 Section H analyses: information-path analysis, positive-feedback analysis, concrete dampeners, and stored-vs-derived state listing.
3. Use profile-driven per-agent parameters instead of hardcoded numeric constants.
4. Include SystemFn Integration and Component Registration sections.
5. Document cross-system interactions through Principle 12: state-mediated, never direct system-to-system calls.

These rules exist to prevent specs from drifting into magic numbers, float-based scoring, and missing foundation analysis that would need correction before implementation.

## Debugging AI Decisions with Decision Traces

When debugging AI-related test failures or investigating agent behavior, use the **decision trace system** before resorting to ad-hoc `eprintln` instrumentation. The trace system records structured per-agent per-tick decision data covering the full pipeline: candidate generation, ranking, plan search, selection, and execution outcome.

**Quick start in golden tests:**

```rust
// Enable before stepping:
h.driver.enable_tracing();

// Run ticks, then query:
let sink = h.driver.trace_sink().unwrap();
let trace = sink.trace_at(agent, Tick(5)).unwrap();

// Dump human-readable summary to stderr:
sink.dump_agent(agent, &h.defs);
```

**Key queries:**
- `sink.traces_for(agent)` — all traces for one agent
- `sink.trace_at(agent, tick)` — single tick lookup
- `trace.outcome.summary()` — one-line human-readable string
- `DecisionOutcome::Planning(p)` — inspect `p.candidates`, `p.planning.attempts`, `p.selection`

**When to reach for traces:**
- "Why did/didn't agent X do Y?" → check `candidates.generated` and `planning.attempts`
- "Why did the agent switch goals?" → check `InterruptTrace` on `ActiveAction` outcomes
- "Why did plan search fail?" → check `PlanSearchOutcome` variants (`BudgetExhausted`, `FrontierExhausted`, `Unsupported`)

Tracing is opt-in and zero-cost when disabled. Do not leave `enable_tracing()` in committed test code unless the test explicitly asserts on trace data.

## Debugging Action Execution with Action Traces

For action lifecycle questions ("Did the action run?", "When did it complete?", "Why was it aborted?"), use the action execution trace system in `worldwake-sim`.

**Quick start in golden tests:**

```rust
// Enable before stepping:
h.enable_action_tracing();

// Run ticks, then query:
let sink = h.action_trace_sink().unwrap();
let agent_events = sink.events_for(agent);
let tick_events = sink.events_at(Tick(5));
let agent_tick_events = sink.events_for_at(agent, Tick(5));

// Dump human-readable summary to stderr:
sink.dump_agent(agent);
```

Key types: `ActionTraceSink`, `ActionTraceEvent`, `ActionTraceKind` (Started, Committed, Aborted, StartFailed).

**When to use which trace:**
- "Why did the agent choose this action?" -> decision trace
- "Did the chosen action actually start or commit?" -> action trace
- "How long did the action take?" -> action trace
- "Why was the action aborted?" -> action trace

**Important**: Some actions (e.g., loot, eat) complete within a single tick. They are invisible to inter-tick `agent_active_action_name()` observation. Use action traces or state-delta checks for these. Multi-tick actions such as harvest, travel, and craft remain visible between ticks.

When in doubt, enable action tracing and inspect `events_for_at(agent, tick)` to see exactly what happened during that tick.

## Delivery Planning

- The implementation plan spans 22 epics across 4 phases.
- Phase 1 (`E01`-`E08`) and Phase 2 (`E09`-`E13`) are completed and archived under `archive/specs/`.
- Active planning material lives in `specs/` and currently includes the `S04`-`S12` specs plus active `E16b`-`E22` epic specs.
- Phase ordering and gates live in `specs/IMPLEMENTATION-ORDER.md`.
- Do not treat phase gates as advisory. New phase work should not begin until the prior gate conditions pass.

## Dependencies

Keep external dependencies minimal. The core expected crates are:

- `serde`
- `bincode`
- `blake3`
- `rand_chacha`

Avoid introducing a third-party ECS crate.

## Key References

- Brainstorming spec: `brainstorming/emergent-prototype-spec.md`
- Design doc: `docs/plans/2026-03-09-worldwake-epic-breakdown-design.md`
- Active specs: `specs/`
- Archived completed specs: `archive/specs/`
- Archival workflow: `docs/archival-workflow.md`

Follow `docs/archival-workflow.md` as the canonical archival policy. Do not duplicate or redefine archival procedure elsewhere; update that document if the policy changes.

## Commit And PR Expectations

Commit subjects in this repo are short and imperative. Existing patterns include:

- `docs: add Spec 12 - CLI`
- `Implemented CORTYPSCHVAL-008`
- `Implemented ENGINEAGNO-007.`

When modifying specs, tickets, or roadmap material:

- Verify cross-spec references.
- Keep numbering and terminology consistent.
- Check that the roadmap and the affected specs do not conflict.

PRs should include:

- A clear summary of what changed and why.
- A linked issue or spec section when applicable.
- Confirmation that references, numbering, and terminology remain consistent.
- A concrete test plan with verification steps.

## GitNexus

<!-- gitnexus:start -->
# GitNexus MCP

This project is indexed by GitNexus as **worldwake** (7504 symbols, 29910 relationships, 300 execution flows).

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
