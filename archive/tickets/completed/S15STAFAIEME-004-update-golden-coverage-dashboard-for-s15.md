# S15STAFAIEME-004: Update Golden Coverage Dashboard For S15 Start-Failure Emergence

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: S15STAFAIEME-001, S15STAFAIEME-002, S15STAFAIEME-003

## Problem

[docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) is now stale against the live golden suite. S15's three start-failure emergence scenarios have already landed in production, trade, and political coverage, but the dashboard still reports the pre-S15 inventory, still frames S08 as care-only in the interaction matrix, and still carries backlog wording that points at the old S08 traceability blocker instead of the remaining S10 pricing blocker on the blocked supply-chain chain.

## Assumption Reassessment (2026-03-19)

1. The current dashboard in [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) still says the suite contains 118 `golden_*` tests across 10 files, but `cargo test -p worldwake-ai -- --list` and the live `golden_*.rs` declarations now show 124 `golden_*` tests across the same 10 files, with 9 files contributing `golden_*` tests.
2. The S15 scenarios are already implemented, not pending future work: `golden_contested_harvest_start_failure_recovers_via_remote_fallback` and replay variant in [crates/worldwake-ai/tests/golden_production.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_production.rs), `golden_local_trade_start_failure_recovers_via_production_fallback` and replay variant in [crates/worldwake-ai/tests/golden_trade.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_trade.rs), and `golden_remote_office_claim_start_failure_loses_gracefully` and replay variant in [crates/worldwake-ai/tests/golden_emergent.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_emergent.rs).
3. Existing documentation has already partially moved on: [docs/golden-e2e-testing.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-testing.md) already cites `golden_local_trade_start_failure_recovers_via_production_fallback` as a golden `StartFailed` example, which makes the coverage dashboard's older inventory and care-only framing internally inconsistent.
4. This remains a documentation-only ticket. The gap is not missing focused/unit coverage or missing golden coverage in code; the gap is stale dashboard inventory and stale narrative. No production or test code should change unless the doc audit uncovers a real mismatch in live test names.
5. The current backlog section incorrectly says S02c is blocked on `specs/S08-ai-decision-traceability.md`, but S08 is already implemented and the live blocker called out by [specs/S15-start-failure-emergence-golden-suites.md](/home/joeloverbeck/projects/worldwake/specs/S15-start-failure-emergence-golden-suites.md) and [archive/specs/S09-travel-aware-plan-search.md](/home/joeloverbeck/projects/worldwake/archive/specs/S09-travel-aware-plan-search.md) is the unfinished S10 bilateral trade-pricing architecture.
6. Because this ticket only reconciles documentation to already-landed behavior, the correct verification layers are inventory and consistency checks against live test binaries plus targeted reruns of the S15 golden tests. No extra action-trace or decision-trace mapping is introduced here beyond accurately naming the existing scenarios.
7. Scope correction: this ticket is no longer "prepare the dashboard before S15 lands." It is "reconcile the dashboard to already-landed S15 coverage and current backlog reality." Do not rewrite scenario semantics or test code here; update the dashboard to match the code that exists.

## Architecture Check

1. Keeping the dashboard grounded in the live `cargo test -p worldwake-ai -- --list` inventory is cleaner than maintaining a hand-waved historical narrative. The doc should describe the suite that exists today, not preserve an outdated transition state.
2. Updating the dashboard is more beneficial than the current architecture because the current architecture has already improved: S08 start-failure handling is now proven as a shared contract across production, trade, and politics. The documentation should reflect that broader, cleaner architecture instead of obscuring it behind the older care-only story.
3. No backwards-compatibility wording, aliasing, or historical caveat should preserve the obsolete "S08 is only proven in care" or "S02c is blocked on S08 traces" narrative once the suite and specs have moved on.

## Verification Layers

1. File counts and `golden_*` totals match the live binaries -> `cargo test -p worldwake-ai -- --list`.
2. The production, trade, and political S15 chains are represented in the dashboard narrative -> doc review against the implemented S15 test names in `golden_production.rs`, `golden_trade.rs`, and `golden_emergent.rs`.
3. The dashboard no longer frames S08 proof as care-only -> coverage-matrix and summary review in `docs/golden-e2e-coverage.md`.
4. The S02c backlog note names the active S10 pricing blocker instead of the stale S08 traceability blocker -> doc diff review against `specs/S15-start-failure-emergence-golden-suites.md` and `archive/specs/S09-travel-aware-plan-search.md`.
5. Additional action/decision-trace mapping is not applicable because this ticket does not change executable behavior; it only references already-implemented scenarios that themselves carry those proofs.

## What to Change

### 1. Update the file layout and summary counts

Revise the file-layout counts, total `golden_*` count, and summary statistics in [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) using the actual live `--list` output and current `golden_*.rs` declarations.

### 2. Add the new S15 cross-system chains

Add the production, trade, and political S15 scenarios to the cross-system interaction section and summary language so the dashboard reflects S08 start-failure recovery as a shared architecture contract across care, production, trade, and politics.

### 3. Fix stale backlog wording

Replace the outdated S02c blocker note that cites the archived traceability spec with the active S10 pricing blocker named in the S15 spec and reinforced by the archived S09 outcome.

## Files to Touch

- `docs/golden-e2e-coverage.md` (modify)

## Out of Scope

- `docs/golden-e2e-scenarios.md`
- `docs/golden-e2e-testing.md`
- any `crates/` test or runtime code
- changing scenario semantics to make the documentation easier to write

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
2. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
3. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
4. Inventory check: `cargo test -p worldwake-ai -- --list`
5. Documentation consistency check: `cargo test -p worldwake-ai --test golden_production --test golden_trade --test golden_emergent`

### Invariants

1. The dashboard must reflect the codebase that actually exists after S15 lands; counts and names must come from the live test binaries.
2. S08 coverage must not be described as care-only now that production, trade, and political start-failure recovery each have explicit golden proof.
3. Documentation must not reintroduce stale blockers or archived-spec references as if they were current active constraints.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo test -p worldwake-ai -- --list`
2. `cargo test -p worldwake-ai golden_contested_harvest_start_failure_recovers_via_remote_fallback -- --exact`
3. `cargo test -p worldwake-ai golden_local_trade_start_failure_recovers_via_production_fallback -- --exact`
4. `cargo test -p worldwake-ai golden_remote_office_claim_start_failure_loses_gracefully -- --exact`
5. `cargo test -p worldwake-ai --test golden_production --test golden_trade --test golden_emergent`

## Outcome

- Completion date: 2026-03-19
- What actually changed: updated [docs/golden-e2e-coverage.md](/home/joeloverbeck/projects/worldwake/docs/golden-e2e-coverage.md) to match the live S15-era golden inventory, including 124 `golden_*` tests, the new production/trade/political S15 start-failure chains, and the corrected S10 pricing blocker for S02c. Updated this ticket first to correct its stale assumptions and scope before implementation.
- Deviations from original plan: no code or test files changed because the S15 scenarios were already implemented. The work was documentation reconciliation, not preparation for future S15 landing. The broader doc architecture still has parallel manual inventory drift in `docs/golden-e2e-scenarios.md`, but that remained out of scope for this ticket.
- Verification results: `cargo test -p worldwake-ai -- --list`, all three exact S15 scenario tests, `cargo test -p worldwake-ai --test golden_production`, `cargo test -p worldwake-ai --test golden_trade`, `cargo test -p worldwake-ai --test golden_emergent`, `cargo test --workspace`, and `cargo clippy --workspace` all passed.
