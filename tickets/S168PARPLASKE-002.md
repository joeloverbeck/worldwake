# S168PARPLASKE-002: Populate `remaining_skeleton` at suspension sites

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/partial_plan.rs` (thread skeleton through `build_partial_plan_segment` + `budget_exhausted_partial_plan_segment`); `crates/worldwake-ai/src/agent_tick/planning.rs` (caller wires remaining skeleton through the seed); `crates/worldwake-ai/src/agenda_manager.rs` (new info-barrier suspension constructor wired from companion-spawning path).
**Deps**: `archive/tickets/S168PARPLASKE-005.md` (planner-owned preservable skeleton source); `specs/S168-partial-plan-skeleton-reuse.md` (D1.a, D1.b); `archive/specs/S149-partial-plan-segments-and-typed-terminals.md` (owns `PartialPlanSegmentSeed` shape).

## Problem

`PartialPlanSegment.remaining_skeleton: Option<Vec<PlannedSkeletonStep>>` is structurally serialized (`partial_plan.rs:33`) but never populated in production code. The only runtime constructor today, `budget_exhausted_partial_plan_segment` (`partial_plan.rs:118`), passes `remaining_skeleton: None` at line 123. The information-barrier suspension path in `agenda_manager.rs:147-180` doesn't persist a `PartialPlanSegment` at all — it spawns companion goals only.

This ticket makes the field live at the two in-scope suspension sites:

- **D1.a**: `budget_exhausted_partial_plan_segment` threads a populated `Some(Vec<PlannedSkeletonStep>)` when a meaningful remainder exists beyond the completed prefix; its caller `write_budget_exhausted_partial_plan_segments` in `agent_tick/planning.rs:1114` threads the skeleton from the partial search frontier through the seed.
- **D1.b**: A new info-barrier suspension constructor (calling `build_partial_plan_segment` at `partial_plan.rs:94`) is wired from the companion-spawning path in `agenda_manager.rs:147-180` with a populated `remaining_skeleton`, `PlanTerminalKind::InformationBarrier { … }` terminal, and the appropriate `BarrierFact`.

Both sites filter combat- and target-identity-bound steps from the preserved skeleton per the spec's exclusion (FND-21 risk: stale binding is more dangerous than replan).

This ticket compiles independently of ticket 001 (revalidation) and ticket 003 (resume consumption), but it depends on `archive/tickets/S168PARPLASKE-005.md` for the lawful planner-owned source of the skeleton. The populated field is just data once that source exists; no resume consumer is required here.

## Assumption Reassessment (2026-05-24)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Codebase shape**. Verified:
   - `build_partial_plan_segment` (`partial_plan.rs:78-108`) is the shared constructor; it absorbs a `PartialPlanSegmentSeed` and passes through `seed.remaining_skeleton` as-is to the constructed segment at line 98.
   - `budget_exhausted_partial_plan_segment` (`partial_plan.rs:111-138`) is the only runtime caller; line 123 passes `remaining_skeleton: None` in the seed.
   - `write_budget_exhausted_partial_plan_segments` in `agent_tick/planning.rs:1114` is the upstream caller (verified by `/reassess-spec` agent 1 census).
   - Test helpers exist at `agenda_manager.rs:1698` (`information_barrier_segment`) and `agenda_manager.rs:1719` (`coordination_barrier_segment`) but are `#[cfg(test)]`-only. They show the expected shape the production constructor must produce.
   - `PartialPlanSegmentSeed.remaining_skeleton` is `Option<Vec<PlannedSkeletonStep>>`; the spec's threading change requires no struct extension.
