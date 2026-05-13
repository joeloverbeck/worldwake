# S137PLACAULIN-007: Revalidation routing — attempt_repair_then_replan at the invalidator seam

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `agent_tick/execution.rs` revalidation seam; payload revalidation for RebindTarget
**Deps**: 006 (plan_repair module)

## Problem

S137 D6 inserts `attempt_repair_then_replan` into the agent-tick revalidation seam at `crates/worldwake-ai/src/agent_tick/execution.rs:90-146`. Today, an `Invalidator` breach falls straight through to `handle_current_step_failure`, which triggers full replan and emits `EventTag::ReplanTriggered`. After this ticket, breaches first attempt bounded localized repair; only if repair returns `Failed` does the agent fall through to the existing full-replan path. Without this routing change, ticket 006's repair module is unreachable from the live agent decision cycle.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The revalidation seam is at `crates/worldwake-ai/src/agent_tick/execution.rs:90-146` inside `enqueue_valid_step_or_handle_failure`. `classify_revalidation` at line 90 returns `RevalidationOutcome::Valid` or `RevalidationOutcome::Invalidated { reason, expectation_kind, mismatch_detail }`. Today the `Invalidated` branch (lines 102-109) extracts the reason and falls straight through to `handle_current_step_failure` (lines 133-146), which builds a `ReplanTriggered` event. Test boundary at line 844 — runtime path. `attempt_repair_then_replan` is added by ticket 006.
2. Spec `specs/S137-plan-causal-links-and-repair.md` D6 specifies the routing: invoke repair first; on `Repaired`, emit `EventTag::RepairApplied` with the chosen kind and replace the plan; on `Failed`, record attempts in `RepairMemory` and fall through to `handle_current_step_failure`.
3. Shared boundary: the revalidation seam itself — the data contract between `classify_revalidation`'s output and the breach-handling code. The new repair attempt operates on the same `RevalidationOutcome::Invalidated` input shape; the change is which code path consumes it first.
4. **Live `GoalKind` under test**: the revalidation seam is goal-agnostic — it fires for any plan step whose guard's `Invalidator` triggers. Existing goldens exercise `GoalKind::TravelTo`, `Trade`, `ProduceCommodity`, `Sleep`. The repair routing path applies uniformly across all goal families.
5. **Ordering contract**: the routing is action-lifecycle-internal — repair attempt and full-replan fall-through both run within the same `agent_tick` call. No tick separation. The contract is *sequential ordering inside the agent tick*: repair-attempt runs first; full-replan runs only on `Failed`. The compared branches (repair-success vs. repair-failure) are symmetric in cost in that both terminate the revalidation pass; the divergence is whether `EventTag::RepairApplied` or `EventTag::ReplanTriggered` is emitted.
6. **Authoritative-to-AI Impact Rule**: payload revalidation applies for `RebindTarget`. When repair synthesizes a new payload (e.g., a different target entity), the action handler's `with_payload_override_validator` registration must accept the synthesized payload. Audit affected handlers (travel, trade, harvest, craft) during reassessment — confirm validators don't reject the rebound target. If a validator rejects, the action handler's registration needs updating in-scope.
7. **Adjacent contradiction**: the existing `EventTag::ReplanTriggered` emission at lines 153-167 fires after `handle_current_step_failure` returns a `replan_reason`. After this ticket: when repair succeeds, `EventTag::RepairApplied` fires instead; when repair fails, the existing `ReplanTriggered` path runs unchanged. The `decisive_*` evidence collection at line 148 remains valid in both branches. Classified as required consequence — both event tags coexist (each fires in mutually exclusive cases).

## Architecture Check

1. **Minimal seam change**: the only added code path is the pre-failure `attempt_repair_then_replan` call. The existing `handle_current_step_failure` lower path is untouched and runs unchanged on `RepairOutcome::Failed`.
2. **FND-29-aligned**: both `RepairApplied` and `ReplanTriggered` are surfaced as decision events; observer (ticket 009) renders both. The revalidation seam stays inspectable.

## Verification Layers

1. Repair-success path emits `RepairApplied` and replaces the plan → action trace + event-log delta in `agent_tick/execution.rs` `#[cfg(test)]` runtime test.
2. Repair-failure path emits `ReplanTriggered` (existing behavior preserved) → action trace + event-log delta in the same runtime test, asserting fall-through to `handle_current_step_failure`.
3. Authoritative payload revalidation for `RebindTarget` → ticket 010 (golden coverage) plus a focused runtime test asserting an action handler's `with_payload_override_validator` accepts the rebound payload.
4. Mixed-layer ticket — repair-routing decision (AI layer) + plan-replacement (authoritative runtime state) + event-log emission. Each invariant maps to a distinct surface above.

