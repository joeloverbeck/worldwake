# S51ARTISS-002: Planner ops and goal dispatch declarations

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new PlannerOpKind variants, classify_action_def mapping, GoalDispatchDeclaration entries
**Deps**: S51ARTISS-001

## Problem

The planner has no operators for bounty/notice posting and no goal dispatch declarations for PostBounty/PostNotice. Without these, candidate generation (ticket 003) would produce goals the planner cannot plan for.

## Assumption Reassessment (2026-04-05)

1. `PlannerOpKind` at `crates/worldwake-ai/src/planner_ops.rs:13-47` derives Copy. Currently ~32 variants. `ClaimBounty` exists. `PostBounty`/`PostNotice` do not.
2. `classify_action_def()` at `planner_ops.rs:82` maps action def names to PlannerOpKind. `post_bounty` and `post_notice` action names exist in the action registry (`artifact_actions.rs`).
3. `GoalDispatchDeclaration` at `crates/worldwake-ai/src/goal_dispatch_decl.rs:42` has fields: `trace_label`, `provenance_family`, `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`.
4. `InvalidationStrategy` at `goal_dispatch_decl.rs:5-26` — may need new variants for posting-goal invalidation (target eliminated, crime resolved).
5. `FeasibilityStrategy` at `goal_dispatch_decl.rs:28-40` — may use existing strategies or need new ones (has coin for reward reserve).
6. `post_bounty` action has `with_payload_override_validator` at `artifact_actions.rs:39`. `post_notice` has one at line 58. Planner-synthesized payloads will be revalidated correctly.
7. Planner semantics for posting: Travel(posting_place) → PostBounty/PostNotice action. Similar to existing patterns like Travel → ConsultRecord.

## Architecture Check

1. PlannerOpKind wraps existing actions — no new action handlers. The planner learns to use `post_bounty` and `post_notice` through the standard op classification pipeline.
2. Goal dispatch uses the declarative registration system (S36) — no special-case planner hooks.
3. Payload override validators already exist on both actions, so planner-synthesized payloads are safely revalidated at action start.
4. No backward-compatibility shims.

## Verification Layers

1. classify_action_def maps `post_bounty` → PostBounty, `post_notice` → PostNotice → focused unit test
2. GoalDispatchDeclaration for PostBounty/PostNotice registered → declaration lookup test
3. Planner can construct Travel → PostBounty plan → focused planner search test
4. Invalidation triggers correctly when target eliminated → focused unit test
5. Cross-layer: planner ops (AI) reference action defs (sim/systems) — verified by classify_action_def mapping.

## What to Change

### 1. Add PlannerOpKind variants

In `crates/worldwake-ai/src/planner_ops.rs`:

Add `PostBounty` and `PostNotice` variants to the `PlannerOpKind` enum.

### 2. Add classify_action_def mappings

In `classify_action_def()` function: map action name `"post_bounty"` → `PlannerOpKind::PostBounty` and `"post_notice"` → `PlannerOpKind::PostNotice`.

### 3. Add planner semantics

Add semantics entries for PostBounty and PostNotice in `semantics_for()`:
- Precondition: actor co-located with posting place
- For PostBounty: actor has coin for reward reserve
- Effect: artifact entity created (hypothetical)
- Duration: 1-2 ticks

### 4. Register GoalDispatchDeclarations

In the goal dispatch registration (wherever existing declarations are registered):

**PostBounty**:
- `trace_label: "post_bounty"`
- `relevant_ops: &[PlannerOpKind::PostBounty, PlannerOpKind::Travel]`
- `invalidation_strategy`: target already eliminated, crime already resolved, or bounty for same target already posted
- `feasibility_strategy`: agent has coin and knows a posting place

**PostNotice**:
- `trace_label: "post_notice"`
- `relevant_ops: &[PlannerOpKind::PostNotice, PlannerOpKind::Travel]`
- `invalidation_strategy`: threat gone (warning no longer relevant), vacancy filled, crime resolved
- `feasibility_strategy`: agent knows a posting place

### 5. Handle plan failure

Ensure `handle_plan_failure` triggers standard replanning for PostBounty/PostNotice — if posting action fails (insufficient funds, not co-located), agent replans. This should work automatically through the existing replan infrastructure if GoalDispatchDeclaration is set up correctly.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify)
- `crates/worldwake-ai/src/search.rs` (modify — if planner semantics live here)

## Out of Scope

- Candidate generation — ticket 003
- CLI display — ticket 004
- Golden tests — ticket 004
- New action handlers (existing post_bounty/post_notice actions are reused)

## Acceptance Criteria

### Tests That Must Pass

1. `classify_action_def` maps `post_bounty` → `PlannerOpKind::PostBounty`
2. `classify_action_def` maps `post_notice` → `PlannerOpKind::PostNotice`
3. GoalDispatchDeclaration lookup for PostBounty/PostNotice returns valid declarations
4. Planner can find Travel → PostBounty plan shape
5. PostBounty invalidated when target is believed eliminated
6. Existing suite: `cargo test --workspace`

### Invariants

1. PlannerOpKind remains Copy
2. Goal dispatch uses declarative registration — no special-case planner code
3. Payload override validators on post_bounty/post_notice actions ensure planner-synthesized payloads are revalidated

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — classify_action_def mapping tests
2. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — Declaration registration and lookup tests
3. `crates/worldwake-ai/src/search.rs` — Focused planner search test: PostBounty plan shape

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
