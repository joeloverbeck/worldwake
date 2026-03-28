# S34GENEPIACT-005: Planner ops — classify epistemic actions and expose terminal planner operators

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-ai: new planner op kinds, planner semantics classification, goal-model operator surface, payload override wiring, barrier behavior
**Deps**: S34GENEPIACT-003 (verify_belief action def exists), S34GENEPIACT-004 (ask_witness action def exists), [specs/S34-general-epistemic-actions.md](/home/joeloverbeck/projects/worldwake/specs/S34-general-epistemic-actions.md)

## Problem

The GOAP planner still cannot construct deliberate epistemic plans even after ticket 003 added the live `verify_belief` action. The goal family already exists in the planner surface, but it bottoms out in an empty operator set: `GoalKind::VerifyBelief` is tagged and partially modeled, yet `VERIFY_BELIEF_OPS` is empty and `build_semantics_table()` intentionally leaves `verify_belief` unclassified. If ticket 006 starts emitting `VerifyBelief` candidates before this is fixed, the AI will create a lawful goal family with no terminal planner operator path.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the planner-facing epistemic contract across `crates/worldwake-ai/src/planner_ops.rs` and `crates/worldwake-ai/src/goal_model.rs`: action-def classification, `PlannerOpKind`, `PlannerOpSemantics`, `GoalKind::VerifyBelief` operator exposure, payload override synthesis, and progress-barrier behavior.
2. `PlannerOpKind` in `crates/worldwake-ai/src/planner_ops.rs` still lacks `VerifyBelief` and `AskWitness`. `build_semantics_table_classifies_registered_planner_action_defs()` currently allows `["verify_belief"]` as the only intentionally unclassified action. That explicit escape hatch was added in ticket 003 to keep the live registry honest until this ticket lands.
3. `GoalKindTag::VerifyBelief` already exists in `crates/worldwake-ai/src/goal_model.rs`, and `GoalKindPlannerExt` already has partial `VerifyBelief` support for `goal_kind_tag()`, `relevant_observed_commodities()`, `goal_relevant_places()`, `is_satisfied()`, and `matches_binding()`. This ticket must not recreate that delivered architecture.
4. The remaining live gap in `goal_model.rs` is operator exposure, not goal-family creation: `VERIFY_BELIEF_OPS` is currently `&[]`, so `GoalKind::VerifyBelief` has no planner-relevant operators even though the rest of the goal model knows the family exists.
5. `GoalKind::VerifyBelief::is_satisfied()` already uses belief-only reads keyed by `generation_tick`, which aligns with P13/P14/P18 in [docs/FOUNDATIONS.md](/home/joeloverbeck/projects/worldwake/docs/FOUNDATIONS.md). This ticket should preserve that architecture and add terminal ops around it rather than bypassing the goal through authoritative checks.
6. `is_progress_barrier()` currently treats `Tell`, `Investigate`, `Accuse`, punishment, and selected political ops as terminal barriers, but not epistemic verification. `VerifyBelief` and `AskWitness` should become explicit barriers for `GoalKind::VerifyBelief`, because the planner cannot lawfully predict what observation or witness testimony will reveal.
7. `build_payload_override()` already has the right structural hook. It can reuse affordance payloads when present, and the `VerifyBelief` goal already carries canonical `VerificationSubject` identity. The missing work is adding epistemic planner-op variants and the exact payload synthesis paths for those ops, not inventing a new side channel.
8. `matches_binding()` already treats `GoalKind::VerifyBelief` as place-bound through the canonical `VerificationSubject`. This ticket should keep that contract rather than adding action-specific alias binding state.
9. `AskWitness` is not yet implemented in systems (ticket 004), so this ticket must treat `VerifyBelief` as the minimum required closure and add `AskWitness` only when the live action definition exists. The architecture should support both terminal ops, but the verification surface must stay honest if only one is live during implementation.
10. Mismatch + correction: the original ticket claimed it needed to add `GoalKindTag::VerifyBelief` and broad `GoalKindPlannerExt` support. Those already exist. The actual remaining work is classifying epistemic action defs into planner ops and filling the currently empty operator/payload/barrier surface for the existing goal family.

## Architecture Check

1. The clean architecture is to make epistemic actions first-class planner ops instead of leaving them as registered runtime actions with planner-side special cases or test-only exemptions. That restores a single canonical mapping from live action definitions to planner semantics.
2. `VerifyBelief` and `AskWitness` should mirror `Investigate`: terminal, belief-driven barriers with `GoalModelFallback` transitions. This keeps deliberate information-seeking lawful under P7, P8, P13, P14, and P18 rather than letting the planner peek through unknown outcomes.
3. No backwards-compatibility shims, no alias planner op, and no permanent “unclassified action” exception for epistemic handlers.

## Verification Layers

