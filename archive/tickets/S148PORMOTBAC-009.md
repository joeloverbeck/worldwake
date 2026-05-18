# S148PORMOTBAC-009: Observer Decision History rendering for slot, motives, claims, conditions

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: No — observer-only rendering changes in `crates/worldwake-cli/src/bin/observer.rs`, plus the matching observer golden fixture update
**Deps**: `archive/tickets/S148PORMOTBAC-004.md`, `archive/tickets/S148PORMOTBAC-006.md`, `archive/tickets/S148PORMOTBAC-007.md`, `specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

The observer Decision History section rendered committed goals and motive-source contribution rows, but it did not expose the S148 slot taxonomy or the enriched `IntentionFrame` fields added by tickets 006 and 007. Operators could see that a goal committed, but not the committed slot, slot weight, explicit claims, resume/abandon conditions, or causal links when those fields were present.

## Outcome

Decision History now adds a `Committed: ... (Slot: ..., weight ...)` detail row for committed goals when a motive source can be mapped to a `SlotKind`. The observer derives that slot from the current matching frame, the committed payload's decisive motive sources, or the selected ranked-goal trace summary, in that order. The slot weight is read from the agent's `PortfolioWeightsProfile`.

When the committed goal still matches the agent's current `IntentionFrame`, the observer renders every populated enriched field: `motive_refs`, `explicit_claims`, `resume_conditions`, `abandon_conditions`, and `causal_links`. Empty vectors are skipped. The live event stream does not contain a historical full-frame snapshot, so this observer-only ticket did not add an engine trace payload to reconstruct cleared historical frames.

## Landed Changes

- Added Decision History committed-intention detail rows with canonical slot names and portfolio weights.
- Added observer helpers for extended `IntentionFrame` rendering, resume/abandon condition formatting, opportunity-anchor formatting, and explicit-claim labeling.
- Added artifact-claim dispatch for contention grants, sale listings, social artifacts, and unknown entities.
- Added focused observer tests for populated and empty frame rendering, condition formatter coverage, sale-listing claim dispatch, and integrated Decision History rendering.
- Updated `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md` for the new committed slot rows.
- Added a D11 implementation note to `specs/S148-portfolio-and-motive-backed-intentions.md`.

## Landed Files

- `crates/worldwake-cli/src/bin/observer.rs`
- `crates/worldwake-cli/tests/fixtures/observer_decision_history/survival_baseline_5_ticks.md`
- `specs/S148-portfolio-and-motive-backed-intentions.md`

## Accepted Invariants

1. The observer renders canonical `SlotKind` names for committed goals when a motive-backed slot can be derived from live observer inputs.
2. The observer renders all populated enriched `IntentionFrame` vectors available on a matching current frame and skips empty vectors silently.
3. The observer remains read-only: it reads world components, event payloads, and trace summaries without mutating simulation state.
4. No `ScenarioDiagnosticsReport` schema change was introduced.

## Verification Result

- Passed: `cargo fmt --all`
- Passed: `cargo test -p worldwake-cli --bin observer`
- Passed: `cargo test -p worldwake-cli`
- Passed: `cargo run -p worldwake-cli --bin observer -- scenarios/survival-baseline.ron --ticks 5 --output /tmp/worldwake-observer-s148-009.md`
- Passed: `rg -n "Slot:|Motives:|Claims:|Resume on:|Abandon if:" /tmp/worldwake-observer-s148-009.md`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
- Passed: `cargo test --workspace`

## Notes

The drafted focused-test command used multiple Cargo test filters, which Cargo does not accept. The actual focused proof used the observer binary test target and package-level CLI tests.
