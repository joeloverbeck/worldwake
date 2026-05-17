# S147HTNMETDEC-010: Observer plan-attempt method rendering

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No — observer-only rendering extension in `crates/worldwake-cli/src/bin/observer.rs`.
**Deps**: `archive/tickets/S147HTNMETDEC-009.md` (PlanAttemptTrace.method_trace field exists)

## Problem

S147 D9 extends the observer to surface the method chosen per plan attempt and its decomposition trace. Without this rendering, the `method_trace` field added in ticket 009 is invisible to operators inspecting scenario runs. This ticket is the observer counterpart to ticket 009's trace surface and the operator-facing payoff for the entire S147 ticket sequence.

**Spec discrepancy surfaced during reassessment**: The spec text says "observer Section 7" but `crates/worldwake-cli/src/bin/observer.rs:4667` Section 7 is "End-State Inventory & Resources" — not a planning section. The actual planning surfaces in the observer are Section 8 (Per-Agent Decision Summary at line 4740), Section 9 (Budget Exhaustion Snapshots at line 1209), and Section 13 (Scenario Diagnostics at line 3713). The correct target for method rendering is most likely **Section 8** (Per-Agent Decision Summary), which is where per-agent plan attempts are summarized. Confirm during ticket reassessment by reading the section's existing structure and locating the per-attempt rendering loop.

## Assumption Reassessment (2026-05-17)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `PlanAttemptTrace.method_trace: Option<MethodPlanAttemptTrace>` field exists after ticket 009 lands at `crates/worldwake-ai/src/decision_trace.rs:1185`. `MethodPlanAttemptTrace.subgoals_attempted: Vec<SubgoalAttemptResult>` carries the per-subgoal outcome list the observer renders.
2. Observer sections (verified via `grep -nE "writeln.*Section [0-9]+" crates/worldwake-cli/src/bin/observer.rs`):
   - Section 1 (line 4186): Run Metadata
   - Section 2 (line 4210): Per-Agent Summary
   - Section 3a (line 804): Opportunities
   - Section 3b (line 933): Decision History
   - Section 4 (line 4379): Anomaly Flags
   - Section 5 (line 4394): Raw Event Sample
   - Section 6 (line 4563): Per-Agent Belief Summary
   - Section 7 (line 4667): **End-State Inventory & Resources** (NOT planning — spec was wrong)
   - Section 8 (line 4740): Per-Agent Decision Summary
   - Section 9 (line 1209): Budget Exhaustion Snapshots
   - Section 10 (line 1361): Critical Window Forensics
   - Section 11 (line 1614): Artifact Lifecycle
   - Section 12 (line 4096): Contention
   - Section 13 (line 3713): Scenario Diagnostics
3. The `PlanAttemptTrace` consumers in observer.rs use helpers like `failed_plan_outcome_label` (line 3368), `failed_plan_breakdown` (line 3414), `collect_failed_plan_attempts` (line 3440), and `failed_plan_target_belief_labels`. These render attempt-level summaries; method rendering attaches alongside as an additional per-attempt line.
4. Shared boundary: `PlanAttemptTrace` is the contract between trace recording (ticket 009) and observer rendering (this ticket). This ticket only reads `method_trace`; it does not modify the trace surface.
5. The actual target section (Section 8 Per-Agent Decision Summary at line 4740, most likely) must be verified during implementation. If Section 8 doesn't have a per-attempt rendering loop suitable for method extension, fall back to Section 13 (Scenario Diagnostics) which aggregates planning metrics including the new `method_usage` from ticket 009.

## Architecture Check

1. Observer rendering is a derived view per FND-27 (caches, never truth). The method-trace rendering reads `Option<MethodPlanAttemptTrace>` and produces formatted text; if the field is `None` (flat-GOAP fallback), the renderer emits a one-line "(no method — flat GOAP fallback)" note rather than synthesizing a fake method.
2. The rendering format follows existing observer conventions (indented prose, `✓`/`✗`/`Pending` markers) per spec D9. No new format conventions introduced.
3. No backwards-compatibility shims. The rendering extension is purely additive in the chosen section.

## Verification Layers

1. Method-rendering produces expected output → headless render test that constructs a `PlanAttemptTrace` with `method_trace: Some(MethodPlanAttemptTrace { … })` and asserts the rendered text contains the expected method-name and subgoal lines.
2. Flat-GOAP fallback renders cleanly → headless render test with `method_trace: None` asserts the rendered text contains the fallback note and does not synthesize method content.
3. Single-layer ticket (observer rendering only) — no engine changes, no simulation runtime modifications.

