# S146GOASCHGOA-007: Migration validation tests for extractor dispatch and per-goal budget traces

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Tests only — no production code changes
**Deps**: archive/tickets/S146GOASCHGOA-005.md, archive/tickets/S146GOASCHGOA-006.md

## Problem

S146's candidate-generation migration and per-goal budget application needed a final validation layer after tickets 005 and 006 landed. The remaining live risk was not an unimplemented production path: ticket 005 had already migrated candidate generation to schema-derived `CandidateExtractorId` dispatch, and ticket 006 had already recorded the applied `GoalPlanningBudget` on search traces. This ticket added focused regressions that prove the schema-derived extractor dispatch still covers every registered extractor exactly once in legacy order, and that the concrete S146 gate examples map and trace as `ProduceCommodity` -> `PRODUCTION` and self-care acquisition/consumption -> `SELF_CARE` under an elevated cognitive ceiling.

## Assumption Reassessment (2026-05-17)

1. `archive/tickets/S146GOASCHGOA-005.md` already landed the `CandidateExtractor` trait, the 20 extractor impls, `build_extractor_registry`, `ordered_candidate_extractors_from_goal_schemas`, and a focused opt-out regression in `crates/worldwake-ai/src/candidate_generation.rs`.
2. `archive/tickets/S146GOASCHGOA-006.md` already landed `search/mod.rs` budget composition and `PlanAttemptTrace.goal_budget`, with focused tests for generic preset clamping and strategic cap composition in `crates/worldwake-ai/src/search/tests.rs`.
3. The drafted JSON fixture protocol depended on pre-migration `emit_*` output capture, but no fixture capture target or committed pre-migration fixture set exists on the live branch. Recreating fixtures after ticket 005 would not prove the claimed pre/post byte identity; it would snapshot the already-migrated implementation. The honest remaining parity seam is structural: registry coverage, legacy-order dispatch, deduplication of schema-shared extractor IDs, and the existing candidate-generation suite.
4. The drafted `golden_per_goal_budget.rs` scenario was superseded by a stronger lower-layer proof surface. The concrete contract under S146 is that `GoalDispatchKey` schema mapping feeds search metadata, not that a long-running autonomous scenario completes a particular branch at a particular tick. A focused search-trace test proves this without scenario timing noise.
5. Live `GoalKind` examples under test are `GoalKind::ProduceCommodity { recipe_id }`, `GoalKind::ConsumeOwnedCommodity { commodity }`, and `GoalKind::AcquireCommodity { purpose: SelfConsume, .. }`.
6. Ordering layer: candidate dispatch order remains the legacy extractor order declared in `candidate_generation.rs::LEGACY_EXTRACTOR_ORDER`; search budget differentiation depends on the schema preset and the elevated `CognitiveProfile` ceiling, while `max_strategic_expansions` remains capped by S145 stage-aware `ExecutionBudget`.

## Architecture Check

1. FND-28: validation targets the single live schema/extractor registry and does not create a parallel fixture registry or stale compatibility path.
2. FND-29: per-goal budget provenance is verified at the decision-trace/search metadata layer where the applied budget is actually recorded.
3. FND-31: the tests falsify the migration/budget contracts at their strongest stable lower layers, avoiding brittle autonomous scenario timing as a proxy for registry correctness.

## Verified Layers

1. Extractor registry coverage and deduped schema-derived legacy order -> `candidate_generation::tests::schema_derived_extractor_order_covers_every_registered_extractor_once`.
2. S146 example goal-to-budget schema mapping -> `search::tests::s146_goal_kind_budget_examples_map_to_expected_presets`.
3. Applied per-goal budget reaches search trace metadata under elevated cognitive ceiling -> `search::tests::s146_search_trace_records_per_goal_budget_under_elevated_cognitive_ceiling`.
4. Full AI candidate-generation, planner, and golden regression surface -> `cargo test -p worldwake-ai --quiet`.
5. Workspace and CI-shaped lint gates -> `cargo test --workspace --quiet` and `cargo clippy --workspace --all-targets -- -D warnings`.

## Landed Changes

### 1. Extractor dispatch validation

Added `schema_derived_extractor_order_covers_every_registered_extractor_once` in `crates/worldwake-ai/src/candidate_generation.rs`. It proves `build_extractor_registry()` covers `CandidateExtractorId::ALL`, `ordered_candidate_extractors_from_goal_schemas()` preserves the legacy top-level extractor order, and schema-derived dispatch dedupes extractor IDs shared across multiple goal schemas.

### 2. Per-goal budget mapping and trace validation

