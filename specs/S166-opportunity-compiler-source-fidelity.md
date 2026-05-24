# S166: Opportunity Compiler Source Fidelity

**Status**: DRAFT

## Problem Statement

The opportunity compiler (`crates/worldwake-ai/src/opportunity_compiler/compile.rs`)
is the belief-to-candidate bridge for acquisition opportunities. Two concrete
fidelity defects make it a weaker FND-15/FND-16/FND-27 substrate than the surrounding
architecture:

1. **Hard-coded belief status.** Every compiled opportunity's `source_belief` is
   stamped `BeliefStatusTag::Probable` regardless of the underlying belief's actual
   status or freshness (`compile.rs:211-224`, `source_belief()` — `status:
   BeliefStatusTag::Probable` at line 222). A stale, disputed, or contradicted
   inventory belief produces an opportunity that *claims to be probable*. The
   `BeliefStatusTag` enum already carries `Certain | Probable | Stale | Disputed |
   Contradicted` (`decision_event_payload.rs:281-287`), and a fully working
   derivation already exists at `agent_tick/frame.rs:708-728`
   (`belief_status_tag_for_claim`) that maps a claim's refutation state +
   effective confidence (against the per-agent `BeliefConfidencePolicy` and
   `claim_confidence_threshold`) onto the same tag. The compiler discards it.
2. **Hard-coded required action.** Every compiled inventory opportunity sets
   `required_actions: vec![PlannerOpKind::MoveCargo]` (`compile.rs:127`), even though
   the compiler already computes the set of actions that produce the relevant effect
   via `EffectSchemaIndex::actions_producing(EffectFactKey::CommodityTransfer)`
   (`compile.rs:23-28`, used only as an emptiness gate). The opportunity therefore
   advertises a fixed action family that may not match the lawful producers in the
   active scenario's action registry. With the default registry the truthful set is
   `{Harvest, Craft, Trade, MoveCargo, StockManagement, DropItem, Loot}` (the
   intersection of `effect_keys_for_steps` producing `CommodityTransfer` at
   `effect_schema_index.rs:60-72` with the `classify_action_def` arms at
   `planner_ops.rs:85-145`), not a single literal. The field has **no current
   production consumer** (grep across `crates/` finds zero runtime reads outside
   construction sites); the fix prevents misinterpretation by any future consumer
   and makes the field's claim truthful (FND-29).

Neither defect is a belief leak (the compiler reads only the lawful belief view), so
this is not consolidation re-litigation. It is a faithfulness gap: the opportunity
record over-asserts provenance (FND-27, summaries must not become more certain than
their source) and decouples from the canonical effect→action mapping (FND-3, concrete
state). Accepted in the triage of
`reports/ai-architecture-improvements-second-iteration.md` as Proposal 2, **narrowed**
to source fidelity only. The "canonical typed opportunity substrate / delete parallel
emitters" ambition from the report is **explicitly out of scope** (the report itself
warns against overgeneralizing early; parity-proof burden and refactor risk are not
justified this iteration).

**Pre-existing FND-28 violation absorbed.** Reassessment found that
`belief_status_tag_for_claim` is duplicated verbatim across
`crates/worldwake-ai/src/agent_tick/frame.rs:708-728` and
`crates/worldwake-ai/src/agenda_manager.rs:488-508`. S166 reuses this derivation; to
avoid becoming a third copy and to resolve the existing duplicate in the same pass,
D4 lifts the function into a shared helper that all three call sites consume.

**Evidence sources.** `reports/ai-architecture-improvements-second-iteration.md`
Proposals 2 and 5; verified against `opportunity_compiler/compile.rs`,
`opportunity_compiler/types.rs`, `effect_schema_index.rs`, `planner_ops.rs`,
`agent_tick/frame.rs`, `agenda_manager.rs`, `decision_trace.rs`, and
`scenario_diagnostics/mod.rs`. **Key interview decision:** narrow source-fidelity
scope; absorb Proposal 5's source-status faithfulness; consolidate the existing
duplicate as the natural FND-28 outcome of reusing the derivation; exclude the
canonical-substrate rewrite.

## Phase and Status

Adjunct Wave: AI Architecture Improvements — First Iteration. Draft, pending ticket
decomposition.

## Crates

