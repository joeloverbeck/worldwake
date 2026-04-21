# S112PORPLAN-004: PortfolioTrace on PlanningPipelineTrace

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No (trace extension; not part of authoritative state)
**Deps**: archive/tickets/S112PORPLAN-002.md (uses `SlotKind`, `FeasibilityVerdict`)

## Problem

S112 D6 extends the decision-trace sink with a per-tick portfolio summary so that observers, goldens, and debug tooling can inspect slot assembly, probe verdicts, and which slots were attempted. The trace belongs on `PlanningPipelineTrace` — not on the top-level `AgentDecisionTrace` — because portfolio assembly runs only during planning ticks, never during `ActiveAction` ticks.

This ticket adds the trace type and the `Option<PortfolioTrace>` field. Ticket 005 populates it when the planning loop is rewritten.

## Assumption Reassessment (2026-04-20)

1. `PlanningPipelineTrace` is defined at `crates/worldwake-ai/src/decision_trace.rs:229` and is reached through `DecisionOutcome::Planning(Box<PlanningPipelineTrace>)` (line 96). It already carries several planning-only fields, including `Option<...>` trace fields such as `frame_transition` and `pursuit_invalidation`. Adding `Option<PortfolioTrace>` follows the existing pattern.
2. Spec S112 D6 defines `PortfolioTrace` and `PortfolioSlotTrace` with `SlotKind`/`GoalKey`/`u32`/`FeasibilityVerdict` fields. `GoalRejectionReason::FeasibilityProbeFailed` already exists at `crates/worldwake-core/src/decision_event_payload.rs:96` (currently unused) — ticket 005 begins populating it when it writes `GoalCommittedPayload::rejected_alternatives`. No new `GoalRejectionReason` variant in this ticket.
3. Shared boundary: the decision-trace sink is an optional observer-facing derived view (FND-27 cache-not-truth). The authoritative surface for slot rejections is the event log (`GoalCommittedPayload::rejected_alternatives`), wired up in ticket 005.
4. Mismatch + correction: the drafted round-trip proof seam was stale. On the live branch, `PlanningPipelineTrace`, `AgentDecisionTrace`, and most neighboring planning trace structs do not derive `Serialize`/`Deserialize`, so there is no honest same-file bincode round-trip contract to extend in this ticket. The scoped deliverable is the staged trace schema (`PortfolioTrace`, `PortfolioSlotTrace`, and `Option<PortfolioTrace>` field) plus constructor/test fallout; serde widening remains out of scope until the wider decision-trace model adopts it.
5. Mismatch + correction: `PlanningPipelineTrace` has no `Default` impl on the live branch. The focused proof therefore binds to explicit `PlanningPipelineTrace { ... }` literals at the same-file constructor/test seams rather than inventing a new `Default` contract just for this field.
6. Adjacent-contradiction classification: `AgentDecisionTrace` itself (line 74) carries only `agent`, `tick`, `outcome` — attaching portfolio data there would be a misplacement (active-action ticks don't assemble portfolios). Correct placement is `PlanningPipelineTrace`.

## Architecture Check

1. Trace-only change — `PortfolioTrace` is a derived read-model (FND-27), not authoritative state. Deleting the trace and recomputing from the live portfolio at the next tick would lose no world meaning.
2. Planning-only visibility: `Option<PortfolioTrace>` is `None` whenever `DecisionOutcome` is `Dead` or `ActiveAction` — this makes "portfolio assembly ran" legible in the trace without ambiguity.
3. FND-28 alignment: no parallel trace — this is an additive extension of the single existing `PlanningPipelineTrace` type, not a separate trace type for portfolio data.

## Verification Layers

1. `PortfolioTrace` field presence on `PlanningPipelineTrace` → focused unit test constructing an explicit `PlanningPipelineTrace` literal and confirming `portfolio: None`.
2. `PortfolioTrace` and `PortfolioSlotTrace` preserve deterministic/equality behavior at the staged trace-schema layer → focused unit test constructing a two-slot trace with both `Plausible` and `RejectedBeforeSearch` entries and asserting exact stored contents.
3. Single-layer ticket — population of the field happens in ticket 005. This ticket only declares the type and updates explicit same-crate constructors.

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

### 3. Unit tests

In `decision_trace.rs` `#[cfg(test)]`:

1. `planning_pipeline_trace_portfolio_defaults_to_none_in_literal` — constructing an explicit `PlanningPipelineTrace` literal leaves `portfolio` at `None`.
2. `portfolio_trace_preserves_slot_contents` — fixture with one `Plausible` slot and one `RejectedBeforeSearch` slot preserves exact stored contents and deterministic `BTreeMap` ordering.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — new types + field)
- Any crate-internal `PlanningPipelineTrace { ... }` literal that enumerates all fields without spread (to be discovered during implementation; typically zero or one test fixture)