Added two focused search tests in `crates/worldwake-ai/src/search/tests.rs`. One checks the concrete S146 examples map to the intended presets. The other runs search with an elevated cognitive ceiling and asserts the applied trace metadata records `PRODUCTION` depth/expansions for `ProduceCommodity` and `SELF_CARE` depth/expansions for self-care acquisition.

## Landed Files

- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-ai/src/search/tests.rs`
- `archive/specs/S146-goal-schema-and-per-goal-budgets.md`
- `specs/IMPLEMENTATION-ORDER.md`
- `archive/tickets/S146GOASCHGOA-007.md`

## Out of Scope

- JSON parity fixtures against pre-ticket-005 `emit_*` output. No committed pre-migration fixture capture exists, and recreating it after migration would not prove the intended before/after identity.
- A new autonomous `golden_per_goal_budget.rs` scenario. The landed lower-layer trace proof is stronger for this contract and avoids scenario timing noise.
- Registry coverage already owned by ticket 004 and generic budget clamping already owned by ticket 006; this ticket validates their concrete S146 handoff examples and migration dispatch order.
- Observer rendering of budget preset names remains ticket 008.

## Acceptance Result

### Tests Passed

1. `candidate_generation::tests::schema_derived_extractor_order_covers_every_registered_extractor_once`
2. `search::tests::s146_goal_kind_budget_examples_map_to_expected_presets`
3. `search::tests::s146_search_trace_records_per_goal_budget_under_elevated_cognitive_ceiling`
4. `cargo test -p worldwake-ai --quiet`
5. `cargo test --workspace --quiet`
6. `cargo clippy --workspace --all-targets -- -D warnings`

### Invariants

1. Every `CandidateExtractorId::ALL` entry has a live extractor registry entry.
2. Schema-derived candidate dispatch keeps legacy extractor order and removes duplicate extractor IDs introduced by goal schemas that share extractor families.
3. `ProduceCommodity` maps to the `PRODUCTION` preset; self-care consume/acquire goals map to `SELF_CARE`.
4. Search trace metadata records the applied effective budget's depth and node-expansion limits under elevated cognitive ceilings.

## Test Plan Result

### Added Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` — `schema_derived_extractor_order_covers_every_registered_extractor_once`
2. `crates/worldwake-ai/src/search/tests.rs` — `s146_goal_kind_budget_examples_map_to_expected_presets`
3. `crates/worldwake-ai/src/search/tests.rs` — `s146_search_trace_records_per_goal_budget_under_elevated_cognitive_ceiling`

### Commands Run

1. `cargo test -p worldwake-ai --lib candidate_generation::tests::schema_derived_extractor_order_covers_every_registered_extractor_once -- --exact`
2. `cargo test -p worldwake-ai --lib search::tests::s146_goal_kind_budget_examples_map_to_expected_presets -- --exact`
3. `cargo test -p worldwake-ai --lib search::tests::s146_search_trace_records_per_goal_budget_under_elevated_cognitive_ceiling -- --exact`
4. `cargo test -p worldwake-ai --quiet`
5. `cargo test --workspace --quiet`
6. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-17.

- Added the final S146 validation seam for candidate extractor registry dispatch and concrete per-goal budget trace examples.
- Replaced the stale drafted parity-fixture and autonomous-golden plan with focused lower-layer tests that prove the live migration and budget contracts directly.
- Truthed the active S146 spec and implementation-order gate to name the landed search-trace proof surface instead of the stale parity-fixture/autonomous-golden plan.
- No production code, scenarios, generated golden docs, or observer surfaces changed.

## Deviations

- The drafted `extractor_parity.rs` JSON fixture suite was not implemented because no pre-migration fixture capture exists on the live branch and post-migration capture would be circular evidence.
- The drafted `golden_per_goal_budget.rs` autonomous scenario was not implemented because the stable contract is the search metadata budget handoff, now covered directly by `s146_search_trace_records_per_goal_budget_under_elevated_cognitive_ceiling`.
- The applied trace budget's `max_strategic_expansions` remains stage-capped by S145; the new trace test therefore asserts the S146-owned depth and node-expansion differentiation instead of requiring whole-preset equality.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib candidate_generation::tests::schema_derived_extractor_order_covers_every_registered_extractor_once -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::s146_goal_kind_budget_examples_map_to_expected_presets -- --exact`
- Passed `cargo test -p worldwake-ai --lib search::tests::s146_search_trace_records_per_goal_budget_under_elevated_cognitive_ceiling -- --exact`
- Passed `cargo test -p worldwake-ai --quiet`
- Passed `cargo test --workspace --quiet`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
