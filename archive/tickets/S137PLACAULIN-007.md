# S137PLACAULIN-007: Revalidation routing — attempt_repair_then_replan at the invalidator seam

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `agent_tick/execution.rs` revalidation seam; payload revalidation for RebindTarget
**Deps**: archive/tickets/S137PLACAULIN-006.md (plan_repair module and causal-link emission), archive/tickets/S137PLACAULIN-011.md (successful localized repair handlers)

## Problem

S137 D6 inserted `attempt_repair_then_replan` into the agent-tick revalidation seam at `crates/worldwake-ai/src/agent_tick/execution.rs:90-146`. Before this ticket, an `Invalidator` breach fell straight through to `handle_current_step_failure`, triggering full replan and emitting `EventTag::ReplanTriggered`. After this ticket, breaches with causal-link context attempt bounded localized repair first; only `RepairOutcome::Failed` falls through to the existing full-replan path. The ticket-006 repair module and ticket-011 successful handlers are now reachable from the live agent decision cycle.

## Reassessment Result (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. The revalidation seam is `crates/worldwake-ai/src/agent_tick/execution.rs` inside `enqueue_valid_step_or_handle_failure`. Before this ticket, `classify_revalidation` returned `RevalidationOutcome::Invalidated { reason, expectation_kind, mismatch_detail }` and the `Invalidated` branch extracted the reason and fell straight through to `handle_current_step_failure`, which built a `ReplanTriggered` event. The landed routing now attempts `attempt_repair_then_replan` for expectation mismatches with causal-link context before that fall-through.
2. Spec `specs/S137-plan-causal-links-and-repair.md` D6 specifies the routing: invoke repair first; on `Repaired`, emit `EventTag::RepairApplied` with the chosen kind and replace the plan; on `Failed`, record attempts in `RepairMemory` and fall through to `handle_current_step_failure`.
3. Shared boundary: the revalidation seam itself — the data contract between `classify_revalidation`'s output and the breach-handling code. The new repair attempt operates on the same `RevalidationOutcome::Invalidated` input shape; the change is which code path consumes it first.
4. **Live `GoalKind` under test**: the revalidation seam is goal-agnostic — it fires for any plan step whose guard's `Invalidator` triggers. Existing goldens exercise `GoalKind::TravelTo`, `Trade`, `ProduceCommodity`, `Sleep`. The repair routing path applies uniformly across all goal families.
5. **Ordering contract**: the routing is action-lifecycle-internal — repair attempt and full-replan fall-through both run within the same `agent_tick` call. No tick separation. The contract is *sequential ordering inside the agent tick*: repair-attempt runs first; full-replan runs only on `Failed`. The compared branches (repair-success vs. repair-failure) are symmetric in cost in that both terminate the revalidation pass; the divergence is whether `EventTag::RepairApplied` or `EventTag::ReplanTriggered` is emitted.
6. **Authoritative-to-AI Impact Rule**: payload revalidation applies for `RebindTarget`. When repair synthesizes a new payload (e.g., a different target entity), the action handler's `with_payload_override_validator` registration must accept the synthesized payload. Audit affected handlers (travel, trade, harvest, craft) during reassessment — confirm validators don't reject the rebound target. If a validator rejects, the action handler's registration needs updating in-scope.
7. **Adjacent contradiction**: the existing `EventTag::ReplanTriggered` emission at lines 153-167 fires after `handle_current_step_failure` returns a `replan_reason`. After this ticket: when repair succeeds, `EventTag::RepairApplied` fires instead; when repair fails, the existing `ReplanTriggered` path runs unchanged. The `decisive_*` evidence collection at line 148 remains valid in both branches. Classified as required consequence — both event tags coexist (each fires in mutually exclusive cases).

## Architecture Check

1. **Minimal seam change**: the only added code path is the pre-failure `attempt_repair_then_replan` call. The existing `handle_current_step_failure` lower path is untouched and runs unchanged on `RepairOutcome::Failed`.
2. **FND-29-aligned**: both `RepairApplied` and `ReplanTriggered` are surfaced as decision events; observer (ticket 009) renders both. The revalidation seam stays inspectable.

## Verification Layers Reassessed

1. Repair-success path emits `RepairApplied` and replaces the plan. This ticket proves that runtime seam in `agent_tick/execution.rs` with `local_repair_success_emits_repair_applied_and_replaces_plan`.
2. Repair-failure path records failed attempts, then preserves the existing full-replan fall-through behavior. This ticket proves the memory entry shape with `local_repair_failure_records_failed_attempts_in_repair_memory`; existing crate/workspace tests cover the unchanged fall-through.
3. Authoritative payload revalidation for concrete `RebindTarget` goldens remains ticket 010. This ticket audited the live validators and did not need handler changes.
4. Mixed-layer routing now spans repair-routing decision (AI layer), plan replacement (authoritative runtime state), and event-log emission.

