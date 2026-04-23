# SURVPREF-001: Surface stale familiar-source failure into preference memory in `survival-preferences`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — AI ranking / survival-scenario preference-memory path
**Deps**: `docs/scenario-roadmap.md` row 7 `survival-preferences`, `scenarios/survival-preferences.ron`, `crates/worldwake-ai/tests/golden_survival_preferences.rs`

## Problem

`survival-preferences.ron` now truthfully proves proactive diversification inside a 1440-tick survival envelope, but it does not yet land the full roadmap row because the experience-preference half never becomes a durable failed familiar-source memory. The tracked scout can lawfully rank or even briefly prefer the empty familiar orchard again, yet the survival run reselects away before `SourceReliability.failed_attempts` records a stale intrinsic failure. That leaves row 7 behaviorally incomplete even though the scenario structurally activates both `preference_profile` and `diversification_profile`.

## Assumption Reassessment (2026-04-23)

1. Focused live coverage already exists for the non-survival preference seam in [`crates/worldwake-ai/tests/golden_experience_preferences.rs`](../crates/worldwake-ai/tests/golden_experience_preferences.rs): Scenarios 91-93 prove route-memory preference behavior, but they are harness-level and not roadmap survival scenarios.
2. Live coverage already exists for proactive diversification in [`crates/worldwake-ai/tests/golden_exploration.rs`](../crates/worldwake-ai/tests/golden_exploration.rs): Scenarios 343-345 prove proactive exploration, vetoing under need pressure, and cooldown spacing outside survival-roadmap ownership.
3. The shared abstraction boundary under audit is the survival-time handoff among `AcquireCommodity(SelfConsume)` ranking, authoritative harvest start failure, and `SourceReliability` persistence for the same believed source.
4. The motivating invariant is: once a survival agent retries an out-of-date familiar source and that source fails intrinsically, the failure must become durable preference state that can lawfully influence later survival acquisition choices.
5. The live `GoalKind` under test is `AcquireCommodity { commodity: Apple, purpose: SelfConsume }`; the relied-on operator surface is travel-to-source plus `harvest:Harvest Apples` start/commit behavior inside the ordinary action registries.
6. This is a golden E2E / runtime `agent_tick` mixed-layer ticket, not a candidate-generation-only issue. Full action registries are required because the missing state update depends on real travel, authoritative start failure, and later ranking.
7. The relevant ordering layer is mixed: ranking can briefly prefer the stale familiar orchard, but the eventual divergence depends on plan continuation / reselection before authoritative harvest start failure records `failed_attempts`.
8. The current preference-memory mechanism is not absent globally; focused tests prove it. The missing substrate is specifically a truthful survival-scenario path that drives stale familiar retry far enough into the authoritative start-failure boundary to persist failure memory.
9. This is a stale-request / start-failure ticket. The first failure boundary to inspect is the authoritative harvest start-failure path in [`crates/worldwake-systems/src/production_actions.rs`](../crates/worldwake-systems/src/production_actions.rs), especially `record_harvest_start_failure()` and `record_failed_source_attempt()`, plus the AI continuity path that can switch away before that boundary matters.
10. Not applicable: no political office claim or closure boundary is involved.
11. Not applicable: no `ControlSource` or queued-input retention behavior is involved.
12. The survival scenario intentionally isolates trade, combat, tell, and office substrate so the missing proof remains about preference memory under survival rather than unrelated lawful branches.
13. The adjacent contradiction is a required consequence of this row, not a separate bug: row 7 explicitly owns "Experience preferences + diversification / curiosity", so a survival scenario that only lands diversification is incomplete.
14. Mismatch + correction: the first implementation draft assumed self-depletion or out-of-sight depletion would naturally surface as `SourceReliability.failed_attempts` in the survival run. Live traces showed the scout proactively discovers and later successfully uses `Novel Grove`, but the familiar empty-source branch reselects away before a durable failure-memory update is recorded.
15. The concrete scenario math currently allows proactive arrival and later novel-grove harvest under survival, but not a stable stale familiar-source failure branch. The ticket must establish the exact cadence / path conditions that make intrinsic familiar-source failure reachable before survival reselection bypasses it.

## Architecture Check

1. The clean fix is to make the stale familiar-source failure path explicit and durable at the real shared boundary, rather than weakening the roadmap row, weakening the golden, or hand-seeding failure memory in tests. That keeps preference learning causal and debuggable.
2. No backwards-compatibility shims or alias paths should be introduced. The fix should strengthen the one real preference-memory path, not add a survival-only side channel.

