# S122FRAASSCOM-003: Evaluation arm + `CriticalFailure` payload widening + stub removal

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `evaluate_assumptions` signature widened to take `agent: EntityId`; `AssumptionEvalResult::CriticalFailure` widened from unit variant to `CriticalFailure(FrameAssumption)`; existing `TargetAlive` arm updated to pass payload; `apply_assumption_result`, `emit_assumption_transitions`, both call sites at `mod.rs:502/599`, and 6 affected test sites all updated; stub comment + always-true arm + `commodity_available_at_stubbed_as_pass` test removed.
**Deps**: archive/tickets/S122FRAASSCOM-001.md, tickets/S122FRAASSCOM-002.md

## Problem

With substrate (001) and population (002) in place, the assumption is being added to frames but never fails. This ticket replaces the always-true `CommodityAvailableAt` stub arm in `evaluate_assumptions` with the real evaluator from S122FRAASSCOM-001's helper, widens `CriticalFailure` to carry the failed `FrameAssumption` so the trace surface (D6 in S122FRAASSCOM-004) can name `(commodity, place)`, removes the stub-pinning test, and lands integration test #9 (failure-to-suppression). The widening is the load-bearing change for the entire feature: it transforms `CommodityAvailableAt` from inert to evaluable.

## Assumption Reassessment (2026-04-20)

1. Existing focused/unit coverage in `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]`: `target_alive_dead_produces_critical_failure` (line 874), `route_exists_severed_produces_recoverable_route_blocked` (line 887), `no_critical_threat_with_critical_candidate_produces_survival_need` (line 904), `all_assumptions_pass_returns_all_pass` (line 920), `no_critical_threat_without_candidates_returns_deferred` (line 940), `commodity_available_at_stubbed_as_pass` (line 949 — DELETED in this ticket), `critical_failure_transitions_to_exhausted` (line 966), `recoverable_failure_transitions_to_suspended` (line 989), `all_pass_on_suspended_frame_resumes_to_active` (line 1015), `resume_does_not_reset_stalled_ticks` (line 1038), `exhausted_frame_not_re_evaluated` (line 1066). Every `assert_eq!(result, AssumptionEvalResult::CriticalFailure)` and every constructor of the bare unit variant must migrate to the payload-bearing pattern.
2. Spec deliverables D3 + D5 + D7 in `specs/S122-frame-assumption-commodity-availability.md` (D3 at lines 137–165, D5 at lines 196–218, D7 at lines 235–241). Migration tests #12 (existing assumption coverage unchanged) and #13 (stub removal verification) at lines 291–294. Integration test #9 (failure-to-suppression path) at lines 246–248.
3. Shared abstraction boundary under audit: `AssumptionEvalResult` enum (private to the `agent_tick/frame.rs` module) and the `evaluate_assumptions` function signature. Both are crossed by `apply_assumption_result` (frame.rs:328), `emit_assumption_transitions` (mod.rs:1300), and the two `evaluate_assumptions` call sites (mod.rs:502 and mod.rs:599). Workspace blast radius for the `CriticalFailure` payload widening: 6 sites — `frame.rs:290, 335, 884, 977` and `mod.rs:512, 1317`. All sites in worldwake-ai; no cross-crate exhaustive matches because `AssumptionEvalResult` is `pub(super)`.
6. Intended layer: AI / planning-layer logic for the unit changes. Integration test #9 (failure-to-suppression) uses the `agent_tick` runtime — full action registries are required because the test exercises plan adoption, frame establishment, perception, world mutation, and discrepancy memory updates.
7. Ordering: assumption evaluation runs in the pre-planning block of `process_agent` (mod.rs:485–534), before candidate generation and ranking. The `CriticalFailure` outcome clears the frame and records a discrepancy via `record_assumption_failure` (frame.rs:431), which then suppresses re-adoption of the same `(goal, place, target)` for `structural_block_ticks` ticks per the post-S109 `TtlExpiry` clearing path.
13. The variant payload widening is internal to one crate (worldwake-ai); no cross-crate matches needed. `FrameAssumption` already derives `Copy`, so the new payload is `Copy`-safe — `AssumptionEvalResult` can keep its existing trait bounds.

