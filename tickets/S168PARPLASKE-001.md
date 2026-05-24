# S168PARPLASKE-001: Skeleton revalidation function

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — adds `partial_plan_revalidation.rs` (or similar sibling module) to `worldwake-ai`; new public function `revalidate_skeleton_step` and supporting `SkeletonRevalidationVerdict` enum.
**Deps**: `specs/S168-partial-plan-skeleton-reuse.md` (D2); `archive/specs/S114-plan-step-guards.md` (provides the `RequiredFact`/`Invalidator` belief-read patterns the revalidation reuses semantically); `archive/specs/S149-partial-plan-segments-and-typed-terminals.md` (owns `PlannedSkeletonStep`).

## Problem

`PartialPlanSegment.remaining_skeleton` (`crates/worldwake-ai/src/partial_plan.rs:33`) currently has no reader. Before the agenda's `try_resume_partial_plan` can consume a populated skeleton (S168 D3, ticket 003) to seed tactical search, the runtime needs a belief-backed predicate-checker that decides whether each `PlannedSkeletonStep` is still reusable against fresh beliefs — or whether the skeleton must be discarded for full replan. Without this function, every populated skeleton would either fossilize into a rail (FND-21 violation) or be unconditionally discarded (defeating the optimization).

This ticket delivers the foundation logic: a standalone function that takes a skeleton step + belief view and returns a verdict. It compiles independently of skeleton population (ticket 002) and resume integration (ticket 003).

## Assumption Reassessment (2026-05-24)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. **Codebase shape**. Verified against current code:
   - `PlannedSkeletonStep` is defined at `crates/worldwake-ai/src/partial_plan.rs:44-49` with fields `op: PlannerOpKind`, `target_template: PayloadTemplate`, `expected_pre: Vec<BeliefPredicate>` — no `ActionDefId`, no resolved bindings, no handler binding.
   - The S114 belief-read helpers reusable for skeleton-shaped facts are `believed_target_location` and `believed_commodity_stock` on `RuntimeBeliefView` (`crates/worldwake-sim/src/belief_view.rs:1608`), called from `check_guard` (`crates/worldwake-ai/src/plan_revalidation.rs:137-225`).
   - `BeliefPredicate` (referenced by `PlannedSkeletonStep.expected_pre`) lives in core; predicate evaluation is the substrate this ticket consumes.
2. **Spec/doc references**. S168 D2 (`specs/S168-partial-plan-skeleton-reuse.md:155-174`) defines this function's contract. S168 reassessment (2026-05-24) established that `classify_revalidation` (`plan_revalidation.rs:46-102`) is **not** reusable for skeleton steps because it requires a fully-instantiated `PlannedStep`. The new function lives in a sibling module to keep that separation explicit.
3. **Mixed-layer boundary**. This is an AI-internal ticket. Shared boundary under audit: `revalidate_skeleton_step(actor: EntityId, step: &PlannedSkeletonStep, view: &dyn RuntimeBeliefView) -> SkeletonRevalidationVerdict`. The boundary does not cross crates — `worldwake-ai` defines the function and consumes `RuntimeBeliefView` from `worldwake-sim`.
4. **Live `GoalKind` / operator surface under test**. The verdict matrix exercises `PlannerOpKind` variants that today carry skeleton-relevant operations (e.g., commodity-acquisition ops); the function operates per-step and is `GoalKind`-agnostic by construction (it consumes whatever skeleton was preserved). Verified: no `GoalKind`-specific dispatch is needed at the revalidation surface.
5. **Heuristic removal discipline**. This ticket adds a substrate (skeleton revalidation), not a heuristic. The "S114 guard semantics reused" framing in the spec refers to the belief-read patterns, not to S114's `PlanGuard` type or its `classify_revalidation` path — both are deliberately excluded per the spec's D2 implementation specifics.

## Architecture Check

1. **Sibling-module placement keeps concerns separate.** `plan_revalidation.rs` revalidates a fully-instantiated `PlannedStep` at execution boundary; `partial_plan_revalidation.rs` revalidates a `PlannedSkeletonStep` at resume boundary. Distinct types, distinct timing, distinct contracts — combining them would conflate the two revalidation surfaces and force `classify_revalidation` to grow an enum-of-step-kinds parameter.
2. **No backward compatibility shim.** The function is net-new; no existing skeleton-revalidation path is being deprecated.
3. **Verdict carries reason for traceability (FND-29).** `SkeletonRevalidationVerdict::Invalid(reason)` names the load-bearing assumption that broke. Ticket 003's `PartialPlanResumeTrace` (D4) will surface this reason in the decision trace.

## Verification Layers

1. **Verdict correctness** → focused unit tests in `crates/worldwake-ai/src/partial_plan_revalidation.rs` `#[cfg(test)]` block. One test per verdict branch: `Reusable` (all predicates hold), `Invalid(BeliefStale)`, `Invalid(BeliefContradicted)`, `Invalid(BeliefUnknown)`, `Invalid(TargetMoved)` (or whatever reason variants the design lands on).
2. **No world-truth read** → unit test using a `RuntimeBeliefView` mock that panics on any world-truth accessor; verdict matrix runs without panic, proving the function reads only belief view.
3. **Single-layer ticket** — this is pure AI-internal logic. No action trace, event-log delta, or authoritative world state involved. Verification stays in focused unit tests per layer-precision (precision rule 2).

