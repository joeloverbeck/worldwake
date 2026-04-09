# S77BELCAPPRI-006: Reconcile scheduler-driven care integration with live affordance resolution

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — care-action affordance exposure or strict request-resolution expectations must be corrected
**Deps**: S08ACTSTAABORES-002

## Problem

Broad verification after `S77BELCAPPRI-005` still fails in `crates/worldwake-systems/tests/e09_needs_integration.rs::scheduler_driven_care_actions_apply_effects_and_preserve_conservation`. The test queues strict external requests for `eat`, `drink`, `sleep`, `toilet`, and `wash`, but the run now aborts with `TickStepError::RequestedAffordanceUnavailable` before the care sequence completes. This ticket must determine whether the live care-action affordance surface is wrong under the current scheduler/runtime contract or whether the integration harness assumptions are stale.

## Assumption Reassessment (2026-04-09)

1. `cargo test -p worldwake-systems` fails outside `S77BELCAPPRI-005`'s tell/listener boundary in `scheduler_driven_care_actions_apply_effects_and_preserve_conservation` at `crates/worldwake-systems/tests/e09_needs_integration.rs:277-351`.
2. The failure boundary is authoritative request resolution, not later action execution: `Harness::run_queued_action_to_completion()` at `crates/worldwake-systems/tests/e09_needs_integration.rs:111-120` calls `self.step_once().unwrap()`, and the failing run returns `TickStepError::RequestedAffordanceUnavailable` from the shared tick-step pipeline.
3. This is a mixed-boundary ticket. The exact contract under audit is strict external request resolution in `worldwake-sim::tick_step` versus the live care-action affordance surface registered by `register_needs_actions()` and exercised through the `e09` harness.
4. The motivating scenario is not a golden narrative but an integration proof for scheduler-driven care and conservation. The invariant is that, given the concrete harness setup in `e09_needs_integration`, each queued care request should remain lawfully requestable at the moment it is issued.
5. Existing archived coverage in `archive/tickets/S08ACTSTAABORES-002-heal-first-effect-medicine-conservation.md` identifies this same `e09` test as the cross-crate proof for scheduler-driven care. Any correction here must preserve that role rather than downgrading it into a weaker unit-only assertion.
6. Reassessment has not yet identified which queued request is first becoming unavailable or whether the break is caused by care affordance exposure, item/control preconditions, or a stricter request-resolution contract. That root cause must be established before code changes.
7. Adjacent contradiction classification: this is a separate bug exposed by broader subsystem verification after `S77BELCAPPRI-005`, not a required consequence of the tell metadata-loss fix.

## Architecture Check

1. Root-causing the first unavailable strict request at the shared request-resolution boundary is cleaner than weakening the integration test around an unknown contract drift. The fix should land at the real mismatch, whether that is the care affordance surface or the harness assumptions.
2. No backward-compatibility shims should be introduced. The ticket should either restore the lawful scheduler-driven care path or update the integration proof to the current canonical behavior if the prior expectation is no longer architecturally valid.

## Verification Layers

1. The first failing queued care request and its exact unavailable boundary are identified -> focused integration/runtime trace or narrowed failing test
2. The corrected care request resolves lawfully through strict external request handling -> focused `e09` integration proof
3. Scheduler-driven care still applies effects and preserves commodity conservation across the full sequence -> existing `scheduler_driven_care_actions_apply_effects_and_preserve_conservation`
4. Mixed-layer ticket: request resolution must be proven at the shared runtime boundary, not inferred only from later need deltas

## What to Change

### 1. Reassess the first unavailable care request

Trace the live `e09` sequence and identify exactly which queued request first fails as `RequestedAffordanceUnavailable`, including the concrete targets, current actor/place state, and shared resolution symbols involved.

### 2. Correct the live mismatch at the real boundary

If the care affordance exposure or action preconditions are wrong, fix production code at the owning layer. If the integration harness is asserting an obsolete request shape or stale setup assumption, update the test to the current lawful contract without weakening its conservation/care-effect coverage.

### 3. Keep the integration proof strong

Preserve an integration-level proof that scheduler-driven care still completes its sequence and preserves conservation under strict external requests, unless reassessment proves a narrower runtime contract is now the correct canonical boundary and a separate integration replacement is required.

## Files to Touch

- `crates/worldwake-sim/src/tick_step.rs` (modify if shared request resolution is wrong)
- `crates/worldwake-systems/tests/e09_needs_integration.rs` (modify if the integration harness assumptions are stale)
- `crates/worldwake-systems/src/*` (modify only if the owning care affordance/action surface is wrong after reassessment)

## Out of Scope

- Tell/listener belief-capacity behavior from `S77BELCAPPRI-005`
- Broad planner/golden expansion unless reassessment shows the care request drift is AI-visible
- Unrelated request-resolution paths outside the scheduler-driven care sequence

## Acceptance Criteria

### Tests That Must Pass

1. Focused: a proof identifying and covering the previously unavailable care request now passes
2. Existing integration: `cargo test -p worldwake-systems --test e09_needs_integration scheduler_driven_care_actions_apply_effects_and_preserve_conservation`
3. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. The `e09` scheduler-driven care sequence remains lawful under the live strict request-resolution contract.
2. Care-action integration coverage still proves both effect application and commodity conservation rather than relying only on lower-layer unit tests.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/tests/e09_needs_integration.rs` — tighten or correct the failing care-sequence proof once the first unavailable request is identified
2. Add one focused runtime/request-resolution proof at the owning layer if reassessment shows the bug sits below the integration harness

### Commands

1. `cargo test -p worldwake-systems --test e09_needs_integration scheduler_driven_care_actions_apply_effects_and_preserve_conservation`
2. `cargo test -p worldwake-systems`
3. `cargo clippy --workspace --all-targets -- -D warnings`
