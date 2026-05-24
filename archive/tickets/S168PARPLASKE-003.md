# S168PARPLASKE-003: Resume consumption + `PartialPlanResumeTrace`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `crates/worldwake-ai/src/search/mod.rs` (new `search_plan_seeded` tactical-search entry); `crates/worldwake-ai/src/agent_tick/planning.rs` (resumed reusable skeletons call seeded search during candidate planning); `crates/worldwake-ai/src/agenda_manager.rs` (`try_resume_partial_plan_with_trace` gates skeleton reuse and emits trace while preserving the public `try_resume_partial_plan` wrapper); `crates/worldwake-ai/src/decision_trace.rs` (new `PartialPlanResumeTrace` struct).
**Deps**: `archive/tickets/S168PARPLASKE-001.md` (`revalidate_skeleton_step` + `SkeletonRevalidationContext` + `SkeletonRevalidationVerdict`); `archive/tickets/S168PARPLASKE-002.md` (budget-exhausted populated `remaining_skeleton` to consume); `archive/tickets/S168PARPLASKE-006.md` (information-barrier producer for end-to-end reuse paths); `specs/S168-partial-plan-skeleton-reuse.md` (D3, D4).

## Problem

Before this ticket, ticket 001 produced the revalidation function, ticket 002 populated `remaining_skeleton` for budget-exhausted segments, and ticket 006 produced selected-plan information-barrier segments. This ticket consumes populated skeletons generically: `try_resume_partial_plan_with_trace` reads the populated skeleton, calls `revalidate_skeleton_step` to gate reuse, records `PartialPlanResumeTrace`, and preserves the seed only when every step is reusable. The later planning pass invokes `search_plan_seeded`, which walks the skeleton's high-level ops as search-control bias while rebuilding tactical detail (bindings, durations, costs) through ordinary search.

On `Invalid` or `None`, the existing `Pending` full-replan re-entry (`agenda_manager.rs:135`) is preserved unchanged — the seeded path is a strict optimization over the existing fallback (FND-12: performance compresses computation, never causality).

D4's `PartialPlanResumeTrace` struct lives in `decision_trace.rs` (parallel to `RepairAttemptTrace`) and is emitted from the traced resume decision point, carrying the reuse-vs-replan decision, the per-step revalidation verdict, and (on reuse) the seeded skeleton ops. D3 and D4 landed in the same ticket because D4's emit site is the D3 resume integration.

## Assumption Reassessment (2026-05-24)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Codebase shape**. Verified:
   - `try_resume_partial_plan` (`crates/worldwake-ai/src/agenda_manager.rs:86-145`) currently sets `entry.phase = AgendaPhase::Pending` at line 135 unconditionally on resume condition satisfaction. It increments `resume_attempt_count` at line 128 and enforces patience via `if u32::from(segment.resume_attempt_count) > patience_limit` at line 130.
   - `search_plan_with_trace_metadata_and_source` is imported at `agent_tick/planning.rs:25`. Its current signature takes no skeleton parameter (verified by `/reassess-spec` agent 2).
   - `RepairAttemptTrace` (`decision_trace.rs:197`) is the analog the new `PartialPlanResumeTrace` parallels. Existing trace event types include `FrameTransitionKind::Resumed` at `decision_trace.rs:61` (records agenda frame resumption but does not carry skeleton/revalidation detail).
   - Decision trace sink installation pattern: traces are appended to the per-agent trace via the existing decision-trace surface. Read `RepairAttemptTrace` consumers (~5-10 sites) to confirm the existing emit/observation pattern before authoring the new trace's wiring.
2. **Spec/doc references**. S168 D3 (`specs/S168-partial-plan-skeleton-reuse.md:176-192`) names `search_plan_seeded` and the search-control-bias semantics. D4 (lines 194-201) names `PartialPlanResumeTrace` and the emit point. The decision to use a separate function (not an optional parameter on `search_plan_with_trace_metadata_and_source`) is explicit in the spec — preserves the unseeded entry's tracing/termination contract.
3. **Mixed-layer boundary**. This is a cross-module AI ticket but stays AI-internal. Shared boundaries under audit:
   - `try_resume_partial_plan` → `revalidate_skeleton_step(SkeletonRevalidationContext { actor, goal: &segment.goal, step, view })` (ticket 001) → verdict.
   - `try_resume_partial_plan_with_trace` → `PartialPlanResumeTrace` emit (new, decision_trace sink).
   - `build_candidate_plans_with_sources` → `search_plan_seeded` (new) when a resumed reusable skeleton remains on the pending entry → planned plan or fallback.
