# E16DPOLPLAN-031: Extract a focused political `agent_tick` test helper with full action registries

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — test-only harness cleanup in `worldwake-ai`
**Deps**: E16DPOLPLAN-029, E16DPOLPLAN-007

## Problem

Focused political runtime tests in `crates/worldwake-ai/src/agent_tick.rs` are more awkward than they should be because the local `Harness::new()` path only registers needs actions. Political runtime regressions therefore have to know that they must replace the harness registries with `build_full_action_registries()` before tracing any office-planning behavior.

That makes the tests easy to mis-specify:
- a ticket can ask for an `agent_tick` regression without saying whether it is needs-only or full-registry
- a new test can silently fail to exercise the intended action/affordance surface if it forgets to swap registries
- setup noise obscures the actual invariant under test

This is a test-infrastructure problem, not a production-architecture problem.

## Assumption Reassessment (2026-03-18)

1. `Harness::new()` in [`crates/worldwake-ai/src/agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs) registers only `register_needs_actions(&mut defs, &mut handlers)` — confirmed.
2. Existing focused helpers in the same file already opt into `build_full_action_registries(&recipes)` for broader action surfaces, specifically `cargo_harness()` and `build_exclusive_queue_harness()` in [`crates/worldwake-ai/src/agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs). The gap is therefore not “political tests have no full-registry helper anywhere”; it is that the base local `Harness` has no reusable opt-in for full registries.
3. The Force-law runtime regression added by E16DPOLPLAN-029 manually replaces `defs` and `handlers` with the full registries before the test setup is semantically correct — confirmed in `agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning` in [`crates/worldwake-ai/src/agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs).
4. The referenced political-planning spec path in the original request is stale. The active spec is [`specs/E16d-political-planning-and-golden-coverage.md`](/home/joeloverbeck/projects/worldwake/specs/E16d-political-planning-and-golden-coverage.md), not `specs/E16d-political-deliberation*.md`.
5. The gap is not missing production behavior and not a political-only harness omission. It is a missing, narrow, reusable base-harness opt-in for full action registries in focused `agent_tick` tests — corrected scope.

## Architecture Check

1. A generic local `Harness::with_full_action_registries()` is cleaner than a political-specific helper because the registry boundary is architectural, not domain-specific. Other focused `agent_tick` tests already need the same opt-in for cargo and queue scenarios.
2. The helper should stay local to `agent_tick` tests and remain a thin mutation of the existing harness. That preserves the fast focused-test style without creating a second golden harness or a generalized test framework.
3. No backwards-compatibility layer is needed. This is a pure test-support cleanup that makes the “needs-only vs full-registry” runtime boundary explicit.

## What to Change

### 1. Add a base-harness full-registry opt-in

- Extend the `#[cfg(test)]` support in `crates/worldwake-ai/src/agent_tick.rs` with a narrow helper such as:
  - `Harness::with_full_action_registries()`
- The helper should:
  - start from the existing local harness
  - load `build_full_action_registries(&recipes)`
  - preserve the local, fast `agent_tick` test style

### 2. Add minimal office-belief setup helpers if they reduce repetition

- If setup repetition remains high after the registry helper, add one or two narrowly scoped helpers for:
  - seeding a vacant office with explicit `OfficeData`
  - seeding direct beliefs for selected entities
- Keep the helpers local to the `agent_tick` test module unless extraction clearly reduces duplication without hiding important setup facts.

### 3. Migrate the existing Force-law runtime regression to the helper

- Update `agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning` to use the new helper.
- If another existing direct `Harness::new()` runtime test benefits from the helper without broad churn, migrate or add one narrow assertion that proves the helper is generic rather than Force-law-specific; otherwise keep the scope minimal.

## Files to Touch

- `crates/worldwake-ai/src/agent_tick.rs` (modify)

## Out of Scope

- Golden harness changes in `crates/worldwake-ai/tests/golden_harness/`
- Production action registration changes
- Changes to political planning behavior
- Cross-crate shared test framework extraction

## Acceptance Criteria

### Tests That Must Pass

1. The Force-law runtime regression still passes after migrating to the new helper.
2. At least one focused `agent_tick` runtime test exercises the full action registry path through the helper instead of bespoke registry replacement.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Focused `agent_tick` political tests explicitly run against the full action registry when they need non-needs affordances.
2. Test-support cleanup does not change production planning behavior or trace semantics.
3. The helper remains narrow and local; it does not become a duplicate golden harness.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — migrate `trace_force_law_office_skips_political_candidates_and_planning` to the new helper so the runtime scope is explicit and reusable.
2. `crates/worldwake-ai/src/agent_tick.rs` — add `harness_with_full_action_registries_exposes_non_needs_actions` to prove the helper is generic test infrastructure rather than a Force-law-only wrapper.

### Commands

1. `cargo test -p worldwake-ai agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning`
2. `cargo test -p worldwake-ai agent_tick::tests::harness_with_full_action_registries_exposes_non_needs_actions`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-18
- **What actually changed**:
  - Reassessed the ticket against the current `agent_tick` test architecture and corrected the scope from a political-specific helper gap to a generic base-harness full-registry opt-in.
  - Added `Harness::with_full_action_registries()` in [`crates/worldwake-ai/src/agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs) as a thin local test helper.
  - Migrated `trace_force_law_office_skips_political_candidates_and_planning` to the helper.
  - Added `harness_with_full_action_registries_exposes_non_needs_actions` to prove the helper exposes full-registry actions beyond needs-only coverage.
- **Deviations from original plan**:
  - No political-specific helper or extra office-belief helper was needed; the cleaner architecture is a generic registry opt-in on the existing harness.
  - The spec reference was corrected to [`specs/E16d-political-planning-and-golden-coverage.md`](/home/joeloverbeck/projects/worldwake/specs/E16d-political-planning-and-golden-coverage.md).
- **Verification results**:
  - `cargo test -p worldwake-ai agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning -- --exact` ✅
  - `cargo test -p worldwake-ai agent_tick::tests::harness_with_full_action_registries_exposes_non_needs_actions -- --exact` ✅
  - `cargo test -p worldwake-ai agent_tick::tests::trace_planning_outcome_for_hungry_agent -- --exact` ✅
  - `cargo test -p worldwake-ai` ✅
  - `cargo clippy --workspace` ✅