## What to Change

### 1. Define `SkeletonRevalidationVerdict`

In a new module `crates/worldwake-ai/src/partial_plan_revalidation.rs`:

```rust
pub enum SkeletonRevalidationVerdict {
    Reusable,
    Invalid(SkeletonRevalidationReason),
}

pub enum SkeletonRevalidationReason {
    BeliefStale,
    BeliefContradicted,
    BeliefUnknown,
    TargetMoved,
    // additional reasons surface as verdict-matrix tests reveal them
}
```

Derives: `Debug`, `Clone`, `Copy` (if all reason variants are payload-free; otherwise `Clone` only), `Eq`, `PartialEq`. Ticket 003's trace struct (D4) will hold `SkeletonRevalidationReason` by value, so trait bounds should support that use.

### 2. Define `revalidate_skeleton_step`

```rust
pub fn revalidate_skeleton_step(
    actor: EntityId,
    step: &PlannedSkeletonStep,
    view: &dyn RuntimeBeliefView,
) -> SkeletonRevalidationVerdict;
```

Implementation specifics per S168 D2:

- Iterate `step.expected_pre: Vec<BeliefPredicate>` and evaluate each against `view`. Return `Invalid(BeliefStale)` / `Invalid(BeliefContradicted)` / `Invalid(BeliefUnknown)` on the first failing predicate (named per the predicate's failure mode).
- For predicates that name a target (e.g., commodity-acquisition steps), additionally check `view.believed_target_location(actor, target)` to detect `TargetMoved`.
- **Do NOT** call into `classify_revalidation`, `check_guard`, or any other path that requires a fully-instantiated `PlannedStep`. The skeleton step has no `ActionDefId`, no resolved bindings.
- Iterate in stable order; the verdict's failure mode is deterministic given the same belief state.

### 3. Re-export from `lib.rs`

Add `pub mod partial_plan_revalidation;` and re-export `SkeletonRevalidationVerdict`, `SkeletonRevalidationReason`, and `revalidate_skeleton_step` so ticket 003 can call them from `agenda_manager.rs`.

### 4. Verdict matrix tests

In the new module's `#[cfg(test)]` block:

1. `revalidate_returns_reusable_when_all_predicates_hold` — predicates evaluate to true → `Reusable`.
2. `revalidate_returns_belief_stale_when_predicate_freshness_expired` — one predicate's source belief is past its freshness window.
3. `revalidate_returns_belief_contradicted_when_predicate_explicitly_contradicted` — belief view reports `BeliefStatus::Contradicted` for a predicate's claim.
4. `revalidate_returns_belief_unknown_when_predicate_has_no_belief` — belief view returns `None` for a load-bearing claim.
5. `revalidate_returns_target_moved_when_believed_location_differs` — `believed_target_location` returns a different place than the skeleton step expected.
6. `revalidate_reads_only_belief_view_never_world_truth` — uses a belief-view mock that panics on any world accessor; verifies all of (1)–(5) pass without triggering the panic.
7. `revalidate_iterates_stable_order` — runs the same belief view twice and confirms identical verdict + same failure-attribution reason.

## Files to Touch

- `crates/worldwake-ai/src/partial_plan_revalidation.rs` (new)
- `crates/worldwake-ai/src/lib.rs` (modify — add module + re-exports)

## Out of Scope

- Calling `revalidate_skeleton_step` from `try_resume_partial_plan` — that's ticket 003 (D3 resume consumption).
- Populating `remaining_skeleton` at suspension sites — ticket 002 (D1.a/D1.b).
- `PartialPlanResumeTrace` struct definition — ticket 003 (D4); this ticket only ensures `SkeletonRevalidationReason` has the trait bounds the trace struct will require.
- Reuse of `PlanGuard` / `RequiredFact` types from `crates/worldwake-ai/src/plan_guard.rs` — the spec's D2 says "optionally reuse" but the cleaner first-ship lands without that dependency; reusing the helper functions (`believed_target_location`, `believed_commodity_stock`) is the actually-named reuse path.

## Acceptance Criteria

### Tests That Must Pass

1. All 7 verdict-matrix tests listed in "What to Change" section 4.
2. `cargo test -p worldwake-ai --lib partial_plan_revalidation` passes.
3. Existing suite: `cargo test -p worldwake-ai` passes (no regressions in unrelated AI tests).

### Invariants

1. `revalidate_skeleton_step` reads only `RuntimeBeliefView`; no `World`, `WorldTxn`, or other authoritative world-state access. Enforced by the no-panic mock test (#6).
2. Iteration of `expected_pre` predicates is deterministic; given the same belief state, the same verdict (and same failure-attribution reason if `Invalid`) is produced. Enforced by stable-order test (#7).
3. The verdict enum carries enough information for ticket 003's trace struct to surface the failure-attribution reason without re-running revalidation. Enforced by the trace struct compiling against `SkeletonRevalidationReason` in ticket 003.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/partial_plan_revalidation.rs` `#[cfg(test)]` block — 7 focused unit tests covering the verdict matrix, world-truth-isolation, and determinism (see "What to Change" section 4).

### Commands

1. `cargo test -p worldwake-ai --lib partial_plan_revalidation` — targeted verdict-matrix tests.
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings` — lint the new module.
3. `cargo test -p worldwake-ai` — full ai-crate suite to confirm no regressions.
