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

## Spec Drafting Rules

All new spec drafts must:

1. Use `Permille` for any [0,1] or [0,1000] range values. Do not use `f32` or `f64`.
2. Include FND-01 Section H analyses: information-path analysis, positive-feedback analysis, concrete dampeners, and stored-vs-derived state listing.
3. Use profile-driven per-agent parameters instead of hardcoded numeric constants.
4. Include SystemFn Integration and Component Registration sections.
5. Document cross-system interactions through Principle 12: state-mediated, never direct system-to-system calls.

These rules exist to prevent specs from drifting into magic numbers, float-based scoring, and missing foundation analysis that would need correction before implementation.

## Delivery Planning

- The implementation plan spans 22 epics across 4 phases.
- Phase 1 (`E01`-`E08`) and Phase 2 (`E09`-`E13`) are completed and archived under `archive/specs/`.
- Active epic specs live in `specs/E14-*.md` through `specs/E22-*.md`.
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
- Active epic specs: `specs/E14-*.md` through `specs/E22-*.md`
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

This project is indexed by GitNexus as **worldwake** (6144 symbols, 23138 relationships, 300 execution flows).

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
