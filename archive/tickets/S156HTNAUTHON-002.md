# S156HTNAUTHON-002: Remove fake `AgentRole` precondition + orphaned `RoleTag`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` HTN method preconditions
**Deps**: archive/specs/S156-htn-authority-honesty.md (D2)

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

## Verified Layers

1. `group_hunt` remains selectable after `AgentRole` removal -> focused selector unit test in
   `htn/selector.rs` driving a belief state that satisfies `TargetBelievedDangerous` +
   `AllyOrBountyOfficeAvailable` and asserting `select_method` returns method id 3.
2. No-op gate is gone (no always-true precondition arm) -> `cargo clippy --workspace --all-targets
   -- -D warnings` passed after the variant removal.
3. Single-layer ticket: precondition evaluation is AI search-control with no authoritative-state
   or action-lifecycle effect — additional layer mapping is not applicable.

## Landed Changes

### 1. Removed the `AgentRole` variant and its selector arm

Deleted `AgentRole(RoleTag)` from `MethodPrecondition` (`method_schema.rs`) and the
`MethodPrecondition::AgentRole(_) => true` arm from `evaluate_precondition` (`htn/selector.rs`).

### 2. Removed the `AgentRole` precondition from `fulfill_bounty_group_hunt`

In `htn/methods.rs`, deleted the `MethodPrecondition::AgentRole(RoleTag::Hunter)` entry from the
method's precondition list. The two belief preconditions and all subgoals stayed unchanged.

### 3. Removed the orphaned `RoleTag` enum

Deleted the `RoleTag` enum definition and its re-export from `htn/mod.rs`. Removed the now-unused
`RoleTag` import from `htn/methods.rs`.

### 4. Updated the `method_schema_constructs_and_clones` unit test

In `method_schema.rs`, removed the `MethodPrecondition::AgentRole(RoleTag::Crafter)` line from the
fixture in `method_schema_constructs_and_clones`.

### 5. Added a `group_hunt`-selectable regression test (D7 distributed)

Added `canonical_group_hunt_selects_from_real_belief_preconditions` in `htn/selector.rs`, asserting
`fulfill_bounty_group_hunt` (id 3) is selected for an agent whose belief state satisfies
`TargetBelievedDangerous` and `AllyOrBountyOfficeAvailable`. The test proves the method stayed
selectable after the `AgentRole` removal.

## Landed Files

- `crates/worldwake-ai/src/htn/method_schema.rs` (modify)
- `crates/worldwake-ai/src/htn/methods.rs` (modify)
- `crates/worldwake-ai/src/htn/selector.rs` (modify)
- `crates/worldwake-ai/src/htn/mod.rs` (modify)

## Out of Scope

- `EntityCriterion` variants and the dead methods (`archive/tickets/S156HTNAUTHON-003.md`).
- `MethodSchema` field removal (`archive/tickets/S156HTNAUTHON-004.md`).
- Trace/fallback restructuring of the selector (S156HTNAUTHON-005).

## Acceptance Result

### Tests That Passed

1. New focused selector test proved `fulfill_bounty_group_hunt` is selectable for a qualifying
   agent after `AgentRole` removal.
2. `method_schema_constructs_and_clones` compiles and passes without the `AgentRole` precondition.
3. Existing suite passed: `cargo test -p worldwake-ai`.

### Invariants

1. Every surviving `MethodPrecondition` variant evaluates to a real, state-dependent result; no
   always-`true` arm remains.
2. `RoleTag` does not exist in live source after this ticket (FND-28: no orphaned type).

## Test Plan Result

### Focused Tests

1. `crates/worldwake-ai/src/htn/selector.rs` (test module) — added `group_hunt`-selectable test.
2. `crates/worldwake-ai/src/htn/method_schema.rs` (test module) — dropped `AgentRole` from the
   construct-and-clone fixture.

### Commands Run

1. Passed `cargo test -p worldwake-ai htn::selector`
2. Passed `cargo test -p worldwake-ai htn::method_schema`
3. Passed `cargo test -p worldwake-ai`
4. Passed `cargo clippy --workspace --all-targets -- -D warnings`
5. Waived `./scripts/verify.sh` for this iteration because the harness runs it once the full S156
   ticket family is complete and ready to push.

## Outcome

Completed on 2026-05-20.

- Removed the fake `MethodPrecondition::AgentRole` gate and its always-true selector branch.
- Removed the orphaned `RoleTag` enum and re-export rather than retaining a fossil role surface.
- Kept `fulfill_bounty_group_hunt`'s real belief preconditions and subgoals unchanged.
- Added focused selector coverage proving canonical group hunt selection from the actual
  `TargetBelievedDangerous` and `AllyOrBountyOfficeAvailable` belief preconditions.

## Verification Result

- Passed `rg -n "AgentRole|RoleTag" crates/worldwake-ai crates/worldwake-cli crates/worldwake-core crates/worldwake-sim crates/worldwake-systems archive/tickets/S156HTNAUTHON-002.md archive/specs/S156-htn-authority-honesty.md` showed only ticket/spec prose references after source edits.
- Passed `cargo test -p worldwake-ai htn::selector`
- Passed `cargo test -p worldwake-ai htn::method_schema`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Waived `./scripts/verify.sh` for this ticket iteration because the `implement-spec-tickets`
  harness owns the full pre-push verification gate after all S156 tickets are complete.
