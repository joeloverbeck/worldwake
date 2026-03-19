# S15STAFAIEME-003: Remote Office Claim Start Failure Loses Gracefully

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: None expected; golden/harness scope only unless a hidden runtime defect is exposed
**Deps**: `specs/S15-start-failure-emergence-golden-suites.md`, S13 political goldens, S08 start-failure architecture

## Problem

Current political/emergent goldens prove claim success, locality, social propagation, and force succession, but they do not prove that a claimant can lawfully lose a political opportunity after planning because another actor reaches and consumes the vacancy first. That leaves the S08 "intent is not entitlement" contract unproven in politics.

## Assumption Reassessment (2026-03-19)

1. Existing political locality and emergence coverage lives in [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs) and [crates/worldwake-ai/tests/golden_offices.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_offices.rs). These tests assert `ClaimOffice` generation, Tell-gated office knowledge, and successful `declare_support`, but none assert `StartFailed` on a political action.
2. Focused candidate-generation coverage already exists for office visibility, vacancy, and eligibility in [crates/worldwake-ai/src/candidate_generation.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/candidate_generation.rs). The missing layer is golden E2E proof for post-selection lawful loss plus next-tick recovery.
3. This ticket targets golden E2E coverage and requires full action registries because it spans social Tell, travel, claim generation, support declaration, authoritative office mutation, and trace inspection.
4. Ordering is explicitly mixed-layer. The compared branches are initially symmetric political opportunities; divergence arises from delayed authoritative occupancy of the office after one claimant's normal path completes first, not from differing motive weights alone.
5. Scenario isolation is required. Remove unrelated lawful office paths such as bribery, threatening, faction eligibility filters, or survival-pressure suppression unless the ticket explicitly needs them. The intended branch is "learn vacancy -> travel/claim -> one installs -> loser gets start failure -> loser stops re-emitting stale claim for occupied office".
6. The assertion surface must distinguish the earlier political action lifecycle from the later office-holder mutation. Per [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md), do not use delayed authoritative installation alone as a proxy for earlier action ordering.
7. Scope correction: if this scenario reveals a real political/runtime defect, update the ticket and split production changes from golden work before implementation.

## Architecture Check

1. A dedicated political-loss golden is cleaner than overloading existing success-path office tests because it proves Principle 19 directly: planned claims do not reserve an office.
2. No backward-compatibility aliasing or office-specific retry suppression should be added. The losing claimant must fall out of the dead claim path because the office is no longer lawfully claimable from current belief/state.

## Verification Layers

1. Both claimants generate the relevant political branch from lawful believed office knowledge -> decision trace.
2. The winner completes the ordinary political claim path first -> action trace for `declare_support` and authoritative office-holder state.
3. The losing claimant's queued political action records `StartFailed` after the office is consumed -> action trace and scheduler start-failure record.
4. The next AI tick consumes the structured failure and removes the stale `ClaimOffice` branch for the occupied office -> decision trace.
5. No repeated stale claim-start loop persists while the office remains occupied -> decision trace history and negative action-trace assertions.

## What to Change

### 1. Add the political-loss golden scenario

Add `golden_remote_office_claim_start_failure_loses_gracefully` and `golden_remote_office_claim_start_failure_loses_gracefully_replays_deterministically` to [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs).

The scenario should:

- use lawful information flow for the vacancy
- send two ambitious claimants toward the same remote office
- let one ordinary claim/install path complete first
- assert `StartFailed` on the loser's political action
- assert next-tick disappearance of the stale claim candidate for the occupied office

### 2. Add harness composition only if necessary

If repeated setup becomes noisy, add a minimal helper in [crates/worldwake-ai/tests/golden_harness/mod.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/mod.rs) that composes existing office/social seeding utilities without bypassing the real political path.

## Files to Touch

- `crates/worldwake-ai/tests/golden_emergent.rs` (modify)
- `crates/worldwake-ai/tests/golden_harness/mod.rs` (modify, only if needed)

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

1. Political planning remains belief-driven and locality-respecting; no claimant may learn or react to vacancy/occupancy without a lawful information path.
2. Claim intent is not entitlement; selecting a `ClaimOffice` plan must not silently reserve the office.
3. The losing claimant must stop pursuing the occupied office until world state changes again; no stale infinite claim-start loop is allowed.
4. Political actions, Tell, travel, and succession remain coupled only through state and the event/action pipelines, never by direct system-to-system calls.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — add the remote-office start-failure golden and deterministic replay companion.
2. `crates/worldwake-ai/tests/golden_harness/mod.rs` — optional narrowly scoped helper reuse if setup duplication becomes material.

### Commands

1. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
2. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully_replays_deterministically -- --exact`
3. `cargo test -p worldwake-ai --test golden_emergent`
