# E16OFFSUCFAC-009: AI Integration — Planner Ops, Goal Model, and Candidate Generation

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — planner_ops, goal_model, candidate_generation in worldwake-ai
**Deps**: E16OFFSUCFAC-004, E16OFFSUCFAC-006

## Problem

E16 requires AI agents to autonomously pursue office claims and support candidates. This needs three things in `worldwake-ai`:

1. **New `PlannerOpKind` variants** — Bribe, Threaten, DeclareSupport — so the GOAP planner can plan these actions.
2. **`GoalKindTag` expansion** — ClaimOffice, SupportCandidateForOffice — with planner extension trait implementations mapping goals to relevant ops.
3. **Candidate generation** — `emit_political_candidates()` that generates `ClaimOffice` and `SupportCandidateForOffice` goals from agent beliefs (belief-mediated — Principle 10).

## Assumption Reassessment (2026-03-15)

1. `PlannerOpKind` in `crates/worldwake-ai/src/planner_ops.rs` currently has 15 variants — confirmed. Each maps to `PlannerOpSemantics`.
2. `GoalKindTag` in `crates/worldwake-ai/src/goal_model.rs` mirrors `GoalKind` for the AI layer — confirmed. Each has `GoalKindPlannerExt` trait methods.
3. `candidate_generation.rs` has `generate_candidates()` with domain-specific emission functions — confirmed. New `emit_political_candidates()` will follow the same pattern.
4. `GoalKind::ClaimOffice` and `GoalKind::SupportCandidateForOffice` will exist in core (E16OFFSUCFAC-004) — dependency.
5. Bribe, Threaten, DeclareSupport actions will be registered (E16OFFSUCFAC-006) — dependency.
6. Agent beliefs are accessed through `GoalBeliefView` (E14) — confirmed, office vacancy awareness is belief-mediated.
7. `BlockedIntentMemory` exists for tracking temporarily blocked goals — confirmed.
8. `UtilityProfile.enterprise_weight` and `social_weight` exist — confirmed, used for goal motive calculation.
9. `loyal_to` relation weights accessible through beliefs — must verify belief view exposes loyalty.

## Architecture Check

1. Following the exact patterns of existing planner ops (Tell, Trade, Attack) and goal tags (ShareBelief, SellCommodity).
2. Candidate generation uses beliefs only — NEVER reads world state directly (Principle 10).
3. Office vacancy awareness is belief-mediated: agent must believe the office holder is dead/absent.
4. No backward-compatibility shims.
5. Goal priority classes follow spec: ClaimOffice = Medium, SupportCandidateForOffice = Low.

## What to Change

### 1. Add `PlannerOpKind` variants

In `crates/worldwake-ai/src/planner_ops.rs`:

```rust
pub enum PlannerOpKind {
    // ... existing 15 variants ...
    Bribe,
    Threaten,
    DeclareSupport,
}
```

Each needs `PlannerOpSemantics`:

**Bribe**:
- `may_appear_mid_plan: true` (can bribe during a claim plan)
- `is_materialization_barrier: false`
- `transition_kind: PlannerTransitionKind::SocialInfluence`
- `relevant_goal_kinds: &[GoalKindTag::ClaimOffice, GoalKindTag::SupportCandidateForOffice]`

**Threaten**:
- `may_appear_mid_plan: true`
- `is_materialization_barrier: false`
- `transition_kind: PlannerTransitionKind::SocialInfluence`
- `relevant_goal_kinds: &[GoalKindTag::ClaimOffice]`

**DeclareSupport**:
- `may_appear_mid_plan: true`
- `is_materialization_barrier: false`
- `transition_kind: PlannerTransitionKind::Terminal` (end of plan)
- `relevant_goal_kinds: &[GoalKindTag::ClaimOffice, GoalKindTag::SupportCandidateForOffice]`

### 2. Add `GoalKindTag` variants

In `crates/worldwake-ai/src/goal_model.rs`:

```rust
pub enum GoalKindTag {
    // ... existing variants ...
    ClaimOffice,
    SupportCandidateForOffice,
}
```

Implement `GoalKindPlannerExt` for each:

**ClaimOffice**:
- `relevant_op_kinds()`: Travel, Bribe, Threaten, DeclareSupport
- `is_satisfied()`: agent believes they hold the office
- Priority class: Medium (below survival/danger, above social)
- Motive: based on `enterprise_weight` (ambition)

