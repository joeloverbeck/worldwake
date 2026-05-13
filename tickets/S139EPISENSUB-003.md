# S139EPISENSUB-003: AskWitness dispatch policy refinement and EPISTEMIC_SENSING_POLICY

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` (`goal_dispatch_decl.rs`, `goal_policy.rs`)
**Deps**: archive/tickets/S139EPISENSUB-001.md

## Problem

Ticket 001 landed the `GoalDispatchKey::AskWitness` variant and, because the live `GoalKindPlannerExt` metadata delegates through declarations, also landed the minimal `DECL_ASK_WITNESS` entry required for compilation. This ticket now owns the remaining policy refinement: add a dedicated `EPISTEMIC_SENSING_POLICY` `GoalFamilyPolicy` constant that suppresses emission under critical-survival stress, switch the existing AskWitness declaration from the temporary share-belief testimony policy to that dedicated policy, and prove the declaration still matches the live goal model.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `GoalDispatchDeclaration` is a struct at `crates/worldwake-ai/src/goal_dispatch_decl.rs:61-74` with fields `trace_label`, `provenance_family`, `relevant_ops`, `invalidation_strategy`, `feasibility_strategy`, `frontier_exhaustion_strategy`, `family_policy`, `progress_barrier_ops`. Existing declarations like `DECL_ENGAGE_HOSTILE` and `DECL_SHARE_BELIEF` at lines 358+ are the structural analogs.
2. `GoalFamilyPolicy` is a struct (not an enum) at `crates/worldwake-ai/src/goal_policy.rs:71-75` with fields `suppression: SuppressionRule`, `penalty_interrupt: PenaltyInterruptEligibility`, `free_interrupt: FreeInterruptRole`. `SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass)` exists at line 35+. `GoalPriorityClass::Critical` is the actual top variant (not `CriticalSurvival` — the spec's earlier draft used a non-existent name, corrected in reassessment).
3. Shared abstraction boundary under audit: `GoalDispatchDeclaration` policy metadata. Ticket 001 already preserved declaration completeness for `GoalDispatchKey::ALL`; this ticket must keep that completeness green while changing only the AskWitness family policy from the temporary existing policy to the dedicated epistemic-sensing policy.
4. Existing inline tests that must continue to pass after this ticket:
   - `goal_dispatch_decl.rs:962 test_declaration_completeness`
   - `goal_dispatch_decl.rs:972 test_declaration_provenance_matches_live_goal_model`
   - `goal_dispatch_decl.rs:984 test_declaration_relevant_ops_match_live_goal_model`
   - `goal_dispatch_decl.rs:1052 test_trace_labels_nonempty_and_distinct_for_payload_splits`
5. Existing inline tests in `goal_policy.rs` that reference the new family after addition:
   - `goal_policy.rs:144 suppression_never_for_self_care_goals`
   - `goal_policy.rs:207 suppression_when_stressed_for_corpse_social_political` (the `EpistemicSensing` family belongs in the same suppression-under-stress class; the test may need extension)
   - `goal_policy.rs:278 share_belief_suppression_depends_on_communication_class`

## Architecture Check

1. The dispatch declaration and family policy are static metadata — they encode "what kind of work is this goal" without entangling per-instance logic. The dedicated policy must replace the temporary policy in the existing AskWitness declaration rather than adding a parallel declaration or compatibility path.
2. `EPISTEMIC_SENSING_POLICY` is a new constant, not a modification of existing policies. Existing policy consumers continue to dispatch through the `family_policy` field; the new policy plugs into the existing read path with no consumer-side change.

## Verification Layers

1. `DECL_ASK_WITNESS` remains registered in the declaration table → `test_declaration_completeness` continues to pass after ticket 001's `ALL` bump.
2. The existing declaration's `relevant_ops` and `provenance_family` continue to match the live `GoalKindPlannerExt` implementation (ticket 001) → `test_declaration_provenance_matches_live_goal_model` and `test_declaration_relevant_ops_match_live_goal_model` pass.
3. `EPISTEMIC_SENSING_POLICY.suppression == WhenStressedAtOrAbove(GoalPriorityClass::Critical)` → new focused unit test in `goal_policy.rs` asserting suppression behavior under simulated critical stress.
4. Single-layer ticket: this ticket is policy/declaration metadata only. No runtime behavior change beyond the metadata that downstream tickets consume.

## What to Change

### 1. Add `EPISTEMIC_SENSING_POLICY` constant

In `crates/worldwake-ai/src/goal_policy.rs` (after `SHARE_BELIEF_TESTIMONY_POLICY` at line 246):

```rust
pub const EPISTEMIC_SENSING_POLICY: GoalFamilyPolicy = GoalFamilyPolicy {
    suppression: SuppressionRule::WhenStressedAtOrAbove(GoalPriorityClass::Critical),
    penalty_interrupt: PenaltyInterruptEligibility::Allowed,  // confirm against SOCIAL_POLICY at line 230
    free_interrupt: FreeInterruptRole::None,                  // confirm against SOCIAL_POLICY
};
```

The exact `penalty_interrupt` and `free_interrupt` values are chosen to match the closest existing analog (`SOCIAL_POLICY`); verify the live values during implementation. The spec's intent is that epistemic-sensing detours can be interrupted by penalty triggers but do not themselves grant interrupt privilege — same shape as social goals.

### 2. Refine the existing `DECL_ASK_WITNESS` declaration

Ticket 001 added the minimal declaration because live metadata lookup required it. In `crates/worldwake-ai/src/goal_dispatch_decl.rs`, keep the existing AskWitness ops, barrier ops, trace label, provenance family, invalidation strategy, feasibility strategy, frontier exhaustion strategy, and representative fixture unless live reassessment proves one is wrong. Change only the `family_policy` from the temporary share-belief testimony policy to `EPISTEMIC_SENSING_POLICY`.

```rust
const ASK_WITNESS_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::AskWitness];
const ASK_WITNESS_BARRIER: &[PlannerOpKind] = &[PlannerOpKind::AskWitness];

