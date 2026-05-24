# S168PARPLASKE-003: Resume consumption + `PartialPlanResumeTrace`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — `crates/worldwake-ai/src/agent_tick/planning.rs` (new `search_plan_seeded` tactical-search entry); `crates/worldwake-ai/src/agenda_manager.rs` (`try_resume_partial_plan` calls revalidation + invokes seeded search + emits trace); `crates/worldwake-ai/src/decision_trace.rs` (new `PartialPlanResumeTrace` struct).
**Deps**: `archive/tickets/S168PARPLASKE-001.md` (`revalidate_skeleton_step` + `SkeletonRevalidationContext` + `SkeletonRevalidationVerdict`); `archive/tickets/S168PARPLASKE-002.md` (budget-exhausted populated `remaining_skeleton` to consume); `archive/tickets/S168PARPLASKE-006.md` (information-barrier producer for end-to-end reuse paths); `specs/S168-partial-plan-skeleton-reuse.md` (D3, D4).

## Problem

Ticket 001 produced the revalidation function. Ticket 002 populated `remaining_skeleton` for budget-exhausted segments, and ticket 006 produced selected-plan information-barrier segments. This ticket consumes populated skeletons generically: `try_resume_partial_plan` reads the populated skeleton, calls `revalidate_skeleton_step` to gate reuse, and on `Reusable` invokes a new tactical-search entry point `search_plan_seeded` that walks the skeleton's high-level ops as search-control bias while rebuilding tactical detail (bindings, durations, costs) through ordinary search.

On `Invalid` or `None`, the existing `Pending` full-replan re-entry (`agenda_manager.rs:135`) is preserved unchanged — the seeded path is a strict optimization over the existing fallback (FND-12: performance compresses computation, never causality).

D4's `PartialPlanResumeTrace` struct lives in `decision_trace.rs` (parallel to `RepairAttemptTrace` at `decision_trace.rs:197`) and is emitted from `try_resume_partial_plan` at the resume decision point, carrying the reuse-vs-replan decision, the per-step revalidation verdict, and (on reuse) the seeded skeleton ops. D3 and D4 must land in the same ticket because D4's only emit site lives inside D3's resume integration — splitting them would leave a transient state where the trace struct exists without an emitter, or the emitter exists without a target struct.

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
   - `try_resume_partial_plan` → `search_plan_seeded` (new) → planned plan or fallback.
   - `try_resume_partial_plan` → `PartialPlanResumeTrace` emit (new, decision_trace sink).
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

## Architecture Check

1. **Separate `search_plan_seeded` function preserves the unseeded entry's contract.** The spec mandates this explicitly: conflating the seeded and unseeded paths via an optional parameter on `search_plan_with_trace_metadata_and_source` would complicate tracing and termination semantics for callers that don't pass a skeleton. The new function lives alongside and shares the core search machinery.
2. **No backward-compatibility shim.** The existing `Pending` re-entry is preserved as the lawful fallback, not as a compatibility path — it remains the canonical response when revalidation fails. There's no deprecated path to remove.
3. **D4's trace parallels `RepairAttemptTrace`.** The repair attempt machinery already provides the precedent for "tried a planning shortcut, here's what happened." `PartialPlanResumeTrace` mirrors the structure: outcome + reason + (on success) what was seeded.
4. **Verdict-then-emit-then-search ordering protects the trace from search-time crashes.** Even if `search_plan_seeded` panics or hits an internal error, the trace records the revalidation verdict and the seeded-ops decision. This is FND-29 (debuggability) materialized.
5. **D3 + D4 combined avoids transient dead-code state (FND-28).** Landing D4 first would leave a trace struct with no emitter; landing D3 first would leave an emitter writing into a missing struct. Both together: the workspace compiles, the live authority path is clean, no orphaned types.

## Verification Layers

1. **Verdict-driven routing** → decision-trace assertion: focused unit test on `try_resume_partial_plan` that sets up a populated skeleton, mocks `revalidate_skeleton_step` (or constructs predicates that produce `Reusable`/`Invalid`), and verifies the emitted `PartialPlanResumeTrace` records the chosen route.
2. **Seeded search produces a valid plan or falls back internally** → focused unit test on `search_plan_seeded` with a synthetic skeleton + planning context. Assert: (a) when the skeleton is satisfiable, the returned plan rebuilds tactical detail through normal search; (b) when an op cannot be satisfied, the function falls back internally and still returns a plan (or `None` if even unconstrained search fails).
3. **Trace emit covers both success and failure paths** → decision-trace assertions in focused tests: on `Reusable` → trace carries verdict + seeded ops; on `Invalid` → trace carries verdict + reason; on `None` (no skeleton) → trace carries the "no skeleton to reuse" decision.
4. **`Pending` fallback semantics unchanged on `Invalid`/`None`** → focused test confirms `entry.phase = AgendaPhase::Pending` (line 135) is still set on the fallback paths; behavior equivalent to today's resume.
5. **Patience-limit interaction preserved** → focused test: `Invalid` verdict still increments `resume_attempt_count` (line 128), and the patience-limit check at line 130 still abandons after exhaustion. The seeded path is a strict optimization over the existing bounded-reuse machinery.

