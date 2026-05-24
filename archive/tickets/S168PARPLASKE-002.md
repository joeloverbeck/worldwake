# S168PARPLASKE-002: Populate budget-exhausted `remaining_skeleton`

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `crates/worldwake-ai/src/partial_plan.rs` and `crates/worldwake-ai/src/agent_tick/planning.rs`.
**Deps**: `archive/tickets/S168PARPLASKE-005.md` (planner-owned preservable skeleton source); `specs/S168-partial-plan-skeleton-reuse.md` (D1.a); `archive/specs/S149-partial-plan-segments-and-typed-terminals.md` (owns `PartialPlanSegmentSeed` shape).

## Problem

Before this ticket, `PartialPlanSegment.remaining_skeleton: Option<Vec<PlannedSkeletonStep>>` was serialized but the budget-exhausted production constructor always populated it as `None`. `archive/tickets/S168PARPLASKE-005.md` added the lawful planner-owned `PartialPlanSkeletonSource`, so this ticket threads that already-derived skeleton source into budget-exhausted `PartialPlanSegment` construction.

Live reassessment corrected the original D1.b wording: `spawn_information_barrier_companions` consumes already-suspended information-barrier `PartialPlanSegment`s; it is not a lawful producer because it skips entries without a segment. The information-barrier production seam remained real S168 work and later landed in `archive/tickets/S168PARPLASKE-006.md`, not this budget-exhausted population ticket.

## Assumption Reassessment (2026-05-24)

1. **Codebase shape**. `build_partial_plan_segment` passes `PartialPlanSegmentSeed.remaining_skeleton` through unchanged. `budget_exhausted_partial_plan_segment` was the budget-exhausted construction boundary and used `remaining_skeleton: None`. `write_budget_exhausted_partial_plan_segments` already carried `CandidatePlanSearch.skeleton_source` from ticket 005 but did not pass it into the constructor.
2. **Corrected D1.b boundary**. `spawn_information_barrier_companions` iterates `state.suspended`, reads `entry.partial_plan_segment`, and continues when it is absent. Because the companion function consumes a segment to discover the information-barrier topic, it cannot also be the first producer of that segment without circular control flow. `archive/tickets/S168PARPLASKE-006.md` owns the corrected producer boundary.
3. **Mixed-layer boundary**. This ticket changes only the AI-internal `PartialPlanSegmentSeed.remaining_skeleton` value at the budget-exhausted writer boundary. It adds no authoritative type and does not change save format.
4. **Filter boundary**. Combat and fixed-target-identity skeleton steps are filtered at `partial_plan.rs` construction time. Ticket 005 already avoids producing such sources in normal planner flow, but the segment constructor now enforces the invariant for direct callers too.
5. **Proof split**. Focused unit tests prove constructor pass-through, construction filtering, budget-exhausted writer threading, and bincode round-trip of populated skeleton content. Cross-system information-barrier reuse and fallback goldens remain downstream of tickets 003, 004, and 006.

## Architecture Check

1. Populating from `CandidatePlanSearch.skeleton_source` preserves FND-12/FND-27: the skeleton is a deleteable planning cache derived while the planner referent is live, not a synthesized truth surface.
2. Filtering at construction keeps consumers from needing to rediscover which `PlannerOpKind` or `PayloadTemplate` values are unsafe to preserve.
3. The information-barrier producer split avoids a circular agenda path and leaves the companion-spawn consumer unchanged until a lawful producer exists.

## Verified Layers

1. Budget-exhausted constructor population -> focused unit test on `budget_exhausted_partial_plan_segment`.
2. Constructor-level filter -> focused unit test on `filter_preservable_skeleton`.
3. Writer threading from `CandidatePlanSearch.skeleton_source` -> focused unit test on `write_budget_exhausted_partial_plan_segments`.
4. Serialization of populated skeleton content -> existing bincode round-trip test now includes populated skeleton values.

## Landed Changes

1. `budget_exhausted_partial_plan_segment` now accepts `Option<Vec<PlannedSkeletonStep>>`, filters it through `filter_preservable_skeleton`, and passes preserved values into `PartialPlanSegmentSeed.remaining_skeleton`.
2. `write_budget_exhausted_partial_plan_segments` now clones the `CandidatePlanSearch.skeleton_source.remaining_skeleton` from ticket 005 and passes it into the budget-exhausted segment constructor.
3. Added constructor/filter tests for preserved trade skeletons, empty skeletons, combat exclusions, and fixed-identity exclusions.
4. Added writer-threading coverage proving a budget-exhausted suspended agenda entry receives the planner-provided skeleton.

## Landed Files

- `crates/worldwake-ai/src/partial_plan.rs`
- `crates/worldwake-ai/src/agent_tick/planning.rs`
- `archive/tickets/S168PARPLASKE-006.md`
- `specs/S168-partial-plan-skeleton-reuse.md`
- `archive/tickets/S168PARPLASKE-003.md`
- `tickets/S168PARPLASKE-004.md`

## Out of Scope

- Information-barrier partial-plan production — `archive/tickets/S168PARPLASKE-006.md`.
- `revalidate_skeleton_step` — archived ticket 001.
- `search_plan_seeded`, resume consumption, and `PartialPlanResumeTrace` — ticket 003.
- Validation goldens — ticket 004.
- Preservation for resource/jurisdiction/coordination barriers or combat — S168 non-goals.

## Acceptance Result

### Tests Passed

1. Passed focused constructor/filter/writer tests via `cargo test -p worldwake-ai --lib partial_plan`.
2. Passed affected crate suite via `cargo test -p worldwake-ai`.
3. Passed final lint gate via `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.

### Invariants

1. Budget-exhausted segments preserve a non-empty planner-provided skeleton after filtering.
2. Combat and fixed-target-identity skeleton steps are absent from populated `remaining_skeleton`.
3. Populated skeleton content round-trips through bincode.
4. `SAVE_FORMAT_VERSION` is unchanged because the serialized field already existed.

## Outcome

Completed on 2026-05-24.

- Populated budget-exhausted partial-plan segments from the planner-owned `PartialPlanSkeletonSource` added by S168PARPLASKE-005.
- Added construction-boundary filtering so combat and fixed-target-identity skeleton steps are not preserved.
- Proved constructor pass-through, filtering, bincode round-trip, and budget-exhausted writer threading with focused tests.
- Split the disproved information-barrier producer path into the now-archived `archive/tickets/S168PARPLASKE-006.md` and truth-synced S168 plus downstream tickets 003 and 004.

## Deviations

- The drafted D1.b agenda-companion producer was disproved by live code. `spawn_information_barrier_companions` needs an existing information-barrier segment before it can spawn a companion. The corrected information-barrier producer later landed in `archive/tickets/S168PARPLASKE-006.md`.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib partial_plan`.
- Passed `cargo test -p worldwake-ai`.
- Passed `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
