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

- strict tick separation
- action lifecycle ordering
- event-log ordering
- authoritative state transition ordering

Do not treat incidental tick-boundary details as the contract unless the system is intentionally specified that way.
If the scenario spans multiple layers, state which earlier layer drives the divergence and which later layer is only a downstream consequence.
If two actors can lawfully complete relevant actions in the same tick, do not rewrite that contract as "later tick" unless strict tick separation is the intended engine rule. In those cases, action-trace ordering should be asserted via the explicit `(tick, sequence_in_tick)` key on `ActionTraceEvent`.

Good:
- no `declare_support` commit while hunger remains `High-or-above`
- `eat` commits before `declare_support`

Bad:
- hunger relief must appear on a strictly earlier tick number than all later political commits

The first pair encodes the architectural rule. The second overfits to scheduler timing.

Do not use delayed authoritative installation as a proxy for earlier political-action ordering when succession or another lawful system can add delay between the action commit and the final office-holder mutation. In that case, prove the earlier ordering with action traces and prove the later durable consequence with authoritative world state.

Do not claim a "same-state, weight-only divergence" unless both compared branches are driven by comparable ranking substrates in the current architecture. If one branch depends on a pressure-scaled or priority-derived substrate and the other uses a flat motive or later system resolution, name that asymmetry explicitly in the ticket and in the test rationale.

## Trace Guidance

### Use action traces when:

- proving one action completed before another
- proving an action started, committed, aborted, or failed to start
- proving same-tick actions that are invisible to inter-tick active-action inspection
- proving same-tick cross-agent causal order without overfitting to tick numbers
- proving a committed `tell` targeted a specific `listener`/`subject` pair via `ActionTraceDetail::Tell`

### Use decision traces when:

- debugging why a goal did or did not appear
- proving suppression, ranking, or planner-search behavior
- distinguishing "candidate missing" from "candidate present but filtered/suppressed"
- proving negative AI invariants such as "this goal never appeared" or "this candidate was never generated"
- inspecting the final selected path via `planning.selection.selected_plan` and `planning.selection.selected_plan_source` when you need the chosen plan shape, terminal semantics, or whether the trace reflects a fresh search result, retained current plan, or snapshot-only continuation
- proving social omission reasons such as `SpeakerHasAlreadyToldCurrentBelief` before any `tell` commit exists

When the contract is about candidate generation, ranking, suppression, or plan selection, do not infer the result indirectly from missing event-log entries or missing committed actions if a decision trace can prove it directly.
For conversation-memory crowd-out scenarios, prove the stale subject was omitted with the concrete social omission reason before claiming an untold subject survived truncation. The absence of a duplicate `tell` commit by itself is too weak because that could also arise from ranking loss, invalidation, or unrelated execution failure.
For social scenarios, action traces and decision traces answer different questions: action traces prove that a committed `tell` happened for a specific `listener`/`subject`, while decision traces prove why another `ShareBelief` candidate was omitted, suppressed, or never generated.

### Use both when:

- the AI reasoning contract and the execution contract are both under test

For same-tick cross-agent chains, `events_at(tick)` and `events_for_at(actor, tick)` tell you which events happened within the tick, but not the contract by themselves. Use the recorded `sequence_in_tick` field when the assertion depends on relative order among those events.

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

For social goldens, document whether the speaker needs an explicit belief about the intended listener for `ShareBelief` to materialize. Blind-perception or heavily isolated setups often require explicit listener-belief seeding even when the agents are co-located.
For social goldens, also document subject choice explicitly. Agent subjects can create additional lawful `ShareBelief` branches around the subject's own changing state or location. If the contract is about resend suppression or a specific downstream office fact, prefer a non-agent subject unless the extra agent-subject branches are part of the invariant under test.

## Ticket Expectations For Golden Work

Golden-related tickets should:

1. name the exact scenario gap
2. state whether the gap is missing focused coverage, missing golden coverage, or both
3. identify the exact assertion surface to use
4. avoid stale command examples
5. distinguish candidate generation, ranking/suppression, execution, and authoritative outcome
6. name the exact layer when similar helpers exist in both AI/planning code and authoritative/system code
7. document scenario-isolation choices when lawful competing affordances exist and the golden is intended to prove one branch
8. if the ticket depends on ordering, state whether the compared branches are symmetric in the current architecture or whether the divergence depends on priority class, motive score, suppression, delayed system resolution, or a mixed-layer combination

## Verification Commands

Typical verification sequence:

1. targeted test name
2. owning golden test binary
3. crate suite
4. repo verification baseline via `scripts/verify.sh`

If a stricter lint or broader suite is required, state that explicitly in the ticket.
