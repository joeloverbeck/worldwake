# Reassessment Checks — Planner and AI

Planner-root, snapshot-completeness, planner-traceability, belief-projection, and AI pipeline validation for Step 2.

## Planner-specific reassessment

For planner-root, snapshot-completeness, or planner-traceability tickets, cite the relevant live contract from [docs/planner-contracts.md](../../../../docs/planner-contracts.md) during reassessment instead of reconstructing planner behavior from archived tickets, stale scenario prose, or local implementation fragments alone.

When the ticket is an audit-then-fix (e.g., "verify path X, fix if needed"), treat the audit as reassessment. Record findings in the reassessment section. If a gap is confirmed, auto-correct `Engine Changes`, `What to Change`, and `Files to Touch` before coding. If no gap exists, close with a reassessment-only Outcome documenting the audit trail.

If focused traces, regression tests, or lower-layer proofs falsify the current implementation hypothesis after coding has already started, stop and reassess immediately. Restate the live boundary, update the ticket sections that define owned scope, remove stale partial edits from the disproved approach, and only then continue.
If the falsified hypothesis was the ticket's core implementability claim rather than just one candidate fix, switch from implementation to rejection-or-successor triage immediately: revert the disproved code path, restate the live contradiction in the active ticket, decide whether the current ticket becomes a factual rejection record or narrows to a remaining valid slice, and create a successor ticket when real work remains.

## AI pipeline and affordance checks

**Affordance prerequisites:**
- When affordance generation depends on self-authoritative profile reads, verify those prerequisites in both production code and test harnesses.
- When proving real affordance enumeration against co-located agents/items/places, verify whether the affordance query also depends on the actor already believing those targets are present. Seed the corresponding belief/perception prerequisite in tests.
- When a ticket gates one agent's affordance on another agent's private belief carriers (e.g., `ExpectationStore`, `LastSeenMemory`), verify the read surface. In `PerAgentBeliefView`-style boundaries these may be self-only; cross-agent checks may need to stay actor-local at affordance time.
- When the ticket asks an existing query to distinguish new enum variants, verify the current read surface exposes enough information.
- When the ticket depends on UtilityProfile or disposition gating, verify the belief/read trait exposes that carrier.

**Goal and candidate pipeline:**
- When the ticket claims a goal family should become behaviorally selectable, check the full AI admission path: candidate generation, goal-policy suppression, ranking, selection.
- When a ticket audits threshold alignment between candidate emission and goal satisfaction, also inspect the matching hypothetical planner transition. Record whether one step or repeated steps clear the relevant band.
- When a planner, ranking, interrupt, or goal-switch ticket names `switch_margin`, `frame_switch_margin`, motive-score deltas, or `Permille` examples, inspect the live comparison helper before writing tests or closeout prose. Do not infer absolute score deltas from `Permille(100)`-style prose; bind examples to the helper's actual relative/absolute arithmetic.
- When an existing operator becomes newly goal-satisfying for an additional goal family, compare operator legality across every live goal family that consumes that operator.
- When making a payload-override action live through the AI pipeline, compare planner-step revalidation against runtime request resolution.

**Typed queries and staged variants:**
- When adding a typed query alongside an existing boolean helper, verify boolean equivalence.
- When the ticket gates behavior on a typed right from a specific provenance source, verify whether right existence alone is lawful or the producing carrier is part of the contract.
- When a staged ticket introduces a shared enum before all variants are producible, distinguish "type surface lands now" from "variant becomes live now." Test reserved variants as absent.

**Belief and projection surfaces:**
- When the ticket keeps an action family unified while widening to new entity kinds, inspect `TargetSpec`, affordance enumeration, authoritative validation, planner semantics, and payload validators.
- When extending a projected belief or derived state, check for parallel snapshot builders, event carriers, or projection helpers.
- For cache/compression/performance tickets over derived belief or summary state, verify whether the derived surface depends only on stored membership or also on external inputs such as `current_tick`, activation, ordering, or other live context. Do not approve "changed set only" invalidation unless the ticket's contract models every input that can change the derived winner.
- When a new world artifact becomes perceivable and the spec says discovery affects behavior, verify at least one lawful downstream consumer exists.
- When the ticket says information should be "internalized," search for an existing belief lane or consumer before inventing a new belief substrate.
- When the ticket changes historical event content or view semantics, inspect renderers and detail views for reconstruction from live runtime state instead of stored event records.

**Planning state parity:**
- When making a new action handler's affordance enumeration live through the planner's search pipeline, verify that every `RuntimeBeliefView` method the handler calls is implemented on `PlanningState` (via `PlanningSnapshot`), not just on `PerAgentBeliefView`. The planning state's view defaults most trait methods to `None`.
- For trait-extraction tickets that move `RuntimeBeliefView` methods onto new sub-traits, audit `PlanningState` / `PlanningSnapshot` parity before broad mock fallout. When the snapshot doesn't carry the lawful backing state, widen the snapshot boundary deliberately rather than defaulting to `None`.