## Architecture Check

1. Widening `CriticalFailure` to carry `FrameAssumption` keeps the trace surface (D6 in S122FRAASSCOM-004) addressable from a single payload, rather than inferring failure identity from frame state at trace emission time. Updating the `TargetAlive` arm to also use `CriticalFailure(*assumption)` keeps the variant payload uniform — sibling unit variants like `RecoverableFailure(SuspensionReason)` already carry their reason, so this is a consistency improvement that completes the pattern.
2. The stub arm and its pinning test are deleted, not aliased (FND-28). The always-true behavior is replaced atomically with the real evaluator — there is no transitional "stub still exists but isn't called" state.

## Verification Layers

1. `CommodityAvailableAt` returns `CriticalFailure(FrameAssumption::CommodityAvailableAt { commodity, place })` when refuted -> focused unit test in `agent_tick/frame.rs#[cfg(test)]` using mock co-located view with no matching lot/source.
2. `CommodityAvailableAt` returns `AllPass` when believed (co-located item lot present) -> focused unit test.
3. `CommodityAvailableAt` returns `Deferred` when unknown (not co-located, no belief) -> focused unit test.
4. `TargetAlive` returns `CriticalFailure(FrameAssumption::TargetAlive(entity))` (payload preserved) -> updated existing test (`target_alive_dead_produces_critical_failure`).
5. Failure-to-suppression integration: agent with belief that `Apple` lot exists at place `P`, world state mutated to remove the lot, agent travels to `P`, on first co-located tick the assumption fails, `record_assumption_failure` records a `Discrepancy::BeliefContradicted` entry with `expires_tick = arrival_tick + structural_block_ticks` and `clearing_condition: TtlExpiry`, frame is cleared with `FrameClearReason::AssumptionFailed` -> integration test verified at the `agent_tick` runtime layer with full action registries (decision-trace + `DiscrepancyMemory` + `runtime.last_frame_clear_reason`).
6. Multi-layer ticket — focused unit coverage proves verdict mapping; integration test proves the suppression chain through `DiscrepancyMemory`. Trace surface assertions are scoped to S122FRAASSCOM-004 (this ticket leaves the payload available but does not yet surface it in the trace summary).

## What to Change

### 1. Widen `AssumptionEvalResult::CriticalFailure` to carry `FrameAssumption`

- File: `crates/worldwake-ai/src/agent_tick/frame.rs` (lines 207–217)
- Change:

  ```rust
  pub(super) enum AssumptionEvalResult {
      AllPass,
      RecoverableFailure(SuspensionReason),
      CriticalFailure(FrameAssumption),
      Deferred,
  }
  ```

- `FrameAssumption` already derives `Copy`, so the payload is Copy-safe. Existing trait bounds on `AssumptionEvalResult` (`Clone, Debug, Eq, PartialEq`) remain satisfied.

### 2. Update `evaluate_assumptions` signature and existing arms

- File: `crates/worldwake-ai/src/agent_tick/frame.rs` (line 279)
- Add `agent: EntityId` parameter: `pub(super) fn evaluate_assumptions(assumptions: &[FrameAssumption], view: &dyn RuntimeBeliefView, agent: EntityId, ranked_candidates: Option<&[RankedGoal]>) -> AssumptionEvalResult`.
- Update the `TargetAlive(entity)` arm (lines 288–292): when `!view.is_alive(entity)`, return `AssumptionEvalResult::CriticalFailure(*assumption)`.

### 3. Replace the `CommodityAvailableAt` stub arm with the real evaluator