4. **Phase distinction (precision rule 1)**. The seeded path replaces the *plan search* phase only. Candidate generation, ranking, suppression, filtering, and authoritative outcome are unchanged. Spec is explicit: the seeded path does not replay actions; it rebuilds tactical detail through ordinary search.
5. **Live `GoalKind` / operator surface under test**. Ticket 003's resume-integration tests should exercise a suspended segment whose skeleton is preservable (e.g., commodity-acquisition where the skeleton's ops are not combat / target-identity-bound). The exact `GoalKind` used in the focused tests can mirror what ticket 002's tests use for filter coverage; end-to-end information-barrier production is owned by ticket 006. The reusable + invalid verdict paths must both be exercised.
6. **Heuristic removal discipline (precision rule 12)**. This ticket does NOT remove any existing heuristic — it adds a substrate (seeded search) and a new code path. The existing `Pending` full-replan re-entry remains as the fallback when the verdict is `Invalid` or the skeleton is `None`. No regression risk in unrelated scenarios because the fallback is bit-for-bit unchanged.
7. **Ordering contract (precision rule 4)**. The verdict-then-emit ordering: revalidation runs first; the trace is emitted with the verdict; on `Reusable`, the trace also names the seeded ops *before* search runs (so a search-time bug that crashes doesn't lose the trace context). Confirm this ordering in the implementation.
8. **ControlSource / runtime intent (precision rule 11)**. This ticket does not manipulate `ControlSource`. The skeleton's revisability is governed by the existing `resume_attempt_count`/patience-limit machinery (unchanged) — the verdict-driven discard simply increments retry attempts when the skeleton is invalid.
9. **Existing test coverage to extend**:
   - `try_resume_partial_plan_returns_segment_when_resume_condition_holds:1828` — exercises the resume-success path. Extend to assert the new trace emit and the verdict-routing behavior.
   - `try_resume_partial_plan_leaves_suspended_when_resume_condition_is_unsatisfied:1860` — exercises the negative case where resume doesn't fire. Confirm no trace emit when resume doesn't fire.
   - `try_resume_partial_plan_abandons_before_resume_when_abandon_condition_holds:2031` and `try_resume_partial_plan_abandons_when_resume_attempt_exceeds_patience_limit:2061` — exercise abandon paths. Confirm trace emit for the abandon route names the reason.
10. **Adjacent contradictions**. The seeded search's internal fallback (D3: "if any op cannot be satisfied, it falls back internally to unconstrained search rather than returning failure") could mask a search-control bug if not traced. Classification: this is a required consequence of the seeded design — the trace must record both "reuse attempted" and "fallback internal-to-search occurred" so downstream debugging can distinguish "skeleton seeded successfully" from "skeleton seeded but fallback ran inside."
11. **Implementation boundary correction**. Live code keeps `try_resume_partial_plan` at the agenda/belief boundary, before planning snapshots, action registries, handlers, execution budgets, and recipe registries exist. Calling `search_plan_seeded` directly inside that public function would either widen the public resume API across unrelated planning context or duplicate the planning pipeline. The landed boundary is therefore: traced resume validates and preserves/discards the skeleton; `build_candidate_plans_with_sources` sees the preserved skeleton on the pending entry and invokes `search_plan_seeded` with the ordinary planning context. This preserves the public resume API for existing golden harnesses while still making reusable skeletons seed tactical search in the same tick.

## Architecture Check

1. **Separate `search_plan_seeded` function preserves the unseeded entry's contract.** The spec mandates this explicitly: conflating the seeded and unseeded paths via an optional parameter on `search_plan_with_trace_metadata_and_source` would complicate tracing and termination semantics for callers that don't pass a skeleton. The new function lives alongside and shares the core search machinery.
2. **No backward-compatibility shim.** The existing `Pending` re-entry is preserved as the lawful fallback, not as a compatibility path — it remains the canonical response when revalidation fails. There's no deprecated path to remove.
3. **D4's trace parallels `RepairAttemptTrace`.** The repair attempt machinery already provides the precedent for "tried a planning shortcut, here's what happened." `PartialPlanResumeTrace` mirrors the structure: outcome + reason + (on success) what was seeded.
4. **Verdict-then-emit-then-search ordering protects the trace from search-time crashes.** Even if `search_plan_seeded` panics or hits an internal error, the trace records the revalidation verdict and the seeded-ops decision. This is FND-29 (debuggability) materialized.
5. **D3 + D4 combined avoids transient dead-code state (FND-28).** Landing D4 first would leave a trace struct with no emitter; landing D3 first would leave an emitter writing into a missing struct. Both together: the workspace compiles, the live authority path is clean, no orphaned types.

## Verified Layers

1. **Verdict-driven routing** → decision-trace assertion: focused unit test on `try_resume_partial_plan` that sets up a populated skeleton, mocks `revalidate_skeleton_step` (or constructs predicates that produce `Reusable`/`Invalid`), and verifies the emitted `PartialPlanResumeTrace` records the chosen route.
2. **Seeded search produces a valid plan or falls back internally** → focused unit test on `search_plan_seeded` with a synthetic skeleton + planning context. Assert: (a) when the skeleton is satisfiable, the returned plan rebuilds tactical detail through normal search; (b) when an op cannot be satisfied, the function falls back internally and still returns a plan (or `None` if even unconstrained search fails).
3. **Trace emit covers both success and failure paths** → decision-trace assertions in focused tests: on `Reusable` → trace carries verdict + seeded ops; on `Invalid` → trace carries verdict + reason; on `None` (no skeleton) → trace carries the "no skeleton to reuse" decision.
4. **`Pending` fallback semantics unchanged on `Invalid`/`None`** → focused test confirms `entry.phase = AgendaPhase::Pending` (line 135) is still set on the fallback paths; behavior equivalent to today's resume.
5. **Patience-limit interaction preserved** → focused test: `Invalid` verdict still increments `resume_attempt_count` (line 128), and the patience-limit check at line 130 still abandons after exhaustion. The seeded path is a strict optimization over the existing bounded-reuse machinery.

Per precision rule 5, each invariant maps to a single proof surface — decision-trace assertions for AI reasoning (precision rule 6), focused unit tests for tactical-search behavior. No action trace, event-log delta, or authoritative world-state assertion is needed at this layer; ticket 004's goldens cover the cross-system observable behavior.

## Landed Changes

### 1. `PartialPlanResumeTrace` struct (D4)

In `crates/worldwake-ai/src/decision_trace.rs`, after `RepairAttemptTrace`:

```rust
pub struct PartialPlanResumeTrace {
    pub segment_id: PartialPlanSegmentId,
    pub decision: PartialPlanResumeDecision,
    pub per_step_verdicts: Vec<SkeletonRevalidationVerdict>,
    pub seeded_ops: Option<Vec<PlannerOpKind>>,
}

pub enum PartialPlanResumeDecision {
    ReusedSeededSearch,
    FallbackToReplanInvalid(SkeletonRevalidationReason),
    FallbackToReplanNoSkeleton,
    Abandoned,
}
```

`AgentDecisionTrace` now carries `partial_plan_resumes: Vec<PartialPlanResumeTrace>`, and `lib.rs` re-exports the new trace types.

### 2. `search_plan_seeded` entry point (D3)

In `crates/worldwake-ai/src/search/mod.rs`, alongside `search_plan_with_trace_metadata_and_source`:

- Added `pub(crate) fn search_plan_seeded(skeleton: &[PlannedSkeletonStep], …rest of search context…) -> PlanSearchResult`.
- The seeded entry delegates to the same `search_plan_inner` used by the unseeded entry. At each expansion depth, a matching skeleton op marks matching successors preferred in the existing dual-frontier preference path.
- If the expected op cannot be satisfied, no preferred successor is added and the existing unconstrained search frontier continues. Final plans still use the same successor construction, precondition filtering, terminal handling, and validation surfaces as unseeded search.
- `crates/worldwake-ai/src/agent_tick/planning.rs::build_candidate_plans_with_sources` calls `search_plan_seeded` only for resumed entries whose `PartialPlanSegment.remaining_skeleton` survived revalidation.

### 3. Resume integration in `try_resume_partial_plan` (D3 + D4)

In `crates/worldwake-ai/src/agenda_manager.rs`:

- Kept the public `try_resume_partial_plan` signature intact and added crate-internal `try_resume_partial_plan_with_trace` for the `agent_tick` decision-trace path.
- After the existing resume-condition checks succeed and the resume-attempt counter is incremented, check whether `segment.remaining_skeleton` is `Some(_)`.
- If `Some(skeleton)`: iterate `skeleton.iter()` calling `revalidate_skeleton_step(SkeletonRevalidationContext { actor, goal: &segment.goal, step, view })` and collect per-step verdicts. If any verdict is `Invalid(_)`, the decision is `FallbackToReplanInvalid(first_invalid_reason)`. If all are `Reusable`, the decision is `ReusedSeededSearch`.
- If `None`: decision is `FallbackToReplanNoSkeleton`.
- Emit `PartialPlanResumeTrace` via `AgentDecisionTrace.partial_plan_resumes` with the segment id, decision, per-step verdicts, and seeded ops (when reused).
- On `ReusedSeededSearch`: keep the skeleton on the resumed pending entry so the planning pass can call `search_plan_seeded` with the ordinary planning context.
- On `FallbackToReplanInvalid(_)` or `FallbackToReplanNoSkeleton`: preserve the existing `entry.phase = AgendaPhase::Pending` fallback and clear the unusable skeleton so the later planning pass runs ordinary unseeded search.
- Patience-limit interaction and `resume_attempt_count` increment remain unchanged; invalid skeleton attempts still count toward patience.

### 4. Focused unit tests

In `crates/worldwake-ai/src/agenda_manager.rs` `#[cfg(test)]`:

1. `try_resume_with_reusable_skeleton_emits_reuse_trace_and_keeps_seed` — populated skeleton + all-reusable verdicts → `ReusedSeededSearch` trace; reusable skeleton remains on the pending entry for seeded search.
2. `try_resume_with_invalid_skeleton_falls_back_to_pending_and_emits_reason` — populated skeleton + one invalid verdict → `FallbackToReplanInvalid(reason)` trace; `entry.phase = AgendaPhase::Pending`.
3. `try_resume_with_no_skeleton_falls_back_to_pending_and_emits_trace` — `None` skeleton → `FallbackToReplanNoSkeleton` trace; `entry.phase = AgendaPhase::Pending`.
4. Existing resume success/fallback tests verify `resume_attempt_count` still increments.
5. `try_resume_partial_plan_leaves_suspended_when_resume_condition_is_unsatisfied` now asserts no `PartialPlanResumeTrace` emit.
6. The abandon-path tests now assert `PartialPlanResumeDecision::Abandoned`.

In `crates/worldwake-ai/src/search/tests.rs`:

7. `search_plan_seeded_satisfies_walkable_skeleton` — synthetic skeleton where all ops are satisfiable → returned plan rebuilds tactical detail through normal search.
8. `search_plan_seeded_falls_back_internally_when_op_unsatisfiable` — synthetic skeleton where one op cannot be expanded as-skeletoned → search internally falls back to unconstrained search for that subtree; still returns a plan.
9. The unconstrained-failure case remains covered by existing ordinary search failure coverage because seeded search now shares `search_plan_inner`; this ticket added the two focused tests for seeded preference and internal fallback.

## Landed Files

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `PartialPlanResumeTrace` + `PartialPlanResumeDecision`; integrate into enclosing trace structure)
- `crates/worldwake-ai/src/search/mod.rs` (modify — add `search_plan_seeded` and shared `search_plan_inner` seed bias)
- `crates/worldwake-ai/src/search/tests.rs` (modify — seeded-search focused tests)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — call seeded search for resumed reusable skeletons)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — collect resume traces into `AgentDecisionTrace`)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — resume revalidation, trace emit, tests)
- `crates/worldwake-ai/src/lib.rs` (modify — re-export new trace types)
- `crates/worldwake-ai/src/scenario_diagnostics/aggregator.rs`, `crates/worldwake-ai/src/survival_forensics.rs`, `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`, and `crates/worldwake-ai/tests/golden_harness/timeline.rs` (modify — constructor fallout for `AgentDecisionTrace.partial_plan_resumes`)

