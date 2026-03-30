# E19GUAPAT-009: Expose patrol-route provenance in AI decision traces

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision trace/frame snapshot surfaces, exported patrol provenance types, and patrol-focused tests
**Deps**: [archive/tickets/guard-patrol/E19GUAPAT-007.md](/home/joeloverbeck/projects/worldwake/archive/tickets/guard-patrol/E19GUAPAT-007.md), [archive/specs/E19-guard-patrol.md](/home/joeloverbeck/projects/worldwake/archive/specs/E19-guard-patrol.md), [docs/planner-contracts.md](/home/joeloverbeck/projects/worldwake/docs/planner-contracts.md)

## Problem

The E19 patrol golden work exposed a traceability gap in the AI stack. Decision traces and action traces were strong enough to show:

1. patrol was selected,
2. patrol start later failed or succeeded,
3. route state had advanced authoritatively,

but they did not expose enough planner-side provenance to answer the key architectural question directly:

Which patrol-route waypoint and opportunity anchor did planning actually use for the selected patrol branch on that tick?

That forced manual source inspection across planning snapshot, planning state, runtime dirtiness, and patrol affordance code. This weakens explainable emergence and violates the repo’s intended debugging contract for mixed-layer AI behavior.

## Assumption Reassessment (2026-03-30)

1. The live patrol goal family is `GoalKind::Patrol { place }` in [`crates/worldwake-ai/src/candidate_generation.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) and [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs). The live operator surface is `PlannerOpKind::Patrol` plus prerequisite `Travel`.
2. Existing decision traces already expose ranked candidates, selected goal, selected `OpportunityKey`, search provenance, and selected-plan summaries through [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) and the `agent_tick` tracing pipeline. They do not currently expose patrol-route snapshot provenance such as route membership, `current_index`, or the planner-visible current waypoint that produced the selected patrol opportunity.
3. Existing frame/runtime snapshots in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) and [`crates/worldwake-ai/src/agent_tick/frame.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/frame.rs) emphasize travel commitments and frame state. Runtime dirtiness already tracks `last_patrol_route`, but the debugger-facing snapshot returned by `AgentTickDriver::frame_snapshot()` does not surface patrol-route provenance yet.
4. The exact shared abstraction boundary under audit is: authoritative patrol state in `worldwake-core` / `worldwake-systems`, copied into `PlanningSnapshot` / `PlanningState` in [`crates/worldwake-ai/src/planning_snapshot.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_snapshot.rs) and [`crates/worldwake-ai/src/planning_state.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planning_state.rs), then summarized back out through `worldwake-ai` decision traces and frame snapshots.
5. The motivating invariant is not “did patrol happen?” but “can a debugger prove which concrete patrol-route facts the planner used when selecting the patrol branch?” This is a stronger provenance requirement than existing patrol outcome coverage.
6. E19GUAPAT-007 already added goldens and fixed the underlying runtime/action bugs. This ticket is not allowed to reopen that behavior surface with new shims. It should improve observability of the now-correct architecture.
7. The missing provenance was specifically painful when `PatrolRoute.current_index` and selected patrol opportunity anchor diverged or were suspected to diverge. Any solution that only repeats selected `GoalKind::Patrol { place }` without exposing the underlying route snapshot is insufficient.
8. This is an `agent_tick` / traceability ticket, not a candidate-generation bug ticket. A focused `agent_tick` harness is sufficient for the core provenance contract because the missing surface is built there, while one patrol golden remains useful to prove the new fields are available through the live mixed-layer pipeline.
9. Existing proof surfaces:
   - patrol mixed-layer behavior: [`crates/worldwake-ai/tests/golden_patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs)
   - patrol action legality/lifecycle: [`crates/worldwake-systems/src/patrol_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/patrol_actions.rs)
   - runtime invalidation: [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs)
10. The adjacent contradiction exposed during E19 implementation was not just missing trace fields. Patrol-route state had previously been missing from planner/runtime surfaces, but that part is already fixed: `PlanningSnapshot`, `PlanningState`, runtime dirtiness, and candidate generation all already carry `PatrolRoute`. The remaining gap is debugger-facing provenance, not planner-state completeness.
11. This aligns with `docs/FOUNDATIONS.md` Principle 1, Principle 3, and Principle 20: surprising behavior must remain legible, state must be concrete, and AI decisions must be explainable as “Agent X chose Y because they believed Z and cared about Q.”
12. The canonical end-state path remains single-source: authoritative `PatrolRoute` -> snapshot/state `patrol_route()` -> patrol candidate/selection trace surfaces. This ticket should not add a second patrol-debug transport path or recomputation helper.
13. Mismatch + correction: the original ticket overstated the stale production bug by saying patrol-route state was still missing from planner/runtime surfaces entirely. The corrected scope is narrower and cleaner: expose already-present patrol-route state through the existing decision-trace and frame-debug outputs.

## Architecture Check

