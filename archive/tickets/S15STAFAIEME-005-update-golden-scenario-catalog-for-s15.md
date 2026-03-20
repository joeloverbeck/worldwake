# S15STAFAIEME-005: Update Golden Scenario Catalog For S15 Start-Failure Emergence

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-001, S15STAFAIEME-002, S15STAFAIEME-003

## Problem

[docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md) is the detailed narrative reference for the golden suite. Without S15 updates, it will continue to describe the new start-failure contract only through care and will not explain how the new production, trade, and politics scenarios differ from the existing success-path goldens they extend.

## Assumption Reassessment (2026-03-20)

1. The current scenario catalog already documents the care-domain S08 case in Scenario 2c and nearby political locality/emergence scenarios such as `golden_tell_propagates_political_knowledge` and `golden_same_place_office_fact_still_requires_tell`, but it still has no Scenario 26-28 entries. Verified in [docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md).
2. The three S15 scenario tests already exist and are the live source of truth for this ticket: `golden_contested_harvest_start_failure_recovers_via_remote_fallback` in [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs), `golden_local_trade_start_failure_recovers_via_production_fallback` in [crates/worldwake-ai/tests/golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs), and `golden_remote_office_claim_start_failure_loses_gracefully` in [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs). This ticket does not introduce those tests; it reconciles catalog prose to the already-implemented behavior.
3. The live tests prove different verification surfaces, and the catalog needs to name that split precisely rather than rephrasing all three as generic "failure recovery". The production and trade scenarios assert authoritative downstream recovery plus action-trace-visible `StartFailed`, while the political scenario asserts the no-retry contract through `declare_support` `StartFailed` counts in the action trace. The ticket should describe those exact contracts instead of copying the spec's more aspirational trace split verbatim.
4. The scenario catalog is the correct place to explain causal distinctness, setup, systems exercised, and cross-system chains. It should explicitly distinguish Scenario 26 from Scenario 3d, Scenario 27 from Scenario 2b, and Scenario 28 from Scenarios 22 and 24 so review does not over-credit the earlier success-path goldens.
5. The catalog must stay aligned with [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md): S08/S15-style cases should separate authoritative start failure, AI/planning reconciliation, and durable world outcome instead of using later world state as a proxy for the whole chain.
6. Scenario isolation is relevant for all three new entries. The implemented tests intentionally keep a single local contested opportunity and a single remote fallback/recovery branch so the intended lawful post-selection loss stays reviewable. The ticket should document that isolation choice rather than implying the branch was inevitable in the full architecture.
7. The header inventory in [docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md) is stale. Live `cargo test -p worldwake-ai -- --list` output shows 124 `golden_*` tests, matching [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md), while the scenario catalog still says 118.
8. Scope correction: this remains a documentation ticket with `Engine Changes: None`, but the earlier wording understated that the implementation already landed in production/trade/emergent golden suites. The ticket should be updated to reconcile docs against live tests and inventory, not against the pre-implementation spec alone.

## Architecture Check

1. Updating the scenario catalog keeps planning and review aligned with the actual causal contract each golden proves. That is cleaner than leaving S15 represented only by the care race or by the spec, because the live architecture intentionally routes start-failure recovery through shared runtime/action-trace/AI-failure surfaces rather than through domain-specific aliases or one-off explanations.
2. No backwards-looking documentation should imply that success-path trade or politics coverage already proved lawful post-selection loss when it did not.
3. No backwards-compatibility aliasing or shim language should be added. If the catalog differs from the live tests, the catalog should move to the current architecture.

## Verification Layers

1. Scenario entries name the exact new tests and their owning files -> doc review against implemented golden test names.
2. Each entry distinguishes authoritative start failure, AI/planning reconciliation, and durable world outcome using the proof surfaces the live tests actually assert -> doc review against implemented assertions in the owning golden files.
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
- `crates/worldwake-ai/tests/golden_emergent.rs` (modify, lint-only)

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
5. Owning binaries stay green after the doc reconciliation references are updated: `cargo test -p worldwake-ai --test golden_production`, `cargo test -p worldwake-ai --test golden_trade`, `cargo test -p worldwake-ai --test golden_emergent`

### Invariants

1. The catalog must accurately describe the actual implemented scenarios, test names, and owning files.
2. Each S15 entry must explicitly separate authoritative start-failure proof, AI/planning reconciliation, and authoritative downstream outcome without claiming a stronger proof surface than the live test actually uses.
3. Each S15 entry must explain why it is not redundant with an existing success-path or contention-path golden.
4. Any suite-count summary inside `docs/golden-e2e-scenarios.md` must stay consistent with the live golden inventory and the coverage dashboard.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_emergent.rs` — remove an unnecessary `clone()` on `GoalKind` so `cargo clippy --workspace --all-targets -- -D warnings` passes without changing test behavior.
2. `None — no new tests were added for this ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
3. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
4. `cargo test -p worldwake-ai --test golden_production`
5. `cargo test -p worldwake-ai --test golden_trade`
6. `cargo test -p worldwake-ai --test golden_emergent`
7. `cargo test -p worldwake-ai -- --list`
8. `cargo test -p worldwake-ai`
9. `cargo test --workspace`
10. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- Completion date: 2026-03-20
- What actually changed: updated [docs/golden-e2e-scenarios.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-scenarios.md) to add Scenario 26-28 entries, corrected the stale `118` golden-test inventory to the live `124`, and aligned each new entry with the actual proof surfaces used by the implemented S15 tests rather than the earlier spec-only wording.
- Deviation from original plan: the ticket started as "documentation-only", but broader required verification exposed one existing clippy violation in [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs). That was fixed with a one-line lint-only edit so workspace lint could pass; no production behavior or scenario assertions changed.
- Verification results: `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`, `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`, `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`, `cargo test -p worldwake-ai --test golden_production`, `cargo test -p worldwake-ai --test golden_trade`, `cargo test -p worldwake-ai --test golden_emergent`, `cargo test -p worldwake-ai -- --list`, `cargo test -p worldwake-ai`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` all passed.
