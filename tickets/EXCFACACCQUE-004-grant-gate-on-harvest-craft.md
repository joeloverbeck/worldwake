# EXCFACACCQUE-004 — Grant Requirement Gate on Harvest and Craft Actions

**Spec sections**: §5, §6, §13
**Crates**: `worldwake-systems`

## Summary

Update harvest and craft action preconditions to require a matching `GrantedFacilityUse` before the exclusive operation can start. Starting the real operation consumes the grant. After completion, the agent must re-enter the queue for another turn.

This ticket also closes the architecture gap left intentionally open by `EXCFACACCQUE-003`: the action-start gate and the queue-head promotion logic must share one authoritative notion of exclusive-facility startability. Do not let queue promotion and harvest/craft start validation evolve into parallel legality engines.

## Deliverables

### 1. Add grant-check precondition to harvest actions

In `crates/worldwake-systems/src/production_actions.rs`, update `harvest_action_def()` (or equivalent) to add a precondition:
- Facility has a `FacilityUseQueue` with a `granted` entry matching `(actor, this ActionDefId)`

This can be a new `Precondition` variant (e.g., `Precondition::FacilityGrantExists { target_index, actor_must_match: true }`) or a custom validation check in the handler's `start` function. Follow whichever pattern is more consistent with existing precondition infrastructure.

### 2. Add grant-check precondition to craft actions

Same change in `crates/worldwake-systems/src/production_actions.rs` for craft action definitions.

### 3. Consume grant on action start

In the `start_harvest` and `start_craft` handler functions, after precondition validation succeeds:
- Call `FacilityUseQueue::clear_grant()` on the facility
- This happens atomically with the reservation lock acquisition

### 4. Unify queue promotability with action startability

`EXCFACACCQUE-003` currently promotes queue heads by reusing shared action constraint/precondition evaluation. This ticket must preserve that architecture:
- Extract or reuse a single helper/classifier for \"exclusive facility action is startable now for `(actor, facility, ActionDefId)`\"
- `facility_queue_system` promotion and `start_harvest` / `start_craft` grant-gated validation must both depend on that same source of truth
- The grant-specific check itself may remain layered on top, but base readiness rules for stock, workstation shape, staged craft occupancy, tools, and recipe knowledge must not be duplicated in two diverging code paths

### 5. One grant = one operation

No changes needed beyond the above — the grant is consumed on start, so the agent cannot start a second operation without re-entering the queue. Verify this is the case in tests.

### 6. No compatibility layer

Remove or disable any path where an autonomous agent can start a harvest/craft at an exclusive facility without a matching grant. The existing best-effort autonomous request handling may remain as a generic engine safety net, but it must not be exercised in the normal contested-harvest path.

## Files to Touch

- `crates/worldwake-systems/src/production_actions.rs` — add grant precondition to harvest and craft defs, consume grant in start handlers
- `crates/worldwake-sim/src/action_semantics.rs` — possibly add `FacilityGrantExists` precondition variant
- `crates/worldwake-sim/src/action_validation.rs` — validation logic for new precondition (if using Precondition variant)

## Out of Scope

- Queue types or component registration (EXCFACACCQUE-001)
- `queue_for_facility_use` action (EXCFACACCQUE-002)
- `facility_queue_system` (EXCFACACCQUE-003)
- AI planner changes (EXCFACACCQUE-007–010)
- Belief views (EXCFACACCQUE-005)
- Non-exclusive actions (eat, drink, sleep, travel, transport, trade, combat) — these remain unchanged

## Acceptance Criteria

### Tests that must pass
- Unit test: starting a harvest without a matching grant fails precondition check (even if facility is otherwise valid and stocked)
- Unit test: starting a harvest with a matching grant succeeds and the grant is consumed (facility queue `granted` becomes `None`)
- Unit test: starting a craft without a matching grant fails
- Unit test: starting a craft with a matching grant succeeds and consumes the grant
- Unit test: after grant is consumed, a second harvest start by the same actor fails (must re-queue)
- Unit test: grant for `ActionDefId::X` does not authorize `ActionDefId::Y` on the same facility
- Unit test: non-exclusive actions (eat, drink, travel) are unaffected — no grant check
- Unit test: queue-head promotion readiness and action-start readiness stay aligned for a temporarily blocked facility (for example depleted stock or occupied craft workstation)
- Unit test: queue-head promotion readiness and action-start readiness stay aligned for a structurally invalid facility (for example workstation tag removed)
- Existing harvest/craft tests updated to set up grants before starting actions (backward compatibility of test infrastructure)
- `cargo test --workspace` — no regressions

### Invariants that must remain true
- Every exclusive facility start requires a matching grant
- Grant is consumed exactly once per exclusive action start
- Reservation lock still prevents overlap (grants complement, not replace, reservations)
- Item conservation remains enforced — this ticket does not change what harvest/craft produce
- Non-exclusive actions are completely unaffected
- Queue promotion and action start share one authoritative readiness classifier; no duplicated legality logic may drift between `EXCFACACCQUE-003` and this ticket
