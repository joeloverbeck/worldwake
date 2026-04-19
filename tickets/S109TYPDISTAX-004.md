# S109TYPDISTAX-004: Migrate Unknown/AssumptionFailed emission via classify_discrepancy

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — new `classify_discrepancy` entry point + `FailureClassification` enum; rewritten `derive_blocking_fact`; migration of emission sites and reader call sites across `worldwake-ai`
**Deps**: S109TYPDISTAX-001, S109TYPDISTAX-002, S109TYPDISTAX-003

## Problem

S109's semantic core: every current emission of `BlockingFact::Unknown` and `BlockingFact::AssumptionFailed` must route through a new `classify_discrepancy` function that returns a typed `FailureClassification` — either a surviving `BlockingFact` written to `BlockerMemory`, or a `Discrepancy` variant written to the new `DiscrepancyMemory`. Reader sites (candidate generation, search, revalidation) must consult both memories when deciding whether a goal is suppressed.

The two variants stay in the `BlockingFact` enum during this ticket (removed in T006) so test sites that still reference them continue to compile. The migration changes which memory entries are written, not which types exist.

## Assumption Reassessment (2026-04-19)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. Runtime emission sites of `BlockingFact::Unknown` / `AssumptionFailed` (verified 2026-04-19):
   - `crates/worldwake-ai/src/failure_handling.rs:66` — `diagnostic_context` filter on `Unknown` inside `record_blocked_intent`.
   - `failure_handling.rs:177` — default fallthrough at end of `derive_blocking_fact` returning `BlockingFact::Unknown`.
   - `failure_handling.rs:568` — `classify_precondition_failure_detail` returns `AssumptionFailed` for `targetatactorplace`/`targetdirectlypossessedbyactor`/`targetgrounded`/`exactidentityrequired` precondition strings.
   - `failure_handling.rs:590` — `map_handler_abort_reason` `SelfTargetForbidden`/`TargetLacksWounds`/`TargetHasNoWounds` arm returns `Unknown`.
   - `failure_handling.rs:745–748` — `derive_clearing_condition` `Unknown | PatienceExhausted | AssumptionFailed | NoBuyer` branch returns `BlockerClearingCondition::TtlOnly`.
   - `failure_handling.rs:999` — `blocking_fact_ttl` maps `Unknown => unknown_block_ticks`.
   - `failure_handling.rs:1009` — `blocking_fact_ttl` maps `AssumptionFailed` into the `structural_block_ticks` bucket.
   - `crates/worldwake-ai/src/agent_tick/frame.rs:435` — `record_assumption_failure_blocked_intent` emits `AssumptionFailed`.
   - `crates/worldwake-ai/src/agent_tick/mod.rs:837–852` — filter populating `PlanningPipelineTrace::unknown_blockers` reads `BlockingFact::Unknown` entries from `BlockerMemory`.
   - `crates/worldwake-ai/src/decision_trace.rs:244, 276` — `unknown_blockers` field doc comment + `UnknownBlockerTrace` struct (handled by T005; this ticket leaves the struct in place for now).
   Existing focused tests at `failure_handling.rs:2517–2543` (`blocking_fact_ttl_uses_budget_classification`, `unknown_blocker_uses_dedicated_ttl`, `transient_blockers_unchanged_ttl`) plus tests at `failure_handling.rs:1673, 1804, 1812, 2094, 2455, 2604, 2678` (test-module boundary at line 1014) assert `BlockingFact::Unknown`/`AssumptionFailed` today — they continue to compile through this ticket because the variants remain; they are updated in T006 when the variants are removed.
   Existing tests at `agent_tick/tests.rs:4124, 6006, 6036` reference `BlockingFact::Unknown`/`AssumptionFailed` — same treatment. Tests at `candidate_generation.rs:8005, 15412, 16170` (test-boundary at 5200) and `search/tests.rs:2323, 2340` — same.
