# Golden E2E Testing Conventions

Use this document when adding or revising tests under `crates/worldwake-ai/tests/golden_*.rs`.

It exists to keep golden assertions aligned with the architecture instead of drifting into brittle scheduler-coupled checks.
For the live mechanical inventory and docs-sync validation workflow, use `python3 scripts/golden_inventory.py --write --check-docs` and the generated artifact at `docs/generated/golden-e2e-inventory.md`.

## Assertion Hierarchy

Prefer the strongest, most semantic assertion surface available:

1. **Request-resolution traces**
   - Use for pre-start request binding or rejection facts.
   - Examples: "`RequestResolutionOutcome::RejectedBeforeStart` proved the request never reached authoritative start", "request bound through `ReproducedAffordance` before a later `StartFailed`".
2. **Authoritative world state**
   - Use for durable outcomes.
   - Examples: office holder, location, commodity totals, wound state, containment, relations.
3. **Action traces**
   - Use for lifecycle ordering and execution facts.
   - Examples: "`eat` committed before `declare_support`", "action started but never committed", "action aborted with reason".
4. **Decision traces**
   - Use for AI reasoning questions.
   - Examples: "candidate existed but was suppressed", "plan search exhausted frontier", "agent selected X over Y".
5. **Event log**
   - Use when event provenance, tags, or public record visibility is itself the contract.
   - Do not default to event-log ordering when action traces or authoritative state express the behavior more directly.

When multiple semantic surfaces could prove the invariant, prefer the earliest causal boundary that proves the contract. Only widen the golden to later execution or durable-state consequences when that later boundary is itself part of the promise under test.

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

### Use request-resolution traces when:

- proving whether a request was rejected before authoritative start
- proving which binding path (`ReproducedAffordance` vs `BestEffortFallback`) carried a request into start
- distinguishing "request never reached start" from "request reached start and then lawfully failed"
- debugging stale or retained concrete requests whose truth boundary is affordance reproduction rather than action execution

When request-resolution tracing exists for the scenario, do not claim pre-start rejection from missing action-trace events alone. Use `RequestResolutionOutcome::RejectedBeforeStart` directly for that boundary.
When a scenario involves stale or retained requests, state explicitly whether the contract is request-resolution rejection before start, authoritative `StartFailed` at start, or post-start abort after lawful start.

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
- proving travel-led route selection when the contract is about the initial planned path rather than only eventual arrival
- proving social omission reasons such as `SpeakerHasAlreadyToldCurrentBelief` before any `tell` commit exists

When the contract is about candidate generation, ranking, suppression, or plan selection, do not infer the result indirectly from missing event-log entries or missing committed actions if a decision trace can prove it directly.
`archive/tickets/completed/S16S09GOLVAL-002.md` is the concrete example of this narrowing: the durable downstream outcome mattered less than the earlier changed-conditions selection boundary, so the golden was corrected to prove "first post-resolution selected goal is non-combat" instead of broad eat/heal follow-through.
`archive/tickets/completed/S16S09GOLVAL-004.md` is the travel-planning example of the same rule: the durable arrival/harvest outcome matters, but the ticket's actual promise starts earlier at the selected path boundary, so the golden proves both `selection.selected_plan.next_step` and the later Orchard Farm outcome instead of inferring route quality from arrival alone.
For conversation-memory crowd-out scenarios, prove the stale subject was omitted with the concrete social omission reason before claiming an untold subject survived truncation. The absence of a duplicate `tell` commit by itself is too weak because that could also arise from ranking loss, invalidation, or unrelated execution failure.
For social scenarios, action traces and decision traces answer different questions: action traces prove that a committed `tell` happened for a specific `listener`/`subject`, while decision traces prove why another `ShareBelief` candidate was omitted, suppressed, or never generated.

### Request-Resolution Boundary Examples

- `strict_request_records_resolution_rejection_without_start_attempt` in `crates/worldwake-sim/src/tick_step.rs` is the focused proof that a request can be rejected before start.
- `best_effort_stale_request_records_start_failure_when_affordance_no_longer_matches` in `crates/worldwake-sim/src/tick_step.rs` is the focused proof that a request can bind first and then still hit authoritative `StartFailed`.
- `golden_care_pre_start_wound_disappearance_records_blocker` and `golden_local_trade_start_failure_recovers_via_production_fallback` in `crates/worldwake-ai/tests/` are golden examples of the later start-failure and reconciliation boundary, not proof of pre-start rejection.

### Recoverable Authoritative Start Failure

When the contract is "a lawful start rejection is recoverable," prove it in two steps:

- use an action trace to prove the action reached authoritative start and recorded `StartFailed`
- use the next AI tick's decision trace to prove `planning.action_start_failures` was consumed and the stale branch was cleared, blocked, or replaced

Do not treat "no later commit happened" as sufficient evidence of reconciliation. That symptom is too weak because it can also come from request-resolution rejection before start, candidate omission, ranking loss, plan-search failure, or unrelated execution failure.
Current golden examples of this proof shape include the care, production, trade, and political start-failure suites in `crates/worldwake-ai/tests/`.

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

When the intended branch depends on authoritative arithmetic or cumulative mechanics, the owning ticket/spec must also state the concrete setup math that makes the branch reachable: the relevant delta, cadence, threshold, tolerance window, capacity, or other live formula inputs. Do not write these scenarios as narrative expectations alone.
For repeated threshold firing, wound accumulation, resource depletion, recovery gating, or similar cumulative mechanics, document the survival/failure envelope explicitly. If the intended branch is impossible under current formulas, correct the scenario numbers in the ticket/spec instead of weakening production behavior or papering over the mismatch with weaker assertions.
`archive/tickets/completed/S17WOULIFGOLSUI-001.md` is the concrete deprivation example: the clean fix was to adjust the scenario thresholds and above-critical hunger values so two lawful deprivation fires could occur under live arithmetic, not to weaken `worsen_or_create_deprivation_wound`.

For social goldens, document whether the speaker needs an explicit belief about the intended listener for `ShareBelief` to materialize. Blind-perception or heavily isolated setups often require explicit listener-belief seeding even when the agents are co-located.
For social goldens, also document subject choice explicitly. Agent subjects can create additional lawful `ShareBelief` branches around the subject's own changing state or location. If the contract is about resend suppression or a specific downstream office fact, prefer a non-agent subject unless the extra agent-subject branches are part of the invariant under test.
For spatial-planning goldens, document whether the contract includes the default planning budget itself. If it does, state that explicitly and remove nearer lawful alternatives from setup only when the invariant under test is route reachability from a branchy hub rather than competition among local food branches.

## Ticket Precision

Golden-related tickets must follow `docs/precision-rules.md` for all technical claims.
Additionally, golden tickets should name the exact scenario gap and state whether it is missing focused coverage, missing golden coverage, or both.

## Verification Commands

Typical verification sequence:

1. targeted test name
2. owning golden test binary
3. crate suite
4. docs inventory refresh/validation via `python3 scripts/golden_inventory.py --write --check-docs`
5. repo verification baseline via `scripts/verify.sh`

If a stricter lint or broader suite is required, state that explicitly in the ticket.
