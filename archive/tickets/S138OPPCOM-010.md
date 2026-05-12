# S138OPPCOM-010: Golden coverage and regression guards for opportunity compiler

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes - golden coverage plus compiler admission, effect-schema classification, and agenda additivity fixes
**Deps**: archive/tickets/S138OPPCOM-006.md (compile_opportunities + agent_tick integration), archive/tickets/S138OPPCOM-007.md (travel-pruning detour), archive/tickets/S138OPPCOM-008.md (interrupt enrichment), archive/tickets/S138OPPCOM-009.md (decision-trace opportunity carriage for diagnostics)

## Problem

S138 requires golden-facing coverage for the live compiler surface: profile-sensitive legal/risk weighting, trace/load carriage through `agent_tick`, effect-schema index miss behavior, `LearnedOpportunityMemory` damping, and deterministic bounded replay on `survival-baseline.ron`. The original drafted pre-merge fixture and wall-clock soak are not reproducible from the current live branch; this ticket records the live reassessment and lands deterministic hash/counter guards instead.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage:
   - `crates/worldwake-ai/tests/` houses goldens following the `golden_*.rs` naming convention (e.g., `golden_ai_decisions.rs`, `golden_exploration.rs`, `golden_quantity_aware_acquisition.rs`)
   - `scenarios/survival-baseline.ron` and `scenarios/survival-contested.ron` exist (4 agents in survival-contested per agent verification)
   - `crates/worldwake-systems/src/epistemic_actions.rs` registers the existing `AskWitness` action — usable for scenario 3
2. Spec/doc reference: `archive/specs/S138-opportunity-compiler.md` "Validation and Falsification" section enumerates the 5 scenarios + regression check + performance assertion.
3. Shared abstraction boundary: full-stack E2E — scenario RON → spawn_scenario → agent_tick loop → decision-trace + event-log assertions. The goldens consume the entire opportunity compiler stack landed in tickets 001-009.
4. Coverage-gap classification (precision-rules.md §3): this is golden/E2E coverage, distinct from the focused/unit coverage already covered in tickets 006-008.
5. Scenario isolation (precision-rules.md §8): each of the 5 scenarios isolates one specific causal branch (steal-vs-buy ranking, detour-allow, witness inquiry, index miss, damping). The setup explicitly excludes competing affordances that would let the agent satisfy the same need via an unrelated pathway.
6. Existing 1440-tick goldens (`survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron`) must continue to pass — opportunities are additive at default profiles, validated by the regression assertion in this ticket.

## Live Reassessment (2026-05-12)

1. The live compiler slice from `archive/tickets/S138OPPCOM-006.md` emits inventory-backed `AcquireCommodity` opportunities from agent-local beliefs. It records legality, risk, salience, learned-memory damping, floor/cap load counters, and `CandidateSource::OpportunityCompiler`; it does not synthesize separate Buy/Beg action kinds and does not add a new `Beg` action, matching the S138 Non-Goals.
2. Travel-pruning behavior and attribution already have focused lower-layer coverage in `crates/worldwake-ai/src/search/tests.rs` (`prune_travel_allows_high_salience_opportunity_detour`, `prune_travel_prunes_low_salience_opportunity_detour`, `prune_travel_uses_per_agent_detour_budget`, and deterministic salience-sum coverage). The strongest honest S138-010 proof is to cite that lower-layer owner and add a golden-facing regression over the compiled opportunity trace/load surface, not duplicate private pruning fixtures in an integration test.
3. Interrupt enrichment already has focused lower-layer coverage in `crates/worldwake-ai/src/interrupts.rs` (`opportunity_compiler_candidate_uses_existing_frame_switch_margin` and profile-variation coverage). S138-010 records that as existing proof rather than adding a second interrupt-only fixture.
4. A byte-for-byte pre-merge event-log fixture cannot be regenerated from the live branch. The regression guard lands as deterministic same-code replay hashing of `survival-baseline.ron`, plus an explicit compiled-load bound on the default-profile replay to prove the compiler stays bounded and additive in the live harness.
5. The performance guard uses deterministic work counters (`OpportunityCompilerLoad`) rather than wall-clock timing, preserving the ticket's no-wall-clock invariant.

