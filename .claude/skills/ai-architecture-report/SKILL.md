---
name: ai-architecture-report
description: "Produces a self-contained structured reference of the AI architecture by analyzing golden E2E tests and tracing into source. Output is optimized for external LLM evaluation against FOUNDATIONS.md."
user-invocable: true
---

# AI Architecture Report

Generates a self-contained structured reference document describing the current AI architecture. The primary input is golden E2E tests in `crates/worldwake-ai/tests/`. The skill traces from tests into source code to build a complete picture of the architecture — planning pipeline, action framework, system interactions, and cross-cutting infrastructure.

The output is optimized for an external LLM (ChatGPT Pro) that has no repository access. The external LLM receives this report alongside `docs/FOUNDATIONS.md` (fed separately) and evaluates it for architecture flaws, missing capabilities, and improvement opportunities.

## Invocation

```
/ai-architecture-report
```

No arguments. Fully autonomous. Writes output to `reports/ai-architecture-report.md`.

## Worktree Awareness

If working inside a worktree (e.g., `.claude/worktrees/<name>/`), ALL file paths — reads, writes, globs, greps — must use the worktree root as the base path. The default working directory is the main repo root; paths without an explicit worktree prefix will silently operate on main.

## Process

Follow these 4 phases in order. Do not skip any phase.

### Phase 1 — Discovery

Read all golden E2E test files in `crates/worldwake-ai/tests/`. For each test, extract:

1. **Goal kinds triggered** — which `GoalKind` variants appear in assertions, setup, or are expected to be generated
2. **Action domains exercised** — which `ActionDomain` entries are used
3. **System functions called** — needs/metabolism, production/crafting, trade, combat, travel/transport, perception, social/institutional
4. **Components and relations involved** — which ECS components are set up, queried, or mutated
5. **Decision chains** — how agents move from needs to goals to candidates to plans to actions to effects
6. **Setup topology** — places, agents, items, relations configured in the test

Build a comprehensive inventory of all architectural elements exercised by the golden test suite.

When the test count is large (>10 files), use up to 3 Explore agents in parallel to read and extract from different test files simultaneously. Provide each agent with a specific subset of test files and the extraction checklist above.

### Phase 2 — Source Tracing

For each architectural element discovered in Phase 1, trace into source code to document the actual implementation. Key areas to trace:

1. **Candidate generation** (`candidate_generation.rs`) — how goal candidates are generated, suppression filters, emission gates
2. **Plan search** (`search.rs`) — planning pipeline, barrier logic, terminal ordering, beam width
3. **Action handlers** — preconditions, effects, payload structure, validation logic
4. **System functions** — tick behavior, state mutations, system registration
5. **Cross-cutting infrastructure**:
   - Contention queues — how actions contend for shared resources
   - Force-control — lifecycle, acquisition, release
   - Event log — append-only structure, event types
   - Perception propagation — how information flows through the place graph
   - Belief management — how agents maintain and update beliefs
   - Affordance queries — how available actions are discovered
   - Plan revalidation — how stale plans are detected and re-checked
6. **Component schemas** — field types, derive bounds, ECS registration in `component_schema.rs`

For each element, record:
- The concrete type names and function signatures
- The data flow (inputs, transformations, outputs)
- How it connects to other elements
- Any constraints, invariants, or edge cases visible in the code

Use Explore agents in parallel when tracing multiple independent areas.

### Phase 3 — Report Assembly

Write the structured reference document. Organize by architectural concern, not by file or crate. Each section must contain:

- **What it is**: 1-2 sentence summary
- **How it works**: Mechanics, data flow, key types with their fields, function signatures
- **Which golden tests exercise it**: Test names as concrete references
- **Current limitations or edge cases**: Anything observed during analysis

Include enough concrete detail (type definitions, enum variants, function signatures, field types) that an LLM with no repo access can reason about the architecture. When referencing a type, include its key fields. When referencing a function, include its signature and a brief description of its logic.

### Phase 4 — Output

Write the assembled report to `reports/ai-architecture-report.md`. Do not commit. Do not prompt for cleanup.

## Report Structure

The report MUST include all of the following sections:

```markdown
# AI Architecture Reference — YYYY-MM-DD

## 1. Architecture Overview
Crate structure (core, sim, systems, ai, cli), ECS design (BTreeMap-based
typed component storage), determinism guarantees (ChaCha8Rng, no floats,
no HashMap). Key foundational types: EntityId, Permille, Quantity, Tick, etc.

## 2. Agent Decision Pipeline
Full pipeline: goal ranking → candidate generation → plan search → action
execution. Key files, key types, suppression filters, barrier logic,
terminal ordering, beam width. How pressure-based GOAP works.

## 3. Action Framework
Action registration and domain taxonomy. Precondition checking, effect
application, payload structure. Handler lifecycle: validate → execute →
emit events. How actions interact with the event log.

## 4. System Interactions
Per-system summaries for each implemented system:
- Needs/Metabolism
- Production/Crafting
- Trade
- Combat
- Travel/Transport
- Perception
- Social/Institutional
- (any others discovered)

For each: what it does, key types, how it interacts with other systems
through state (Principle 26 — never direct calls).

## 5. Cross-Cutting Infrastructure
Contention queues, force-control lifecycle, event log structure,
perception propagation, belief management, affordance queries,
plan revalidation. For each: mechanics, key types, how it integrates
with the decision pipeline.

## 6. Golden Test Coverage Map
Table with columns: Test Name | Goal Kinds | Action Domains | Systems Exercised
One row per golden test. Shows what the test suite covers and implicitly
what it does not.

## 7. Architectural Observations
Patterns, asymmetries, or oddities noticed during analysis. Things that
might be worth the external LLM investigating further. Not prescriptive —
flags only, no recommendations. The external LLM's job is to evaluate
these against FOUNDATIONS.md and propose changes.
```

## Guardrails

- **Self-contained**: The report must make complete sense without repository access. Never write "see file X" without including the relevant content inline. Include type definitions, enum variants, function signatures, and data flow descriptions.
- **Architecture only**: Focus on infrastructure, pipelines, and system interaction patterns. Do not analyze game content (specific creature types, bounty mechanics, looting rules, etc.). The boundary is: if it's a reusable architectural mechanism, include it. If it's a specific game feature built on top of that mechanism, exclude it.
- **No prescriptions**: Section 7 flags patterns and observations but does not recommend fixes or improvements. That is the external LLM's job. The report is descriptive, not prescriptive.
- **Concrete over vague**: Prefer exact type names, field lists, and function signatures over abstract descriptions. "GoalKind::SatisfyNeed { need_kind: NeedKind }" is better than "goals for satisfying needs."
- **No commit**: Write the file and stop. The user handles the file lifecycle.
- **Parallel agents**: Use Explore agents in parallel for Phase 1 and Phase 2 when the volume of tests or source files warrants it. This keeps the skill efficient even as the codebase grows.
- **Date stamp**: Always include the current date in the report title so the user knows when it was generated.
