# S138OPPCOM-006: Opportunity compiler core and agent_tick integration

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — adds per-tick compile_opportunities pass before candidate generation; populates `RootCandidateTrace.source` and records `OpportunityCompilerLoad` per-agent per-tick
**Deps**: archive/tickets/S138OPPCOM-001.md, archive/tickets/S138OPPCOM-002.md, archive/tickets/S138OPPCOM-003.md, archive/tickets/S138OPPCOM-004.md, archive/tickets/S138OPPCOM-005.md

## Problem

The cornerstone deliverable: implement `compile_opportunities(agent, belief_view, action_index) -> Vec<Opportunity>`, build the per-tick `PerceivedOpportunityIndex`, insert the call into `agent_tick/observation.rs` immediately before `generate_candidates_with_*` at line 273, and route opportunities into `candidate_generation.rs` as a parallel input alongside the existing emitters. When candidate generation emits a goal whose binding originated from an opportunity (e.g., the agent perceives unguarded bread and a `Steal`-routed `AcquireCommodity` candidate is produced), the `RootCandidateTrace.source` field is set to `CandidateSource::OpportunityCompiler`; otherwise it remains `CandidateSource::Emitter`. Per-agent per-tick `OpportunityCompilerLoad` is recorded on the decision-trace sink.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage:
   - `candidate_generation.rs:250` — `generate_candidates` is the public entrypoint consumed by `agent_tick/observation.rs:273`
   - `agent_tick/observation.rs:273` — currently invokes `generate_candidates_with_current_plan_with_memories_with_travel_horizon(...)`; the compile pass inserts immediately before this call
   - `decision_trace.rs:1310` — `DecisionTraceSink` accepts new `OpportunityCompilerLoad` entries via the existing per-agent per-tick recording surface
   - `ranking.rs:59` — ranking consumes `GoalOffer` from candidate generation; opportunity-derived candidates flow through the same pipeline
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` deliverable section "`worldwake-ai::opportunity_compiler` (new module)"; the spec's `compile_opportunities` signature was updated to drop `PerceptionState` (does not exist in the codebase) and use `&impl RuntimeBeliefView`.
3. Shared abstraction boundary: `compile_opportunities` reads only via the agent's `RuntimeBeliefView` (FND-7 locality preserved) plus the registry-time `EffectSchemaIndex`. No cross-system imperative.
4. Mixed-layer ticket: candidate generation (ai), decision-trace surface (ai), authoritative read of registry (sim) — bounded inside the ai crate at the consumption surface.
5. Authoritative-to-AI Impact Rule (AGENTS.md): this ticket modifies candidate emission. 7-point check applies:
   1. `get_affordances` — unaffected (compile_opportunities reads beliefs, not affordances)
   2. `generate_candidates` — extended to consume opportunities; new goal kinds are NOT emitted (FND-3, S138 Non-Goals); existing kinds gain opportunity-derived bindings
   3. `search_plan` — unaffected (planner consumes ranked candidates as before)
   4. `BestEffort` action start — unaffected
   5. `handle_plan_failure` — unaffected
   6. Payload revalidation — N/A (no new action introduced)
   7. Golden tests — exercised by ticket 010
6. Per-agent profile reads: the compiler consults `RiskWeightProfile` and `LawAbidingProfile` via the `GoalBeliefView::risk_weight_profile` and `law_abiding_profile` accessors added in `archive/tickets/S138OPPCOM-002.md`.
7. Per-tick budget: result length bounded by `CognitiveProfile.compile_opportunity_cap` (archive/tickets/S138OPPCOM-003.md); salience floor enforced via `PerceptionProfile.opportunity_floor_permille` (archive/tickets/S138OPPCOM-003.md).
8. Effect-schema index handoff: `archive/tickets/S138OPPCOM-005.md` lands a payload-free `EffectFactKey` category index over `EffectStep` declarations. This ticket owns the next binding layer: combine category lookups such as `EffectFactKey::CommodityTransfer` with the agent's perceived/believed commodity, target, legal-status, and required-action evidence before emitting concrete opportunities.

## Architecture Check

1. The compile pass runs inside the observation phase before candidate generation — the ordering is mandated by the spec (line 156 SystemFn Integration) and matches the perception-then-deliberation flow already in `agent_tick/observation.rs`.
2. Opportunities feed candidate generation as a parallel input alongside the existing 52 `emit_*` functions — no `emit_*` is removed or restructured (FND-28 no double-truth: opportunity-derived candidates are sourced from the new pass; emitter-derived candidates are sourced from `relevant_ops` hints; both produce `GoalOffer` records that the unified ranking pipeline consumes).
3. The `RootCandidateTrace.source` attribution makes the dual-source decomposition inspectable (FND-29 debuggability) — observer ticket 009 will surface this.
4. Bounded compute: the per-tick cap (`compile_opportunity_cap`) is the workspace-native substitute for the originally-specced `SmallVec<_, 16>` inline size. Per-agent budget governs.
5. No backward-compatibility shim: the integration is additive; existing emitter behavior is preserved at default profiles, validated by the regression assertion in ticket 010.

## Verification Layers

1. `compile_opportunities` produces the expected `Opportunity` list given a constructed belief view + perceived entities — focused unit test in `opportunity_compiler/compile.rs`
2. Result truncation at `CognitiveProfile.compile_opportunity_cap` — focused unit test exercising the cap with > N entities
3. Salience floor filtering via `PerceptionProfile.opportunity_floor_permille` — focused unit test asserting below-floor entries are not emitted
4. `LearnedOpportunityMemory` damping — focused unit test (low-yield opportunity gets damped after repeated emission)
5. `agent_tick/observation.rs:273` integration — runtime trace coverage: a focused integration test confirms `compile_opportunities` runs once per agent per tick before `generate_candidates`
6. `RootCandidateTrace.source` attribution: opportunity-derived candidates land with `source: CandidateSource::OpportunityCompiler`; emitter-derived candidates land with `source: CandidateSource::Emitter` — runtime trace coverage in `decision_trace.rs`
7. `OpportunityCompilerLoad` recorded per-agent per-tick — focused unit test on the trace sink

## What to Change

### 1. Implement `compile_opportunities`

In `crates/worldwake-ai/src/opportunity_compiler/`, add `compile.rs`:

```rust
pub fn compile_opportunities(
    agent: EntityId,
    belief_view: &impl RuntimeBeliefView,
    action_index: &EffectSchemaIndex,
) -> (Vec<Opportunity>, OpportunityCompilerLoad) {
    // Implementation: iterate perceived entities (belief_view.entities_at(current_place)
    // + perceived records via belief_view.agent_belief_store(agent)), derive possible
    // effects per entity via action_index.actions_producing(...), score salience using
    // RiskWeightProfile / LawAbidingProfile, filter below salience floor, dampen via
    // LearnedOpportunityMemory, truncate to CognitiveProfile.compile_opportunity_cap.
    // Return (opportunities, load_counter).
}
```

Implementation details:
- Read `belief_view.risk_weight_profile(agent)` and `belief_view.law_abiding_profile(agent)` (accessors from `archive/tickets/S138OPPCOM-002.md`)
- Read `belief_view.learned_opportunity_memory(agent)` to dampen repeated emissions
- Read `belief_view.survey_memory(agent)` to skip opportunities anchored on confirmed-empty places (spec line 28)
- Honor `PerceptionProfile.opportunity_floor_permille` as the salience-emission gate
- Honor `CognitiveProfile.compile_opportunity_cap` as the result-length cap; track truncation in `OpportunityCompilerLoad.cap_truncated`
- Track per-stage counters in `OpportunityCompilerLoad`: `compiled_count`, `salience_floored`, `learned_memory_damped`, `cap_truncated`

### 2. Build `PerceivedOpportunityIndex`

In the same module, add:

```rust
pub fn build_perceived_opportunity_index(opportunities: Vec<Opportunity>) -> PerceivedOpportunityIndex {
    // Group by anchor entity (via Opportunity.key.anchor) and by perceived place.
    // Populate `by_place`, `by_anchor`, `all`. Handles assigned dense indices into `all`.
}
```

### 3. Integrate into `agent_tick/observation.rs`

Modify `crates/worldwake-ai/src/agent_tick/observation.rs:273`:

Immediately before the existing `generate_candidates_with_*` call:

```rust
let (opportunities, load) = compile_opportunities(
    agent,
    &belief_view,
    driver.effect_schema_index(),
);
let opportunity_index = build_perceived_opportunity_index(opportunities.clone());
decision_trace_sink.record_opportunity_compiler_load(agent, current_tick, load);
// Pass `opportunities` and `opportunity_index` into generate_candidates_with_*
```

Thread `opportunities` and `opportunity_index` through the existing call signature. `opportunity_index` is consumed by ticket 007 (travel pruning) and ticket 008 (interrupts); the `Vec<Opportunity>` itself is consumed by candidate generation here.

### 4. Extend candidate generation to consume opportunities

Modify `crates/worldwake-ai/src/candidate_generation.rs`:

Update `generate_candidates` (line 250) to accept `&[Opportunity]` as an additional input. Iterate the opportunities, emit `GoalOffer`s for each opportunity whose `required_actions` and `legal_status` resolve to viable bindings, and tag the resulting `RootCandidateTrace.source` with `CandidateSource::OpportunityCompiler`. Emitter-sourced candidates continue to tag `CandidateSource::Emitter`.

When a goal's existing `relevant_ops` hint set is exhausted AND `urgency_class >= GoalPriorityClass::HighPriority` AND `relevant_ops_authority()` returns `HintOnly` (ticket 004), query `action_index.actions_producing(...)` for additional category-matched action defs. The category lookup is not payload-specific; filter and bind commodity/entity/legal relevance from the agent's accessible belief view before emitting a candidate.

### 5. Record `OpportunityCompilerLoad` on decision-trace sink

Modify `crates/worldwake-ai/src/decision_trace.rs` (`DecisionTraceSink` at line 1310): add a `record_opportunity_compiler_load(agent, tick, load: OpportunityCompilerLoad)` method that stores the per-tick counter alongside existing per-agent trace data.

## Files to Touch

- `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (new)
- `crates/worldwake-ai/src/opportunity_compiler/mod.rs` (modify — re-export `compile_opportunities`, `build_perceived_opportunity_index`)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — insert compile pass at line 273 + thread results into generate_candidates_with_*)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — accept `&[Opportunity]`; tag `source` field; consult `EffectSchemaIndex` when hint exhausted)
- `crates/worldwake-ai/src/decision_trace.rs` (modify — add `record_opportunity_compiler_load` method)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — pass `driver.effect_schema_index()` and trace sink to observation phase)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — add integration coverage)