## Out of Scope

- Populating `PlanningPipelineTrace::portfolio` from the planning loop — ticket 005.
- Emitting `GoalRejectionReason::FeasibilityProbeFailed` into `GoalCommittedPayload::rejected_alternatives` — ticket 005.
- Observer binary (`crates/worldwake-cli/src/bin/observer.rs`) rendering of `PortfolioTrace` — deferred to a follow-up observer ticket, not part of S112.
- Golden assertions over `PortfolioTrace` contents — ticket 006 introduces the golden that will read the trace.

## Acceptance Criteria

### Tests That Must Pass

1. `planning_pipeline_trace_portfolio_defaults_to_none_in_literal` passes.
2. `portfolio_trace_preserves_slot_contents` passes.
3. Existing suite: `cargo test -p worldwake-ai`, `cargo test --workspace`.
4. `cargo clippy --workspace --all-targets -- -D warnings` remains clean.

### Invariants

1. `PortfolioTrace` is optional on `PlanningPipelineTrace` and remains `None` until ticket 005 populates it — existing decision-trace consumers that don't know about it are unaffected.
2. `PortfolioTrace::slots` uses `BTreeMap<SlotKind, _>` for deterministic iteration.
3. `SlotKind` and `FeasibilityVerdict` remain `pub(crate)` — the trace type stays crate-internal like the surrounding decision-trace sink.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — inline `#[cfg(test)]` additions per the What to Change section.

### Commands

1. `cargo test -p worldwake-ai decision_trace`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-21.

- Added `PortfolioTrace` and `PortfolioSlotTrace` to [crates/worldwake-ai/src/decision_trace.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs:1) and extended `PlanningPipelineTrace` with `portfolio: Option<PortfolioTrace>`.
- Updated every live `PlanningPipelineTrace { ... }` constructor fallout site that enumerates fields, including same-crate tests plus external harness/test literals in [crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs:1) and [crates/worldwake-cli/src/bin/observer.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-cli/src/bin/observer.rs:1).
- Added focused unit coverage in `decision_trace.rs` for the staged `portfolio: None` default-at-construction boundary and exact `PortfolioTrace` slot-content preservation.

## Deviations

- Reassessment corrected the drafted proof seam: the live decision-trace model does not expose a serde/bincode round-trip contract for `PlanningPipelineTrace`, so this ticket landed as a staged trace-schema extension plus constructor fallout rather than widening the whole trace family to `Serialize`/`Deserialize`.
- Reassessment also corrected the drafted `Default` assumption: `PlanningPipelineTrace` has no `Default` impl on the live branch, so the focused proof binds to explicit struct literals instead of inventing a new defaulting contract.
- Although the new slot payload types remain staged and are not yet populated until ticket 005, `PlanningPipelineTrace` is constructed from integration-test and observer helper code outside `decision_trace.rs`. The `portfolio` field therefore remains publicly constructible there, and carries a narrow `#[allow(dead_code)]` until ticket 005 begins reading/populating it in live planning traces.

## Verification Result

- Passed `cargo test -p worldwake-ai decision_trace`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test --workspace`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
