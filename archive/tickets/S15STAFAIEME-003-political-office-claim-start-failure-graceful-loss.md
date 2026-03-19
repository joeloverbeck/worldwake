# S15STAFAIEME-003: Political Office Claim Start Failure Loses Gracefully

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None
**Deps**: `specs/S15-start-failure-emergence-golden-suites.md`, S13 political goldens, S08 start-failure architecture

## Problem

Current political/emergent goldens prove claim success, locality, social propagation, and force succession, but they did not prove that a claimant can lawfully lose a political opportunity after earlier valid intent because another actor closes the office claim window before authoritative start. That left the S08 "intent is not entitlement" contract unproven in politics.

## Assumption Reassessment (2026-03-19)

1. Existing political locality and emergence coverage does live in [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) and [crates/worldwake-ai/tests/golden_offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs). Verified current guardrails include `golden_tell_propagates_political_knowledge`, `golden_same_place_office_fact_still_requires_tell`, `golden_simple_office_claim_via_declare_support`, and the remote-office success-path suites in `golden_offices.rs`. None asserted `StartFailed` on a political action or next-tick start-failure reconciliation for politics.
2. Focused coverage for the political candidate layer already exists in [candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs) for visible vacancy, support-law filtering, eligibility, and already-declared support omission. The gap was specifically golden E2E coverage for authoritative start failure plus AI reconciliation after a once-lawful `ClaimOffice` path becomes invalid.
3. The actionable political step is not a separate `ClaimOffice` runtime action. Current architecture plans `GoalKind::ClaimOffice { office }` through `Travel -> DeclareSupport` in [goal_model.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs), and authoritative start/commit validation happens in `validate_declare_support_context_in_world` in [office_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/office_actions.rs). The ticket scope therefore targets `declare_support` as the start-failure boundary.
4. The first live rejection boundary is authoritative start, not request-resolution rejection and not post-start abort. I verified the shared runtime path through [tick_step.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-sim/src/tick_step.rs), scheduler `action_start_failures`, and [agent_tick.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs). The golden therefore proves: request bound and start attempted -> authoritative `StartFailed` on `declare_support` -> next-tick AI reconciliation.
5. The original "remote symmetric travel race" assumption was too strong for deterministic coverage in the current support-law architecture. Support-law declarations do not close the claim window by themselves; succession must actually run. The delivered scenario therefore narrows to a deterministic stale-request branch: the delayed claimant lawfully generated `ClaimOffice`, was occupied by self-care, another lawful claimant closed the office through the ordinary support-law path with a short succession period, and the delayed claimant's stale later `declare_support` hit authoritative `StartFailed`.
6. Scenario isolation is required. Remove unrelated lawful office paths such as bribery, threatening, faction eligibility filters, and combat pressure. The delivered branch is: lawful office knowledge -> delayed claimant generates `ClaimOffice` while heal is selected first -> another lawful claimant declares support and support-law succession closes the office -> delayed claimant retries a stale `declare_support` request and hits authoritative `StartFailed` -> next tick no fresh `ClaimOffice` candidate appears while the office remains not visibly vacant.
7. The assertion surface must distinguish the earlier action lifecycle from the later office state change. Per [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md), the proof boundary is still `declare_support` action-trace `StartFailed` plus next-tick decision-trace reconciliation. Durable office-state assertions are used only to show that the political opportunity was lawfully closed before the stale retry.
8. Real-name verification is complete. `cargo test -p worldwake-ai -- --list` confirms the existing S15 guardrails named in this ticket, and also confirms the already-delivered sibling S15 production/trade tests `golden_contested_harvest_start_failure_recovers_via_remote_fallback` and `golden_local_trade_start_failure_recovers_via_production_fallback`.

## Architecture Check

1. A dedicated political-loss golden is cleaner than overloading existing success-path office tests because it proves Principle 19 directly: planned claims do not reserve an office.
2. The clean architecture is still the current one: a generic shared start-failure pipeline with domain-specific authoritative validation at action start. This ticket validates that design in politics rather than replacing it with office-specific retry suppression or aliasing.
3. The delivered test shape is more robust than forcing a brittle pure-AI travel race. It isolates the architectural contract that matters: stale political intent must fail generically at authoritative start and then reconcile generically in AI.
4. No backward-compatibility shims, parallel political action aliases, or office-specific reservation semantics were added. The losing claimant falls out of the dead path because candidate generation stops emitting `ClaimOffice` once the office is no longer visibly vacant and because shared S08 reconciliation clears the stale plan.

## Verification Layers