1. Registered epistemic action defs classify into planner semantics instead of remaining in the unclassified escape hatch -> focused `planner_ops` unit tests
2. `GoalKind::VerifyBelief` exposes the correct terminal op set and barrier contract -> focused `goal_model` unit tests
3. Planner constructs `Travel -> VerifyBelief` for remote verification -> focused planner/search test
4. Planner constructs `AskWitness` for co-located verification only when the live `ask_witness` def exists -> focused planner/search test
5. `VerifyBelief` satisfaction remains belief-only and generation-tick based -> focused `goal_model` unit tests
6. Single-layer planner contract ticket. Downstream authoritative mutation belongs to tickets 003/004; this ticket proves planner closure, not runtime handler semantics.

## What to Change

### 1. Classify live epistemic action defs into planner ops

In `crates/worldwake-ai/src/planner_ops.rs`, add:
```rust
VerifyBelief,
AskWitness,
```

Add `PlannerOpSemantics` entries for both live epistemic terminals:
- `VerifyBelief`: `may_appear_mid_plan = false`, `is_materialization_barrier = false`, `transition_kind = GoalModelFallback`, `relevant_goal_kinds = &[GoalKindTag::VerifyBelief]`. `is_progress_barrier` returns true for `VerifyBelief` goals.
- `AskWitness`: Same properties. Terminal for `VerifyBelief` goals when a co-located witness is available.

Wire the new ops to their corresponding action defs in the action-def-to-op mapping and remove the temporary “`verify_belief` may stay unclassified” exception from the planner semantics inventory test once the mapping is live.

### 2. Fill the existing `VerifyBelief` goal family operator surface

In `crates/worldwake-ai/src/goal_model.rs`:

- change `VERIFY_BELIEF_OPS` from `&[]` to the real epistemic planner surface
- keep `Travel` as the place-reaching prerequisite op
- include `VerifyBelief` as the canonical terminal op
- include `AskWitness` only when the corresponding live action def and planner classification are present in-scope

### 3. Finish the remaining `GoalKindPlannerExt` work for epistemic terminals

In `crates/worldwake-ai/src/goal_model.rs`:

- keep the existing `GoalKindTag`, `is_satisfied()`, and `matches_binding()` behavior
- extend `build_payload_override()` so `PlannerOpKind::VerifyBelief` synthesizes `VerifyBeliefPayload { subject }` from the goal’s canonical `VerificationSubject`
- synthesize `AskWitnessPayload` from the same subject only if ticket 004 has made the action live and the current goal state contains enough lawful witness-topic identity to do so without planner-side guesswork
- extend `is_progress_barrier()` so terminal epistemic ops are explicit barriers for `GoalKind::VerifyBelief`
- keep `apply_planner_step()` side-effect free for epistemic terminals; the barrier contract, not hypothetical state mutation, is the correct architecture here

### 4. Update planner exhaustiveness and search-facing tests

Update all planner-op match sites and tests so:

- the semantics table fully classifies `verify_belief`
- `GoalKind::VerifyBelief` no longer reports an empty relevant-op set
- planner search can legally terminate on epistemic actions
- any still-deferred `AskWitness` branch is explicitly documented and tested as deferred rather than silently omitted

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add epistemic op kinds, semantics, action-def classification, tests)
- `crates/worldwake-ai/src/goal_model.rs` (modify — fill `VERIFY_BELIEF_OPS`, payload override synthesis, barrier behavior, tests)
- `crates/worldwake-ai/src/search/` (modify if search tests or exhaustiveness require it)

## Out of Scope

- Candidate generation (`emit_verify_belief_goals`) — ticket 006
- Ranking and motive scoring — ticket 007
- Golden E2E tests — ticket 008
- Any authoritative handler semantics or action tracing — tickets 003/004/009
- New hypothetical epistemic world-state mutation in `PlanningState`; epistemic terminals remain barriers, not simulated observation outcomes

## Acceptance Criteria

### Tests That Must Pass

1. `build_semantics_table()` classifies the live `verify_belief` action instead of relying on an unclassified exception
2. `GoalKind::VerifyBelief` no longer reports an empty relevant-op set
3. Planner constructs `Travel -> VerifyBelief` for a remote `VerifyBelief` goal
4. `VerifyBelief` remains a progress barrier for `GoalKind::VerifyBelief`
5. `VerifyBelief` satisfaction remains generation-tick based (`observed_tick >= generation_tick`)
6. If `ask_witness` is live by implementation time, planner constructs `AskWitness` for a co-located `VerifyBelief` goal and treats it as the same terminal barrier family
7. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Planner only reads belief state, never authoritative world state, when reasoning about `VerifyBelief`
2. Epistemic terminal ops are explicit barriers — planner does not hallucinate post-observation or post-testimony outcomes
3. No permanent action-classification escape hatch remains for live epistemic actions
4. Determinism — no `HashMap`, no floats, no wall-clock time in planner paths

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — classify `verify_belief` into planner semantics and remove the temporary unclassified exception
2. `crates/worldwake-ai/src/goal_model.rs` — prove `VerifyBelief` relevant-op exposure, payload override synthesis, and barrier behavior
3. `crates/worldwake-ai/src/search/` — prove planner search can terminate on epistemic operators rather than leaving `VerifyBelief` unreachable

### Commands

1. `cargo test -p worldwake-ai build_semantics_table_classifies_registered_planner_action_defs`
2. `cargo test -p worldwake-ai`
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`