## Out of Scope

- Validation goldens (info-barrier suspend → satisfy → resume; assumption stale → full replan; negative no-skeleton-as-rail) — ticket 004.
- Budget-exhausted skeleton population — ticket 002.
- Information-barrier segment production — ticket 006.
- `revalidate_skeleton_step` function definition — ticket 001.
- Resource/jurisdiction/coordination barrier skeleton reuse — spec Non-Goals.
- Combat plan / target-identity-bound skeleton preservation — spec Non-Goals (filtered at ticket 002's construction sites).
- Changes to agenda arbitration / ranking authority — spec Non-Goals.

## Acceptance Result

### Tests Passed

1. Focused resume and seeded-search unit tests listed in "Landed Changes" §4.
2. Existing resume tests pass with extensions: `try_resume_partial_plan_returns_segment_when_resume_condition_holds`, `try_resume_partial_plan_leaves_suspended_when_resume_condition_is_unsatisfied`, `try_resume_partial_plan_abandons_before_resume_when_abandon_condition_holds`, `try_resume_partial_plan_abandons_when_resume_attempt_exceeds_patience_limit`.
3. Existing suite: `cargo test -p worldwake-ai` passed.

### Invariants

1. The verdict from `revalidate_skeleton_step` deterministically routes resume: all-`Reusable` → preserved seed for seeded search; any `Invalid` → `Pending` fallback with seed cleared; `None` skeleton → `Pending` fallback. No silent third route.
2. `PartialPlanResumeTrace` is emitted on every resume-condition-satisfied tick where a partial plan segment is consulted (including fallback paths). Enforced by tests #1-3, #6.
3. The patience-limit / `resume_attempt_count` bounded-reuse machinery is preserved — `Invalid` verdicts still increment the counter, and the abandon path at `agenda_manager.rs:130` still fires after exhaustion. Enforced by test #4.
4. `search_plan_seeded` never returns a plan that bypasses precondition checks; the validation pipeline is shared with the unseeded entry. Enforced by sharing the core search machinery rather than reimplementing.
5. The trace is emitted at the agenda resume seam before the later planning pass can run `search_plan_seeded`, so a search-time failure does not lose the resume/revalidation context.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` `#[cfg(test)]` — 6 new/extended tests covering resume routing, trace emit, patience-limit interaction (see §4 #1-6).
2. `crates/worldwake-ai/src/search/tests.rs` — 2 new tests covering seeded-search satisfaction and internal fallback.

### Commands Run

1. Passed `cargo test -p worldwake-ai --lib try_resume -- --nocapture`.
2. Passed `cargo test -p worldwake-ai --lib search_plan_seeded -- --nocapture`.
3. Passed `cargo test -p worldwake-ai`.
4. Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.

## Outcome

Completed on 2026-05-24.

- Added `PartialPlanResumeTrace` / `PartialPlanResumeDecision` and attached resume traces to `AgentDecisionTrace`.
- Added traced skeleton revalidation at the agenda resume seam. Reusable skeletons remain on the resumed pending entry; invalid or absent skeletons fall back to the existing pending replan path, and invalid skeletons are cleared so they cannot seed search.
- Added `search_plan_seeded` as a separate tactical-search entry point that shares the ordinary search implementation and uses skeleton ops only as successor preference. If no successor matches the seed, ordinary search continues.
- Wired resumed reusable skeletons into candidate planning so the seeded path runs with the normal planning snapshot, registries, handlers, budgets, and validation pipeline.

## Deviations

- The drafted text placed the `search_plan_seeded` call directly inside `try_resume_partial_plan`. Live reassessment showed that function intentionally lacks tactical planning context, so the landed boundary keeps the public resume API intact and performs seeded search in `build_candidate_plans_with_sources`.
- The drafted third seeded-search failure test was not added as a separate test because seeded and unseeded search now share `search_plan_inner`; the owned seeded-specific behavior is covered by the satisfiable-seed and unsatisfied-seed fallback tests, while ordinary unconstrained failure remains covered by existing search failure tests and the full `worldwake-ai` suite.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib try_resume -- --nocapture`.
- Passed `cargo test -p worldwake-ai --lib search_plan_seeded -- --nocapture`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