**SupportCandidateForOffice**:
- `relevant_op_kinds()`: Travel, DeclareSupport
- `is_satisfied()`: agent has declared support for the candidate
- Priority class: Low (above idle social, below enterprise)
- Motive: based on `social_weight * loyal_to` strength to candidate

### 3. Add `emit_political_candidates()` in candidate generation

In `crates/worldwake-ai/src/candidate_generation.rs`:

```rust
fn emit_political_candidates(ctx: &GenerationContext, candidates: &mut Vec<GroundedGoal>) {
    // For each office the agent knows about (via beliefs):
    //   If agent believes office is vacant (holder dead/absent):
    //     If agent is eligible (faction membership check via beliefs):
    //       Emit ClaimOffice { office } with motive = enterprise_weight
    //     For each candidate the agent is loyal to (loyal_to weight above threshold):
    //       If candidate is eligible:
    //         Emit SupportCandidateForOffice { office, candidate }
    //           with motive = social_weight * loyal_to_weight
}
```

Key design constraints:
- **Belief-only**: reads from `GoalBeliefView`, never from `World` (Principle 10)
- **Eligibility check**: uses agent's belief about faction membership and office eligibility rules
- **Blocked intent filtering**: skips goals blocked in `BlockedIntentMemory`
- **Zero-motive filter**: respects the system-wide zero-motive filter in `rank_candidates()`

### 4. Wire into `generate_candidates()`

Add `emit_political_candidates(&ctx, &mut candidates)` call in the main `generate_candidates()` function.

### 5. Classify new actions in planner op classification

Update the action-to-planner-op mapping so `bribe`, `threaten`, and `declare_support` actions are classified to their respective `PlannerOpKind` variants.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add 3 variants + semantics)
- `crates/worldwake-ai/src/goal_model.rs` (modify — add 2 GoalKindTag variants + trait impls)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — add `emit_political_candidates()`)

## Out of Scope

- Action handler implementations (E16OFFSUCFAC-006 — already done)
- Succession system (E16OFFSUCFAC-007 — already done)
- Public order function (E16OFFSUCFAC-008)
- Modifying existing planner ops, goal tags, or candidate emission functions
- Full E2E test of AI autonomously claiming offices (deferred to integration testing)
- Belief view expansion for office-specific queries (if needed, create follow-up ticket)

## Acceptance Criteria

### Tests That Must Pass

1. `PlannerOpKind::Bribe`, `Threaten`, `DeclareSupport` variants exist with correct semantics.
2. `GoalKindTag::ClaimOffice` maps to relevant ops: Travel, Bribe, Threaten, DeclareSupport.
3. `GoalKindTag::SupportCandidateForOffice` maps to relevant ops: Travel, DeclareSupport.
4. `emit_political_candidates()` generates `ClaimOffice` when agent believes office is vacant AND is eligible.
5. `emit_political_candidates()` does NOT generate `ClaimOffice` when agent doesn't believe office is vacant.
6. `emit_political_candidates()` does NOT generate `ClaimOffice` when agent is not eligible (not faction member).
7. `emit_political_candidates()` generates `SupportCandidateForOffice` when agent has loyalty above threshold to eligible candidate.
8. `ClaimOffice` motive is based on `enterprise_weight`.
9. `SupportCandidateForOffice` motive is based on `social_weight * loyal_to` strength.
10. Belief-mediated: candidate generation uses `GoalBeliefView`, never `World` (Principle 10).
11. New actions (bribe, threaten, declare_support) are classified to correct `PlannerOpKind` variants.
12. Blocked intent filtering works for political goals.
13. `cargo clippy --workspace --all-targets -- -D warnings`
14. `cargo test --workspace`

### Invariants

1. **Belief-only planning** (Principle 10): candidate generation reads beliefs, never world state.
2. No existing planner ops, goal tags, or candidate emission functions are modified.
3. Determinism: all operations use integer arithmetic and `BTreeMap` iteration.
4. Goal priority ordering: ClaimOffice (Medium) > SupportCandidateForOffice (Low).
5. Zero-motive goals are filtered out by the existing system-wide filter.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — test new variant semantics (mid-plan, barrier, transition, relevant goals).
2. `crates/worldwake-ai/src/goal_model.rs` — test new goal tag trait implementations (relevant ops, satisfaction, priority).
3. `crates/worldwake-ai/src/candidate_generation.rs` — test `emit_political_candidates()` with various belief states (vacant/filled office, eligible/ineligible agent, loyal/disloyal to candidate).

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