1. The clean solution is to expose patrol-route provenance directly in the existing AI trace/frame snapshot structures, sourced from the same concrete planning snapshot used by planning. That preserves one canonical truth path instead of inventing separate debug-only recomputation.
2. This is cleaner than adding ad-hoc `eprintln!` instrumentation, one-off patrol debug helpers, or weaker downstream assertions in goldens. Those would violate the project’s explainability goals and rot quickly.
3. The trace should report concrete patrol facts, not a derived “patrol state score.” For this subsystem, that means route membership, current index, current waypoint, and the selected patrol opportunity anchor.
4. No backwards-compatibility aliasing or parallel debug path. The trace data must come from the same planner/runtime state the AI actually used.

## Verification Layers

1. Planner-side patrol provenance is captured from the live planning snapshot/state boundary -> focused `agent_tick` trace and `frame_snapshot()` tests
2. Selected patrol branch exposes matching opportunity anchor and waypoint provenance -> decision trace assertions
3. Patrol lifecycle still matches the traced provenance in an end-to-end scenario -> patrol golden with decision trace plus action trace
4. Strongest lower-layer proof surface remains `PlanningSnapshot` / `PlanningState` parity if trace summaries still omit enough provenance for diagnosis
5. This is a mixed-layer observability ticket, so a generic “trace exists” assertion is not enough; the new fields must be checked against the configured authoritative patrol-route setup

## What to Change

### 1. Extend AI trace/frame snapshot surfaces with patrol-route provenance

Add explicit patrol-route provenance fields to the decision-trace and frame-snapshot structures used for debugging. The exact shape can be finalized during implementation, but it should minimally expose:

- whether the actor had a patrol route in the planning snapshot,
- `assigned_places`,
- `current_index`,
- current waypoint at selection time,
- selected patrol opportunity anchor when the chosen goal is `GoalKind::Patrol`.

### 2. Thread the provenance from the real planning/runtime snapshot

Populate the new fields from the existing planning/runtime path rather than from a separate recomputation helper. The decision trace must describe the patrol snapshot actually used by planning on that tick, and the frame snapshot should expose the current patrol-route debugger view without adding a parallel patrol-debug subsystem.

### 3. Add focused tests for traceability contract

Add `agent_tick`-level tests that set up patrol state and assert the trace/frame snapshot includes the correct current waypoint and selected patrol anchor after route changes and replans.

### 4. Strengthen patrol golden proof where useful

If the focused traceability tests prove the fields cleanly, add only minimal golden assertions needed to show the new provenance is available during a real patrol scenario. Do not bloat the golden with redundant internal-state assertions.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/src/lib.rs` (modify)
- `crates/worldwake-ai/tests/golden_patrol.rs` (modify)

## Out of Scope

- Changing patrol motive arithmetic
- Changing patrol action legality
- Changing `PatrolRoute` semantics
- Adding debug-only shadow recomputation paths
- Reopening the public-order/thief-deterrence architecture

## Acceptance Criteria

### Tests That Must Pass

1. New focused `agent_tick` test(s) prove patrol-route provenance is present and correct in the decision trace and `frame_snapshot()`
2. Patrol golden coverage can assert the selected patrol branch’s provenance without source diving
3. Existing suite: `cargo test -p worldwake-ai golden_patrol -- --nocapture`
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo test --workspace`
6. Existing suite: `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Patrol trace provenance is sourced from the concrete planning snapshot actually used by the planner
2. Selected patrol opportunity anchor and traced current waypoint cannot silently diverge without the trace making that divergence visible

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — `trace_planning_outcome_includes_patrol_route_provenance`
   Rationale: proves the planner-facing decision trace now exposes the concrete patrol route, current waypoint, and selected patrol anchor from the live agent-tick path.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — `frame_snapshot_reports_patrol_route_provenance`
   Rationale: proves `AgentTickDriver::frame_snapshot()` exposes the debugger-facing patrol route/current waypoint view without needing a second patrol-debug path.
3. `crates/worldwake-ai/tests/golden_patrol.rs` — `golden_patrol_route_adaptation_retargets_after_local_report`
   Rationale: proves the new provenance surface is available in the live patrol scenario after authoritative route adaptation and AI retargeting.

### Commands

1. `cargo test -p worldwake-ai golden_patrol -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-30
- What actually changed:
  - Added `PatrolRouteSnapshotTrace` to the existing AI trace model and surfaced it on `PlanningPipelineTrace`.
  - Added `selected_patrol_anchor` to planning traces so the winning patrol branch exposes the exact selected anchor alongside the route snapshot.
  - Surfaced patrol-route provenance through `FrameDebugSnapshot` by reusing the same live belief/runtime path, not a debug-only recomputation layer.
  - Added focused `agent_tick` tests for decision-trace and frame-snapshot patrol provenance.
  - Strengthened the existing patrol-route-adaptation golden with provenance assertions.
- Deviations from original plan:
  - `crates/worldwake-ai/src/decision_runtime.rs` did not need changes; the cleaner implementation reused the existing runtime/belief path and extended the exported trace/debug surfaces instead.
  - The final implementation added a small public export in `crates/worldwake-ai/src/lib.rs` so the new patrol provenance type remains available through the crate’s existing public surface.
- Verification results:
  - `cargo test -p worldwake-ai golden_patrol -- --nocapture`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
