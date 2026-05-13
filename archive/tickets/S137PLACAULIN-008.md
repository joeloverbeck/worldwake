# S137PLACAULIN-008: Decision-trace RepairAttemptTrace

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `decision_trace.rs` adds `RepairAttemptTrace` to `AgentDecisionTrace`
**Deps**: archive/tickets/S137PLACAULIN-006.md (plan_repair module, RepairKind, RepairFailure)

## Problem

S137 D10 adds `RepairAttemptTrace` to the decision trace surface so debuggers and the observer (ticket 009) can inspect which repair kinds were attempted, which succeeded, which failed, and the budget consumed. Without this surface, the chosen `RepairKind` is visible in the event log (via `RepairAppliedPayload`) but the rejected alternatives and their failure reasons are not — the "why this repair and not that one?" question becomes unanswerable from inspection alone (FND-29).

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-ai/src/decision_trace.rs` defines ~30 existing trace types (`PortfolioTrace`, `PortfolioSlotTrace`, `AgentDecisionTrace`, `PlanAttemptTrace`, etc.). Test boundary at line 2515. No existing `RepairAttemptTrace`. `RepairKind` exists in core; `RepairFailure` exists in `crates/worldwake-ai/src/plan_repair.rs` from archive/tickets/S137PLACAULIN-006.md.
2. Spec `archive/specs/S137-plan-causal-links-and-repair.md` D10 specifies the trace shape: chosen `RepairKind`, breach signature, rejected `(RepairKind, RepairFailure)` pairs, budget consumed. Should compose with existing `AgentDecisionTrace` per the `PortfolioSlotTrace`/`PlanAttemptTrace` precedent.
3. Shared boundary: the `DecisionTraceSink` surface (existing). Per `references/worldwake-validation-patterns.md` Dual-Use Read-Model Types and Read-Only Tooling Consumer patterns, trace types live in `worldwake-ai/src/` (not `tests/`) so the observer binary in `worldwake-cli` can consume them via existing public API.
4. Decision-trace preference (precision rule #6): for AI reasoning, candidate absence, suppression, or planner behavior, prefer decision-trace assertions over weaker indirect evidence. `RepairAttemptTrace` is the canonical surface for "why this repair, not that one" — strictly stronger than indirect inference from `RepairAppliedPayload` alone.

## Architecture Check

1. **Composes with existing trace surface**: `RepairAttemptTrace` slots into `AgentDecisionTrace` (line ~92) following the `PlanAttemptTrace` precedent. No parallel trace pipeline introduced.
2. **No back-compat shim**: net-new type added to an existing trace surface; no legacy alternative.

## Verification Layers

1. Trace-shape invariant → focused unit test in `decision_trace.rs` `#[cfg(test)]` asserting bincode/serde roundtrip.
2. Sink installation → focused test asserting `DecisionTraceSink` receives `RepairAttemptTrace` when repair attempts run (this requires ticket 007's routing to call into the trace emission seam; this ticket adds the seam).
3. Single-layer ticket (trace type addition + sink installation); the consumer-side rendering of this trace lives in ticket 009.

## What to Change

### 1. Add `RepairAttemptTrace` to `decision_trace.rs`

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepairAttemptTrace {
    pub breach: BreachSignature,
    pub chosen_kind: Option<RepairKind>,           // None if RepairOutcome::Failed
    pub rejected: Vec<(RepairKind, RepairFailure)>,
    pub budget_consumed: u16,                       // node expansions used
    pub budget_total: u16,                          // repair_budget_fraction × max_node_expansions
}
```

Add a field on `AgentDecisionTrace` (around line 92) or a sibling collection:

```rust
pub repair_attempts: Vec<RepairAttemptTrace>,
```

### 2. Add `CausalLinkCapHit` annotation

Archive ticket 006 capped `PlanGuard.causal_links` silently at emit time. If this ticket keeps cap-hit traceability in scope, define the trace annotation here:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CausalLinkCapHit {
    pub plan_step_index: u16,
    pub truncated_count: u8,
    pub cap: u8,
}
```

Add `pub causal_link_cap_hits: Vec<CausalLinkCapHit>` to `AgentDecisionTrace`.

### 3. Sink integration

Update `DecisionTraceSink` consumers in the new `plan_repair` module (ticket 006) to push `RepairAttemptTrace` entries at attempt completion. This change lands inside `plan_repair.rs` as part of this ticket's scope.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — `RepairAttemptTrace`, `CausalLinkCapHit`, `AgentDecisionTrace` field, tests)
- `crates/worldwake-ai/src/plan_repair.rs` (modify — push `RepairAttemptTrace` at attempt completion; this is the seam wired in by this ticket)

## Out of Scope

- Observer rendering of `RepairAttemptTrace` — ticket 009.
- Golden tests asserting trace contents — ticket 010.
- New event tags — `EventTag::RepairApplied` already exists.

## Acceptance Result

### Tests Passed

1. `cargo test -p worldwake-ai --lib repair_attempt_trace` passed focused repair trace shape, roundtrip, and failed-attempt accounting coverage.
2. `cargo test -p worldwake-ai --lib causal_link_cap_hit` passed focused cap-hit shape and derived cap-hit coverage.
3. `cargo test -p worldwake-ai --lib plan_repair` passed the existing ticket-006/011 repair module tests with the widened repaired-outcome shape.
4. `cargo test --workspace` passed.
5. `cargo clippy --workspace --all-targets -- -D warnings` passed.

### Invariants

1. `RepairAttemptTrace.chosen_kind == Some(kind)` is emitted for `RepairOutcome::Repaired`, while failed repair traces use `None`.
2. `RepairAttemptTrace.rejected` preserves the deterministic repair attempt order produced by `attempt_repair_then_replan`.
3. `RepairAttemptTrace.budget_consumed <= budget_total`; focused tests cover failed and successful accounting.
4. `CausalLinkCapHit` is reported when a traced plan step retained fewer `PlanGuard.causal_links` than its `required_facts` count.

## Test Plan Result

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` `#[cfg(test)]`:
   - `repair_attempt_trace_roundtrips_through_bincode`
   - `causal_link_cap_hit_roundtrips_through_bincode`
2. `crates/worldwake-ai/src/plan_repair.rs`:
   - `replace_provider_returns_repaired_plan_for_lawful_route_provider` now asserts prior rejected attempts are carried on `RepairOutcome::Repaired`.
3. `crates/worldwake-ai/src/agent_tick/execution.rs`:
   - `failed_local_repair_attempt_trace_records_budget_and_rejections`
   - `successful_local_repair_budget_consumed_includes_chosen_attempt`
4. `crates/worldwake-ai/src/agent_tick/tests.rs`:
   - `causal_link_cap_hits_report_truncated_plan_guards`

### Commands

1. `cargo test -p worldwake-ai --lib repair_attempt_trace`
2. `cargo test -p worldwake-ai --lib causal_link_cap_hit`
3. `cargo test -p worldwake-ai --lib plan_repair`
4. `cargo test -p worldwake-ai --lib agent_tick::execution`
5. `cargo test -p worldwake-ai`
6. `cargo test --workspace`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-13.

1. Added `RepairAttemptTrace` and `CausalLinkCapHit` to `crates/worldwake-ai/src/decision_trace.rs`, with `AgentDecisionTrace.repair_attempts` and `AgentDecisionTrace.causal_link_cap_hits` as the sink-visible carriage fields.
2. Wired localized repair attempts in `agent_tick/execution.rs` into per-tick decision traces for both successful and failed repair outcomes. `RepairOutcome::Repaired` now carries prior rejected attempts so a successful trace can report the alternatives tried before the chosen kind.
3. Added derived cap-hit reporting from the final traced plan by comparing each guarded step's `required_facts` with retained `causal_links`.
4. Updated explicit `AgentDecisionTrace` literals in AI tests, golden harness helpers, observer tests, survival forensics, and visualizer trace buffers for the new trace fields.
5. Truth-synced `archive/specs/S137-plan-causal-links-and-repair.md` for the landed trace fields and `RepairOutcome::Repaired` shape. Updated the now-archived `archive/tickets/S137PLACAULIN-009.md` to cite this completed ticket by path.

## Deviations

1. `CausalLinkCapHit` is emitted as a derived `AgentDecisionTrace.causal_link_cap_hits` entry from the final traced plan, not as a separate planner event pipeline.
2. Observer rendering remains out of scope and was later completed by `archive/tickets/S137PLACAULIN-009.md`.

## Verification Result

1. Passed `cargo test -p worldwake-ai --lib repair_attempt_trace`.
2. Passed `cargo test -p worldwake-ai --lib causal_link_cap_hit`.
3. Passed `cargo test -p worldwake-ai --lib plan_repair`.
4. Passed `cargo test -p worldwake-ai --lib agent_tick::execution`.
5. Passed `cargo test -p worldwake-ai`.
6. Passed `cargo test --workspace`.
7. Passed `cargo clippy --workspace --all-targets -- -D warnings`.
8. Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py tickets/S137PLACAULIN-008.md`.
9. Passed `git diff --check`.