## Out of Scope

- Travel pruning consumption of `PerceivedOpportunityIndex` — lands in ticket 007
- Interrupt-layer consumption — lands in ticket 008
- Observer rendering — lands in ticket 009
- Golden coverage — lands in ticket 010
- New `GoalKind` variants — spec Non-Goal
- HTN methods over opportunities — spec Non-Goal (Phase 12)

## Acceptance Criteria

### Tests That Must Pass

1. New test: `compile_opportunities` over a constructed belief view + 3 perceived entities produces 3 opportunities with expected salience values
2. New test: truncation at `compile_opportunity_cap = 2` with 5 candidate entities returns 2 highest-salience entries, with `OpportunityCompilerLoad.cap_truncated = 3`
3. New test: salience floor filters below-floor entries; `OpportunityCompilerLoad.salience_floored` reflects the count
4. New test: `LearnedOpportunityMemory` damping reduces salience for opportunities the agent has previously seen and not pursued
5. New test: `RootCandidateTrace.source = CandidateSource::OpportunityCompiler` for opportunity-derived candidates emitted in `candidate_generation.rs`
6. New integration test in `agent_tick/tests.rs`: a perceived bread + starving agent scenario produces opportunity-derived `AcquireCommodity` candidate
7. Existing 1440-tick goldens (`survival-baseline.ron`, `survival-scattered.ron`, `survival-contested.ron`) continue to pass — opportunities are additive at default profiles (ticket 010 enforces the strict regression)
8. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `compile_opportunities` reads only the agent's belief view + the registry-time `EffectSchemaIndex` (FND-7 locality preserved)
2. `EffectSchemaIndex` supplies payload-free effect categories only; `compile_opportunities` supplies commodity/entity/legal binding from belief-local evidence
3. No new `GoalKind` variant is introduced (spec Non-Goal)
4. `RootCandidateTrace.source` is `OpportunityCompiler` only for candidates whose binding originated from a compiled opportunity; `Emitter` for everything else
5. The compile pass runs synchronously within `agent_tick`, exactly once per agent per tick, immediately before `generate_candidates` (FND-29A determinism preserved)
6. Per-agent `compile_opportunity_cap` and `opportunity_floor_permille` govern truncation and filtering — no hard-coded constants

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/opportunity_compiler/compile.rs` (inline `#[cfg(test)]`) — unit tests for compile + truncation + floor + damping
2. `crates/worldwake-ai/src/candidate_generation.rs` (inline `#[cfg(test)]`) — opportunity-derived emission tags `CandidateSource::OpportunityCompiler`
3. `crates/worldwake-ai/src/agent_tick/tests.rs` — end-to-end agent_tick integration test
4. `crates/worldwake-ai/src/decision_trace.rs` (inline `#[cfg(test)]`) — `record_opportunity_compiler_load` round-trip on trace sink

### Commands

1. `cargo test -p worldwake-ai opportunity_compiler candidate_generation agent_tick`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