- `worldwake-ai` — `opportunity_compiler/compile.rs` (rewrite `source_belief()` to
  call the shared derivation from D4; replace the `required_actions` literal with
  the registry-derived set from D2), `opportunity_compiler/types.rs` (no shape
  change — `source_belief: BeliefRef` and `required_actions: Vec<PlannerOpKind>`
  already exist), `effect_schema_index.rs` (extend `EffectSchemaIndex::build` to
  cache `(ActionDefId, Option<PlannerOpKind>)` pairs so the per-tick caller does
  not need the registry), `planner_ops.rs` (reuse `classify_action_def`),
  new `belief_status.rs` module (D4 shared helper),
  `agent_tick/frame.rs` and `agenda_manager.rs` (replace local
  `belief_status_tag_for_claim` definitions with the shared helper),
  `decision_trace.rs` (extend `OpportunityCompilerLoad` with a per-`BeliefStatusTag`
  count), `scenario_diagnostics/mod.rs` (mirror the per-tag count into the
  diagnostics report).
- `worldwake-core` — no change. `BeliefStatusTag` and the belief envelope status axis
  already exist.

## Dependencies

- **S138** (Affordance-to-Opportunity Compiler) — completed/archived. Owns the
  compiler this spec refines.
- **S134** (Canonical Effect Schema) — completed/archived. Owns `EffectStep` →
  `EffectFactKey` and `EffectSchemaIndex`.
- **S113** (Belief Envelope) — completed/archived. Owns `BelievedEntityState`,
  `PerceptionSource`, confidence, and the status axis the derived `BeliefStatusTag`
  reads.

## Design Goals

1. **Truthful source status.** `source_belief.status` is derived from the underlying
   belief claim by calling the existing `belief_status_tag_for_claim` derivation
   (post-consolidation, the shared helper from D4): `Contradicted` when the claim
   is refuted, `Certain` when effective confidence ≥ `2 × claim_confidence_threshold`
   (capped at 1000), `Probable` when ≥ threshold, `Stale` otherwise. `Disputed` flows
   through the existing source-enum→tag map. A compiled opportunity never claims
   more certainty than its source carries.
2. **Derived required actions.** `required_actions` is built from the lawful producers
   the compiler already enumerates: `EffectSchemaIndex::actions_producing(
   EffectFactKey::CommodityTransfer)` mapped to `PlannerOpKind` through the existing
   `planner_ops::classify_action_def` classifier, with `None`-classifying actions
   filtered out, deduplicated, and `BTreeSet`-ordered. The `MoveCargo` literal is
   removed; if the classifier yields `MoveCargo` for the active registry it appears
   because the registry produces it, not because it was hard-coded. With the
   default registry the truthful set is
   `{Harvest, Craft, Trade, MoveCargo, StockManagement, DropItem, Loot}`.
3. **No shape change, no new emitter, no deleted emitter.** `Opportunity`'s fields are
   unchanged. Candidate-generation parity is preserved: the same opportunities are
   emitted, now carrying truthful status and registry-derived actions.
4. **Determinism.** Status derivation is a pure function of belief state; action
   derivation iterates the registry deterministically (`BTreeMap` + `BTreeSet`).
5. **Faithful trace.** `OpportunityCompilerLoad` gains a per-`BeliefStatusTag` count
   map; `ScenarioDiagnosticsReport` mirrors the per-tag aggregation. "All compiled
   opportunities are Probable" can no longer be true by construction.
6. **Single derivation site.** The existing duplicate `belief_status_tag_for_claim`
   functions in `frame.rs` and `agenda_manager.rs` are lifted to a shared helper
   reused by all three call sites, preventing this fix from becoming a third copy
   and resolving the pre-existing FND-28 violation.

## Non-Goals

- **Canonical opportunity substrate / emitter unification.** Deferred. No parallel
  emitter is migrated or deleted.
- **New opportunity families** (consume/produce/harvest/trade/ask/consult/search).
  The compiler's known-inventory acquisition scope is unchanged.
- **New belief-view accessors.** Status and freshness are read through the existing
  `RuntimeBeliefView` surface used by `belief_status_tag_for_claim` today
  (`belief_confidence_policy`, `claim_confidence_threshold`, plus access to the
  claim list keyed by `BeliefClaimKey`).
