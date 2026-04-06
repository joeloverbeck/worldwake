# S59EXPOBLSUB-007: report_missing and report_found actions

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — new actions in worldwake-systems
**Deps**: S59EXPOBLSUB-002, S59EXPOBLSUB-005

## Problem

Agents with overdue expectations need to formally report missing persons to offices or co-located agents. Agents who find missing persons need to report results. These two Social domain communication actions complete the report lifecycle.

## Assumption Reassessment (2026-04-06)

1. Action registration follows `register_*_action()` pattern called from `register_all_actions()` at `crates/worldwake-systems/src/action_registry.rs:23-55`.
2. `ActionDef` at `crates/worldwake-sim/src/action_def.rs:12` defines preconditions, duration, domain, visibility, payload.
3. `ActionHandler` at `crates/worldwake-sim/src/action_handler.rs:138` uses function pointer fields: on_start, on_tick, on_commit, on_abort, on_start_failure, affordance_targets, affordance_payloads, payload validators.
4. `ViolationKind::EntityMissing { entity, expected_place }` exists at `crates/worldwake-core/src/violation.rs:25-28`. `report_missing` creates this through the existing violation framework.
5. `ViolationMemory` component stores recorded violations with deduplication. Pattern from `justice_actions.rs`.
6. Tell mechanism at `crates/worldwake-systems/src/tell_actions.rs` provides the information propagation pattern that `report_found` uses.
7. `ActionDomain::Social` exists at `crates/worldwake-core/src/action_domain.rs:9`.

## Architecture Check

1. Both actions follow the standard ActionDef+ActionHandler pattern with no special infrastructure. `report_missing` reuses the existing violation framework. `report_found` reuses the existing Tell mechanism.
2. No backward compatibility shims.

## Verification Layers

1. report_missing creates ViolationKind::EntityMissing → action trace + event-log delta
2. report_found updates ExpectationRecord state → authoritative world state
3. report_found triggers Tell to expectation owner → action trace
4. Both actions have correct preconditions → focused unit test

## What to Change

### 1. Create report_missing action

Create `crates/worldwake-systems/src/report_actions.rs`:

**report_missing**:
- Domain: `ActionDomain::Social`
- Preconditions: Actor has overdue ExpectationRecord; actor is at a place
- Duration: Short (2-3 ticks, matching tell_action pattern)
- on_commit: Create `ViolationKind::EntityMissing` in actor's ViolationMemory. If an office with jurisdiction is at the place, create institutional record. Update expectation state.
- Affordance targets: co-located offices or agents
- Affordance payloads: enumerate from overdue expectations

### 2. Create report_found action

**report_found**:
- Domain: `ActionDomain::Social`
- Preconditions: Actor has a resolved search result (FoundAlive or FoundDead). Actor is at a place with interested parties.
- Duration: Short (2-3 ticks)
- on_commit: Notify expectation owner via Tell channels. If found dead, trigger corpse handling cascade event. Update institutional records if at an office.
- Affordance targets: co-located agents who are expectation owners or office holders
- Affordance payloads: enumerate from resolved search results

### 3. Register actions

In `crates/worldwake-systems/src/action_registry.rs`, add calls to `register_report_missing_action()` and `register_report_found_action()` in `register_all_actions()`.

### 4. Update action name list

In the `build_full_action_registries_returns_complete_action_catalog` test, add `"report_missing"` and `"report_found"` to the required names list.

## Files to Touch

- `crates/worldwake-systems/src/report_actions.rs` (new)
- `crates/worldwake-systems/src/lib.rs` (modify — add module)
- `crates/worldwake-systems/src/action_registry.rs` (modify — register + test)

## Out of Scope

- Candidate generation for ReportMissing goal — ticket 011
- PlannerOpKind classification — ticket 005 (already defined)
- ask_about_person, search_place, escort_to_safety — separate tickets

## Acceptance Criteria

### Tests That Must Pass

1. report_missing creates ViolationKind::EntityMissing with correct entity and expected_place
2. report_missing updates expectation state from Overdue
3. report_found resolves the expectation with correct outcome
4. report_found uses Tell mechanism to notify interested agents
5. Both actions reject when preconditions not met (no overdue expectations, no resolved search)
6. Action registry completeness test includes both action names
7. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. report_missing only fires for overdue expectations (not active or resolved)
2. report_found only fires when actor has a resolved search result
3. Information propagation uses existing Tell mechanism (P15, P26)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/report_actions.rs` — unit tests for both actions
2. `crates/worldwake-systems/src/action_registry.rs` — updated completeness test

### Commands

1. `cargo test -p worldwake-systems report`
2. `cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
