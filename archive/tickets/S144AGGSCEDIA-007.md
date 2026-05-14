# S144AGGSCEDIA-007: Golden coverage and survival-baseline regression fixture

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None - test infrastructure, observer JSON test helper extraction, and committed fixture
**Deps**: archive/tickets/S144AGGSCEDIA-005.md, archive/tickets/S144AGGSCEDIA-006.md

## Problem

Before this ticket, S144's deterministic report had no committed golden fixture. A change that silently shifted the scenario diagnostics output could pass without review.

## Assumption Reassessment (2026-05-14)

1. `crates/worldwake-ai/tests/golden_scenario_diagnostics.rs` did not exist before this ticket. Golden tests follow the `crates/worldwake-ai/tests/golden_*.rs` naming convention, and `docs/golden-e2e-testing.md` requires generated inventory refreshes when adding golden files.
2. S144 spec D9+D10 in `archive/specs/S144-aggregate-scenario-diagnostics.md` called for deterministic `survival-baseline.ron` coverage, schema coverage, top-N overflow coverage, observer JSON round-trip coverage, and a committed `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` fixture.
3. The observer's deterministic diagnostics JSON representation existed only inside the observer binary. The golden needed to exercise that same representation without copying private mirror structs, so this ticket extracted that JSON representation into `worldwake_cli::diagnostics_json` and kept the observer binary as a caller.
4. The live `survival-baseline.ron` run has no repair attempts. The landed schema coverage therefore proves field presence, fixture stability, deterministic zero values, and populated fields that the live scenario actually exercises; it does not force non-empty repair-budget percentile buckets for a scenario that records zero repairs.

## Architecture Check

1. The committed fixture plus replay companion test makes diagnostics drift a hard golden failure instead of silent output churn.
2. The observer JSON helper extraction is a test/tooling boundary change, not a simulation engine change. It removes duplicated JSON schema logic from the golden and keeps the observer binary and golden test on the same representation.
3. The generated golden inventory was refreshed after adding the new golden file, so the new Scenario 421 metadata is registered in the generated docs.

## Outcome

Completed on 2026-05-14. The ticket landed a deterministic `survival-baseline.ron` scenario diagnostics golden, committed fixture, observer JSON representation helper, regenerated golden inventory docs, and S144 spec truth-sync for the live D9 proof shape.

## Landed Changes

1. Added `crates/worldwake-ai/tests/golden_scenario_diagnostics.rs` with Scenario 421 metadata and two tests:
   - `golden_scenario_diagnostics_survival_baseline_fixture_is_stable`
   - `golden_scenario_diagnostics_survival_baseline_replays_deterministically`
2. Added `crates/worldwake-ai/tests/fixtures/expected-scenario-diagnostics.json` as the committed deterministic fixture for `scenarios/survival-baseline.ron`.
3. Added `crates/worldwake-cli/src/diagnostics_json.rs` and exported it from `crates/worldwake-cli/src/lib.rs`.
4. Updated `crates/worldwake-cli/src/bin/observer.rs` to render diagnostics JSON through the shared library helper instead of private duplicate mirror structs.
5. Regenerated golden inventory docs, including the new `docs/generated/golden-scenario-details/scenario-diagnostics.md` page. Other touched generated scenario-detail files are expected index/source-line fallout from adding the new golden file.
6. Resolved the post-ticket review metadata blocker by changing Scenario 421's source comment to the generator-recognized `// Chain:` key so generated docs publish the cross-system chain.

## Out of Scope Result

1. The existing aggregator and report type implementation from tickets 004 and 005 was not changed.
2. The observer top-N text rendering behavior from ticket 006 was not re-owned here. This ticket proves overflow eligibility through the raw diagnostics histogram cardinality and exercises the observer JSON representation through the shared helper.
3. No diagnostics coverage was added for scenarios other than `survival-baseline.ron`.

## Acceptance Result

1. Determinism is covered by the replay companion test, which reruns the scenario and compares the report against the cached first run.
2. Schema coverage is covered by the stable-fixture test, including field presence and deterministic zero repair values for the live `survival-baseline.ron` output.
3. Top-N overflow coverage is covered by asserting that the raw report contains more than three candidate groups, so the observer top-N renderer has overflow data available.
4. Observer JSON round-trip coverage is covered through `scenario_diagnostics_report_to_json_pretty` and `scenario_diagnostics_report_from_json`, with equality against the report structure and committed fixture.
5. Generated golden docs are current after `python3 scripts/golden_inventory.py --write --check-docs`.

## Deviations

1. The drafted ticket expected the golden to assert `--diagnostics-top-n 3` text output directly. The landed proof keeps ticket 006's observer rendering test as the text-rendering owner and proves this ticket's top-N requirement by asserting raw overflow cardinality plus observer JSON representation parity.
2. The drafted schema-coverage wording implied every field would be non-empty. The live scenario legitimately records zero repair attempts, so the landed test asserts deterministic zero repair fields instead of fabricating repair data outside `survival-baseline.ron`.
3. The golden inventory regeneration touched additional generated detail pages because adding a new golden file changed source-line references and scenario index placement.

## Verification Result

- Passed `cargo fmt --all`.
- Passed `cargo test -p worldwake-ai --test golden_scenario_diagnostics`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `cargo test -p worldwake-ai`.
- Passed `./scripts/verify.sh` after fixing a stale observer test import exposed by the first wrapper run. The passing rerun covered `cargo fmt --all -- --check`, `cargo test --workspace`, repository removal checks, `cargo clippy --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo run -p worldwake-cli --bin scenario-coverage -- --check`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs` after resolving the Scenario 421 `// Chain:` metadata key.
