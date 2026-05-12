# S138OPPCOM-010: Golden coverage and regression guards for opportunity compiler

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: None (tests only; uses existing harness)
**Deps**: archive/tickets/S138OPPCOM-006.md (compile_opportunities + agent_tick integration), archive/tickets/S138OPPCOM-007.md (travel-pruning detour), archive/tickets/S138OPPCOM-008.md (interrupt enrichment), archive/tickets/S138OPPCOM-009.md (decision-trace opportunity carriage for diagnostics)

## Problem

S138 requires golden coverage proving the compiler's headline scenarios: steal/buy/beg ranking with a starving agent, detour-allow when an alternative source lies along the route, witness-inquiry detour, effect-schema index miss (negative test), and `LearnedOpportunityMemory` damping. The ticket also lands two regression guards: (a) `survival-baseline.ron` event-log equality between pre-S138 and post-S138 at default profiles (the bottom-up pass is additive), and (b) a soft performance assertion that per-tick opportunity compilation stays bounded under 1440-tick `survival-contested.ron` with 4 agents.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage:
   - `crates/worldwake-ai/tests/` houses goldens following the `golden_*.rs` naming convention (e.g., `golden_ai_decisions.rs`, `golden_exploration.rs`, `golden_quantity_aware_acquisition.rs`)
   - `scenarios/survival-baseline.ron` and `scenarios/survival-contested.ron` exist (4 agents in survival-contested per agent verification)
   - `crates/worldwake-systems/src/epistemic_actions.rs` registers the existing `AskWitness` action — usable for scenario 3
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` "Validation and Falsification" section enumerates the 5 scenarios + regression check + performance assertion.
3. Shared abstraction boundary: full-stack E2E — scenario RON → spawn_scenario → agent_tick loop → decision-trace + event-log assertions. The goldens consume the entire opportunity compiler stack landed in tickets 001-009.
4. Coverage-gap classification (precision-rules.md §3): this is golden/E2E coverage, distinct from the focused/unit coverage already covered in tickets 006-008.
5. Scenario isolation (precision-rules.md §8): each of the 5 scenarios isolates one specific causal branch (steal-vs-buy ranking, detour-allow, witness inquiry, index miss, damping). The setup explicitly excludes competing affordances that would let the agent satisfy the same need via an unrelated pathway.
6. Existing 1440-tick goldens (`survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron`) must continue to pass — opportunities are additive at default profiles, validated by the regression assertion in this ticket.

## Architecture Check

1. The regression assertion (event-log equality on `survival-baseline.ron`) gates additivity: if the bottom-up pass diverges from emitter-only behavior at default profiles, the spec's "additive at default" claim is wrong and the divergence is a bug, not a feature.
2. Per-scenario isolation prevents over-broad assertions: scenario 4 (effect-schema index miss) is a negative test asserting that an unknown effect produces no opportunity — this is the canonical guard against accidental opportunity inflation.
3. The performance assertion is a soft soak (≤ 5% of `agent_tick` total) rather than a wall-clock benchmark — it measures relative cost using existing per-tick timing infrastructure (or candidate-count growth as a proxy if wall-clock isn't deterministic on CI).
4. Goldens use scenarios authored in RON, not hardcoded fixtures — the test surface remains scenario-author-modifiable per the project's authoring contract.

## Verification Layers

1. Scenario 1 (steal/buy/beg ranking): authoritative world state after N ticks shows the agent's chosen action — golden assertion on the event log + decision-trace `RootCandidateTrace.source` attribution
2. Scenario 2 (detour-allow for alternative resource): action trace shows the agent traveling along the detour path; event log shows the alternative resource consumed
3. Scenario 3 (witness-inquiry detour): action trace shows `AskWitness` action started along the detour path; bounded by `detour_budget_permille`
4. Scenario 4 (effect-schema index miss): decision trace shows no opportunity emitted for the unknown effect; `OpportunityCompilerLoad.compiled_count == 0` for the relevant tick
5. Scenario 5 (`LearnedOpportunityMemory` damping): decision trace across successive ticks shows declining salience for the repeated low-yield opportunity
6. Regression (survival-baseline): event-log equality assertion between pre-S138 baseline (recorded once before this ticket lands) and post-S138 replay
7. Performance: per-tick `OpportunityCompilerLoad.compiled_count` × work-per-entity stays bounded under the existing perception budget (S105 cap) — recorded in `OpportunityCompilerLoad` already; assertion compares aggregate cost against `agent_tick` total

## What to Change

### 1. New golden file `crates/worldwake-ai/tests/golden_opportunity_compiler.rs`

Five `#[test]` functions, each spawning a scenario, running N ticks, and asserting on the resulting traces:

