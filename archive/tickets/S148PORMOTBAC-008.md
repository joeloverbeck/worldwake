# S148PORMOTBAC-008: Remove max_candidates_to_plan from CognitiveProfile and ReasoningProfile

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — removed the legacy `CognitiveProfile.max_candidates_to_plan` field and migrated live planning-cap reads to `PortfolioWeightsProfile.max_plans_for_mode(runtime.operating_mode)`. The ticket's drafted `ReasoningProfile` target was stale: no live `ReasoningProfile` exists in the current codebase, so the same-seam work removed the test-only `ProfileFixture.max_candidates_to_plan` relay instead.
**Deps**: `archive/tickets/S148PORMOTBAC-002.md`, `archive/tickets/S148PORMOTBAC-003.md`, `archive/tickets/S148PORMOTBAC-004.md`, `archive/specs/S148-portfolio-and-motive-backed-intentions.md`

## Problem

`max_candidates_to_plan` previously lived as a single cap on `CognitiveProfile`, while S148's portfolio work had already introduced per-mode plan caps on `PortfolioWeightsProfile` (`max_plans_normal`, `max_plans_emergency`, `max_plans_idle`) and cached `OperatingMode` on `AgentDecisionRuntime`. Leaving both cap surfaces live would preserve two sources of truth for the same planning budget and block the intended mode-aware cap behavior.

## Outcome

The legacy cap field is gone from `CognitiveProfile`, default construction, serde round-trip fixtures, scenario RON files, CLI scenario fixtures, and AI test fixtures. Runtime planning now reads the cap from `PortfolioWeightsProfile::max_plans_for_mode` using the current `AgentDecisionRuntime.operating_mode`.

The drafted `ReasoningProfile` field was not present in live code. Current fixtures used a local `ProfileFixture` relay in `crates/worldwake-ai/src/lib.rs`; that relay and its construction sites were removed in the same migration.

## Landed Changes

- Added `PortfolioWeightsProfile::max_plans_for_mode(OperatingMode)` and focused coverage for Normal, Emergency, and Idle caps.
- Removed `CognitiveProfile.max_candidates_to_plan` from the core profile, defaults, delta fixtures, and tests.
- Removed the test-only `ProfileFixture.max_candidates_to_plan` relay from AI fixtures.
- Updated `agent_tick::planning` so both the primary candidate cap and same-goal planning trace use `portfolio_weights.max_plans_for_mode(runtime.operating_mode)`.
- Updated CLI inspect output to render the portfolio max-plan fields instead of the removed cognitive cap.
- Removed authored `max_candidates_to_plan` fields from committed scenario RON files.

## Landed Files

- `crates/worldwake-core/src/portfolio_weights_profile.rs`
- `crates/worldwake-core/src/cognitive_profile.rs`
- `crates/worldwake-core/src/delta.rs`
- `crates/worldwake-ai/src/lib.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`
- `crates/worldwake-ai/src/decision_runtime.rs`
- `crates/worldwake-ai/src/failure_handling.rs`
- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/search/tests.rs`
- `crates/worldwake-cli/src/handlers/inspect.rs`
- `crates/worldwake-cli/src/handlers/persistence.rs`
- `crates/worldwake-cli/src/scenario/types.rs`
- `scenarios/*.ron` files that authored the removed field

## Accepted Invariants

1. `CognitiveProfile.max_candidates_to_plan` no longer exists in source or committed scenarios.
2. No live `ReasoningProfile.max_candidates_to_plan` exists; the stale test-only relay was removed.
3. Live planning-cap reads use `PortfolioWeightsProfile.max_plans_for_mode(runtime.operating_mode)`.
4. No compatibility shim, deprecated alias, or replacement `max_candidates_to_plan` accessor was added.

## Verification Result

- Passed: `rg -n "max_candidates_to_plan" crates/worldwake-core/src crates/worldwake-ai/src crates/worldwake-cli/src scenarios` returned no matches.
- Passed: `cargo fmt --all`
- Passed: `cargo test -p worldwake-core portfolio_weights_profile`
- Passed: `cargo test -p worldwake-core cognitive_profile`
- Passed: `cargo test -p worldwake-ai agent_tick::planning`
- Passed: `cargo test -p worldwake-cli scenario`
- Passed: `cargo test -p worldwake-cli handlers`
- Passed: `cargo clippy --workspace --all-targets -- -D warnings`
- Passed: `cargo test --workspace`

## Notes

The drafted combined command `cargo test -p worldwake-core portfolio_weights_profile cognitive_profile` is not a valid Cargo invocation because Cargo accepts a single test filter. It was replaced with separate `worldwake-core` test-filter runs for `portfolio_weights_profile` and `cognitive_profile`.

The broad verification was run as direct cargo commands rather than through `./scripts/verify.sh`.