- File: `crates/worldwake-ai/src/agent_tick/frame.rs` (lines 314–316)
- Replace with:

  ```rust
  FrameAssumption::CommodityAvailableAt { commodity, place } => {
      match assess_commodity_availability(view, agent, commodity, place) {
          AvailabilityVerdict::Believed => continue,
          AvailabilityVerdict::Refuted => {
              return AssumptionEvalResult::CriticalFailure(*assumption);
          }
          AvailabilityVerdict::UnknownOrStale => has_deferred = true,
      }
  }
  ```

- Remove the `// CommodityAvailableAt is stubbed as always-true (future work).` doc-comment line at frame.rs:278.

### 4. Update `apply_assumption_result` to ignore the new payload

- File: `crates/worldwake-ai/src/agent_tick/frame.rs` (line 335)
- Change `AssumptionEvalResult::CriticalFailure =>` to `AssumptionEvalResult::CriticalFailure(_) =>`. The trace surface in S122FRAASSCOM-004 will read the payload through `emit_assumption_transitions`; this function only needs to know failure occurred.

### 5. Update both `evaluate_assumptions` call sites

- File: `crates/worldwake-ai/src/agent_tick/mod.rs`
- Line 502: change `evaluate_assumptions(&frame.assumptions, &view, None)` to `evaluate_assumptions(&frame.assumptions, &view, agent, None)`.
- Line 599: change `evaluate_assumptions(&[FrameAssumption::NoCriticalThreat], &runtime_belief_view(...), Some(&ranked_candidates))` to `evaluate_assumptions(&[FrameAssumption::NoCriticalThreat], &runtime_belief_view(...), agent, Some(&ranked_candidates))`.
- Line 512: change `if matches!(eval, AssumptionEvalResult::CriticalFailure)` to `if matches!(eval, AssumptionEvalResult::CriticalFailure(_))`.

### 6. Update `emit_assumption_transitions`

- File: `crates/worldwake-ai/src/agent_tick/mod.rs` (line 1317)
- Change `AssumptionEvalResult::CriticalFailure =>` to `AssumptionEvalResult::CriticalFailure(_) =>`. Payload surfacing in the trace itself lands in S122FRAASSCOM-004; this ticket leaves the payload bound-but-unused at this site (`_` pattern is acceptable here because S122FRAASSCOM-004 is the next ticket and will widen the binding).

### 7. Update affected test sites in `frame.rs#[cfg(test)]`

- File: `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]`
- Line 884: replace `assert_eq!(result, AssumptionEvalResult::CriticalFailure)` with `assert!(matches!(result, AssumptionEvalResult::CriticalFailure(FrameAssumption::TargetAlive(e)) if e == dead_entity))`.
- Line 977: update the `apply_assumption_result(&frame, &AssumptionEvalResult::CriticalFailure, ...)` call to construct a payload-bearing variant (e.g., `&AssumptionEvalResult::CriticalFailure(FrameAssumption::TargetAlive(make_entity(99)))`).
- Lines 879, 893, 909, 929, 944, 952: each `evaluate_assumptions(...)` call site needs the new `agent` argument inserted between `&view` and `Some(&[])`/`None`. Use `make_entity(0)` (or a sentinel agent already in scope).
- Add 4 new unit tests for the `CommodityAvailableAt` arm:
  - `evaluate_commodity_available_at_returns_critical_failure_when_refuted` — co-located view, no lot/source for commodity at the place.
  - `evaluate_commodity_available_at_returns_all_pass_when_believed` — co-located view with item lot for commodity.
  - `evaluate_commodity_available_at_returns_deferred_when_unknown` — not co-located, no belief about place.
  - `evaluate_commodity_available_at_co_located_resource_source_returns_all_pass` — co-located view with a viable resource source.

### 8. Delete the stub-pinning test

- File: `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]` (lines 949–961)
- Remove `commodity_available_at_stubbed_as_pass` entirely — the assertion (`AllPass` for an unevaluable assumption) is incorrect under the new evaluator (an empty mock view returns `Deferred`).

### 9. Add integration test #9 (failure-to-suppression)

