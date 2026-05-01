# CONTESTIDLE-001: Investigate pre-existing survival-contested stuck-idle golden failure

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` source-invalidation discrepancy scoping
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
6. Trace inspection identified the live stuck-window goal family as
   `AcquireCommodity(Water, SelfConsume)` before any code or golden
   assertion changes were made.
7. Reproduced on the live branch with the same ignored golden command.
   Agent B was idle from ticks 349..389 while its maximum need was 799.
8. Decision traces during ticks 349..389 showed the AI tick was running,
   but candidate generation/ranking produced zero ranked candidates.
   There were no search attempts, selected plans, active actions, or
   action lifecycle failures during the stuck window.
9. At tick 356, Agent B's thirst was critical, but both water
   acquisition opportunities were suppressed by a prior
   `SourceInvalidated` discrepancy recorded for the whole water goal:
   `place = None`, `target = None`, `action_def = None`, expiring at
   tick 430. That made one invalidated committed water source suppress
   alternate lawful water sources.
10. The first failing boundary was candidate suppression before search,
    not ranking priority, planner budget, action start, active-action
    lifecycle, or scenario/golden drift.

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

## Completed Changes

### 1. Reproduced and classified the stuck window

Ran the focused ignored golden and captured Agent B decision/action
state around ticks 349..389. The first failing boundary was candidate
suppression caused by an over-broad `SourceInvalidated` discrepancy.

### 2. Applied the narrowest truthful fix

Changed production AI source-invalidation handling so a failed committed
source suppresses that source entity rather than every same-goal source.
No scenario, golden metadata, generated docs, or roadmap prose changes
were needed.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/frame.rs`
- `crates/worldwake-ai/src/agent_tick/mod.rs`
- `crates/worldwake-ai/src/agent_tick/tests.rs`

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

1. Updated source-invalidation unit coverage so a committed-source
   reliability failure records the failed source entity as the
   discrepancy target.

### Commands

1. `cargo test --release -p worldwake-ai --test golden_survival_contested no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`
2. `cargo test -p worldwake-ai`

## Outcome

Completed on 2026-05-01.

- Fixed production AI failure handling so committed-source reliability
  invalidation records `SourceInvalidated` against the failed source
  entity instead of the entire goal family.
- This keeps the failed source suppressed while allowing alternate
  same-goal sources to remain candidates. In survival-contested, Agent B
  can now pursue another water source instead of sitting idle through
  the tick-349..389 elevated-need window.
- No golden, generated docs, or roadmap prose changes were needed. The
  original survival-contested no-stuck-idle contract is true after the
  runtime fix.
- No save-format version bump was required because the serialized
  discrepancy shape did not change; only the recorded blocker key values
  changed.

## Verification Result

- Initially reproduced failure with
  `cargo test --release -p worldwake-ai --test golden_survival_contested no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`.
- Passed
  `cargo test -p worldwake-ai agent_tick::frame::tests::record_source_invalidation_scopes_suppression_to_committed_source_target -- --exact`.
- Passed
  `cargo test --release -p worldwake-ai --test golden_survival_contested no_stuck_idle_windows_with_elevated_needs -- --ignored --test-threads=1`.
- Passed `cargo test -p worldwake-ai`.
