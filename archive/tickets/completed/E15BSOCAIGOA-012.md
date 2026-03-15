# E15BSOCAIGOA-012: Expose per-agent belief confidence policy to GoalBeliefView and remove ranking default-policy fallback

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — belief-view boundary in sim crate and ShareBelief ranking in ai crate
**Deps**: archive/tickets/completed/E15BSOCAIGOA-005.md, specs/E15b-social-ai-goals.md, specs/E14-perception-beliefs.md

## Problem

`ShareBelief` ranking derives motive from belief provenance and staleness, but it still does so using `BeliefConfidencePolicy::default()` inside `crates/worldwake-ai/src/ranking.rs`. That is an architectural shortcut. `PerceptionProfile.confidence_policy` is concrete per-agent state, so ranking should consume the acting agent's actual policy through the belief-view boundary instead of silently substituting a global default. Otherwise, social motivation can diverge from the same per-agent belief-confidence rules used elsewhere.

## Assumption Reassessment (2026-03-15)

1. `PerceptionProfile` already stores `confidence_policy: BeliefConfidencePolicy` in `crates/worldwake-core/src/belief.rs`.
2. `GoalBeliefView` currently exposes `known_entity_beliefs(agent)` and `tell_profile(agent)` but does not expose `PerceptionProfile` or `BeliefConfidencePolicy`.
3. `PerAgentBeliefView` has authoritative access to the acting agent's components, so it can supply the acting agent's confidence policy without violating belief-only reads for non-self state.
4. `crates/worldwake-ai/src/ranking.rs` currently hardcodes `BeliefConfidencePolicy::default()` for ShareBelief scoring. That is the remaining architectural shortcut.
5. `GoalKind::ShareBelief`, `GoalKindTag::ShareBelief`, `PlannerOpKind::Tell`, and social candidate generation already exist in the codebase. This ticket is not a feature-introduction ticket; it is a boundary-hardening ticket.
6. `crates/worldwake-ai/src/ranking.rs` already has ShareBelief unit coverage, but the current tests only validate behavior under the default confidence policy. They do not prove that per-agent policy variance is respected.
7. No active ticket in `tickets/` currently owns this boundary cleanup. E15BSOCAIGOA-011 is about shared relay-subject selection, not policy exposure.
8. The clean boundary is narrower than exposing the full `PerceptionProfile`: ranking only needs the confidence policy, so the trait should expose the smallest stable contract that solves this problem.

## Architecture Check

1. The right fix is to expose `BeliefConfidencePolicy` through the existing AI belief-view boundary and use that in ranking. This keeps policy derivation at the boundary instead of duplicating defaults inside ranking logic.
2. Do not preserve both paths. Once this lands, ranking should stop falling back to `BeliefConfidencePolicy::default()` for live agent reads. Broken test doubles should be updated rather than masked by a compatibility shim.
3. Because agents are created with a `PerceptionProfile` by invariant, the clean boundary is a required self-authoritative read, not an optional convenience read. The preferred contract is:

```rust
fn belief_confidence_policy(&self, agent: EntityId) -> BeliefConfidencePolicy;
```

`PerAgentBeliefView` should enforce the same self-only rule it uses for other authoritative profile reads. If a required self component is unexpectedly missing, that should fail loudly instead of silently reintroducing a second authority source.
4. Exposing the narrow `BeliefConfidencePolicy` is cleaner than exposing the full `PerceptionProfile` to goal-reading modules. It gives ranking exactly what it needs and no more.
5. This remains principle-compliant: the policy is concrete per-agent state, and ranking remains a pure derived computation over agent-local inputs.

## What to Change

### 1. Extend the AI belief-view boundary with confidence-policy access

Add a narrow accessor on the AI-facing belief-view traits for the acting agent's confidence policy:

```rust
fn belief_confidence_policy(&self, agent: EntityId) -> BeliefConfidencePolicy;
```

Thread it through:

- `crates/worldwake-sim/src/belief_view.rs`
- `impl_goal_belief_view!` forwarding
- `crates/worldwake-sim/src/per_agent_belief_view.rs`

The accessor should only expose the acting agent's own policy, matching the existing self-authoritative pattern used by other profile reads.

### 2. Remove ranking's hardcoded default-policy fallback

Update `crates/worldwake-ai/src/ranking.rs` so ShareBelief scoring uses the policy obtained from the view boundary instead of `BeliefConfidencePolicy::default()`.

