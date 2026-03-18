# E16DPOLPLAN-031: Extract a focused political `agent_tick` test helper with full action registries

**Status**: PENDING
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
2. Existing cargo-style helper setups in the same file already use `build_full_action_registries(&recipes)` for richer action surfaces, for example in `cargo_harness()` — confirmed in [`crates/worldwake-ai/src/agent_tick.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick.rs).
3. The new Force-law runtime regression added by E16DPOLPLAN-029 had to manually replace `defs` and `handlers` with the full registries before the test setup was semantically correct — confirmed in `agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning`.
4. The gap is not missing production behavior. It is missing focused test support for runtime political-planning cases — corrected scope.

## Architecture Check

1. A narrow test helper is cleaner than repeating bespoke registry replacement and political setup logic in every future runtime test.
2. The helper should stay local to `agent_tick` tests or a nearby test-only support module. It should not become a second golden harness or a general-purpose abstraction.
3. No backwards-compatibility layer is needed. This is a pure test-support cleanup that makes the intended runtime boundary explicit.

## What to Change

### 1. Add a political runtime harness helper

- Extend the `#[cfg(test)]` support in `crates/worldwake-ai/src/agent_tick.rs` with a narrow helper such as:
  - `Harness::with_full_action_registries()`
  - or a clearly named free function like `political_trace_harness()`
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
- If another existing runtime trace test benefits from the helper without broad churn, migrate that test too; otherwise keep the scope minimal.

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
2. At least one focused `agent_tick` political runtime test exercises the full action registry path through the helper instead of bespoke registry replacement.
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Focused `agent_tick` political tests explicitly run against the full action registry when they need non-needs affordances.
2. Test-support cleanup does not change production planning behavior or trace semantics.
3. The helper remains narrow and local; it does not become a duplicate golden harness.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/agent_tick.rs` — migrate `trace_force_law_office_skips_political_candidates_and_planning` to the new helper so the runtime scope is explicit and reusable.
2. `crates/worldwake-ai/src/agent_tick.rs` — add or adjust one additional small runtime trace test only if needed to prove the helper is not Force-law-specific.

### Commands

1. `cargo test -p worldwake-ai agent_tick::tests::trace_force_law_office_skips_political_candidates_and_planning`
2. `cargo test -p worldwake-ai agent_tick::tests::trace_planning_outcome_for_hungry_agent`
3. `cargo test -p worldwake-ai`

