# E19GUAPAT-009: Expose patrol-route provenance in AI decision traces

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` decision trace/frame snapshot surfaces and patrol-focused tests
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
2. Existing decision traces already expose ranked candidates, selected goal, search provenance, and selected-plan summaries through [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) and the `agent_tick` tracing pipeline. They do not currently expose patrol-route snapshot provenance such as current waypoint index or the actor’s patrol-route snapshot used by search.
3. Existing frame/runtime snapshots in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs) emphasize travel commitments and frame state, not patrol-route provenance.
4. The exact shared abstraction boundary under audit is: authoritative patrol state in `worldwake-core` / `worldwake-systems`, copied into `PlanningSnapshot` / `PlanningState`, then summarized back out through `worldwake-ai` decision traces and frame snapshots.
5. The motivating invariant is not “did patrol happen?” but “can a debugger prove which concrete patrol-route facts the planner used when selecting the patrol branch?” This is a stronger provenance requirement than existing patrol outcome coverage.
6. E19GUAPAT-007 already added goldens and fixed the underlying runtime/action bugs. This ticket is not allowed to reopen that behavior surface with new shims. It should improve observability of the now-correct architecture.
7. The missing provenance was specifically painful when `PatrolRoute.current_index` and selected patrol opportunity anchor diverged or were suspected to diverge. Any solution that only repeats selected `GoalKind::Patrol { place }` without exposing the underlying route snapshot is insufficient.
8. This is an `agent_tick` / traceability ticket, not a candidate-generation bug ticket. Full action registries and mixed-layer tracing remain required because the point is the live planner/runtime handoff.
9. Existing proof surfaces:
   - patrol mixed-layer behavior: [`crates/worldwake-ai/tests/golden_patrol.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_patrol.rs)
   - patrol action legality/lifecycle: [`crates/worldwake-systems/src/patrol_actions.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/patrol_actions.rs)
   - runtime invalidation: [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs)
10. The adjacent contradiction exposed during E19 implementation was not just missing trace fields. Patrol-route state was missing from planner/runtime surfaces entirely. That production issue is now fixed; this follow-up is strictly about making the planner’s patrol provenance legible enough to debug without source diving.
11. This aligns with `docs/FOUNDATIONS.md` Principle 1, Principle 3, and Principle 20: surprising behavior must remain legible, state must be concrete, and AI decisions must be explainable as “Agent X chose Y because they believed Z and cared about Q.”

## Architecture Check

1. The clean solution is to expose patrol-route provenance directly in the existing AI trace/frame snapshot structures, sourced from the same concrete planning snapshot used by planning. That preserves one canonical truth path instead of inventing separate debug-only recomputation.
2. This is cleaner than adding ad-hoc `eprintln!` instrumentation, one-off patrol debug helpers, or weaker downstream assertions in goldens. Those would violate the project’s explainability goals and rot quickly.
3. The trace should report concrete patrol facts, not a derived “patrol state score.” For this subsystem, that means route membership, current index, current waypoint, and the selected patrol opportunity anchor.
4. No backwards-compatibility aliasing or parallel debug path. The trace data must come from the same planner/runtime state the AI actually used.

## Verification Layers

1. Planner-side patrol provenance is captured from the live planning snapshot -> focused `agent_tick` trace/frame-snapshot tests
2. Selected patrol branch exposes matching opportunity anchor and waypoint provenance -> decision trace assertions
3. Patrol lifecycle still matches the traced provenance in an end-to-end scenario -> patrol golden with decision trace plus action trace
4. Strongest lower-layer proof surface remains `PlanningSnapshot` / `PlanningState` tests if trace summaries still omit enough provenance for diagnosis
5. This is a mixed-layer observability ticket, so a generic “trace exists” assertion is not enough; the fields must be checked against authoritative patrol-route setup

## What to Change

### 1. Extend AI trace/frame snapshot surfaces with patrol-route provenance

Add explicit patrol-route provenance fields to the decision-trace and/or frame-snapshot structures used for debugging. The exact shape can be finalized during implementation, but it should minimally expose:

- whether the actor had a patrol route in the planning snapshot,
- `assigned_places`,
- `current_index`,
- current waypoint at selection time,
- selected patrol opportunity anchor when the chosen goal is `GoalKind::Patrol`.

### 2. Thread the provenance from the real planning snapshot

Populate the new fields from the existing planning/runtime path rather than from a separate recomputation helper. The trace must describe the snapshot actually used by planning on that tick.

### 3. Add focused tests for traceability contract

Add `agent_tick`-level tests that set up patrol state and assert the trace/frame snapshot includes the correct current waypoint and selected patrol anchor after route changes and replans.

### 4. Strengthen patrol golden proof where useful

If the focused traceability tests prove the fields cleanly, add only minimal golden assertions needed to show the new provenance is available during a real patrol scenario. Do not bloat the golden with redundant internal-state assertions.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify)
- `crates/worldwake-ai/src/decision_runtime.rs` (modify)
- `crates/worldwake-ai/src/agent_tick/mod.rs` or adjacent trace plumbing (modify)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify)
- `crates/worldwake-ai/tests/golden_patrol.rs` (modify, if needed)

## Out of Scope

- Changing patrol motive arithmetic
- Changing patrol action legality
- Changing `PatrolRoute` semantics
- Adding debug-only shadow recomputation paths
- Reopening the public-order/thief-deterrence architecture

## Acceptance Criteria

### Tests That Must Pass

1. New focused `agent_tick` test(s) prove patrol-route provenance is present and correct in the trace/frame snapshot
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

1. `crates/worldwake-ai/src/agent_tick/tests.rs` — prove patrol-route provenance appears in the trace/frame snapshot and matches the configured route state
2. `crates/worldwake-ai/tests/golden_patrol.rs` — prove the real patrol scenario exposes the new provenance surface where needed

### Commands

1. `cargo test -p worldwake-ai golden_patrol -- --nocapture`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