- `scenario_1_starving_agent_steal_vs_buy()` — agent with high `theft_aversion = 200`, low `criminal_threshold = 300` vs another agent with reversed profiles; assert different action choices
- `scenario_2_thirsty_agent_detour_to_alternative_source()` — primary well dry, alternative source via detour; assert detour taken
- `scenario_3_witness_inquiry_detour()` — agent en route to market sees a witness anchor along path; assert `AskWitness` started within `detour_budget_permille`
- `scenario_4_effect_schema_index_miss()` — agent perceives an entity whose effect facts have no actions producing them (e.g., a decorative landmark); assert no opportunity emitted, `OpportunityCompilerLoad.compiled_count` for that entity = 0
- `scenario_5_learned_memory_damping()` — same opportunity perceived across 3 successive ticks; assert salience declines per tick due to `LearnedOpportunityMemory` decay

Each scenario uses an inline RON-style scenario constructor or references a new scenario file under `scenarios/s138/`.

### 2. Regression test `survival_baseline_pre_s138_equality`

Record the survival-baseline event log once (pre-merge baseline checked in as a fixture under `crates/worldwake-ai/tests/fixtures/survival-baseline-pre-s138.bin` or similar). The test replays `survival-baseline.ron` post-S138 and asserts the event log matches the recorded baseline byte-for-byte at default profiles.

If baseline drift is intentional (e.g., perception-budget-related), record and check in the new baseline with explicit rationale in the test comment.

### 3. Performance soak `opportunity_compilation_under_5_percent_agent_tick`

A 1440-tick replay of `survival-contested.ron` (4 agents) instrumented with `OpportunityCompilerLoad` aggregation. Assert that the per-tick aggregate `compile_opportunities` work stays bounded relative to total `agent_tick` time. Use the existing per-tick timer if available; otherwise count expanded-entity per-tick work as a proxy.

The exact metric expression depends on the timing infrastructure — confirm during implementation. If wall-clock is non-deterministic, prefer a deterministic proxy (e.g., `OpportunityCompilerLoad.compiled_count ≤ S105_perception_budget × N_agents`).

## Files to Touch

- `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` (new — 5 scenario tests)
- `crates/worldwake-ai/tests/regression_survival_baseline_s138.rs` (new — equality test)
- `crates/worldwake-ai/tests/perf_opportunity_compiler.rs` (new — soak test)
- `crates/worldwake-ai/tests/fixtures/survival-baseline-pre-s138.bin` (new — checked-in baseline event log)
- Likely: `scenarios/s138/scenario_1.ron` through `scenarios/s138/scenario_5.ron` (new — scenario RON files, if not inlined in the test file)

## Out of Scope

- Engine modifications — this is tests-only
- Tuning default profile values — defaults remain as landed in `archive/tickets/S138OPPCOM-002.md` and `archive/tickets/S138OPPCOM-003.md`
- New action types — none introduced
- HTN methods over opportunities — spec Non-Goal

## Acceptance Criteria

### Tests That Must Pass

1. Scenario 1: agent with high `theft_aversion` chooses Buy when merchant present; agent with low `theft_aversion` and absent merchant chooses Steal; ranking visible in decision trace
2. Scenario 2: detour to alternative source occurs within `detour_budget_permille = 150`
3. Scenario 3: `AskWitness` started within budget; if budget halved, the inquiry is not taken
4. Scenario 4: no opportunity emitted for the unknown-effect entity; `OpportunityCompilerLoad.compiled_count == 0`
5. Scenario 5: salience declines monotonically across 3 successive ticks of repeated perception
6. Regression: event log on `survival-baseline.ron` byte-identical to pre-S138 baseline
7. Performance: per-tick opportunity compilation work stays under the asserted bound on `survival-contested.ron`
8. Existing 1440-tick goldens (`survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron`) continue to pass
9. Existing suite: `cargo test --workspace`

### Invariants

1. Bottom-up opportunity emission is additive at default profiles — `survival-baseline.ron` event log unchanged
2. Each scenario isolates one causal branch (precision-rules.md §8) — competing affordances are explicitly excluded from scenario setup
3. Performance soak is deterministic — no wall-clock assertion, no float comparison, no `HashMap` iteration

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` — 5 scenario tests per Acceptance Criteria
2. `crates/worldwake-ai/tests/regression_survival_baseline_s138.rs` — pre/post-S138 event-log equality
3. `crates/worldwake-ai/tests/perf_opportunity_compiler.rs` — deterministic bound on per-tick compile work

### Commands

1. `cargo test -p worldwake-ai golden_opportunity_compiler`
2. `cargo test -p worldwake-ai regression_survival_baseline_s138`
3. `cargo test -p worldwake-ai perf_opportunity_compiler`
4. `cargo test -p worldwake-ai` (full ai crate including pre-existing goldens)
5. `cargo test --workspace`
6. `./scripts/verify.sh`
