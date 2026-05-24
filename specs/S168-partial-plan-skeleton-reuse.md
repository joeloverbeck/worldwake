# S168: Partial-Plan Skeleton Reuse

**Status**: DRAFT

## Problem Statement

`PartialPlanSegment` already carries `remaining_skeleton: Option<Vec<PlannedSkeletonStep>>`
(`crates/worldwake-ai/src/partial_plan.rs:33`) and a `PlannedSkeletonStep` type
(`partial_plan.rs:44`), the intended substrate for resuming a barrier-suspended pursuit
from a remembered high-level plan instead of re-deliberating from scratch. But the field
is **dead**:

- The budget-exhaustion constructor writes `remaining_skeleton: None`
  (`partial_plan.rs:123`, `budget_exhausted_partial_plan_segment`).
- No code path reads `remaining_skeleton` for planning. Resume
  (`agenda_manager.rs::try_resume_partial_plan`) sets `entry.phase =
  AgendaPhase::Pending` (`agenda_manager.rs:135`) and re-enters the normal agenda
  decision cycle — a full replan — discarding any remembered skeleton.

So a barrier-suspended intention that resumes (information barrier satisfied, budget
window elapsed) re-runs candidate generation, ranking, and tactical search from
scratch even when the agent's high-level pursuit is unchanged. This is wasted bounded
reasoning (FND-20) and a thinner expression of revisable commitment than the
architecture already models (FND-21): commitment persists as an agenda entry, but the
*plan shape* the agent had worked out is forgotten.

S149 itself flagged this as a deliberate deferral: it "does not replay
`remaining_skeleton` directly, because the live skeleton carrier still lacks resolved
`ActionDefId`s, authoritative targets, payloads, and planner context"
(archived `S149` line 11). S168 resolves that constraint by treating the skeleton as a
*search seed* (D3), not an action-dispatch surface — tactical detail (`ActionDefId`s,
targets, payloads) is rebuilt through ordinary search rather than replayed from the
skeleton. The earlier scope was right to defer replay; the present scope adds the
seeding path that S149's deferral pointed to.

A second drift to flag for D1: the information-barrier suspension path in
`agenda_manager.rs:147-180` currently spawns *companion goals* without persisting a
`PartialPlanSegment` in production code (test helpers `information_barrier_segment` /
`coordination_barrier_segment` exist at `agenda_manager.rs:1698, 1719` but are
`#[cfg(test)]`-only). The only runtime suspension constructor today is the
budget-exhausted one. Making the field live at information-barrier suspensions
therefore also requires wiring a new constructor (D1.b below), not only populating an
existing one.

Accepted in the triage of `reports/ai-architecture-improvements-second-iteration.md`
(Proposal 3), explicitly the **lowest-benefit** of the accepted set — an optimization
over an already-working resume path, not a correctness fix. Scoped tightly to avoid
fossilized skeletons.

**Evidence sources.** `reports/ai-architecture-improvements-second-iteration.md`
Proposal 3; verified against `partial_plan.rs` and `agenda_manager.rs`. **Key interview
decision:** scope to information barriers and search-budget barriers; exclude volatile
combat and target-identity bindings; require revalidation-before-reuse so a skeleton
never becomes a rail.

## Phase and Status

Adjunct Wave: AI Architecture Improvements — First Iteration. Draft, pending ticket
decomposition.

## Crates

- `worldwake-ai` — `partial_plan.rs` (populate `remaining_skeleton` at the
  budget-exhausted suspension site; the skeleton-revalidation function may live here
  or in a sibling module), `agenda_manager.rs` (a new info-barrier suspension
  constructor wired from the companion-spawning path; `try_resume_partial_plan`
  consumes the skeleton when present and valid, else falls back to the existing
  `Pending` re-entry; emit `PartialPlanResumeTrace`), `agent_tick/planning.rs` (new
  `search_plan_seeded` tactical-search entry point parallel to
  `search_plan_with_trace_metadata_and_source`), `decision_trace.rs` (new
  `PartialPlanResumeTrace` struct parallel to `RepairAttemptTrace`).
- `worldwake-core` — no new authoritative type. `PartialPlanSegment` /
  `PlannedSkeletonStep` already exist and already serialize.

## Dependencies