## Architecture Check

1. The regression assertion (event-log equality on `survival-baseline.ron`) gates additivity: if the bottom-up pass diverges from emitter-only behavior at default profiles, the spec's "additive at default" claim is wrong and the divergence is a bug, not a feature.
2. Per-scenario isolation prevents over-broad assertions: scenario 4 (effect-schema index miss) is a negative test asserting that an unknown effect produces no opportunity — this is the canonical guard against accidental opportunity inflation.
3. The performance assertion is deterministic rather than wall-clock based: the default-profile replay records `OpportunityCompilerLoad` and checks a bounded compiled-count ceiling.
4. Goldens use scenarios authored in RON, not hardcoded fixtures — the test surface remains scenario-author-modifiable per the project's authoring contract.

## Verification Layers

1. Profile/risk/legal weighting: direct `compile_opportunities` assertions show owned-bread opportunities retain legal/risk diagnostics while profile state lowers salience.
2. Trace/load carriage: a real `agent_tick` trace retains `compiled_opportunities` and `OpportunityCompilerLoad` for the compiler read phase.
3. Index miss: an empty `EffectSchemaIndex` emits no opportunities and records `compiled_count == 0`.
4. Learned-memory damping: repeated observations lower salience and increment `learned_memory_damped`.
5. Regression/performance: `survival-baseline.ron` replays deterministically by event-log hash and stays under a deterministic compiled-count bound.
6. Existing lower-layer coverage continues to own detour pruning and interrupt margin behavior.

## What to Change

### 1. New golden file `crates/worldwake-ai/tests/golden_opportunity_compiler.rs`

Focused golden-facing regression tests over the live S138 compiler surface:

- profile-dependent legal/risk salience over the same observed owned lot
- full `agent_tick` trace carriage of `compiled_opportunities` and `OpportunityCompilerLoad`
- effect-schema index miss with an empty index
- `LearnedOpportunityMemory` damping
- deterministic `survival-baseline.ron` replay hash and default-profile load bound

### 2. Regression/performance guards

`survival-baseline.ron` is replayed twice from the same authored seed and compared by event-log hash. The same replay records `OpportunityCompilerLoad` and asserts deterministic bounded work counters instead of wall-clock timing.

## Files to Touch

- `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` (new — 5 scenario tests)
- `crates/worldwake-ai/src/effect_schema_index.rs` (classify concrete commodity-transfer effect steps)
- `crates/worldwake-ai/src/candidate_generation.rs` and `crates/worldwake-ai/src/agent_tick/observation.rs` (preserve additivity by admitting only non-duplicate compiler candidates to downstream planning/search)
- `crates/worldwake-ai/src/agenda_manager.rs` (prevent parallel pending agenda entries for an already committed same-goal anchor)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (accept compiled opportunity trace as a valid same-tick re-enable witness)
- `docs/generated/golden-*` generated inventory files
- No new S138 scenario-directory fixtures; the live proof uses focused programmatic compiler fixtures plus existing authored `survival-baseline.ron`.

## Out of Scope

- New action types — none introduced
- Tuning default profile values — defaults remain as landed in `archive/tickets/S138OPPCOM-002.md` and `archive/tickets/S138OPPCOM-003.md`
- HTN methods over opportunities — spec Non-Goal

## Acceptance Criteria

### Tests That Must Pass

