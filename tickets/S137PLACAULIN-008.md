# S137PLACAULIN-008: Decision-trace RepairAttemptTrace

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `decision_trace.rs` adds `RepairAttemptTrace` to `AgentDecisionTrace`
**Deps**: archive/tickets/S137PLACAULIN-006.md (plan_repair module, RepairKind, RepairFailure)

## Problem

S137 D10 adds `RepairAttemptTrace` to the decision trace surface so debuggers and the observer (ticket 009) can inspect which repair kinds were attempted, which succeeded, which failed, and the budget consumed. Without this surface, the chosen `RepairKind` is visible in the event log (via `RepairAppliedPayload`) but the rejected alternatives and their failure reasons are not — the "why this repair and not that one?" question becomes unanswerable from inspection alone (FND-29).

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `crates/worldwake-ai/src/decision_trace.rs` defines ~30 existing trace types (`PortfolioTrace`, `PortfolioSlotTrace`, `AgentDecisionTrace`, `PlanAttemptTrace`, etc.). Test boundary at line 2515. No existing `RepairAttemptTrace`. `RepairKind` exists in core; `RepairFailure` exists in `crates/worldwake-ai/src/plan_repair.rs` from archive/tickets/S137PLACAULIN-006.md.
2. Spec `specs/S137-plan-causal-links-and-repair.md` D10 specifies the trace shape: chosen `RepairKind`, breach signature, rejected `(RepairKind, RepairFailure)` pairs, budget consumed. Should compose with existing `AgentDecisionTrace` per the `PortfolioSlotTrace`/`PlanAttemptTrace` precedent.
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

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai decision_trace::repair_attempt_trace` — new focused tests for shape + roundtrip + sink integration.
2. `cargo test -p worldwake-ai plan_repair` — existing ticket-006 tests still pass with the trace-emission seam added.
3. Existing suite: `cargo test --workspace`.

### Invariants

1. `RepairAttemptTrace.chosen_kind == Some(kind)` exactly when the trace was emitted from a `RepairOutcome::Repaired` outcome.
2. `RepairAttemptTrace.rejected` lists attempts in deterministic `RepairKind` Ord order (matches ticket 006's invariant).
3. `RepairAttemptTrace.budget_consumed ≤ budget_total`.
4. `CausalLinkCapHit` is emitted exactly when the planner emitter truncates `PlanGuard.causal_links` at the cap.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` `#[cfg(test)]` — new tests:
   - `repair_attempt_trace_roundtrips_through_bincode`
   - `causal_link_cap_hit_roundtrips_through_bincode`
2. `crates/worldwake-ai/src/plan_repair.rs` — new test `repair_search_emits_attempt_trace_with_rejected_kinds`.

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test -p worldwake-ai plan_repair`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