- **S149** (Partial Plan Segments and Typed Terminals) — completed/archived. Owns
  `PartialPlanSegment`, `PlannedSkeletonStep`, `PlanTerminalKind`, resume/abandon
  conditions, and `AgendaManager` resume.
- **S114** (Plan Step Guards) — completed/archived. Provides the guard/expectation
  machinery the skeleton revalidation reuses to check skeleton steps against fresh
  beliefs.

## Design Goals

1. **Make the dead field live, in scope.** Populate `remaining_skeleton` when an
   intention suspends at an **information barrier** or **search-budget barrier** and a
   meaningful remaining high-level shape exists. Leave it `None` where no useful
   skeleton exists (a true cold budget exhaustion may legitimately have none).
2. **Revalidate before reuse — never a rail.** On resume, validate each skeleton step
   against the agent's current lawful beliefs (reusing S114 guard/expectation checks)
   before seeding tactical search. If any load-bearing assumption is stale,
   contradicted, or unknown, discard the skeleton and fall back to the existing full
   agenda re-entry. The skeleton accelerates planning; it never authorizes an action
   whose preconditions no longer lawfully hold (FND-21).
3. **Skeleton seeds tactical search, not action dispatch.** Reuse rebuilds tactical
   detail (bindings, durations, costs) through ordinary search seeded by the skeleton's
   high-level ops; it does not replay concrete committed steps. Action legality remains
   GOAP/dispatch's job (FND-20, FND-26).
4. **Bounded reuse.** Honor the existing `resume_attempt_count` /
   `last_resume_attempt_tick` / patience limits; a skeleton that fails revalidation
   repeatedly is abandoned through the existing abandon conditions.
5. **Exclusions.** Do not preserve skeletons for volatile combat plans or
   target-identity-bound steps, where a stale binding is more dangerous than a replan.
6. **Determinism & traceability.** Skeleton construction and revalidation iterate
   stable order; the trace records reuse-vs-replan and the revalidation verdict so
   FND-29 can answer "why did the agent resume its old plan instead of replanning?"

## Non-Goals

- **Skeleton preservation for every barrier type.** Resource/jurisdiction/coordination
  barriers and combat are out of scope this iteration (the report defers them too).
- **New terminal kinds, resume conditions, or abandon conditions.**
- **Persisting concrete committed steps as a resumable plan.** Only the high-level
  skeleton is preserved; tactical detail is rebuilt.
- **Changing the agenda arbitration / ranking authority.** Resume still yields to
  higher-priority intentions through the existing agenda.

## FOUNDATIONS Alignment

| Principle | How satisfied |
|-----------|---------------|
| FND-12 (Performance compresses computation, never causality) | Skeleton reuse is a derived planning cache with an explicit causal-equivalence contract (Section H.6): full replan from the same beliefs must produce a lawful equivalent plan; revalidation gates reuse; the skeleton can be deleted without changing world meaning. |
| FND-20 (Resource-bounded reasoning) | Resume reuses remembered planning work instead of full re-deliberation, within existing budgets/patience. |
| FND-21 (Revisable commitments) | Commitment persists as a remembered skeleton, but every reuse is gated by belief revalidation; a broken assumption discards it. |
| FND-26 (Systems interact through state) | Skeleton reuse seeds search; action legality stays with GOAP/dispatch. No privileged cross-system command. |
| FND-27 (Summaries are caches, not truth) | The skeleton is a derived planning cache, always replaceable by full replan and invalidated by revalidation. |
| FND-29 (Debuggability) | The trace records reuse-vs-replan and the revalidation verdict per skeleton step. |

## Deliverables

### D1.a. Populate `remaining_skeleton` at the budget-exhausted suspension site

In `budget_exhausted_partial_plan_segment` (`partial_plan.rs:118`, which currently
passes `remaining_skeleton: None` to `build_partial_plan_segment` at line 123), thread
a populated `Some(Vec<PlannedSkeletonStep>)` when a meaningful remainder exists beyond
the completed prefix, excluding combat- and target-identity-bound steps. Sites with no
meaningful remainder keep `None`. The caller
`write_budget_exhausted_partial_plan_segments` in `agent_tick/planning.rs:1114` must
also thread the skeleton from the partial search frontier through the seed.

### D1.b. Add an info-barrier suspension constructor