- File: `crates/worldwake-ai/src/agent_tick/tests.rs`
- New test `commodity_assumption_failure_records_suppression`:
  - Construct world: agent A at place A, target lot L of `Apple` at place P, route A→P.
  - Establish belief in A's `AgentBeliefStore` that L exists at P with Apple inventory.
  - Adopt `Travel(P) → pick_up(L)` plan for `AcquireCommodity { Apple, SelfConsume }`.
  - Mutate world to remove L from P (lot despawned).
  - Step the agent until co-located at P.
  - Assert: within one tick of arrival, `evaluate_assumptions` returns `CriticalFailure(FrameAssumption::CommodityAvailableAt { commodity: Apple, place: P })`.
  - Assert: `DiscrepancyMemory` contains an entry with `discrepancy: Discrepancy::BeliefContradicted`, `expires_tick: arrival_tick + structural_block_ticks`, `clearing_condition: TtlExpiry`, `blocker_key.goal_key` matching the AcquireCommodity goal.
  - Assert: `runtime.last_frame_clear_reason == Some(FrameClearReason::AssumptionFailed)`.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs` (modify — variant widening, eval arm, test updates, stub deletion, 4 new unit tests)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — both call sites at lines 502 and 599, `matches!` patterns at lines 512 and 1317)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — new integration test #9)

## Out of Scope

- Trace surface payload reading — D6 in S122FRAASSCOM-004.
- Suppression-prevents-re-adoption integration test (#10) — S122FRAASSCOM-004.
- Stale-defers integration test (#11) — S122FRAASSCOM-004.
- Survival-golden re-run — S122FRAASSCOM-004.
- Falsification probes — S122FRAASSCOM-005.

## Acceptance Criteria

### Tests That Must Pass

1. New: 4 unit tests for `CommodityAvailableAt` arm verdict cases (Refuted/Believed/Deferred/co-located resource source).
2. Updated: `target_alive_dead_produces_critical_failure` asserts payload `CriticalFailure(TargetAlive(_))`.
3. Updated: `critical_failure_transitions_to_exhausted` constructs a payload-bearing `CriticalFailure`.
4. New integration: `commodity_assumption_failure_records_suppression` (test #9) verifies the failure-to-suppression chain end-to-end.
5. Deleted: `commodity_available_at_stubbed_as_pass` no longer exists — `grep -n "commodity_available_at_stubbed_as_pass" crates/worldwake-ai/src/agent_tick/frame.rs` returns 0 matches.
6. Removed: the `// CommodityAvailableAt is stubbed as always-true (future work).` comment is gone — `grep -n "stubbed as always-true" crates/worldwake-ai/src/agent_tick/frame.rs` returns 0 matches.
7. All other pre-S122 tests in `agent_tick/frame.rs` continue to pass after the signature/payload migration.
8. Existing suite: `cargo test -p worldwake-ai` (full crate) passes.

### Invariants

1. `CommodityAvailableAt` is evaluable: `evaluate_assumptions` returns one of `AllPass` / `CriticalFailure(...)` / `Deferred` based on the helper's verdict — never silently passes a refuted assumption. (FND-21.)
2. `CriticalFailure` payload identifies which assumption failed, uniformly across `TargetAlive` and `CommodityAvailableAt`. (FND-29.)
3. Stub removal is total — no comment, no always-true arm, no test pinning the stub. (FND-28.)
4. The widening preserves the suppression contract from S109: failed assumptions record `Discrepancy::BeliefContradicted` (with target) or `PartialExecutionDrift` (without target) under `TtlExpiry` clearing for `structural_block_ticks` ticks. No change to `record_assumption_failure` semantics.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/frame.rs#[cfg(test)]` — 4 new `CommodityAvailableAt` unit tests; 6 existing tests migrated for new signatures and payloads; 1 test deleted (stub pin).
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — 1 new integration test (`commodity_assumption_failure_records_suppression`).

### Commands

1. `cargo test -p worldwake-ai --lib agent_tick::frame`
2. `cargo test -p worldwake-ai --lib agent_tick::tests commodity_assumption_failure_records_suppression`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
