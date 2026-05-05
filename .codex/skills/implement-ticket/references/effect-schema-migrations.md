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

For staged category-owned steps, prefer adding default-rejecting methods to the shared sink trait and overriding them only in the local authoritative sink that owns execution. This keeps unsupported planner or generic paths explicit without forcing unrelated sinks to grow placeholder behavior.

When several category-owned sinks would repeat the same unsupported-step boilerplate, search first for an existing rejecting adapter/helper or a narrow shared helper pattern. Add a bounded helper only if it reduces real duplication without widening the public API or coupling unrelated domains; otherwise record why local repetition was accepted to preserve category seams.

Do not flatten branch-specific behavior into a generic need-delta, transfer, consume, produce, or similar abstraction just to satisfy a draft sketch.

## Category-Owned Migration Skeleton

When reassessment chooses a typed category-owned step, use this implementation shape unless the live module already has a stronger local pattern:

1. Add one explicit `EffectStep` variant per owned action or tightly coupled action branch.
2. Add a default-rejecting `EffectSink` method for each new category step. The default should return `Discrepancy::ImproperPlanningState` so unsupported generic and hypothetical sinks fail clearly until the planner/parity ticket owns them.
3. Populate `ActionDef.effect_schema` at registration time with the category step and any truly generic preconditions or steps that the existing shared sink can lawfully interpret.
4. Preserve the live mutation boundary by renaming or extracting the old `commit_*` body into a narrow `apply_*_effect` helper, or by calling an existing `apply_*` helper directly. Do not duplicate aftermath writes in a second path.
5. Add a module-local authoritative sink that owns only the category step(s), validates actor/target/payload/current-action context against the `ActionInstance`, and translates `ActionError` into a stored error plus `Discrepancy::PartialExecutionDrift` for the evaluator.
6. Replace the public `commit_*` handler body with `apply_effects_with_context(..., EffectMode::Authoritative)` when payload, targets, or current action id are needed. Use the simpler wrapper only when the action truly needs no context beyond actor/targets.
7. Add or update focused registration tests that assert the action definition carries the expected category step. Keep existing behavior tests as the parity proof for the authoritative mutation helper.
8. During closeout, record the category-owned step names, the domain aftermath that made generic steps insufficient, whether hypothetical mode remains default-rejected, and whether `SAVE_FORMAT_VERSION` changed or intentionally stayed unchanged.

This skeleton is a starting point, not permission to widen public API. Prefer a local sink or local helper over a shared abstraction when the domain aftermath is not genuinely reusable.

## Reassessment Questions

- Does the schema live in registry-time template data, or does it need runtime actor, target, payload, or current-action references?
- Are the existing `commit_*` or `apply_*` helpers the lawful mutation boundary?
- Does the old handler produce trace data, materialization output, cleanup side effects, or evidence beyond the visible event?
- Does the generic authoritative sink already have a safe all-or-nothing discipline for this effect shape?
- Is the planner still supposed to use old hypothetical arms until a later ticket?

Record the answers in the active ticket before coding when they change the drafted implementation boundary.

## Planner Parity / Old-Path Deletion

Use this section when the ticket owns switching planner hypothetical evaluation to the shared schema evaluator, deleting old planner transition paths, or proving hypothetical parity for category-owned steps.

1. Enumerate every registered `EffectStep` that can appear in planner-visible `ActionDef.effect_schema` values. Include category-owned steps added by earlier staged migration tickets, not only generic transfer/consume/produce steps.
2. Find every default-rejecting or unimplemented `EffectSink` method that the hypothetical sink could now reach. Decide per step whether hypothetical mode can lawfully interpret it from `PlanningState`, payload, action context, and planning targets, or whether the active ticket/spec must narrow the planner-visible schema boundary.
3. Implement hypothetical interpretations through the planner-owned sink or a seam-local helper. Do not call authoritative systems, read authoritative world state for an agent, or flatten category aftermath into generic effects to make parity compile.
4. For materialization, split-lot, movement, travel, queue, office, social, combat, or other category steps with nontrivial aftermath, record which projected state is planner-relevant and which authoritative trace/event residue remains outside the hypothetical overlay.
5. Prove registered schemas no longer hit unsupported hypothetical steps on the intended planner path. Prefer focused conformance/coverage tests plus the affected AI package or golden lane over ad hoc probes.
6. Delete the old transition symbols in the same closeout when the ticket owns old-path deletion, then run a targeted grep such as `rg -n 'apply_hypothetical_transition|PlannerTransitionKind|apply_planner_step' crates`.
7. If planner parity required changing planning targets, payload context, expected materializations, or action-start behavior, load `references/reassessment-planner-ai.md` and prove the earliest affected boundary before broad verification.

## Closeout

If the landed schema uses category-owned steps instead of the draft's generic step sketch, close out the ticket with that deviation explicitly:

- name the typed steps that landed
- name the domain aftermath that made generic steps insufficient
- state whether planner hypothetical mode remains old-path or has parity
- state whether persisted save shape changed
- update active sibling tickets/spec text whose forward-looking handoff now falsely assumes generic step coverage
- run a targeted stale-term scan over the active ticket, cited spec, and active sibling tickets for rejected generic sketch terms and old domain nouns, such as `AssertBelief`, `CreateRecord`, `EffectFact`, `record artifact`, `notice creation`, or the specific superseded step names from reassessment