## What to Change

### 1. Confirm target section number

Read the existing per-attempt rendering location in `crates/worldwake-cli/src/bin/observer.rs`. Search for where `PlanAttemptTrace` instances are rendered into operator-facing output:

```bash
grep -n "PlanAttemptTrace\|failed_plan_breakdown\|attempts" crates/worldwake-cli/src/bin/observer.rs
```

The likely site is in or near Section 8 (Per-Agent Decision Summary at line 4740). Confirm and proceed; if the rendering happens in a different section (Section 9 or 13), adjust.

### 2. Add method-trace rendering

Extend the per-attempt rendering loop to format the `method_trace` field. Format (per spec D9):

```text
Plan attempt: ProduceCommodity{recipe="Bake Bread"} (Method: ProduceWithGather)
  Subgoal 1: AcquireCommodity(Grain, ≥3) ✓
  Subgoal 2: AcquireCommodity(Flour, ≥2) ✓
  Subgoal 3: TravelTo(KnownWorkstationFor{recipe="Bake Bread"}) ✓
  Subgoal 4: PerformAction(Craft, Bake Bread) — Pending
```

For `method_trace: None`:

```text
Plan attempt: ConsumeOwnedCommodity{commodity=Bread} (Method: none — flat GOAP fallback)
```

For method failures, append the structured failure mode:

```text
Plan attempt: FulfillBounty{bounty=#42} (Method: FulfillBountyDirect)
  Subgoal 1: TravelTo(LastKnownTargetPlace{target=#7}) ✓
  Subgoal 2: ObserveTarget(Target{target=#7}) ✗
  Failure: SubgoalUnachievable(index=1) — Discrepancy::MethodFailure(SubgoalUnachievable)
```

### 3. Update Section 13 (Scenario Diagnostics) to surface `method_usage`

Section 13 already renders `PlanningMetrics`. After ticket 009 adds the `method_usage` field, render the per-method counts here. Format (matching existing Section 13 conventions):

```text
Method usage:
  ProduceWithGather: 12 attempts, 12 selected, 0 fallback, 1 failed
  FulfillBountyDirect: 3 attempts, 3 selected, 0 fallback, 0 failed
  (no method): 47 fallbacks
```

### 4. Inline render tests

In `crates/worldwake-cli/src/bin/observer.rs` test module (the file already has extensive test scaffolding):
- `render_method_trace_with_subgoals_produces_expected_text` — constructs a `PlanAttemptTrace` with a populated `method_trace` and asserts the rendered output matches the expected format.
- `render_method_trace_none_produces_fallback_note` — `method_trace: None` produces the fallback note.
- `render_method_trace_failure_includes_discrepancy_reference` — failed method renders the structured failure line.

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — extend per-attempt rendering loop in Section 8 (likely; confirm during implementation) + extend Section 13's PlanningMetrics rendering to include method_usage + 3 new inline tests)

## Out of Scope

- Trace recording — owned by ticket 009.
- New observer section addition — extends existing sections rather than introducing new ones.
- Per-method tuning of failure-message format — moderate defaults are sufficient for first ship.

## Acceptance Criteria

### Tests That Must Pass

1. `render_method_trace_with_subgoals_produces_expected_text`.
2. `render_method_trace_none_produces_fallback_note`.
3. `render_method_trace_failure_includes_discrepancy_reference`.
4. Existing observer rendering tests pass after the per-attempt extension lands.
5. Section 13 method_usage rendering test (inline) passes — `PlanningMetrics.method_usage` formatted correctly.
6. `cargo clippy -p worldwake-cli --all-targets -- -D warnings` clean.

### Invariants

1. `method_trace: None` produces a clean fallback note rather than a synthesized method.
2. Failed method attempts surface both the rich `MethodFailureMode` (in the trace render) AND the typed `Discrepancy::MethodFailure(MethodFailureKind)` reference (so the operator can connect the trace to the authoritative chain).
3. Section 13 method_usage rendering uses the existing Section 13 indentation and label conventions (no new formatting introduced).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` inline tests — 3 new method-trace render cases + 1 method_usage Section 13 case.

### Commands

1. `cargo test -p worldwake-cli --bin observer`
2. `cargo test -p worldwake-cli`
3. `./scripts/verify.sh`
