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
   BeliefStatusTag::Probable`). A stale, disputed, or contradicted inventory belief
   produces an opportunity that *claims to be probable*. The `BeliefStatusTag` enum
   already carries `Certain | Probable | Stale | Disputed | Contradicted`
   (`decision_event_payload.rs:281-287`), and the underlying `BelievedEntityState`
   carries `last_observed_tick()` and the belief envelope's status axis — none of it
   reaches the opportunity.
2. **Hard-coded required action.** Every compiled inventory opportunity sets
   `required_actions: vec![PlannerOpKind::MoveCargo]` (`compile.rs:127`), even though
   the compiler already computes the set of actions that produce the relevant effect
   via `EffectSchemaIndex::actions_producing(EffectFactKey::CommodityTransfer)`
   (`compile.rs:23-28`, used only as an emptiness gate). The opportunity therefore
   advertises a fixed action family that may not match the lawful producers in the
   active scenario's action registry.

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

**Evidence sources.** `reports/ai-architecture-improvements-second-iteration.md`
Proposals 2 and 5; verified against `opportunity_compiler/compile.rs`,
`opportunity_compiler/types.rs`, `effect_schema_index.rs`, and `planner_ops.rs`.
**Key interview decision:** narrow source-fidelity scope; absorb Proposal 5's
source-status faithfulness; exclude the canonical-substrate rewrite.

## Phase and Status

Adjunct Wave: AI Architecture Improvements — First Iteration. Draft, pending ticket
decomposition.

## Crates

- `worldwake-ai` — `opportunity_compiler/compile.rs` (the `source_belief()` helper
  and the `required_actions` construction), `opportunity_compiler/types.rs` (no shape
  change — `source_belief: BeliefRef` and `required_actions: Vec<PlannerOpKind>`
  already exist), `effect_schema_index.rs` (reuse `actions_producing`),
  `planner_ops.rs` (reuse the `ActionDefId`→`PlannerOpKind` classifier),
  `decision_trace.rs` / `scenario_diagnostics` (verify the now-truthful status flows
  to the opportunity-compiler trace).
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
   `BelievedEntityState` for the inventory claim: map the belief envelope's status
   axis and freshness (relative to the live staleness policy) onto `BeliefStatusTag`
   — `Contradicted`/`Disputed` pass through, decayed-past-freshness maps to `Stale`,
   high-confidence direct observation maps to `Certain`, otherwise `Probable`. A
   compiled opportunity never claims more certainty than its source carries.
2. **Derived required actions.** `required_actions` is built from the lawful producers
   the compiler already enumerates: `EffectSchemaIndex::actions_producing(
   EffectFactKey::CommodityTransfer)` mapped to `PlannerOpKind` through the existing
   `planner_ops` classifier, deduplicated and `BTreeSet`-stable. The `MoveCargo`
   literal is removed; if the classifier yields `MoveCargo` for the active registry it
   appears because the registry produces it, not because it was hard-coded.
3. **No shape change, no new emitter, no deleted emitter.** `Opportunity`'s fields are
   unchanged. Candidate-generation parity is preserved: the same opportunities are
   emitted, now carrying truthful status and registry-derived actions.
4. **Determinism.** Status derivation is a pure function of belief state; action
   derivation iterates the registry deterministically.
5. **Faithful trace.** The opportunity-compiler decision trace and scenario
   diagnostics surface the real status distribution, so "all compiled opportunities
   are Probable" can no longer be true by construction.

## Non-Goals

- **Canonical opportunity substrate / emitter unification.** Deferred. No parallel
  emitter is migrated or deleted.
- **New opportunity families** (consume/produce/harvest/trade/ask/consult/search).
  The compiler's known-inventory acquisition scope is unchanged.
- **New belief-view accessors.** Status and freshness are read from the existing
  belief surface; if a needed read is genuinely absent it is added as a belief-backed
  accessor, but the expectation is reuse.
- **Changing salience, risk, legal-status, or social-exposure computation.**

## FOUNDATIONS Alignment

| Principle | How satisfied |
|-----------|---------------|
| FND-3 (Concrete state over abstract scores) | `required_actions` is derived from the concrete effect→action registry, not a magic literal. |
| FND-15 (Knowledge carriers carry provenance/freshness) | `source_belief` carries the real status and freshness of the inventory belief. |
| FND-16 (Stale/false/contradicted first-class) | A stale or contradicted inventory belief produces a `Stale`/`Contradicted` opportunity, not a `Probable` one. |
| FND-27 (Summaries are caches, never truth) | The opportunity (a derived view) no longer over-asserts certainty beyond its source belief. |
| FND-29 (Debuggability) | The compiler trace shows the true status distribution and the registry-derived action set. |

## Deliverables

### D1. Derived `source_belief` status

Rewrite `source_belief()` to take the `BelievedEntityState` (already in hand) and
compute `BeliefStatusTag` from its status axis + freshness against the live staleness
policy, instead of the `Probable` literal. `claim_held_at_tick` continues to use
`last_observed_tick()`.

### D2. Derived `required_actions`

Replace `required_actions: vec![PlannerOpKind::MoveCargo]` with a set built from
`EffectSchemaIndex::actions_producing(EffectFactKey::CommodityTransfer)` mapped through
the `planner_ops` `ActionDefId`→`PlannerOpKind` classifier, deduplicated,
`BTreeSet`-ordered. Compute once per compile pass (the producer set is registry-fixed)
and reuse for every emitted opportunity.

### D3. Trace/diagnostics verification

Confirm the opportunity-compiler decision trace and `ScenarioDiagnosticsReport`
opportunity metrics surface the derived status; add a status-distribution assertion if
the current trace aggregates only counts.

## FND-01 Section H

1. **Information-path analysis.** No new information path. The status now read was
   already in the belief envelope the compiler consumes; the required-action set was
   already enumerated for the emptiness gate. This spec stops discarding both.
2. **Positive-feedback analysis.** None introduced. The compiler remains a per-tick
   derived view with no self-amplifying loop.
3. **Concrete dampeners.** Not applicable (no loop). Existing salience floor /
   learned-memory damping / caps are unchanged.
4. **Stored state vs. derived read-model.** No new stored state. `Opportunity` remains
   a transient derived view; `source_belief.status` and `required_actions` become
   *more faithfully* derived. No derived value is promoted to authoritative state.
5. **Planner-formalism analysis.** Plain GOAP/affordance opportunity emission;
   unchanged formalism. No HTN, no method-required behavior.
6. **Causal-equivalence contract.** Not applicable — no compression, offscreen sim,
   or new save-load surface. `Opportunity` is not persisted authoritatively.
7. **Systemic-validation analysis.** Negative illegal paths: (a) an opportunity
   claiming higher certainty than its source belief; (b) `required_actions`
   advertising an action family no registry action produces; (c) candidate parity
   regression (an opportunity disappearing). Checks: focused unit tests on
   `source_belief` status derivation across all `BeliefStatusTag` cases and on
   registry-derived actions; a candidate-parity test asserting the same opportunity
   keys emit before/after; the existing survival/acquisition goldens must not regress.

## SystemFn Integration

No new `SystemFn`. The compiler runs in the existing `agent_tick` read phase.

## Component Registration

No new components.

## Cross-System Interactions (FND-26)

AI-internal only: `opportunity_compiler` reads the belief view and the
`EffectSchemaIndex` (built from the action registry). No cross-system call.

## Profile-Driven Parameters

No new parameters. Status freshness derivation reads the live staleness policy already
threaded through the belief envelope.

## Authoritative-to-AI Impact Analysis

1. `get_affordances` — N/A.
2. `generate_candidates` — affected only in the *content* of the opportunity records
   passed in (truthful status, derived actions); the same opportunities emit.
3. `search_plan` — unaffected (required_actions informs relevance, and the derived set
   for the standard registry includes the same transfer ops).
4. `BestEffort` — N/A.
5. `handle_plan_failure` — N/A.
6. Payload revalidation — N/A (no synthesized payloads added).
7. Golden tests — candidate-parity + survival goldens must pass.

## Validation and Falsification (FND-31)

- **Focused**: status-derivation matrix (Certain/Probable/Stale/Disputed/Contradicted
  inputs → expected tag); registry-derived `required_actions` test.
- **Parity**: opportunity-key set unchanged before/after across an acquisition
  scenario.
- **Negative cases**: no opportunity over-asserts certainty; no phantom action family.
- **No-regression**: acquisition/survival goldens unaffected.

## Risks

- **Status mapping drift.** The belief-axis→`BeliefStatusTag` mapping must be a single
  documented function reused anywhere else that maps the same axis (grep before
  adding) to avoid a second mapping (FND-28). The focused matrix locks it.
- **Registry-derived action surprise.** If a scenario registers a transfer producer
  the planner classifies to an unexpected `PlannerOpKind`, the derived set will
  reflect it. That is correct behavior; the parity test guards against opportunity
  loss, and any genuinely-new op surfaces in review rather than being masked by the
  `MoveCargo` literal.
