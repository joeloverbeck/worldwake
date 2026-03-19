# S15STAFAIEME-005: Update Golden Scenario Catalog For S15 Start-Failure Emergence

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-001, S15STAFAIEME-002, S15STAFAIEME-003

## Problem

[docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md) is the detailed narrative reference for the golden suite. Without S15 updates, it will continue to describe the new start-failure contract only through care and will not explain how the new production, trade, and politics scenarios differ from the existing success-path goldens they extend.

## Assumption Reassessment (2026-03-19)

1. The current scenario catalog already documents the care-domain S08 case in Scenario 2c and nearby political locality/emergence scenarios such as `golden_tell_propagates_political_knowledge` and `golden_same_place_office_fact_still_requires_tell`, but it has no entries for Scenarios 26-28 from the S15 spec.
2. The scenario catalog is where the suite explains causal distinctness, setup, systems exercised, and cross-system chains. This is the correct place to explain why the new S15 scenarios are not redundant with Scenario 3d, Scenario 2b, or the existing political suites.
3. This is a documentation-only ticket. Its dependency is the implemented test names and behavior from the three scenario tickets, not new focused or runtime coverage.
4. The catalog must follow the testing guidance in [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md): for S08-style cases, it should explicitly state that action traces prove the `StartFailed` lifecycle fact and decision traces prove next-tick AI reconciliation.
5. The descriptions must remain precise about the verification layer split. They should not collapse "failed start", "AI cleared stale plan", and "later world recovery" into one vague scenario-level assertion.
6. Scenario isolation should be documented where relevant: each entry should say which competing lawful affordances were removed so the intended start-failure recovery branch remains reviewable.
7. The catalog also duplicates suite inventory in its header summary. Because [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) was already corrected to the live 124-test inventory, this ticket should reconcile any stale counts in `docs/golden-e2e-scenarios.md` while it is already editing that file rather than preserving a second drifting manual count.
8. Scope correction: if implemented scenario behavior diverges from the spec, update the catalog to match the actual final tests, file placement, and live inventory rather than preserving the earlier assumption.

## Architecture Check

1. Updating the scenario catalog keeps planning and review aligned with the actual causal contract each golden proves.
2. No backwards-looking documentation should imply that success-path trade/politics coverage already proved lawful post-selection loss when it did not.

## Verification Layers

1. Scenario entries name the exact new tests and their owning files -> doc review against implemented golden test names.
2. Each entry distinguishes action-trace proof, decision-trace proof, and authoritative outcome -> doc review against the implemented assertions.
3. "Distinct from existing scenario" rationale is documented for each S15 case -> doc review against Scenario 3d, 2b, and the political suites.
4. Any duplicated suite-count or inventory summary in `docs/golden-e2e-scenarios.md` matches the live `cargo test -p worldwake-ai -- --list` output and the corrected coverage dashboard -> doc review plus inventory check.
5. Additional executable verification layers are not applicable because this ticket only updates catalog prose.

## What to Change

### 1. Add Scenario 26 entry

Document the contested-harvest start-failure chain in [docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md), including its distinction from `golden_resource_exhaustion_race`.

### 2. Add Scenario 27 entry

Document the local-trade opportunity drift chain, its S10-safe simple pricing setup, and its distinction from `golden_buyer_driven_trade_acquisition`.

### 3. Add Scenario 28 entry

Document the remote-office claim loss chain, including lawful office knowledge propagation, `StartFailed` on the losing political action, and why it is distinct from the current political locality and claim-success suites.

### 4. Reconcile duplicated inventory prose

If `docs/golden-e2e-scenarios.md` still states stale suite counts or file totals, update those header-level inventory lines to match the live `cargo test -p worldwake-ai -- --list` output while making the S15 scenario additions.

## Files to Touch

- `docs/golden-e2e-scenarios.md` (modify)

## Out of Scope

- `docs/golden-e2e-coverage.md`
- `docs/golden-e2e-testing.md`
- modifying the tests themselves
- broad reorganizations of scenario numbering outside S15-related additions

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
3. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
4. Inventory check: `cargo test -p worldwake-ai -- --list`

### Invariants

1. The catalog must accurately describe the actual implemented scenarios, test names, and owning files.
2. Each S15 entry must explicitly separate action-trace lifecycle proof, decision-trace AI proof, and authoritative downstream outcome.
3. Each S15 entry must explain why it is not redundant with an existing success-path or contention-path golden.
4. Any suite-count summary inside `docs/golden-e2e-scenarios.md` must stay consistent with the live golden inventory and the coverage dashboard.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai --test golden_production`
3. `cargo test -p worldwake-ai --test golden_emergent`