Per precision rule 5, each invariant maps to a single proof surface — decision-trace assertions for AI reasoning (precision rule 6), focused unit tests for tactical-search behavior. No action trace, event-log delta, or authoritative world-state assertion is needed at this layer; ticket 004's goldens cover the cross-system observable behavior.

## What to Change

### 1. `PartialPlanResumeTrace` struct (D4)

In `crates/worldwake-ai/src/decision_trace.rs`, after `RepairAttemptTrace` (line 197):

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
}
```

Derives match `RepairAttemptTrace`. Re-export through `lib.rs` if not automatic.

Integrate the trace into the appropriate enclosing trace structure (`AgentDecisionTrace` or whatever holds `RepairAttemptTrace` — verify the actual host structure during implementation).

### 2. `search_plan_seeded` entry point (D3)

In `crates/worldwake-ai/src/agent_tick/planning.rs`, alongside the existing `search_plan_with_trace_metadata_and_source` import (line 25):

- Define `pub(crate) fn search_plan_seeded(skeleton: &[PlannedSkeletonStep], …rest of search context…) -> SearchResult` (signature mirrors the unseeded entry's parameters minus the skeleton, plus the skeleton itself).
- Internal control flow: walk the skeleton's ops in order, using each op as a search-control bias (prefer expansions that match the op's `PlannerOpKind` and `target_template`). If an op cannot be satisfied at its expected expansion point, fall back internally to unconstrained search for that subtree.
- The function should share the core search machinery with the unseeded entry — it differs only in the search-control bias at expansion time.
- On final result, the returned plan goes through the same validation pipeline as the unseeded entry; the seeded path never bypasses precondition checks.

### 3. Resume integration in `try_resume_partial_plan` (D3 + D4)

In `crates/worldwake-ai/src/agenda_manager.rs::try_resume_partial_plan` (lines 86-145):

- After the existing resume-condition checks succeed and the resume-attempt counter is incremented (line 128), check whether `segment.remaining_skeleton` is `Some(_)`.
- If `Some(skeleton)`: iterate `skeleton.iter()` calling `revalidate_skeleton_step(SkeletonRevalidationContext { actor, goal: &segment.goal, step, view })` and collect per-step verdicts. If any verdict is `Invalid(_)`, the decision is `FallbackToReplanInvalid(first_invalid_reason)`. If all are `Reusable`, the decision is `ReusedSeededSearch`.
- If `None`: decision is `FallbackToReplanNoSkeleton`.
- Emit `PartialPlanResumeTrace` via the decision-trace sink with the segment id, decision, per-step verdicts, and seeded ops (when reused). Emit happens *before* `search_plan_seeded` runs (per Architecture Check #4).
- On `ReusedSeededSearch`: call `search_plan_seeded(skeleton, …)` instead of setting `entry.phase = AgendaPhase::Pending`. The returned plan is committed via the existing plan-commit machinery (the resume should yield the same final agenda state as full replan would).
- On `FallbackToReplanInvalid(_)` or `FallbackToReplanNoSkeleton`: preserve the existing `entry.phase = AgendaPhase::Pending` at line 135 unchanged.
- Patience-limit interaction (line 130) and `resume_attempt_count` increment (line 128) remain unchanged — invalid skeleton attempts still count toward patience.

### 4. Focused unit tests

In `crates/worldwake-ai/src/agenda_manager.rs` `#[cfg(test)]`:

1. `try_resume_with_reusable_skeleton_invokes_seeded_search_and_emits_trace` — populated skeleton + all-reusable verdicts → `ReusedSeededSearch` trace; seeded search invoked; plan committed.
2. `try_resume_with_invalid_skeleton_falls_back_to_pending_and_emits_reason` — populated skeleton + one invalid verdict → `FallbackToReplanInvalid(reason)` trace; `entry.phase = AgendaPhase::Pending`.
3. `try_resume_with_no_skeleton_falls_back_to_pending` — `None` skeleton → `FallbackToReplanNoSkeleton` trace; `entry.phase = AgendaPhase::Pending`.
4. `try_resume_invalid_skeleton_increments_resume_attempt_count` — `Invalid` verdict still increments the counter (verifies bounded-reuse machinery preserved).
5. `try_resume_does_not_emit_trace_when_resume_condition_unsatisfied` — extend `try_resume_partial_plan_leaves_suspended_when_resume_condition_is_unsatisfied:1860` to assert no `PartialPlanResumeTrace` emit.
6. `try_resume_emits_trace_on_abandon_path` — extend the abandon-path tests (`:2031`, `:2061`) to assert trace emit when the abandon condition fires.

