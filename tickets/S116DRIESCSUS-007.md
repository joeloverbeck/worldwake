# S116DRIESCSUS-007: Retrofit survival-contested MAX_CRITICAL_RUN_TICKS 400 → 300

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None (test-constant + rationale-comment update)
**Deps**: S116DRIESCSUS-006

## Problem

Spec S116 D8 requires `golden_survival_contested::MAX_CRITICAL_RUN_TICKS` to tighten from 400 to 300 after the full S116 pipeline lands and ticket 006 confirms empirical improvement on the contested scenario. The current 400-tick bound was explicitly relaxed to accommodate the wash-starvation dynamic; with motive escalation interrupting the priority-override cycle, the bound can safely tighten without approaching the water-possession bottleneck that S116 does not address.

## Assumption Reassessment (2026-04-17)

1. Current constant: `const MAX_CRITICAL_RUN_TICKS: u32 = 400;` at `crates/worldwake-ai/tests/golden_survival_contested.rs:34`.
2. Existing rationale comment at lines 22-33 documents the water-possession architectural bottleneck. Per the spec's D8 direction, the comment must be replaced to reference S116 and preserve the water-possession note as a named follow-up.
3. Empirical max-consecutive-run figures from `reports/scenario-analysis-report.md` for survival-contested seed 306006: Agent A = 319 ticks, Agent B = 315 ticks, Agent C = 313 ticks, Agent D = 372 ticks. Worst case = 372. Tightening to 300 requires motive escalation to break runs ≥72 ticks earlier than today's worst case — verified empirically by ticket 006's run-through of the contested scenario under the new pipeline.
4. Live `GoalKind` and affordance surface unchanged: `wash_preconditions` (`crates/worldwake-systems/src/needs_actions.rs:196`) still requires `TargetDirectlyPossessedByActor(0)` with `CommodityKind::Water`. This remains the architectural limit on tightening further than 300.
5. Follow-up: water-possession bottleneck resolution is named in the spec D8 text and in the replaced rationale comment. Not in scope here.
6. Intended verification layer (precision rule 3): Golden E2E coverage. Harness already full-action-registries per the existing test's setup — no change to the harness itself.

## Architecture Check

1. Purely a test-constant tightening + rationale update. No production code changes, no new tests.
2. If the tightened bound fails empirically after this ticket is implemented (i.e., ticket 006's calibration held but 300 is still too aggressive on unseeded variations), apply the 1-3-1 rule to the user rather than silently relaxing.
3. `golden_survival_scattered.rs` is explicitly out of scope — the spec's D8 text notes its tightening is conditional on empirical data supporting a bound tighter than 300, which ticket 006's calibration does not specifically investigate.

## Verification Layers

1. Contested regression bound → `golden_survival_contested` passes with `MAX_CRITICAL_RUN_TICKS = 300`. Authoritative proof surface: the existing per-agent `max_consecutive_critical_ticks < MAX_CRITICAL_RUN_TICKS` assertion in the golden itself.
2. Single-layer ticket — the golden provides the only verification surface needed. No action-trace or event-log assertion needed at this layer (they are ticket 003's territory).

## What to Change

### 1. Tighten the constant

Change `crates/worldwake-ai/tests/golden_survival_contested.rs:34` from:

```rust
const MAX_CRITICAL_RUN_TICKS: u32 = 400;
```

to:

```rust
const MAX_CRITICAL_RUN_TICKS: u32 = 300;
```

### 2. Replace rationale comment

Replace the existing doc-comment block at `golden_survival_contested.rs:22-33` with a new comment referencing S116:

```rust
/// Tightened from 400 to 300 by S116 (Drive Escalation Under Sustained
/// Critical Need). Motive-score escalation interrupts the wash-cycle
/// priority-override that previously allowed dirtiness-critical runs up
/// to ~372 ticks empirically. The 300-tick bound gives ≥72 ticks of
/// headroom over pre-S116 worst case while not depending on resolving
/// the `wash_preconditions` water-possession bottleneck (`needs_actions.rs:196`
/// still requires `TargetDirectlyPossessedByActor(0)` Water). Further
/// tightening toward 200 requires a follow-up spec that changes
/// acquire-water-for-wash precedence or the wash precondition itself.
```

## Files to Touch

- `crates/worldwake-ai/tests/golden_survival_contested.rs` (modify — constant + doc-comment replacement)

## Out of Scope

- `golden_survival_scattered::MAX_CRITICAL_RUN_TICKS` adjustment — ticket 006's calibration pass does not specifically investigate whether scattered's bound can tighten; leave as-is unless a subsequent scenario-analysis run motivates a follow-up.
- Resolving the water-possession bottleneck — separate spec.
- New test additions — existing assertion surface is sufficient.

## Acceptance Criteria

### Tests That Must Pass

1. `golden_survival_contested` passes with `MAX_CRITICAL_RUN_TICKS = 300`.
2. Existing suite: `cargo test -p worldwake-ai --test golden_survival_contested`.

### Invariants

1. The rationale comment accurately describes the remaining architectural limit (water-possession bottleneck) so future tightening attempts have explicit context.
2. No production code change beyond the test constant and comment.

## Test Plan

### New/Modified Tests

1. None — documentation-only ticket at the comment level; the constant change exercises existing assertions. Verification is command-based and existing runtime coverage is named in Assumption Reassessment.

### Commands

1. `cargo test -p worldwake-ai --test golden_survival_contested`
2. `cargo clippy --workspace --all-targets -- -D warnings`
