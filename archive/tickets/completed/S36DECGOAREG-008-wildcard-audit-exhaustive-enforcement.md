# S36DECGOAREG-008: Wildcard audit and legitimate-default documentation

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` decision-trace wildcard documentation and focused trace regression tests
**Deps**: S36DECGOAREG-005, S36DECGOAREG-006, S36DECGOAREG-007, S36DECGOAREG-009

## Problem

S36’s declaration migration was intended to remove wildcard dispatch shortcuts where adding a new goal shape should force architectural review. The remaining task is no longer a broad AI-crate wildcard cleanup. Reassessment shows the dispatch surfaces already migrated to exhaustive declaration-owned routing, while the residual in-scope wildcards are narrow, legitimate defaults in decision-trace omission helpers. The ticket must therefore document the real end-state and harden proof for those legitimate defaults instead of performing stale cleanup.

## Assumption Reassessment (2026-03-29)

1. The exact AI abstraction boundary under audit is the declaration-owned dispatch path `GoalKind -> GoalDispatchKey -> GoalDispatchDeclaration` across [`crates/worldwake-ai/src/goal_dispatch_key.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs), [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs), [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs), [`crates/worldwake-ai/src/feasibility.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs), and trace rendering in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs).
2. The priority S36 dispatch sites named by the spec are already exhaustive in live code. `GoalDispatchKey::from_goal_kind()` in [`crates/worldwake-ai/src/goal_dispatch_key.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs) and `GoalDispatchKey::declaration()` in [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) use exhaustive `match`es with no wildcard arms. `derive_invalidation_conditions()` in [`crates/worldwake-ai/src/exhaustion.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/exhaustion.rs) and `goal_specific_feasibility()` in [`crates/worldwake-ai/src/feasibility.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/feasibility.rs) now route through declaration-selected strategy enums rather than direct wildcarded `GoalKind` dispatch.
3. The ticket’s original `GoalKindTag` assumption is stale. Repository search currently returns no `GoalKindTag` or `goal_kind_tag` symbols in `worldwake-ai` or `worldwake-core`; ticket 009 already removed that shadow identity, so there is no remaining coarse-tag dispatch surface to audit here.
4. The ticket’s original `goal_model.rs` cleanup assumptions are stale. `GoalKindPlannerExt::relevant_op_kinds()` in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) already delegates to `GoalDispatchKey::from_goal_kind(self).declaration().relevant_ops`, and the old `*_OPS` arrays now live in [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) as active declaration data. They are not dead code and should not be removed by this ticket.
5. Remaining wildcard arms in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) are still concentrated in goal-semantic methods such as `build_payload_override()`, `apply_planner_step()`, `is_progress_barrier()`, `is_satisfied()`, and `goal_relevant_places()`. These are goal-behavior methods explicitly excluded from S36 by [`S36-declarative-goal-registration.md`](/home/joeloverbeck/projects/worldwake/archive/specs/S36-declarative-goal-registration.md), not declaration-routing shortcuts.
6. The remaining in-scope `GoalKind` wildcard defaults are in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs): `omitted_political_reason_for_goal()` and `omitted_social_reason_for_goal()`. In both helpers, `_ => None` is the correct contract for goals outside the political or social omission family. Those defaults are legitimate because the omission lists themselves are family-scoped, not universal goal registries.
7. The original proposed `#[deny(unreachable_patterns)]` work is not the right architecture. It does not create stronger completeness guarantees than the existing exhaustive `match`es already provide, and it would not meaningfully improve the declaration contract. The cleaner robust architecture is to rely on the current exhaustive matches plus focused tests over the legitimate-default wildcard helpers.
8. The original proposed compile-time check “add a dummy `GoalKind` variant in a test build and confirm compilation fails” is not a real repository verification command. The live proof surface is existing exhaustive compile-checked matches plus focused unit coverage. `cargo test -p worldwake-ai -- --list` confirms the current focused targets are real and includes `goal_dispatch_key`, `goal_dispatch_decl`, `feasibility`, and `decision_trace` unit modules.
9. Existing focused coverage already proves most of S36’s delivered contract: [`crates/worldwake-ai/src/goal_dispatch_key.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs) has payload-sensitive split and exhaustiveness tests; [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs) has declaration completeness, provenance parity, relevant-op parity, and strategy assignment tests; [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) already proves declaration trace labels are used in summaries. The remaining gap is explicit regression coverage for the legitimate omission-default wildcards.
10. Scope correction: this is not a “documentation-only” ticket. The production code needs narrow inline documentation on the legitimate wildcard defaults, and the test surface should add focused regression coverage so future edits do not silently broaden or break those omission-family contracts.
11. Adjacent contradiction classification: the stale parts of the original ticket are not separate bugs in code; they are outdated ticket assumptions caused by prior S36 deliveries. The ticket must be corrected first, then only the residual trace-default work should proceed.

## Architecture Check

1. The cleaner long-term architecture is to keep S36’s compile-time completeness guarantees at the real dispatch boundary only: exhaustive `GoalKind -> GoalDispatchKey` and `GoalDispatchKey -> GoalDispatchDeclaration` matches, plus strategy enums for dynamic routing. Reopening already-clean dispatch sites with lint attributes or dead-code cleanup would add noise without strengthening the architecture.
2. Legitimate-default wildcards should remain only where the data contract is explicitly family-scoped rather than exhaustive over all goals. `decision_trace` omission helpers are such a case: they answer “does this goal belong to the omission family recorded in this trace bucket?” Returning `None` for other goal families is the correct stable default.
3. No backwards-compatibility aliasing or fallback dispatch paths are introduced. The ticket tightens documentation and proof around the existing declaration architecture rather than reintroducing tags, aliases, or parallel routing logic.

## Verification Layers

1. Exhaustive declaration routing remains intact -> compile-checked exhaustive matches in [`crates/worldwake-ai/src/goal_dispatch_key.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_key.rs) and [`crates/worldwake-ai/src/goal_dispatch_decl.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_dispatch_decl.rs), plus focused `goal_dispatch_key` / `goal_dispatch_decl` unit coverage.
2. Legitimate wildcard-default behavior for political/social omission helpers -> focused `decision_trace` unit tests.
3. Trace rendering still uses declaration labels rather than ad-hoc `Debug` strings -> focused `decision_trace` unit tests.
4. AI-crate regression guard for the audit cleanup -> `cargo test -p worldwake-ai`.
5. Workspace-level regression and lint cleanliness -> `cargo test --workspace` and `cargo clippy --workspace`.
6. Single-crate architectural cleanup ticket: action traces and authoritative event-log proofs are not applicable because this work touches AI-internal dispatch/trace metadata, not action lifecycle or world mutation ordering.

## What to Change

### 1. Document the legitimate omission-family wildcard defaults

Add succinct inline comments in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) on the `_ => None` arms in:
- `omitted_political_reason_for_goal()`
- `omitted_social_reason_for_goal()`

The comments must explain that these helpers intentionally answer only for one omission family, so non-family goals correctly produce `None`.

### 2. Add focused regression coverage for omission-family matching

Strengthen [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs) tests to prove:
- political omissions match only the intended political goal family and return `None` for unrelated goals,
- social omissions match only `ShareBelief` goals and return `None` for unrelated goals.

### 3. Leave goal-semantic wildcard sites unchanged

Do not convert or annotate goal-semantic wildcards in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs) under this ticket. Those methods are explicitly outside S36 and should only change under a separate semantic-planning design ticket.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — document legitimate wildcards, add focused tests)
- `tickets/S36DECGOAREG-008-wildcard-audit-exhaustive-enforcement.md` (modify — correct assumptions/scope before implementation)

## Out of Scope

- Reopening already-exhaustive declaration dispatch in `goal_dispatch_key.rs`, `goal_dispatch_decl.rs`, `exhaustion.rs`, or `feasibility.rs`
- Removing active declaration data arrays from `goal_dispatch_decl.rs`
- Converting wildcard sites in goal-semantic methods in `goal_model.rs`
- Reintroducing or replacing `GoalKindTag`
- Any behavioral change in planning, ranking, invalidation, or feasibility
- Any changes to `worldwake-core`

## Acceptance Criteria

### Tests That Must Pass

1. Focused `decision_trace` tests prove omission-family helpers match the intended goal families and return `None` for unrelated goals.
2. Existing focused declaration dispatch tests still pass unchanged.
3. Existing suite: `cargo test -p worldwake-ai`
4. Full workspace: `cargo test --workspace`
5. Lint: `cargo clippy --workspace`

### Invariants

1. All declaration-routing surfaces required by S36 remain exhaustive and wildcard-free.
2. The only remaining in-scope wildcard defaults are legitimate family-scoped trace defaults, and they are documented inline.
3. No behavioral change: planning, feasibility, invalidation, and ranking outputs are unchanged.
4. No backwards-compatibility aliasing or parallel dispatch surfaces are introduced.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — add focused omission-family tests so legitimate `_ => None` defaults are explicit regression contracts.
2. `crates/worldwake-ai/src/goal_dispatch_decl.rs` — existing declaration coverage remains the proof surface for exhaustive dispatch parity; no new test required here.
3. `crates/worldwake-ai/src/goal_dispatch_key.rs` — existing payload-sensitive/exhaustiveness coverage remains the proof surface for declaration-key completeness; no new test required here.

### Commands

1. `cargo test -p worldwake-ai decision_trace::tests`
2. `cargo test -p worldwake-ai goal_dispatch_decl::tests`
3. `cargo test -p worldwake-ai goal_dispatch_key::tests`
4. `cargo test -p worldwake-ai`
5. `cargo test --workspace`
6. `cargo clippy --workspace`

## Outcome

Completed: 2026-03-29

What actually changed:
- Corrected the ticket scope to match the delivered S36 architecture: declaration routing was already exhaustive, `GoalKindTag` was already retired, and the remaining valuable work was limited to legitimate omission-family wildcard defaults in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs).
- Added inline comments on the `_ => None` branches in `omitted_political_reason_for_goal()` and `omitted_social_reason_for_goal()` to document why those defaults are architecturally correct.
- Added focused `decision_trace` regression tests proving political omissions only match political goal families and social omissions only match `ShareBelief`, with unrelated goals returning `None`.

Deviations from original plan:
- Did not add `#[deny(unreachable_patterns)]`; reassessment showed it would not strengthen the existing exhaustive dispatch contract.
- Did not remove `*_OPS` arrays or touch `goal_model.rs`; those assumptions were stale because the arrays are active declaration data in `goal_dispatch_decl.rs`, and goal-semantic wildcard sites remain intentionally out of scope.
- Did not attempt a synthetic “dummy `GoalKind` variant” compile-fail test; the real proof surface remains the live exhaustive matches plus focused declaration tests already in the repository.

Verification results:
- `cargo test -p worldwake-ai decision_trace::tests`
- `cargo test -p worldwake-ai goal_dispatch_decl::tests`
- `cargo test -p worldwake-ai goal_dispatch_key::tests`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace`
