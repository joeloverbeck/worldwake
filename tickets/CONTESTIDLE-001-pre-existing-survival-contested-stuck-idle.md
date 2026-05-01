# CONTESTIDLE-001: Investigate pre-existing survival-contested stuck-idle golden failure

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — likely `worldwake-ai` runtime/planning or scenario/golden calibration, pending reassessment
**Deps**: archive/tickets/RELIEFACT-001-per-need-relief-actionability-predicate.md, specs/IMPLEMENTATION-ORDER.md, docs/generated/golden-scenario-details/survival-contested.md, crates/worldwake-ai/tests/golden_survival_contested.rs

## Problem

During RELIEFACT-001 verification on 2026-05-01, the ignored
`golden_survival_contested::no_stuck_idle_windows_with_elevated_needs`
assertion failed:

```text
survival contested should have no idle windows >= 40 ticks with needs > 300 permille:
[StuckIdleWindow { agent_name: "Agent B", start_tick: 349, end_tick: 389, max_need_at_start: 799 }]
```

The same command failed in a clean temporary `HEAD` worktree at commit
`564ddcea`, so the failure predates RELIEFACT-001 and is not caused by
the per-need relief-actionability refactor.

`specs/IMPLEMENTATION-ORDER.md` still describes survival-contested as
landed with "no stuck idle windows >= 40 ticks", so either the golden
fixture/contract drifted, the roadmap status is stale, or a production
AI/runtime behavior regressed before RELIEFACT-001.

## Assumption Reassessment (2026-05-01)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The failing proof surface is golden E2E:
   `crates/worldwake-ai/tests/golden_survival_contested.rs::no_stuck_idle_windows_with_elevated_needs`.
2. The command run from the RELIEFACT-001 worktree failed:
   `cargo test --release -p worldwake-ai --test golden_survival_contested no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`.
3. The same command failed in a clean `HEAD` worktree at commit
   `564ddcea` with the same Agent B stuck-idle window, classifying this
   as a pre-existing blocker rather than current-ticket fallout.
4. The intended invariant, per the test and roadmap prose, is not
   "Agent B always picks a specific self-care branch." It is: in the
   authored survival-contested scenario, no AI agent should remain idle
   for at least 40 ticks while any homeostatic need is above 300
   permille.
5. This ticket must not assume the fix layer. Reassessment should map
   the tick-349..389 window across candidate generation, ranking /
   suppression, plan search / execution, authoritative idle/action
   state, and scenario isolation before editing code or relaxing the
   golden.
6. The live `GoalKind` and operator family under audit are unknown until
   traces for Agent B at the stuck window are inspected. Use decision
   traces and action traces before assigning blame to a specific
   self-care goal.

## Architecture Check

1. This ticket keeps the pre-existing golden failure out of unrelated
   refactors while giving it an explicit owner.
2. No compatibility shim or assertion weakening is authorized before
   reassessment proves whether the bug is production behavior,
   fixture/calibration drift, or stale roadmap prose.

## Verification Layers

1. Stuck-idle reproduction -> ignored golden command above.
2. Candidate/ranking/search cause -> Agent B decision trace around
   ticks 349..389.
3. Runtime action lifecycle cause -> action trace or scheduler state
   during the same window.
4. Scenario or roadmap drift -> compare
   `golden_survival_contested.rs`, generated golden docs, and
   `specs/IMPLEMENTATION-ORDER.md`.

## What to Change

### 1. Reproduce and classify the stuck window

Run the focused ignored golden and capture the Agent B decision/action
state around ticks 349..389. Classify the first failing boundary as
candidate absence, ranking/suppression, plan search, action start,
active action lifecycle, or authored scenario/golden drift.

### 2. Apply the narrowest truthful fix

If production AI/runtime behavior is wrong, fix that owning layer with a
focused lower-layer regression before rerunning the golden. If the
scenario/golden contract is stale, update the golden and generated docs
truthfully. If roadmap prose is stale, update only the false roadmap
claim.

## Files to Touch

- `crates/worldwake-ai/tests/golden_survival_contested.rs` (modify if the golden contract or tracing needs adjustment)
- `crates/worldwake-ai/src/**` (modify only if reassessment proves production AI ownership)
- `docs/generated/golden-scenario-details/survival-contested.md` and related generated golden docs (modify via generator if scenario metadata changes)
- `specs/IMPLEMENTATION-ORDER.md` (modify only if the landed-status prose is false)

## Out of Scope

- Changing RELIEFACT-001's per-need relief-actionability dispatch.
- Broad survival retuning without first proving the failure boundary.
- Weakening the stuck-idle assertion solely to make the suite green.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test --release -p worldwake-ai --test golden_survival_contested no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`
2. Any focused lower-layer regression added for the proved failure boundary.
3. `cargo test -p worldwake-ai`

### Invariants

1. The final ticket explains the first failing boundary for Agent B's
   tick-349..389 stuck window.
2. Roadmap, golden metadata, and executable assertion text do not make
   conflicting claims about survival-contested stuck-idle coverage.

## Test Plan

### New/Modified Tests

1. TBD after reassessment; prefer focused lower-layer coverage if
   production behavior is wrong.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_survival_contested no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`
2. `cargo test -p worldwake-ai`