## What to Change

### 1. Insert `attempt_repair_then_replan` at the invalidator-handling site

In `crates/worldwake-ai/src/agent_tick/execution.rs:102-146`, restructure the `Invalidated` branch to:

```rust
let (plan_invalidation_reason, expectation_kind, mismatch_detail) = match classification {
    RevalidationOutcome::Valid => (None, None, None),
    RevalidationOutcome::Invalidated { reason, expectation_kind, mismatch_detail } => {
        let repair_outcome = attempt_repair_then_replan(
            runtime, ctx.cognitive, repair_memory, discrepancy_memory,
            /* … other context fields … */
        );
        match repair_outcome {
            RepairOutcome::Repaired { kind, new_plan } => {
                emit_decision_event(
                    ctx.event_log, tick, agent, EventTag::RepairApplied,
                    DecisionEventPayload::RepairApplied(RepairAppliedPayload {
                        agent, goal_key, step_index, repair_kind: kind,
                        substitute_target: /* derived from new_plan */,
                        substitute_recipe: /* derived from new_plan */,
                    }),
                );
                replace_current_plan(runtime, new_plan);
                return Ok(());
            }
            RepairOutcome::Failed { tried } => {
                record_repair_attempts(repair_memory, &tried, tick, ctx.cognitive);
                (Some(reason), expectation_kind, mismatch_detail)
            }
        }
    }
};
// existing path continues with handle_current_step_failure → ReplanTriggered emission
```

### 2. Payload revalidation audit for `RebindTarget`

Grep `with_payload_override_validator` across action handler registrations. For each handler that may receive a rebound target (travel, trade, harvest, craft handlers), verify the validator accepts the synthesized payload. If any validator rejects, update its registration in this ticket — splitting to a follow-up would leave repair in a half-working state.

### 3. Record repair attempts in `RepairMemory`

Implement `record_repair_attempts` writing `RepairEntry { signature, kind, succeeded: false, observed_tick, expires_tick, success_count: 0 }` for each `(RepairKind, RepairFailure)` in `tried`. The TTL is governed by `cognitive.repair_memory_ticks` (existing field).

## Files to Touch

- `crates/worldwake-ai/src/agent_tick/execution.rs` (modify — revalidation seam at 90-146, runtime tests at 844+)
- Likely: `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — `replace_current_plan` helper if it lives here; otherwise create in execution.rs)
- Likely: `crates/worldwake-systems/src/` action handler registration files (modify — `with_payload_override_validator` updates if audit surfaces rejections; grep `with_payload_override_validator` to confirm)

## Out of Scope

- Decision-trace `RepairAttemptTrace` emission — ticket 008.
- Observer rendering of new event — ticket 009.
- Golden coverage proving the routing path — ticket 010.
- `classify_accepted_repair` (post-hoc classifier) modification — preserved unchanged as the full-replan fall-through.

## Acceptance Criteria

### Tests That Must Pass

1. `cargo test -p worldwake-ai agent_tick::execution` — new runtime tests for both repair-success and repair-failure paths.
2. `cargo test -p worldwake-ai` — existing agent-tick tests pass; no regression in the `Invalidated` branch's fall-through behavior.
3. Existing suite: `cargo test --workspace`.

### Invariants

1. `EventTag::RepairApplied` fires exactly when `attempt_repair_then_replan` returns `Repaired`; otherwise `EventTag::ReplanTriggered` fires (preserved behavior).
2. `RepairMemory.repairs[signature].succeeded == false` records every failed `(BreachSignature, RepairKind)` pair from `RepairOutcome::Failed.tried`.
3. The `Authoritative-to-AI Impact Rule` checklist applies: `with_payload_override_validator` accepts synthesized payloads for `RebindTarget` cases.
4. No `EventTag::RepairApplied` fires without a corresponding plan replacement in `AgentDecisionRuntime.current_plan`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick/execution.rs` `#[cfg(test)]` — new tests:
   - `repair_success_emits_repair_applied_and_replaces_plan`
   - `repair_failure_falls_through_to_full_replan`
   - `repair_failure_records_attempts_in_repair_memory`
2. If action handler validators need updates: focused test in the affected handler module asserting rebound payload acceptance.

### Commands

1. `cargo test -p worldwake-ai agent_tick::execution`
2. `cargo test -p worldwake-ai`
3. `cargo test --workspace`
4. `cargo clippy --workspace --all-targets -- -D warnings`