2. `RequestResolutionRejectionReason::ExactIdentityRequired` lives at `crates/worldwake-sim/src/request_resolution_trace.rs:60` and is matched in `crates/worldwake-sim/src/tick_step.rs:289` where BestEffort failures classify as `"ExactIdentityRequired"`. Today, the precondition-detail string `exactidentityrequired` flows through `classify_precondition_failure_detail` at `failure_handling.rs:561–572` and returns `BlockingFact::AssumptionFailed`. After this ticket, that path returns `Discrepancy::NoLegalBinding`. Spec D5 covers this.
3. Shared abstraction boundary: the `derive_blocking_fact` function at `failure_handling.rs:98` is the single classification surface called by `record_blocked_intent` (`failure_handling.rs:28–86`). The new `classify_discrepancy` returning `FailureClassification { Blocker(BlockingFact) | Discrepancy(Discrepancy) }` replaces it at that call site. `record_blocked_intent` branches on the classification and writes to `BlockerMemory` or `DiscrepancyMemory` accordingly. Post-T001, all blocker-memory type and accessor names in this ticket refer to `BlockerMemory` / `get_component_blocker_memory`, not the removed `BlockedIntentMemory` names.
6. AI regression scope: this ticket is a candidate-generation + runtime `agent_tick` migration, not a golden-only change. The runtime harness requires full action registries because `failure_handling.rs` and `agent_tick/frame.rs` are exercised during real plan-failure flows. Verification is candidate-generation focused/unit coverage (new emission tests in `failure_handling.rs#[cfg(test)]`) + runtime `agent_tick` decision-trace coverage (extended tests at `agent_tick/tests.rs`).
8. Heuristic removal discipline: `BlockingFact::Unknown` is not a heuristic being removed — it is being replaced by typed classification. The underlying "why did this step fail?" inference logic is preserved and refined, not bypassed. Each `Unknown` call site maps to a specific `Discrepancy` variant per spec D5. No regression surface reopens because the information content is strictly preserved.
9. Stale-request / start-failure boundary: S108's `ExactIdentityRequired` is a start-time request-resolution failure (proof surface: focused runtime request-resolution coverage at `tick_step.rs:289`). After this ticket, that failure reaches AI recovery through `classify_discrepancy` and lands in `DiscrepancyMemory` as `Discrepancy::NoLegalBinding`. Proof surface for AI recovery: decision trace via `DiscrepancyTrace` (T005) + focused test at `failure_handling.rs#[cfg(test)]`.
13. Adjacent contradiction: `agent_tick/mod.rs:837–852` populates `PlanningPipelineTrace::unknown_blockers` from `BlockerMemory` entries matching `BlockingFact::Unknown`. After this ticket, `Unknown` entries are no longer written to `BlockerMemory` — they go to `DiscrepancyMemory`. The filter would return an empty list unless rewritten. Classified as a required consequence of this ticket: the filter is rewritten to read `DiscrepancyMemory` during this ticket, even though the struct replacement (`UnknownBlockerTrace` → `DiscrepancyTrace`) is T005's scope. Interim state (T004 lands, T005 not yet): `unknown_blockers` still exists as a field but is populated from discrepancy entries. T005 then renames the field and struct. Alternatively, this ticket leaves `unknown_blockers` as a `Vec<UnknownBlockerTrace>` populated with empty data during the single-ticket gap, relying on T005 to land immediately after. Decision: the filter is rewritten here to populate `unknown_blockers` by mapping `DiscrepancyMemory` entries to `UnknownBlockerTrace` (single-field struct adapter) so observer tooling continues to see failure data during the T004→T005 gap. T005 then performs the renames.

## Architecture Check