2. **Spec/doc references**. S168 D1.a and D1.b (`specs/S168-partial-plan-skeleton-reuse.md:132-153`). The spec's Problem Statement (lines 36-43) explicitly flags that D1.b adds a *new* construction site, not just populates an existing one — landed during reassessment, so this ticket carries the work.
3. **Mixed-layer boundary**. Shared boundary under audit: `PartialPlanSegmentSeed.remaining_skeleton` shape (already `Option<Vec<PlannedSkeletonStep>>`; this ticket changes the values flowing through it, not the type). All edits stay in `worldwake-ai` — no cross-crate impact.
4. **Existing tests under modification**:
   - `partial_plan.rs::budget_exhausted_partial_plan_segment_uses_typed_terminal_and_backoff:639` — verify whether this test asserts `remaining_skeleton: None`; if so, extend to cover both `None` (when no remainder) and `Some(_)` (when remainder exists).
   - `partial_plan.rs::build_partial_plan_segment_writes_concrete_barrier_segment:574` — likely passes `remaining_skeleton: None` in its seed; should add a sibling test passing `Some(_)` to verify pass-through.
   - `partial_plan.rs::partial_plan_segment_roundtrips_through_bincode_with_all_barrier_facts:378` — bincode round-trip; extend to include populated `remaining_skeleton` so the save/load path is covered.
   - `agenda_manager.rs::information_barrier_spawns_social_motive_ask_witness_companion:2089` — this test exercises the companion-spawning path D1.b wires into. Confirm whether the test now needs to assert that a `PartialPlanSegment` is also persisted in addition to the companion goal.
   - `agenda_manager.rs::information_barrier_does_not_spawn_without_plausible_witness:2136` — verify the negative path: no witness → no companion AND no segment. (The new constructor must fire under the same preconditions as the companion spawn.)
   - `agenda_manager.rs::abandoning_information_barrier_primary_cancels_companion:2167` — ensure the new segment is also abandoned when the primary is canceled (the existing `remove_companions_for_primary` path may or may not cover this; verify and extend if needed).
5. **Filter logic for excluded steps**. The spec excludes "combat- and target-identity-bound steps." This is a new filter that lives at the population sites. Reassessment: the filter operates on `PlannedSkeletonStep.target_template: PayloadTemplate` and `op: PlannerOpKind`; combat is identified by op kind, target-identity-bound is identified by `target_template` not being `FromContext` (i.e., the template carries a resolved `EntityId` rather than a context-resolved binding). Confirm by reading `PayloadTemplate` variants before implementing.
6. **Adjacent contradictions**. The D1.b constructor wires into the companion-spawning path, which may have abandon-condition implications: if the primary is canceled, the new segment must be cleaned up alongside the companion. Classification: required consequence of D1.b — the new segment shares lifecycle with the companion it accompanies.

## Reassessment Update (2026-05-24)

1. Live reassessment found the drafted D1.a source claim was too strong: `PlanSearchResult::BudgetExhausted` currently carries only `expansions_used`, so `write_budget_exhausted_partial_plan_segments` has no lawful remaining skeleton to thread yet.
2. Under FND-12/FND-27, this ticket must not synthesize a skeleton after the fact. `archive/tickets/S168PARPLASKE-005.md` now owns the prerequisite planner/search carrier that preserves the high-level shape while the planning referent is still live.
3. Ticket 002 resumes only after ticket 005 lands. Its D1.a implementation should consume the carrier from ticket 005, apply the construction filter, and pass the result into `PartialPlanSegmentSeed.remaining_skeleton`.

## Architecture Check