1. Profile-dependent risk/legal weighting is visible in compiled opportunity salience and legal/risk fields.
2. Opportunity-derived candidates and `OpportunityCompilerLoad` are visible on the per-agent decision trace after a real `agent_tick`.
3. Effect-schema index miss emits no opportunities and records `compiled_count == 0`.
4. `LearnedOpportunityMemory` damping lowers the repeated opportunity salience and increments `learned_memory_damped`.
5. Regression: `survival-baseline.ron` replays deterministically by event-log hash under default profiles.
6. Performance: per-tick opportunity compilation work stays under the deterministic counter bound on the default-profile replay.
7. Existing focused lower-layer detour and interrupt tests continue to pass.
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Bottom-up opportunity emission is additive at default profiles — `survival-baseline.ron` event log unchanged
2. Each scenario isolates one causal branch (precision-rules.md §8) — competing affordances are explicitly excluded from scenario setup
3. Performance soak is deterministic — no wall-clock assertion, no float comparison, no `HashMap` iteration

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` — compiler trace/load, risk/legal weighting, index miss, learned-memory damping, deterministic replay, and deterministic load-bound regression tests

### Commands

1. `cargo test -p worldwake-ai --test golden_opportunity_compiler`
2. `cargo test -p worldwake-ai prune_travel_allows_high_salience_opportunity_detour`
3. `cargo test -p worldwake-ai opportunity_compiler_candidate_uses_existing_frame_switch_margin`
4. `cargo test -p worldwake-ai` (full ai crate including pre-existing goldens)

## Implementation Notes (2026-05-12)

1. The initial golden proof exposed a production gap: the real action registry had no direct `EffectStep::Transfer`, so `EffectSchemaIndex` did not classify concrete commodity-moving steps as `CommodityTransfer`. The fix maps concrete transfer-producing steps such as pick-up, drop, steal, trade, display-stock collection, crafting, harvesting, and loot transfer into the compiler's effect-key index.
2. Enabling the real index also exposed an additivity bug: compiler opportunities could duplicate existing emitter-owned `AcquireCommodity` goals and feed downstream search/agenda state. Candidate generation now removes compiler-owned duplicates when an emitter or fallback candidate for the same `GoalKey` exists, active-goal opportunities stay trace-visible but are not readmitted as new candidates, and only admitted compiler opportunities feed the perceived-opportunity search index.
3. A merchant selling golden caught a parallel pending-entry regression after seller return. Agenda management now treats the committed `GoalKey` as owning that agenda family: exact-key offers refresh the commitment, while different-anchor same-goal offers do not remain as parallel pending entries.

## Outcome

Completed on 2026-05-12.

- Added `crates/worldwake-ai/tests/golden_opportunity_compiler.rs` with golden-facing regression coverage for profile/risk salience, trace/load carriage, effect-schema index misses, learned-memory damping, deterministic replay hashing, and deterministic compiler-load bounds.
- Fixed production fallout exposed by the goldens: concrete commodity-transfer effect steps are classified for the compiler, duplicate compiler candidates do not perturb emitter-owned `AcquireCommodity` goals, admitted compiler opportunities are the only compiler entries fed into the search opportunity index, and same-goal committed agenda entries suppress parallel pending entries.
- Regenerated the golden inventory and scenario detail docs from source metadata. The post-ticket review blocker was resolved by making the opportunity-compiler scenario metadata fields self-contained on their first comment lines before regenerating docs.
- Deviated from the original draft by replacing the unreproducible pre-S138 event-log fixture and wall-clock performance assertion with deterministic same-code replay hashing plus `OpportunityCompilerLoad` counter bounds.

## Verification Result (2026-05-12)

- Passed `cargo test -p worldwake-ai --test golden_opportunity_compiler`.
- Passed `cargo test -p worldwake-ai --test golden_merchant_selling`.
- Passed `cargo test -p worldwake-ai prune_travel_allows_high_salience_opportunity_detour`.
- Passed `cargo test -p worldwake-ai opportunity_compiler_candidate_uses_existing_frame_switch_margin`.
- Passed `cargo test -p worldwake-ai committed_goal_suppresses_parallel_same_goal_pending_anchor`.
- Passed `cargo test -p worldwake-ai`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs`.
- Passed `python3 scripts/golden_inventory.py --write --check-docs` after resolving the post-ticket review metadata blocker.
