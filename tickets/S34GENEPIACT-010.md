# S34GENEPIACT-010: Golden E2E coverage for deliberate epistemic prerequisite chains

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None expected after S34GENEPIACT-011 lands
**Deps**: S34GENEPIACT-011, S34GENEPIACT-009

## Problem

Once deliberate verification is modeled as canonical prerequisite/progress-barrier work inside the originating goal path, the repo still needs golden E2E coverage proving those chains emerge end-to-end through the full AI pipeline.

## Assumption Reassessment (2026-03-28)

1. This ticket depends on the one-shot architectural replacement in [S34GENEPIACT-011.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-011.md). It should not be implemented against the current standalone top-level `VerifyBelief` model.
2. The exact mixed-layer boundary under audit here is:
   - decision-trace proof that an originating goal selected an epistemic prerequisite barrier path
   - action-trace proof of committed `AskWitness` / `VerifyBelief`
   - authoritative belief / violation aftermath
   - later decision-trace proof of continuation or replan
3. Existing adjacent goldens already cover passive discovery, stale-prerequisite recovery, and downstream sharing, but not the deliberate prerequisite path:
   - [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs)
   - [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)
   - [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs)
4. In particular, `golden_stale_prerequisite_belief_discovery_replan` in [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) already proves same-goal stale-branch recovery for live `GoalKind::RestockCommodity`, but it proves passive discovery after a spent branch, not deliberate epistemic verification as the selected prerequisite barrier.
5. Typed epistemic `ActionTraceDetail` in [`/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs) is now the canonical action-lifecycle proof surface for committed epistemic identity. These goldens should rely on it directly.
6. Scenario isolation must be explicit:
   - intended branch: originating goal -> epistemic prerequisite barrier -> aftermath -> continuation/replan
   - competing lawful branches to control: passive same-place perception, unrelated social telling, unrelated self-care, and alternative non-epistemic satisfiers when they are outside the contract under test

## Architecture Check

1. These goldens should validate the post-011 architecture, not replace lower-layer planner tests.
2. Keeping them in a follow-up ticket is cleaner than mixing architecture correction and end-to-end scenario work into one ticket. It avoids weakening assertions during a moving-contract refactor.

## Verification Layers

1. originating goal selected with epistemic prerequisite branch -> decision trace
2. committed `ask_witness` / `verify_belief` identity -> action trace
3. belief refresh or violation recording -> authoritative belief / violation state
4. continuation or replan after epistemic outcome -> later decision trace
5. deterministic replay -> replay companion tests

## What to Change

Add new golden scenarios, likely in [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs), that prove:

1. stale entity-location prerequisite refresh
   - originating goal depends on a stale location belief
   - selected plan inserts `Travel -> VerifyBelief`
   - belief refreshes through direct observation
   - original branch then continues or becomes newly plannable

2. ask-witness prerequisite chain
   - originating goal depends on stale location knowledge
   - a co-located witness lawfully knows the subject's location
   - selected plan inserts `AskWitness`
   - belief updates with `PerceptionSource::Report { from: witness, chain_len: 1 }`
   - original branch then continues from the new knowledge

3. supply depletion verification and replan
   - originating goal depends on a stale supply belief
   - selected plan inserts `Travel -> VerifyBelief`
   - action records `SupplyDepleted` aftermath
   - later planning abandons the stale branch and chooses a lawful alternative or investigation path, depending on live architecture

Each should get a deterministic replay companion.

## Files to Touch

- [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs) or another fitting `golden_*.rs` suite if the final architecture suggests a better home

## Out of Scope

- changing the planner/goal-model contract itself
- weakening assertions to compensate for pre-008 behavior
- revisiting unrelated existing goldens

## Acceptance Criteria

### Tests That Must Pass

1. golden for stale entity-location prerequisite refresh
2. replay companion for stale entity-location prerequisite refresh
3. golden for ask-witness prerequisite chain
4. replay companion for ask-witness prerequisite chain
5. golden for supply depletion verification and replan
6. replay companion for supply depletion verification and replan
7. `cargo test -p worldwake-ai`

### Invariants

1. Goldens assert the deliberate prerequisite path, not merely passive discovery.
2. Committed epistemic action identity is proven through action traces, not inferred only from downstream state.
3. Continuation or replan is proven through later decision traces rather than event absence alone.

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — deliberate stale-location prerequisite golden
   Rationale: prove remote verification as a selected prerequisite barrier under a live originating goal.
2. `crates/worldwake-ai/tests/golden_social.rs` — ask-witness prerequisite golden
   Rationale: prove the local social knowledge-acquisition path under the same contract.
3. `crates/worldwake-ai/tests/golden_social.rs` — supply depletion prerequisite/replan golden
   Rationale: prove contradiction aftermath and later replanning, not only belief refresh.

### Commands

1. `cargo test -p worldwake-ai`