1. **No new authoritative type.** The change is content-only: populate an already-serialized `Option` field at construction time. No struct extension, no migration, no save-format bump (S168 Section H.6 confirms).
2. **Filter excluded steps at the construction boundary, not at the consumer.** The spec excludes combat- and target-identity-bound steps from the preserved skeleton. Filtering at construction (this ticket) keeps the consumer (ticket 003's `search_plan_seeded`) free of step-kind awareness — it can trust the skeleton as preserved.
3. **D1.a and D1.b share the same `build_partial_plan_segment` core constructor.** Both routes converge on `partial_plan.rs:94`; the change is in what each calling path passes for `remaining_skeleton`. Keeps the construction logic centralized and avoids two divergent code paths.
4. **D1.b's new constructor parallels the existing test helpers' shape.** `information_barrier_segment` (`agenda_manager.rs:1698`) and `coordination_barrier_segment:1719` already encode the right shape; the production constructor lifts that into runtime without redesigning the substrate.

## Verification Layers

1. **D1.a population correctness** → focused unit test extending `budget_exhausted_partial_plan_segment_uses_typed_terminal_and_backoff` (`partial_plan.rs:639`) to assert `Some(_)` skeleton when remainder exists, `None` when not.
2. **D1.a caller threading** → new focused test for `write_budget_exhausted_partial_plan_segments` confirming the skeleton from the partial search frontier reaches the seed (or, if the function is not directly testable, an integration check via `agent_tick/planning.rs` harness).
3. **D1.b construction site fires** → focused test on the new info-barrier constructor showing it produces a `PartialPlanSegment` with the expected `PlanTerminalKind::InformationBarrier { … }` terminal, the right `BarrierFact`, and a populated `remaining_skeleton`. **Strong proof surface**: assert against the actual constructor output, not via downstream observable behavior (which is ticket 003's territory).
4. **D1.b lifecycle cleanup** → focused test on `agenda_manager.rs` confirming that when an info-barrier primary is canceled (per `abandoning_information_barrier_primary_cancels_companion:2167`'s pattern), the new segment is also cleaned up.
5. **Filter for excluded step kinds** → focused unit tests on the filter function (combat-op skeleton step → excluded; resolved-`EntityId` target template → excluded; ordinary acquisition op with `FromContext` template → preserved).
6. **Save/load equivalence with populated skeleton** → extend `partial_plan_segment_roundtrips_through_bincode_with_all_barrier_facts:378` to include cases where `remaining_skeleton` is `Some(_)`.

This is a foundation-data ticket; no action trace or event-log delta is involved. Per precision rule 5, all verification surfaces are focused unit tests on the construction logic, exactly mapping each invariant to its proof.

## What to Change

### 1. D1.a — populate budget-exhausted skeleton

In `crates/worldwake-ai/src/partial_plan.rs`:

- Extend `budget_exhausted_partial_plan_segment` (line 111) to accept the remaining skeleton (e.g., add a `remaining: Option<Vec<PlannedSkeletonStep>>` parameter), apply the combat/target-identity filter, and pass the filtered result through the seed (replacing the `None` at line 123).
- Define a helper `filter_preservable_skeleton(steps: Vec<PlannedSkeletonStep>) -> Option<Vec<PlannedSkeletonStep>>` that excludes combat-op and target-identity-bound steps and returns `None` if the result is empty (no meaningful remainder).

In `crates/worldwake-ai/src/agent_tick/planning.rs`:

- Update `write_budget_exhausted_partial_plan_segments` (line 1114) to thread the partial search frontier's remaining op sequence through to `budget_exhausted_partial_plan_segment` as the new parameter. The source of the remaining sequence is the search frontier at the point of budget exhaustion — read from the same data the existing terminal-construction reads.

### 2. D1.b — info-barrier suspension constructor

In `crates/worldwake-ai/src/partial_plan.rs`:

- Add a new public constructor `info_barrier_partial_plan_segment(goal, completed_prefix, remaining_skeleton, barrier_fact, witness_target, created_tick, local_counter, cognitive) -> PartialPlanSegment`. It builds a `PartialPlanSegmentSeed` with `terminal_barrier: PlanTerminalKind::InformationBarrier { … }` and the supplied `BarrierFact`, then calls `build_partial_plan_segment`.
- Apply `filter_preservable_skeleton` (from §1) to the skeleton before passing through.

In `crates/worldwake-ai/src/agenda_manager.rs`:

- Inside the information-barrier suspension path (lines 147-180, the companion-spawning region), after the companion goal is spawned and before returning, construct a `PartialPlanSegment` via the new `info_barrier_partial_plan_segment` constructor and insert it into the agenda's partial-plan registry alongside the suspended entry. Mirror the shape used by the test helper `information_barrier_segment:1698`.
- Ensure cleanup parity: when `remove_companions_for_primary` (`agenda_manager.rs:979`) fires for an info-barrier primary, the new segment is also removed. If the existing cleanup already covers segments-by-primary, no extra work; otherwise extend it.

### 3. Test updates

In `crates/worldwake-ai/src/partial_plan.rs` `#[cfg(test)]`:

- Extend `budget_exhausted_partial_plan_segment_uses_typed_terminal_and_backoff:639` with `Some(_)` skeleton assertions (parameterized: empty-remainder → `None`, populated remainder → `Some(filtered)`).
- Extend `build_partial_plan_segment_writes_concrete_barrier_segment:574` to add a sibling test passing `Some(_)` in the seed and asserting pass-through.
- Extend `partial_plan_segment_roundtrips_through_bincode_with_all_barrier_facts:378` to include `Some(_)` cases.
- Add new tests for `filter_preservable_skeleton`: combat-op excluded, target-identity-bound excluded, ordinary `FromContext` preserved, all-excluded → `None`.

In `crates/worldwake-ai/src/agenda_manager.rs` `#[cfg(test)]`:

- Extend `information_barrier_spawns_social_motive_ask_witness_companion:2089` to also assert the persisted segment exists with populated `remaining_skeleton`.
- Add a new test confirming cleanup: info-barrier primary canceled → companion AND segment both removed.

## Files to Touch

- `crates/worldwake-ai/src/partial_plan.rs` (modify — D1.a constructor change, new `filter_preservable_skeleton`, new `info_barrier_partial_plan_segment` constructor, tests)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — thread skeleton through `write_budget_exhausted_partial_plan_segments`)
- `crates/worldwake-ai/src/agenda_manager.rs` (modify — wire `info_barrier_partial_plan_segment` from companion-spawning path; cleanup parity; test extensions)

## Out of Scope

- `revalidate_skeleton_step` — ticket 001 (D2).
- `search_plan_seeded` and `try_resume_partial_plan`'s consumption of the populated skeleton — ticket 003 (D3).
- `PartialPlanResumeTrace` struct and trace emission — ticket 003 (D4).
- Validation goldens — ticket 004.
- Preservation for resource/jurisdiction/coordination barriers or combat — spec Non-Goals.

## Acceptance Criteria

### Tests That Must Pass

1. All new and extended focused tests listed in "What to Change" §3.
2. Existing tests retain coverage: `budget_exhausted_partial_plan_segment_uses_typed_terminal_and_backoff`, `build_partial_plan_segment_writes_concrete_barrier_segment`, `build_partial_plan_segment_rejects_non_barrier_terminals`, `partial_plan_segment_roundtrips_through_bincode_with_all_barrier_facts`, `information_barrier_spawns_social_motive_ask_witness_companion`, `information_barrier_does_not_spawn_without_plausible_witness`, `abandoning_information_barrier_primary_cancels_companion` — all pass.
3. Existing suite: `cargo test -p worldwake-ai` passes (no regressions).

### Invariants

1. Combat-op and target-identity-bound `PlannedSkeletonStep`s are never present in a populated `remaining_skeleton`. Enforced by `filter_preservable_skeleton` tests.
2. The populated `remaining_skeleton` round-trips through bincode losslessly. Enforced by extended round-trip test.
3. Info-barrier suspension persists a `PartialPlanSegment` whenever a companion is spawned, and cleans both up together when the primary is canceled. Enforced by extended agenda tests.
4. `SAVE_FORMAT_VERSION` is **not** bumped — the field already serializes; only its populated content changes (S168 Section H.6).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/partial_plan.rs` `#[cfg(test)]` — extended budget-exhausted/build-segment/bincode tests, new filter tests (per §3).
2. `crates/worldwake-ai/src/agenda_manager.rs` `#[cfg(test)]` — extended info-barrier companion-spawn test, new info-barrier cleanup test (per §3).

### Commands

1. `cargo test -p worldwake-ai --lib partial_plan` — targeted partial-plan tests.
2. `cargo test -p worldwake-ai --lib agenda_manager` — targeted agenda-manager tests.
3. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` — lint.
4. `cargo test -p worldwake-ai` — full ai-crate suite.