## Planner traceability and search filters

When a planner/search change introduces a new pre-successor candidate filter, pruning mode, or other search-loop omission path, explicitly sweep the planner-owned traceability surfaces for that new boundary: candidate/root outcome enums, filter reasons, expansion summaries, and any existing omission/provenance structures that should explain the filtered branch. Do not leave the new planner filter behavior invisible in decision-trace inventories when the trace surface already claims to explain candidate loss.

When a planner/search filter consumes a per-tick derived read-model or index, trace the whole handoff before patching: the read-phase constructor, the production planning call, any test-only or public wrappers that need neutral defaults, the search/heuristic consumer, and the trace surface that should explain retained or pruned candidates. Treat wrappers that default to empty/neutral data as acceptable only when they delegate to the canonical implementation and do not create a second lawful behavior path.

## Profile/component absent negative cases

When a ticket's proof or negative case depends on a profile, component, or carrier being absent, verify whether that data is actually optional on the live runtime subject under test. If the runtime bootstrap or factory path seeds it universally by default, correct the ticket and proof surface to the lawful distinction that still exists (for example self vs. non-self access, empty contents vs. missing carrier, or pre-perception vs. post-perception state) instead of writing tests around an impossible "component missing" state.
For cross-crate accessor or belief/profile read tickets over universal components, prove the live boundary directly: if the component is universally seeded, default the negative case to `self` vs. `non-self` visibility or another lawful access distinction, not to component absence.

## Planner-visible belief and snapshot carriage

For planner-visible belief, profile, or snapshot-completeness tickets, verify the full carriage path before coding: runtime belief view -> snapshot builder -> snapshot storage -> `PlanningState`/planner-facing view surface. Do not stop at the final accessor if planner-visible data can be dropped earlier in the pipeline.
For planner behavior coverage tickets that add representative goal tests, also verify that the local test harness or belief fixture carries the full lawful planner inputs for each goal family under test before treating a failure as a production contradiction. Profiles, routes, violation records, evidence carriers, and similar planner-visible state often need fixture support even when the production planner path is already correct.
Before writing positive FF/RPG heuristic assertions on a planner/search fixture, confirm that the fixture's active tactical goal facts are actually reachable from the current expansion's successor operator set. If those facts are not reachable at that expansion, use the fixture only for dead-end or fallback assertions instead of treating missing positive FF trace fields as a production contradiction.

## Dedicated goal-root and planner-root tickets

For dedicated goal-root, planner-root, or golden-isolation tickets, verify that the claimed downstream effect is uniquely attributable to the named goal/root rather than already reachable through a more generic operator family. If a generic path can already lawfully produce the same outcome, narrow the ticket and scenario so they prove the dedicated goal's distinct contract instead of over-claiming a broader downstream chain.

When a staged planner module or substrate already supports multiple goal families, verify each proposed live family against existing conformance/golden ownership before integrating them together. If live proof only clearly justifies part of that staged surface, default the ticket to the narrowest goal-family slice that is already supported rather than activating every plausible family at once.

For planner-root and tactical-barrier tickets, verify that each planner-produced subgoal is a lawful tactical destination rather than a transient probe, fallback waypoint, or exploration scaffold. Do not assume every emitted subgoal should become a scoped barrier target just because it passes through the planner; if the live search contract treats a subgoal as exploratory carriage rather than a durable destination, keep the ticket scoped to the lawful destination family and record the deviation explicitly.

## Planner output consumption and tactical-layer scope

When a planner ticket changes the shape of strategic output, verify how much of that output the downstream tactical/search layer actually consumes. If the live boundary only reads the first/current strategic step, do not author or implement a multi-step strategic fallback shape as though later steps are planner-visible; correct the ticket to the real consumed contract before coding.
When a planner-side filter, selector, or helper derives scope from a goal-level value, also verify whether the live tactical/search boundary sometimes operates on an active subgoal or stage-local contract instead of the root goal alone. If tactical search is currently solving a staged prerequisite, social-query, or other intermediate commodity/path contract, keep the implementation aligned with that active tactical contract rather than pruning or routing solely from the root goal's top-level shape.

Before making a generic planner fallback live as a tactical barrier, check whether grounded goals with explicit evidence carriers (`evidence_entities`, `evidence_places`, or equivalent exact-bound evidence) should keep their existing evidence-backed search path instead. Do not let a new generic probe barrier override lawful evidence-backed routing or exact-goal operator paths unless the ticket explicitly owns that broader change.