The information-barrier suspension path in `agenda_manager.rs:147-180` currently spawns
companion goals without persisting a `PartialPlanSegment` in production code. Add a new
info-barrier suspension constructor that calls `build_partial_plan_segment`
(`partial_plan.rs:94`) from the companion-spawning path with a populated
`remaining_skeleton`, the appropriate `PlanTerminalKind::InformationBarrier { … }`
terminal, and the corresponding `BarrierFact`. Combat- and target-identity-bound steps
are excluded as for D1.a. The test helpers `information_barrier_segment` and
`coordination_barrier_segment` (`agenda_manager.rs:1698, 1719`) are the shape
reference; the production constructor wires the same shape into the actual suspension
path.

### D2. Skeleton revalidation

A new function `revalidate_skeleton_step` (in `worldwake-ai`, alongside
`agenda_manager.rs` or in a sibling module — implementer's choice) that checks each
`PlannedSkeletonStep` against current lawful beliefs and the stored `GoalOffer` context
needed to resolve goal-relative templates, then returns a verdict: `Reusable` (seed
search) or `Invalid(reason)` (fall back to full replan).

Implementation specifics:

- Evaluate `expected_pre: Vec<BeliefPredicate>` (`partial_plan.rs:44-49`) directly
  against the belief view.
- Optionally reuse S114's `RequiredFact` belief-read helpers
  (`believed_target_location`, `believed_commodity_stock`, etc. via
  `RuntimeBeliefView`) where the skeleton step carries guard-shaped facts.
- Do **not** route through `classify_revalidation` (`plan_revalidation.rs:46-102`) —
  that path requires a fully-instantiated `PlannedStep` (`ActionDefId`, resolved
  bindings, handler binding) which a `PlannedSkeletonStep` (fields `op: PlannerOpKind`,
  `target_template: PayloadTemplate`, `expected_pre: Vec<BeliefPredicate>`) does not
  carry by construction.
- No world-truth read; belief-backed only.

### D3. Resume consumption — `search_plan_seeded`

`try_resume_partial_plan` (`agenda_manager.rs:86-145`) calls D2 on the
`remaining_skeleton` when present. On `Reusable`, it invokes a new tactical-search
entry point `search_plan_seeded(skeleton, …)` in `agent_tick/planning.rs`, parallel to
the existing `search_plan_with_trace_metadata_and_source` (imported at
`agent_tick/planning.rs:25`). The seeded entry walks the skeleton's high-level ops as
search-control bias and rebuilds tactical bindings, durations, and costs through
ordinary search; if any op cannot be satisfied, it falls back internally to
unconstrained search rather than returning failure. On `Invalid` or `None`,
`try_resume_partial_plan` preserves its existing `Pending` full-replan re-entry
(`agenda_manager.rs:135`) unchanged.

A separate function (rather than an optional parameter on the existing search entry)
is chosen because the seeded path has distinct control flow — ops walk before
expansion, with internal fallback — and conflating the two would complicate the
unseeded entry's tracing and termination contract.

### D4. Trace

Define a new `PartialPlanResumeTrace` struct in `decision_trace.rs` parallel to
`RepairAttemptTrace` (`decision_trace.rs:197`), carrying the reuse-vs-replan decision,
the per-step revalidation verdict, and (on reuse) the seeded skeleton ops. Emit from
`agenda_manager.rs::try_resume_partial_plan` (the resume decision point) into the
existing decision-trace sink. `plan_repair.rs` is not on the emit path (it is pure
repair orchestration with no trace exports).

## FND-01 Section H

1. **Information-path analysis.** No new information path. Revalidation reads the
   agent's existing belief state through the lawful belief view; the resume trigger
   (barrier satisfied / window elapsed) is the existing resume-condition machinery.
2. **Positive-feedback analysis.** No new amplifying loop. A potential
   resume→fail→resume churn is bounded by existing attempt counts and patience.
3. **Concrete dampeners.** Existing `resume_attempt_count`, patience limit, and
   abandon conditions (physical: the agent gives up after bounded retries). No numeric
   clamp introduced.
4. **Stored state vs. derived read-model.** No new authoritative type.
   `remaining_skeleton` is an already-serialized field of the authoritative
   `PartialPlanSegment`; this spec changes its *content* at the in-scope sites — for
   the budget-exhausted constructor (D1.a), from `None` to populated; for the new
   info-barrier suspension constructor (D1.b), populated from inception — and adds
   derived revalidation/seeding logic. The skeleton is a planning cache (FND-27), not
   promoted to truth.
5. **Planner-formalism analysis.** Plain GOAP; the skeleton is search seeding/control
   derived from the agent's prior bounded lookahead. Not an HTN method and not a
   method-required leaf. Action legality remains in tactical search/dispatch.
6. **Causal-equivalence contract.** The skeleton is a derived planning cache: deleting
   it and forcing full replan must produce a lawful equivalent plan (no behavior the
   skeleton enables that full replan from the same beliefs could not produce). Referent:
   the full-replan path. Preserved: the agent's beliefs, the resume conditions, action
   legality. The reuse path must not yield an action that full replan from the same
   beliefs would reject — locked by the revalidation gate (D2) and a replay/equivalence
   test. `SAVE_FORMAT_VERSION` is unchanged (the field already serializes; only its
   populated content changes).
7. **Systemic-validation analysis.** Negative illegal paths: (a) a skeleton step
   executing whose preconditions no longer lawfully hold (skeleton-as-rail); (b) reuse
   reading world truth during revalidation; (c) a skeleton preserved for a
   combat/target-identity-bound step; (d) reuse producing a plan full replan would not.
   Checks: focused revalidation tests (reusable / each invalidation reason); a golden
   where an agent suspends on an information barrier, the barrier is satisfied by a
   lawful carrier, and the agent resumes the same pursuit via skeleton reuse with the
   trace showing revalidation; a negative golden where the assumption goes stale and
   reuse correctly falls back to full replan; replay/save-load equivalence on both.

## SystemFn Integration

No new `SystemFn`. Resume runs in the existing agenda/planning phase of `agent_tick`.

## Component Registration

No new components.

## Cross-System Interactions (FND-26)

AI-internal: agenda/partial-plan/planning collaborate through `AgendaState` and the
belief view. No cross-system call.

## Profile-Driven Parameters

No new parameters. Existing sources:

- **Patience / attempt limits**: `IntentionFrame.patience_limit` (S149) and
  `PartialPlanSegment.resume_attempt_count` / `last_resume_attempt_tick`
  (`partial_plan.rs:39-40`), enforced by `try_resume_partial_plan`
  (`agenda_manager.rs:128-130`).
- **Search budgets**: `CognitiveProfile` (existing tactical-search budgets; consumed
  by the seeded entry point identically to the unseeded one).

## Authoritative-to-AI Impact Analysis

1. `get_affordances` — N/A.
2. `generate_candidates` — N/A (reuse seeds tactical search, not candidate emission).
3. `search_plan` — affected: skeleton-seeded search must produce a valid plan or
   correctly fail to the full-replan fallback; verify terminal ordering and that a
   skeleton never bypasses precondition checks.
4. `BestEffort` — N/A.
5. `handle_plan_failure` — affected: an invalid skeleton must route to the existing
   full-replan path without loops; bounded by patience/attempt counts.
6. Payload revalidation — N/A (no new synthesized payloads; tactical search rebuilds
   bindings through normal paths).
7. Golden tests — required (D2 reuse + fallback goldens).

## Validation and Falsification (FND-31)

- **Focused**: revalidation verdict matrix (reusable; each invalidation reason).
- **Golden (reuse)**: information-barrier suspend → lawful carrier satisfies barrier →
  skeleton revalidates → same pursuit resumes via seeded search, trace shows reuse.
- **Golden (fallback)**: assumption goes stale before resume → reuse rejected → full
  replan, trace shows the invalidation reason.
- **Negative cases**: no skeleton-as-rail; no world-truth read; no skeleton for
  combat/target-identity steps.
- **Replay/save-load equivalence**: both goldens replay identically; the budget-
  exhaustion save round-trips with the now-populated skeleton.
- **No-regression**: survival/integration goldens unaffected (resume behavior is
  equivalent or strictly better-bounded).

## Risks

- **Fossilized skeleton.** The central risk; mitigated by mandatory revalidation
  before reuse, attempt/patience bounds, and the fallback golden.
- **Equivalence drift.** Skeleton reuse must not enable a plan full replan would
  reject; the causal-equivalence test (Section H.6) and the negative golden guard this.
- **Low realized benefit.** This is an optimization; if profiling shows resume replans
  are rare or cheap, the population scope can be narrowed further at ticket time
  without affecting the other Adjunct Wave specs.