## Verification Layers

1. Stale familiar-source retry reaches the first real failure boundary -> action trace plus authoritative harvest start-failure path
2. Intrinsic familiar-source failure persists into learned preference state -> authoritative world state (`SourceReliability.failed_attempts`)
3. Later apple-choice divergence is caused by that stored failure memory -> decision trace / ranked-goal summary with the relevant preference effect surfaced
4. Survival loop remains healthy while the failure-memory seam is exercised -> authoritative world state plus survival forensic assertions in `golden_survival_preferences.rs`
5. If traces still prove the downstream choice without enough provenance for the exact memory handoff, add the strongest missing lower-layer proof surface rather than broadening weaker scenario-level assertions
6. Additional layer mapping is required because this is explicitly a mixed ranking / authoritative action / stored-memory contract

## What to Change

### 1. Reassess the live stale familiar-source failure path

Inspect the exact point where the scout can rank or begin preferring the familiar orchard again in `survival-preferences`, then determine why the run leaves that branch before `record_failed_source_attempt()` persists failure memory.

### 2. Make the survival-time preference-memory handoff truthful

Implement the smallest architectural change needed so a lawful stale familiar-source retry records durable failure memory without adding test-only helpers or hidden scenario exceptions. This may require changes in action start-failure handling, plan continuation / reselection, or the scenario envelope itself, but the canonical path must stay ordinary gameplay logic.

### 3. Finish row-7 proof and roadmap truth

Upgrade `golden_survival_preferences.rs` so it can prove the full row-7 chain: proactive diversification remains live under survival, a stale familiar source fails intrinsically, `SourceReliability` records that failure, and later apple selection changes for that causal reason.

## Files to Touch

- `crates/worldwake-ai/tests/golden_survival_preferences.rs` (modify)
- `scenarios/survival-preferences.ron` (modify)
- `crates/worldwake-systems/src/production_actions.rs` (modify, if the authoritative failure boundary needs repair)
- `crates/worldwake-ai/src/ranking.rs` (modify, if live preference-effect surfacing needs repair)
- `docs/scenario-roadmap.md` (modify)

## Out of Scope

- Landing trade, substitute preferences, or any roadmap row beyond row 7
- Reworking the non-survival auxiliary preference goldens unless the truthful shared fix requires it
- Adding manual belief seeding or test-only preference-memory shortcuts to the survival scenario

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_preferences` proves a stale familiar-source failure records durable preference memory before the later divergent apple choice
2. The same golden still proves proactive diversification survives inside the 1440-tick survival envelope
3. Existing suite: `cargo test -p worldwake-ai --test golden_survival_preferences -- --ignored --test-threads=1`

### Invariants

1. Preference learning must still enter through ordinary authoritative success / intrinsic failure boundaries, not through hidden survival-only mutation
2. The later apple-choice divergence must be explainable as stored preference state plus live beliefs, not as an untracked planner whim

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_survival_preferences.rs` — extend the survival scenario golden to prove the missing stale familiar-source failure-memory chain
2. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_preferences -- --ignored --test-threads=1`
2. `cargo test -p worldwake-ai --test golden_experience_preferences`
3. `cargo test -p worldwake-ai --test golden_exploration`

## Outcome

- Completion date: 2026-04-23
- What actually changed: `survival-preferences.ron` and `golden_survival_preferences.rs` now prove the full row-7 chain inside the 1440-tick survival envelope: proactive discovery of `Novel Grove`, durable familiar-orchard failure memory in `SourceReliability.failed_attempts`, and later apple selection that keeps the familiar orchard as a discounted candidate while selecting the novel grove. The live landing also threaded the preference-memory repair through the AI retained-plan / observation / ranking path named in the follow-up `S124OPEXFAL-001` ticket.
- Deviations from original plan: the truthful persisted-failure seam is broader than the original draft's harvest-start-only framing. The live implementation records the missing survival-time failure memory through the retained-plan / local-contradiction path documented by `S124OPEXFAL-001`, while preserving the same user-visible roadmap invariant.
- Verification results: `cargo test -p worldwake-ai --test golden_survival_preferences -- --ignored --test-threads=1` passed on 2026-04-23 in the live repo (`2 passed`).