1. A single `classify_discrepancy` entry point with an exhaustive match on `Discrepancy` is the right shape because it gives us compile-time proof that every failure classification ends up at a specific class — no default fallthrough, no silent fallback to a catch-all. The old `derive_blocking_fact` returned a single type (`BlockingFact`) which hid the discrepancy vs. blocker distinction; the new `FailureClassification` enum makes the split explicit. This is cleaner than reusing `BlockingFact` with a new variant for every discrepancy (which would blur the epistemic vs. world-state distinction that S109's whole design turns on).
2. No backwards-compatibility aliasing. `derive_blocking_fact` is replaced by `classify_discrepancy`; no wrapper function preserves the old signature. Callers are updated in-scope. FND-28 compliant.

## Verification Layers

1. Each runtime emission site that previously produced `BlockingFact::Unknown` now produces a specific `FailureClassification` → focused unit test per site in `failure_handling.rs#[cfg(test)]`. Proof surface: focused/unit coverage at the classification boundary.
2. `classify_discrepancy` exhaustive match → compile-time proof (Validation test 7); no runtime coverage needed because the exhaustive match is enforced by rustc.
3. Writer path: `record_blocked_intent` branches to `BlockerMemory` or `DiscrepancyMemory` based on `FailureClassification` → focused runtime test at `failure_handling.rs#[cfg(test)]` asserting the correct memory receives the entry per classification.
4. Candidate generation suppression reads both memories → focused runtime coverage at `candidate_generation.rs#[cfg(test)]`: construct a world where `DiscrepancyMemory` has a live entry for a goal and assert the candidate is suppressed; similarly for `BlockerMemory`.
5. Search suppression → focused coverage at `search/candidates.rs#[cfg(test)]`: `find_blocked_for_search` still reads `BlockerMemory` only (spec D5 reader migration). Interior `DiscrepancyMemory` reads happen at candidate generation, before search.
6. AI recovery after `ExactIdentityRequired` → decision-trace coverage (via extended `DiscrepancyTrace` surface from T005, or interim `UnknownBlockerTrace` adapter in this ticket).

## What to Change

### 1. Introduce `FailureClassification` and `classify_discrepancy`

In `crates/worldwake-ai/src/failure_handling.rs`, alongside existing helpers:

```rust
pub enum FailureClassification {
    Blocker(BlockingFact),
    Discrepancy(Discrepancy),
}

fn classify_discrepancy(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    goal_key: &GoalKey,
    step: &PlannedStep,
    execution_failure: Option<ExecutionFailure<'_>>,
) -> FailureClassification {
    // 1. Target-gone / path-unknown / production / trade / combat routing unchanged.
    //    Reuse existing helpers (target_gone, no_known_path, classify_trade_failure,
    //    classify_production_failure, classify_input_failure, combat_too_risky,
    //    danger_too_high). These all return a specific BlockingFact → wrap as
    //    FailureClassification::Blocker.
    // 2. map_execution_failure now returns Option<FailureClassification> instead of
    //    Option<BlockingFact>. The SelfTargetForbidden / TargetLacksWounds /
    //    TargetHasNoWounds arm returns Discrepancy::NoLegalBinding.
    // 3. classify_precondition_failure_detail now returns Option<FailureClassification>.
    //    The exactidentityrequired string returns Discrepancy::NoLegalBinding; other
    //    strings (targetatactorplace, targetdirectlypossessedbyactor, targetgrounded)
    //    return Discrepancy::ImproperPlanningState.
    // 4. Final fallthrough (previously returning BlockingFact::Unknown) returns
    //    FailureClassification::Discrepancy(Discrepancy::ImproperPlanningState) —
    //    the agent is in a state from which no further legal step can be classified,
    //    which is the spec's definition of ImproperPlanningState.
}
```

Update signatures of `map_execution_failure`, `map_handler_abort_reason`, `map_start_failure_reason`, `classify_precondition_failure_detail`, `parse_abort_detail` to return `Option<FailureClassification>` and rewrite their arms per spec D5. The old arms returning `BlockingFact::AssumptionFailed` (currently at `failure_handling.rs:568`) now return `FailureClassification::Discrepancy(Discrepancy::NoLegalBinding)` for `exactidentityrequired` and `FailureClassification::Discrepancy(Discrepancy::ImproperPlanningState)` for the other precondition-assertion strings.

Remove `derive_blocking_fact`.

### 2. Rewrite `record_blocked_intent` to branch on classification

In `failure_handling.rs:28–86`, `record_blocked_intent` becomes the entry point that calls `classify_discrepancy` and writes to `BlockerMemory` or `DiscrepancyMemory` based on the result:

```rust
pub fn record_failure(
    context: FailureContext<'_>,
    blocker_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    runtime: &mut AgentDecisionRuntime,
) {
    let classification = classify_discrepancy(/* ... */);
    let cognitive = /* ... */;
    match classification {
        FailureClassification::Blocker(fact) => {
            let expires = context.current_tick + u64::from(blocking_fact_ttl(fact, cognitive));
            let (clearing, baseline) = derive_clearing_condition(/* ... */);
            blocker_memory.record(Blocker { /* ... */ });
        }
        FailureClassification::Discrepancy(disc) => {
            let expires = context.current_tick + u64::from(discrepancy_ttl(disc, cognitive));
            let clearing = derive_discrepancy_clearing(disc, step);
            discrepancy_memory.record(DiscrepancyEntry { /* ... */ });
        }
    }
    runtime.dirty.insert(DirtySet::REPLAN_SIGNAL);
}
```

Rename the function from `record_blocked_intent` to `record_failure` (or similar) to reflect its broader role. Introduce a small helper `derive_discrepancy_clearing(disc, step) -> DiscrepancyClearing` that selects `TtlExpiry` by default, `ReobservationOf { target }` when the step has a target, and `BeliefUpdate { claim_key }` when the discrepancy is `BeliefContradicted` with an identifiable belief aspect.

Delete the `diagnostic_context` Unknown filter at line 66 — `BlockerDiagnostic` is now only attached to `BlockerMemory` entries; `DiscrepancyMemory` carries its action-def context through `blocker_key.action_def` already.

Remove the line 745–748 `Unknown | AssumptionFailed` arm from `derive_clearing_condition` — those variants no longer reach that function because they're routed to `DiscrepancyMemory` instead. Keep `PatienceExhausted` and `NoBuyer` there (they remain on `BlockerMemory`).

### 3. Update `clear_resolved_blockers` and add `clear_resolved_discrepancies`

In `failure_handling.rs:88–96`, update the sweep to handle both memories. Current:

```rust
pub fn clear_resolved_blockers(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocker_memory: &mut BlockerMemory,  // renamed by T001
    current_tick: Tick,
)
```

Extend to:

```rust
pub fn clear_resolved_failures(
    view: &dyn RuntimeBeliefView,
    agent: EntityId,
    blocker_memory: &mut BlockerMemory,
    discrepancy_memory: &mut DiscrepancyMemory,
    current_tick: Tick,
) {
    blocker_memory.expire(current_tick);
    blocker_memory.sweep_cleared(|blocker| is_blocker_cleared(view, agent, blocker));
    discrepancy_memory.expire(current_tick);
    discrepancy_memory.sweep_cleared(|entry| is_discrepancy_cleared(view, agent, entry));
}
```

Add `is_discrepancy_cleared` that matches on `DiscrepancyClearing` variants:

- `TtlExpiry` → never sweeps (only `expire` clears it).
- `ReobservationOf { target }` → cleared when the perception layer has recorded a new observation of `target` after `observed_tick`.
- `BeliefUpdate { claim_key }` → cleared when the agent's `AgentBeliefStore` has a newer claim matching `claim_key`.
- `WorldStructureChange` → deferred; always returns `false` for now (T007 or later spec can wire specific signals).

Update all call sites of `clear_resolved_blockers` to the new signature.

### 4. Migrate `agent_tick/frame.rs:435`

Rewrite `record_assumption_failure_blocked_intent` to consult `classify_discrepancy` and write to the correct memory. For frame assumption failures, the routing is deterministic from the frame's break reason:

- Target-gone-style breaks → `BlockerMemory` with `BlockingFact::TargetGone`.
- Belief-contradicted breaks (identity mismatch, claim staleness) → `DiscrepancyMemory` with `Discrepancy::BeliefContradicted`.
- Partial-execution breaks → `DiscrepancyMemory` with `Discrepancy::PartialExecutionDrift`.

Rename the function to `record_assumption_failure` (drops the `_blocked_intent` suffix since it no longer always writes to `BlockerMemory`). Update callers in `agent_tick/frame.rs` and related tests.

### 5. Migrate candidate-generation readers

In `crates/worldwake-ai/src/candidate_generation.rs:2030, 2264` (`ctx.blocked.is_blocked(...)`):

- Update `CandidateContext` (or equivalent) to carry both `&BlockerMemory` and `&DiscrepancyMemory` via the new belief-view accessors from T003.
- Replace `ctx.blocked.is_blocked(...)` with a helper `is_goal_suppressed(ctx, goal_key, place, target, action_def, tick)` that returns `true` if either `ctx.blocker.is_blocked(...)` or `ctx.discrepancy.is_suppressed(...)` returns `true`.

In `crates/worldwake-ai/src/search/candidates.rs:1154` (`blocked.find_blocked_for_search(...)`):

- Keep reading `BlockerMemory` only (per spec D5 — search does not consult `DiscrepancyMemory`; that filter runs at candidate-generation time before search). Rename the local variable from `blocked` to `blocker_memory` for clarity.

### 6. Migrate `agent_tick/mod.rs:837–852` filter (interim)

For this ticket, rewrite the filter so it populates `unknown_blockers: Vec<UnknownBlockerTrace>` from `DiscrepancyMemory` entries:

```rust
unknown_blockers: discrepancy_memory
    .entries
    .values()
    .filter(|e| e.expires_tick > tick)
    .map(|e| UnknownBlockerTrace {
        goal_key: e.blocker_key.goal_key,
        failed_action_def: e.blocker_key.action_def.unwrap_or(ActionDefId(0)),
        op_kind: /* lookup or default */,
        target: e.blocker_key.target,
        place: e.blocker_key.place,
    })
    .collect(),
```

This is an interim adapter. T005 renames the field and struct. The purpose of keeping the name during T004 is to preserve observer tooling continuity.

### 7. Test updates

Update existing tests at `failure_handling.rs:2455, 2604` (`handles_unknown_blockers` and similar) to reflect that `Unknown` classification now writes to `DiscrepancyMemory` as `Discrepancy::ImproperPlanningState`. Update `derive_blocking_fact_default_fallthrough_unknown`-style tests to assert against `classify_discrepancy` returning `FailureClassification::Discrepancy(Discrepancy::ImproperPlanningState)`.

Add new tests in `failure_handling.rs#[cfg(test)]`:

- `classify_discrepancy_maps_exact_identity_required_to_no_legal_binding` — feed a precondition-failure detail containing `exactidentityrequired`; assert `FailureClassification::Discrepancy(Discrepancy::NoLegalBinding)`.
- `classify_discrepancy_maps_self_target_forbidden_to_no_legal_binding` — feed `ActionAbortRequestReason::SelfTargetForbidden`; assert same.
- `classify_discrepancy_default_fallthrough_maps_to_improper_planning_state` — feed a failure with no specific classification; assert `ImproperPlanningState`.
- `classify_discrepancy_target_gone_stays_blocker` — feed a target-gone failure; assert `FailureClassification::Blocker(BlockingFact::TargetGone)`.
- `record_failure_writes_blocker_to_blocker_memory` — classification returns `Blocker`; entry lands in `BlockerMemory`, not `DiscrepancyMemory`.
- `record_failure_writes_discrepancy_to_discrepancy_memory` — classification returns `Discrepancy`; entry lands in `DiscrepancyMemory`, not `BlockerMemory`.

Add tests in `candidate_generation.rs#[cfg(test)]`:

- `generate_candidates_suppresses_goal_with_live_discrepancy_entry` — analogous to existing `BlockerMemory`-suppression tests; exercises the `DiscrepancyMemory` path.

## Files to Touch

- `crates/worldwake-ai/src/failure_handling.rs` (modify — add `FailureClassification`, `classify_discrepancy`, `derive_discrepancy_clearing`, `is_discrepancy_cleared`; rewrite `record_blocked_intent`→`record_failure` and `clear_resolved_blockers`→`clear_resolved_failures`; delete `derive_blocking_fact`; update internal helpers; new tests)
- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — `record_assumption_failure_blocked_intent` rewrite to route by classification)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — filter at lines 837–852; call sites of renamed recorders)
- `crates/worldwake-ai/src/agent_tick/active_action.rs` (modify — call sites)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify — call sites)
- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — call sites)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — call sites)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — call sites)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — reader migration at lines 2030, 2264)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — reader local-variable rename at line 1154)
- `crates/worldwake-ai/src/planning_snapshot.rs` (modify — call sites of memory accessors)
- `crates/worldwake-ai/src/feasibility.rs` (modify — call sites of memory accessors)
- `crates/worldwake-ai/src/plan_revalidation.rs` (modify — new read of `DiscrepancyMemory` during revalidation classification, per S109 Cross-System Interactions)