- **Changing salience, risk, legal-status, or social-exposure computation.**
- **`required_actions` consumers.** No new reader is added. The field has no current
  production consumer; the fix preserves that surface while making the value
  truthful for any future consumer.

## FOUNDATIONS Alignment

| Principle | How satisfied |
|-----------|---------------|
| FND-3 (Concrete state over abstract scores) | `required_actions` is derived from the concrete effect→action registry, not a magic literal. |
| FND-14B (Planner-visible inputs belief-backed) | `source_belief.status` is derived from the actual belief envelope through `RuntimeBeliefView`, not from a hard-coded literal. The compiler continues to read only the lawful belief view. |
| FND-15 (Knowledge carriers carry provenance/freshness) | `source_belief` carries the real status and freshness of the inventory belief. |
| FND-16 (Stale/false/contradicted first-class) | A stale or contradicted inventory belief produces a `Stale`/`Contradicted` opportunity, not a `Probable` one. |
| FND-27 (Summaries are caches, never truth) | The opportunity (a derived view) no longer over-asserts certainty beyond its source belief. |
| FND-28 (No backward compatibility in live authority paths) | The hard-coded literals are removed in place (no shim, no parallel path). The pre-existing duplicate `belief_status_tag_for_claim` is consolidated into a single helper rather than gaining a third copy. |
| FND-29 (Debuggability) | The compiler trace shows the true per-status count and the registry-derived action set. |

## Deliverables

### D1. Derived `source_belief` status

Rewrite `source_belief()` in `crates/worldwake-ai/src/opportunity_compiler/compile.rs`
to derive `BeliefStatusTag` from the underlying belief claim instead of stamping
`Probable`. The function builds the `BeliefClaimKey` it already constructs at
`compile.rs:217-220`, looks up the corresponding `EntityBeliefClaim` from the
`RuntimeBeliefView` already in scope, and calls the shared
`belief_status_tag_for_claim` helper from D4. `claim_held_at_tick` continues to use
`state.last_observed_tick()`. If the claim lookup yields no matching claim (the
belief state is present but no claim exists for the inventory aspect — defensive
guard), the opportunity is not emitted; this case must be impossible in practice
because the compiler only iterates entities the belief view confirms are known.

Signature change: `source_belief()` gains the belief-view reference and the agent
id, both of which are already in scope at the single call site
(`compile.rs:121`). No new public surface is added.

### D2. Derived `required_actions`

Extend `EffectSchemaIndex` in `crates/worldwake-ai/src/effect_schema_index.rs` to
cache the `(ActionDefId, Option<PlannerOpKind>)` mapping at `build()` time. The
registry is already in scope at `effect_schema_index.rs:19`, and pre-computing the
mapping avoids threading `&ActionDefRegistry` through every per-tick caller.
Concretely, add a `by_effect_op: BTreeMap<EffectFactKey, BTreeSet<PlannerOpKind>>`
field populated alongside the existing `by_effect` map; for each action whose
`effect_schema.steps` produce a given `EffectFactKey`, call
`classify_action_def(action_def)`, drop `None` results, and insert into the set.

Expose a new accessor
`EffectSchemaIndex::planner_ops_producing(&self, fact: EffectFactKey) ->
&BTreeSet<PlannerOpKind>` returning an empty set when no producers exist.

In `compile_opportunities`, replace
`required_actions: vec![PlannerOpKind::MoveCargo]` with
`required_actions: planner_ops_for_transfer.iter().copied().collect::<Vec<_>>()`
where `planner_ops_for_transfer = action_index.planner_ops_producing(
EffectFactKey::CommodityTransfer)` is bound once before the outer `for` loop at
`compile.rs:45` so the set is computed once per compile pass.

With the default registry the resulting set is
`{Harvest, Craft, Trade, MoveCargo, StockManagement, DropItem, Loot}` (sourced from
the intersection of `effect_keys_for_steps` arms producing `CommodityTransfer` at
`effect_schema_index.rs:60-72` and the matching `classify_action_def` arms at
`planner_ops.rs:85-145`). Actions whose classifier returns `None` (e.g., a generic
`transfer` action) are filtered out — the opportunity advertises only ops the
planner can actually emit.