## Implemented Scope

1. `crates/worldwake-ai/src/agent_tick/execution.rs` now routes `RevalidationOutcome::Invalidated` expectation mismatches with causal-link context through `attempt_repair_then_replan` before `handle_current_step_failure`.
2. `RepairOutcome::Repaired` installs the repaired plan into `AgentDecisionRuntime.current_plan`, preserves the current plan's committed source and expectation metadata, clears in-flight repair state, and emits `EventTag::RepairApplied` with `RepairAppliedPayload`.
3. `RepairOutcome::Failed` records failed repair attempts in `RepairMemory` with `succeeded: false`, the live `BreachSignature`, `observed_tick`, `expires_tick` derived from `cognitive.repair_memory_ticks`, and `success_count: 0`, then falls through to the existing `handle_current_step_failure` and `ReplanTriggered` behavior.
4. `crates/worldwake-ai/src/agent_tick/mod.rs` and `crates/worldwake-ai/src/agent_tick/tests.rs` were updated for the mutable repair-memory handoff and memory-capacity enforcement.
5. `RepairPlanCandidate` now carries `reusable_suffix_index` provenance so a candidate promoted from the reusable suffix is not appended a second time when `plan_repair` composes `preserved_prefix + candidate.step + reusable_suffix`.
6. `RepairAppliedPayload.substitute_target` is derived from the repaired plan step at the repaired `step_index`, not from the first step in the plan, so prefix-preserving repairs report the substituted target from the active repaired step.

## No-Change Audits

1. `crates/worldwake-ai/src/plan_repair.rs` was used as the live API boundary; strategy logic stayed there.
2. `with_payload_override_validator` registrations were audited across `crates/worldwake-systems/src/` and the AI test registries. The travel path has no payload override validator, and the relevant trade/harvest/craft payload validation did not require code changes for this routing seam.
3. `classify_accepted_repair` stayed unchanged as the post-hoc classifier for the full-replan fall-through path.

## Out of Scope

1. Decision-trace `RepairAttemptTrace` emission remains ticket 008.
2. Observer rendering of the new repair path remains ticket 009.
3. Golden coverage proving the routing path remains ticket 010.

## Acceptance Result

1. `EventTag::RepairApplied` is emitted by the execution seam when localized repair returns `Repaired`; the focused test `local_repair_success_emits_repair_applied_and_replaces_plan` proves the event and plan replacement together.
2. Failed localized repair attempts are recorded in `RepairMemory` with `succeeded == false`; the focused test `local_repair_failure_records_failed_attempts_in_repair_memory` proves the persisted entry shape.
3. Full-replan fall-through remains the existing `handle_current_step_failure` path after `RepairOutcome::Failed`; existing `worldwake-ai` and workspace tests passed unchanged.

## Post-ticket Review Resolution (2026-05-13)

The review blocker is resolved. `repair_candidates_from_reusable_suffix` now annotates suffix-sourced `RepairPlanCandidate`s with the originating suffix index, and `plan_repair::plan_from_parts` skips that index when appending the reusable suffix. Focused tests prove both the strategy boundary and execution seam:

1. `suffix_sourced_candidate_is_promoted_without_duplication` proves `plan_repair` promotes a suffix-sourced candidate without duplicating the promoted step.
2. `suffix_sourced_local_repair_promotes_candidate_without_duplication` proves the live execution seam installs the prefix plus promoted suffix candidate once and emits a `RepairAppliedPayload` whose `substitute_target` comes from the active repaired step.

## Implementation Pass Result

Implementation pass on 2026-05-13.

- Landed the S137 D6 revalidation routing in `agent_tick/execution.rs`.
- Added focused execution tests for repair-success event/plan replacement and failed-attempt repair memory.
- Resolved the post-review suffix-candidate blocker by adding candidate provenance and focused no-duplication coverage.
- Kept action handler validator code unchanged after audit; no validator rejection surfaced at this seam.
- Preserved sibling ownership for trace emission, observer rendering, and golden coverage.

## Verification Result

- Passed `cargo test -p worldwake-ai agent_tick::execution`
- Passed `cargo test -p worldwake-ai plan_repair`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `cargo test --workspace`

## Outcome

Completed on 2026-05-13. The revalidation seam now attempts bounded localized repair before full-replan fallback, records failed repair attempts, emits `RepairApplied` on successful plan replacement, and preserves suffix-sourced repair candidates without duplicating promoted suffix steps.
