# Golden E2E Testing Conventions

Use this document when adding or revising tests under `crates/worldwake-ai/tests/golden_*.rs`.

It exists to keep golden assertions aligned with the architecture instead of drifting into brittle scheduler-coupled checks.

## Assertion Hierarchy

Prefer the strongest, most semantic assertion surface available:

1. **Authoritative world state**
   - Use for durable outcomes.
   - Examples: office holder, location, commodity totals, wound state, containment, relations.
2. **Action traces**
   - Use for lifecycle ordering and execution facts.
   - Examples: "`eat` committed before `declare_support`", "action started but never committed", "action aborted with reason".
3. **Decision traces**
   - Use for AI reasoning questions.
   - Examples: "candidate existed but was suppressed", "plan search exhausted frontier", "agent selected X over Y".
4. **Event log**
   - Use when event provenance, tags, or public record visibility is itself the contract.
   - Do not default to event-log ordering when action traces or authoritative state express the behavior more directly.

## Ordering Rules

When a test needs ordering, state explicitly which ordering is the contract:

- action lifecycle ordering
- event-log ordering
- authoritative state transition ordering

Do not treat incidental tick-boundary details as the contract unless the system is intentionally specified that way.

Good:
- no `declare_support` commit while hunger remains `High-or-above`
- `eat` commits before `declare_support`

Bad:
- hunger relief must appear on a strictly earlier tick number than all later political commits

The first pair encodes the architectural rule. The second overfits to scheduler timing.

## Trace Guidance

### Use action traces when:

- proving one action completed before another
- proving an action started, committed, aborted, or failed to start
- proving same-tick actions that are invisible to inter-tick active-action inspection

### Use decision traces when:

- debugging why a goal did or did not appear
- proving suppression, ranking, or planner-search behavior
- distinguishing "candidate missing" from "candidate present but filtered/suppressed"
- proving negative AI invariants such as "this goal never appeared" or "this candidate was never generated"
- inspecting the final selected path via `planning.selection.selected_plan` and `planning.selection.selected_plan_source` when you need the chosen plan shape, terminal semantics, or whether the trace reflects a fresh search result, retained current plan, or snapshot-only continuation

When the contract is about candidate generation, ranking, suppression, or plan selection, do not infer the result indirectly from missing event-log entries or missing committed actions if a decision trace can prove it directly.

### Use both when:

- the AI reasoning contract and the execution contract are both under test

### Use a cross-layer timeline when:

- you are debugging or asserting a mixed-layer chain and need one derived per-tick view across decision, action, politics, and explicitly selected event-log records
- you want a readable merged timeline without weakening the underlying assertions

Keep authoritative event-log selection explicit. Do not rely on helper heuristics to infer which authoritative records belong in the timeline.

## Determinism Pattern

New golden scenarios should usually add a deterministic replay companion test unless one of these is true:

- the scenario is intentionally non-deterministic by design
- the scenario is too small and redundant with an existing deterministic helper
- the owning ticket explicitly justifies why replay coverage is unnecessary

## Scenario Isolation

When a golden scenario is intended to prove one specific causal branch, document the scenario-isolation choice explicitly if the current architecture lawfully permits competing affordances that could also satisfy local needs or planner branching.

State all of the following in the owning ticket/spec:

1. the intended branch or invariant under test
2. the lawful competing affordances the current architecture would otherwise allow
3. which unrelated lawful branches were intentionally removed from setup, and why they are outside the contract under test

This guidance exists to keep goldens honest, not to stage-manage outcomes. Remove unrelated lawful affordances only when they would obscure the invariant you are trying to prove. If the competing branch is part of the architecture contract, keep it and assert the branching behavior directly instead.

## Ticket Expectations For Golden Work

Golden-related tickets should:

1. name the exact scenario gap
2. state whether the gap is missing focused coverage, missing golden coverage, or both
3. identify the exact assertion surface to use
4. avoid stale command examples
5. distinguish candidate generation, ranking/suppression, execution, and authoritative outcome
6. name the exact layer when similar helpers exist in both AI/planning code and authoritative/system code
7. document scenario-isolation choices when lawful competing affordances exist and the golden is intended to prove one branch

## Verification Commands

Typical verification sequence:

1. targeted test name
2. owning golden test binary
3. crate suite
4. repo verification baseline via `scripts/verify.sh`

If a stricter lint or broader suite is required, state that explicitly in the ticket.