**Consumer note.** `Opportunity.required_actions` has no current production reader
(grep across `crates/` confirms zero non-construction call sites). This fix is
fidelity-preserving substrate work, not a current planner correctness fix; it
ensures any future consumer reads a truthful value.

### D3. Trace and diagnostics for derived status distribution

Extend `OpportunityCompilerLoad` in `crates/worldwake-ai/src/decision_trace.rs:1008`
with `compiled_by_status: BTreeMap<BeliefStatusTag, u32>`, populated as each
opportunity is emitted in `compile_opportunities`. `BeliefStatusTag` derives `Copy +
Ord + Hash + Serialize + Deserialize` (`decision_event_payload.rs:280`), so the map
is determinism-safe.

Mirror the new field on `ScenarioDiagnosticsReport` in
`crates/worldwake-ai/src/scenario_diagnostics/mod.rs` alongside the existing
`opportunity_compiled_count`/`opportunity_salience_floored`/
`opportunity_learned_memory_damped`/`opportunity_cap_truncated` percentile buckets,
using the per-tag aggregation shape the report already uses for analogous
distributions. Specific shape and aggregator selection is delegated to the
implementing ticket so it matches whatever distribution convention the report
currently applies for `BeliefStatusTag`-keyed maps.

Add a focused diagnostics assertion: across an acquisition scenario with mixed
belief freshness (e.g., one direct-observation claim + one decayed-past-threshold
claim seeded by the test), the reported `compiled_by_status` distribution must
contain at least two distinct tags. Without this assertion the post-fix
`all-Probable` regression cannot be detected.

### D4. Consolidate `belief_status_tag_for_claim` into a shared helper

Create a new `crates/worldwake-ai/src/belief_status.rs` module that exposes a
single `pub(crate) fn belief_status_tag_for_claim(view: &dyn RuntimeBeliefView,
agent: EntityId, claim: &EntityBeliefClaim, tick: Tick) -> BeliefStatusTag`
implementing the existing logic verbatim (refutation check → `Contradicted`;
effective confidence vs. threshold and `2 × threshold` floor → `Certain`/`Probable`/
`Stale`).

Delete the duplicate definitions at `crates/worldwake-ai/src/agent_tick/frame.rs:708-728`
and `crates/worldwake-ai/src/agenda_manager.rs:488-508`, replace each `fn`-local
call with the shared helper (re-exported through `lib.rs` as needed), and add D1's
new call from `opportunity_compiler/compile.rs` as the third consumer.

No behavior change at either existing call site; this is a pure FND-28 cleanup
required by D1's reuse. Verify by re-running the existing AI crate test suite,
which exercises both surviving call sites today.

## FND-01 Section H

This is an (e)+(b) bugfix-with-consolidation spec. Section H is abbreviated to the
declarations the deliverables change.

1. **Information-path analysis.** No new information path. The status now read was
   already in the belief envelope the compiler consumes; the required-action set was
   already enumerated for the emptiness gate. This spec stops discarding both. The
   D4 consolidation moves an existing computation behind a single function symbol
   without changing what it reads.
2. **Positive-feedback analysis.** None introduced. The compiler remains a per-tick
   derived view with no self-amplifying loop.
3. **Concrete dampeners.** Not applicable (no loop). Existing salience floor /
   learned-memory damping / caps are unchanged.
4. **Stored state vs. derived read-model.** No new stored state. `Opportunity` remains
   a transient derived view; `source_belief.status` and `required_actions` become
   *more faithfully* derived. The new `compiled_by_status` map and its diagnostics
   mirror are derived counts, not authoritative state. No derived value is promoted
   to authoritative state.
5. **Planner-formalism analysis.** Plain GOAP/affordance opportunity emission;
   unchanged formalism. No HTN, no method-required behavior.
6. **Causal-equivalence contract.** Not applicable — no compression, offscreen sim,
   or new save-load surface. `Opportunity` is not persisted authoritatively.
