# S156HTNAUTHON-002: Remove fake `AgentRole` precondition + orphaned `RoleTag`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` HTN method preconditions
**Deps**: specs/S156-htn-authority-honesty.md (D2)

## Problem

`MethodPrecondition::AgentRole(RoleTag)` is a no-op gate: the selector evaluates it as
`MethodPrecondition::AgentRole(_) => true` (`htn/selector.rs:77`) because the agent context
profile has no role field to check. Its only non-test use is in `fulfill_bounty_group_hunt`
(`htn/methods.rs:182`, `AgentRole(RoleTag::Hunter)`), where it filters nothing. A precondition
that always passes is fossilized logic the schema *looks* like it enforces but doesn't (FND-28).
This ticket removes the variant, the no-op selector arm, the precondition from `group_hunt`, and
the `RoleTag` enum (which becomes workspace-orphaned once `AgentRole` is gone).

## Assumption Reassessment (2026-05-20)

1. `MethodPrecondition` (`crates/worldwake-ai/src/htn/method_schema.rs:22-27`) has the variant
   `AgentRole(RoleTag)`. Its selector arm `MethodPrecondition::AgentRole(_) => true` is at
   `htn/selector.rs:77`. Confirmed always-true.
2. `RoleTag` (`crates/worldwake-ai/src/htn/method_schema.rs:121-130`, 7 variants) is used only via
   `MethodPrecondition::AgentRole`: construction in `fulfill_bounty_group_hunt` (`htn/methods.rs:182`,
   `RoleTag::Hunter`), in the unit test `method_schema_constructs_and_clones` (`htn/method_schema.rs:302`,
   `RoleTag::Crafter`), the type position at `method_schema.rs:25`, and a re-export at
   `htn/mod.rs:9`. Workspace grep confirms no other use — removing `AgentRole` orphans `RoleTag`.
3. Shared boundary under audit: the `MethodPrecondition` enum and the `evaluate_precondition`
   match in `htn/selector.rs` (the live precondition-evaluation surface). No cross-crate boundary —
   the cross-crate grep for `MethodPrecondition::AgentRole` / `RoleTag` outside worldwake-ai is empty.
4. `fulfill_bounty_group_hunt` (`htn/methods.rs:173-216`, method id 3) retains its real
   preconditions after the `AgentRole` line is removed: `BeliefHolds(TargetBelievedDangerous)`,
   `BeliefHolds(AllyOrBountyOfficeAvailable)`, and real subgoals (`DeclareSupport` → `TravelTo` →
   `Attack`). It remains selectable for a qualifying agent.
5. Existing tests on the changed surface: `htn/selector.rs` inline tests
   `select_method_returns_top_ranked_method_by_motive_score` (:847),
   `select_method_skips_methods_with_failed_preconditions` (:927),
   `select_method_is_deterministic_across_repeated_calls` (:1122); the unit test
   `method_schema_constructs_and_clones` (`method_schema.rs:293`) constructs `AgentRole(RoleTag::Crafter)`
   and must drop that precondition. No golden currently selects `fulfill_bounty_group_hunt`
   (golden `htn_methods.rs` covers id 1/2/12), so a new focused selectability test is added here.
6. Adjacent contradiction classification: none. `group_hunt`'s belief preconditions are real; the
   `AgentRole` gate was the only fake one. Removal does not reopen unrelated regressions.

## Architecture Check

1. Deleting a precondition that always evaluates `true` removes a fossil that misrepresented the
   schema's enforcement surface. The clean path (FND-28) is removal, not a documented "always
   passes" comment that would leave the dead arm live.
2. No shim: `RoleTag` is deleted outright (including its re-export) rather than retained as an
   unused type "in case roles return." Per the spec triage, role state returns later *with*
   enforcement, as a fresh design.

## Verification Layers

1. `group_hunt` remains selectable after `AgentRole` removal -> focused selector unit test in
   `htn/selector.rs` driving a belief state that satisfies `TargetBelievedDangerous` +
   `AllyOrBountyOfficeAvailable` and asserting `select_method` returns method id 3.
2. No-op gate is gone (no always-true precondition arm) -> `cargo clippy --workspace --all-targets
   -- -D warnings` (the removed variant must leave no unreachable/unused-match warnings).
3. Single-layer ticket: precondition evaluation is AI search-control with no authoritative-state
   or action-lifecycle effect — additional layer mapping is not applicable.

## What to Change

### 1. Remove the `AgentRole` variant and its selector arm

Delete `AgentRole(RoleTag)` from `MethodPrecondition` (`method_schema.rs`) and the
`MethodPrecondition::AgentRole(_) => true` arm from `evaluate_precondition` (`htn/selector.rs:77`).

### 2. Remove the `AgentRole` precondition from `fulfill_bounty_group_hunt`

In `htn/methods.rs`, delete the `MethodPrecondition::AgentRole(RoleTag::Hunter)` entry from the
method's precondition list (line ~182). Leave the two belief preconditions and all subgoals
unchanged.

### 3. Remove the orphaned `RoleTag` enum

Delete the `RoleTag` enum definition (`method_schema.rs:121-130`) and its re-export at
`htn/mod.rs:9`. Remove any now-unused imports of `RoleTag` (e.g. `htn/methods.rs:4`).

### 4. Update the `method_schema_constructs_and_clones` unit test

In `method_schema.rs`, remove the `MethodPrecondition::AgentRole(RoleTag::Crafter)` line from the
fixture in `method_schema_constructs_and_clones` (line ~302).

### 5. Add a `group_hunt`-selectable regression test (D7 distributed)

Add a focused unit test in `htn/selector.rs`'s test module asserting `fulfill_bounty_group_hunt`
(id 3) is selected (or at least passes preconditions) for an agent whose belief state satisfies
`TargetBelievedDangerous` and `AllyOrBountyOfficeAvailable`, proving the method stayed selectable
after the `AgentRole` removal.

## Files to Touch

- `crates/worldwake-ai/src/htn/method_schema.rs` (modify)
- `crates/worldwake-ai/src/htn/methods.rs` (modify)
- `crates/worldwake-ai/src/htn/selector.rs` (modify)
- `crates/worldwake-ai/src/htn/mod.rs` (modify)

## Out of Scope

- `EntityCriterion` variants and the dead methods (S156HTNAUTHON-003).
- `MethodSchema` field removal (S156HTNAUTHON-004).
- Trace/fallback restructuring of the selector (S156HTNAUTHON-005).

## Acceptance Criteria

### Tests That Must Pass

1. New focused selector test proves `fulfill_bounty_group_hunt` is selectable for a qualifying
   agent after `AgentRole` removal.
2. `method_schema_constructs_and_clones` compiles and passes without the `AgentRole` precondition.
3. Existing suite: `cargo test -p worldwake-ai`.

### Invariants

1. Every surviving `MethodPrecondition` variant evaluates to a real, state-dependent result (no
   always-`true` arm remains).
2. `RoleTag` does not exist anywhere in the workspace after this ticket (FND-28: no orphaned type).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/htn/selector.rs` (test module) — new `group_hunt`-selectable test.
2. `crates/worldwake-ai/src/htn/method_schema.rs` (test module) — drop `AgentRole` from the
   construct-and-clone fixture.

### Commands

1. `cargo test -p worldwake-ai htn::selector`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `./scripts/verify.sh` (before PR)