In `crates/worldwake-ai/src/agent_tick/planning.rs` `#[cfg(test)]`:

7. `search_plan_seeded_satisfies_walkable_skeleton` — synthetic skeleton where all ops are satisfiable → returned plan rebuilds tactical detail through normal search.
8. `search_plan_seeded_falls_back_internally_when_op_unsatisfiable` — synthetic skeleton where one op cannot be expanded as-skeletoned → search internally falls back to unconstrained search for that subtree; still returns a plan.
9. `search_plan_seeded_returns_none_when_unconstrained_search_also_fails` — synthetic skeleton where neither seeded nor unconstrained search succeeds → returns `None`/failure (the strictly-bounded case).

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `PartialPlanResumeTrace` + `PartialPlanResumeDecision`; integrate into enclosing trace structure)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — add `search_plan_seeded` + tests)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — resume integration in `try_resume_partial_plan`; trace emit; tests)
- `crates/worldwake-ai/src/lib.rs` (modify if needed — re-export new trace types)

## Out of Scope

- Validation goldens (info-barrier suspend → satisfy → resume; assumption stale → full replan; negative no-skeleton-as-rail) — ticket 004.
- Budget-exhausted skeleton population — ticket 002.
- Information-barrier segment production — ticket 006.
- `revalidate_skeleton_step` function definition — ticket 001.
- Resource/jurisdiction/coordination barrier skeleton reuse — spec Non-Goals.
- Combat plan / target-identity-bound skeleton preservation — spec Non-Goals (filtered at ticket 002's construction sites).
- Changes to agenda arbitration / ranking authority — spec Non-Goals.

## Acceptance Criteria

### Tests That Must Pass

1. All 9 focused unit tests listed in "What to Change" §4.
2. Existing resume tests pass with extensions: `try_resume_partial_plan_returns_segment_when_resume_condition_holds`, `try_resume_partial_plan_leaves_suspended_when_resume_condition_is_unsatisfied`, `try_resume_partial_plan_abandons_before_resume_when_abandon_condition_holds`, `try_resume_partial_plan_abandons_when_resume_attempt_exceeds_patience_limit`.
3. Existing suite: `cargo test -p worldwake-ai` passes.

### Invariants

1. The verdict from `revalidate_skeleton_step` deterministically routes resume: all-`Reusable` → seeded search; any `Invalid` → `Pending` fallback; `None` skeleton → `Pending` fallback. No silent third route.
2. `PartialPlanResumeTrace` is emitted on every resume-condition-satisfied tick where a partial plan segment is consulted (including fallback paths). Enforced by tests #1-3, #6.
3. The patience-limit / `resume_attempt_count` bounded-reuse machinery is preserved — `Invalid` verdicts still increment the counter, and the abandon path at `agenda_manager.rs:130` still fires after exhaustion. Enforced by test #4.
4. `search_plan_seeded` never returns a plan that bypasses precondition checks; the validation pipeline is shared with the unseeded entry. Enforced by sharing the core search machinery rather than reimplementing.
5. The trace is emitted *before* `search_plan_seeded` runs, so a search-time crash does not lose the trace context. Enforced by test #1 (which asserts the trace exists even when the seeded plan is observed afterward) and code ordering in §3.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agenda_manager.rs` `#[cfg(test)]` — 6 new/extended tests covering resume routing, trace emit, patience-limit interaction (see §4 #1-6).
2. `crates/worldwake-ai/src/agent_tick/planning.rs` `#[cfg(test)]` — 3 new tests covering seeded-search satisfaction, internal fallback, and unconstrained-search failure (see §4 #7-9).

### Commands

1. `cargo test -p worldwake-ai --lib agenda_manager::tests::try_resume` — targeted resume tests.
2. `cargo test -p worldwake-ai --lib agent_tick::planning::tests::search_plan_seeded` — targeted seeded-search tests.
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` — lint.
4. `cargo test -p worldwake-ai` — full ai-crate suite (the seeded path could affect existing decision-tree traces via the new emit; this guards against unintended regressions).
