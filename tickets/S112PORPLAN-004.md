# S112PORPLAN-004: PortfolioTrace on PlanningPipelineTrace

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No (trace extension; not part of authoritative state)
**Deps**: S112PORPLAN-002 (uses `SlotKind`, `FeasibilityVerdict`)

## Problem

S112 D6 extends the decision-trace sink with a per-tick portfolio summary so that observers, goldens, and debug tooling can inspect slot assembly, probe verdicts, and which slots were attempted. The trace belongs on `PlanningPipelineTrace` — not on the top-level `AgentDecisionTrace` — because portfolio assembly runs only during planning ticks, never during `ActiveAction` ticks.

This ticket adds the trace type and the `Option<PortfolioTrace>` field. Ticket 005 populates it when the planning loop is rewritten.

## Assumption Reassessment (2026-04-20)

1. `PlanningPipelineTrace` is defined at `crates/worldwake-ai/src/decision_trace.rs:229` and is reached through `DecisionOutcome::Planning(Box<PlanningPipelineTrace>)` (line 96). It already carries several `Option<...>` trace fields (e.g., `frame_transition`, `patrol_route`). Adding `Option<PortfolioTrace>` follows the existing pattern.
2. Spec S112 D6 defines `PortfolioTrace` and `PortfolioSlotTrace` with `SlotKind`/`GoalKey`/`u32`/`FeasibilityVerdict` fields. `GoalRejectionReason::FeasibilityProbeFailed` already exists at `crates/worldwake-core/src/decision_event_payload.rs:96` (currently unused) — ticket 005 begins populating it when it writes `GoalCommittedPayload::rejected_alternatives`. No new `GoalRejectionReason` variant in this ticket.
3. Shared boundary: the decision-trace sink is an optional observer-facing derived view (FND-27 cache-not-truth). The authoritative surface for slot rejections is the event log (`GoalCommittedPayload::rejected_alternatives`), wired up in ticket 005.
4. Adjacent-contradiction classification: `AgentDecisionTrace` itself (line 74) carries only `agent`, `tick`, `outcome` — attaching portfolio data there would be a misplacement (active-action ticks don't assemble portfolios). Correct placement is `PlanningPipelineTrace`.

## Architecture Check

1. Trace-only change — `PortfolioTrace` is a derived read-model (FND-27), not authoritative state. Deleting the trace and recomputing from the live portfolio at the next tick would lose no world meaning.
2. Planning-only visibility: `Option<PortfolioTrace>` is `None` whenever `DecisionOutcome` is `Dead` or `ActiveAction` — this makes "portfolio assembly ran" legible in the trace without ambiguity.
3. FND-28 alignment: no parallel trace — this is an additive extension of the single existing `PlanningPipelineTrace` type, not a separate trace type for portfolio data.

## Verification Layers

1. `PortfolioTrace` field presence on `PlanningPipelineTrace` → focused unit test constructing a default `PlanningPipelineTrace` and confirming `portfolio: None`.
2. Trace type round-trips through serde → focused unit test, since downstream observers (golden forensics + observer binary) require stable serialization.
3. Single-layer ticket — population of the field happens in ticket 005. This ticket only declares the type.

## What to Change

### 1. Add `PortfolioTrace` and `PortfolioSlotTrace` to `decision_trace.rs`

Insert near the other trace sub-types (e.g., after `FrameTransitionTrace`):

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortfolioTrace {
    pub slots: BTreeMap<SlotKind, PortfolioSlotTrace>,
    pub slots_attempted: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortfolioSlotTrace {
    pub goal_key: GoalKey,
    pub motive_score: u32,
    pub feasibility: FeasibilityVerdict,
}
```

`SlotKind` and `FeasibilityVerdict` come from `crate::agent_tick::portfolio`. Promote them to `pub(crate)` visibility in `portfolio.rs` (ticket 002 declared them `pub(crate)`, which already satisfies same-crate use).

### 2. Add `portfolio` field to `PlanningPipelineTrace`

Insert `pub portfolio: Option<PortfolioTrace>,` in the struct body. Update any exhaustive struct-literal construction sites inside the crate (typically test harnesses) to supply `portfolio: None`.

### 3. Ensure serde derives on referenced types

`SlotKind`, `FeasibilityVerdict` need `Serialize`/`Deserialize` for the trace to round-trip. Verify and add those derives in `portfolio.rs` if not already present (ticket 002's derive list was declared for runtime equality only — this ticket extends it).

### 4. Unit tests

In `decision_trace.rs` `#[cfg(test)]`:

1. `planning_pipeline_trace_default_portfolio_is_none` — constructing a default `PlanningPipelineTrace` leaves `portfolio` at `None`.
2. `portfolio_trace_roundtrips_through_bincode` — fixture with one `Plausible` slot and one `RejectedBeforeSearch` slot round-trips through bincode.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — new types + field)
- `crates/worldwake-ai/src/agent_tick/portfolio.rs` (modify — add `Serialize`/`Deserialize` derives to `SlotKind` and `FeasibilityVerdict` if 002 omitted them)
- Any crate-internal `PlanningPipelineTrace { ... }` literal that enumerates all fields without spread (to be discovered during implementation; typically zero or one test fixture)

## Out of Scope

- Populating `PlanningPipelineTrace::portfolio` from the planning loop — ticket 005.
- Emitting `GoalRejectionReason::FeasibilityProbeFailed` into `GoalCommittedPayload::rejected_alternatives` — ticket 005.
- Observer binary (`crates/worldwake-cli/src/bin/observer.rs`) rendering of `PortfolioTrace` — deferred to a follow-up observer ticket, not part of S112.
- Golden assertions over `PortfolioTrace` contents — ticket 006 introduces the golden that will read the trace.

## Acceptance Criteria

### Tests That Must Pass

1. `planning_pipeline_trace_default_portfolio_is_none` passes.
2. `portfolio_trace_roundtrips_through_bincode` passes.
3. Existing suite: `cargo test -p worldwake-ai`, `cargo test --workspace`.
4. `cargo clippy --workspace --all-targets -- -D warnings` remains clean.

### Invariants

1. `PortfolioTrace` is optional on `PlanningPipelineTrace` — existing decision-trace consumers that don't know about it are unaffected.
2. `PortfolioTrace::slots` uses `BTreeMap<SlotKind, _>` for deterministic iteration.
3. `SlotKind` and `FeasibilityVerdict` remain `pub(crate)` — the trace type exposes them only through `PortfolioSlotTrace`, keeping the public trace surface stable.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — inline `#[cfg(test)]` additions per the What to Change section.

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