const DECL_ASK_WITNESS: GoalDispatchDeclaration = GoalDispatchDeclaration {
    trace_label: "AskWitness",
    provenance_family: Some(RankedGoalProvenanceFamily::EpistemicSensing),
    relevant_ops: ASK_WITNESS_OPS,
    invalidation_strategy: InvalidationStrategy::PositionAndTargetDead,
    feasibility_strategy: FeasibilityStrategy::ColocationOrDead,
    frontier_exhaustion_strategy: FrontierExhaustionStrategy::PermanentUntilInvalidator,
    family_policy: EPISTEMIC_SENSING_POLICY,
    progress_barrier_ops: ASK_WITNESS_BARRIER,
};
```

### 3. Extend dispatch-decl tests

Ticket 001 already extended the declaration completeness/provenance/relevant-op fixtures and `representative_goal_for(GoalDispatchKey::AskWitness)`. Keep those tests green and add or extend a focused assertion that `GoalDispatchKey::AskWitness.declaration().family_policy == EPISTEMIC_SENSING_POLICY`.

### 4. Extend goal_policy tests

Extend `suppression_when_stressed_for_corpse_social_political:207` (or add a new test `suppression_when_stressed_for_epistemic_sensing`) asserting that the new family suppresses under `WhenStressedAtOrAbove(GoalPriorityClass::Critical)`.

## Files to Touch

- `crates/worldwake-ai/src/goal_dispatch_decl.rs` (modify — switch the existing AskWitness declaration to `EPISTEMIC_SENSING_POLICY` + focused policy assertion)
- `crates/worldwake-ai/src/goal_policy.rs` (modify — EPISTEMIC_SENSING_POLICY constant + test extension)

## Out of Scope

- `GoalKind::AskWitness` variant and `GoalDispatchKey::AskWitness` variant — ticket 001 provides these.
- First `DECL_ASK_WITNESS` declaration entry and representative fixture — ticket 001 provides the minimal entry because live metadata lookup requires it.
- `RankedGoalProvenanceFamily::EpistemicSensing` variant addition (if missing) — ticket 001's `GoalKindPlannerExt` implementation owns adding it because the trait method names it first.
- Candidate emitter referencing this declaration — ticket 004.
- Ranking integration — ticket 005.

## Acceptance Criteria

### Tests That Must Pass

1. `test_declaration_completeness:962` continues to pass with `AskWitness` covered.
2. `test_declaration_provenance_matches_live_goal_model:972` continues to pass (asserts `DECL_ASK_WITNESS.provenance_family` matches `GoalKindPlannerExt::ranked_goal_provenance_family` for the new variant).
3. `test_declaration_relevant_ops_match_live_goal_model:984` continues to pass (asserts `DECL_ASK_WITNESS.relevant_ops` matches `GoalKindPlannerExt::relevant_op_kinds` for the new variant).
4. New focused unit test in `goal_policy.rs` asserting `EPISTEMIC_SENSING_POLICY.suppression` suppresses emission at `GoalPriorityClass::Critical` and above.
5. Existing suite: `cargo test -p worldwake-ai` passes.

### Invariants

1. Every `GoalDispatchKey::ALL` variant has exactly one corresponding `GoalDispatchDeclaration` constant — enforced by `test_declaration_completeness`.
2. `DECL_ASK_WITNESS.family_policy` is changed from the temporary ticket-001 policy to `EPISTEMIC_SENSING_POLICY` (single source of truth for the family's suppression rule).
3. `EPISTEMIC_SENSING_POLICY.suppression` is `WhenStressedAtOrAbove(GoalPriorityClass::Critical)` — verified by focused unit test.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — switch the existing AskWitness declaration to `EPISTEMIC_SENSING_POLICY` and add/extend a focused family-policy assertion.
2. `crates/worldwake-ai/src/goal_policy.rs` (extend `#[cfg(test)]` block at line 121) — new focused unit test for epistemic-sensing suppression.

### Commands

1. `cargo test -p worldwake-ai -- goal_dispatch_decl::tests goal_policy::tests` — targeted test run.
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` — confirm the refined declaration compiles cleanly.
3. `./scripts/verify.sh` — full pre-PR gate.