After this lands, ranking should not quietly reintroduce a global default for live agent scoring. If the acting agent lacks the required component unexpectedly, fail in the same explicit way other missing required self components are handled. Do not keep both behaviors alive.

### 3. Update test doubles and boundary tests

Any `RuntimeBeliefView` / `GoalBeliefView` test doubles affected by the trait change should be updated to carry an explicit confidence policy. Add focused tests proving:

- the live per-agent view returns the acting agent's stored policy
- ShareBelief ranking changes when the acting agent's policy changes, even with identical belief source and staleness
- non-self reads do not expose another agent's confidence policy through `PerAgentBeliefView`

## Files to Touch

- `crates/worldwake-sim/src/belief_view.rs` (modify)
- `crates/worldwake-sim/src/per_agent_belief_view.rs` (modify)
- `crates/worldwake-sim/src/lib.rs` (modify if re-export surface changes)
- `crates/worldwake-ai/src/ranking.rs` (modify)
- relevant test modules that implement `RuntimeBeliefView` / `GoalBeliefView` (modify)

## Out of Scope

- Social relay-subject deduplication (`E15BSOCAIGOA-011`)
- New social goal kinds such as `InvestigateMismatch`
- Golden social E2E tests
- Any widening of AI goal modules from `GoalBeliefView` to `RuntimeBeliefView`
- Changes to candidate generation semantics

## Acceptance Criteria

### Tests That Must Pass

1. `PerAgentBeliefView` exposes the acting agent's `BeliefConfidencePolicy` through the AI belief-view boundary.
2. ShareBelief ranking uses the policy returned by the view rather than `BeliefConfidencePolicy::default()`.
3. Two otherwise identical agents with different confidence policies produce different ShareBelief motive scores for the same stale rumor/direct-observation input.
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo test -p worldwake-sim`

### Invariants

1. Per-agent confidence policy remains concrete component state; ranking does not store or cache a second authority copy.
2. Ranking does not retain a parallel default-policy path once the belief-view accessor exists.
3. Goal-reading AI modules stay on the `GoalBeliefView` boundary.
4. Deterministic ranking behavior is preserved for identical policy + belief inputs.
5. The new belief-view accessor preserves the self-authoritative boundary and does not widen non-self reads into `PerceptionProfile`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/src/per_agent_belief_view.rs` — verify the view returns the acting agent's stored confidence policy and does not expose other agents' policy through non-self reads.
2. `crates/worldwake-ai/src/ranking.rs` — verify ShareBelief motive changes with different confidence policies for identical source/staleness inputs.
3. Affected `RuntimeBeliefView` / `GoalBeliefView` test doubles — update compile-time/runtime coverage to reflect the new required boundary contract.

### Commands

1. `cargo test -p worldwake-ai ranking -- --nocapture`
2. `cargo test -p worldwake-sim`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-15
- What actually changed:
  - Added a required `belief_confidence_policy(agent) -> BeliefConfidencePolicy` accessor to the AI belief-view boundary and forwarded it through `impl_goal_belief_view!`.
  - Implemented the accessor in `PerAgentBeliefView` as a self-authoritative read backed by `PerceptionProfile.confidence_policy`, with an explicit non-self rejection.
  - Removed ShareBelief ranking's hardcoded `BeliefConfidencePolicy::default()` path and made ranking read the acting agent's policy through `GoalBeliefView`.
  - Threaded the actor confidence policy through `PlanningSnapshot`/`PlanningState` so planner-side ranking and simulation-side ranking use the same concrete source of truth.
  - Updated affected `RuntimeBeliefView` test doubles and stubs to implement the new required contract explicitly.
  - Added focused tests for live boundary exposure, non-self rejection, and per-agent policy variance in ShareBelief scoring.
- Deviations from original plan:
  - The ticket originally treated this as a small sim/ai boundary cleanup. The actual clean implementation also required `PlanningSnapshot`/`PlanningState` updates so planner-internal ranking would not regress to a fallback.
  - `crates/worldwake-sim/src/lib.rs` did not need changes.
  - `crates/worldwake-ai/src/goal_explanation.rs` only needed a test-double contract update, not production behavior changes.
- Verification results:
  - `cargo test -p worldwake-ai`
  - `cargo test -p worldwake-sim`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