1. Political candidate generation from lawful office knowledge -> decision trace `goal_history_for(... ClaimOffice ...)`.
2. Delayed claimant is kept off the political action by lawful self-care first -> decision trace plus action trace for `heal`.
3. Another lawful claimant closes the office through ordinary support-law action and succession -> action trace for `declare_support`, then authoritative office state.
4. Losing claimant request reaches authoritative start and fails there after the office is no longer lawfully claimable -> action trace `StartFailed` plus scheduler `action_start_failures`.
5. Next AI tick reconciles the structured failure instead of retaining the stale plan -> decision trace `action_start_failures`, selected-plan source, and candidate history.
6. Closed office stays non-claimable until world state changes again -> decision-trace absence / political omission evidence, with negative action-trace assertions for repeated stale `declare_support` starts.

## What to Change

### 1. Add the political-loss golden scenario

Add `golden_remote_office_claim_start_failure_loses_gracefully` and `golden_remote_office_claim_start_failure_loses_gracefully_replays_deterministically` to [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs).

The test names stay as-is for continuity with the S15 ticket lineage, but the delivered scenario is the corrected deterministic political stale-request variant described above.

The scenario should:

- use lawful information flow for the vacancy
- prove the delayed claimant generated `ClaimOffice` before the office closed
- close the office through the ordinary support-law path before the stale retry
- assert `StartFailed` on the delayed claimant's `declare_support` action rather than on an abstract `ClaimOffice` action
- assert next-tick disappearance of the stale claim candidate for the closed office

### 2. Add harness composition only if necessary

No new harness helper should be added unless the setup genuinely becomes reusable across multiple goldens.

## Files Touched

- `crates/worldwake-ai/tests/golden_emergent.rs`

## Out of Scope

- `crates/worldwake-ai/tests/golden_offices.rs`
- `crates/worldwake-ai/tests/golden_trade.rs`
- `crates/worldwake-ai/tests/golden_production.rs`
- new office laws, new claim mechanics, or succession redesign
- introducing special reservation semantics for political opportunities

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
2. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully_replays_deterministically -- --exact`
3. Existing guardrail: `cargo test -p worldwake-ai golden_tell_propagates_political_knowledge -- --exact`
4. Existing guardrail: `cargo test -p worldwake-ai golden_same_place_office_fact_still_requires_tell -- --exact`
5. Owning binary: `cargo test -p worldwake-ai --test golden_emergent`

### Invariants

1. Political planning remains belief-driven and locality-respecting; no claimant may learn or react to vacancy/closure without a lawful information path.
2. Claim intent is not entitlement; selecting or previously selecting a `ClaimOffice` path must not silently reserve the office.
3. The losing claimant must stop pursuing the closed office until world state changes again; no stale infinite claim-start loop is allowed.
4. Political actions, care, and succession remain coupled only through state and the event/action pipelines, never by direct system-to-system calls.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — `golden_remote_office_claim_start_failure_loses_gracefully`.
Rationale: proves the real political S08 boundary end-to-end: lawful office knowledge, stale late `declare_support` start failure after the office closes, and next-tick AI recovery without repeated stale retries.
2. `crates/worldwake-ai/tests/golden_emergent.rs` — `golden_remote_office_claim_start_failure_loses_gracefully_replays_deterministically`.
Rationale: locks the mixed-layer political loss chain to deterministic replay, matching the golden convention already used by the sibling S15 production/trade suites.

### Commands

1. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
2. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai golden_tell_propagates_political_knowledge -- --exact`
4. `cargo test -p worldwake-ai golden_same_place_office_fact_still_requires_tell -- --exact`
5. `cargo test -p worldwake-ai --test golden_emergent`
6. `cargo test -p worldwake-ai`
7. `cargo test --workspace`
8. `cargo clippy --workspace`

## Outcome

Completed: 2026-03-19

What actually changed:
- Added `golden_remote_office_claim_start_failure_loses_gracefully` and its deterministic replay companion to [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs).
- The new golden proves a stale political `declare_support` request can hit authoritative `StartFailed`, and that the next AI tick consumes the shared S08 failure record and drops the stale `ClaimOffice` branch.

Deviations from original plan:
- The original ticket assumed a deterministic remote AI-vs-AI travel race ending in "winner installs, loser arrives late." Reassessment against current support-law timing showed that assumption was too strong for stable coverage.
- The delivered scenario uses a narrower, architecture-faithful stale-request setup: the delayed claimant first lawfully generates `ClaimOffice` but is occupied with heal; another lawful claimant closes the office via the normal support-law path with a short succession period; the delayed claimant's later stale `declare_support` fails at authoritative start; the next AI tick reconciles the failure.
- No production or harness changes were required.

Verification results:
- `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
- `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully_replays_deterministically -- --exact`
- `cargo test -p worldwake-ai golden_tell_propagates_political_knowledge -- --exact`
- `cargo test -p worldwake-ai golden_same_place_office_fact_still_requires_tell -- --exact`
- `cargo test -p worldwake-ai --test golden_emergent`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace`
