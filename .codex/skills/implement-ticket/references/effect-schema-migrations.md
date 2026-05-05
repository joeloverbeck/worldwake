# Effect-Schema Category Migrations

Use this reference when a ticket adds, replaces, or migrates `ActionDef.effect_schema` behavior, especially for staged S-series schema work where the authoritative runtime path and planner hypothetical path do not move in the same ticket.

## Classify the Effect Step

Before adding an `EffectStep`, decide whether it is genuinely generic or category-owned.

Use a generic step only when the operation is fully represented by shared inputs and shared sink behavior. Commodity movement, event tags, expectation fulfillment, and contention-grant consumption can be generic when the existing sink can apply them without hidden domain state.

Prefer a typed category-specific step when the old commit branch carries any of these:

- domain-specific payloads
- lifecycle state
- local helper invariants
- aftermath components or traces
- cleanup of contention, queues, jobs, source state, or evidence
- split/materialization results that must be returned to the surrounding commit path

The owning action module's authoritative sink should interpret those typed steps. Generic or hypothetical sinks should reject unsupported category-owned steps clearly, normally with `Discrepancy::ImproperPlanningState`, until the planner/parity ticket owns them.

Do not flatten branch-specific behavior into a generic need-delta, transfer, consume, produce, or similar abstraction just to satisfy a draft sketch.

## Reassessment Questions

- Does the schema live in registry-time template data, or does it need runtime actor, target, payload, or current-action references?
- Are the existing `commit_*` or `apply_*` helpers the lawful mutation boundary?
- Does the old handler produce trace data, materialization output, cleanup side effects, or evidence beyond the visible event?
- Does the generic authoritative sink already have a safe all-or-nothing discipline for this effect shape?
- Is the planner still supposed to use old hypothetical arms until a later ticket?

Record the answers in the active ticket before coding when they change the drafted implementation boundary.

## Closeout

If the landed schema uses category-owned steps instead of the draft's generic step sketch, close out the ticket with that deviation explicitly:

- name the typed steps that landed
- name the domain aftermath that made generic steps insufficient
- state whether planner hypothetical mode remains old-path or has parity
- state whether persisted save shape changed
- update active sibling tickets/spec text whose forward-looking handoff now falsely assumes generic step coverage
