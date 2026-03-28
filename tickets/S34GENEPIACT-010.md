# S34GENEPIACT-010: Golden E2E coverage for deliberate epistemic prerequisite chains

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None expected after S34GENEPIACT-012 lands
**Deps**: [tickets/S34GENEPIACT-012.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-012.md), [archive/tickets/completed/S34GENEPIACT-011.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-011.md), [archive/tickets/completed/S34GENEPIACT-009.md](/home/joeloverbeck/projects/worldwake/archive/tickets/completed/S34GENEPIACT-009.md)

## Problem

Once the epistemic barrier contract is corrected to be fact-sensitive, the repo still needs golden E2E coverage proving those canonical chains emerge end-to-end through the full AI pipeline. The goldens must validate the right information path for each fact class rather than forcing explicit `verify_belief` commits where lawful arrival perception is already canonical.

## Assumption Reassessment (2026-03-28)

1. This ticket depends on the one-shot architectural replacement in [S34GENEPIACT-011.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-011.md). It should not be implemented against the current standalone top-level `VerifyBelief` model.
   Correction: after archival, the architectural dependency is now [S34GENEPIACT-012.md](/home/joeloverbeck/projects/worldwake/tickets/S34GENEPIACT-012.md), because the repo still needs one more foundations-aligned cleanup pass before golden obligations are stable.
2. The exact mixed-layer boundary under audit here is:
   - decision-trace proof that an originating goal selected the canonical epistemic prerequisite barrier path
   - action-trace proof of committed `AskWitness` and any surviving `VerifyBelief` cases only where explicit action commit is truly part of the fact's contract
   - authoritative belief / violation aftermath
   - later decision-trace proof of continuation or replan
3. Existing adjacent goldens already cover passive discovery, stale-prerequisite recovery, and downstream sharing, but not the deliberate prerequisite path:
   - [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs)
   - [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs)
   - [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs)
4. In particular, `golden_stale_prerequisite_belief_discovery_replan` in [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_supply_chain.rs) already proves same-goal stale-branch recovery for live `GoalKind::RestockCommodity`. After S34GENEPIACT-011 reassessment, that scenario also exposed the duplicate-path contradiction: same-place arrival perception can lawfully refresh the stale source belief before a `verify_belief` commit. This ticket must not overcorrect by demanding a committed `verify_belief` there if S34GENEPIACT-012 removes that duplicate path.
5. Typed epistemic `ActionTraceDetail` in [`/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/action_trace.rs) remains the canonical action-lifecycle proof surface for committed epistemic identity, but these goldens should use it only where an explicit action commit is itself the contract.
6. Scenario isolation must be explicit:
   - intended branch: originating goal -> canonical epistemic prerequisite barrier -> aftermath -> continuation/replan
   - competing lawful branches to control: unrelated social telling, unrelated self-care, and alternative non-epistemic satisfiers when they are outside the contract under test
   - lawful arrival perception is not a competing branch to suppress when the fact under test is arrival-observable; it is the intended carrier in that case
7. The live focused proof surface for the stale-source planner contract is currently `goal_model::tests::search_restock_goal_returns_travel_then_verify_belief_barrier_for_remote_stale_source`, but this ticket should not assume that exact operator sequence remains canonical after S34GENEPIACT-012.
8. Adjacent contradiction classification:
   - required consequence of this ticket: retarget golden assertions to the corrected fact-sensitive contract
   - separate architecture work: removing duplicate planner/runtime paths for arrival-observable facts belongs to S34GENEPIACT-012, not here

## Architecture Check

1. These goldens should validate the post-012 architecture, not replace lower-layer planner tests.
2. The clean golden contract is fact-sensitive:
   - arrival-observable facts should prove travel, lawful local refresh, and continuation/replan
   - `AskWitness` should prove committed conversation and report-sourced aftermath
   - `VerifyBelief` should only be asserted if a surviving inspection-only fact class still exists after S34GENEPIACT-012
3. Keeping this ticket separate from S34GENEPIACT-012 remains cleaner than mixing architecture correction and end-to-end scenario work into one ticket.

## Verification Layers

1. originating goal selected with the canonical epistemic prerequisite branch -> decision trace
2. committed `ask_witness` identity, and committed `verify_belief` only for any surviving inspection-only fact class -> action trace
3. belief refresh or violation recording -> authoritative belief / violation state
4. continuation or replan after arrival refresh or explicit epistemic outcome -> later decision trace
5. deterministic replay -> replay companion tests

## What to Change

Add new golden scenarios, likely in [`/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs`](file:///home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_social.rs), that prove:

1. stale arrival-observable prerequisite refresh
   - originating goal depends on a stale location belief
   - selected plan inserts the canonical travel-side epistemic barrier for that fact class
   - belief refreshes through lawful local observation on arrival if the fact remains arrival-observable after S34GENEPIACT-012
   - original branch then continues or becomes newly plannable

2. ask-witness prerequisite chain
   - originating goal depends on stale location knowledge
   - a co-located witness lawfully knows the subject's location
   - selected plan inserts `AskWitness`
   - belief updates with `PerceptionSource::Report { from: witness, chain_len: 1 }`
   - original branch then continues from the new knowledge

3. supply depletion contradiction and replan
   - originating goal depends on a stale supply belief
   - selected path follows the canonical fact-sensitive barrier contract after S34GENEPIACT-012
   - contradiction aftermath is recorded through the lawful refresh mechanism for that fact class
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

1. golden for stale arrival-observable prerequisite refresh
2. replay companion for stale arrival-observable prerequisite refresh
3. golden for ask-witness prerequisite chain
4. replay companion for ask-witness prerequisite chain
5. golden for supply depletion contradiction and replan
6. replay companion for supply depletion contradiction and replan
7. `cargo test -p worldwake-ai`

### Invariants

1. Goldens assert the canonical prerequisite path for the fact class under test, not an obsolete generic `VerifyBelief` path.
2. Committed epistemic action identity is proven through action traces only when explicit action commit is part of the contract.
3. Continuation or replan is proven through later decision traces rather than event absence alone.

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_social.rs` — deliberate stale-location prerequisite golden
   Rationale: prove the canonical arrival-sensitive stale-location barrier under a live originating goal.
2. `crates/worldwake-ai/tests/golden_social.rs` — ask-witness prerequisite golden
   Rationale: prove the local social knowledge-acquisition path under the same contract.
3. `crates/worldwake-ai/tests/golden_supply_chain.rs` or another fitting `golden_*.rs` suite — supply depletion contradiction/replan golden
   Rationale: prove contradiction aftermath and later replanning under the corrected fact-sensitive epistemic contract.

### Commands

1. `cargo test -p worldwake-ai golden_stale_prerequisite_belief_discovery_replan -- --exact`
2. `cargo test -p worldwake-ai goal_model::tests::search_restock_goal_returns_ask_witness_barrier_for_matching_colocated_payload -- --exact`
3. `cargo test -p worldwake-ai`