## Out of Scope

- Replacement of `UnknownBlockerTrace` with `DiscrepancyTrace` (T005). This ticket populates `unknown_blockers` from `DiscrepancyMemory` as an interim adapter; T005 does the rename.
- Removal of `BlockingFact::Unknown` and `AssumptionFailed` variants (T006).
- Removal of `CognitiveProfile::unknown_block_ticks` (T006).
- Scenario RON changes (T006).
- Golden test extension — test 9 from the spec's Validation section lands in T006.
- Changes to `BlockerClearingCondition`, `ClearingBaseline`, `BlockerDiagnostic`, `blocks_goal_generation`.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai failure_handling` — all classification and recording tests pass.
2. `cargo test -p worldwake-ai candidate_generation` — suppression tests for both `BlockerMemory` and `DiscrepancyMemory` paths.
3. `cargo test -p worldwake-ai agent_tick` — frame assumption-failure routing tests.
4. Existing focused tests continue to pass: `blocking_fact_ttl_uses_budget_classification`, `unknown_blocker_uses_dedicated_ttl`, `transient_blockers_unchanged_ttl`. (The `Unknown`/`AssumptionFailed` variants still exist; the TTL function still handles them; the emission sites just no longer feed them. These tests exercise the TTL function directly and are unchanged.)
5. Existing golden suite: `cargo test -p worldwake-ai golden` — no regression. Any assertion in existing goldens that relied on `BlockingFact::Unknown`/`AssumptionFailed` appearing in `BlockerMemory` is updated here to match the new routing.
6. Full workspace: `cargo test --workspace`.

### Invariants

1. No runtime path emits `BlockingFact::Unknown` or `BlockingFact::AssumptionFailed` into `BlockerMemory`. Verified by grepping `failure_handling.rs` and `agent_tick/frame.rs` after the change: zero matches for `BlockingFact::Unknown |` or `blocking_fact: BlockingFact::Unknown` outside `#[cfg(test)]` blocks.
2. `classify_discrepancy` is exhaustive over `Discrepancy` (compile-time check via the match).
3. Every candidate-generation and search reader path that previously called `BlockedIntentMemory::is_blocked` / `find_blocked_for_search` now consults the new memories via the belief-view accessors from T003.
4. `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/failure_handling.rs` `#[cfg(test)]` — add six new classification/recording tests listed in Section 7 above. Update existing `handles_unknown_blockers` and similar tests to reflect the new routing.
2. `crates/worldwake-ai/src/candidate_generation.rs` `#[cfg(test)]` — add `generate_candidates_suppresses_goal_with_live_discrepancy_entry`.
3. `crates/worldwake-ai/src/agent_tick/frame.rs` `#[cfg(test)]` — add `record_assumption_failure_routes_belief_contradiction_to_discrepancy` and `record_assumption_failure_routes_target_gone_to_blocker`.
4. `crates/worldwake-ai/src/agent_tick/tests.rs` — update existing `record_blocked_intent`/`assumption_failed`-style tests to reflect the new routing; tests that assert specific `BlockingFact` variants end up in `BlockerMemory` are updated to check the appropriate memory based on expected classification.

### Commands

1. `cargo test -p worldwake-ai failure_handling`
2. `cargo test -p worldwake-ai candidate_generation`
3. `cargo test -p worldwake-ai agent_tick`
4. `cargo test -p worldwake-ai golden`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`