7. **Systemic-validation analysis.** Negative illegal paths: (a) an opportunity
   claiming higher certainty than its source belief; (b) `required_actions`
   advertising an action family no registry action produces; (c) candidate parity
   regression (an opportunity disappearing); (d) cap-truncation ordering regression
   when `source_belief.status` (the tie-breaker at `compile.rs:140-146`) flips from
   always-`Probable` to varied values. Checks: focused unit tests on
   `source_belief` status derivation across all `BeliefStatusTag` cases and on
   registry-derived actions; a candidate-parity test asserting the same opportunity
   keys emit before/after; an explicit cap-stress parity scenario where multiple
   opportunities share `(salience, key)` but differ in status, asserting the
   cap-truncated subset is stable across the change; the existing
   survival/acquisition goldens must not regress.

## SystemFn Integration

No new `SystemFn`. The compiler runs in the existing `agent_tick` read phase
(specifically inside `refresh_runtime_for_read_phase_with_memories()` at
`crates/worldwake-ai/src/agent_tick/observation.rs:240`).

## Component Registration

No new components.

## Cross-System Interactions (FND-26)

AI-internal only: `opportunity_compiler` reads the belief view and the
`EffectSchemaIndex` (built from the action registry). The D4 shared helper is read
by `agent_tick/frame.rs`, `agenda_manager.rs`, and `opportunity_compiler/compile.rs`
— all within `worldwake-ai`. No cross-system call.

## Profile-Driven Parameters

No new parameters. Status derivation reads the existing per-agent
`BeliefConfidencePolicy` (`crates/worldwake-core/src/belief.rs:2565-2587`,
`staleness_penalty_per_tick: Permille`) and `claim_confidence_threshold` already
threaded through `RuntimeBeliefView`.

## Authoritative-to-AI Impact Analysis

1. `get_affordances` — N/A.
2. `generate_candidates` — affected only in the *content* of the opportunity records
   passed in (truthful status, derived actions); the same opportunities emit.
3. `search_plan` — unaffected (required_actions has no production consumer, and the
   derived set for the standard registry includes the same transfer ops the planner
   already discovers via affordance search).
4. `BestEffort` — N/A.
5. `handle_plan_failure` — N/A.
6. Payload revalidation — N/A (no synthesized payloads added).
7. Golden tests — candidate-parity + survival goldens must pass; the explicit
   cap-stress parity scenario (Section H point 7) is required to cover the
   tie-breaker ordering change.

## Validation and Falsification (FND-31)

- **Focused**: status-derivation matrix (Certain/Probable/Stale/Disputed/Contradicted
  inputs → expected tag, achieved by varying claim refutation, effective confidence,
  and the per-agent threshold); registry-derived `required_actions` test asserting
  the expected set under the default action registry; D4 call-site equivalence test
  asserting `frame.rs`, `agenda_manager.rs`, and the new compiler call all return
  identical tags for identical inputs.
- **Parity**: opportunity-key set unchanged before/after across an acquisition
  scenario; additionally, the cap-truncated subset is unchanged across a
  cap-stress scenario where `compile_opportunity_cap` is exceeded by opportunities
  whose `(salience, key)` ties resolve through `source_belief` (proves the
  tie-breaker order does not silently reshuffle).
- **Negative cases**: no opportunity over-asserts certainty; no phantom action
  family; the `compiled_by_status` distribution under mixed-freshness inputs
  contains at least two distinct tags (proves the all-`Probable` regression cannot
  recur silently).
- **No-regression**: acquisition/survival goldens unaffected; the AI crate test
  suite passes (covers D4's two existing call sites).

## Risks

- **Cap-truncation tie-break shift.** When the cap is hit, `compile.rs:140-146`
  sorts by `(Reverse(salience), key, source_belief)` and the `source_belief`
  tie-breaker now includes a varying `BeliefStatusTag`. Different cap-truncated
  subsets are possible if multiple opportunities share `(salience, key)` but
  differ in status. The Validation cap-stress parity scenario locks this.
- **Registry-derived action surprise.** If a scenario registers a transfer producer
  the planner classifies to an unexpected `PlannerOpKind`, the derived set will
  reflect it. That is correct behavior; the parity test guards against opportunity
  loss, and any genuinely-new op surfaces in review rather than being masked by the
  `MoveCargo` literal.
- **D4 consolidation regression.** Replacing two existing call sites with a shared
  helper could mask a subtle behavioral difference if the two existing copies have
  silently diverged. Reassessment confirmed they are presently verbatim-identical;
  the D4 equivalence test pins this. Any future divergence would require an
  explicit branch in the shared helper rather than a quiet copy-paste fork.
